use std::collections::VecDeque;
use std::process;
use std::rc::Rc;

use anyhow::Result;
use common::command::Command;
use common::display::color::Color;
use common::view::View;
use embedded_graphics::prelude::*;
use tracing::{debug, warn};

use common::database::Database;
use common::display::Display;
use common::platform::{DefaultPlatform, Platform};
use common::stylesheet::Stylesheet;

use crate::consoles::ConsoleMapper;
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

        let mut console_mapper = ConsoleMapper::new();
        console_mapper.load_config()?;
        let console_mapper = Rc::new(console_mapper);

        let view = App::load_or_new(
            display.bounding_box().into(),
            database,
            console_mapper,
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
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        loop {
            self.view.update()?;

            if self.view.should_draw() && self.view.draw(&mut self.display, &self.styles)? {
                self.display.flush()?;
            }

            #[cfg(unix)]
            tokio::select! {
                _ = sigint.recv() => {
                    self.handle_command(Command::Exit).await?;
                }
                _ = sigterm.recv() => {
                    self.handle_command(Command::Exit).await?;
                }
                event = self.platform.poll() => {
                    let mut bubble = VecDeque::new();
                    self.view.handle_key_event(event, tx.clone(), &mut bubble).await?;
                }
                else => {}
            }

            #[cfg(not(unix))]
            tokio::select! {
                event = self.platform.poll() => {
                    let mut bubble = VecDeque::new();
                    self.view.handle_key_event(event, tx.clone(), &mut bubble).await?;
                }
                else => {}
            }

            while let Ok(cmd) = rx.try_recv() {
                self.handle_command(cmd).await?;
            }
        }
    }

    async fn handle_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::Exit => {
                debug!("goodbye from allium launcher");
                self.view.save()?;
                self.display.clear(Color::new(0, 0, 0))?;
                self.display.flush()?;
                process::exit(0);
            }
            Command::Exec(mut cmd) => {
                debug!("executing command: {:?}", cmd);
                self.view.save()?;
                self.display.clear(Color::new(0, 0, 0))?;
                self.display.flush()?;
                #[cfg(unix)]
                {
                    use std::os::unix::process::CommandExt;
                    cmd.exec();
                }
                #[cfg(not(unix))]
                cmd.spawn()?;
            }
            Command::SaveStylesheet(mut styles) => {
                debug!("saving stylesheet");
                styles.load_fonts()?;
                styles.save()?;
                self.display.clear(styles.background_color)?;
                self.display.save()?;
                self.styles = *styles;
                self.view.set_should_draw();
            }
            Command::SaveDisplaySettings(settings) => {
                debug!("saving display settings");
                settings.save()?;
                self.platform.set_display_settings(&settings)?;
            }
            Command::Redraw => {
                debug!("redrawing");
                self.display.load(self.display.bounding_box().into())?;
                self.view.set_should_draw();
            }
            command => {
                warn!("unhandled command: {:?}", command);
            }
        }
        Ok(())
    }
}
