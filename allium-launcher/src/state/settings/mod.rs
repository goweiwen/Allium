mod display;
mod system;
mod theme;
mod wifi;

use anyhow::{anyhow, Result};
use common::{constants::BUTTON_DIAMETER, display::font::FontTextStyleBuilder};
use embedded_graphics::{
    prelude::*,
    primitives::{Primitive, PrimitiveStyleBuilder, Rectangle, StrokeAlignment},
    text::Alignment,
};
use serde::{Deserialize, Serialize};
use strum::{EnumCount, FromRepr, VariantNames};

use common::{display::color::Color, platform::Key, stylesheet::Stylesheet};
use common::{
    display::Display,
    platform::{DefaultPlatform, KeyEvent, Platform},
};

use crate::state::State;
use crate::{
    command::AlliumCommand,
    state::settings::{
        display::SettingsDisplayState, system::SettingsSystemState, theme::SettingsThemeState,
        wifi::SettingsWiFiState,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsState {
    selected: usize,
    #[serde(skip)]
    section: Option<SettingsSection>,
}

impl SettingsState {
    pub fn new() -> Result<Self> {
        Ok(Self {
            selected: 0,
            section: None,
        })
    }
}

impl State for SettingsState {
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
        let (x, mut y) = (24, 58);
        display.load(Rectangle::new(
            Point::new(x - 12, y - 4),
            Size::new(110 + 12 * 2, display.size().height - y as u32 + 4),
        ))?;

        if self.section.is_none() {
            for (i, label) in SettingsSection::VARIANTS.iter().enumerate() {
                display.draw_entry(
                    Point { x, y },
                    label,
                    styles,
                    Alignment::Left,
                    300,
                    i == self.selected,
                    true,
                    0,
                )?;
                y += 42;
            }
            display.load(Rectangle::new(
                Point::new(146 - 12, 58 - 4),
                Size::new(506, 422),
            ))?;
        } else {
            let selected_style = FontTextStyleBuilder::new(styles.ui_font.clone())
                .font_size(styles.ui_font_size)
                .text_color(styles.highlight_color)
                .background_color(styles.background_color)
                .build();

            let inactive_style = FontTextStyleBuilder::new(styles.ui_font.clone())
                .font_size(styles.ui_font_size)
                .text_color(styles.foreground_color)
                .background_color(styles.background_color)
                .build();
            for (i, label) in SettingsSection::VARIANTS.iter().enumerate() {
                display.draw_text(
                    Point { x, y },
                    label,
                    if i == self.selected {
                        selected_style.clone()
                    } else {
                        inactive_style.clone()
                    },
                    Alignment::Left,
                )?;
                y += 42;
            }
        }

        if let Some(ref mut section) = self.section {
            section.draw(display, styles)?;
        }

        // Draw button hints
        let y = display.size().height as i32 - BUTTON_DIAMETER as i32 - 8;
        let mut x = display.size().width as i32 - 12;

        x = display
            .draw_button_hint(Point::new(x, y), Key::A, "Select", styles, Alignment::Right)?
            .top_left
            .x
            - 18;
        display.draw_button_hint(Point::new(x, y), Key::B, "Back", styles, Alignment::Right)?;

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<(Option<AlliumCommand>, bool)> {
        if let Some(ref mut section) = self.section {
            if key_event == KeyEvent::Pressed(Key::B) {
                let mut ret = section.handle_key_event(key_event)?;
                if !ret.1 {
                    self.section = None;
                    ret.1 = true;
                }
                Ok(ret)
            } else {
                section.handle_key_event(key_event)
            }
        } else {
            match key_event {
                KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                    self.selected = (self.selected as isize - 1)
                        .rem_euclid(SettingsSection::COUNT as isize)
                        as usize;
                    Ok((None, true))
                }
                KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                    self.selected = (self.selected as isize + 1)
                        .rem_euclid(SettingsSection::COUNT as isize)
                        as usize;
                    Ok((None, true))
                }
                KeyEvent::Pressed(Key::A) => {
                    self.section = SettingsSection::from_repr(self.selected);
                    Ok((None, true))
                }
                _ => Ok((None, false)),
            }
        }
    }
}

#[derive(Debug, Clone, EnumCount, FromRepr)]
enum SettingsSection {
    WiFi(SettingsWiFiState),
    Display(SettingsDisplayState),
    Theme(SettingsThemeState),
    System(SettingsSystemState),
}

impl VariantNames for SettingsSection {
    const VARIANTS: &'static [&'static str] = &["Wi-Fi", "Display", "Theme", "System"];
}

impl SettingsSection {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<()> {
        match self {
            Self::WiFi(s) => s.draw(display, styles),
            Self::Display(s) => s.draw(display, styles),
            Self::Theme(s) => s.draw(display, styles),
            Self::System(s) => s.draw(display, styles),
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<(Option<AlliumCommand>, bool)> {
        match self {
            Self::WiFi(s) => s.handle_key_event(key_event),
            Self::Display(s) => s.handle_key_event(key_event),
            Self::Theme(s) => s.handle_key_event(key_event),
            Self::System(s) => s.handle_key_event(key_event),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Settings(Vec<Setting>);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Setting {
    label: &'static str,
    value: SettingValue,
    disabled: bool,
}

impl Setting {
    fn new(label: &'static str, value: SettingValue) -> Self {
        Self {
            label,
            value,
            disabled: false,
        }
    }

    fn bool(label: &'static str, value: bool) -> Self {
        Self::new(label, SettingValue::Bool(value))
    }

    fn percentage(label: &'static str, value: u8) -> Self {
        Self::new(label, SettingValue::Percentage(value))
    }

    fn string(label: &'static str, value: impl Into<String>) -> Self {
        Self::new(label, SettingValue::String(value.into()))
    }

    fn color(label: &'static str, value: Color) -> Self {
        Self::new(label, SettingValue::Color(value))
    }

    fn none(label: &'static str) -> Self {
        Self::new(label, SettingValue::None)
    }
}

impl Settings {
    fn draw(
        &self,
        display: &mut impl Display,
        styles: &Stylesheet,
        selected: usize,
        editing: bool,
        width: i32,
    ) -> Result<()> {
        let x0 = 146;
        let x1 = 146 + width;
        let mut y = 58;
        for (i, setting) in self.0.iter().enumerate() {
            display.draw_entry(
                Point::new(x0, y),
                setting.label,
                styles,
                Alignment::Left,
                224,
                i == selected,
                !editing,
                0,
            )?;
            setting
                .value
                .draw(display, styles, Point::new(x1, y), i == selected, editing)?;
            y += 42;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SettingValue {
    None,
    Bool(bool),
    Percentage(u8),
    String(String),
    Color(Color),
}

impl SettingValue {
    fn draw(
        &self,
        display: &mut impl Display,
        styles: &Stylesheet,
        point: Point,
        selected: bool,
        editing: bool,
    ) -> Result<()> {
        match self {
            SettingValue::None => {}
            SettingValue::Bool(value) => {
                display.draw_entry(
                    point,
                    if *value { "Yes" } else { "No" },
                    styles,
                    Alignment::Right,
                    224,
                    selected,
                    editing,
                    0,
                )?;
            }
            SettingValue::Percentage(value) => {
                display.draw_entry(
                    point,
                    &format!("{}%", value),
                    styles,
                    Alignment::Right,
                    224,
                    selected,
                    editing,
                    0,
                )?;
            }
            SettingValue::String(value) => {
                display.draw_entry(
                    point,
                    value.as_str(),
                    styles,
                    Alignment::Right,
                    224,
                    selected,
                    editing,
                    0,
                )?;
            }
            SettingValue::Color(value) => {
                display.draw_entry(
                    point,
                    &format!("#{:X}", value),
                    styles,
                    Alignment::Right,
                    224,
                    selected,
                    editing,
                    30 + 12,
                )?;

                let fill_style = PrimitiveStyleBuilder::new()
                    .fill_color(value.to_owned())
                    .stroke_color(styles.foreground_color)
                    .stroke_width(1)
                    .stroke_alignment(StrokeAlignment::Inside)
                    .build();

                Rectangle::new(
                    Point::new(point.x - styles.ui_font_size as i32 - 6, point.y - 2),
                    Size::new(30, 30),
                )
                .into_styled(fill_style)
                .draw(display)
                .map_err(|_| anyhow!("failed to draw rectangle"))?;
            }
        }
        Ok(())
    }
}
