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
use embedded_graphics::prelude::{OriginDimensions, Size};
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

#[derive(Debug, Clone)]
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
            true,
        );
        Ok(())
    }

    fn open_menu(&mut self) {
        let Rect { x, y, w, h } = self.rect;
        let styles = self.res.get::<Stylesheet>();
        let locale = self.res.get::<Locale>();

        let labels = vec![
            locale.t("menu-launch"),
            locale.t("menu-remove-from-recents"),
            locale.t("menu-repopulate-database"),
        ];

        let height = labels.len() as u32 * (styles.ui_font.size + SELECTION_MARGIN);

        let mut menu = ScrollList::new(
            Rect::new(
                x + 12 + (w as i32 - 24) / 6,
                (y + h as i32 - height as i32) / 2,
                (w - 24) * 2 / 3,
                height,
            ),
            labels,
            Alignment::Center,
            styles.ui_font.size + SELECTION_MARGIN,
        );
        menu.set_background_color(Some(StylesheetColor::BackgroundHighlightBlend));
        self.menu = Some(menu);
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
                let mut rect = menu
                    .children_mut()
                    .iter_mut()
                    .map(|v| v.bounding_box(styles))
                    .reduce(|acc, r| acc.union(&r))
                    .unwrap_or_default();
                rect.y -= 12;
                rect.h += 24;
                rect.x -= 24;
                rect.w += 48;
                RoundedRectangle::new(
                    rect.into(),
                    CornerRadii::new(Size::new_equal((styles.ui_font.size + 8) / 2)),
                )
                .into_styled(PrimitiveStyle::with_fill(
                    StylesheetColor::BackgroundHighlightBlend.to_color(styles),
                ))
                .draw(display)?;
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
                KeyEvent::Pressed(Key::Select | Key::B) => {
                    self.menu = None;
                    commands.send(Command::Redraw).await?;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::A) => match menu.selected() {
                    0 => {
                        self.select_entry(commands).await?;
                        self.menu = None;
                        Ok(true)
                    }
                    1 => {
                        if let Some(Entry::Game(game)) = self.entries.get(self.list.selected()) {
                            self.res.get::<Database>().delete_game(&game.path)?;
                            self.load_entries()?;
                            commands.send(Command::Redraw).await?;
                            self.menu = None;
                        }
                        Ok(true)
                    }
                    2 => {
                        commands.send(Command::Redraw).await?;
                        self.menu = None;
                        let toast = self.res.get::<Locale>().t("populating-database");
                        commands.send(Command::Toast(toast, None)).await?;
                        commands.send(Command::PopulateDb).await?;
                        commands
                            .send(Command::Toast(String::new(), Some(Duration::ZERO)))
                            .await?;
                        commands.send(Command::Redraw).await?;
                        Ok(true)
                    }
                    _ => unreachable!("invalid menu selection"),
                },
                _ => menu.handle_key_event(event, commands, bubble).await,
            }
        } else {
            match event {
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
                    self.open_menu();
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
