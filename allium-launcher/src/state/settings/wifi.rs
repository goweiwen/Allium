use anyhow::Result;
use common::display::font::FontTextStyleBuilder;
use common::platform::Key;
use common::wifi::{self, WiFiSettings};
use embedded_graphics::{prelude::*, primitives::Rectangle};
use strum::{EnumCount, EnumIter, FromRepr, IntoEnumIterator};

use common::stylesheet::Stylesheet;
use common::{
    display::Display,
    platform::{DefaultPlatform, KeyEvent, Platform},
};

use crate::state::settings::Settings;
use crate::state::State;
use crate::{command::AlliumCommand, state::settings::Setting};

#[derive(Debug, Clone)]
pub struct SettingsWiFiState {
    settings: WiFiSettings,
    selected: usize,
    ip_address: Option<String>,
}

impl SettingsWiFiState {
    pub fn new() -> Result<Self> {
        Ok(Self {
            settings: WiFiSettings::load()?,
            selected: 0,
            ip_address: None,
        })
    }

    pub fn select_entry(&mut self, index: usize) -> Result<Option<AlliumCommand>> {
        match WiFiSetting::from_repr(index) {
            Some(WiFiSetting::WiFi) => {
                self.settings.toggle_wifi()?;
                if !self.settings.wifi {
                    self.ip_address = None;
                }
            }
            Some(WiFiSetting::Ssid) => (),
            Some(WiFiSetting::Password) => (),
            Some(WiFiSetting::Telnet) => self.settings.toggle_telnet()?,
            Some(WiFiSetting::Ftp) => self.settings.toggle_ftp()?,
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
            Size::new(width - 146 - 12, height - 58 - 4),
        ))?;

        let settings = Settings(
            WiFiSetting::iter()
                .map(|s| s.setting(&self.settings))
                .collect(),
        );

        settings.draw(display, styles, self.selected, false, 470)?;

        // Try to get the IP address if we don't have it yet
        if self.ip_address.is_none() {
            self.ip_address = wifi::ip_address();
        }
        if let Some(ip_address) = self.ip_address.as_deref() {
            display.draw_text(
                Point::new(display.size().width as i32 - 12, 392),
                &format!("IP Address: {}", ip_address),
                FontTextStyleBuilder::new(styles.ui_font.clone())
                    .font_size(styles.ui_font_size)
                    .text_color(styles.foreground_color)
                    .background_color(styles.background_color)
                    .build(),
                embedded_graphics::text::Alignment::Right,
            )?;
        }

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
    Ssid,
    Password,
    Telnet,
    Ftp,
}

impl WiFiSetting {
    fn setting(&self, settings: &WiFiSettings) -> Setting {
        match self {
            Self::WiFi => Setting::bool("Wi-Fi Enabled", settings.wifi),
            Self::Ssid => Setting::string("Wi-Fi Network", &settings.ssid),
            Self::Password => Setting::string("Wi-Fi Password", "********"),
            Self::Telnet => {
                let mut setting = Setting::bool("Telnet Enabled", settings.telnet);
                setting.disabled = !settings.wifi;
                setting
            }
            Self::Ftp => {
                let mut setting = Setting::bool("FTP Enabled", settings.ftp);
                setting.disabled = !settings.wifi;
                setting
            }
        }
    }
}
