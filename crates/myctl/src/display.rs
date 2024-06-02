#![allow(unreachable_code, unused_variables)]
use std::fs::File;
use std::io::Write;

use anyhow::{Context, Result};

pub fn blank() -> Result<()> {
    File::create("/proc/mi_modules/fb/mi_fb0")
        .context("failed to open mi_fb0")?
        .write_all(b"GUI_SHOW 0 off")?;
    Ok(())
}

pub fn unblank() -> Result<()> {
    File::create("/proc/mi_modules/fb/mi_fb0")
        .context("failed to open mi_fb0")?
        .write_all(b"GUI_SHOW 0 on")?;
    Ok(())
}
