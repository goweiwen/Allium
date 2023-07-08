#![deny(clippy::all)]
#![warn(rust_2018_idioms)]

mod allium_menu;
pub mod view;

use std::env;

use anyhow::Result;

use allium_menu::AlliumMenu;
use common::platform::{DefaultPlatform, Platform};
use log::trace;
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new().init().unwrap();

    let mut args = env::args().skip(1);

    let disk_slot = args.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    let max_disk_slots = args.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    let state_slot = args.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    trace!(
        "max_disk_slots: {}, disk_slot: {}, state_slot: {}",
        max_disk_slots,
        disk_slot,
        state_slot
    );

    let platform = DefaultPlatform::new()?;
    let mut app = AlliumMenu::new(platform, disk_slot, max_disk_slots, state_slot).await?;
    app.run_event_loop().await?;
    Ok(())
}
