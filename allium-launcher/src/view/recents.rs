use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::{Command, Value};
use common::constants::{ALLIUM_GAMES_DIR, RECENT_GAMES_LIMIT};
use common::database::Database;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, ButtonIcon, Keyboard, Row, View};
use log::error;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::consoles::ConsoleMapper;
use crate::entry::directory::Directory;
use crate::entry::game::Game;
use crate::entry::{Entry, Sort};
use crate::view::entry_list::EntryList;

#[derive(Debug)]
pub struct Recents {
    res: Resources,
    rect: Rect,
    list: EntryList<RecentsSort>,
    button_hints: Row<ButtonHint<String>>,
    keyboard: Option<Keyboard>,
}

impl Recents {
    pub fn new(rect: Rect, res: Resources) -> Result<Self> {
        let Rect { x, y, w, h } = rect;

        let styles = res.get::<Stylesheet>();

        let list = EntryList::new(Rect::new(x, y, w, h), res.clone(), RecentsSort::LastPlayed)?;

        let button_hints = Row::new(
            Point::new(
                x + 12,
                y + h as i32 - ButtonIcon::diameter(&styles) as i32 - 8,
            ),
            {
                let locale = res.get::<Locale>();
                vec![ButtonHint::new(
                    Point::zero(),
                    Key::X,
                    locale.t("sort-search"),
                    Alignment::Left,
                )]
            },
            Alignment::Left,
            12,
        );

        drop(styles);

        Ok(Self {
            res,
            rect,
            list,
            button_hints,
            keyboard: None,
        })
    }

    pub fn search(&mut self) {
        self.keyboard = Some(Keyboard::new(self.res.clone(), String::new(), false));
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

        if self.list.should_draw() {
            drawn |= self.list.should_draw() && self.list.draw(display, styles)?;
            self.button_hints.set_should_draw();
        }
        drawn |= self.button_hints.should_draw() && self.button_hints.draw(display, styles)?;

        if let Some(keyboard) = self.keyboard.as_mut() {
            if drawn {
                keyboard.set_should_draw();
            }
            drawn |= keyboard.should_draw() && keyboard.draw(display, styles)?;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.list.should_draw()
            || self.button_hints.should_draw()
            || self.keyboard.as_ref().map_or(false, |k| k.should_draw())
    }

    fn set_should_draw(&mut self) {
        self.list.set_should_draw();
        self.button_hints.set_should_draw();
        if let Some(keyboard) = self.keyboard.as_mut() {
            keyboard.set_should_draw();
        }
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if let Some(keyboard) = self.keyboard.as_mut() {
            if keyboard
                .handle_key_event(event, commands.clone(), bubble)
                .await?
            {
                bubble.retain_mut(|c| match c {
                    Command::ValueChanged(_, val) => {
                        if let Value::String(val) = val {
                            if let Err(e) = self.list.sort(RecentsSort::Search(val.clone())) {
                                error!("Failed to sort: {}", e);
                            }
                        }
                        false
                    }
                    Command::CloseView => {
                        self.keyboard = None;
                        false
                    }
                    _ => true,
                });
                return Ok(true);
            }
        }

        match event {
            KeyEvent::Pressed(Key::X) => {
                if self.keyboard.is_none() {
                    self.search();
                } else {
                    self.keyboard = None;
                    self.list.sort(RecentsSort::LastPlayed)?;
                    commands.send(Command::Redraw).await?;
                }
                return Ok(true);
            }
            _ => self.list.handle_key_event(event, commands, bubble).await,
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.list]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.list]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecentsSort {
    LastPlayed,
    MostPlayed,
    Search(String),
}

impl Sort for RecentsSort {
    fn button_hint(&self, locale: &Locale) -> String {
        match self {
            RecentsSort::LastPlayed => locale.t("sort-last-played"),
            RecentsSort::MostPlayed => locale.t("sort-most-played"),
            RecentsSort::Search(_) => locale.t("sort-search"),
        }
    }

    fn next(&self) -> Self {
        match self {
            RecentsSort::LastPlayed => RecentsSort::MostPlayed,
            RecentsSort::MostPlayed => RecentsSort::LastPlayed,
            RecentsSort::Search(_) => RecentsSort::LastPlayed,
        }
    }

    fn with_directory(&self, _directory: Directory) -> Self {
        unimplemented!();
    }

    fn entries(&self, database: &Database, console_mapper: &ConsoleMapper) -> Result<Vec<Entry>> {
        let games = match self {
            RecentsSort::LastPlayed => database.select_last_played(RECENT_GAMES_LIMIT),
            RecentsSort::MostPlayed => database.select_most_played(RECENT_GAMES_LIMIT),
            RecentsSort::Search(query) => {
                if !database.has_indexed()? {
                    Directory::new(ALLIUM_GAMES_DIR.clone())
                        .populate_db(database, console_mapper)?;
                }
                database.search(query, RECENT_GAMES_LIMIT)
            }
        };

        let games = match games {
            Ok(games) => games,
            Err(err) => {
                log::error!("Failed to select games: {}", err);
                return Err(err);
            }
        };

        Ok(games
            .into_iter()
            .map(|game| {
                let extension = game
                    .path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or_default()
                    .to_owned();

                let full_name = game.name.clone();

                Entry::Game(Game {
                    name: game.name,
                    full_name,
                    path: game.path,
                    image: Some(game.image),
                    extension,
                })
            })
            .collect())
    }
}
