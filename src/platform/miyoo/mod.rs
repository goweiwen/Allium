mod battery;
mod evdev;
mod framebuffer;

use anyhow::Result;

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
        let model = detect_model();

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

    pub async fn poll(&mut self) -> Result<Option<KeyEvent>> {
        self.keys.poll().await
    }

    pub fn display(&mut self) -> &mut FramebufferDisplay {
        &mut self.display
    }

    pub fn flush(&mut self) -> Result<()> {
        self.display.flush()
    }

    pub fn display_size(&self) -> (i32, i32) {
        (640, 480)
    }

    pub fn update_battery(&mut self) -> Result<()> {
        self.battery.update()
    }

    pub fn battery_percentage(&self) -> i32 {
        self.battery.percentage()
    }
}

fn detect_model() -> MiyooDeviceModel {
    if std::path::Path::new("/customer/app/axp_test").exists() {
        MiyooDeviceModel::Miyoo354
    } else {
        MiyooDeviceModel::Miyoo283
    }
}
