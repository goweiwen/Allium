use std::fs::{self, File};
use std::io::Write;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::display::settings::DisplaySettings;

pub fn get_brightness() -> Result<u8> {
    Ok(
        fs::read_to_string("/sys/devices/soc0/soc/1f003400.pwm/pwm/pwmchip0/pwm0/duty_cycle")?
            .trim()
            .parse()?,
    )
}

pub fn set_brightness(brightness: u8) -> Result<()> {
    File::create("/sys/devices/soc0/soc/1f003400.pwm/pwm/pwmchip0/pwm0/duty_cycle")
        .context("failed to open pwm/duty_cycle")?
        .write_all(format!("{}", brightness.max(3)).as_bytes())?;
    Ok(())
}

pub fn blank(blank: bool) -> Result<()> {
    File::create("/proc/mi_modules/fb/mi_fb0")
        .context("failed to open mi_fb0")?
        .write_all(if blank {
            b"GUI_SHOW 0 off"
        } else {
            b"GUI_SHOW 0 on"
        })?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct SystemConfig {
    vol: u8,
    keymap: String,
    mute: u8,
    bgmvol: u8,
    brightness: u8,
    language: String,
    hibernate: u8,
    lumination: u8,
    hue: u8,
    saturation: u8,
    contrast: u8,
    theme: String,
    fontsize: u8,
    audiofix: u8,
    wifi: u8,
}

pub fn set_display_settings(settings: &DisplaySettings) -> Result<()> {
    let json = fs::read_to_string("/appconfigs/system.json")?;

    let mut config: SystemConfig = serde_json::from_str(&json)?;

    // Expects 20 as maximum, but we use 100 as maximum.
    config.lumination = settings.luminance / 5;
    config.hue = settings.hue / 5;
    config.saturation = settings.saturation / 5;
    config.contrast = settings.contrast / 5;

    let file = File::create("/appconfigs/system.json")?;
    serde_json::to_writer(file, &config)?;
    Ok(())
}
