use std::fs::File;
use std::io::Write;

use anyhow::Result;

pub fn set_brightness(brightness: u8) -> Result<()> {
    let mut file = File::create("/sys/devices/soc0/soc/1f003400.pwm/pwm/pwmchip0/pwm0/duty_cycle")?;
    file.write_all(format!("{}", brightness.max(3)).as_bytes())?;
    Ok(())
}
