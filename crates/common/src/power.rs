use std::fs::{self, File};

use anyhow::Result;
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use strum::FromRepr;

use crate::constants::ALLIUM_POWER_SETTINGS;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerSettings {
    pub power_button_action: PowerButtonAction,
    pub auto_sleep_when_charging: bool,
    pub auto_sleep_duration_minutes: i32,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, FromRepr, Default)]
pub enum PowerButtonAction {
    #[default]
    Suspend,
    Shutdown,
    Nothing,
}

impl Default for PowerSettings {
    fn default() -> Self {
        Self {
            power_button_action: PowerButtonAction::Suspend,
            auto_sleep_when_charging: true,
            auto_sleep_duration_minutes: 5,
        }
    }
}

impl PowerSettings {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn load() -> Result<Self> {
        if ALLIUM_POWER_SETTINGS.exists() {
            debug!("found state, loading from file");
            let file = File::open(ALLIUM_POWER_SETTINGS.as_path())?;
            if let Ok(json) = serde_json::from_reader(file) {
                return Ok(json);
            }
            warn!("failed to read power file, removing");
            fs::remove_file(ALLIUM_POWER_SETTINGS.as_path())?;
        }
        Ok(Self::new())
    }

    pub fn save(&self) -> Result<()> {
        let file = File::create(ALLIUM_POWER_SETTINGS.as_path())?;
        serde_json::to_writer(file, &self)?;
        Ok(())
    }
}
