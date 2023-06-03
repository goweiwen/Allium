mod battery;
mod evdev;
mod framebuffer;
mod volume;

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use tracing::warn;

use crate::battery::Battery;
use crate::platform::miyoo::evdev::EvdevKeys;
use crate::platform::miyoo::framebuffer::FramebufferDisplay;
use crate::platform::KeyEvent;
use crate::platform::Platform;

use self::battery::{Miyoo283Battery, Miyoo354Battery};

pub struct MiyooPlatform {
    model: MiyooDeviceModel,
    keys: EvdevKeys,
}

impl MiyooPlatform {
    fn link_retroarch(model: MiyooDeviceModel) {
        let mut binary = PathBuf::from("/mnt/SDCARD/RetroArch");
        binary.push(match model {
            MiyooDeviceModel::Miyoo283 => "retroarch_miyoo283",
            MiyooDeviceModel::Miyoo354 => "retroarch_miyoo354",
        });

        if binary.exists() {
            if let Err(e) = fs::copy(binary, "/mnt/SDCARD/RetroArch/retroarch") {
                warn!("Failed to link RetroArch: {}", e);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MiyooDeviceModel {
    Miyoo283,
    Miyoo354,
}

impl Platform for MiyooPlatform {
    type Display = FramebufferDisplay;
    type Battery = Box<dyn Battery>;

    fn new() -> Result<MiyooPlatform> {
        let model = detect_model();

        Self::link_retroarch(model);

        Ok(MiyooPlatform {
            model,
            keys: EvdevKeys::new()?,
        })
    }

    async fn poll(&mut self) -> Result<Option<KeyEvent>> {
        self.keys.poll().await
    }

    fn display(&mut self) -> Result<FramebufferDisplay> {
        Ok(FramebufferDisplay::new()?)
    }

    fn battery(&self) -> Result<Box<dyn Battery>> {
        Ok(match self.model {
            MiyooDeviceModel::Miyoo283 => Box::new(Miyoo283Battery::new()),
            MiyooDeviceModel::Miyoo354 => Box::new(Miyoo354Battery::new()),
        })
    }

    fn set_volume(&mut self, volume: i32) -> Result<()> {
        match self.model {
            MiyooDeviceModel::Miyoo283 => Ok(()),
            MiyooDeviceModel::Miyoo354 => volume::set_volume(volume),
        }
    }
}

fn detect_model() -> MiyooDeviceModel {
    if std::path::Path::new("/customer/app/axp_test").exists() {
        MiyooDeviceModel::Miyoo354
    } else {
        MiyooDeviceModel::Miyoo283
    }
}
