mod battery;
mod display;
mod evdev;
mod framebuffer;
mod hardware;
mod screen;
mod volume;

use std::fmt;
use std::fs::File;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::process::Command;

use anyhow::Result;
use async_trait::async_trait;
use log::warn;

use crate::battery::Battery;
use crate::display::settings::DisplaySettings;
use crate::platform::miyoo::evdev::EvdevKeys;
use crate::platform::miyoo::framebuffer::FramebufferDisplay;
use crate::platform::{Display, KeyEvent, Platform};

use self::battery::{Miyoo283Battery, Miyoo354Battery};

pub struct MiyooPlatform {
    model: MiyooDeviceModel,
    keys: EvdevKeys,
}

pub struct SuspendContext {
    brightness: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MiyooDeviceModel {
    Miyoo283,
    Miyoo354,
}

impl fmt::Display for MiyooDeviceModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MiyooDeviceModel::Miyoo283 => write!(f, "Miyoo Mini (MY283)"),
            MiyooDeviceModel::Miyoo354 => write!(f, "Miyoo Mini+ (MY354)"),
        }
    }
}

#[async_trait(?Send)]
impl Platform for MiyooPlatform {
    type Display = FramebufferDisplay;
    type Battery = Box<dyn Battery>;
    type SuspendContext = SuspendContext;

    fn new() -> Result<MiyooPlatform> {
        let model = hardware::detect_model();

        Ok(MiyooPlatform {
            model,
            keys: EvdevKeys::new()?,
        })
    }

    async fn poll(&mut self) -> KeyEvent {
        self.keys.poll().await
    }

    fn display(&mut self) -> Result<FramebufferDisplay> {
        FramebufferDisplay::new()
    }

    fn battery(&self) -> Result<Box<dyn Battery>> {
        Ok(match self.model {
            MiyooDeviceModel::Miyoo283 => Box::new(Miyoo283Battery::new()),
            MiyooDeviceModel::Miyoo354 => Box::new(Miyoo354Battery::new()),
        })
    }

    fn shutdown(&self) -> Result<()> {
        #[cfg(unix)]
        {
            std::process::Command::new("sync").spawn()?.wait()?;
            match self.model {
                MiyooDeviceModel::Miyoo283 => {
                    std::process::Command::new("reboot").exec();
                }
                MiyooDeviceModel::Miyoo354 => {
                    std::process::Command::new("poweroff").exec();
                }
            }
        }
        Ok(())
    }

    fn suspend(&self) -> Result<Self::SuspendContext> {
        let brightness = screen::get_brightness()?;
        let ctx = SuspendContext { brightness };
        screen::set_brightness(0)?;
        screen::blank(true)?;
        Ok(ctx)
    }

    fn unsuspend(&self, ctx: Self::SuspendContext) -> Result<()> {
        screen::blank(false)?;
        screen::set_brightness(ctx.brightness)?;
        Ok(())
    }

    fn set_volume(&mut self, volume: i32) -> Result<()> {
        match self.model {
            MiyooDeviceModel::Miyoo283 => Ok(()),
            MiyooDeviceModel::Miyoo354 => volume::set_volume(volume),
        }
    }

    fn get_brightness(&self) -> Result<u8> {
        screen::get_brightness()
    }

    fn set_brightness(&mut self, brightness: u8) -> Result<()> {
        screen::set_brightness(brightness)
    }

    fn set_display_settings(&mut self, settings: &mut DisplaySettings) -> Result<()> {
        if settings.contrast < 10 {
            settings.contrast = 10;
        }

        let mut file = match File::create("/proc/mi_modules/mi_disp/mi_disp0") {
            Ok(file) => file,
            Err(err) => {
                warn!("failed to open display settings file: {}", err);
                return Ok(());
            }
        };

        if settings.r < 15 && settings.g < 15 && settings.b < 15 {
            settings.r = 15;
            settings.g = 15;
            settings.b = 15;
        }

        file.write_all(
            format!(
                "csc 0 3 {:.0} {:.0} {:.0} {:.0} 0 0\n",
                settings.contrast, settings.hue, settings.luminance, settings.saturation,
            )
            .as_bytes(),
        )?;
        file.write_all(
            format!(
                "colortemp 0 0 0 0 {:.0} {:.0} {:.0}\n",
                settings.b as f32 * 255.0 / 100.0,
                settings.g as f32 * 255.0 / 100.0,
                settings.r as f32 * 255.0 / 100.0,
            )
            .as_bytes(),
        )?;

        Ok(())
    }

    fn device_model() -> String {
        hardware::detect_model().to_string()
    }

    fn firmware() -> String {
        hardware::detect_firmware()
    }

    fn has_wifi() -> bool {
        match detect_model() {
            MiyooDeviceModel::Miyoo283 => false,
            MiyooDeviceModel::Miyoo354 => true,
        }
    }
}

impl Default for MiyooPlatform {
    fn default() -> Self {
        Self::new().unwrap()
    }
}
