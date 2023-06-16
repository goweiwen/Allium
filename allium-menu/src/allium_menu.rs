use std::collections::VecDeque;
use std::process;

use anyhow::Result;
use common::command::Command;
use common::display::color::Color;
use common::display::Display;
use common::game_info::GameInfo;
use common::platform::{DefaultPlatform, Platform};
use common::stylesheet::Stylesheet;
use common::view::View;
use embedded_graphics::prelude::*;
use tracing::warn;

use crate::view::IngameMenu;

#[cfg(unix)]
use tokio::signal::unix::SignalKind;

pub struct AlliumMenu<P>
where
    P: Platform,
{
    platform: P,
    display: P::Display,
    styles: Stylesheet,
    view: IngameMenu<P::Battery>,
}

impl AlliumMenu<DefaultPlatform> {
    pub fn new(mut platform: DefaultPlatform) -> Result<Self> {
        let display = platform.display()?;
        let battery = platform.battery()?;
        let rect = display.bounding_box().into();

        let game_info = GameInfo::load()?;
        let name = game_info.map(|game| game.name).unwrap_or("".to_string());

        Ok(AlliumMenu {
            platform,
            display,
            styles: Stylesheet::load()?,
            view: IngameMenu::new(rect, name, battery),
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        self.display
            .map_pixels(|pixel| pixel.blend(self.styles.background_color.overlay(pixel), 192))?;
        self.display.save()?;

        #[cfg(unix)]
        let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;

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
                self.display.clear(Color::new(0, 0, 0))?;
                self.display.flush()?;
                process::exit(0);
            }
            command => {
                warn!("unhandled command: {:?}", command);
            }
        }
        Ok(())
    }
}
