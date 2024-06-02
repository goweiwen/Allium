use std::collections::VecDeque;
use std::process;

use anyhow::Result;
use common::command::Command;
use common::geom;
use common::locale::{Locale, LocaleSettings};
use common::resources::Resources;
use common::view::View;
use embedded_graphics::prelude::*;
use log::{trace, warn};

use common::database::Database;
use common::display::Display;
use common::platform::{DefaultPlatform, Platform};
use common::stylesheet::Stylesheet;
use type_map::TypeMap;

use crate::view::App;

#[derive(Debug)]
pub struct ActivityTracker<P: Platform> {
    platform: P,
    display: P::Display,
    res: Resources,
    view: App<P::Battery>,
}

impl ActivityTracker<DefaultPlatform> {
    pub fn new(mut platform: DefaultPlatform) -> Result<Self> {
        let display = platform.display()?;
        let battery = platform.battery()?;

        let mut res = TypeMap::new();
        res.insert(Database::new()?);
        res.insert(Stylesheet::load()?);
        res.insert(Locale::new(&LocaleSettings::load()?.lang));
        res.insert(Into::<geom::Size>::into(display.size()));
        let res = Resources::new(res);

        let view = App::new(display.bounding_box().into(), res.clone(), battery)?;

        Ok(ActivityTracker {
            platform,
            display,
            res,
            view,
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        self.display
            .clear(self.res.get::<Stylesheet>().background_color)?;
        self.display.save()?;

        #[cfg(unix)]
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        loop {
            if self.view.should_draw()
                && self
                    .view
                    .draw(&mut self.display, &self.res.get::<Stylesheet>())?
            {
                self.display.flush()?;
            }

            #[cfg(unix)]
            tokio::select! {
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
                process::exit(0);
            }
            Command::Redraw => {
                trace!("redrawing");
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
