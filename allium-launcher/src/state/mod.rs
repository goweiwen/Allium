mod games;
mod recents;
mod settings;

use anyhow::Result;

use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::stylesheet::Stylesheet;

use crate::command::AlliumCommand;

pub use self::games::GamesState;
pub use self::recents::RecentsState;
pub use self::settings::SettingsState;

pub trait State {
    fn enter(&mut self) -> Result<()>;
    fn leave(&mut self) -> Result<()>;
    fn draw(
        &self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<()>;
    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<(Option<AlliumCommand>, bool)>;
}
