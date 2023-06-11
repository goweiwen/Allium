use std::{
    fs::{self, File},
    io::Write,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

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
        if ALLIUM_DISPLAY_SETTINGS.exists() {
            debug!("found state, loading from file");
            if let Ok(json) = fs::read_to_string(ALLIUM_DISPLAY_SETTINGS.as_path()) {
                if let Ok(json) = serde_json::from_str(&json) {
                    return Ok(json);
                }
            }
            warn!("failed to read state file, removing");
            fs::remove_file(ALLIUM_DISPLAY_SETTINGS.as_path())?;
        }
        Ok(Self::new())
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
