use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use log::debug;
use serde::{Deserialize, Serialize};

use crate::constants::{ALLIUM_GAME_INFO, ALLIUM_GAMES_DIR, ALLIUM_SCRIPTS_DIR};

#[derive(Debug, Serialize, Deserialize)]
/// Information about a game. Used to restore a game after a restart, and to calculate playtime.
pub struct GameInfo {
    /// Display name of the game.
    pub name: String,
    /// Path to the game rom. This is used to generate the screenshot name.
    pub path: PathBuf,
    /// Core used to run the game. This is used to generate the screenshot name.
    pub core: String,
    /// Command to run the core.
    pub command: String,
    /// Arguments to pass to the core to run the game.
    pub args: Vec<String>,
    /// Do we enable the menu? Currently only enabled if RetroArch is used.
    pub has_menu: bool,
    /// Whether swap should be enabled.
    pub needs_swap: bool,
    /// Path to the image.
    pub image: Option<PathBuf>,
    /// Paths to the guide text files.
    pub guides: Vec<PathBuf>,
    /// Start time. Used to measure playtime.
    pub start_time: DateTime<Utc>,
}

impl Default for GameInfo {
    fn default() -> Self {
        Self {
            name: String::new(),
            path: PathBuf::new(),
            core: String::new(),
            command: String::new(),
            args: Vec::new(),
            has_menu: false,
            needs_swap: false,
            image: None,
            guides: Vec::new(),
            start_time: Utc::now(),
        }
    }
}

impl GameInfo {
    /// Create a new GameInfo object.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        path: PathBuf,
        core: String,
        image: Option<PathBuf>,
        command: String,
        args: Vec<String>,
        has_menu: bool,
        needs_swap: bool,
    ) -> Self {
        let guides = find_guides(&path);

        Self {
            name,
            path,
            core,
            command,
            args,
            has_menu,
            needs_swap,
            image,
            guides,
            start_time: Utc::now(),
        }
    }

    /// Loads the current game info from file, if exists.
    pub fn load() -> Result<Option<Self>> {
        Ok(if ALLIUM_GAME_INFO.exists() {
            let file = File::open(ALLIUM_GAME_INFO.as_path())?;
            let Ok(game_info) = serde_json::from_reader::<_, Self>(file) else {
                fs::remove_file(ALLIUM_GAME_INFO.as_path())?;
                return Ok(None);
            };
            if game_info.needs_swap() {
                debug!("enabling swap");
                Command::new(ALLIUM_SCRIPTS_DIR.join("swap-on.sh"))
                    .spawn()?
                    .wait()?;
            }
            Some(game_info)
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

    /// Deletes the current game info file.
    pub fn delete() -> Result<()> {
        if ALLIUM_GAME_INFO.exists() {
            fs::remove_file(ALLIUM_GAME_INFO.as_path())?;
        }
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
        Utc::now().signed_duration_since(self.start_time)
    }

    /// Whether swap should be enabled.
    pub fn needs_swap(&self) -> bool {
        self.needs_swap
    }
}

/// Searches for all guide paths and returns them
pub fn find_guides(path: &Path) -> Vec<PathBuf> {
    // Search for Guides folder upwards, recursively
    let mut parent = path.to_path_buf();
    let mut guides = Vec::new();
    'guide: while parent.pop() {
        let mut guide_path = parent.join("Guides");
        if guide_path.is_dir() {
            guide_path.extend(path.strip_prefix(&parent).unwrap());

            debug!("Checking guide path: {:?}", guide_path);
            guide_path.set_extension("");
            if guide_path.is_dir() {
                debug!("Found guide directory: {:?}", guide_path);
                // Collect all guides in the directory
                if let Ok(entries) = fs::read_dir(&guide_path) {
                    let mut found_guides: Vec<PathBuf> = entries
                        .filter_map(|entry| {
                            let entry = entry.ok()?;
                            let path = entry.path();
                            if path.is_file() { Some(path) } else { None }
                        })
                        .collect();
                    // Sort guides alphabetically for consistent ordering
                    found_guides.sort();
                    if !found_guides.is_empty() {
                        guides = found_guides;
                        break 'guide;
                    }
                }
            }

            const GUIDE_EXTENSIONS: [&str; 1] = ["txt"];
            for ext in &GUIDE_EXTENSIONS {
                guide_path.set_extension(ext);
                debug!("Checking guide file: {:?}", guide_path);
                if guide_path.is_file() {
                    guides.push(guide_path.clone());
                    break 'guide;
                }
            }
        }
        if parent.to_str() == ALLIUM_GAMES_DIR.to_str() {
            break;
        }
    }
    guides
}
