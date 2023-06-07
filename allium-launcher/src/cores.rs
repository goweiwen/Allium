use std::env;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::{collections::HashMap, path::Path};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use common::constants::{ALLIUM_CONFIG_DIR, ALLIUM_RETROARCH};

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
    pub fn launch(&self, rom: &PathBuf) -> Result<()> {
        if let Some(path) = self.path.as_ref() {
            #[cfg(windows)]
            Command::new(&path)
                .arg(rom)
                .spawn()
                .context("Failed to launch core")?;

            #[cfg(unix)]
            Command::new(&path).arg(rom).exec();
        } else if let Some(retroarch_core) = self.retroarch_core.as_ref() {
            #[cfg(windows)]
            Command::new(ALLIUM_RETROARCH)
                .arg(retroarch_core)
                .arg(rom)
                .spawn()
                .context("Failed to launch core")?;

            #[cfg(unix)]
            Command::new(ALLIUM_RETROARCH)
                .arg(retroarch_core)
                .arg(rom)
                .exec();
        } else {
            bail!("No path or retroarch_core specified for core {}", self.name);
        }
        Ok(())
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
        let roms_dir: PathBuf = env::var("ALLIUM_CONFIG_DIR")
            .unwrap_or(ALLIUM_CONFIG_DIR.to_owned())
            .into();
        let config = std::fs::read_to_string(roms_dir.join("cores.toml"))
            .context("Failed to load cores.toml. Is ALLIUM_CONFIG_DIR set correctly?")?;
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

    pub fn get_core(&self, path: &Path) -> Option<&Core> {
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

        if let Some(parent_filename) = path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|parent| parent.to_str())
        {
            let core = self.cores.iter().find(|core| {
                core.patterns
                    .iter()
                    .any(|pattern| parent_filename.contains(&format!("({})", pattern)))
            });
            if core.is_some() {
                return core;
            }
        }

        None
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
