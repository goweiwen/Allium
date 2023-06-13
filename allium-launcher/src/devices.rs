use std::path::PathBuf;
use std::{collections::HashMap, path::Path};

use anyhow::{anyhow, bail, Context, Result};
use common::database::Database;
use common::game_info::GameInfo;
use serde::{Deserialize, Serialize};

use common::constants::{ALLIUM_CONFIG_DIR, ALLIUM_GAMES_DIR, ALLIUM_RETROARCH};

use crate::command::AlliumCommand;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Device {
    #[serde(skip)]
    /// The name of the device.
    pub name: String,
    /// If present, takes priority over RetroArch cores.
    pub path: Option<PathBuf>,
    /// List of RetroArch cores to use. First is default.
    pub cores: Vec<String>,
    /// Folder names to match against. If the folder matches exactly OR contains a parenthesized string that matches exactly, this core will be used.
    /// e.g. "GBA" matches "GBA", "Game Boy Advance (GBA)"
    pub folders: Vec<String>,
    /// File extensions to match against. This matches against all extensions, if there are multiple.
    /// e.g. "gba" matches "Game.gba", "Game.GBA", "Game.gba.zip"
    pub extensions: Vec<String>,
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
struct DeviceConfig(HashMap<String, Device>);

#[derive(Debug, Clone)]
pub struct DeviceMapper {
    devices: Vec<Device>,
}

impl Default for DeviceMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceMapper {
    pub fn new() -> DeviceMapper {
        DeviceMapper {
            devices: Vec::new(),
        }
    }

    pub fn load_config(&mut self) -> Result<()> {
        let config =
            std::fs::read_to_string(ALLIUM_CONFIG_DIR.join("devices.toml")).map_err(|e| {
                anyhow!(
                    "Failed to load devices config: {:?}, {}",
                    &*ALLIUM_CONFIG_DIR.join("devices.toml"),
                    e
                )
            })?;
        let config: DeviceConfig =
            toml::from_str(&config).context("Failed to parse devices.toml.")?;
        self.devices = config
            .0
            .into_iter()
            .map(|(name, mut core)| {
                core.name = name;
                core
            })
            .collect();

        Ok(())
    }

    pub fn get_device(&self, mut path: &Path) -> Option<&Device> {
        let path_lowercase = path.as_os_str().to_ascii_lowercase();

        if let Some(extensions) = path_lowercase.to_str() {
            for ext in extensions.split('.').skip(1) {
                let device = self
                    .devices
                    .iter()
                    .find(|core| core.extensions.iter().any(|s| s == ext));
                if device.is_some() {
                    return device;
                }
            }
        }

        while let Some(parent) = path.parent() {
            if let Some(parent_filename) = parent.file_name().and_then(|parent| parent.to_str()) {
                let device = self.devices.iter().find(|core| {
                    core.folders.iter().any(|folder| {
                        parent_filename == folder
                            || parent_filename.contains(&format!("({})", folder))
                    })
                });
                if device.is_some() {
                    return device;
                }
            }
            path = parent;
        }

        None
    }

    pub fn launch_game(
        &self,
        database: &Database,
        game: &mut Game,
    ) -> Result<Option<AlliumCommand>> {
        game.image();
        database.increment_play_count(&game.name, game.path.as_path(), game.image_ref())?;

        let core = self.get_device(game.path.as_path());
        Ok(if let Some(device) = core {
            let game_info = if let Some(path) = device.path.as_ref() {
                GameInfo::new(
                    game.name.clone(),
                    game.path.to_owned(),
                    path.display().to_string(),
                    vec![game.path.display().to_string()],
                )
            } else if let Some(retroarch_core) = device.cores.first() {
                GameInfo::new(
                    game.name.clone(),
                    game.path.to_owned(),
                    ALLIUM_RETROARCH.display().to_string(),
                    vec![retroarch_core.to_owned(), game.path.display().to_string()],
                )
            } else {
                bail!("Device \"{}\" has no path or cores.", device.name);
            };
            game_info.save()?;
            Some(AlliumCommand::Exec(game_info.command()))
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
    fn test_core_mapper() {
        let mut mapper = DeviceMapper::new();
        mapper.devices = vec![Device {
            name: "Test".to_string(),
            folders: vec!["POKE".to_string(), "PKM".to_string()],
            extensions: vec!["gb".to_string(), "gbc".to_string()],
            cores: vec![],
            path: None,
        }];

        assert!(mapper.get_device(Path::new("Roms/POKE/rom.zip")).is_some());
        assert!(mapper.get_device(Path::new("Roms/PKM/rom.zip")).is_some());
        assert!(mapper
            .get_device(Path::new("Roms/Pokemon Mini (POKE)/rom.zip"))
            .is_some());
        assert!(mapper
            .get_device(Path::new("Roms/POKE MINI/rom.zip"))
            .is_none());
        assert!(mapper.get_device(Path::new("Roms/rom.gb")).is_some());
        assert!(mapper.get_device(Path::new("Roms/rom.gbc")).is_some());
        assert!(mapper.get_device(Path::new("Roms/rom.gbc.zip")).is_some());
        assert!(mapper.get_device(Path::new("Roms/rom.zip.gbc")).is_some());
        assert!(mapper.get_device(Path::new("Roms/gbc")).is_some());
        assert!(mapper.get_device(Path::new("Roms/rom.gba")).is_none());
    }

    #[test]
    fn test_config() {
        env::set_var("ALLIUM_CONFIG_DIR", "../assets/root/.allium");

        let mut mapper = DeviceMapper::new();
        mapper.load_config().unwrap();

        let eq = |rom: &str, device_name: &str, core: &str| -> bool {
            let device = mapper.get_device(Path::new(rom)).unwrap();
            if device.name == device_name && device.cores.first() == Some(&core.to_string()) {
                true
            } else {
                println!(
                    "Expected device: {} core: {:?}, got device: {} core: {}",
                    device_name,
                    device.cores.first(),
                    device.name,
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
        assert!(eq("rom.pbp", "Sony - PlayStation", "pcsx_rearmed"));

        // Neo Geo Pocket
        assert!(eq("NGP/rom", "SNK - Neo Geo Pocket Color", "mednafen_ngp"));
        assert!(eq("NGC/rom", "SNK - Neo Geo Pocket Color", "mednafen_ngp"));
        assert!(eq("rom.ngp", "SNK - Neo Geo Pocket Color", "mednafen_ngp"));
        assert!(eq("rom.ngc", "SNK - Neo Geo Pocket Color", "mednafen_ngp"));

        // Sega - Game Gear
        assert!(eq("GG/rom", "Sega - Game Gear", "picodrive"));
        assert!(eq("rom.gg", "Sega - Game Gear", "picodrive"));
    }
}
