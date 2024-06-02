use std::{fs::File, path::PathBuf};

use anyhow::Result;
use common::command::Command;
use serde::{Deserialize, Serialize};

/// Corresponds to the config.json file, compatible with stock/OnionOS.
#[derive(Debug, Deserialize)]
struct AppConfig {
    /// The name of the app.
    label: String,
    /// The path to the icon file.
    #[serde(default)]
    icon: Option<PathBuf>,
    /// The path to the app's launch script.
    launch: String,
    /// Short description of the app.
    #[allow(dead_code)]
    #[serde(default)]
    description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct App {
    pub name: String,
    pub directory: PathBuf,
    pub launch: PathBuf,
    pub image: Option<PathBuf>,
}

impl App {
    pub fn new(directory: PathBuf) -> Result<Self> {
        let config = File::open(directory.join("config.json"))?;
        let config: AppConfig = serde_json::from_reader(config)?;

        let name = config.label;
        let image = config.icon;
        let command = directory.join(config.launch);

        Ok(Self {
            name,
            launch: command,
            directory,
            image,
        })
    }

    pub fn command(&self) -> Command {
        let mut command = std::process::Command::new(&self.launch);
        command.current_dir(self.directory.as_path());
        Command::Exec(command)
    }
}

impl Ord for App {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for App {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
