mod screenshot_viewer;
mod view;

use anyhow::Result;
use common::platform::{DefaultPlatform, Platform};
use screenshot_viewer::ScreenshotViewer;
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new().env().init().unwrap();
    let platform = DefaultPlatform::new()?;
    let mut app = ScreenshotViewer::new(platform)?;
    app.run_event_loop().await?;
    Ok(())
}
