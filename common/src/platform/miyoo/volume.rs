use anyhow::Result;
use ffi::{MI_AO_GetVolume, MI_AO_SetMute, MI_AO_SetVolume};
use log::debug;

const MAX_VOLUME: i32 = 20;
const MIN_RAW_VALUE: i32 = -60;
const MAX_RAW_VALUE: i32 = 0;

/// Set volume output between -60 and 30
fn set_volume_raw(volume: i32) -> Result<()> {
    let mut prev_volume = 0;
    unsafe { MI_AO_GetVolume(0, &mut prev_volume) };

    let volume = volume.clamp(MIN_RAW_VALUE, MAX_RAW_VALUE);
    unsafe {
        MI_AO_SetVolume(0, volume);
    }

    if prev_volume <= MIN_RAW_VALUE && volume > MIN_RAW_VALUE {
        unsafe {
            MI_AO_SetMute(0, false as u8);
        }
    } else if prev_volume > MIN_RAW_VALUE && volume <= MIN_RAW_VALUE {
        unsafe {
            MI_AO_SetMute(0, true as u8);
        }
    }

    Ok(())
}

// Volume curve:
// |   0 |   1 |   2 |   3 |   4 |   5 |   6 |   7 |   8 |   9 |  10 |  11 |  12 |  13 |  14 |  15 |  16 |  17 |  18 |  19 |  20 |
// | -60 | -46 | -38 | -33 | -28 | -25 | -22 | -19 | -17 | -15 | -13 | -11 |  -9 |  -8 |  -7 |  -5 |  -4 |  -3 |  -2 |  -1 |   0 |
pub fn set_volume(volume: i32) -> Result<()> {
    let volume = volume.clamp(0, MAX_VOLUME);
    let volume_raw = (volume as f32 + 1.0).log10() / 21f32.log10() * 60.0 - 60.0;
    debug!("set volume: {}", volume_raw as i32);
    set_volume_raw(volume_raw as i32)?;
    Ok(())
}
