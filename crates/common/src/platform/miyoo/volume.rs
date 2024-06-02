use anyhow::Result;
use log::debug;
use std::process::Command;

const MIN_VOLUME: i32 = 0;
const MAX_VOLUME: i32 = 20;

/// Set volume output between -60 and 30
fn set_volume_raw(volume: i32) -> Result<()> {
    Command::new("myctl")
        .arg("volume")
        .arg(volume.to_string())
        .spawn()?
        .wait()?;
    Ok(())
}

// Volume curve:
// |   0 |   1 |   2 |   3 |   4 |   5 |   6 |   7 |   8 |   9 |  10 |  11 |  12 |  13 |  14 |  15 |  16 |  17 |  18 |  19 |  20 |
// | -60 | -46 | -38 | -33 | -28 | -25 | -22 | -19 | -17 | -15 | -13 | -11 |  -9 |  -8 |  -7 |  -5 |  -4 |  -3 |  -2 |  -1 |   0 |
pub fn set_volume(volume: i32) -> Result<()> {
    let volume = volume.clamp(MIN_VOLUME, MAX_VOLUME);
    let volume_raw = (volume as f32 + 1.0).log10() / 21f32.log10() * 60.0 - 60.0;
    debug!("set volume: {}", volume_raw as i32);
    set_volume_raw(volume_raw as i32)?;
    Ok(())
}
