use std::fs;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;
#[cfg(unix)]
use {std::os::unix::process::CommandExt, std::process, tokio::signal::unix::SignalKind};

use anyhow::Result;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::{Alignment, Text};
use embedded_graphics::Drawable;
use serde::Deserialize;
use serde::Serialize;
use tracing::trace;
use tracing::{debug, warn};

use common::battery::Battery;
use common::constants::ALLIUM_LAUNCHER_STATE;
use common::constants::BATTERY_UPDATE_INTERVAL;
use common::database::Database;
use common::display::font::FontTextStyleBuilder;
use common::display::Display;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;

use crate::command::AlliumCommand;
use crate::devices::DeviceMapper;
use crate::state::GamesState;
use crate::state::RecentsState;
use crate::state::SettingsState;
use crate::state::State;

#[derive(Debug)]
pub struct AlliumLauncher<P: Platform> {
    platform: P,
    display: P::Display,
    battery: P::Battery,
    styles: Stylesheet,
    state: AlliumLauncherState,
    dirty: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct AlliumLauncherState {
    state: usize,
    states: (GamesState, RecentsState, SettingsState),
}

impl AlliumLauncherState {
    fn new() -> Result<Self> {
        Ok(Self {
            state: 0,
            states: (
                GamesState::new()?,
                RecentsState::new(),
                SettingsState::new()?,
            ),
        })
    }

    fn state_mut(&mut self) -> &mut dyn State {
        match self.state {
            0 => &mut self.states.0,
            1 => &mut self.states.1,
            2 => &mut self.states.2,
            _ => panic!("invalid state"),
        }
    }

    fn next(&mut self) -> Result<()> {
        self.state_mut().leave()?;
        self.state = (self.state + 1).clamp(0, 2);
        self.state_mut().enter()?;
        Ok(())
    }

    fn prev(&mut self) -> Result<()> {
        self.state_mut().leave()?;
        self.state = (self.state as isize - 1).clamp(0, 2) as usize;
        self.state_mut().enter()?;
        Ok(())
    }
}

impl AlliumLauncher<DefaultPlatform> {
    pub fn new() -> Result<AlliumLauncher<DefaultPlatform>> {
        let mut platform = DefaultPlatform::new()?;
        let display = platform.display()?;
        let battery = platform.battery()?;
        let database = Database::new()?;

        let mut core_mapper = DeviceMapper::new();
        core_mapper.load_config()?;
        let core_mapper = Rc::new(core_mapper);

        let mut state = Self::load()?;
        state
            .states
            .0
            .init(Rc::clone(&core_mapper), database.clone());
        state.states.1.init(core_mapper, database);

        Ok(AlliumLauncher {
            platform,
            display,
            battery,
            styles: Stylesheet::load()?,
            state,
            dirty: true,
        })
    }

    fn load() -> Result<AlliumLauncherState> {
        if ALLIUM_LAUNCHER_STATE.exists() {
            debug!("found state, loading from file");
            if let Ok(json) = fs::read_to_string(ALLIUM_LAUNCHER_STATE.as_path()) {
                if let Ok(json) = serde_json::from_str(&json) {
                    return Ok(json);
                }
            }
            warn!("failed to read state file, removing");
            fs::remove_file(ALLIUM_LAUNCHER_STATE.as_path())?;
        }
        AlliumLauncherState::new()
    }

    fn save(&self) -> Result<()> {
        let json = serde_json::to_string(&self.state).unwrap();
        File::create(ALLIUM_LAUNCHER_STATE.as_path())?.write_all(json.as_bytes())?;
        Ok(())
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        self.display.clear(self.styles.background_color)?;
        self.display.save()?;

        self.state.state_mut().enter()?;

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
                self.draw()?;
                self.state
                    .state_mut()
                    .draw(&mut self.display, &self.styles)?;
                self.display.flush()?;
                self.dirty = false;
            }

            #[cfg(unix)]
            {
                let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;
                tokio::select! {
                    _ = sigterm.recv() => {
                        self.save()?;
                        process::exit(0);
                    }
                    key_event = self.platform.poll() => {
                        self.handle_key_event(key_event?).await?;
                    }
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
        let state = self.state.state_mut();
        let (command, dirty) = match key_event {
            Some(KeyEvent::Pressed(Key::L)) => {
                self.state.prev()?;
                (None, true)
            }
            Some(KeyEvent::Pressed(Key::R)) => {
                self.state.next()?;
                (None, true)
            }
            Some(key_event) => state.handle_key_event(key_event)?,
            None => (None, false),
        };

        if dirty {
            self.dirty = true;
        }

        if let Some(command) = command {
            self.handle_command(command).await?;
        }

        Ok(())
    }

    async fn handle_command(&mut self, command: AlliumCommand) -> Result<()> {
        trace!("received command: {:?}", &command);
        match command {
            AlliumCommand::Exec(mut cmd) => {
                self.save()?;
                self.display.load(self.display.bounding_box())?;
                self.display.flush()?;
                trace!("executing command: {:?}", cmd);
                #[cfg(unix)]
                cmd.exec();
                #[cfg(not(unix))]
                cmd.spawn()?;
            }
            AlliumCommand::SaveStylesheet(styles) => {
                styles.save()?;
                self.display.clear(styles.background_color)?;
                self.display.save()?;
                self.styles = *styles;
                self.dirty = true;
            }
            AlliumCommand::SaveDisplaySettings(settings) => {
                settings.save()?;
                self.platform.set_display_settings(&settings)?;
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
            .background_color(self.styles.background_color)
            .build();

        let primary_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(self.styles.ui_font_size)
            .text_color(self.styles.highlight_color)
            .background_color(self.styles.background_color)
            .build();

        // Draw battery percentage
        self.display
            .load(Rectangle::new(Point::new(508, 3), Size::new(120, 40)))?;
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
        for (i, text) in ["Games", "Recents", "Settings"].iter().enumerate() {
            let text = Text::with_alignment(
                text,
                Point { x, y: 8 },
                if i == self.state.state {
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
