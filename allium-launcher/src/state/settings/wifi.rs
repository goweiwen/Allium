use std::fs::{self, File};
use std::io::Write;

use anyhow::Result;
use common::constants::ALLIUM_WIFI_SETTINGS;
use common::platform::Key;
use embedded_graphics::{prelude::*, primitives::Rectangle};
use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumIter, FromRepr, IntoEnumIterator};

use common::stylesheet::Stylesheet;
use common::{
    display::Display,
    platform::{DefaultPlatform, KeyEvent, Platform},
};
use tracing::{debug, warn};

use crate::state::settings::Settings;
use crate::state::State;
use crate::{command::AlliumCommand, state::settings::Setting};

#[derive(Debug, Clone)]
pub struct SettingsWiFiState {
    settings: WiFiSettings,
    selected: usize,
}

impl SettingsWiFiState {
    pub fn new() -> Result<Self> {
        Ok(Self {
            settings: WiFiSettings::load()?,
            selected: 0,
        })
    }

    pub fn select_entry(&mut self, index: usize) -> Result<Option<AlliumCommand>> {
        match WiFiSetting::from_repr(index) {
            Some(WiFiSetting::WiFi) => self.settings.wifi = !self.settings.wifi,
            Some(WiFiSetting::Telnet) => self.settings.telnet = !self.settings.telnet,
            Some(WiFiSetting::Ftp) => self.settings.ftp = !self.settings.ftp,
            None => panic!("Invalid wifi setting index"),
        }
        self.settings.save()?;
        Ok(None)
    }
}

impl Default for SettingsWiFiState {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl State for SettingsWiFiState {
    fn enter(&mut self) -> Result<()> {
        Ok(())
    }

    fn leave(&mut self) -> Result<()> {
        Ok(())
    }

    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<()> {
        let Size { width, height } = display.size();
        display.load(Rectangle::new(
            Point::new(146 - 12, 58 - 4),
            Size::new(width - 156 - 12, height - 58 - 4),
        ))?;

        let settings = Settings(
            WiFiSetting::iter()
                .map(|s| s.setting(&self.settings))
                .collect(),
        );

        settings.draw(display, styles, self.selected, false, 470)?;

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<(Option<AlliumCommand>, bool)> {
        match key_event {
            KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                self.selected =
                    (self.selected as isize - 1).rem_euclid(WiFiSetting::COUNT as isize) as usize;
                Ok((None, true))
            }
            KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                self.selected =
                    (self.selected as isize + 1).rem_euclid(WiFiSetting::COUNT as isize) as usize;
                Ok((None, true))
            }
            KeyEvent::Pressed(Key::A) => Ok((self.select_entry(self.selected)?, true)),
            _ => Ok((None, false)),
        }
    }
}

#[derive(Debug, EnumCount, EnumIter, FromRepr)]
enum WiFiSetting {
    WiFi,
    Telnet,
    Ftp,
}

impl WiFiSetting {
    fn setting(&self, settings: &WiFiSettings) -> Setting {
        match self {
            Self::WiFi => Setting::bool("Wi-Fi Enabled", settings.wifi),
            Self::Telnet => Setting::bool("Telnet Enabled", settings.telnet),
            Self::Ftp => Setting::bool("FTP Enabled", settings.ftp),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WiFiSettings {
    pub wifi: bool,
    pub telnet: bool,
    pub ftp: bool,
}

impl WiFiSettings {
    pub fn new() -> Self {
        Self {
            wifi: false,
            telnet: false,
            ftp: false,
        }
    }

    pub fn load() -> Result<Self> {
        if ALLIUM_WIFI_SETTINGS.exists() {
            debug!("found state, loading from file");
            if let Ok(json) = fs::read_to_string(ALLIUM_WIFI_SETTINGS.as_path()) {
                if let Ok(json) = serde_json::from_str(&json) {
                    return Ok(json);
                }
            }
            warn!("failed to read state file, removing");
            fs::remove_file(ALLIUM_WIFI_SETTINGS.as_path())?;
        }
        Ok(Self::new())
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string(&self).unwrap();
        File::create(ALLIUM_WIFI_SETTINGS.as_path())?.write_all(json.as_bytes())?;
        Ok(())
    }
}

impl Default for WiFiSettings {
    fn default() -> Self {
        Self::new()
    }
}
