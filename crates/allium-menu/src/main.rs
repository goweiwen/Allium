#![deny(clippy::all)]
#![warn(rust_2018_idioms)]

mod allium_menu;
mod retroarch_info;
pub mod view;

use std::time::Duration;

use anyhow::Result;

use allium_menu::AlliumMenu;
use common::{
    platform::{DefaultPlatform, Platform},
    retroarch::RetroArchCommand,
};
use simple_logger::SimpleLogger;

use crate::retroarch_info::RetroArchInfo;

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new().init().unwrap();

    #[cfg(not(feature = "simulator"))]
    let info = RetroArchCommand::GetInfo.send_recv().await?.map(|ret| {
        let mut rets = ret.split_ascii_whitespace().skip(1);

        let max_disk_slots = rets.next().map_or(0, |s| s.parse().unwrap_or(0));
        let disk_slot = rets.next().map_or(0, |s| s.parse().unwrap_or(0));
        let state_slot = rets.next().map(|s| s.parse().unwrap_or(0));

        RetroArchInfo {
            max_disk_slots,
            disk_slot,
            state_slot,
        }
    });

    #[cfg(feature = "simulator")]
    let info = Some(RetroArchInfo {
        max_disk_slots: 3,
        disk_slot: 0,
        state_slot: Some(0),
    });

    if info.is_some() {
        RetroArchCommand::Pause.send().await?;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let platform = DefaultPlatform::new()?;
    let mut app = AlliumMenu::new(platform, info).await?;
    app.run_event_loop().await?;
    Ok(())
}
