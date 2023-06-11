use anyhow::Result;
use common::constants::{SELECTION_HEIGHT, SELECTION_MARGIN};
use common::platform::Key;
use embedded_graphics::{prelude::*, primitives::Rectangle};

use common::stylesheet::Stylesheet;
use common::{
    display::Display,
    platform::{DefaultPlatform, KeyEvent, Platform},
};

use crate::state::settings::Settings;
use crate::state::State;
use crate::{command::AlliumCommand, state::settings::Setting};

#[derive(Debug, Clone)]
pub struct SettingsSystemState {
    settings: Settings,
    selected: usize,
}

impl SettingsSystemState {
    pub fn new() -> Self {
        let device = DefaultPlatform::device_model();
        Self {
            settings: Settings(vec![
                Setting::string("Version", "Allium v0.3.0"),
                Setting::string("Device Model", device),
            ]),
            selected: 0,
        }
    }
}

impl Default for SettingsSystemState {
    fn default() -> Self {
        Self::new()
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
        let (x, y) = (156, 58);
        display.load(Rectangle::new(
            Point::new(x - 12, y - 4),
            Size::new(484, (SELECTION_HEIGHT + SELECTION_MARGIN) * 2),
        ))?;

        self.settings
            .draw(display, styles, self.selected, false, 460)?;

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<(Option<AlliumCommand>, bool)> {
        match key_event {
            KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                self.selected = (self.selected as isize - 1)
                    .rem_euclid(self.settings.0.len() as isize)
                    as usize;
                Ok((None, true))
            }
            KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                self.selected = (self.selected as isize + 1)
                    .rem_euclid(self.settings.0.len() as isize)
                    as usize;
                Ok((None, true))
            }
            _ => Ok((None, false)),
        }
    }
}
