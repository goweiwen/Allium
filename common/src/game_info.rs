use std::{
    fs::{self, File},
    path::PathBuf,
    process::Command,
};

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::constants::ALLIUM_GAME_INFO;

#[derive(Debug, Serialize, Deserialize)]
/// Information about a game. Used to restore a game after a restart, and to calculate playtime.
pub struct GameInfo {
    /// Display name of the game.
    pub name: String,
    /// Path to the game rom.
    pub path: PathBuf,
    /// Command to run the core.
    pub command: String,
    /// Arguments to pass to the core to run the game.
    pub args: Vec<String>,
    /// Start time
    pub start_time: DateTime<Utc>,
}

impl GameInfo {
    /// Create a new GameInfo object.
    pub fn new(name: String, path: PathBuf, command: String, args: Vec<String>) -> Self {
        Self {
            name,
            path,
            command,
            args,
            start_time: Utc::now(),
        }
    }

    /// Loads the current game info from file, if exists.
    pub fn load() -> Result<Option<Self>> {
        Ok(if ALLIUM_GAME_INFO.exists() {
            let file = File::open(ALLIUM_GAME_INFO.as_path())?;
            let game_info = serde_json::from_reader(file);
            if game_info.is_err() {
                fs::remove_file(ALLIUM_GAME_INFO.as_path())?;
            }
            game_info.ok()
        } else {
            None
        })
    }

    /// Saves the current game info to file.
    pub fn save(&self) -> Result<()> {
        let file = File::create(ALLIUM_GAME_INFO.as_path())?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }

    /// Returns a command to run the game.
    pub fn command(self) -> Command {
        let mut command = Command::new(self.command);
        command.args(self.args);
        command
    }

    /// How long the game has been running.
    pub fn play_time(&self) -> Duration {
        self.start_time.signed_duration_since(Utc::now())
    }
}
