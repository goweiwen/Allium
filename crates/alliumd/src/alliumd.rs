use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Instant;

use allium_menu::{AlliumMenu, RetroArchInfo};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use common::battery::Battery;
use common::constants::{
    ALLIUM_GAME_INFO, ALLIUM_SD_ROOT, ALLIUM_VERSION, ALLIUMD_STATE, BATTERY_SHUTDOWN_THRESHOLD,
    BATTERY_UPDATE_INTERVAL, BATTERY_WARNING_THRESHOLD, IDLE_TIMEOUT,
};
use common::display::settings::DisplaySettings;
use common::locale::{Locale, LocaleSettings};
use common::power::{PowerButtonAction, PowerSettings};
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
    nix::sys::signal::Signal, nix::sys::signal::kill, nix::unistd::Pid,
    tokio::signal::unix::SignalKind,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlliumDState {
    #[serde(default = "Utc::now")]
    time: DateTime<Utc>,
    volume: i32,
    brightness: u8,
}

/// Handle to the persistent menu thread
struct MenuHandle {
    tx: mpsc::Sender<Option<RetroArchInfo>>,
    done_rx: tokio::sync::mpsc::UnboundedReceiver<()>,
    _handle: JoinHandle<Result<()>>,
}

impl MenuHandle {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel::<Option<RetroArchInfo>>();
        let (done_tx, done_rx) = tokio::sync::mpsc::unbounded_channel();
        let rt = tokio::runtime::Handle::current();

        let handle = std::thread::spawn(move || -> Result<()> {
            rt.block_on(async {
                let platform = common::platform::DefaultPlatform::new()?;
                let mut app = AlliumMenu::new(platform).await?;

                while let Ok(info) = rx.recv() {
                    if let Err(e) = app.prepare(info).await {
                        log::error!("menu prepare failed: {:?}", e);
                    } else if let Err(e) = app.run_event_loop().await {
                        log::error!("menu run failed: {:?}", e);
                    }
                    if let Err(e) = app.save() {
                        log::error!("menu save failed: {:?}", e);
                    }
                    let _ = done_tx.send(());
                }
                Ok(())
            })
        });

        MenuHandle {
            tx,
            done_rx,
            _handle: handle,
        }
    }
}

pub struct AlliumD<P: Platform> {
    platform: P,
    main: Child,
    menu: MenuHandle,
    menu_open: bool,
    keys: EnumMap<Key, bool>,
    is_menu_pressed_alone: bool,
    is_terminating: bool,
    state: AlliumDState,
    locale: Locale,
    power_settings: PowerSettings,
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
            if let Ok(json) = fs::read_to_string(ALLIUMD_STATE.as_path())
                && let Ok(this) = serde_json::from_str::<AlliumDState>(&json)
            {
                if Utc::now() < this.time {
                    info!(
                        "RTC is not working, advancing time to {}",
                        this.time.format("%F %T")
                    );
                    let mut date = std::process::Command::new("date")
                        .arg("--utc")
                        .arg("--set")
                        .arg(this.time.format("%F %T").to_string())
                        .spawn()?;
                    date.wait()?;
                    let mut hwclock = std::process::Command::new("/sbin/hwclock")
                        .arg("--systohc")
                        .arg("--utc")
                        .arg(this.time.format("%F %T").to_string())
                        .spawn()?;
                    hwclock.wait()?;
                }
                return Ok(this);
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

async fn spawn_main() -> Result<Child> {
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
    pub async fn new() -> Result<AlliumD<DefaultPlatform>> {
        #[cfg(feature = "miyoo")]
        common::platform::miyoo::try_fix_resolution().await?;

        let mut platform = DefaultPlatform::new()?;
        let state = AlliumDState::load()?;

        info!("setting volume: {}", state.volume);
        platform.set_volume(state.volume)?;

        info!("setting brightness: {}", state.brightness);
        platform.set_brightness(state.brightness)?;

        info!("loading display settings");
        platform.set_display_settings(&mut DisplaySettings::load()?)?;

        let main = spawn_main().await?;
        let locale = Locale::new(&LocaleSettings::load()?.lang);
        let power_settings = PowerSettings::load()?;

        // Spawn the persistent menu thread at startup
        let menu = MenuHandle::new();

        Ok(AlliumD {
            platform,
            main,
            menu,
            menu_open: false,
            keys: EnumMap::default(),
            is_menu_pressed_alone: false,
            is_terminating: false,
            state,
            locale,
            power_settings,
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        info!("hello from Allium {}", &*ALLIUM_VERSION);

        if DefaultPlatform::has_wifi() {
            info!("wifi detected, loading wifi settings");
            WiFiSettings::load()?.init()?;
        }

        info!("starting event loop");
        #[cfg(unix)]
        {
            let mut sigint = tokio::signal::unix::signal(SignalKind::interrupt())?;
            let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;

            let mut battery_interval = Instant::now();

            // If battery is charging, suspend.
            let mut battery = self.platform.battery()?;
            battery.update()?;
            if battery.charging() {
                self.handle_charging().await?;
            }

            let mut battery_led_task = None;

            loop {
                if battery_interval.elapsed() >= BATTERY_UPDATE_INTERVAL {
                    battery_interval = Instant::now();
                    trace!("updating battery");
                    if let Err(e) = battery.update() {
                        error!("failed to update battery: {}", e);
                    }

                    if battery.percentage() <= BATTERY_WARNING_THRESHOLD && !battery.charging() {
                        if battery_led_task.is_none() {
                            warn!(
                                "battery is low ({}%), consider charging soon",
                                battery.percentage()
                            );

                            battery_led_task = Some(tokio::spawn(async {
                                loop {
                                    <DefaultPlatform as Platform>::Battery::update_led(true);
                                    tokio::time::sleep(std::time::Duration::from_millis(1750))
                                        .await;
                                    <DefaultPlatform as Platform>::Battery::update_led(false);
                                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                                }
                            }));
                        }
                    } else if let Some(task) = battery_led_task.take() {
                        info!("aborting battery LED blink task");
                        task.abort();
                        <DefaultPlatform as Platform>::Battery::update_led(false);
                    }

                    if battery.percentage() <= BATTERY_SHUTDOWN_THRESHOLD && !battery.charging() {
                        warn!("battery is low, shutting down");
                        self.handle_quit().await?;
                    }
                }

                let auto_sleep_duration = match self.power_settings.auto_sleep_duration_minutes {
                    0 => std::time::Duration::MAX, // disabled
                    t => std::time::Duration::new(t as u64 * 60, 0),
                };
                tokio::select! {
                    key_event = self.platform.poll() => {
                        self.handle_key_event(key_event).await?;
                    }
                    _ = self.menu.done_rx.recv() => {
                        info!("menu finished, resuming game");
                        self.menu_open = false;
                        self.is_menu_pressed_alone = false;
                        RetroArchCommand::Unpause.send().await?;
                    }
                    _ = tokio::time::sleep(auto_sleep_duration) => {
                        if !self.power_settings.auto_sleep_when_charging && battery.charging() {
                            info!("battery charging, don't auto sleep");
                        } else if Path::new("/tmp/stay_awake").exists() {
                            info!("/tmp/stay_awake exists, don't auto sleep");
                        } else {
                            info!("idle timeout, shutting down");
                            self.handle_quit().await?;
                        }
                    }
                    _ = self.main.wait() => {
                        if !self.is_terminating {
                            info!("main process terminated, recording play time");
                            self.update_play_time()?;
                            GameInfo::delete()?;
                            self.main = spawn_main().await?;
                        }
                    }
                    _ = sigint.recv() => self.handle_quit().await?,
                    _ = sigterm.recv() => self.handle_quit().await?,
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
        debug!(
            "main: {:?}, ingame: {}, key_event: {:?}",
            self.main.id(),
            self.is_ingame(),
            key_event
        );

        // Handle menu key
        match key_event {
            KeyEvent::Pressed(Key::Menu) => {
                self.is_menu_pressed_alone = true;
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
                        .arg("--rumble")
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
                        #[cfg(unix)]
                        self.handle_quit().await?;
                    }
                }
                KeyEvent::Released(Key::Power) => {
                    if !self.keys[Key::Menu] {
                        #[cfg(unix)]
                        match self.power_settings.power_button_action {
                            PowerButtonAction::Suspend => self.handle_suspend().await?,
                            PowerButtonAction::Shutdown => self.handle_quit().await?,
                            PowerButtonAction::Nothing => {}
                        }
                    }
                }
                KeyEvent::Pressed(Key::LidClose) =>
                {
                    #[cfg(unix)]
                    match self.power_settings.lid_close_action {
                        PowerButtonAction::Suspend => self.handle_suspend().await?,
                        PowerButtonAction::Shutdown => self.handle_quit().await?,
                        PowerButtonAction::Nothing => {}
                    }
                }
                KeyEvent::Released(Key::Menu) => {
                    if self.is_menu_pressed_alone {
                        if !self.menu_open
                            && self.is_ingame()
                            && self
                                .keys
                                .iter()
                                .all(|(k, pressed)| k == Key::Menu || !pressed)
                            && let Some(game_info) = GameInfo::load()?
                            && game_info.has_menu
                        {
                            let info = RetroArchCommand::GetInfo.send_recv().await?.map(|ret| {
                                let mut rets = ret.split_ascii_whitespace().skip(1);
                                let max_disk_slots =
                                    rets.next().map_or(0, |s| s.parse().unwrap_or(0));
                                let disk_slot = rets.next().map_or(0, |s| s.parse().unwrap_or(0));
                                let state_slot = rets.next().map(|s| s.parse().unwrap_or(0));
                                RetroArchInfo {
                                    max_disk_slots,
                                    disk_slot,
                                    state_slot,
                                }
                            });

                            if info.is_some() {
                                RetroArchCommand::Pause.send().await?;
                                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                            }

                            self.menu_open = true;
                            if self.menu.tx.send(info).is_err() {
                                error!("failed to send to menu thread");
                                self.menu_open = false;
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
    async fn handle_charging(&mut self) -> Result<()> {
        info!("charging...");

        signal(&self.main, Signal::SIGSTOP)?;

        Command::new("say")
            .arg(self.locale.t("charging"))
            .spawn()?
            .wait()
            .await?;

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        Command::new("show").arg("-c").spawn()?.wait().await?;

        #[allow(clippy::let_unit_value)]
        let ctx = self.platform.suspend()?;

        let mut battery = self.platform.battery()?;

        loop {
            tokio::select! {
                key_event = self.platform.poll() => {
                    if matches!(key_event, KeyEvent::Released(Key::Power)) {
                        break;
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                    battery.update()?;
                    if !battery.charging() {
                        self.platform.shutdown()?;
                    }
                }
            }
        }

        signal(&self.main, Signal::SIGCONT)?;
        self.platform.unsuspend(ctx)
    }

    #[cfg(unix)]
    async fn handle_suspend(&mut self) -> Result<()> {
        info!("suspending...");
        #[allow(clippy::let_unit_value)]
        let ctx = self.platform.suspend()?;
        signal(&self.main, Signal::SIGSTOP)?;

        loop {
            tokio::select! {
                key_event = self.platform.poll()=> {
                    if matches!(key_event, KeyEvent::Released(Key::Power)) || matches!(key_event, KeyEvent::Released(Key::LidClose)) {
                        self.keys[Key::Power] = false;
                        self.keys[Key::LidClose] = false;
                        break;
                    }
                }
                _ = tokio::time::sleep(IDLE_TIMEOUT) => {
                    info!("idle timeout, shutting down");
                    signal(&self.main, Signal::SIGCONT)?;
                    self.platform.unsuspend(ctx)?;
                    self.handle_quit().await?;
                    return Ok(());
                }
            }
        }

        info!("waking up from suspend...");
        signal(&self.main, Signal::SIGCONT)?;
        self.platform.unsuspend(ctx)
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
            // Menu thread will exit on its own when the process shuts down
        }

        terminate(&mut self.main).await?;

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

        let duration = game_info.play_time();

        // As a sanity check, don't add play time if the game was played for more than 24 hours
        if duration > Duration::hours(24) {
            warn!("play time is too long, not adding to database");
            return Ok(());
        }

        let database = Database::new()?;
        database.add_play_time(game_info.path.as_path(), duration)?;

        // Record game session
        let start_time = game_info.start_time.timestamp();
        let end_time = Utc::now().timestamp();
        database.insert_game_session(
            game_info.path.as_path(),
            start_time,
            end_time,
            duration.num_seconds(),
        )?;

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

    #[cfg(unix)]
    if let Err(_e) = tokio::time::timeout(std::time::Duration::from_secs(5), child.wait()).await {
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
