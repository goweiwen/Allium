use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Result;
use common::constants::{ALLIUMD_STATE, ALLIUM_GAME_INFO, ALLIUM_LAUNCHER, ALLIUM_MENU};
use common::retroarch::RetroArchCommand;
use futures::future::{Fuse, FutureExt};
use serde::{Deserialize, Serialize};
use tokio::process::{Child, Command};
use tracing::{debug, info, trace};

use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};

#[cfg(unix)]
use {
    nix::sys::signal::kill, nix::sys::signal::Signal, nix::unistd::Pid,
    std::os::unix::process::CommandExt, tokio::signal::unix::SignalKind,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct AlliumD<P: Platform> {
    #[serde(skip)]
    platform: P,
    #[serde(skip, default = "spawn_main")]
    main: Child,
    #[serde(skip)]
    menu: Option<Child>,
    volume: i32,
}

fn spawn_main() -> Child {
    try_load_game().unwrap_or_else(|| {
        info!("no game info found, launching menu");
        Command::new(ALLIUM_LAUNCHER).spawn().unwrap()
    })
}

fn try_load_game() -> Option<Child> {
    let path = Path::new(ALLIUM_GAME_INFO);
    if path.exists() {
        let game_info = fs::read_to_string(path).ok()?;
        fs::remove_file(path).ok()?;
        let mut split = game_info.split('\n');
        let core = split.next().and_then(|path| PathBuf::from_str(path).ok())?;
        let rom = split.next()?;
        Command::new(core).arg(rom).spawn().ok()
    } else {
        None
    }
}

impl AlliumD<DefaultPlatform> {
    pub fn new() -> Result<AlliumD<DefaultPlatform>> {
        let platform = DefaultPlatform::new()?;

        Ok(AlliumD {
            platform,
            main: spawn_main(),
            menu: None,
            volume: 0,
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        info!("running Alliumd");

        self.platform.set_volume(self.volume)?;

        #[cfg(unix)]
        {
            let mut sighup = tokio::signal::unix::signal(SignalKind::hangup())?;
            let mut sigint = tokio::signal::unix::signal(SignalKind::interrupt())?;
            let mut sigquit = tokio::signal::unix::signal(SignalKind::quit())?;
            let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;

            loop {
                let menu_terminated = match self.menu.as_mut() {
                    Some(menu) => menu.wait().fuse(),
                    None => Fuse::terminated(),
                };

                tokio::select! {
                    key_event = self.platform.poll() => {
                        if let Some(key_event) = key_event? {
                            self.handle_key_event(key_event).await?;
                        }
                    }
                    _ = self.main.wait() => {
                        info!("main process terminated, restarting");
                        self.main = spawn_main();
                    }
                    _ = menu_terminated => {
                        info!("menu process terminated, resuming game");
                        self.menu = None;
                        #[cfg(unix)]
                        signal(&self.main, Signal::SIGCONT)?;
                        RetroArchCommand::PauseToggle.send().await?;
                    }
                    _ = sighup.recv() => self.handle_quit()?,
                    _ = sigint.recv() => self.handle_quit()?,
                    _ = sigquit.recv() => self.handle_quit()?,
                    _ = sigterm.recv() => self.handle_quit()?,
                }
            }
        }

        #[cfg(not(unix))]
        loop {
            tokio::select! {
                key_event = self.platform.poll() => {
                    if let Some(key_event) = key_event? {
                        self.handle_key_event(key_event).await?;
                    }
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
            KeyEvent::Released(Key::VolDown) => self.add_volume(-1)?,
            KeyEvent::Released(Key::VolUp) => self.add_volume(1)?,
            KeyEvent::Released(Key::Power) => {
                self.save()?;
                if self.is_ingame() {
                    if self.menu.is_some() {
                        #[cfg(unix)]
                        signal(&self.main, Signal::SIGCONT)?;
                        RetroArchCommand::PauseToggle.send().await?;
                    }
                    #[cfg(unix)]
                    signal(&self.main, Signal::SIGTERM)?;
                    self.main.wait().await?;
                }
                #[cfg(unix)]
                std::process::Command::new("poweroff").exec();
            }
            KeyEvent::Released(Key::Menu) => {
                if self.is_ingame() {
                    if let Some(menu) = &mut self.menu {
                        terminate(menu).await?;
                    } else {
                        RetroArchCommand::PauseToggle.send().await?;
                        #[cfg(unix)]
                        signal(&self.main, Signal::SIGSTOP)?;
                        self.menu = Some(Command::new(ALLIUM_MENU).spawn()?);
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_quit(&mut self) -> Result<()> {
        debug!("terminating, saving state");
        self.save()?;
        Ok(())
    }

    pub fn load() -> Result<AlliumD<DefaultPlatform>> {
        let path = Path::new(ALLIUMD_STATE);
        if !path.exists() {
            debug!("can't find state, creating new");
            Self::new()
        } else {
            debug!("found state, loading from file");
            let json = fs::read_to_string(path)?;
            let alliumd: AlliumD<DefaultPlatform> = serde_json::from_str(&json)?;
            Ok(alliumd)
        }
    }

    fn save(&mut self) -> Result<()> {
        let json = serde_json::to_string(self).unwrap();
        File::create(ALLIUMD_STATE)?.write_all(json.as_bytes())?;
        Ok(())
    }

    fn is_ingame(&self) -> bool {
        Path::new(ALLIUM_GAME_INFO).exists()
    }

    fn set_volume(&mut self, volume: i32) -> Result<()> {
        self.volume = volume.clamp(0, 20);
        debug!("set volume: {}", self.volume);
        self.platform.set_volume(self.volume)?;
        Ok(())
    }

    fn add_volume(&mut self, add: i32) -> Result<()> {
        self.set_volume(self.volume + add)
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
