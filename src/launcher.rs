use std::{env, path::PathBuf};

use anyhow::Result;

use crate::constants::ALLIUM_ROMS_DIR;

pub struct Launcher {
    roms_dir: PathBuf,
}

impl Launcher {
    pub fn new() -> Launcher {
        let roms_dir: PathBuf = env::var("ALLIUM_ROMS_DIR")
            .unwrap_or(ALLIUM_ROMS_DIR.to_owned())
            .into();
        Launcher { roms_dir }
    }

    pub fn roms(&self) -> Result<impl Iterator<Item = Result<PathBuf>>> {
        Ok(std::fs::read_dir(&self.roms_dir)?.map(|entry| Ok(entry?.path())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roms() -> Result<()> {
        env::set_var("ALLIUM_ROMS_DIR", "./assets/roms");
        let launcher = Launcher::new();
        for rom in launcher.roms()? {
            println!("{:?}", rom?);
        }
        Ok(())
    }
}
