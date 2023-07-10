use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::RECENT_GAMES_LIMIT;
use common::database::Database;
use common::geom::{Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::View;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::consoles::ConsoleMapper;
use crate::entry::directory::Directory;
use crate::entry::game::Game;
use crate::entry::{Entry, Sort};
use crate::view::entry_list::EntryList;

#[derive(Debug)]
pub struct Recents {
    rect: Rect,
    list: EntryList<RecentsSort>,
}

impl Recents {
    pub fn new(rect: Rect, res: Resources) -> Result<Self> {
        let Rect { x, y, w, h } = rect;

        let styles = res.get::<Stylesheet>();

        let list = EntryList::new(Rect::new(x, y, w, h), res.clone(), RecentsSort::LastPlayed)?;

        drop(styles);

        Ok(Self { rect, list })
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

        drawn |= self.list.should_draw() && self.list.draw(display, styles)?;

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.list.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.list.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        self.list.handle_key_event(event, commands, bubble).await
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RecentsSort {
    LastPlayed,
    MostPlayed,
}

impl Sort for RecentsSort {
    fn button_hint(&self, locale: &Locale) -> String {
        match self {
            RecentsSort::LastPlayed => locale.t("sort-last-played"),
            RecentsSort::MostPlayed => locale.t("sort-most-played"),
        }
    }

    fn next(&self) -> Self {
        match self {
            RecentsSort::LastPlayed => RecentsSort::MostPlayed,
            RecentsSort::MostPlayed => RecentsSort::LastPlayed,
        }
    }

    fn with_directory(&self, _directory: Directory) -> Self {
        unimplemented!();
    }

    fn entries(&self, database: &Database, _console_mapper: &ConsoleMapper) -> Result<Vec<Entry>> {
        let games = match self {
            RecentsSort::LastPlayed => database.select_last_played(RECENT_GAMES_LIMIT),
            RecentsSort::MostPlayed => database.select_most_played(RECENT_GAMES_LIMIT),
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
