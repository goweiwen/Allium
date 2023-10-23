use std::fs::{self, File};
use std::io::Write;

use anyhow::{Context, Result};

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
