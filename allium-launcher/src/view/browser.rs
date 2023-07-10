use std::collections::VecDeque;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Duration;
use common::command::Command;
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
use crate::entry::{Entry, Sort};
use crate::view::entry_list::{EntryList, EntryListState};

pub type BrowserState = EntryListState<GamesSort>;

#[derive(Debug, Clone)]
pub struct Browser {
    rect: Rect,
    list: EntryList<GamesSort>,
}

impl Browser {
    pub fn new(rect: Rect, res: Resources, directory: Directory, selected: usize) -> Result<Self> {
        let Rect { x, y, w, h } = rect;

        let mut list = EntryList::new(
            Rect::new(x, y, w, h),
            res,
            GamesSort::Alphabetical(directory),
        )?;
        list.select(selected);

        Ok(Self { rect, list })
    }

    pub fn load(rect: Rect, res: Resources, state: BrowserState) -> Result<Self> {
        let Rect {
            x: _,
            y: _,
            w: _,
            h: _,
        } = rect;
        let list = EntryList::load(rect, res, state)?;
        Ok(Self { rect, list })
    }

    pub fn save(&self) -> BrowserState {
        self.list.save()
    }
}

#[async_trait(?Send)]
impl View for Browser {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GamesSort {
    Alphabetical(Directory),
    LastPlayed(Directory),
    MostPlayed(Directory),
}

impl GamesSort {
    pub fn directory(&self) -> &Directory {
        match self {
            GamesSort::Alphabetical(d) => d,
            GamesSort::LastPlayed(d) => d,
            GamesSort::MostPlayed(d) => d,
        }
    }
}

impl Sort for GamesSort {
    fn button_hint(&self, locale: &Locale) -> String {
        match self {
            GamesSort::Alphabetical(_) => locale.t("sort-alphabetical"),
            GamesSort::LastPlayed(_) => locale.t("sort-last-played"),
            GamesSort::MostPlayed(_) => locale.t("sort-most-played"),
        }
    }

    fn next(&self) -> Self {
        match self {
            GamesSort::Alphabetical(d) => GamesSort::LastPlayed(d.clone()),
            GamesSort::LastPlayed(d) => GamesSort::MostPlayed(d.clone()),
            GamesSort::MostPlayed(d) => GamesSort::Alphabetical(d.clone()),
        }
    }

    fn with_directory(&self, directory: Directory) -> Self {
        match self {
            GamesSort::Alphabetical(_) => GamesSort::Alphabetical(directory),
            GamesSort::LastPlayed(_) => GamesSort::LastPlayed(directory),
            GamesSort::MostPlayed(_) => GamesSort::MostPlayed(directory),
        }
    }

    fn entries(&self, database: &Database, console_mapper: &ConsoleMapper) -> Result<Vec<Entry>> {
        let directory = self.directory();

        let gamelist = directory.path.join("gamelist.xml");
        if gamelist.exists() {
            return directory.parse_game_list(&gamelist);
        }

        let gamelist = directory.path.join("miyoogamelist.xml");
        if gamelist.exists() {
            return directory.parse_game_list(&gamelist);
        }

        let mut entries: Vec<_> = std::fs::read_dir(&directory.path)
            .map_err(|e| anyhow!("Failed to open directory: {:?}, {}", &directory.path, e))?
            .filter_map(std::result::Result::ok)
            .filter_map(|entry| match Entry::new(entry.path(), console_mapper) {
                Ok(Some(entry)) => Some(entry),
                _ => None,
            })
            .collect();

        match self {
            GamesSort::Alphabetical(_) => {
                entries.sort_unstable();
            }
            GamesSort::LastPlayed(_) => {
                // With this current implementation, apps will appear before games.
                // TOOD: think about whether this is OK?
                let mut games = Vec::with_capacity(entries.len());
                let mut i = 0;
                while i < entries.len() {
                    if matches!(entries[i], Entry::Game(_)) {
                        match entries.remove(i) {
                            Entry::Game(game) => games.push(game),
                            _ => unreachable!(),
                        }
                    } else {
                        i += 1;
                    }
                }

                let db_games = database
                    .select_games(&games.iter().map(|g| g.path.as_path()).collect::<Vec<_>>())?;

                let mut games = games.into_iter().zip(db_games).collect::<Vec<_>>();
                games.sort_unstable_by_key(|(_, db_game)| {
                    db_game.as_ref().map(|g| -g.last_played).unwrap_or_default()
                });
                entries.retain(|e| matches!(e, Entry::Directory(_) | Entry::App(_)));
                entries.sort_unstable();
                entries.extend(games.into_iter().map(|(game, _)| Entry::Game(game)));
            }
            GamesSort::MostPlayed(_) => {
                let mut games = Vec::with_capacity(entries.len());
                let mut i = 0;
                while i < entries.len() {
                    if matches!(entries[i], Entry::Game(_)) {
                        match entries.remove(i) {
                            Entry::Game(game) => games.push(game),
                            _ => unreachable!(),
                        }
                    } else {
                        i += 1;
                    }
                }

                let db_games = database
                    .select_games(&games.iter().map(|g| g.path.as_path()).collect::<Vec<_>>())?;

                let mut games = games.into_iter().zip(db_games).collect::<Vec<_>>();
                games.sort_unstable_by_key(|(_, db_game)| {
                    db_game
                        .as_ref()
                        .map(|g| -g.play_time)
                        .unwrap_or(Duration::zero())
                });
                entries.retain(|e| matches!(e, Entry::Directory(_) | Entry::App(_)));
                entries.sort_unstable();
                entries.extend(games.into_iter().map(|(game, _)| Entry::Game(game)));
            }
        }

        Ok(entries)
    }
}
