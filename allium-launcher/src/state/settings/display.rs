use anyhow::Result;
use common::constants::{ALLIUM_CONFIG_DIR, SELECTION_HEIGHT, SELECTION_MARGIN};
use common::display::font::FontTextStyleBuilder;
use common::display::settings::DisplaySettings;
use common::platform::Key;
use embedded_graphics::{prelude::*, primitives::Rectangle};
use strum::{EnumCount, EnumIter, FromRepr, IntoEnumIterator};

use common::stylesheet::Stylesheet;
use common::{
    display::Display,
    platform::{DefaultPlatform, KeyEvent, Platform},
};

use crate::state::settings::{SettingValue, Settings};
use crate::state::State;
use crate::{command::AlliumCommand, state::settings::Setting};

#[derive(Debug, Clone)]
pub struct SettingsDisplayState {
    settings: DisplaySettings,
    selected: usize,
    value: Option<u8>,
    has_drawn_image: bool,
    has_changes: bool,
}

impl SettingsDisplayState {
    pub fn new() -> Result<Self> {
        Ok(Self {
            settings: DisplaySettings::load()?,
            selected: 0,
            value: None,
            has_drawn_image: false,
            has_changes: false,
        })
    }

    pub fn select_entry(&mut self, index: usize) -> Result<Option<AlliumCommand>> {
        if let Some(value) = self.value {
            match DisplaySetting::from_repr(index) {
                Some(
                    DisplaySetting::Luminance
                    | DisplaySetting::Hue
                    | DisplaySetting::Saturation
                    | DisplaySetting::Contrast,
                ) => self.has_changes = true,
                Some(DisplaySetting::Brightness) => (),
                None => panic!("Invalid display setting index"),
            }

            match DisplaySetting::from_repr(index) {
                Some(DisplaySetting::Brightness) => self.settings.brightness = value,
                Some(DisplaySetting::Luminance) => self.settings.luminance = value,
                Some(DisplaySetting::Hue) => self.settings.hue = value,
                Some(DisplaySetting::Saturation) => self.settings.saturation = value,
                Some(DisplaySetting::Contrast) => self.settings.contrast = value,
                None => panic!("Invalid display setting index"),
            }
            self.value = None;
            Ok(Some(AlliumCommand::SaveDisplaySettings(Box::new(
                self.settings.to_owned(),
            ))))
        } else {
            match DisplaySetting::from_repr(index) {
                Some(DisplaySetting::Brightness) => self.value = Some(self.settings.brightness),
                Some(DisplaySetting::Luminance) => self.value = Some(self.settings.luminance),
                Some(DisplaySetting::Hue) => self.value = Some(self.settings.hue),
                Some(DisplaySetting::Saturation) => self.value = Some(self.settings.saturation),
                Some(DisplaySetting::Contrast) => self.value = Some(self.settings.contrast),
                None => panic!("Invalid display setting index"),
            }
            Ok(None)
        }
    }
}

impl Default for SettingsDisplayState {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl State for SettingsDisplayState {
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
        if !self.has_drawn_image {
            let Size { width, height } = display.size();
            display.load(Rectangle::new(
                Point::new(146 - 12, 58 - 4),
                Size::new(width - 146 + 12, height - 58 - 4),
            ))?;
            display.draw_image(
                Point::new(358, 58),
                &ALLIUM_CONFIG_DIR.join("images/display.png"),
            )?;
        }

        if self.has_changes {
            display.draw_text(
                Point::new(display.size().width as i32 - 12, 392),
                "*Restart device to apply changes",
                FontTextStyleBuilder::new(styles.ui_font.clone())
                    .font_size(styles.ui_font_size)
                    .text_color(styles.foreground_color)
                    .background_color(styles.background_color)
                    .build(),
                embedded_graphics::text::Alignment::Right,
            )?;
        }

        let (x, y) = (146, 58);
        display.load(Rectangle::new(
            Point::new(x - 12, y - 4),
            Size::new(
                200 + 12 * 2,
                (SELECTION_HEIGHT + SELECTION_MARGIN) * DisplaySetting::COUNT as u32,
            ),
        ))?;

        let settings = Settings(
            DisplaySetting::iter()
                .map(|s| s.setting(&self.settings))
                .collect(),
        );

        settings.draw(display, styles, self.selected, self.value.is_some(), 200)?;

        if let Some(value) = self.value {
            let x = 156 + 214 - 24;
            let y = 58 + self.selected as i32 * 42;
            let selected = true;
            let editing = true;

            display.load(Rectangle::new(Point::new(x - 64, y - 4), Size::new(64, 42)))?;

            SettingValue::Percentage(value).draw(
                display,
                styles,
                Point::new(x, y),
                selected,
                editing,
            )?;
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<(Option<AlliumCommand>, bool)> {
        if let Some(value) = self.value {
            match key_event {
                KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                    self.value = Some((value + 1).min(100));
                    Ok((None, true))
                }
                KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                    self.value = Some((value as i8 - 1).max(0) as u8);
                    Ok((None, true))
                }
                KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                    self.value = Some((value + 10).min(100));
                    Ok((None, true))
                }
                KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                    self.value = Some((value as i8 - 10).max(0) as u8);
                    Ok((None, true))
                }
                KeyEvent::Pressed(Key::A) => Ok((self.select_entry(self.selected)?, true)),
                KeyEvent::Pressed(Key::B) => {
                    self.value = None;
                    Ok((None, true))
                }
                _ => Ok((None, false)),
            }
        } else {
            match key_event {
                KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                    self.selected = (self.selected as isize - 1)
                        .rem_euclid(DisplaySetting::COUNT as isize)
                        as usize;
                    Ok((None, true))
                }
                KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                    self.selected = (self.selected as isize + 1)
                        .rem_euclid(DisplaySetting::COUNT as isize)
                        as usize;
                    Ok((None, true))
                }
                KeyEvent::Pressed(Key::A) => Ok((self.select_entry(self.selected)?, true)),
                _ => Ok((None, false)),
            }
        }
    }
}

#[derive(Debug, EnumCount, EnumIter, FromRepr)]
enum DisplaySetting {
    Brightness,
    Luminance,
    Hue,
    Saturation,
    Contrast,
}

impl DisplaySetting {
    fn setting(&self, display_settings: &DisplaySettings) -> Setting {
        match self {
            Self::Brightness => Setting::percentage("Brightness", display_settings.brightness),
            Self::Luminance => Setting::percentage("Luminance", display_settings.luminance),
            Self::Hue => Setting::percentage("Hue", display_settings.hue),
            Self::Saturation => Setting::percentage("Saturation", display_settings.saturation),
            Self::Contrast => Setting::percentage("Contrast", display_settings.contrast),
        }
    }
}
