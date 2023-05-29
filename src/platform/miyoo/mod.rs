use anyhow::Result;

use crate::platform::miyoo::evdev::EvdevKeys;
use crate::platform::miyoo::framebuffer::FramebufferDisplay;
use crate::platform::KeyEvent;

mod evdev;
mod framebuffer;

pub struct MiyooPlatform {
    display: FramebufferDisplay,
    keys: EvdevKeys,
}

impl MiyooPlatform {
    pub fn new() -> Result<MiyooPlatform> {
        let display = FramebufferDisplay::new()?;

        Ok(MiyooPlatform {
            display,
            keys: EvdevKeys::new()?,
        })
    }

    pub async fn init() -> Result<()> {
        FramebufferDisplay::init().await?;
        Ok(())
    }

    pub async fn poll(&mut self) -> Result<Option<KeyEvent>> {
        self.keys.poll().await
    }

    pub fn display(&mut self) -> Result<&mut FramebufferDisplay> {
        Ok(&mut self.display)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.display.flush()
    }
}
