#![feature(trait_upcasting)]

mod activity_tracker;
mod view;

use anyhow::Result;

use common::platform::{DefaultPlatform, Platform};
use simple_logger::SimpleLogger;

use crate::activity_tracker::ActivityTracker;

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new().init().unwrap();

    let platform = DefaultPlatform::new()?;
    let mut app = ActivityTracker::new(platform)?;
    app.run_event_loop().await?;
    Ok(())
}
