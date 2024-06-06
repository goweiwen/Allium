#![deny(clippy::all)]
#![warn(rust_2018_idioms)]
#![feature(trait_upcasting)]

mod allium_launcher;

use anyhow::Result;
use common::style::Stylesheet;
use simple_logger::SimpleLogger;

use crate::allium_launcher::AlliumLauncher;

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new().env().init().unwrap();

    let app = AlliumLauncher {};
    let style = Stylesheet::default();
    common::app::run(app, style).await
}
