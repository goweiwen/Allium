use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::signal;
use tracing::debug;

use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};

#[cfg(unix)]
use {
    crate::retroarch::RetroArchCommand,
    nix::sys::signal::kill,
    nix::sys::signal::Signal::{SIGCONT, SIGSTOP},
    nix::unistd::Pid,
    std::os::unix::process::CommandExt,
    std::process::Command,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Alliumd<P: Platform> {
    #[serde(skip)]
    platform: P,
    volume: i32,
    #[serde(skip)]
    is_core_stopped: bool,
}

impl Alliumd<DefaultPlatform> {
    pub fn new() -> Result<Alliumd<DefaultPlatform>> {
        let platform = DefaultPlatform::new()?;

        Ok(Alliumd {
            platform,
            volume: 0,
            is_core_stopped: false,
        })
    }

    pub fn load() -> Result<Alliumd<DefaultPlatform>> {
        let path = Path::new("/mnt/SDCARD/.allium/alliumd.json");
        if !path.exists() {
            debug!("can't find state, creating new");
            Self::new()
        } else {
            debug!("found state, loading from file");
            let json = fs::read_to_string(path)?;
            let alliumd: Alliumd<DefaultPlatform> = serde_json::from_str(&json)?;
            Ok(alliumd)
        }
    }

    fn save(&mut self) -> Result<()> {
        let json = serde_json::to_string(self).unwrap();
        File::create("/mnt/SDCARD/.allium/alliumd.json")?.write_all(json.as_bytes())?;
        Ok(())
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        tracing::trace!("running Alliumd");

        self.platform.set_volume(self.volume)?;

        loop {
            tokio::select! {
                key = self.platform.poll() => {
                    match key? {
                        Some(KeyEvent::Released(Key::VolDown)) => self.add_volume(-1)?,
                        Some(KeyEvent::Released(Key::VolUp)) => self.add_volume(1)?,
                        Some(KeyEvent::Pressed(Key::Menu)) => {
                            #[cfg(unix)]
                            {
                                let path = Path::new("/tmp/allium_core.pid");
                                if path.exists() {
                                    let pid = fs::read_to_string(path)?;
                                    let pid = Pid::from_raw(pid.parse::<i32>()?);
                                    if self.is_core_stopped {
                                        RetroArchCommand::MenuToggle.send().await?;
                                        // kill(pid, SIGCONT)?;
                                        self.is_core_stopped = false;
                                    } else {
                                        RetroArchCommand::MenuToggle.send().await?;
                                        // kill(pid, SIGSTOP)?;
                                        self.is_core_stopped = true;
                                    }
                                }
                            }
                        }
                        Some(KeyEvent::Pressed(Key::Power)) => {
                            self.save()?;
                            #[cfg(unix)]
                            Command::new("poweroff").exec();
                        }
                        _ => {}
                    }
                }
                _ = signal::ctrl_c() => {
                    debug!("caught SIGKILL, saving state");
                    self.save()?;
                    break;
                }
            }
        }

        Ok(())
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
