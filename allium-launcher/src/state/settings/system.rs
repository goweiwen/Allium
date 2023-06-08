use anyhow::Result;
use embedded_graphics::{prelude::*, primitives::Rectangle};

use common::stylesheet::Stylesheet;
use common::{
    display::Display,
    platform::{DefaultPlatform, KeyEvent, Platform},
};

use crate::state::settings::Settings;
use crate::state::State;
use crate::{command::AlliumCommand, state::settings::Setting};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSystemState {
    settings: Settings,
}

impl SettingsSystemState {
    pub fn new() -> Self {
        let device = DefaultPlatform::device_model();
        Self {
            settings: Settings(vec![
                Setting::string("Version", "Allium v0.3.0"),
                Setting::string("Device Model", device),
            ]),
        }
    }
}

impl State for SettingsSystemState {
    fn enter(&mut self) -> Result<()> {
        Ok(())
    }

    fn leave(&mut self) -> Result<()> {
        Ok(())
    }

    fn draw(
        &self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<()> {
        let Size { width, height } = display.size();
        display.load(Rectangle::new(
            Point::new(156, 60),
            Size::new(width, height - 46),
        ))?;

        self.settings.draw(display, styles)?;

        Ok(())
    }

    fn handle_key_event(&mut self, _key_event: KeyEvent) -> Result<(Option<AlliumCommand>, bool)> {
        Ok((None, false))
    }
}
