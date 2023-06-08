use std::fs;

use anyhow::Result;
use common::constants::ALLIUM_GAME_INFO;
use common::display::color::Color;
use embedded_font::FontTextStyleBuilder;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text};
use embedded_graphics::Drawable;
use tracing::{debug, warn};

use common::battery::Battery;
use common::constants::BATTERY_UPDATE_INTERVAL;
use common::display::Display;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;

use crate::state::State;

#[derive(Debug)]
pub struct AlliumLauncher<P: Platform> {
    platform: P,
    display: P::Display,
    battery: P::Battery,
    styles: Stylesheet,
    state: State,
    dirty: bool,
}

impl AlliumLauncher<DefaultPlatform> {
    pub fn new() -> Result<AlliumLauncher<DefaultPlatform>> {
        let mut platform = DefaultPlatform::new()?;
        let display = platform.display()?;
        let battery = platform.battery()?;

        Ok(AlliumLauncher {
            platform,
            display,
            battery,
            styles: Default::default(),
            state: State::new()?,
            dirty: true,
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        // Remove game info now that Allium is running again
        if ALLIUM_GAME_INFO.exists() {
            fs::remove_file(ALLIUM_GAME_INFO.as_path())
                .map_err(|e| warn!("failed to remove game info: {}", e))
                .ok();
        }

        self.display.clear(Color::new(0, 0, 0))?;
        self.display.save()?;

        self.state.enter()?;

        let mut last_updated_battery = std::time::Instant::now();
        self.battery.update()?;

        loop {
            let now = std::time::Instant::now();

            // Update battery every 5 seconds
            if now.duration_since(last_updated_battery) > BATTERY_UPDATE_INTERVAL {
                self.battery.update()?;
                last_updated_battery = now;
                self.dirty = true;
            }

            self.state.update()?;

            if self.dirty {
                self.draw()?;
                self.state.draw(&mut self.display, &self.styles)?;
                self.display.flush()?;
                self.dirty = false;
            }

            self.dirty = match self.platform.poll().await? {
                Some(KeyEvent::Pressed(Key::L)) => {
                    if let Some(next_state) = self.state.prev()? {
                        self.state.leave()?;
                        self.state = next_state;
                        self.state.enter()?;
                    }
                    true
                }
                Some(KeyEvent::Pressed(Key::R)) => {
                    if let Some(next_state) = self.state.next()? {
                        self.state.leave()?;
                        self.state = next_state;
                        self.state.enter()?;
                    }
                    true
                }
                Some(key_event) => self.state.handle_key_event(key_event)?,
                None => false,
            };
        }
    }

    fn draw(&mut self) -> Result<()> {
        let Size { width, height: _ } = self.display.size();

        let text_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(self.styles.ui_font_size)
            .text_color(self.styles.fg_color)
            .build();

        let primary_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(self.styles.ui_font_size)
            .text_color(self.styles.primary)
            .build();

        // Draw battery percentage
        if self.battery.charging() {
            Text::with_alignment(
                &format!("Charging: {}%", self.battery.percentage()),
                Point {
                    x: width as i32 - 8,
                    y: 8,
                },
                text_style.clone(),
                Alignment::Right,
            )
            .draw(&mut self.display)?;
        } else {
            Text::with_alignment(
                &format!("{}%", self.battery.percentage()),
                Point {
                    x: width as i32 - 8,
                    y: 8,
                },
                text_style.clone(),
                Alignment::Right,
            )
            .draw(&mut self.display)?;
        }

        // Draw header navigation
        let mut x = 12;
        let selected = match self.state {
            State::Games(_) => 0,
            State::Recents(_) => 1,
            State::Settings(_) => 2,
        };
        for (i, text) in ["Games", "Recents", "Settings"].iter().enumerate() {
            let text = Text::with_alignment(
                text,
                Point { x, y: 8 },
                if i == selected {
                    primary_style.clone()
                } else {
                    text_style.clone()
                },
                Alignment::Left,
            );
            x += text.bounding_box().size.width as i32 + 12;
            text.draw(&mut self.display)?;
        }

        Ok(())
    }
}
