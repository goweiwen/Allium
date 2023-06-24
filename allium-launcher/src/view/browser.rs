use std::collections::VecDeque;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use common::command::Command;
use common::constants::{ALLIUM_GAMES_DIR, BUTTON_DIAMETER, IMAGE_SIZE, SELECTION_MARGIN};
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::{Stylesheet, StylesheetColor};
use common::view::{ButtonHint, Image, ImageMode, Row, ScrollList, View};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::consoles::{ConsoleMapper, Game};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserState {
    pub directory: Directory,
    pub selected: usize,
    pub child: Option<Box<BrowserState>>,
}

#[derive(Debug)]
pub struct Browser {
    rect: Rect,
    res: Resources,
    directory: Directory,
    entries: Vec<Entry>,
    list: ScrollList,
    image: Image,
    button_hints: Row<ButtonHint<String>>,
    pub child: Option<Box<Browser>>,
}

impl Browser {
    pub fn new(rect: Rect, res: Resources, directory: Directory, selected: usize) -> Result<Self> {
        let Rect { x, y, w, h } = rect;

        let entries = entries(&directory)?;
        let mut list = ScrollList::new(
            Rect::new(
                x + 12,
                y + 8,
                w - IMAGE_SIZE.width - 12 - 12 - 24,
                h - 8 - 48,
            ),
            entries.iter().map(|e| e.name().to_string()).collect(),
            Alignment::Left,
            res.get::<Stylesheet>().ui_font.size + SELECTION_MARGIN,
        );
        list.select(selected);

        let mut image = Image::empty(
            Rect::new(
                x + w as i32 - IMAGE_SIZE.width as i32 - 24,
                y + 8,
                IMAGE_SIZE.width,
                IMAGE_SIZE.height,
            ),
            ImageMode::Contain,
        );
        image
            .set_background_color(StylesheetColor::Background)
            .set_border_radius(12);

        let button_hints = Row::new(
            Point::new(x + w as i32 - 12, y + h as i32 - BUTTON_DIAMETER as i32 - 8),
            {
                let locale = res.get::<Locale>();
                vec![
                    ButtonHint::new(
                        Point::zero(),
                        Key::A,
                        locale.t("button-select"),
                        Alignment::Right,
                    ),
                    ButtonHint::new(
                        Point::zero(),
                        Key::B,
                        locale.t("button-back"),
                        Alignment::Right,
                    ),
                ]
            },
            Alignment::Right,
            12,
        );

        Ok(Self {
            rect,
            res,
            directory,
            entries,
            list,
            image,
            button_hints,
            child: None,
        })
    }

    pub fn load(rect: Rect, res: Resources, state: BrowserState) -> Result<Self> {
        let mut browser = Self::new(rect, res.clone(), state.directory, state.selected)?;
        if let Some(child) = state.child {
            browser.child = Some(Box::new(Self::load(rect, res, *child)?));
        }
        Ok(browser)
    }

    pub fn save(&self) -> BrowserState {
        BrowserState {
            directory: self.directory.clone(),
            selected: self.list.selected(),
            child: self.child.as_ref().map(|c| Box::new(c.save())),
        }
    }

    async fn select_entry(&mut self, commands: Sender<Command>) -> Result<()> {
        if let Some(entry) = self.entries.get_mut(self.list.selected()) {
            match entry {
                Entry::Directory(dir) => {
                    let child = Browser::new(self.rect, self.res.clone(), dir.to_owned(), 0)?;
                    self.child = Some(Box::new(child));
                }
                Entry::Game(game) => {
                    if let Some(command) = self
                        .res
                        .get::<ConsoleMapper>()
                        .launch_game(&self.res.get(), game)?
                    {
                        commands.send(command).await?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[async_trait(?Send)]
impl View for Browser {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if let Some(ref mut child) = self.child {
            return Ok(child.should_draw() && child.draw(display, styles)?);
        }

        let mut drawn = false;

        if self.list.should_draw() && self.list.draw(display, styles)? {
            drawn = true;
        }

        if styles.enable_box_art {
            // TODO: relayout list if box art is enabled/disabled
            if let Some(entry) = self.entries.get_mut(self.list.selected()) {
                if let Some(path) = entry.image() {
                    self.image.set_path(Some(path.to_path_buf()));
                } else {
                    self.image.set_path(None);
                }
                if self.image.should_draw() && self.image.draw(display, styles)? {
                    drawn = true;
                }
            }
        }

        if self.button_hints.should_draw() && self.button_hints.draw(display, styles)? {
            drawn = true;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.child
            .as_ref()
            .map(|c| c.should_draw())
            .unwrap_or(false)
            || self.list.should_draw()
            || self.button_hints.should_draw()
    }

    fn set_should_draw(&mut self) {
        if let Some(c) = self.child.as_mut() {
            c.set_should_draw()
        }
        self.list.set_should_draw();
        self.image.set_should_draw();
        self.button_hints.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if let Some(child) = self.child.as_deref_mut() {
            if child.handle_key_event(event, commands, bubble).await? {
                bubble.retain(|cmd| match cmd {
                    Command::CloseView => {
                        self.child = None;
                        self.set_should_draw();
                        false
                    }
                    _ => true,
                });
                return Ok(true);
            }
            return Ok(false);
        }

        match event {
            KeyEvent::Pressed(Key::A) => {
                self.select_entry(commands).await?;
                Ok(true)
            }
            KeyEvent::Pressed(Key::B) => {
                bubble.push_back(Command::CloseView);
                Ok(true)
            }
            _ => self.list.handle_key_event(event, commands, bubble).await,
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        if let Some(child) = self.child.as_ref() {
            vec![child.as_ref()]
        } else {
            vec![&self.list, &self.image, &self.button_hints]
        }
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        if let Some(child) = self.child.as_mut() {
            vec![child.as_mut()]
        } else {
            vec![&mut self.list, &mut self.image, &mut self.button_hints]
        }
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}

pub fn entries(directory: &Directory) -> Result<Vec<Entry>> {
    let mut entries: Vec<_> = std::fs::read_dir(&directory.path)
        .map_err(|e| anyhow!("Failed to open directory: {:?}, {}", &directory.path, e))?
        .flat_map(|entry| entry.ok())
        .flat_map(|entry| match Entry::new(entry.path()) {
            Ok(Some(entry)) => Some(entry),
            _ => None,
        })
        .collect();
    entries.sort_unstable();
    Ok(entries)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Entry {
    Directory(Directory),
    Game(Game),
}

impl Entry {
    pub fn name(&self) -> &str {
        match self {
            Entry::Game(game) => &game.name,
            Entry::Directory(directory) => &directory.name,
        }
    }

    pub fn image(&mut self) -> Option<&Path> {
        match self {
            Entry::Game(game) => game.image(),
            Entry::Directory(_) => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Directory {
    pub name: String,
    pub full_name: String,
    pub path: PathBuf,
}

impl Ord for Directory {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.full_name.cmp(&other.full_name)
    }
}

impl PartialOrd for Directory {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Default for Directory {
    fn default() -> Self {
        Directory {
            name: "Games".to_string(),
            full_name: "Games".to_string(),
            path: ALLIUM_GAMES_DIR.to_owned(),
        }
    }
}

const EXCLUDE_EXTENSIONS: [&str; 1] = ["db"];

impl Entry {
    fn new(path: PathBuf) -> Result<Option<Entry>> {
        // Don't add hidden files starting with .
        let file_name = match path.file_name().and_then(OsStr::to_str) {
            Some(file_name) => file_name,
            None => return Ok(None),
        };
        if file_name.starts_with('.') {
            return Ok(None);
        }

        let extension = path
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or_default()
            .to_owned();

        // Exclude Imgs and Guide directories
        if file_name == "Imgs" || file_name == "Guides" {
            return Ok(None);
        }
        if EXCLUDE_EXTENSIONS.contains(&extension.as_str()) {
            return Ok(None);
        }

        let full_name = match path.file_stem().and_then(OsStr::to_str) {
            Some(name) => name.to_owned(),
            None => return Ok(None),
        };
        let mut name = full_name.clone();

        // Remove numbers
        lazy_static! {
            static ref NUMBERS_RE: Regex = Regex::new(r"^\d+[.\)]").unwrap();
        }
        name = NUMBERS_RE.replace(&name, "").to_string();

        // Remove trailing parenthesis
        lazy_static! {
            static ref PARENTHESIS_RE: Regex = Regex::new(r"[\(\[].+[\)\]]$").unwrap();
        }
        name = PARENTHESIS_RE.replace(&name, "").to_string();

        // Trim whitespaces
        name = name.trim().to_owned();

        // Directories without extensions can be navigated into
        if extension.is_empty() && path.is_dir() {
            return Ok(Some(Entry::Directory(Directory {
                name,
                full_name,
                path,
            })));
        }

        Ok(Some(Entry::Game(Game::new(name, path))))
    }
}
