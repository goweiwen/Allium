#![feature(async_fn_in_trait)]

use allium::Allium;
use anyhow::Result;

mod allium;
mod platform;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = Allium::new()?;
    app.init().await?;
    app.run_event_loop().await?;
    Ok(())
}
