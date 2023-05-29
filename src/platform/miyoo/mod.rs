mod battery;
mod evdev;
mod framebuffer;

use anyhow::{bail, Result};

use crate::platform::miyoo::evdev::EvdevKeys;
use crate::platform::miyoo::framebuffer::FramebufferDisplay;
use crate::platform::KeyEvent;

use self::battery::{Battery, Miyoo283Battery, Miyoo354Battery};

pub struct MiyooPlatform {
    model: MiyooDeviceModel,
    display: FramebufferDisplay,
    keys: EvdevKeys,
    battery: Box<dyn Battery>,
}

enum MiyooDeviceModel {
    Miyoo283,
    Miyoo354,
}

impl MiyooPlatform {
    pub fn new() -> Result<MiyooPlatform> {
        let model = match std::fs::read_to_string("/tmp/deviceModel")?.as_str() {
            "283" => MiyooDeviceModel::Miyoo283,
            "354" => MiyooDeviceModel::Miyoo354,
            model => bail!("Unknown device model: {}", model),
        };

        let display = FramebufferDisplay::new()?;

        let battery: Box<dyn Battery> = match model {
            MiyooDeviceModel::Miyoo283 => Box::new(Miyoo283Battery::new()),
            MiyooDeviceModel::Miyoo354 => Box::new(Miyoo354Battery::new()),
        };

        Ok(MiyooPlatform {
            model,
            display,
            keys: EvdevKeys::new()?,
            battery,
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

    pub fn battery_percentage(&self) -> i32 {
        self.battery.percentage()
    }
}
