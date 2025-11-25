use std::collections::VecDeque;

use anyhow::Result;
use base32::encode;
use common::command::Command;
use common::constants::ALLIUM_SCREENSHOTS_DIR;
use common::database::Database;
use common::display::Display;
use common::game_info::GameInfo;
use common::geom;
use common::locale::{Locale, LocaleSettings};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::View;
use log::{info, warn};
use sha2::{Digest, Sha256};
use type_map::TypeMap;

use crate::retroarch_info::RetroArchInfo;
use crate::view::ingame_menu::IngameMenu;

pub struct AlliumMenu<P>
where
    P: Platform,
{
    platform: P,
    display: P::Display,
    res: Resources,
    view: IngameMenu<P::Battery>,
    a_pressed: bool,
    b_pressed: bool,
}

impl AlliumMenu<DefaultPlatform> {
    pub async fn new(mut platform: DefaultPlatform) -> Result<Self> {
        let display = platform.display()?;
        let battery = platform.battery()?;
        let rect = display.bounding_box();

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
            view: IngameMenu::load_or_new(rect, res, battery, None).await?,
            a_pressed: false,
            b_pressed: false,
        })
    }

    /// Prepare the menu for a new session with fresh RetroArchInfo.
    /// Call this before run_event_loop to refresh game info and state.
    pub async fn prepare(&mut self, info: Option<RetroArchInfo>) -> Result<()> {
        // Refresh game info in case it changed
        self.res.insert(GameInfo::load()?.unwrap_or_default());

        // Recreate the view with fresh info
        let rect = self.display.bounding_box();
        let battery = self.platform.battery()?;
        self.view = IngameMenu::load_or_new(rect, self.res.clone(), battery, info).await?;

        self.a_pressed = false;
        self.b_pressed = false;

        Ok(())
    }

    /// Save the menu state. Call this after run_event_loop returns.
    pub fn save(&mut self) -> Result<()> {
        self.view.save()
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        self.display.sync()?;
        self.display.save()?;
        {
            let styles = self.res.get::<Stylesheet>();
            self.display.map_pixels(|pixel| {
                pixel.blend(styles.menu.background_color.overlay(pixel), 192)
            })?;
        }
        self.display.save()?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        let mut should_exit = false;

        loop {
            if self.view.should_draw() && self.view.draw(&mut self.display, &self.res.get())? {
                self.display.flush()?;
            }

            tokio::select! {
                Some(command) = rx.recv() => {
                    if self.handle_command(command).await? {
                        should_exit = true;
                    }
                }
                event = self.platform.poll() => {
                    match event {
                        KeyEvent::Pressed(Key::A) => self.a_pressed = true,
                        KeyEvent::Released(Key::A) => self.a_pressed = false,
                        KeyEvent::Pressed(Key::B) => self.b_pressed = true,
                        KeyEvent::Released(Key::B) => self.b_pressed = false,
                        _ => {}
                    }
                    let mut bubble = VecDeque::new();
                    self.view.handle_key_event(event, tx.clone(), &mut bubble).await?;
                }
                else => {}
            }

            // Exit only after A and B buttons are released
            if should_exit && !self.a_pressed && !self.b_pressed {
                info!("exiting menu, and b buttons released");
                return Ok(());
            }
        }
    }

    /// Returns true if the menu should exit
    async fn handle_command(&mut self, command: Command) -> Result<bool> {
        match command {
            Command::Exit => {
                self.view.save()?;
                self.display.pop();
                self.display.load(self.display.bounding_box())?;
                self.display.flush()?;
                self.display.pop();
                return Ok(true);
            }
            Command::Redraw => {
                self.display.load(self.display.bounding_box())?;
                self.view.set_should_draw();
            }
            Command::SaveStateScreenshot { path, core, slot } => {
                if self.display.pop() {
                    self.display.load(self.display.bounding_box())?;
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

                    #[cfg(feature = "miyoo")]
                    tokio::process::Command::new("screenshot")
                        .arg(screenshot_path)
                        .arg(format!(
                            "--width={}",
                            common::constants::SAVE_STATE_IMAGE_WIDTH
                        ))
                        .arg("--crop")
                        .spawn()?
                        .wait()
                        .await?;

                    #[cfg(feature = "simulator")]
                    std::fs::copy(
                        common::constants::ALLIUM_SD_ROOT.join("bg-640x480.png"),
                        screenshot_path,
                    )?;
                }
            }
            Command::RetroArchCommand(command) => {
                command.send().await?;
            }
            command => {
                warn!("unhandled command: {:?}", command);
            }
        }
        Ok(false)
    }
}
