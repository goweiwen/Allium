use std::fs;

use anyhow::Result;
use common::battery::Battery;
use common::constants::ALLIUM_GAME_INFO;
use common::constants::BATTERY_UPDATE_INTERVAL;
use common::display::font::FontTextStyleBuilder;
use common::display::Display;
use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text};

use crate::menu::Menu;

#[cfg(unix)]
use {std::process, tokio::signal::unix::SignalKind};

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

        let game_info = fs::read_to_string(ALLIUM_GAME_INFO.as_path()).unwrap_or("".to_owned());
        let mut split = game_info.split('\n');
        let name = split.next().unwrap_or("").to_owned();

        Ok(AlliumMenu {
            platform,
            display,
            battery,
            styles: Default::default(),
            menu: Menu::new()?,
            dirty: true,
            name,
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        self.display.darken()?;
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
                    self.display.load(self.display.bounding_box())?;
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
            let dirty = self
                .menu
                .handle_key_event(key_event, &mut self.display)
                .await?;
            if dirty {
                self.dirty = true;
            }
        }
        Ok(())
    }

    fn draw(&mut self) -> Result<()> {
        let Size { width, height: _ } = self.display.size();

        let text_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(self.styles.ui_font_size)
            .text_color(self.styles.foreground_color)
            .build();

        let primary_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(self.styles.ui_font_size)
            .text_color(self.styles.highlight_color)
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
