use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use common::battery::Battery;
use common::constants::{
    ALLIUMD_STATE, ALLIUM_GAME_INFO, ALLIUM_MENU, ALLIUM_VERSION, BATTERY_SHUTDOWN_THRESHOLD,
    BATTERY_UPDATE_INTERVAL,
};
use common::display::settings::DisplaySettings;
use common::retroarch::RetroArchCommand;
use common::wifi::WiFiSettings;
use enum_map::EnumMap;
use futures::future::join3;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use tokio::process::{Child, Command};

use common::database::Database;
use common::game_info::GameInfo;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use tokio::select;

#[cfg(unix)]
use {
    futures::future::{Fuse, FutureExt},
    nix::sys::signal::kill,
    nix::sys::signal::Signal,
    nix::unistd::Pid,
    tokio::signal::unix::SignalKind,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlliumDState {
    #[serde(default = "Utc::now")]
    time: DateTime<Utc>,
    volume: i32,
    brightness: u8,
}

#[derive(Debug)]
pub struct AlliumD<P: Platform> {
    platform: P,
    main: Child,
    menu: Option<Child>,
    keys: EnumMap<Key, bool>,
    is_menu_pressed_alone: bool,
    is_terminating: bool,
    state: AlliumDState,
}

impl AlliumDState {
    pub fn new() -> Self {
        Self {
            time: Utc::now(),
            volume: 0,
            brightness: 50,
        }
    }

    pub fn load() -> Result<AlliumDState> {
        if ALLIUMD_STATE.exists() {
            debug!("found state, loading from file");
            if let Ok(json) = fs::read_to_string(ALLIUMD_STATE.as_path()) {
                if let Ok(this) = serde_json::from_str::<AlliumDState>(&json) {
                    if Utc::now() < this.time {
                        info!(
                            "RTC is not working, advancing time to {}",
                            this.time.format("%F %T")
                        );
                        std::process::Command::new("/sbin/hwclock")
                            .arg("-w")
                            .arg("-u")
                            .arg(this.time.format("%F %T").to_string())
                            .spawn()?
                            .wait()?;
                    }
                    return Ok(this);
                }
            }
            warn!("failed to read state file, removing");
            fs::remove_file(ALLIUMD_STATE.as_path())?;
        }
        Ok(Self::new())
    }

    fn save(&self) -> Result<()> {
        let json = serde_json::to_string(self).unwrap();
        File::create(ALLIUMD_STATE.as_path())?.write_all(json.as_bytes())?;
        Ok(())
    }
}

fn spawn_main() -> Result<Child> {
    #[cfg(feature = "miyoo")]
    return Ok(match GameInfo::load()? {
        Some(mut game_info) => {
            debug!("found game info, resuming game");
            game_info.start_time = Utc::now();
            game_info.save()?;
            game_info.command().into()
        }
        None => {
            debug!("no game info found, launching launcher");
            use common::constants::ALLIUM_LAUNCHER;
            Command::new(ALLIUM_LAUNCHER.as_path())
        }
    }
    .spawn()?);

    #[cfg(not(feature = "miyoo"))]
    return Ok(Command::new("/bin/sh")
        .arg("-c")
        .arg("make simulator-launcher")
        .spawn()?);
}

impl AlliumD<DefaultPlatform> {
    pub fn new() -> Result<AlliumD<DefaultPlatform>> {
        let platform = DefaultPlatform::new()?;
        let state = AlliumDState::load()?;
        let main = spawn_main()?;

        Ok(AlliumD {
            platform,
            main,
            menu: None,
            keys: EnumMap::default(),
            is_menu_pressed_alone: false,
            is_terminating: false,
            state,
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        info!("hello from Allium {}", ALLIUM_VERSION);

        self.platform.set_volume(self.state.volume)?;
        self.platform.set_brightness(self.state.brightness)?;

        DisplaySettings::load()?.apply()?;

        if DefaultPlatform::has_wifi() {
            WiFiSettings::load()?.init()?;
        }

        #[cfg(unix)]
        {
            let mut sigint = tokio::signal::unix::signal(SignalKind::interrupt())?;
            let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;

            let mut battery_interval = tokio::time::interval(BATTERY_UPDATE_INTERVAL);
            let mut battery = self.platform.battery()?;

            loop {
                let menu_terminated = match self.menu.as_mut() {
                    Some(menu) => menu.wait().fuse(),
                    None => Fuse::terminated(),
                };

                tokio::select! {
                    key_event = self.platform.poll() => {
                        self.handle_key_event(key_event).await?;
                    }
                    _ = self.main.wait() => {
                        if !self.is_terminating {
                            info!("main process terminated, recording play time");
                            self.update_play_time()?;
                            GameInfo::delete()?;
                            self.main = spawn_main()?;
                        }
                    }
                    _ = menu_terminated => {
                        info!("menu process terminated, resuming game");
                        self.menu = None;
                        #[cfg(unix)]
                        signal(&self.main, Signal::SIGCONT)?;
                    }
                    _ = sigint.recv() => self.handle_quit().await?,
                    _ = sigterm.recv() => self.handle_quit().await?,
                    _ = battery_interval.tick() => {
                        if let Err(e) = battery.update() {
                            error!("failed to update battery: {}", e);
                        }
                        if battery.percentage() <= BATTERY_SHUTDOWN_THRESHOLD && !battery.charging() {
                            warn!("battery is low, shutting down");
                            self.handle_quit().await?;
                        }
                    }
                }
            }
        }

        #[cfg(not(unix))]
        loop {
            tokio::select! {
                key_event = self.platform.poll() => {
                    self.handle_key_event(key_event).await?;
                }
            }
        }
    }

    async fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        trace!(
            "menu: {:?}, main: {:?}, ingame: {}",
            self.menu.as_ref().map(tokio::process::Child::id),
            self.main.id(),
            self.is_ingame()
        );
        match key_event {
            KeyEvent::Pressed(Key::Menu) => {
                self.is_menu_pressed_alone = true;
            }
            KeyEvent::Pressed(_) => {
                self.is_menu_pressed_alone = false;
            }
            KeyEvent::Released(_) | KeyEvent::Autorepeat(_) => {}
        }
        match key_event {
            KeyEvent::Pressed(key) => {
                self.keys[key] = true;
            }
            KeyEvent::Released(key) => {
                self.keys[key] = false;
            }
            KeyEvent::Autorepeat(_) => {}
        }
        match key_event {
            KeyEvent::Pressed(Key::L2) | KeyEvent::Autorepeat(Key::L2) => {
                if self.keys[Key::Start] {
                    self.add_brightness(-5)?;
                } else if self.keys[Key::Select] {
                    self.add_volume(-1)?
                }
            }
            KeyEvent::Pressed(Key::R2) | KeyEvent::Autorepeat(Key::R2) => {
                if self.keys[Key::Start] {
                    self.add_brightness(5)?;
                } else if self.keys[Key::Select] {
                    self.add_volume(1)?
                }
            }
            KeyEvent::Pressed(Key::VolDown) | KeyEvent::Autorepeat(Key::VolDown) => {
                if self.keys[Key::Menu] {
                    self.add_brightness(-5)?;
                } else {
                    self.add_volume(-1)?
                }
            }
            KeyEvent::Pressed(Key::VolUp) | KeyEvent::Autorepeat(Key::VolUp) => {
                if self.keys[Key::Menu] {
                    self.add_brightness(5)?;
                } else {
                    self.add_volume(1)?
                }
            }
            KeyEvent::Autorepeat(Key::Power) => {
                self.handle_quit().await?;
            }
            KeyEvent::Released(Key::Menu) => {
                if self.is_menu_pressed_alone {
                    if self.is_ingame()
                        && self
                            .keys
                            .iter()
                            .all(|(k, pressed)| k == Key::Menu || !pressed)
                    {
                        if let Some(game_info) = GameInfo::load()? {
                            if let Some(menu) = &mut self.menu {
                                terminate(menu).await?;
                            } else if game_info.has_menu {
                                // TODO: combine these into one command?
                                let (max_disk_slots, disk_slot, state_slot) = join3(
                                    RetroArchCommand::GetDiskCount.send_recv(),
                                    RetroArchCommand::GetDiskSlot.send_recv(),
                                    RetroArchCommand::GetStateSlot.send_recv(),
                                )
                                .await;

                                let max_disk_slots = max_disk_slots?;
                                let max_disk_slots = max_disk_slots
                                    .as_ref()
                                    .and_then(|s| s.split_ascii_whitespace().skip(1).next())
                                    .unwrap_or("0");

                                let disk_slot = disk_slot?;
                                let disk_slot = disk_slot
                                    .as_ref()
                                    .and_then(|s| s.split_ascii_whitespace().skip(1).next())
                                    .unwrap_or("0");

                                let state_slot = state_slot?;
                                let state_slot = state_slot
                                    .as_ref()
                                    .and_then(|s| s.split_ascii_whitespace().skip(1).next())
                                    .unwrap_or("0");

                                #[cfg(unix)]
                                signal(&self.main, Signal::SIGSTOP)?;
                                self.menu = Some(
                                    Command::new(ALLIUM_MENU.as_path())
                                        .args([disk_slot, max_disk_slots, state_slot])
                                        .spawn()?,
                                );
                            }
                        }
                    }
                    self.is_menu_pressed_alone = false;
                }
            }
            _ => {}
        }

        Ok(())
    }

    #[cfg(unix)]
    async fn handle_quit(&mut self) -> Result<()> {
        if self.is_terminating {
            return Ok(());
        }

        debug!("terminating, saving state");

        self.state.time = Utc::now();
        self.state.save()?;

        if self.is_ingame() {
            self.update_play_time()?;

            if let Some(menu) = self.menu.as_mut() {
                #[cfg(unix)]
                signal(&self.main, Signal::SIGCONT)?;
                menu.kill().await?;
            }

            terminate(&mut self.main).await?;
        }

        self.is_terminating = true;
        self.platform.shutdown()?;

        Ok(())
    }

    #[allow(unused)]
    fn update_play_time(&self) -> Result<()> {
        if !self.is_ingame() {
            return Ok(());
        }

        let file = File::open(ALLIUM_GAME_INFO.as_path())?;
        let mut game_info: GameInfo = serde_json::from_reader(file)?;

        // As a sanity check, don't add play time if the game was played for more than 24 hours
        if game_info.play_time() > Duration::hours(24) {
            warn!("play time is too long, not adding to database");
            return Ok(());
        }

        let database = Database::new()?;
        database.add_play_time(game_info.path.as_path(), game_info.play_time());

        Ok(())
    }

    fn is_ingame(&self) -> bool {
        Path::new(&*ALLIUM_GAME_INFO).exists()
    }

    fn add_volume(&mut self, add: i32) -> Result<()> {
        info!("adding volume: {}", add);
        self.state.volume = (self.state.volume + add).clamp(0, 20);
        self.platform.set_volume(self.state.volume)?;
        Ok(())
    }

    fn add_brightness(&mut self, add: i8) -> Result<()> {
        info!("adding brightness: {}", add);
        self.state.brightness = (self.state.brightness as i8 + add).clamp(0, 100) as u8;
        self.platform.set_brightness(self.state.brightness)?;
        Ok(())
    }
}

async fn terminate(child: &mut Child) -> Result<()> {
    #[cfg(unix)]
    signal(child, Signal::SIGTERM)?;
    #[cfg(not(unix))]
    child.kill().await?;

    select! {
        _ = child.wait() => {}
        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
            signal(child, Signal::SIGKILL)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn signal(child: &Child, signal: Signal) -> Result<()> {
    if let Some(pid) = child.id() {
        let pid = Pid::from_raw(pid as i32);
        kill(pid, signal)?;
    }
    Ok(())
}
