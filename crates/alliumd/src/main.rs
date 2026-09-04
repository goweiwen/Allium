#![deny(clippy::all)]
#![warn(rust_2018_idioms)]

mod alliumd;
mod osd;

use anyhow::Result;
use simple_logger::SimpleLogger;

use crate::alliumd::AlliumD;

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new().env().init().unwrap();

    #[cfg(feature = "console")]
    {
        log::info!("Starting tokio console at :6669");
        console_subscriber::init();
    }

    let mut app = AlliumD::new().await?;
    app.run_event_loop().await?;
    Ok(())
}
