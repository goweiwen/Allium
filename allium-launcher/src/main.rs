#![deny(clippy::all)]
#![warn(rust_2018_idioms)]

mod allium_launcher;
mod consoles;
mod entry;
mod view;

use anyhow::Result;

use allium_launcher::AlliumLauncher;
use common::platform::{DefaultPlatform, Platform};
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new().init().unwrap();

    let platform = DefaultPlatform::new()?;
    let mut app = AlliumLauncher::new(platform)?;
    app.run_event_loop().await?;
    Ok(())
}
