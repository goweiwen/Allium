use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::signal;
use tracing::debug;

use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};

#[cfg(unix)]
use {
    common::retroarch::RetroArchCommand,
    nix::sys::signal::kill,
    nix::sys::signal::Signal::{SIGCONT, SIGSTOP},
    nix::unistd::Pid,
    std::os::unix::process::CommandExt,
    std::process::Command,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct AlliumD<P: Platform> {
    #[serde(skip)]
    platform: P,
    volume: i32,
}

impl AlliumD<DefaultPlatform> {
    pub fn new() -> Result<AlliumD<DefaultPlatform>> {
        let platform = DefaultPlatform::new()?;

        Ok(AlliumD {
            platform,
            volume: 0,
        })
    }

    pub fn load() -> Result<AlliumD<DefaultPlatform>> {
        let path = Path::new("/mnt/SDCARD/.allium/alliumd.json");
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
                                    debug!("sending SIGSTOP to {}", pid);
                                    kill(pid, SIGSTOP)?;
                                    debug!("starting alliumm");
                                    let process = tokio::process::Command::new("/mnt/SDCARD/.allium/alliumm").spawn()?.wait().await?;
                                    debug!("sending SIGCONT to {}", pid);
                                    kill(pid, SIGCONT)?;
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
