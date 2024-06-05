use anyhow::Result;
use enum_map::EnumMap;

use crate::platform::Platform;

#[derive(Debug)]
pub struct App<P: Platform> {
    platform: P,
    display: P::Display,
}

impl<P: Platform> App<P> {
    pub fn new(mut platform: P) -> Result<Self> {
        let display = platform.display()?;
        let battery = platform.battery()?;

        Ok(AlliumLauncher { platform, display })
    }

    pub async fn run_event_loop(&mut self) -> ! {
        let mut keys: EnumMap<Key, bool> = EnumMap::default();

        let mut frame_interval =
            tokio::time::interval(tokio::time::Duration::from_micros(1000.0 / 60.0)); // 60 fps
        frame_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let events: Vec<Event> = Vec::new();
        loop {
            tokio::select! {
                _ = frame_interval.tick() => {}
                event = self.platform.poll() => {
                    events.push(event);
                }
            }
            let events = self.platform.poll().await;
        }
    }
}
