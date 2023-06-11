use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::{collections::HashMap, path::Path};

use anyhow::{anyhow, Context, Result};
use common::database::Database;
use serde::{Deserialize, Serialize};

use common::constants::{ALLIUM_CONFIG_DIR, ALLIUM_GAME_INFO, ALLIUM_RETROARCH};
use tracing::{trace, warn};

use crate::command::AlliumCommand;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Core {
    #[serde(skip)]
    pub name: String,
    /// If ithe parent directory matches a pattern exactly, or contains this string within parenthesis, this core will be used.
    /// e.g. patterns: "POKE", "PKM"
    /// Roms/POKE/rom.zip -> matches
    /// Roms/PKM/rom.zip -> matches
    /// Roms/Pokemon Mini (POKE)/rom.zip -> matches
    /// Roms/POKE MINI/rom.zip -> doesn't match
    pub patterns: Vec<String>,
    /// If any of the extensions of the rom matches this string, this core will be used.
    /// The extension should be lowercase.
    /// e.g. extensions: "gb", "gbc"
    /// Roms/rom.gb -> matches
    /// Roms/rom.gbc -> matches
    /// Roms/rom.gbc.zip -> matches
    /// Roms/rom.gba -> doesn't match
    pub extensions: Vec<String>,
    pub path: Option<PathBuf>,
    pub retroarch_core: Option<String>,
}

impl Core {
    pub fn launch(&self, rom: &PathBuf) -> Option<AlliumCommand> {
        trace!("launching: {:?}", rom);
        if let Some(path) = self.path.as_ref() {
            let mut cmd = Command::new(path);
            cmd.arg(rom);
            Some(AlliumCommand::Exec(cmd))
        } else if let Some(retroarch_core) = self.retroarch_core.as_ref() {
            trace!("ra: {:?}, core: {:?}", &*ALLIUM_RETROARCH, retroarch_core);
            let mut cmd = Command::new(ALLIUM_RETROARCH.as_path());
            cmd.arg(retroarch_core).arg(rom);
            Some(AlliumCommand::Exec(cmd))
        } else {
            warn!("No path or retroarch_core specified for core {}", self.name);
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Game {
    pub name: String,
    pub full_name: String,
    pub path: PathBuf,
    pub image: Option<PathBuf>,
    pub extension: String,
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
struct CoreConfig {
    cores: HashMap<String, Core>,
}

#[derive(Debug, Clone)]
pub struct CoreMapper {
    cores: Vec<Core>,
}

impl Default for CoreMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreMapper {
    pub fn new() -> CoreMapper {
        CoreMapper { cores: Vec::new() }
    }

    pub fn load_config(&mut self) -> Result<()> {
        let config =
            std::fs::read_to_string(ALLIUM_CONFIG_DIR.join("cores.toml")).map_err(|e| {
                anyhow!(
                    "Failed to load cores config: {:?}, {}",
                    &*ALLIUM_CONFIG_DIR.join("cores.toml"),
                    e
                )
            })?;
        let config: CoreConfig = toml::from_str(&config).context("Failed to parse cores.toml.")?;
        self.cores = config
            .cores
            .into_iter()
            .map(|(name, mut core)| {
                core.name = name;
                core
            })
            .collect();

        Ok(())
    }

    pub fn get_core(&self, mut path: &Path) -> Option<&Core> {
        let path_lowercase = path.as_os_str().to_ascii_lowercase();
        let Some(extensions) = path_lowercase.to_str() else {
            return None;
        };
        for ext in extensions.split('.') {
            let core = self
                .cores
                .iter()
                .find(|core| core.extensions.iter().any(|s| s == ext));
            if core.is_some() {
                return core;
            }
        }

        while let Some(parent) = path.parent() {
            if let Some(parent_filename) = parent.file_name().and_then(|parent| parent.to_str()) {
                let core = self.cores.iter().find(|core| {
                    core.patterns
                        .iter()
                        .any(|pattern| parent_filename.contains(&format!("({})", pattern)))
                });
                if core.is_some() {
                    return core;
                }
            }
            path = parent;
        }

        None
    }

    pub fn launch_game(&self, database: &Database, game: &Game) -> Result<Option<AlliumCommand>> {
        database.increment_play_count(
            &game.name,
            game.path.as_path(),
            game.image.as_deref(),
        )?;

        let core = self.get_core(game.path.as_path());
        Ok(if let Some(core) = core {
            if let Some(path) = core.path.as_ref() {
                write!(
                    File::create(ALLIUM_GAME_INFO.as_path())?,
                    "{}\n{}\n{}",
                    game.name,
                    path.as_path().as_os_str().to_str().unwrap_or(""),
                    game.path.as_path().as_os_str().to_str().unwrap_or(""),
                )?;
            } else if let Some(retroarch_core) = core.retroarch_core.as_ref() {
                write!(
                    File::create(&*ALLIUM_GAME_INFO)?,
                    "{}\n{}\n{}\n{}",
                    game.name,
                    ALLIUM_RETROARCH.as_os_str().to_str().unwrap_or(""),
                    retroarch_core,
                    game.path.as_path().as_os_str().to_str().unwrap_or(""),
                )?;
            }
            core.launch(&game.path)
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
        let mut mapper = CoreMapper::new();
        mapper.cores = vec![Core {
            name: "Test".to_string(),
            patterns: vec!["POKE".to_string(), "PKM".to_string()],
            extensions: vec!["gb".to_string(), "gbc".to_string()],
            path: None,
            retroarch_core: None,
        }];

        assert!(mapper.get_core(Path::new("Roms/POKE/rom.zip")).is_some());
        assert!(mapper.get_core(Path::new("Roms/PKM/rom.zip")).is_some());
        assert!(mapper
            .get_core(Path::new("Roms/Pokemon Mini (POKE)/rom.zip"))
            .is_some());
        assert!(mapper
            .get_core(Path::new("Roms/POKE MINI/rom.zip"))
            .is_none());
        assert!(mapper.get_core(Path::new("Roms/rom.gb")).is_some());
        assert!(mapper.get_core(Path::new("Roms/rom.gbc")).is_some());
        assert!(mapper.get_core(Path::new("Roms/rom.gbc.zip")).is_some());
        assert!(mapper.get_core(Path::new("Roms/rom.zip.gbc")).is_some());
        assert!(mapper.get_core(Path::new("Roms/gbc")).is_some());
        assert!(mapper.get_core(Path::new("Roms/rom.gba")).is_none());
    }

    #[test]
    fn test_config() {
        env::set_var("ALLIUM_CONFIG_DIR", "./assets/root/.allium");

        let mut mapper = CoreMapper::new();
        mapper.load_config().unwrap();

        // GB, GBC
        let core = mapper
            .cores
            .iter()
            .find(|core| core.name == "gambatte")
            .unwrap();
        assert_eq!(
            Some(core),
            mapper.get_core(Path::new("Roms/Game Boy (GB)/rom.zip"))
        );
        assert_eq!(
            Some(core),
            mapper.get_core(Path::new("Roms/Game Boy Color (GBC)/rom.zip"))
        );
        assert_eq!(Some(core), mapper.get_core(Path::new("Roms/GB/rom.zip")));
        assert_eq!(Some(core), mapper.get_core(Path::new("Roms/GBC/rom.zip")));
        assert_eq!(Some(core), mapper.get_core(Path::new("Roms/rom.gb")));
        assert_eq!(Some(core), mapper.get_core(Path::new("Roms/rom.gbc")));

        // mGBA
        let core = mapper
            .cores
            .iter()
            .find(|core| core.name == "mgba")
            .unwrap();
        assert_eq!(
            Some(core),
            mapper.get_core(Path::new("Roms/Game Boy Advance (GBA)/rom.zip"))
        );
        assert_eq!(Some(core), mapper.get_core(Path::new("Roms/GBA/rom.zip")));
        assert_eq!(Some(core), mapper.get_core(Path::new("Roms/rom.gba")));
        assert_eq!(Some(core), mapper.get_core(Path::new("Roms/rom.sgb")));

        // NES
        let core = mapper
            .cores
            .iter()
            .find(|core| core.name == "fceumm")
            .unwrap();
        assert_eq!(
            Some(core),
            mapper.get_core(Path::new(
                "Roms/Nintendo Entertainment System (NES)/rom.zip"
            ))
        );
        assert_eq!(Some(core), mapper.get_core(Path::new("Roms/NES/rom.zip")));
        assert_eq!(Some(core), mapper.get_core(Path::new("Roms/FC/rom.zip")));
        assert_eq!(Some(core), mapper.get_core(Path::new("Roms/rom.nes")));

        // SNES
        let core = mapper
            .cores
            .iter()
            .find(|core| core.name == "fceumm")
            .unwrap();
        assert_eq!(Some(core), mapper.get_core(Path::new("Roms/SNES/rom.zip")));
        assert_eq!(Some(core), mapper.get_core(Path::new("Roms/SFC/rom.zip")));
        assert_eq!(Some(core), mapper.get_core(Path::new("Roms/rom.sfc")));
        assert_eq!(Some(core), mapper.get_core(Path::new("Roms/rom.smc")));
    }
}
