use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::{IMAGE_SIZE, RECENT_GAMES_LIMIT, SELECTION_MARGIN};
use common::database::Database;
use common::display::Display;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::{Stylesheet, StylesheetColor};
use common::view::{ButtonHint, ButtonIcon, Image, ImageMode, Row, ScrollList, View};
use embedded_graphics::prelude::OriginDimensions;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::consoles::{ConsoleMapper, Game};

#[derive(Debug)]
pub struct Recents {
    rect: Rect,
    res: Resources,
    entries: Vec<Game>,
    sort: Sort,
    list: ScrollList,
    image: Image,
    button_hints: Row<ButtonHint<String>>,
}

impl Recents {
    pub fn new(rect: Rect, res: Resources) -> Result<Self> {
        let Rect { x, y, w, h } = rect;

        let styles = res.get::<Stylesheet>();

        let list = ScrollList::new(
            Rect::new(
                x + 12,
                y + 8,
                w - IMAGE_SIZE.w - 12 - 12 - 24,
                h - 8 - ButtonIcon::diameter(&styles),
            ),
            Vec::new(),
            Alignment::Left,
            res.get::<Stylesheet>().ui_font.size + SELECTION_MARGIN,
        );

        let mut image = Image::empty(
            Rect::new(
                x + w as i32 - IMAGE_SIZE.w as i32 - 24,
                y + 8,
                IMAGE_SIZE.w,
                IMAGE_SIZE.h,
            ),
            ImageMode::Contain,
        );
        image
            .set_background_color(StylesheetColor::Background)
            .set_border_radius(12);

        let button_hints = Row::new(
            Point::new(
                x + w as i32 - 12,
                y + h as i32 - ButtonIcon::diameter(&styles) as i32 - 8,
            ),
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
                        Key::Y,
                        Sort::LastPlayed.button_hint(&locale),
                        Alignment::Right,
                    ),
                ]
            },
            Alignment::Right,
            12,
        );

        drop(styles);

        let mut this = Self {
            rect,
            res,
            entries: Vec::new(),
            sort: Sort::LastPlayed,
            list,
            image,
            button_hints,
        };

        this.load_entries()?;

        Ok(this)
    }

    async fn select_entry(&mut self, commands: Sender<Command>) -> Result<()> {
        if let Some(entry) = self.entries.get_mut(self.list.selected()) {
            let command = self
                .res
                .get::<ConsoleMapper>()
                .launch_game(&self.res.get(), entry)?;
            if let Some(command) = command {
                commands.send(command).await?;
            }
        }
        Ok(())
    }

    fn load_entries(&mut self) -> Result<()> {
        let games = match self.sort {
            Sort::LastPlayed => self
                .res
                .get::<Database>()
                .select_last_played(RECENT_GAMES_LIMIT)?,
            Sort::MostPlayed => self
                .res
                .get::<Database>()
                .select_most_played(RECENT_GAMES_LIMIT)?,
        };

        self.entries = games
            .into_iter()
            .map(|game| Game::new(game.name, game.path))
            .collect();

        self.list.set_items(
            self.entries.iter().map(|e| e.name.to_string()).collect(),
            true,
        );

        Ok(())
    }
}

#[async_trait(?Send)]
impl View for Recents {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
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
        self.list.should_draw() || self.image.should_draw() || self.button_hints.should_draw()
    }

    fn set_should_draw(&mut self) {
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
        match event {
            KeyEvent::Pressed(Key::A) => {
                self.select_entry(commands).await?;
                Ok(true)
            }
            KeyEvent::Pressed(Key::Y) => {
                self.sort = self.sort.next();
                self.button_hints
                    .get_mut(1)
                    .unwrap()
                    .set_text(self.sort.button_hint(&self.res.get::<Locale>()));
                self.load_entries()?;
                Ok(true)
            }
            _ => self.list.handle_key_event(event, commands, bubble).await,
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.list, &self.image, &self.button_hints]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.list, &mut self.image, &mut self.button_hints]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum Sort {
    LastPlayed,
    MostPlayed,
}

impl Sort {
    fn button_hint(&self, locale: &Locale) -> String {
        match self {
            Sort::LastPlayed => locale.t("recents-sort-currently-last-played"),
            Sort::MostPlayed => locale.t("recents-sort-currently-most-played"),
        }
    }

    fn next(self) -> Self {
        match self {
            Sort::LastPlayed => Sort::MostPlayed,
            Sort::MostPlayed => Sort::LastPlayed,
        }
    }
}
