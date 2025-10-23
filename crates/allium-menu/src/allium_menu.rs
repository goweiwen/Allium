use std::collections::VecDeque;

use anyhow::Result;
use base32::encode;
use common::command::Command;
use common::constants::{ALLIUM_SCREENSHOTS_DIR, SAVE_STATE_IMAGE_WIDTH};
use common::database::Database;
use common::display::Display;
use common::game_info::GameInfo;
use common::geom;
use common::locale::{Locale, LocaleSettings};
use common::platform::{DefaultPlatform, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::View;
use embedded_graphics::prelude::*;
use log::{info, warn};
use sha2::{Digest, Sha256};
use type_map::TypeMap;

use crate::retroarch_info::RetroArchInfo;
use crate::view::ingame_menu::IngameMenu;

#[cfg(unix)]
use tokio::signal::unix::SignalKind;

pub struct AlliumMenu<P>
where
    P: Platform,
{
    platform: P,
    display: P::Display,
    res: Resources,
    view: IngameMenu<P::Battery>,
}

impl AlliumMenu<DefaultPlatform> {
    pub async fn new(mut platform: DefaultPlatform, info: Option<RetroArchInfo>) -> Result<Self> {
        let display = platform.display()?;
        let battery = platform.battery()?;
        let rect = display.bounding_box().into();

        let mut res = TypeMap::new();
        res.insert(Database::new()?);
        res.insert(GameInfo::load()?.unwrap_or_default());
        res.insert(Stylesheet::load()?);
        res.insert(Locale::new(&LocaleSettings::load()?.lang));
        res.insert(Into::<geom::Size>::into(display.size()));
        let res = Resources::new(res);

        Ok(AlliumMenu {
            platform,
            display,
            res: res.clone(),
            view: IngameMenu::load_or_new(rect, res, battery, info).await?,
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        self.display.save()?;
        {
            let styles = self.res.get::<Stylesheet>();
            self.display
                .map_pixels(|pixel| pixel.blend(styles.background_color.overlay(pixel), 192))?;
        }
        self.display.save()?;

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
                self.view.save()?;
                if self.display.pop() {
                    self.display.load(self.display.bounding_box().into())?;
                    self.display.flush()?;
                }
                std::process::exit(0);
            }
            Command::Redraw => {
                self.display.load(self.display.bounding_box().into())?;
                self.view.set_should_draw();
            }
            Command::SaveStateScreenshot { path, core, slot } => {
                if self.display.pop() {
                    self.display.load(self.display.bounding_box().into())?;
                    self.display.flush()?;

                    let mut hasher = Sha256::new();
                    hasher.update(&path);
                    hasher.update(&core);
                    hasher.update(slot.to_le_bytes());
                    let hash = hasher.finalize();
                    let base32 = encode(base32::Alphabet::Crockford, &hash);
                    let file_name = format!("{}.png", base32);

                    std::fs::create_dir_all(&*ALLIUM_SCREENSHOTS_DIR).ok();

                    let screenshot_path = ALLIUM_SCREENSHOTS_DIR.join(&file_name);
                    info!("saving screenshot to {:?}", screenshot_path);

                    let database = self.res.get::<Database>();
                    let game_path = std::path::Path::new(&path);
                    database
                        .update_screenshot_path(game_path, Some(&screenshot_path))
                        .ok();

                    std::process::Command::new("screenshot")
                        .arg(screenshot_path)
                        .arg(format!("--width={}", SAVE_STATE_IMAGE_WIDTH))
                        .arg("--crop")
                        .spawn()?;
                }
            }
            command => {
                warn!("unhandled command: {:?}", command);
            }
        }
        Ok(())
    }
}
