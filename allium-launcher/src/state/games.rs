use std::cmp::min;
use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Result;
use embedded_font::FontTextStyleBuilder;
use embedded_graphics::{
    image::{Image, ImageRaw},
    prelude::*,
    primitives::Rectangle,
    text::Alignment,
};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::trace;

use common::constants::{
    self, ALLIUM_ROMS_DIR, BUTTON_DIAMETER, IMAGE_SIZE, LISTING_JUMP_SIZE, LISTING_SIZE,
    SELECTION_HEIGHT, SELECTION_MARGIN,
};
use common::display::Display;
use common::platform::Color;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;

use crate::cores::CoreMapper;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamesState {
    top: i32,
    selected: i32,
    entries: Vec<Entry>,
    stack: Vec<Directory>,
    #[serde(skip)]
    core_mapper: CoreMapper,
}

impl GamesState {
    pub fn new() -> Result<GamesState> {
        let directory = Directory::default();
        Ok(GamesState {
            top: 0,
            selected: 0,
            entries: entries(&directory)?,
            stack: vec![directory],
            core_mapper: CoreMapper::new(),
        })
    }

    pub fn enter(&mut self) -> Result<()> {
        self.core_mapper.load_config()?;
        Ok(())
    }

    pub fn leave(&mut self) -> Result<()> {
        Ok(())
    }

    fn directory(&self) -> &Directory {
        self.stack.last().unwrap()
    }

    fn push_directory(&mut self, directory: Directory) -> Result<()> {
        self.stack.push(directory);
        self.entries = entries(self.directory())?;
        self.top = 0;
        self.selected = 0;
        Ok(())
    }

    fn pop_directory(&mut self) -> Result<()> {
        if self.stack.len() > 1 {
            self.stack.pop();
            self.entries = entries(self.directory())?;
            self.top = 0;
            self.selected = 0;
        }
        Ok(())
    }

    fn launch_game(&mut self, game: &Game) -> Result<()> {
        let core = self.core_mapper.get_core(&game.extension);
        if let Some(core) = core {
            lazy_static! {
                static ref ALLIUM_GAME_INFO: String = env::var("ALLIUM_GAME_INFO")
                    .unwrap_or_else(|_| constants::ALLIUM_GAME_INFO.to_string());
            }
            write!(
                File::create(&*ALLIUM_GAME_INFO)?,
                "{}\n{}\n{}\n{}",
                core.path.as_path().as_os_str().to_str().unwrap_or(""),
                game.path.as_path().as_os_str().to_str().unwrap_or(""),
                game.name,
                game.image
                    .as_ref()
                    .and_then(|path| path.as_path().as_os_str().to_str())
                    .unwrap_or(""),
            )?;
            core.launch(&game.path)?;
        }
        Ok(())
    }

    fn select_entry(&mut self, entry: Entry) -> Result<()> {
        match entry {
            Entry::Directory(directory) => self.push_directory(directory)?,
            Entry::Game(game) => self.launch_game(&game)?,
        }
        Ok(())
    }

    pub fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<()> {
        let Size { width, height } = display.size();

        let text_style = FontTextStyleBuilder::new(styles.ui_font.clone())
            .font_size(styles.ui_font_size)
            .text_color(styles.fg_color)
            .build();

        let selection_style = FontTextStyleBuilder::new(styles.ui_font.clone())
            .font_size(styles.ui_font_size)
            .text_color(styles.fg_color)
            .background_color(styles.primary)
            .build();

        // Draw game list
        let (x, mut y) = (24, 58);

        // Clear previous selection
        display.load(Rectangle::new(
            Point::new(x - 12, y - 4),
            Size::new(
                336,
                LISTING_SIZE as u32 * (SELECTION_HEIGHT + SELECTION_MARGIN),
            ),
        ))?;

        for i in (self.top as usize)
            ..std::cmp::min(
                self.entries.len(),
                self.top as usize + LISTING_SIZE as usize,
            )
        {
            let entry = &self.entries[i];

            if self.selected == i as i32 {
                if let Entry::Game(Game {
                    image: Some(image), ..
                }) = entry
                {
                    let mut image = image::open(image)?;
                    if image.width() != IMAGE_SIZE.width || image.height() > IMAGE_SIZE.height {
                        let new_height = min(
                            IMAGE_SIZE.height,
                            IMAGE_SIZE.width * image.height() / image.width(),
                        );
                        image = image.resize_to_fill(
                            IMAGE_SIZE.width,
                            new_height,
                            image::imageops::FilterType::Triangle,
                        );
                    }
                    display.load(Rectangle::new(
                        Point::new(
                            width as i32 - IMAGE_SIZE.width as i32 - 24,
                            54 + image.height() as i32,
                        ),
                        Size::new(IMAGE_SIZE.width, IMAGE_SIZE.height - image.height()),
                    ))?;

                    let mut image = image.to_rgb8();
                    common::image::round(&mut image, image::Rgb([0u8; 3]), 12);
                    let image: ImageRaw<Color> = ImageRaw::new(&image, IMAGE_SIZE.width);
                    let image = Image::new(
                        &image,
                        Point::new(width as i32 - IMAGE_SIZE.width as i32 - 24, 54),
                    );
                    image.draw(display)?;
                } else {
                    display.load(Rectangle::new(
                        Point::new(width as i32 - IMAGE_SIZE.width as i32 - 24, 54),
                        IMAGE_SIZE,
                    ))?;
                }

                display.draw_entry(
                    Point { x, y },
                    entry.name(),
                    selection_style.clone(),
                    Alignment::Left,
                    300,
                    true,
                )?;
            } else {
                display.draw_entry(
                    Point { x, y },
                    entry.name(),
                    text_style.clone(),
                    Alignment::Left,
                    300,
                    true,
                )?;
            }
            y += (SELECTION_HEIGHT + SELECTION_MARGIN) as i32;
        }

        // Draw button hints
        let y = height as i32 - BUTTON_DIAMETER as i32 - 8;
        let mut x = width as i32 - 12;

        x = display
            .draw_button_hint(
                Point::new(x, y),
                Key::A,
                text_style.clone(),
                "Start",
                styles,
            )?
            .top_left
            .x
            - 18;
        display.draw_button_hint(Point::new(x, y), Key::B, text_style, "Back", styles)?;

        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<bool> {
        Ok(match key_event {
            KeyEvent::Pressed(Key::A) => {
                let entry = self.entries.get(self.selected as usize);
                if let Some(entry) = entry {
                    self.select_entry(entry.to_owned())?;
                }
                true
            }
            KeyEvent::Pressed(Key::B) => {
                self.pop_directory()?;
                true
            }
            KeyEvent::Pressed(key) | KeyEvent::Autorepeat(key) => match key {
                Key::Up => {
                    self.selected = (self.selected - 1).rem_euclid(self.entries.len() as i32 - 1);
                    if self.selected < self.top {
                        self.top = self.selected
                    }
                    trace!("selected: {}, top: {}", self.selected, self.top);
                    true
                }
                Key::Down => {
                    self.selected = (self.selected + 1).rem_euclid(self.entries.len() as i32 - 1);
                    if self.selected - LISTING_SIZE >= self.top {
                        self.top = self.selected - LISTING_SIZE + 1;
                    }
                    trace!("selected: {}, top: {}", self.selected, self.top);
                    true
                }
                Key::Left => {
                    self.selected =
                        (self.selected - LISTING_JUMP_SIZE).clamp(0, self.entries.len() as i32 - 1);
                    if self.selected < self.top {
                        self.top = self.selected
                    }
                    true
                }
                Key::Right => {
                    self.selected =
                        (self.selected + LISTING_JUMP_SIZE).clamp(0, self.entries.len() as i32 - 1);
                    if self.selected - LISTING_SIZE >= self.top {
                        self.top = self.selected - LISTING_SIZE + 1;
                    }
                    true
                }
                _ => false,
            },
            _ => false,
        })
    }
}

pub fn entries(directory: &Directory) -> Result<Vec<Entry>> {
    let mut entries: Vec<_> = std::fs::read_dir(&directory.path)?
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Game {
    pub name: String,
    pub full_name: String,
    pub path: PathBuf,
    pub image: Option<PathBuf>,
    pub extension: String,
}

impl Ord for Game {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.full_name.cmp(&other.full_name)
    }
}

impl PartialOrd for Game {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
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
            path: PathBuf::from_str(
                &env::var("ALLIUM_ROMS_DIR").unwrap_or_else(|_| ALLIUM_ROMS_DIR.to_owned()),
            )
            .unwrap(),
        }
    }
}

const IMAGE_EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "webp", "gif", "tga", "bmp"];

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

        // Don't add images
        if file_name == "Imgs" {
            return Ok(None);
        }
        if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
            return Ok(None);
        }

        let full_name = match path.file_stem().and_then(OsStr::to_str) {
            Some(name) => name.to_owned(),
            None => return Ok(None),
        };
        let mut name = full_name.clone();

        // Remove numbers
        lazy_static! {
            static ref NUMBERS_RE: Regex = Regex::new(r"^\d+").unwrap();
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

        let image = path.parent().and_then(|path| {
            let mut path = path.to_path_buf();
            path.extend(["Imgs", file_name]);
            for extension in IMAGE_EXTENSIONS {
                if path.set_extension(extension) && path.exists() {
                    return Some(path);
                }
            }
            None
        });

        Ok(Some(Entry::Game(Game {
            name,
            full_name,
            path,
            image,
            extension,
        })))
    }
}
