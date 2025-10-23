use anyhow::Result;

const MIN_RAW_VALUE: i32 = -60;
const MAX_RAW_VALUE: i32 = 0;

// Real implementations for the device target (ARM)
#[cfg(target_arch = "arm")]
pub fn set(volume: i32) -> Result<()> {
    let mut prev_volume = 0;
    unsafe { ffi::MI_AO_GetVolume(0, &mut prev_volume) };

    let volume = volume.clamp(MIN_RAW_VALUE, MAX_RAW_VALUE);
    unsafe { ffi::MI_AO_SetVolume(0, volume) };

    if prev_volume <= MIN_RAW_VALUE && volume > MIN_RAW_VALUE {
        unsafe { ffi::MI_AO_SetMute(0, false as u8) };
    } else if prev_volume > MIN_RAW_VALUE && volume <= MIN_RAW_VALUE {
        unsafe { ffi::MI_AO_SetMute(0, true as u8) };
    }

    Ok(())
}

#[cfg(target_arch = "arm")]
pub fn get() -> Result<i32> {
    let mut volume = 0;
    unsafe { ffi::MI_AO_GetVolume(0, &mut volume) };
    Ok(volume)
}

// Stub implementations for host builds (e.g., CI/testing on x86_64)
#[cfg(not(target_arch = "arm"))]
pub fn set(_volume: i32) -> Result<()> {
    Ok(())
}

#[cfg(not(target_arch = "arm"))]
pub fn get() -> Result<i32> {
    Ok(0)
}
