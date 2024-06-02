#![deny(clippy::all)]
#![warn(rust_2018_idioms)]

mod app;
pub mod view;

use anyhow::Result;

use app::App;
use common::platform::{DefaultPlatform, Platform};
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new().init().unwrap();

    let platform = DefaultPlatform::new()?;
    let mut app = App::new(platform).await?;
    app.run_event_loop().await?;
    Ok(())
}
