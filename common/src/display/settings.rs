use std::{
    fs::{self, File},
    io::Write,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

use crate::constants::ALLIUM_DISPLAY_SETTINGS;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplaySettings {
    pub brightness: u8,
    pub luminance: u8,
    pub hue: u8,
    pub saturation: u8,
    pub contrast: u8,
}

impl DisplaySettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load() -> Result<Self> {
        trace!(
            "loading display settings: {}",
            ALLIUM_DISPLAY_SETTINGS.display()
        );
        Ok(if !ALLIUM_DISPLAY_SETTINGS.exists() {
            debug!("can't find display settings, creating new");
            Self::new()
        } else {
            debug!("found display settings, loading from file");
            let json = fs::read_to_string(ALLIUM_DISPLAY_SETTINGS.as_path())?;
            serde_json::from_str(&json)?
        })
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string(&self).unwrap();
        File::create(ALLIUM_DISPLAY_SETTINGS.as_path())?.write_all(json.as_bytes())?;
        Ok(())
    }
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            brightness: 50,
            luminance: 50,
            hue: 50,
            saturation: 50,
            contrast: 50,
        }
    }
}
