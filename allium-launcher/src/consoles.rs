use std::path::PathBuf;
use std::{collections::HashMap, path::Path};

use anyhow::{anyhow, bail, Context, Result};
use common::command::Command;
use common::database::Database;
use common::game_info::GameInfo;
use serde::{Deserialize, Serialize};

use common::constants::{ALLIUM_CONFIG_CONSOLES, ALLIUM_GAMES_DIR, ALLIUM_RETROARCH};
use tracing::debug;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Console {
    /// The name of the console.
    #[serde(skip)]
    pub name: String,
    /// If present, takes priority over RetroArch cores.
    #[serde(default)]
    pub path: Option<PathBuf>,
    /// List of RetroArch cores to use. First is default.
    #[serde(default)]
    pub cores: Vec<String>,
    /// Folder/file names to match against. If the folder/file matches exactly OR contains a parenthesized string that matches exactly, this core will be used.
    /// e.g. "GBA" matches "GBA", "Game Boy Advance (GBA)"
    #[serde(default)]
    pub patterns: Vec<String>,
    /// File extensions to match against. This matches against all extensions, if there are multiple.
    /// e.g. "gba" matches "Game.gba", "Game.GBA", "Game.gba.zip"
    #[serde(default)]
    pub extensions: Vec<String>,
    /// File names to match against. This matches against the entire file name, including extension.
    /// e.g. "Doukutsu.exe" for NXEngine
    #[serde(default)]
    pub file_name: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Game {
    pub name: String,
    pub full_name: String,
    pub path: PathBuf,
    /// image is loaded lazily.
    /// None means image hasn't been looked for, Some(None) means no image was found, Some(Some(path)) means an image was found.
    image: Option<Option<PathBuf>>,
    pub extension: String,
}

impl Game {
    pub fn new(name: String, path: PathBuf) -> Game {
        let full_name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("")
            .to_string();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_string();
        Game {
            name,
            full_name,
            path,
            image: None,
            extension,
        }
    }

    /// Returns the image path if already cached
    pub fn image_ref(&self) -> Option<&Path> {
        if let Some(Some(ref image)) = self.image {
            Some(image)
        } else {
            None
        }
    }

    /// Searches for the image path, caches it, and returns it
    pub fn image(&mut self) -> Option<&Path> {
        // Search for Imgs folder upwards, recursively
        let mut parent = self.path.clone();
        let mut image = None;
        'image: while parent.pop() {
            let mut image_path = parent.join("Imgs");
            if image_path.is_dir() {
                image_path.extend(self.path.strip_prefix(&parent).unwrap());
                const IMAGE_EXTENSIONS: [&str; 7] =
                    ["png", "jpg", "jpeg", "webp", "gif", "tga", "bmp"];
                for ext in &IMAGE_EXTENSIONS {
                    image_path.set_extension(ext);
                    if image_path.is_file() {
                        image = Some(image_path);
                        break 'image;
                    }
                }
            }
            if parent.to_str() == ALLIUM_GAMES_DIR.to_str() {
                break;
            }
        }
        self.image = Some(image);
        self.image_ref()
    }
}

impl Ord for Game {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.full_name.cmp(&other.full_name)
    }
}

impl PartialOrd for Game {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Deserialize)]
struct ConsoleConfig(HashMap<String, Console>);

#[derive(Debug, Clone)]
pub struct ConsoleMapper {
    consoles: Vec<Console>,
}

impl Default for ConsoleMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleMapper {
    pub fn new() -> ConsoleMapper {
        ConsoleMapper {
            consoles: Vec::new(),
        }
    }

    pub fn load_config(&mut self) -> Result<()> {
        let config = std::fs::read_to_string(ALLIUM_CONFIG_CONSOLES.as_path()).map_err(|e| {
            anyhow!(
                "Failed to load consoles config: {:?}, {}",
                &*ALLIUM_CONFIG_CONSOLES,
                e
            )
        })?;
        let config: ConsoleConfig =
            toml::from_str(&config).context("Failed to parse consoles.toml.")?;
        self.consoles = config
            .0
            .into_iter()
            .map(|(name, mut core)| {
                core.name = name;
                core
            })
            .collect();

        Ok(())
    }

    pub fn get_console(&self, path: &Path) -> Option<&Console> {
        let path_lowercase = path.as_os_str().to_ascii_lowercase();

        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            let console = self
                .consoles
                .iter()
                .find(|core| core.file_name.iter().any(|s| name == s));
            if console.is_some() {
                return console;
            }
        }

        if let Some(extensions) = path_lowercase.to_str() {
            for ext in extensions.split('.').skip(1) {
                let console = self
                    .consoles
                    .iter()
                    .find(|core| core.extensions.iter().any(|s| s == ext));
                if console.is_some() {
                    return console;
                }
            }
        }

        let mut parent = Some(path);
        while let Some(path) = parent {
            println!("path: {:?}", path);
            if let Some(filename) = path.file_name().and_then(|path| path.to_str()) {
                let console = self.consoles.iter().find(|core| {
                    core.patterns.iter().any(|pattern| {
                        filename == pattern || filename.contains(&format!("({})", pattern))
                    })
                });
                if console.is_some() {
                    return console;
                }
            }
            parent = path.parent();
        }

        None
    }

    pub fn launch_game(&self, database: &Database, game: &mut Game) -> Result<Option<Command>> {
        game.image();
        database.increment_play_count(&game.name, game.path.as_path(), game.image_ref())?;

        let core = self.get_console(game.path.as_path());
        Ok(if let Some(console) = core {
            let game_info = if let Some(ref path) = console.path {
                GameInfo::new(
                    game.name.clone(),
                    game.path.to_owned(),
                    path.display().to_string(),
                    vec![game.path.display().to_string()],
                    false,
                )
            } else if let Some(retroarch_core) = console.cores.first() {
                GameInfo::new(
                    game.name.clone(),
                    game.path.to_owned(),
                    ALLIUM_RETROARCH.display().to_string(),
                    vec![retroarch_core.to_owned(), game.path.display().to_string()],
                    true,
                )
            } else {
                bail!("Console \"{}\" has no path or cores.", console.name);
            };
            debug!("Saving game info: {:?}", game_info);
            game_info.save()?;
            Some(Command::Exec(game_info.command()))
        } else {
            None
        })
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn test_console_mapper() {
        let mut mapper = ConsoleMapper::new();
        mapper.consoles = vec![Console {
            name: "Test".to_string(),
            patterns: vec!["POKE".to_string(), "PKM".to_string()],
            extensions: vec!["gb".to_string(), "gbc".to_string()],
            cores: vec![],
            path: None,
            file_name: vec![],
        }];

        assert!(mapper.get_console(Path::new("Roms/POKE/rom.zip")).is_some());
        assert!(mapper.get_console(Path::new("Roms/PKM/rom.zip")).is_some());
        assert!(mapper
            .get_console(Path::new("Roms/Pokemon Mini (POKE)/rom.zip"))
            .is_some());
        assert!(mapper
            .get_console(Path::new("Roms/POKE MINI/rom.zip"))
            .is_none());
        assert!(mapper.get_console(Path::new("Roms/rom.gb")).is_some());
        assert!(mapper.get_console(Path::new("Roms/rom.gbc")).is_some());
        assert!(mapper.get_console(Path::new("Roms/rom.gbc.zip")).is_some());
        assert!(mapper.get_console(Path::new("Roms/rom.zip.gbc")).is_some());
        assert!(mapper.get_console(Path::new("Roms/gbc")).is_none());
        assert!(mapper.get_console(Path::new("Roms/rom.gba")).is_none());
    }

    #[test]
    fn test_config() {
        env::set_var("ALLIUM_BASE_DIR", "../assets/root/.allium");

        let mut mapper = ConsoleMapper::new();
        mapper.load_config().unwrap();

        let eq = |rom: &str, console_name: &str, core: &str| -> bool {
            let console = mapper.get_console(Path::new(rom));
            if console.is_none() {
                println!("No console found for {}", rom);
                return false;
            }
            let console = console.unwrap();
            if console.name == console_name && console.cores.first() == Some(&core.to_string()) {
                true
            } else {
                println!(
                    "Expected console: {} core: {:?}, got console: {} core: {}",
                    console_name,
                    console.cores.first(),
                    console.name,
                    core
                );
                false
            }
        };

        // GB
        assert!(eq("GB/rom.zip", "Nintendo - Game Boy", "gambatte"));
        assert!(eq("rom.gb", "Nintendo - Game Boy", "gambatte"));

        // GBC
        assert!(eq("GBC/rom.zip", "Nintendo - Game Boy Color", "gambatte"));
        assert!(eq("rom.gbc", "Nintendo - Game Boy Color", "gambatte"));

        // GBA
        assert!(eq("GBA/rom.zip", "Nintendo - Game Boy Advance", "gpsp"));
        assert!(eq("rom.gba", "Nintendo - Game Boy Advance", "gpsp"));

        // NES
        assert!(eq("FC/rom.zip", "Nintendo - NES", "fceumm"));
        assert!(eq("NES/rom.zip", "Nintendo - NES", "fceumm"));
        assert!(eq("rom.nes", "Nintendo - NES", "fceumm"));

        // SNES
        assert!(eq("SFC/rom.zip", "Nintendo - SNES", "mednafen_supafaust"));
        assert!(eq("SNES/rom.zip", "Nintendo - SNES", "mednafen_supafaust"));
        assert!(eq("rom.sfc", "Nintendo - SNES", "mednafen_supafaust"));
        assert!(eq("rom.smc", "Nintendo - SNES", "mednafen_supafaust"));

        // PS1
        assert!(eq("PSX/rom.zip", "Sony - PlayStation", "pcsx_rearmed"));
        assert!(eq("PS1/rom.zip", "Sony - PlayStation", "pcsx_rearmed"));
        assert!(eq("PS/rom.zip", "Sony - PlayStation", "pcsx_rearmed"));
        assert!(eq("PS/playlist.m3u", "Sony - PlayStation", "pcsx_rearmed"));
        assert!(eq("rom.pbp", "Sony - PlayStation", "pcsx_rearmed"));

        // Neo Geo Pocket
        assert!(eq("NGP/rom", "SNK - Neo Geo Pocket Color", "mednafen_ngp"));
        assert!(eq("NGC/rom", "SNK - Neo Geo Pocket Color", "mednafen_ngp"));
        assert!(eq("rom.ngp", "SNK - Neo Geo Pocket Color", "mednafen_ngp"));
        assert!(eq("rom.ngc", "SNK - Neo Geo Pocket Color", "mednafen_ngp"));

        // Sega - Game Gear
        assert!(eq("GG/rom", "Sega - Game Gear", "picodrive"));
        assert!(eq("rom.gg", "Sega - Game Gear", "picodrive"));

        // NXEngine
        assert!(eq("Cave Story/Doukutsu.exe", "NXEngine", "nxengine"));
        assert!(eq("Cave Story (NXENGINE).m3u", "NXEngine", "nxengine"));
    }
}
