mod alliumd;

use anyhow::Result;

use crate::alliumd::AlliumD;

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let mut app = AlliumD::load()?;
    app.run_event_loop().await?;
    Ok(())
}
