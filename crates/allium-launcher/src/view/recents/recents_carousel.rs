use std::collections::VecDeque;
use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::RECENT_GAMES_LIMIT;
use common::database::Database;
use common::display::Display;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, ButtonHints, Image, ImageMode, Label, View};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::consoles::ConsoleMapper;
use crate::entry::game::Game;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecentsCarouselState {
    pub selected: usize,
}

#[derive(Debug)]
pub struct RecentsCarousel {
    rect: Rect,
    res: Resources,
    games: Vec<Game>,
    selected: usize,
    screenshot: Image,
    game_name: Label<String>,
    button_hints: ButtonHints<String>,
    dirty: bool,
}

impl RecentsCarousel {
    pub fn new(rect: Rect, res: Resources, state: RecentsCarouselState) -> Result<Self> {
        let Rect { x, y, w, .. } = rect;

        let games = Self::load_games(&res)?;
        let selected = state.selected.min(games.len().saturating_sub(1));

        let styles = res.get::<Stylesheet>();

        let mut button_hints = {
            let locale = res.get::<Locale>();
            ButtonHints::new(
                res.clone(),
                vec![],
                vec![
                    ButtonHint::new(
                        res.clone(),
                        Point::zero(),
                        Key::A,
                        locale.t("button-select"),
                        Alignment::Right,
                    ),
                    ButtonHint::new(
                        res.clone(),
                        Point::zero(),
                        Key::X,
                        locale.t("sort-search"),
                        Alignment::Right,
                    ),
                ],
            )
        };

        let button_hints_rect = button_hints.bounding_box(&styles);
        let label_height = styles.ui.margin_y * 2 + styles.ui.ui_font.size as i32;
        // Calculate screenshot height based on where button hints start
        let available_height = (button_hints_rect.y - y) as u32;
        let screenshot_height = available_height.saturating_sub(label_height as u32);

        let mut screenshot =
            Image::empty(Rect::new(x, y, w, screenshot_height), ImageMode::Contain);
        screenshot.set_border_radius(12);
        screenshot.set_alignment(Alignment::Center);

        let game_name = Label::new(
            Point::new(
                x + w as i32 / 2,
                y + screenshot_height as i32 + styles.ui.margin_y,
            ),
            String::new(),
            Alignment::Center,
            Some(w - (styles.ui.margin_x * 2) as u32),
        );

        drop(styles);

        let mut carousel = Self {
            rect,
            res: res.clone(),
            games,
            selected,
            screenshot,
            game_name,
            button_hints,
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
            self.game_name.scroll(false);
            self.game_name.set_text(String::new());
            return Ok(());
        }

        let game = &mut self.games[self.selected];

        self.screenshot.set_path(
            game.screenshot_path
                .clone()
                .or_else(|| game.image.image().map(Path::to_owned)),
        );
        self.screenshot.set_should_draw();
        self.game_name.set_text(game.name.clone());
        self.game_name.scroll(true);
        self.button_hints.set_should_draw();

        self.dirty = true;
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
        } else if self.game_name.should_draw() {
            display.load(self.game_name.bounding_box(styles))?;
            drawn |= self.game_name.draw(display, styles)?;
        }

        if self.button_hints.should_draw() {
            drawn |= self.button_hints.draw(display, styles)?;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.dirty
            || self.screenshot.should_draw()
            || self.game_name.should_draw()
            || self.button_hints.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        self.screenshot.set_should_draw();
        self.game_name.set_should_draw();
        self.button_hints.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        _bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
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
                commands.send(Command::StartSearch).await?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.screenshot, &self.game_name, &self.button_hints]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![
            &mut self.screenshot,
            &mut self.game_name,
            &mut self.button_hints,
        ]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, point: Point) {
        self.rect.x = point.x;
        self.rect.y = point.y;
    }
}
