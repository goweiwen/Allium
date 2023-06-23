mod battery;
mod evdev;
mod framebuffer;
mod screen;
mod volume;

use std::fmt;
use std::process::Command;

use anyhow::Result;
use async_trait::async_trait;

use crate::battery::Battery;
use crate::display::settings::DisplaySettings;
use crate::platform::miyoo::evdev::EvdevKeys;
use crate::platform::miyoo::framebuffer::FramebufferDisplay;
use crate::platform::KeyEvent;
use crate::platform::Platform;

use self::battery::{Miyoo283Battery, Miyoo354Battery};

pub struct MiyooPlatform {
    model: MiyooDeviceModel,
    keys: EvdevKeys,
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

    fn new() -> Result<MiyooPlatform> {
        let model = detect_model();

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
            self.update_play_time()?;
            std::process::Command::new("sync").spawn()?;
            match self.model {
                MiyooDeviceModel::Miyoo283 => {
                    std::process::Command::new("reboot").exec()?;
                }
                MiyooDeviceModel::Miyoo354 => {
                    std::process::Command::new("poweroff").exec();
                }
            }
        }
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

    fn set_display_settings(&mut self, settings: &DisplaySettings) -> Result<()> {
        screen::set_display_settings(settings)
    }

    fn device_model() -> String {
        detect_model().to_string()
    }

    fn firmware() -> String {
        detect_firmware()
    }
}

impl Default for MiyooPlatform {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

fn detect_model() -> MiyooDeviceModel {
    if std::path::Path::new("/customer/app/axp_test").exists() {
        MiyooDeviceModel::Miyoo354
    } else {
        MiyooDeviceModel::Miyoo283
    }
}

fn detect_firmware() -> String {
    let stdout = Command::new("/etc/fw_printenv").output().unwrap().stdout;
    let stdout = std::str::from_utf8(&stdout).unwrap();
    parse_firmware(stdout).to_string()
}

fn parse_firmware(data: &str) -> &str {
    for line in data.lines() {
        if line.starts_with("miyoo_version=") {
            return &line[14..];
        }
    }
    ""
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_firmware() {
        let data = r#"SdUpgradeImage=miyoo354_fw.img
baudrate=115200
bootargs=console=ttyS0,115200 root=/dev/mtdblock4 rootfstype=squashfs ro init=/linuxrc LX_MEM=0x7f00000 mma_heap=mma_heap_name0,miu=0,sz=0x1500000 mma_memblock_remove=1 highres=off mmap_reserved=fb,miu=0,sz=0x300000,max_start_off=0x7C00000,max_end_off=0x7F00000
bootcmd=gpio output 85 1; bootlogo 0 0 0 0 0; mw 1f001cc0 11; gpio out 8 0; sf probe 0;sf read 0x22000000 ${sf_kernel_start} ${sf_kernel_size}; gpio out 8 1; sleepms 1000; gpio output 4 1; bootm 0x22000000
bootdelay=0
cpu_part_start=14270000
dispout=K101_IM2BVL
ethact=sstar_emac
ethaddr=00:30:1b:ba:02:db
filesize=1774c
miyoo_version=202303262339
sf_kernel_size=200000
sf_kernel_start=60000
sf_part_size=20000
sf_part_start=270000
stderr=serial
stdin=serial
stdout=serial
usb_folder=images
"#;
        assert_eq!(parse_firmware(data), "202303262339");
    }
}
