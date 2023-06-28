use anyhow::Result;

pub fn set(volume: i32) -> Result<()> {
    unsafe {
        ffi::MI_AO_SetVolume(0, volume);
    }
    Ok(())
}

pub fn get() -> Result<i32> {
    let mut volume = 0;
    unsafe {
        ffi::MI_AO_GetVolume(0, &mut volume);
    }
    Ok(volume)
}
