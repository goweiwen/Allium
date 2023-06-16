use std::collections::VecDeque;
use std::process;
use std::rc::Rc;

use anyhow::Result;
use common::command::Command;
use common::display::color::Color;
use common::view::View;
use embedded_graphics::prelude::*;
use tracing::{trace, warn};

use common::database::Database;
use common::display::Display;
use common::platform::{DefaultPlatform, Platform};
use common::stylesheet::Stylesheet;

use crate::devices::DeviceMapper;
use crate::view::App;

#[derive(Debug)]
pub struct AlliumLauncher<P: Platform> {
    platform: P,
    display: P::Display,
    styles: Stylesheet,
    view: App<P::Battery>,
}

impl AlliumLauncher<DefaultPlatform> {
    pub fn new(mut platform: DefaultPlatform) -> Result<Self> {
        let display = platform.display()?;
        let battery = platform.battery()?;
        let database = Database::new()?;

        let mut device_mapper = DeviceMapper::new();
        device_mapper.load_config()?;
        let device_mapper = Rc::new(device_mapper);

        let view = App::new(
            display.bounding_box().into(),
            database,
            device_mapper,
            battery,
        )?;

        Ok(AlliumLauncher {
            platform,
            display,
            styles: Stylesheet::load()?,
            view,
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        self.display.clear(self.styles.background_color)?;
        self.display.save()?;

        #[cfg(unix)]
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        loop {
            self.view.update()?;

            if self.view.should_draw() && self.view.draw(&mut self.display, &self.styles)? {
                self.display.flush()?;
            }

            #[cfg(unix)]
            tokio::select! {
                _ = sigterm.recv() => {
                    self.handle_command(Command::Exit);
                }
                Some(command) = rx.recv() => {
                    self.handle_command(command).await?;
                }
                event = self.platform.poll() => {
                    let mut bubble = VecDeque::new();
                    self.view.handle_key_event(event, tx.clone(), &mut bubble).await?;
                }
                else => {}
            }

            #[cfg(not(unix))]
            tokio::select! {
                Some(command) = rx.recv() => {
                    self.handle_command(command).await?;
                }
                event = self.platform.poll() => {
                    let mut bubble = VecDeque::new();
                    self.view.handle_key_event(event, tx.clone(), &mut bubble).await?;
                }
                else => {}
            }
        }
    }

    async fn handle_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::Exit => {
                self.view.save()?;
                self.display.clear(Color::new(0, 0, 0))?;
                self.display.flush()?;
                process::exit(0);
            }
            Command::Exec(mut cmd) => {
                self.view.save()?;
                self.display.load(self.display.bounding_box().into())?;
                self.display.flush()?;
                trace!("executing command: {:?}", cmd);
                #[cfg(unix)]
                {
                    use std::os::unix::process::CommandExt;
                    cmd.exec();
                }
                #[cfg(not(unix))]
                cmd.spawn()?;
            }
            Command::SaveStylesheet(styles) => {
                styles.save()?;
                self.display.clear(styles.background_color)?;
                self.display.save()?;
                self.styles = *styles.to_owned();
                self.view.set_should_draw();
            }
            Command::SaveDisplaySettings(settings) => {
                settings.save()?;
                self.platform.set_display_settings(&settings)?;
            }
            command => {
                warn!("unhandled command: {:?}", command);
            }
        }
        Ok(())
    }
}
