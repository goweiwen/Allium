use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;
use tokio::process::{Child, Command};

use crate::constants::ALLIUM_CONFIG_DIR;

#[derive(Debug, Clone, Deserialize)]
pub struct Core {
    extensions: Vec<String>,
    path: PathBuf,
}

impl Core {
    pub fn launch(&self, rom: &PathBuf) -> Result<Child> {
        Command::new(&self.path)
            .arg(rom)
            .spawn()
            .context("Failed to launch core")
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
        self.cores = config.cores.into_values().collect::<Vec<Core>>();

        Ok(())
    }

    pub fn get_core(&self, extension: &str) -> Option<&Core> {
        self.cores
            .iter()
            .find(|core| core.extensions.contains(&extension.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn test_core_mapper() {
        env::set_var("ALLIUM_CONFIG_DIR", "./assets/root/.allium");

        let mut mapper = CoreMapper::new();
        mapper.load_config().unwrap();

        let core = mapper.get_core("gba");
        assert!(core.is_some());

        let core = mapper.get_core("gbc");
        assert!(core.is_some());

        let core = mapper.get_core("gb");
        assert!(core.is_some());
    }
}
