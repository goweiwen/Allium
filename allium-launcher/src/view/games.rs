use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Duration;
use common::command::Command;
use common::constants::ALLIUM_GAMES_DIR;
use common::database::Database;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, ButtonIcon, Row, View};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::consoles::ConsoleMapper;
use crate::entry::directory::Directory;
use crate::entry::{Entry, Sort};
use crate::view::entry_list::{EntryList, EntryListState};

pub type GamesState = EntryListState<GamesSort>;

#[derive(Debug)]
pub struct Games {
    rect: Rect,
    list: EntryList<GamesSort>,
    button_hints: Row<ButtonHint<String>>,
}

impl Games {
    pub fn new(rect: Rect, res: Resources, list: EntryList<GamesSort>) -> Result<Self> {
        let Rect { x, y, w: _, h } = rect;

        let styles = res.get::<Stylesheet>();

        let button_hints = Row::new(
            Point::new(
                x + 12,
                y + h as i32 - ButtonIcon::diameter(&styles) as i32 - 8,
            ),
            {
                let locale = res.get::<Locale>();
                vec![ButtonHint::new(
                    res.clone(),
                    Point::zero(),
                    Key::X,
                    locale.t("sort-search"),
                    Alignment::Left,
                )]
            },
            Alignment::Left,
            12,
        );

        Ok(Self {
            rect,
            list,
            button_hints,
        })
    }

    pub fn load_or_new(rect: Rect, res: Resources, state: Option<GamesState>) -> Result<Self> {
        let list = if let Some(state) = state {
            let selected = state.selected;
            let mut list = EntryList::load(rect, res.clone(), state)?;
            list.select(selected);
            list
        } else {
            EntryList::new(
                rect,
                res.clone(),
                GamesSort::Alphabetical(Directory::new(ALLIUM_GAMES_DIR.clone())),
            )?
        };

        Self::new(rect, res, list)
    }

    pub fn save(&self) -> GamesState {
        self.list.save()
    }
}

#[async_trait(?Send)]
impl View for Games {
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

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.list.should_draw() || self.button_hints.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.list.set_should_draw();
        self.button_hints.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        match event {
            KeyEvent::Pressed(Key::X) => {
                commands.send(Command::StartSearch).await?;
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
pub enum GamesSort {
    Alphabetical(Directory),
    LastPlayed(Directory),
    MostPlayed(Directory),
    Random(Directory),
}

impl GamesSort {
    pub fn directory(&self) -> &Directory {
        match self {
            GamesSort::Alphabetical(d) => d,
            GamesSort::LastPlayed(d) => d,
            GamesSort::MostPlayed(d) => d,
            GamesSort::Random(d) => d,
        }
    }
}

impl Sort for GamesSort {
    fn button_hint(&self, locale: &Locale) -> String {
        match self {
            GamesSort::Alphabetical(_) => locale.t("sort-alphabetical"),
            GamesSort::LastPlayed(_) => locale.t("sort-last-played"),
            GamesSort::MostPlayed(_) => locale.t("sort-most-played"),
            GamesSort::Random(_) => locale.t("sort-random"),
        }
    }

    fn next(&self) -> Self {
        match self {
            GamesSort::Alphabetical(d) => GamesSort::LastPlayed(d.clone()),
            GamesSort::LastPlayed(d) => GamesSort::MostPlayed(d.clone()),
            GamesSort::MostPlayed(d) => GamesSort::Random(d.clone()),
            GamesSort::Random(d) => GamesSort::Alphabetical(d.clone()),
        }
    }

    fn with_directory(&self, directory: Directory) -> Self {
        match self {
            GamesSort::Alphabetical(_) => GamesSort::Alphabetical(directory),
            GamesSort::LastPlayed(_) => GamesSort::LastPlayed(directory),
            GamesSort::MostPlayed(_) => GamesSort::MostPlayed(directory),
            GamesSort::Random(_) => GamesSort::Random(directory),
        }
    }

    fn entries(
        &self,
        database: &Database,
        console_mapper: &ConsoleMapper,
        locale: &Locale,
    ) -> Result<Vec<Entry>> {
        let mut entries = self.directory().entries(database, console_mapper, locale)?;

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
            GamesSort::Random(_) => {
                entries.shuffle(&mut rand::thread_rng());
            }
        }

        Ok(entries)
    }

    fn preserve_selection(&self) -> bool {
        !matches!(self, GamesSort::Alphabetical(_) | GamesSort::Random(_))
    }
}
