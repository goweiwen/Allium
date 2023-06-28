use std::fs::{self, File};
use std::io::Write;
use std::ops::Add;
use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};
use common::battery::Battery;
use common::constants::{
    ALLIUMD_STATE, ALLIUM_GAME_INFO, ALLIUM_LAUNCHER, ALLIUM_MENU, ALLIUM_VERSION,
    AUTO_SLEEP_TIMEOUT, BATTERY_SHUTDOWN_THRESHOLD, BATTERY_UPDATE_INTERVAL,
};
use common::wifi::WiFiSettings;
use enum_map::EnumMap;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use tokio::process::{Child, Command};

use common::database::Database;
use common::game_info::GameInfo;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use tokio::time::Instant;

#[cfg(unix)]
use {
    futures::future::{Fuse, FutureExt},
    nix::sys::signal::kill,
    nix::sys::signal::Signal,
    nix::unistd::Pid,
    tokio::signal::unix::SignalKind,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct AlliumD<P: Platform> {
    #[serde(skip)]
    platform: P,
    #[serde(skip, default = "spawn_main")]
    main: Child,
    #[serde(skip)]
    menu: Option<Child>,
    #[serde(skip)]
    keys: EnumMap<Key, bool>,
    #[serde(skip)]
    is_menu_pressed_alone: bool,
    #[serde(skip)]
    is_terminating: bool,
    #[serde(default = "Utc::now")]
    time: DateTime<Utc>,
    volume: i32,
    brightness: u8,
}

fn spawn_main() -> Child {
    match GameInfo::load().unwrap() {
        Some(mut game_info) => {
            debug!("found game info, resuming game");
            game_info.start_time = Utc::now();
            game_info.save().unwrap();
            game_info.command().into()
        }
        None => {
            debug!("no game info found, launching launcher");
            Command::new(ALLIUM_LAUNCHER.as_path())
        }
    }
    .spawn()
    .unwrap()
}

impl AlliumD<DefaultPlatform> {
    pub fn new() -> Result<AlliumD<DefaultPlatform>> {
        let platform = DefaultPlatform::new()?;

        Ok(AlliumD {
            platform,
            main: spawn_main(),
            menu: None,
            keys: EnumMap::default(),
            is_menu_pressed_alone: false,
            is_terminating: false,
            time: Utc::now(),
            volume: 0,
            brightness: 50,
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        info!("hello from Allium {}", ALLIUM_VERSION);

        self.platform.set_volume(self.volume)?;
        self.platform.set_brightness(self.brightness)?;
        WiFiSettings::load()?.init()?;

        #[cfg(unix)]
        {
            let mut sigint = tokio::signal::unix::signal(SignalKind::interrupt())?;
            let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;

            let mut battery_interval = tokio::time::interval(BATTERY_UPDATE_INTERVAL);
            let mut battery = self.platform.battery()?;

            let auto_sleep_timer = tokio::time::sleep(AUTO_SLEEP_TIMEOUT);
            tokio::pin!(auto_sleep_timer);

            loop {
                let menu_terminated = match self.menu.as_mut() {
                    Some(menu) => menu.wait().fuse(),
                    None => Fuse::terminated(),
                };

                tokio::select! {
                    key_event = self.platform.poll() => {
                        auto_sleep_timer.as_mut().reset(Instant::now().add(AUTO_SLEEP_TIMEOUT));
                        self.handle_key_event(key_event).await?;
                    }
                    _ = self.main.wait() => {
                        if !self.is_terminating {
                            info!("main process terminated, recording play time");
                            self.update_play_time()?;
                            GameInfo::delete()?;
                            self.main = spawn_main();
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
                    _ = &mut auto_sleep_timer => {
                        auto_sleep_timer.as_mut().reset(Instant::now().add(AUTO_SLEEP_TIMEOUT));
                        let mut battery = self.platform.battery()?;
                        battery.update()?;
                        if !battery.charging() {
                            info!("auto sleep timer expired, shutting down");
                            self.handle_quit().await?;
                        }
                    }
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
            self.menu.as_ref().map(|c| c.id()),
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
                self.is_terminating = true;
                self.handle_quit().await?;
            }
            KeyEvent::Pressed(Key::Menu) => {
                self.is_menu_pressed_alone = true;
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
                                #[cfg(unix)]
                                signal(&self.main, Signal::SIGSTOP)?;
                                self.menu = Some(Command::new(ALLIUM_MENU.as_path()).spawn()?);
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
        debug!("terminating, saving state");
        if let Some(menu) = self.menu.as_mut() {
            menu.kill().await?;
        }
        if self.is_ingame() {
            if self.menu.is_some() {
                #[cfg(unix)]
                signal(&self.main, Signal::SIGCONT)?;
            }
            #[cfg(unix)]
            signal(&self.main, Signal::SIGTERM)?;
            self.main.wait().await?;
        }
        self.time = Utc::now();
        self.save()?;
        self.update_play_time()?;
        self.platform.shutdown()?;
        Ok(())
    }

    pub fn load() -> Result<AlliumD<DefaultPlatform>> {
        if ALLIUMD_STATE.exists() {
            debug!("found state, loading from file");
            if let Ok(json) = fs::read_to_string(ALLIUMD_STATE.as_path()) {
                if let Ok(this) = serde_json::from_str::<AlliumD<_>>(&json) {
                    if Utc::now() < this.time {
                        info!(
                            "RTC is not working, advancing time to {}",
                            this.time.format("%F %T")
                        );
                        Command::new("date")
                            .arg("-s")
                            .arg(this.time.format("%F %T").to_string())
                            .spawn()?;
                    }
                    return Ok(this);
                }
            }
            warn!("failed to read state file, removing");
            fs::remove_file(ALLIUMD_STATE.as_path())?;
        }
        Self::new()
    }

    fn save(&self) -> Result<()> {
        let json = serde_json::to_string(self).unwrap();
        File::create(ALLIUMD_STATE.as_path())?.write_all(json.as_bytes())?;
        Ok(())
    }

    #[allow(unused)]
    fn update_play_time(&self) -> Result<()> {
        if !self.is_ingame() {
            return Ok(());
        }

        let file = File::open(ALLIUM_GAME_INFO.as_path())?;
        let mut game_info: GameInfo = serde_json::from_reader(file)?;

        let database = Database::new()?;
        database.add_play_time(game_info.path.as_path(), game_info.play_time());

        Ok(())
    }

    fn is_ingame(&self) -> bool {
        Path::new(&*ALLIUM_GAME_INFO).exists()
    }

    fn add_volume(&mut self, add: i32) -> Result<()> {
        info!("adding volume: {}", add);
        self.volume = (self.volume + add).clamp(0, 20);
        self.platform.set_volume(self.volume)?;
        Ok(())
    }

    fn add_brightness(&mut self, add: i8) -> Result<()> {
        info!("adding brightness: {}", add);
        self.brightness = (self.brightness as i8 + add).clamp(0, 100) as u8;
        self.platform.set_brightness(self.brightness)?;
        Ok(())
    }
}

async fn terminate(child: &mut Child) -> Result<()> {
    #[cfg(unix)]
    signal(child, Signal::SIGTERM)?;
    #[cfg(not(unix))]
    child.kill().await?;
    child.wait().await?;
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
