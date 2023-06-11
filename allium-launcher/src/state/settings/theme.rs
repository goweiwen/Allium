use anyhow::Result;
use common::display::color::Color;
use common::platform::Key;
use embedded_font::FontTextStyleBuilder;
use embedded_graphics::primitives::PrimitiveStyle;
use embedded_graphics::text::Alignment;
use embedded_graphics::{prelude::*, primitives::Rectangle};
use strum::{EnumCount, EnumIter, FromRepr, IntoEnumIterator};

use common::stylesheet::Stylesheet;
use common::{
    display::Display,
    platform::{DefaultPlatform, KeyEvent, Platform},
};
use tracing::warn;

use crate::state::settings::{SettingValue, Settings};
use crate::state::State;
use crate::{command::AlliumCommand, state::settings::Setting};

#[derive(Debug, Clone)]
pub struct SettingsThemeState {
    stylesheet: Stylesheet,
    selected: usize,
    selected_color: Option<ColorEditState>,
}

impl SettingsThemeState {
    pub fn new() -> Self {
        let stylesheet = Stylesheet::load().unwrap();
        Self {
            stylesheet,
            selected: 0,
            selected_color: None,
        }
    }

    fn select_entry(&mut self, selected: usize) -> Result<Option<AlliumCommand>> {
        if let Some(color) = self.selected_color.take() {
            match ThemeSetting::from_repr(selected) {
                Some(ThemeSetting::HighlightColor) => {
                    self.stylesheet.highlight_color = color.into()
                }
                Some(ThemeSetting::ButtonAColor) => self.stylesheet.button_a_color = color.into(),
                Some(ThemeSetting::ButtonBColor) => self.stylesheet.button_b_color = color.into(),
                Some(ThemeSetting::ButtonXColor) => self.stylesheet.button_x_color = color.into(),
                Some(ThemeSetting::ButtonYColor) => self.stylesheet.button_y_color = color.into(),
                Some(setting) => {
                    warn!(
                        "Trying to change color for non-color setting: {:?}",
                        setting
                    );
                }
                None => {
                    warn!("Invalid theme setting selected: {}", selected);
                }
            }
            Ok(Some(AlliumCommand::SaveStylesheet(Box::new(
                self.stylesheet.clone(),
            ))))
        } else {
            match ThemeSetting::from_repr(selected) {
                Some(ThemeSetting::HighlightColor) => {
                    self.selected_color = Some(self.stylesheet.highlight_color.into());
                    Ok(None)
                }
                Some(ThemeSetting::ButtonAColor) => {
                    self.selected_color = Some(self.stylesheet.button_a_color.into());
                    Ok(None)
                }
                Some(ThemeSetting::ButtonBColor) => {
                    self.selected_color = Some(self.stylesheet.button_b_color.into());
                    Ok(None)
                }
                Some(ThemeSetting::ButtonXColor) => {
                    self.selected_color = Some(self.stylesheet.button_x_color.into());
                    Ok(None)
                }
                Some(ThemeSetting::ButtonYColor) => {
                    self.selected_color = Some(self.stylesheet.button_y_color.into());
                    Ok(None)
                }
                Some(ThemeSetting::EnableBoxArt) => {
                    self.stylesheet.enable_box_art = !self.stylesheet.enable_box_art;
                    Ok(Some(AlliumCommand::SaveStylesheet(Box::new(
                        self.stylesheet.clone(),
                    ))))
                }
                None => {
                    warn!("Invalid theme setting selected: {}", selected);
                    Ok(None)
                }
            }
        }
    }
}

impl Default for SettingsThemeState {
    fn default() -> Self {
        Self::new()
    }
}

impl State for SettingsThemeState {
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
            Point::new(156 - 12, 58 - 4),
            Size::new(width, height - 58),
        ))?;

        let settings = Settings(
            ThemeSetting::iter()
                .map(|s| s.setting(&self.stylesheet))
                .collect(),
        );

        settings.draw(
            display,
            styles,
            self.selected,
            self.selected_color.is_some(),
            460,
        )?;

        if let Some(state) = &self.selected_color {
            let mut x = display.size().width as i32 - 24;
            let y = 58 + self.selected as i32 * 42;
            let selected = true;
            let editing = true;

            display.load(Rectangle::new(
                Point::new(x - 224, y - 4),
                Size::new(224, 42),
            ))?;

            SettingValue::Color(state.color).draw(
                display,
                styles,
                Point::new(x, y),
                selected,
                editing,
            )?;

            let text_style = FontTextStyleBuilder::new(styles.ui_font.clone())
                .font_size(styles.ui_font_size)
                .text_color(styles.foreground_color)
                .background_color(styles.highlight_color)
                .build();

            x = x - 30 - 12;
            for i in (0..6).rev() {
                let rect = display.draw_text(
                    Point::new(x, y),
                    &state.color.char(i),
                    text_style.clone(),
                    Alignment::Right,
                )?;
                if i == state.selected {
                    Rectangle::new(
                        Point::new(
                            rect.top_left.x,
                            rect.top_left.y + rect.size.height as i32 - 2,
                        ),
                        Size::new(rect.size.width, 3),
                    )
                    .into_styled(PrimitiveStyle::with_fill(styles.foreground_color))
                    .draw(display)?;
                }
                x = rect.top_left.x - 1;
            }
            display.draw_text(Point::new(x, y), "#", text_style, Alignment::Right)?;
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<(Option<AlliumCommand>, bool)> {
        if let Some(state) = self.selected_color.as_mut() {
            match key_event {
                KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                    state.color = match state.selected {
                        0 => state
                            .color
                            .with_r((state.color.r() as i32 + 16).rem_euclid(256) as u8),
                        1 => state.color.with_r(
                            (state.color.r() - state.color.r() % 16)
                                + (state.color.r() as i8 % 16 + 1).rem_euclid(16) as u8,
                        ),
                        2 => state
                            .color
                            .with_g((state.color.g() as i32 + 16).rem_euclid(256) as u8),
                        3 => state.color.with_g(
                            (state.color.g() - state.color.g() % 16)
                                + (state.color.g() as i8 % 16 + 1).rem_euclid(16) as u8,
                        ),
                        4 => state
                            .color
                            .with_b((state.color.b() as i32 + 16).rem_euclid(256) as u8),
                        5 => state.color.with_b(
                            (state.color.b() - state.color.b() % 16)
                                + (state.color.b() as i8 % 16 + 1).rem_euclid(16) as u8,
                        ),
                        _ => unreachable!(),
                    };
                    Ok((None, true))
                }
                KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                    state.color = match state.selected {
                        0 => state
                            .color
                            .with_r((state.color.r() as i32 - 16).rem_euclid(256) as u8),
                        1 => state.color.with_r(
                            (state.color.r() - state.color.r() % 16)
                                + (state.color.r() as i8 % 16 - 1).rem_euclid(16) as u8,
                        ),
                        2 => state
                            .color
                            .with_g((state.color.g() as i32 - 16).rem_euclid(256) as u8),
                        3 => state.color.with_g(
                            (state.color.g() - state.color.g() % 16)
                                + (state.color.g() as i8 % 16 - 1).rem_euclid(16) as u8,
                        ),
                        4 => state
                            .color
                            .with_b((state.color.b() as i32 - 16).rem_euclid(256) as u8),
                        5 => state.color.with_b(
                            (state.color.b() - state.color.b() % 16)
                                + (state.color.b() as i8 % 16 - 1).rem_euclid(16) as u8,
                        ),
                        _ => unreachable!(),
                    };
                    Ok((None, true))
                }
                KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                    state.selected = (state.selected as isize - 1).clamp(0, 5) as usize;
                    Ok((None, true))
                }
                KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                    state.selected = (state.selected as isize + 1).clamp(0, 5) as usize;
                    Ok((None, true))
                }
                KeyEvent::Pressed(Key::A) => Ok((self.select_entry(self.selected)?, true)),
                KeyEvent::Pressed(Key::B) => {
                    self.selected_color = None;
                    Ok((None, true))
                }
                _ => Ok((None, false)),
            }
        } else {
            match key_event {
                KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                    self.selected = (self.selected as isize - 1)
                        .rem_euclid(ThemeSetting::COUNT as isize)
                        as usize;
                    Ok((None, true))
                }
                KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                    self.selected = (self.selected as isize + 1)
                        .rem_euclid(ThemeSetting::COUNT as isize)
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
enum ThemeSetting {
    HighlightColor,
    ButtonAColor,
    ButtonBColor,
    ButtonXColor,
    ButtonYColor,
    EnableBoxArt,
}

impl ThemeSetting {
    fn setting(&self, stylesheet: &Stylesheet) -> Setting {
        match self {
            Self::HighlightColor => Setting::color("Highlight Color", stylesheet.highlight_color),
            Self::ButtonAColor => Setting::color("Button A Color", stylesheet.button_a_color),
            Self::ButtonBColor => Setting::color("Button B Color", stylesheet.button_b_color),
            Self::ButtonXColor => Setting::color("Button X Color", stylesheet.button_x_color),
            Self::ButtonYColor => Setting::color("Button Y Color", stylesheet.button_y_color),
            Self::EnableBoxArt => Setting::bool("Enable Box Art", stylesheet.enable_box_art),
        }
    }
}

#[derive(Debug, Clone)]
struct ColorEditState {
    color: Color,
    selected: usize,
}

impl From<Color> for ColorEditState {
    fn from(color: Color) -> Self {
        Self { color, selected: 0 }
    }
}

impl From<ColorEditState> for Color {
    fn from(state: ColorEditState) -> Self {
        state.color
    }
}
