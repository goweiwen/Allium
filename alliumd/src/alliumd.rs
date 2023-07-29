use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use common::battery::Battery;
use common::constants::{
    ALLIUMD_STATE, ALLIUM_GAME_INFO, ALLIUM_MENU, ALLIUM_SD_ROOT, ALLIUM_VERSION,
    BATTERY_SHUTDOWN_THRESHOLD, BATTERY_UPDATE_INTERVAL, LONG_PRESS_DURATION,
};
use common::display::settings::DisplaySettings;
use common::locale::{Locale, LocaleSettings};
use common::retroarch::RetroArchCommand;
use common::wifi::WiFiSettings;
use enum_map::EnumMap;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use tokio::process::{Child, Command};

use common::database::Database;
use common::game_info::GameInfo;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};

#[cfg(unix)]
use {
    nix::sys::signal::kill, nix::sys::signal::Signal, nix::unistd::Pid,
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
    pressed_menu: Instant,
    is_terminating: bool,
    state: AlliumDState,
    locale: Locale,
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
                        let mut date = std::process::Command::new("date")
                            .arg("-s")
                            .arg(this.time.format("%F %T").to_string())
                            .spawn()?;
                        let mut hwclock = std::process::Command::new("/sbin/hwclock")
                            .arg("-w")
                            .arg("-u")
                            .arg(this.time.format("%F %T").to_string())
                            .spawn()?;
                        date.wait()?;
                        hwclock.wait()?;
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
        let locale = Locale::new(&LocaleSettings::load()?.lang);

        Ok(AlliumD {
            platform,
            main,
            menu: None,
            keys: EnumMap::default(),
            is_menu_pressed_alone: false,
            pressed_menu: Instant::now(),
            is_terminating: false,
            state,
            locale,
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        info!("hello from Allium {}", ALLIUM_VERSION);

        info!("setting volume: {}", self.state.volume);
        self.platform.set_volume(self.state.volume)?;

        info!("setting brightness: {}", self.state.brightness);
        self.platform.set_brightness(self.state.brightness)?;

        info!("loading display settings");
        DisplaySettings::load()?.apply()?;

        if DefaultPlatform::has_wifi() {
            info!("wifi detected, loading wifi settings");
            WiFiSettings::load()?.init()?;
        }

        info!("starting event loop");
        #[cfg(unix)]
        {
            let mut sigint = tokio::signal::unix::signal(SignalKind::interrupt())?;
            let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;

            let mut battery_interval = tokio::time::interval(BATTERY_UPDATE_INTERVAL);
            let mut battery = self.platform.battery()?;

            loop {
                if let Some(menu) = self.menu.as_mut() {
                    if menu.try_wait()?.is_some() {
                        info!("menu process terminated, resuming game");
                        self.menu = None;
                        RetroArchCommand::Unpause.send().await?;
                    }
                }

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
                    _ = sigint.recv() => self.handle_quit().await?,
                    _ = sigterm.recv() => self.handle_quit().await?,
                    _ = battery_interval.tick() => {
                        trace!("updating battery");
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

        // Handle menu key
        match key_event {
            KeyEvent::Pressed(Key::Menu) => {
                self.is_menu_pressed_alone = true;
                self.pressed_menu = Instant::now();
            }
            KeyEvent::Pressed(_) => {
                self.is_menu_pressed_alone = false;
            }
            KeyEvent::Released(_) | KeyEvent::Autorepeat(_) => {}
        }

        // Update self.keys
        match key_event {
            KeyEvent::Pressed(key) => {
                self.keys[key] = true;
            }
            KeyEvent::Released(key) => {
                self.keys[key] = false;
            }
            KeyEvent::Autorepeat(_) => {}
        }

        if self.keys[Key::Menu] {
            // Global hotkeys
            match key_event {
                KeyEvent::Autorepeat(Key::Menu) => {
                    if self.is_menu_pressed_alone
                        && self.pressed_menu.elapsed() >= LONG_PRESS_DURATION
                    {
                        // Don't show menu
                        self.is_menu_pressed_alone = false;
                        #[cfg(unix)]
                        {
                            signal(&self.main, Signal::SIGSTOP)?;
                            if let Some(menu) = self.menu.as_mut() {
                                signal(menu, Signal::SIGSTOP)?;
                            }
                        }
                        Command::new("show-hotkeys").spawn()?.wait().await?;
                        #[cfg(unix)]
                        {
                            signal(&self.main, Signal::SIGCONT)?;
                            if let Some(menu) = self.menu.as_mut() {
                                signal(menu, Signal::SIGCONT)?;
                            }
                        }
                    }
                }
                KeyEvent::Pressed(Key::Up | Key::VolUp)
                | KeyEvent::Autorepeat(Key::Up | Key::VolUp) => {
                    self.add_brightness(5)?;
                }
                KeyEvent::Pressed(Key::Down | Key::VolDown)
                | KeyEvent::Autorepeat(Key::Down | Key::VolDown) => {
                    self.add_brightness(-5)?;
                }
                KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                    self.add_volume(-1)?;
                }
                KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                    self.add_volume(1)?;
                }
                KeyEvent::Released(Key::Power) => {
                    let game_info = GameInfo::load()?;
                    let name = match game_info.as_ref() {
                        Some(game_info) => game_info.name.as_str(),
                        None => "Allium",
                    };
                    let file_name = format!(
                        "{}-{}.png",
                        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"),
                        name,
                    );
                    Command::new("screenshot")
                        .arg(ALLIUM_SD_ROOT.join("Screenshots").join(file_name))
                        .spawn()?
                        .wait()
                        .await?;
                }
                _ => {}
            }
        } else {
            match key_event {
                KeyEvent::Pressed(Key::VolDown) | KeyEvent::Autorepeat(Key::VolDown) => {
                    self.add_volume(-1)?
                }
                KeyEvent::Pressed(Key::VolUp) | KeyEvent::Autorepeat(Key::VolUp) => {
                    self.add_volume(1)?
                }
                KeyEvent::Autorepeat(Key::Power) => {
                    if !self.keys[Key::Menu] {
                        self.handle_quit().await?;
                    }
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
                                    self.menu = Some(Command::new(ALLIUM_MENU.as_path()).spawn()?);
                                }
                            }
                        }
                        self.is_menu_pressed_alone = false;
                    }
                }
                _ => {}
            }
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
                terminate(menu).await?;
            }

            terminate(&mut self.main).await?;
        }

        self.is_terminating = true;

        Command::new("show").arg("--darken").spawn()?.wait().await?;
        Command::new("say")
            .arg(self.locale.t("powering-off"))
            .spawn()?
            .wait()
            .await?;
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;

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

#[allow(clippy::needless_pass_by_ref_mut)]
async fn terminate(child: &mut Child) -> Result<()> {
    #[cfg(unix)]
    signal(child, Signal::SIGTERM)?;
    #[cfg(not(unix))]
    child.kill().await?;

    if let Err(_e) = tokio::time::timeout(std::time::Duration::from_secs(1), child.wait()).await {
        signal(child, Signal::SIGKILL)?;
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
