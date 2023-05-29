#![feature(async_fn_in_trait)]

use allium::Allium;
use anyhow::Result;

mod allium;
mod display;
mod keys;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = Allium::new()?;
    app.init().await?;
    app.run_event_loop().await?;
    Ok(())
}
