mod system;

use std::fmt;

use anyhow::Result;
use embedded_font::FontTextStyleBuilder;
use embedded_graphics::{
    prelude::*,
    primitives::{Primitive, PrimitiveStyleBuilder, Rectangle, StrokeAlignment},
    text::Alignment,
};
use serde::{Deserialize, Serialize};

use common::{display::color::Color, stylesheet::Stylesheet};
use common::{
    display::Display,
    platform::{DefaultPlatform, KeyEvent, Platform},
};

use crate::{
    command::AlliumCommand,
    state::{settings::system::SettingsSystemState, State},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsState {
    section: usize,
    #[serde(skip, default = "SettingsState::sections")]
    sections: Vec<SettingsSection>,
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            section: 0,
            sections: Self::sections(),
        }
    }

    fn sections() -> Vec<SettingsSection> {
        vec![SettingsSection::System(SettingsSystemState::new())]
    }

    fn section(&self) -> &SettingsSection {
        &self.sections[self.section]
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
        &self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<()> {
        let Size { width, height } = display.size();
        display.load(Rectangle::new(
            Point::new(0, 46),
            Size::new(width, height - 46),
        ))?;

        self.section().draw(display, styles)?;
        Ok(())
    }

    fn handle_key_event(&mut self, _key_event: KeyEvent) -> Result<(Option<AlliumCommand>, bool)> {
        Ok((None, false))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SettingsSection {
    // Display(SettingsDisplayState),
    // Theme(SettingsThemeState),
    System(SettingsSystemState),
}

impl fmt::Display for SettingsSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Self::Display(_) => write!(f, "Display"),
            // Self::Theme(_) => write!(f, "Theme"),
            Self::System(_) => write!(f, "System"),
        }
    }
}

impl SettingsSection {
    fn next(&self) -> Self {
        match self {
            // Self::Display(_) => Self::Theme(SettingsThemeState::new()),
            // Self::Theme(_) => Self::System(SettingsSystemState::new()),
            // Self::System(_) => Self::Display(SettingsDisplayState::new()),
            Self::System(s) => Self::System(s.clone()),
        }
    }

    fn prev(&self) -> Self {
        match self {
            // Self::Display(_) => Self::System(SettingsSystemState::new()),
            // Self::Theme(_) => Self::Display(SettingsDisplayState::new()),
            // Self::System(_) => Self::Theme(SettingsThemeState::new()),
            Self::System(s) => Self::System(s.clone()),
        }
    }

    fn draw(
        &self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<()> {
        match self {
            // Self::Display(s) => s.draw(display, styles),
            // Self::Theme(s) => s.draw(display, styles),
            Self::System(s) => s.draw(display, styles),
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
    fn bool(label: &'static str, value: bool) -> Self {
        Self {
            label,
            value: SettingValue::Bool(value),
            disabled: false,
        }
    }

    fn string(label: &'static str, value: impl Into<String>) -> Self {
        Self {
            label,
            value: SettingValue::String(value.into()),
            disabled: false,
        }
    }

    fn color(label: &'static str, value: Color) -> Self {
        Self {
            label,
            value: SettingValue::Color(value),
            disabled: false,
        }
    }
}

impl Settings {
    fn draw(
        &self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<()> {
        let text_style = FontTextStyleBuilder::new(styles.ui_font.clone())
            .font_size(styles.ui_font_size)
            .text_color(styles.fg_color)
            .build();

        let x0 = 156;
        let x1 = display.size().width as i32 - 24;
        let mut y = 46;
        for setting in self.0.iter() {
            display.draw_text(
                Point::new(x0, y),
                setting.label,
                text_style.clone(),
                Alignment::Left,
            )?;
            match &setting.value {
                SettingValue::Bool(value) => {
                    display.draw_text(
                        Point::new(x1, y),
                        if *value { "Yes" } else { "No" },
                        text_style.clone(),
                        Alignment::Right,
                    )?;
                }
                SettingValue::Percentage(value) => {
                    display.draw_text(
                        Point::new(x1, y),
                        &format!("{}%", value),
                        text_style.clone(),
                        Alignment::Right,
                    )?;
                }
                SettingValue::String(value) => {
                    display.draw_text(
                        Point::new(x1, y),
                        value.as_str(),
                        text_style.clone(),
                        Alignment::Right,
                    )?;
                }
                SettingValue::Color(value) => {
                    let fill_style = PrimitiveStyleBuilder::new()
                        .fill_color(value.to_owned())
                        .stroke_color(styles.fg_color)
                        .stroke_width(1)
                        .stroke_alignment(StrokeAlignment::Inside)
                        .build();

                    Rectangle::new(Point::new(x1 - 30, y - 30), Size::new(30, 30))
                        .into_styled(fill_style)
                        .draw(display)?;

                    display.draw_text(
                        Point::new(x1 - 30 - 12, y),
                        &value.to_string(),
                        text_style.clone(),
                        Alignment::Right,
                    )?;
                }
            };
            y += 42;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SettingValue {
    Bool(bool),
    Percentage(i32),
    String(String),
    Color(Color),
}
