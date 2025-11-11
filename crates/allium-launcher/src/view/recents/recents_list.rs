use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::RECENT_GAMES_LIMIT;
use common::database::Database;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, ButtonIcon, Row, View};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::consoles::ConsoleMapper;
use crate::entry::directory::Directory;
use crate::entry::game::Game;
use crate::entry::lazy_image::LazyImage;
use crate::entry::{Entry, Sort};
use crate::view::entry_list::{EntryList, EntryListState};

pub type RecentsListState = EntryListState<RecentsSort>;

#[derive(Debug)]
pub struct RecentsList {
    rect: Rect,
    list: EntryList<RecentsSort>,
    button_hints: Row<ButtonHint<String>>,
}

impl RecentsList {
    pub fn new(rect: Rect, res: Resources, list: EntryList<RecentsSort>) -> Result<Self> {
        let Rect { x, y, w: _w, h } = rect;

        let styles = res.get::<Stylesheet>();

        let button_hints = Row::new(
            Point::new(
                x + styles.margin_x,
                y + h as i32 - ButtonIcon::diameter(&styles) as i32 - styles.margin_y,
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

        drop(styles);

        Ok(Self {
            rect,
            list,
            button_hints,
        })
    }

    pub fn load_or_new(
        rect: Rect,
        res: Resources,
        state: Option<RecentsListState>,
    ) -> Result<Self> {
        let list = if let Some(state) = state {
            EntryList::load(rect, res.clone(), state)?
        } else {
            EntryList::new(rect, res.clone(), RecentsSort::LastPlayed)?
        };

        Self::new(rect, res, list)
    }

    pub fn save(&self) -> RecentsListState {
        self.list.save()
    }
}

#[async_trait(?Send)]
impl View for RecentsList {
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
pub enum RecentsSort {
    LastPlayed,
    MostPlayed,
    Favorites,
    Random,
    Search(String),
}

impl Sort for RecentsSort {
    fn button_hint(&self, locale: &Locale) -> String {
        match self {
            RecentsSort::LastPlayed => locale.t("sort-last-played"),
            RecentsSort::MostPlayed => locale.t("sort-most-played"),
            RecentsSort::Favorites => locale.t("sort-favorites"),
            RecentsSort::Random => locale.t("sort-random"),
            RecentsSort::Search(_) => locale.t("sort-search"),
        }
    }

    fn next(&self) -> Self {
        match self {
            RecentsSort::LastPlayed => RecentsSort::MostPlayed,
            RecentsSort::MostPlayed => RecentsSort::Favorites,
            RecentsSort::Favorites => RecentsSort::Random,
            RecentsSort::Random => RecentsSort::LastPlayed,
            RecentsSort::Search(_) => RecentsSort::LastPlayed,
        }
    }

    fn with_directory(&self, _directory: Directory) -> Self {
        unimplemented!();
    }

    fn entries(
        &self,
        database: &Database,
        _console_mapper: &ConsoleMapper,
        _locale: &Locale,
    ) -> Result<Vec<Entry>> {
        let games = match self {
            RecentsSort::LastPlayed => database.select_last_played(RECENT_GAMES_LIMIT),
            RecentsSort::MostPlayed => database.select_most_played(RECENT_GAMES_LIMIT),
            RecentsSort::Favorites => database.select_favorites(RECENT_GAMES_LIMIT),
            RecentsSort::Random => database.select_random(RECENT_GAMES_LIMIT),
            RecentsSort::Search(query) => database.search(query, RECENT_GAMES_LIMIT),
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

                let image = LazyImage::from_path(&game.path, game.image);

                Entry::Game(Game {
                    name: game.name,
                    full_name,
                    path: game.path,
                    image,
                    extension,
                    core: game.core,
                    rating: game.rating,
                    release_date: game.release_date,
                    developer: game.developer,
                    publisher: game.publisher,
                    genres: game.genres,
                    favorite: game.favorite,
                    screenshot_path: game.screenshot_path,
                })
            })
            .collect())
    }

    fn preserve_selection(&self) -> bool {
        false
    }
}
