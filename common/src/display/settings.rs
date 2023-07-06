use std::{
    fs::{self, File},
    io::Write,
};

use anyhow::Result;
use log::{debug, warn};
use serde::{Deserialize, Serialize};

use crate::constants::ALLIUM_DISPLAY_SETTINGS;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplaySettings {
    pub luminance: u8,
    pub hue: u8,
    pub saturation: u8,
    pub contrast: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl DisplaySettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&self) -> Result<()> {
        let mut file = File::create("/proc/mi_modules/mi_disp/mi_disp0")?;
        file.write_all(
            format!(
                "csc 0 3 {:.0} {:.0} {:.0} {:.0} 0 0\n",
                self.contrast, self.hue, self.luminance, self.saturation,
            )
            .as_bytes(),
        )?;
        file.write_all(
            format!(
                "colortemp 0 0 0 0 {:.0} {:.0} {:.0}\n",
                self.b as f32 * 255.0 / 100.0,
                self.g as f32 * 255.0 / 100.0,
                self.r as f32 * 255.0 / 100.0,
            )
            .as_bytes(),
        )?;

        Ok(())
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
            luminance: 50,
            hue: 50,
            saturation: 50,
            contrast: 50,
            r: 50,
            g: 50,
            b: 50,
        }
    }
}
