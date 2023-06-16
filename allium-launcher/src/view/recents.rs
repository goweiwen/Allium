use std::collections::VecDeque;
use std::rc::Rc;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::{BUTTON_DIAMETER, IMAGE_SIZE, RECENT_GAMES_LIMIT, SELECTION_HEIGHT};
use common::database::Database;
use common::geom::{Alignment, Point, Rect};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::{Stylesheet, StylesheetColor};
use common::view::{ButtonHint, Image, ImageMode, Row, ScrollList, View};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::devices::{DeviceMapper, Game};

#[derive(Debug, Serialize, Deserialize)]
pub struct Recents {
    rect: Rect,
    entries: Vec<Game>,
    sort: Sort,
    list: ScrollList,
    image: Image,
    button_hints: Row<ButtonHint<String>>,
    #[serde(skip)]
    database: Database,
    #[serde(skip)]
    device_mapper: Option<Rc<DeviceMapper>>,
}

impl Recents {
    pub fn new(rect: Rect) -> Result<Self> {
        let Rect { x, y, w, h } = rect;

        let list = ScrollList::new(
            Rect::new(x + 12, y + 8, w - IMAGE_SIZE.width - 12 - 12 - 24, h - 16),
            Vec::new(),
            Alignment::Left,
            SELECTION_HEIGHT,
        );

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
            vec![
                ButtonHint::new(Point::zero(), Key::A, "Select".to_owned(), Alignment::Right),
                ButtonHint::new(
                    Point::zero(),
                    Key::Y,
                    Sort::LastPlayed.button_hint().to_string(),
                    Alignment::Right,
                ),
            ],
            Alignment::Right,
            12,
        );

        Ok(Self {
            rect,
            entries: Vec::new(),
            sort: Sort::LastPlayed,
            list,
            image,
            button_hints,
            database: Default::default(),
            device_mapper: None,
        })
    }

    pub fn init(&mut self, database: Database, device_mapper: Rc<DeviceMapper>) -> Result<()> {
        self.database = database;
        self.device_mapper = Some(device_mapper);
        self.load_entries()?;
        Ok(())
    }

    async fn select_entry(&mut self, commands: Sender<Command>) -> Result<()> {
        let entry = &mut self.entries[self.list.selected()];

        if let Some(command) = self
            .device_mapper
            .as_ref()
            .unwrap()
            .launch_game(&self.database, entry)?
        {
            commands.send(command).await?;
        }

        Ok(())
    }

    fn load_entries(&mut self) -> Result<()> {
        let games = match self.sort {
            Sort::LastPlayed => self.database.select_last_played(RECENT_GAMES_LIMIT)?,
            Sort::MostPlayed => self.database.select_most_played(RECENT_GAMES_LIMIT)?,
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

        if self.button_hints.should_draw() && self.button_hints.draw(display, styles)? {
            drawn = true;
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
                    .set_text(self.sort.button_hint().to_owned());
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
    fn button_hint(&self) -> &'static str {
        match self {
            Sort::LastPlayed => "Sort: Last Played",
            Sort::MostPlayed => "Sort: Most Played",
        }
    }

    fn next(self) -> Self {
        match self {
            Sort::LastPlayed => Sort::MostPlayed,
            Sort::MostPlayed => Sort::LastPlayed,
        }
    }
}
