use std::collections::VecDeque;
use std::process;

use anyhow::Result;
use common::command::Command;
use common::display::Display;
use common::locale::{Locale, LocaleSettings};
use common::platform::{DefaultPlatform, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::View;
use embedded_graphics::prelude::*;
use log::warn;
use type_map::TypeMap;

use crate::view::hotkeys::Hotkeys;

#[cfg(unix)]
use tokio::signal::unix::SignalKind;

pub struct App<P>
where
    P: Platform,
{
    platform: P,
    display: P::Display,
    res: Resources,
    view: Hotkeys,
}

impl App<DefaultPlatform> {
    pub async fn new(mut platform: DefaultPlatform) -> Result<Self> {
        let display = platform.display()?;
        let rect = display.bounding_box().into();

        let mut res = TypeMap::new();
        res.insert(Stylesheet::load()?);
        res.insert(Locale::new(&LocaleSettings::load()?.lang));
        let res = Resources::new(res);

        Ok(App {
            platform,
            display,
            res: res.clone(),
            view: Hotkeys::new(rect, res),
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        {
            let styles = self.res.get::<Stylesheet>();
            self.display
                .map_pixels(|pixel| pixel.blend(styles.background_color.overlay(pixel), 192))?;
            self.display.save()?;
        }

        #[cfg(unix)]
        let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        loop {
            if self.view.should_draw() && self.view.draw(&mut self.display, &self.res.get())? {
                self.display.flush()?;
            }

            #[cfg(unix)]
            tokio::select! {
                _ = sigterm.recv() => {
                    self.handle_command(Command::Exit)?;
                }
                Some(command) = rx.recv() => {
                    self.handle_command(command)?;
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
                    self.handle_command(command)?;
                }
                event = self.platform.poll() => {
                    let mut bubble = VecDeque::new();
                    self.view.handle_key_event(event, tx.clone(), &mut bubble).await?;
                }
                else => {}
            }
        }
    }

    fn handle_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::Exit => {
                process::exit(0);
            }
            Command::Redraw => {
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
