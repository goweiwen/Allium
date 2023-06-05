#![feature(async_fn_in_trait)]

mod alliumd;

use anyhow::Result;

use crate::alliumd::Alliumd;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let mut app = Alliumd::load()?;
    app.run_event_loop().await?;
    Ok(())
}
