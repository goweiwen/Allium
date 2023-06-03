use anyhow::Result;
use tracing::debug;

use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};

pub struct Alliumd<P: Platform> {
    platform: P,
    volume: i32,
}

impl Alliumd<DefaultPlatform> {
    pub fn new() -> Result<Alliumd<DefaultPlatform>> {
        let platform = DefaultPlatform::new()?;

        Ok(Alliumd {
            platform,
            volume: 0,
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        tracing::trace!("running Alliumd");
        loop {
            let key = self.platform.poll().await?;
            match key {
                Some(KeyEvent::Released(Key::VolDown)) => self.add_volume(-1)?,
                Some(KeyEvent::Released(Key::VolUp)) => self.add_volume(1)?,
                Some(KeyEvent::Pressed(Key::Menu)) => {}
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
