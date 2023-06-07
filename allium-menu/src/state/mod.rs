mod menu;
mod netplay;

use anyhow::Result;

use common::display;
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
    ) -> Result<()> {
        match self {
            State::Menu(state) => state.draw(display, styles),
            State::Netplay(state) => state.draw(display, styles),
        }
    }

    pub fn update(&mut self) -> Result<()> {
        match self {
            State::Menu(state) => state.update(),
            State::Netplay(state) => state.update(),
        }
    }

    pub async fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
        display: &mut <DefaultPlatform as Platform>::Display,
    ) -> Result<bool> {
        match self {
            State::Menu(state) => state.handle_key_event(key_event, display).await,
            State::Netplay(state) => state.handle_key_event(key_event),
        }
    }
}
