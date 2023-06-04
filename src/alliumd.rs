use std::fs;
use std::path::Path;

use anyhow::Result;
use nix::sys::signal::kill;
use nix::sys::signal::Signal::{SIGCONT, SIGSTOP};
use nix::unistd::Pid;
use tracing::debug;

use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};

pub struct Alliumd<P: Platform> {
    platform: P,
    volume: i32,
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

    pub async fn run_event_loop(&mut self) -> Result<()> {
        tracing::trace!("running Alliumd");
        loop {
            let key = self.platform.poll().await?;
            match key {
                Some(KeyEvent::Released(Key::VolDown)) => self.add_volume(-1)?,
                Some(KeyEvent::Released(Key::VolUp)) => self.add_volume(1)?,
                Some(KeyEvent::Pressed(Key::Menu)) => {
                    let path = Path::new("/tmp/allium_core.pid");
                    if path.exists() {
                        let pid = fs::read_to_string(path)?;
                        let pid = Pid::from_raw(pid.parse::<i32>()?);
                        if self.is_core_stopped {
                            kill(pid, SIGCONT)?;
                            self.is_core_stopped = false;
                        } else {
                            kill(pid, SIGSTOP)?;
                            self.is_core_stopped = true;
                        }
                    }
                }
                _ => {}
            };
        }
    }

    fn add_volume(&mut self, add: i32) -> Result<()> {
        self.volume = (self.volume + add).clamp(0, 20);
        debug!("set volume: {}", self.volume);
        self.platform.set_volume(self.volume)?;
        Ok(())
    }
}
