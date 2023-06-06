use std::fs;
use std::path::Path;

use anyhow::Result;

use common::battery::Battery;
use common::constants::{ALLIUM_CORE_ID, BATTERY_UPDATE_INTERVAL};
use common::display::Display;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;

use crate::state::State;

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
        // Remove core pid now that Allium is running again
        let path = Path::new(ALLIUM_CORE_ID);
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

            self.state.update()?;

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
                Some(key_event) => self.state.handle_key_event(key_event)?,
                None => false,
            };
        }
    }
}
