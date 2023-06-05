mod menu;
mod netplay;

use anyhow::Result;
use embedded_font::FontTextStyleBuilder;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text};
use embedded_graphics::Drawable;

use common::battery::Battery;
use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::stylesheet::Stylesheet;

pub use self::menu::MenuState;
pub use self::netplay::NetplayState;

#[derive(Debug, Clone)]
pub enum State {
    Menu(MenuState),
    Netplay(NetplayState),
}

impl State {
    pub fn new() -> Result<State> {
        Ok(State::Menu(MenuState::new()?))
    }

    pub fn next(&self) -> Result<Option<State>> {
        match self {
            State::Menu(_) => Ok(Some(State::Netplay(NetplayState::new()))),
            State::Netplay(_) => Ok(None),
        }
    }

    pub fn prev(&self) -> Result<Option<State>> {
        match self {
            State::Menu(_) => Ok(None),
            State::Netplay(_) => Ok(Some(State::Menu(MenuState::new()?))),
        }
    }

    pub fn enter(&mut self) -> Result<()> {
        match self {
            State::Menu(state) => state.enter(),
            State::Netplay(state) => state.enter(),
        }
    }

    pub fn leave(&mut self) -> Result<()> {
        match self {
            State::Menu(state) => state.leave(),
            State::Netplay(state) => state.leave(),
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

        // Draw game name
        let text = Text::with_alignment(
            "Dragon Quest Warriors II",
            Point { x: 12, y: 8 },
            primary_style.clone(),
            Alignment::Left,
        );
        text.draw(display)?;

        match self {
            State::Menu(state) => state.draw(display, styles),
            State::Netplay(state) => state.draw(display, styles),
        }?;

        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        match self {
            State::Menu(state) => state.update(),
            State::Netplay(state) => state.update(),
        }
    }

    pub async fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<bool> {
        match self {
            State::Menu(state) => state.handle_key_event(key_event).await,
            State::Netplay(state) => state.handle_key_event(key_event),
        }
    }
}
