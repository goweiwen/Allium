#![deny(clippy::all)]
#![warn(rust_2018_idioms)]

mod allium_menu;
pub mod view;

use anyhow::Result;

use allium_menu::AlliumMenu;
use common::platform::{DefaultPlatform, Platform};
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new().init().unwrap();

    let platform = DefaultPlatform::new()?;
    let mut app = AlliumMenu::new(platform).await?;
    app.run_event_loop().await?;
    Ok(())
}
