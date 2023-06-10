use std::process::Command;

use common::{display::settings::DisplaySettings, stylesheet::Stylesheet};

#[derive(Debug)]
pub enum AlliumCommand {
    Exec(Command),
    SaveStylesheet(Box<Stylesheet>),
    SaveDisplaySettings(Box<DisplaySettings>),
}
