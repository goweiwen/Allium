use std::fs;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::Path;
#[cfg(unix)]
use std::process::Command;

use anyhow::Result;

use crate::battery::Battery;
use crate::constants::BATTERY_UPDATE_INTERVAL;
use crate::display::Display;
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::state::State;
use crate::stylesheet::Stylesheet;

pub struct Allium<P: Platform> {
    platform: P,
    display: P::Display,
    battery: P::Battery,
    styles: Stylesheet,
    state: State,
    dirty: bool,
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
        // Remove core pid now that Allium is running again
        let path = Path::new("/tmp/allium_core.pid");
        if path.exists() {
            fs::remove_file(path)?;
        }

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
