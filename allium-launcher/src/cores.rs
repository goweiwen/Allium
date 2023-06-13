use std::path::PathBuf;
use std::{collections::HashMap, path::Path};

use anyhow::{anyhow, bail, Context, Result};
use common::database::Database;
use common::game_info::GameInfo;
use serde::{Deserialize, Serialize};

use common::constants::{ALLIUM_CONFIG_DIR, ALLIUM_GAMES_DIR, ALLIUM_RETROARCH};

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

    pub fn launch_game(
        &self,
        database: &Database,
        game: &mut Game,
    ) -> Result<Option<AlliumCommand>> {
        game.image();
        database.increment_play_count(
            &game.name,
            game.path.as_path(),
            game.image_ref(),
        )?;

        let core = self.get_core(game.path.as_path());
        Ok(if let Some(core) = core {
            let game_info = if let Some(path) = core.path.as_ref() {
                GameInfo::new(
                    game.name.clone(),
                    game.path.to_owned(),
                    path.display().to_string(),
                    vec![game.path.display().to_string()],
                )
            } else if let Some(retroarch_core) = core.retroarch_core.as_ref() {
                GameInfo::new(
                    game.name.clone(),
                    game.path.to_owned(),
                    ALLIUM_RETROARCH.display().to_string(),
                    vec![retroarch_core.to_owned(), game.path.display().to_string()],
                )
            } else {
                bail!("Core {} has no path or retroarch_core.", core.name);
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
