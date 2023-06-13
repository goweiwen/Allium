use std::process;

use anyhow::Result;
use common::battery::Battery;
use common::constants::BATTERY_UPDATE_INTERVAL;
use common::display::color::Color;
use common::display::font::FontTextStyleBuilder;
use common::display::Display;
use common::game_info::GameInfo;
use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text};

use crate::command::AlliumMenuCommand;
use crate::menu::Menu;

#[cfg(unix)]
use tokio::signal::unix::SignalKind;

pub struct AlliumMenu<P: Platform> {
    platform: P,
    display: P::Display,
    battery: P::Battery,
    styles: Stylesheet,
    menu: Menu,
    dirty: bool,
    name: String,
}

impl AlliumMenu<DefaultPlatform> {
    pub fn new() -> Result<AlliumMenu<DefaultPlatform>> {
        let mut platform = DefaultPlatform::new()?;
        let display = platform.display()?;
        let battery = platform.battery()?;

        let game_info = GameInfo::load()?;

        Ok(AlliumMenu {
            platform,
            display,
            battery,
            styles: Stylesheet::load()?,
            menu: Menu::new()?,
            dirty: true,
            name: game_info.map(|game| game.name).unwrap_or("".to_string()),
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        self.display
            .map_pixels(|pixel| pixel.blend(self.styles.background_color.overlay(pixel), 192))?;
        self.display.save()?;

        let mut last_updated_battery = std::time::Instant::now();
        self.battery.update()?;

        #[cfg(unix)]
        let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;

        loop {
            let now = std::time::Instant::now();

            // Update battery every 5 seconds
            if now.duration_since(last_updated_battery) > BATTERY_UPDATE_INTERVAL {
                self.battery.update()?;
                last_updated_battery = now;
                self.dirty = true;
            }

            self.menu.update()?;

            if self.dirty {
                self.draw()?;
                self.menu.draw(&mut self.display, &self.styles)?;
                self.display.flush()?;
                self.dirty = false;
            }

            #[cfg(unix)]
            tokio::select! {
                _ = sigterm.recv() => {
                    self.display.clear(Color::new(0, 0, 0))?;
                    self.display.flush()?;
                    process::exit(0);
                }
                key_event = self.platform.poll() => {
                    let key_event = key_event?;
                    self.handle_key_event(key_event).await?;
                }
            }

            #[cfg(not(unix))]
            {
                let key_event = self.platform.poll().await?;
                self.handle_key_event(key_event).await?;
            }
        }
    }

    async fn handle_key_event(&mut self, key_event: Option<KeyEvent>) -> Result<()> {
        if let Some(key_event) = key_event {
            let (command, dirty) = self.menu.handle_key_event(key_event).await?;
            if dirty {
                self.dirty = true;
            }

            if let Some(AlliumMenuCommand::Close) = command {
                self.display.clear(Color::new(0, 0, 0))?;
                self.display.flush()?;
                process::exit(0);
            }
        }
        Ok(())
    }

    fn draw(&mut self) -> Result<()> {
        let Size { width, height: _ } = self.display.size();

        let text_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(self.styles.ui_font_size)
            .text_color(self.styles.foreground_color)
            .background_color(self.styles.background_color)
            .build();

        let primary_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(self.styles.ui_font_size)
            .text_color(self.styles.highlight_color)
            .background_color(self.styles.background_color)
            .build();

        // Draw battery percentage
        if self.battery.charging() {
            Text::with_alignment(
                &format!("Charging: {}%", self.battery.percentage()),
                Point {
                    x: width as i32 - 8,
                    y: 8,
                },
                text_style,
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
                text_style,
                Alignment::Right,
            )
            .draw(&mut self.display)?;
        }

        // Draw game name
        let text = Text::with_alignment(
            &self.name,
            Point { x: 12, y: 8 },
            primary_style,
            Alignment::Left,
        );
        text.draw(&mut self.display)?;

        Ok(())
    }
}
