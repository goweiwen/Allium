mod alliumd;

use anyhow::Result;
use simple_logger::SimpleLogger;

use crate::alliumd::AlliumD;

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new().init().unwrap();

    let mut app = AlliumD::load()?;
    app.run_event_loop().await?;
    Ok(())
}
