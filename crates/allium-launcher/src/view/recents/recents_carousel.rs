use std::collections::VecDeque;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use common::command::{Command, Value};
use common::constants::RECENT_GAMES_LIMIT;
use common::database::Database;
use common::display::Display;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, ButtonIcon, Image, ImageMode, Keyboard, Label, Row, View};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::consoles::ConsoleMapper;
use crate::entry::game::Game;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentsCarouselState {
    pub selected: usize,
}

impl Default for RecentsCarouselState {
    fn default() -> Self {
        Self { selected: 0 }
    }
}

#[derive(Debug)]
pub struct RecentsCarousel {
    rect: Rect,
    res: Resources,
    games: Vec<Game>,
    selected: usize,
    screenshot: Image,
    game_name: Label<String>,
    button_hints: Row<ButtonHint<String>>,
    keyboard: Option<Keyboard>,
    dirty: bool,
}

impl RecentsCarousel {
    pub fn new(rect: Rect, res: Resources, state: RecentsCarouselState) -> Result<Self> {
        let Rect { x, y, w, h } = rect;

        let games = Self::load_games(&res)?;
        let selected = state.selected.min(games.len().saturating_sub(1));

        let styles = res.get::<Stylesheet>();
        let y_margin = 8;
        let x_margin = 12;
        let ui_font_size = styles.ui_font.size as i32;
        let bottom_area_height = (y_margin * 3) + (ui_font_size * 2);
        let screenshot_height = h.saturating_sub((bottom_area_height + y_margin) as u32);

        let mut screenshot = Image::empty(
            Rect::new(x, y + y_margin, w, screenshot_height),
            ImageMode::Contain,
        );
        screenshot.set_alignment(Alignment::Center);

        let game_name = Label::new(
            Point::new(
                x + w as i32 / 2,
                y + y_margin + screenshot_height as i32 + y_margin,
            ),
            String::new(),
            Alignment::Center,
            Some(w - (x_margin * 2) as u32),
        );

        let button_hints = Row::new(
            Point::new(
                x + w as i32 - 12,
                y + h as i32 - ButtonIcon::diameter(&styles) as i32 - 8,
            ),
            {
                let locale = res.get::<Locale>();
                vec![ButtonHint::new(
                    res.clone(),
                    Point::zero(),
                    Key::A,
                    locale.t("button-select"),
                    Alignment::Right,
                )]
            },
            Alignment::Right,
            12,
        );

        drop(styles);

        let mut carousel = Self {
            rect,
            res,
            games,
            selected,
            screenshot,
            game_name,
            button_hints,
            keyboard: None,
            dirty: true,
        };

        carousel.game_name.scroll(true);
        carousel.update_current_game()?;

        Ok(carousel)
    }

    pub fn load_or_new(
        rect: Rect,
        res: Resources,
        state: Option<RecentsCarouselState>,
    ) -> Result<Self> {
        let state = state.unwrap_or_default();
        Self::new(rect, res, state)
    }

    fn load_games(res: &Resources) -> Result<Vec<Game>> {
        let database = res.get::<Database>();
        let db_games = database.select_last_played(RECENT_GAMES_LIMIT)?;

        let mut games = Vec::new();

        for game in db_games {
            let extension = game
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default()
                .to_owned();

            let image =
                crate::entry::lazy_image::LazyImage::from_path(&game.path, game.image.clone());

            games.push(Game {
                name: game.name.clone(),
                full_name: game.name,
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
            });
        }

        Ok(games)
    }

    fn update_current_game(&mut self) -> Result<()> {
        if self.games.is_empty() {
            self.screenshot.set_path(None);
            self.game_name.set_text(String::new());
            return Ok(());
        }

        let game = &self.games[self.selected];

        self.screenshot.set_path(game.screenshot_path.clone());
        self.screenshot.set_should_draw();
        self.game_name.set_text(game.name.clone());
        self.button_hints.set_should_draw();

        self.dirty = true;
        Ok(())
    }

    pub fn start_search(&mut self) {
        self.keyboard = Some(Keyboard::new(self.res.clone(), String::new(), false));
    }

    pub fn search(&mut self, _query: String) -> Result<()> {
        Ok(())
    }

    pub async fn try_search(&mut self, commands: Sender<Command>, query: String) -> Result<()> {
        if !self.res.get::<Database>().has_indexed()? {
            let toast = self.res.get::<Locale>().t("populating-database");
            commands.send(Command::Toast(toast, None)).await?;
            commands.send(Command::PopulateDb).await?;
            commands
                .send(Command::Toast(String::new(), Some(Duration::ZERO)))
                .await?;
        }

        commands.send(Command::Search(query)).await?;

        Ok(())
    }

    pub fn save(&self) -> RecentsCarouselState {
        RecentsCarouselState { selected: 0 }
    }

    fn navigate_up(&mut self) -> Result<()> {
        if self.selected > 0 {
            self.selected -= 1;
            self.update_current_game()?;
        }
        Ok(())
    }

    fn navigate_down(&mut self) -> Result<()> {
        if self.selected < self.games.len().saturating_sub(1) {
            self.selected += 1;
            self.update_current_game()?;
        }
        Ok(())
    }

    async fn launch_game(&mut self, commands: Sender<Command>) -> Result<()> {
        if let Some(game) = self.games.get_mut(self.selected) {
            let command =
                self.res
                    .get::<ConsoleMapper>()
                    .launch_game(&self.res.get(), game, false)?;
            if let Some(cmd) = command {
                commands.send(cmd).await?;
            }
        }
        Ok(())
    }
}

#[async_trait(?Send)]
impl View for RecentsCarousel {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let mut drawn = false;

        if self.dirty {
            display.load(self.rect)?;
            self.dirty = false;
            drawn = true;
        }

        if self.screenshot.should_draw() {
            drawn |= self.screenshot.draw(display, styles)?;
        }

        if self.games.is_empty() {
            let locale = self.res.get::<Locale>();
            let mut empty_label = Label::new(
                Point::new(
                    self.rect.x + self.rect.w as i32 / 2,
                    self.rect.y + self.rect.h as i32 / 2,
                ),
                locale.t("no-recent-games"),
                Alignment::Center,
                None,
            );
            drawn |= empty_label.draw(display, styles)?;
        } else {
            if self.game_name.should_draw() {
                drawn |= self.game_name.draw(display, styles)?;
            }
        }

        if self.button_hints.should_draw() {
            drawn |= self.button_hints.draw(display, styles)?;
        }

        if let Some(keyboard) = self.keyboard.as_mut() {
            if drawn {
                keyboard.set_should_draw();
            }
            drawn |= keyboard.should_draw() && keyboard.draw(display, styles)?;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.dirty
            || self.screenshot.should_draw()
            || self.game_name.should_draw()
            || self.button_hints.should_draw()
            || self.keyboard.as_ref().is_some_and(|k| k.should_draw())
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        self.screenshot.set_should_draw();
        self.game_name.set_should_draw();
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
        if let Some(keyboard) = self.keyboard.as_mut()
            && keyboard
                .handle_key_event(event, commands.clone(), bubble)
                .await?
        {
            let mut query = None;
            bubble.retain_mut(|c| match c {
                Command::ValueChanged(_, val) => {
                    if let Value::String(val) = val {
                        query = Some(val.clone());
                    }
                    false
                }
                Command::CloseView => {
                    self.keyboard = None;
                    false
                }
                _ => true,
            });
            if let Some(query) = query {
                self.try_search(commands, query).await?;
            }
            return Ok(true);
        }

        match event {
            KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                self.navigate_up()?;
                Ok(true)
            }
            KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                self.navigate_down()?;
                Ok(true)
            }
            KeyEvent::Pressed(Key::A) => {
                self.launch_game(commands).await?;
                Ok(true)
            }
            KeyEvent::Pressed(Key::X) => {
                if self.keyboard.is_none() {
                    self.start_search();
                } else {
                    self.keyboard = None;
                    commands.send(Command::Redraw).await?;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, point: Point) {
        self.rect.x = point.x;
        self.rect.y = point.y;
    }
}
