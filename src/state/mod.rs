mod games;
mod recents;
mod settings;

use anyhow::Result;
use embedded_font::FontTextStyleBuilder;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text};
use embedded_graphics::Drawable;
use serde::{Deserialize, Serialize};

use crate::allium::Stylesheet;
use crate::battery::Battery;
use crate::platform::{DefaultPlatform, KeyEvent, Platform};

pub use self::games::GamesState;
pub use self::recents::RecentsState;
pub use self::settings::SettingsState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum State {
    Games(GamesState),
    Recents(RecentsState),
    Settings(SettingsState),
}

impl State {
    pub fn new() -> Result<State> {
        Ok(State::Games(GamesState::new()?))
    }

    pub fn next(&self) -> Result<Option<State>> {
        match self {
            State::Games(_) => Ok(Some(State::Recents(RecentsState::new()))),
            State::Recents(_) => Ok(Some(State::Settings(SettingsState::new()))),
            State::Settings(_) => Ok(None),
        }
    }

    pub fn prev(&self) -> Result<Option<State>> {
        match self {
            State::Games(_) => Ok(None),
            State::Recents(_) => Ok(Some(State::Games(GamesState::new()?))),
            State::Settings(_) => Ok(Some(State::Recents(RecentsState::new()))),
        }
    }

    pub fn enter(&mut self) -> Result<()> {
        match self {
            State::Games(state) => state.enter(),
            State::Recents(state) => state.enter(),
            State::Settings(state) => state.enter(),
        }
    }

    pub fn leave(&mut self) -> Result<()> {
        match self {
            State::Games(state) => state.leave(),
            State::Recents(state) => state.leave(),
            State::Settings(state) => state.leave(),
        }
    }

    pub fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
        battery: &<DefaultPlatform as Platform>::Battery,
    ) -> Result<()> {
        let Size { width, height: _ } = display.size();

        let text_style = FontTextStyleBuilder::new(styles.ui_font.clone())
            .font_size(styles.ui_font_size)
            .text_color(styles.fg_color)
            .build();

        let primary_style = FontTextStyleBuilder::new(styles.ui_font.clone())
            .font_size(styles.ui_font_size)
            .text_color(styles.primary)
            .build();

        // Draw battery percentage
        if battery.charging() {
            Text::with_alignment(
                &format!("Charging: {}%", battery.percentage()),
                Point {
                    x: width as i32 - 8,
                    y: 8,
                },
                text_style.clone(),
                Alignment::Right,
            )
            .draw(display)?;
        } else {
            Text::with_alignment(
                &format!("{}%", battery.percentage()),
                Point {
                    x: width as i32 - 8,
                    y: 8,
                },
                text_style.clone(),
                Alignment::Right,
            )
            .draw(display)?;
        }

        // Draw header navigation
        let mut x = 12;
        let selected = match self {
            State::Games(_) => 0,
            State::Recents(_) => 1,
            State::Settings(_) => 2,
        };
        for (i, text) in ["Games", "Recents", "Settings"].iter().enumerate() {
            let text = Text::with_alignment(
                text,
                Point { x, y: 8 },
                if i == selected {
                    primary_style.clone()
                } else {
                    text_style.clone()
                },
                Alignment::Left,
            );
            x += text.bounding_box().size.width as i32 + 12;
            text.draw(display)?;
        }

        match self {
            State::Games(state) => state.draw(display, styles),
            State::Recents(state) => state.draw(display, styles),
            State::Settings(state) => state.draw(display, styles),
        }
    }

    pub fn update(&mut self) -> Result<()> {
        match self {
            State::Games(state) => state.update(),
            State::Recents(state) => state.update(),
            State::Settings(state) => state.update(),
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<bool> {
        match self {
            State::Games(state) => state.handle_key_event(key_event),
            State::Recents(state) => state.handle_key_event(key_event),
            State::Settings(state) => state.handle_key_event(key_event),
        }
    }
}
