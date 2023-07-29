use std::collections::VecDeque;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::{IMAGE_WIDTH, SELECTION_MARGIN};
use common::database::Database;
use common::display::Display;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::{Stylesheet, StylesheetColor};
use common::view::{ButtonHint, ButtonIcon, Image, ImageMode, Row, ScrollList, View};
use embedded_graphics::prelude::{Dimensions, OriginDimensions, Size};
use embedded_graphics::primitives::{CornerRadii, Primitive, PrimitiveStyle, RoundedRectangle};
use embedded_graphics::Drawable;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::consoles::ConsoleMapper;
use crate::entry::{Entry, Sort};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryListState<S> {
    pub sort: S,
    pub selected: usize,
    pub child: Option<Box<EntryListState<S>>>,
}

#[derive(Debug)]
pub struct CoreSelection {
    core: usize,
    cores: Vec<String>,
}

#[derive(Debug)]
pub struct EntryList<S>
where
    S: Sort,
{
    rect: Rect,
    res: Resources,
    entries: Vec<Entry>,
    sort: S,
    list: ScrollList,
    image: Image,
    menu: Option<ScrollList>,
    core: Option<CoreSelection>,
    button_hints: Row<ButtonHint<String>>,
    pub child: Option<Box<EntryList<S>>>,
}

impl<S> EntryList<S>
where
    S: Sort,
{
    pub fn new(rect: Rect, res: Resources, sort: S) -> Result<Self> {
        let Rect { x, y, w, h } = rect;

        let styles = res.get::<Stylesheet>();

        let list = ScrollList::new(
            Rect::new(
                x + 12,
                y + 8,
                w - IMAGE_WIDTH - 12 - 12 - 24,
                h - 8 - ButtonIcon::diameter(&styles) - 8,
            ),
            Vec::new(),
            Alignment::Left,
            res.get::<Stylesheet>().ui_font.size + SELECTION_MARGIN,
        );

        let mut image = Image::empty(
            Rect::new(
                x + w as i32 - IMAGE_WIDTH as i32 - 24,
                y + 8,
                IMAGE_WIDTH,
                h - 8 - ButtonIcon::diameter(&styles) - 8,
            ),
            ImageMode::Contain,
        );
        image.set_border_radius(12);

        let mut button_hints = Row::new(
            Point::new(
                x + w as i32 - 12,
                y + h as i32 - ButtonIcon::diameter(&styles) as i32 - 8,
            ),
            Vec::with_capacity(2),
            Alignment::Right,
            12,
        );
        {
            let locale = res.get::<Locale>();

            button_hints.push(ButtonHint::new(
                Point::zero(),
                Key::A,
                locale.t("button-select"),
                Alignment::Right,
            ));
            if S::HAS_BUTTON_HINTS {
                button_hints.push(ButtonHint::new(
                    Point::zero(),
                    Key::Y,
                    sort.button_hint(&locale),
                    Alignment::Right,
                ))
            }
        }

        drop(styles);

        let mut this = Self {
            rect,
            res,
            entries: vec![],
            sort,
            list,
            image,
            menu: None,
            core: None,
            button_hints,
            child: None,
        };

        this.load_entries()?;

        Ok(this)
    }

    pub fn save(&self) -> EntryListState<S> {
        EntryListState {
            sort: self.sort.clone(),
            selected: self.list.selected(),
            child: self.child.as_ref().map(|c| Box::new(c.save())),
        }
    }

    pub fn load(rect: Rect, res: Resources, state: EntryListState<S>) -> Result<Self> {
        let mut this = Self::new(rect, res.clone(), state.sort)?;
        this.select(state.selected);
        if let Some(child) = state.child {
            this.child = Some(Box::new(Self::load(rect, res, *child)?));
        }
        Ok(this)
    }

    pub fn select(&mut self, index: usize) {
        self.list.select(index);
    }

    async fn select_entry(&mut self, commands: Sender<Command>) -> Result<()> {
        if let Some(entry) = self.entries.get_mut(self.list.selected()) {
            match entry {
                Entry::Directory(dir) => {
                    let child = EntryList::new(
                        self.rect,
                        self.res.clone(),
                        self.sort.with_directory(dir.clone()),
                    )?;
                    self.child = Some(Box::new(child));
                }
                Entry::Game(game) => {
                    let command = self
                        .res
                        .get::<ConsoleMapper>()
                        .launch_game(&self.res.get(), game)?;
                    if let Some(cmd) = command {
                        commands.send(cmd).await?;
                    }
                }
                Entry::App(app) => {
                    commands.send(app.command()).await?;
                }
            }
        }
        Ok(())
    }

    pub fn sort(&mut self, sort: S) -> Result<()> {
        self.sort = sort;
        self.load_entries()?;
        if S::HAS_BUTTON_HINTS {
            self.button_hints
                .get_mut(1)
                .unwrap()
                .set_text(self.sort.button_hint(&self.res.get::<Locale>()));
        }
        Ok(())
    }

    fn load_entries(&mut self) -> Result<()> {
        self.entries = self.sort.entries(&self.res.get(), &self.res.get())?;
        self.list.set_items(
            self.entries.iter().map(|e| e.name().to_string()).collect(),
            false,
        );
        Ok(())
    }

    fn open_menu(&mut self) -> Result<()> {
        let Rect { x, y, w, h } = self.rect;
        let styles = self.res.get::<Stylesheet>();
        let locale = self.res.get::<Locale>();

        let mut entries = vec![
            MenuEntry::Launch(None),
            MenuEntry::RemoveFromRecents,
            MenuEntry::RepopulateDatabase,
        ];

        let entry = self.entries.get(self.list.selected()).unwrap();
        match entry {
            Entry::Game(game) => {
                let cores = self
                    .res
                    .get::<ConsoleMapper>()
                    .get_console(&game.path)
                    .map(|c| c.cores.clone())
                    .unwrap_or_default();

                if !cores.is_empty() {
                    let core = game.core.to_owned().unwrap_or_else(|| cores[0].clone());
                    let i = cores.iter().position(|c| c == &core).unwrap_or_default();

                    if let MenuEntry::Launch(ref mut launch_core) = entries[0] {
                        let console_mapper = self.res.get::<ConsoleMapper>();
                        *launch_core = Some(console_mapper.get_core_name(&core));
                    }

                    self.core = Some(CoreSelection { core: i, cores });
                } else {
                    self.core = None;
                }
            }
            Entry::App(_) | Entry::Directory(_) => {}
        }

        let height = entries.len() as u32 * (styles.ui_font.size + SELECTION_MARGIN);

        let mut menu = ScrollList::new(
            Rect::new(
                x + 12 + (w as i32 - 24) / 6,
                (y + h as i32 - height as i32) / 2,
                (w - 24) * 2 / 3,
                height,
            ),
            entries.iter().map(|e| e.text(&locale)).collect(),
            Alignment::Left,
            styles.ui_font.size + SELECTION_MARGIN,
        );
        menu.set_background_color(Some(StylesheetColor::BackgroundHighlightBlend));
        self.menu = Some(menu);

        Ok(())
    }
}

#[async_trait(?Send)]
impl<S> View for EntryList<S>
where
    S: Sort,
{
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if let Some(child) = &mut self.child {
            return child.draw(display, styles);
        }

        let mut drawn = false;

        if let Some(menu) = &mut self.menu {
            if menu.should_draw() {
                let mut rect = menu.bounding_box(styles);
                rect.y -= 12;
                rect.h += 24;
                rect.x -= 24;
                rect.w += 48;
                rect = rect.intersection(&display.bounding_box().into());
                RoundedRectangle::new(
                    rect.into(),
                    CornerRadii::new(Size::new_equal((styles.ui_font.size + 8) / 2)),
                )
                .into_styled(PrimitiveStyle::with_fill(
                    StylesheetColor::BackgroundHighlightBlend.to_color(styles),
                ))
                .draw(display)?;
                menu.set_should_draw();
                menu.draw(display, styles)?;
                drawn = true;
            }
            return Ok(drawn);
        }

        drawn |= self.list.should_draw() && self.list.draw(display, styles)?;

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
            } else {
                self.image.set_path(None);
            }
        }

        if self.button_hints.should_draw() {
            display.load(Rect::new(
                0,
                display.size().height as i32 - 48,
                display.size().width,
                48,
            ))?;
            self.button_hints.set_should_draw();
            if self.button_hints.draw(display, styles)? {
                drawn = true;
            }
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        if let Some(child) = self.child.as_ref() {
            child.should_draw()
        } else {
            self.menu
                .as_ref()
                .map_or(false, common::view::View::should_draw)
                || self.list.should_draw()
                || self.image.should_draw()
                || self.button_hints.should_draw()
        }
    }

    fn set_should_draw(&mut self) {
        if let Some(child) = self.child.as_mut() {
            child.set_should_draw();
        } else {
            if let Some(menu) = self.menu.as_mut() {
                menu.set_should_draw();
            }
            self.list.set_should_draw();
            self.image.set_should_draw();
            self.button_hints.set_should_draw();
        }
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if let Some(child) = self.child.as_mut() {
            match child.handle_key_event(event, commands, bubble).await? {
                true => {
                    bubble.retain_mut(|c| match c {
                        Command::CloseView => {
                            self.child = None;
                            self.set_should_draw();
                            true
                        }
                        _ => true,
                    });
                    Ok(true)
                }
                false => Ok(false),
            }
        } else if let Some(menu) = self.menu.as_mut() {
            match event {
                KeyEvent::Pressed(Key::Left) => {
                    if let Some(core) = self.core.as_mut() {
                        let mut selected = MenuEntry::from_repr(menu.selected());
                        if let MenuEntry::Launch(ref mut launch_core) = selected {
                            core.core = core.core.saturating_sub(1);
                            let console_mapper = self.res.get::<ConsoleMapper>();
                            *launch_core =
                                Some(console_mapper.get_core_name(&core.cores[core.core]));
                            menu.set_item(menu.selected(), selected.text(&self.res.get()));
                        }
                    }
                    Ok(true) // trap tab focus
                }
                KeyEvent::Pressed(Key::Right) => {
                    if let Some(core) = self.core.as_mut() {
                        let mut selected = MenuEntry::from_repr(menu.selected());
                        if let MenuEntry::Launch(ref mut launch_core) = selected {
                            core.core = (core.core + 1).min(core.cores.len() - 1);
                            let console_mapper = self.res.get::<ConsoleMapper>();
                            *launch_core =
                                Some(console_mapper.get_core_name(&core.cores[core.core]));
                            menu.set_item(menu.selected(), selected.text(&self.res.get()));
                        }
                    }
                    Ok(true) // trap tab focus
                }
                KeyEvent::Pressed(Key::Select | Key::B) => {
                    self.menu = None;
                    commands.send(Command::Redraw).await?;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::A) => {
                    let selected = MenuEntry::from_repr(menu.selected());
                    match selected {
                        MenuEntry::Launch(_) => {
                            let entry = self.entries.get_mut(self.list.selected()).unwrap();
                            if let (Some(core), Entry::Game(game)) = (self.core.as_ref(), entry) {
                                let db = self.res.get::<Database>();
                                let core = &core.cores[core.core];
                                db.set_core(&game.path, core)?;
                                game.core = Some(core.to_string());
                            }
                            self.core = None;
                            self.select_entry(commands).await?;
                        }
                        MenuEntry::RemoveFromRecents => {
                            if let Some(Entry::Game(game)) = self.entries.get(self.list.selected())
                            {
                                if game.path.exists() {
                                    self.res.get::<Database>().reset_game(&game.path)?;
                                } else {
                                    self.res.get::<Database>().delete_game(&game.path)?;
                                }
                                self.load_entries()?;
                                commands.send(Command::Redraw).await?;
                            }
                        }
                        MenuEntry::RepopulateDatabase => {
                            commands.send(Command::Redraw).await?;
                            let toast = self.res.get::<Locale>().t("populating-database");
                            commands.send(Command::Toast(toast, None)).await?;
                            commands.send(Command::PopulateDb).await?;
                            commands
                                .send(Command::Toast(String::new(), Some(Duration::ZERO)))
                                .await?;
                            commands.send(Command::Redraw).await?;
                        }
                    }
                    self.menu = None;
                    Ok(true)
                }
                _ => menu.handle_key_event(event, commands, bubble).await,
            }
        } else {
            match event {
                KeyEvent::Pressed(Key::L2) => {
                    let selected = self.list.selected();
                    let len = self.entries.len();
                    let mut entries = self
                        .entries
                        .iter()
                        .rev()
                        .skip(len - selected)
                        .map(|e| e.name().chars().next());
                    println!("{:?}", entries.clone().collect::<Vec<_>>());
                    let Some(char) = entries.next() else {
                        self.list.select(0);
                        return Ok(true);
                    };

                    println!("entries: {:?}, char: {:?}", entries, char);
                    if let Some(i) = entries.position(|c| c != char) {
                        self.list.select(selected - i - 1);
                    } else {
                        self.list.select(0);
                    }
                    Ok(true)
                }
                KeyEvent::Pressed(Key::R2) => {
                    let selected = self.list.selected();
                    let mut entries = self
                        .entries
                        .iter()
                        .skip(selected)
                        .map(|e| e.name().chars().next());
                    let Some(char) = entries.next() else {
                        self.list.select(self.entries.len() - 1);
                        return Ok(true);
                    };

                    if let Some(i) = entries.position(|c| c != char) {
                        self.list.select(selected + 1 + i);
                    } else {
                        self.list.select(self.entries.len() - 1);
                    }
                    Ok(true)
                }
                KeyEvent::Pressed(Key::B) => {
                    bubble.push_back(Command::CloseView);
                    Ok(true)
                }
                KeyEvent::Pressed(Key::A) => {
                    self.select_entry(commands).await?;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Y) => {
                    self.sort(self.sort.next())?;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Select) => {
                    self.open_menu()?;
                    Ok(true)
                }
                _ => self.list.handle_key_event(event, commands, bubble).await,
            }
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        if let Some(child) = self.child.as_ref() {
            vec![child.as_ref() as &dyn View]
        } else {
            vec![&self.list, &self.image, &self.button_hints]
        }
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        if let Some(child) = self.child.as_mut() {
            vec![child.as_mut() as &mut dyn View]
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

enum MenuEntry {
    Launch(Option<String>),
    RemoveFromRecents,
    RepopulateDatabase,
}

impl MenuEntry {
    fn from_repr(i: usize) -> Self {
        match i {
            0 => MenuEntry::Launch(None),
            1 => MenuEntry::RemoveFromRecents,
            2 => MenuEntry::RepopulateDatabase,
            _ => unreachable!("invalid menu entry"),
        }
    }

    fn text(&self, locale: &Locale) -> String {
        match self {
            MenuEntry::Launch(core) => {
                if let Some(core) = core.as_deref() {
                    locale.ta(
                        "menu-launch-with-core",
                        &[("core".to_string(), core.into())].into_iter().collect(),
                    )
                } else {
                    locale.t("menu-launch")
                }
            }
            MenuEntry::RemoveFromRecents => locale.t("menu-remove-from-recents"),
            MenuEntry::RepopulateDatabase => locale.t("menu-repopulate-database"),
        }
    }
}
