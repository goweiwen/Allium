use std::collections::VecDeque;
use std::process;

use anyhow::Result;
use common::command::Command;
use common::display::color::Color;
use common::geom;
use common::locale::{Locale, LocaleSettings};
use common::resources::Resources;
use common::view::View;
use embedded_graphics::prelude::*;
use log::{debug, warn};

use common::database::Database;
use common::display::Display;
use common::platform::{DefaultPlatform, Platform};
use common::stylesheet::Stylesheet;
use type_map::TypeMap;

use crate::consoles::ConsoleMapper;
use crate::view::App;

#[derive(Debug)]
pub struct AlliumLauncher<P: Platform> {
    platform: P,
    display: P::Display,
    res: Resources,
    view: App<P::Battery>,
}

impl AlliumLauncher<DefaultPlatform> {
    pub fn new(mut platform: DefaultPlatform) -> Result<Self> {
        let display = platform.display()?;
        let battery = platform.battery()?;

        let mut console_mapper = ConsoleMapper::new();
        console_mapper.load_config()?;

        let mut res = TypeMap::new();
        res.insert(Database::new()?);
        res.insert(console_mapper);
        res.insert(Stylesheet::load()?);
        res.insert(Locale::new(&LocaleSettings::load()?.lang));
        res.insert(Into::<geom::Size>::into(display.size()));
        let res = Resources::new(res);

        let view = App::load_or_new(display.bounding_box().into(), res.clone(), battery)?;

        Ok(AlliumLauncher {
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
                debug!("goodbye from allium launcher");
                self.view.save()?;
                self.display.clear(Color::new(0, 0, 0))?;
                self.display.flush()?;
                process::exit(0);
            }
            #[allow(unused_mut)]
            Command::Exec(mut cmd) => {
                debug!("executing command: {:?}", cmd);
                self.view.save()?;
                self.display.clear(Color::new(0, 0, 0))?;
                self.display.flush()?;
                #[cfg(feature = "miyoo")]
                {
                    use std::os::unix::process::CommandExt;
                    cmd.exec();
                }
                #[cfg(not(feature = "miyoo"))]
                {
                    #[cfg(unix)]
                    use std::os::unix::process::CommandExt;
                    process::Command::new("/bin/sh")
                        .arg("-c")
                        .arg("make simulator-menu")
                        .exec();

                    #[cfg(not(unix))]
                    process::exit();
                }
            }
            Command::SaveStylesheet(mut styles) => {
                debug!("saving stylesheet");
                styles.load_fonts()?;
                styles.save()?;
                self.display.clear(styles.background_color)?;
                self.display.save()?;
                self.res.insert(*styles);
                self.view.save()?;
                self.view = App::load_or_new(
                    self.display.bounding_box().into(),
                    self.res.clone(),
                    self.platform.battery()?,
                )?;
            }
            Command::SaveDisplaySettings(settings) => {
                debug!("saving display settings");
                settings.save()?;
                self.platform.set_display_settings(&settings)?;
            }
            Command::SaveLocaleSettings(settings) => {
                debug!("saving locale settings");
                settings.save()?;
                self.res.insert(Locale::new(&settings.lang));
                self.view.save()?;
                self.view = App::load_or_new(
                    self.display.bounding_box().into(),
                    self.res.clone(),
                    self.platform.battery()?,
                )?;
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
