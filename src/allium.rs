#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(unix)]
use std::process::Command;

use anyhow::Result;
use embedded_graphics::pixelcolor::Rgb888;
use rusttype::Font;

use crate::battery::Battery;
use crate::constants::BATTERY_UPDATE_INTERVAL;
use crate::display::Display;
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::state::State;

pub struct Allium<P: Platform> {
    platform: P,
    display: P::Display,
    battery: P::Battery,
    styles: Stylesheet,
    state: State,
    dirty: bool,
}

pub struct Stylesheet {
    pub fg_color: Rgb888,
    pub bg_color: Rgb888,
    pub primary: Rgb888,
    pub button_a_color: Rgb888,
    pub button_b_color: Rgb888,
    pub button_x_color: Rgb888,
    pub button_y_color: Rgb888,
    pub ui_font: Font<'static>,
    pub ui_font_size: u32,
}

impl Default for Stylesheet {
    fn default() -> Self {
        Self {
            fg_color: Rgb888::new(255, 255, 255),
            bg_color: Rgb888::new(0, 0, 0),
            primary: Rgb888::new(151, 135, 187),
            button_a_color: Rgb888::new(235, 26, 29),
            button_b_color: Rgb888::new(254, 206, 21),
            button_x_color: Rgb888::new(7, 73, 180),
            button_y_color: Rgb888::new(0, 141, 69),
            ui_font: Font::try_from_bytes(include_bytes!("../assets/font/Lato/Lato-Bold.ttf"))
                .unwrap(),
            ui_font_size: 24,
        }
    }
}

impl Allium<DefaultPlatform> {
    pub fn new() -> Result<Allium<DefaultPlatform>> {
        let mut platform = DefaultPlatform::new()?;
        let display = platform.display()?;
        let battery = platform.battery()?;

        Ok(Allium {
            platform,
            display,
            battery,
            styles: Default::default(),
            state: State::new()?,
            dirty: true,
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
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

            if self.dirty {
                self.state
                    .draw(&mut self.display, &self.styles, &self.battery)?;
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
                Some(KeyEvent::Pressed(Key::Power)) => {
                    #[cfg(unix)]
                    Command::new("poweroff").exec();
                    false
                }
                Some(key_event) => self.state.handle_key_event(key_event)?,
                None => false,
            };

            self.state.update()?;
        }
    }
}
