mod games;
mod recents;
mod settings;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::stylesheet::Stylesheet;

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
    ) -> Result<()> {
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
