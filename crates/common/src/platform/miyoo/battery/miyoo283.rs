use std::fs::File;
use std::os::fd::AsRawFd;

use anyhow::Result;
use log::error;
use nix::{ioctl_write_ptr_bad, request_code_none};
use sysfs_gpio::{Direction, Pin};

use crate::battery::Battery;

const SARADC_IOC_MAGIC: u8 = b'a';

ioctl_write_ptr_bad!(sar_init, request_code_none!(SARADC_IOC_MAGIC, 0), ());
ioctl_write_ptr_bad!(
    sar_set_channel_read_value,
    request_code_none!(SARADC_IOC_MAGIC, 1),
    AdcConfig
);

pub struct AdcConfig {
    _channel: i32,
    adc_value: i32,
}

pub struct Miyoo283Battery {
    percentage: i32,
    adc_value: i32,
}

impl Miyoo283Battery {
    pub fn new() -> Miyoo283Battery {
        let battery = Miyoo283Battery {
            percentage: 100,
            adc_value: 0,
        };

        let sar_fd = match File::open("/dev/sar") {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to open /dev/sar: {}", e);
                return battery;
            }
        };

        if let Err(e) = unsafe { sar_init(sar_fd.as_raw_fd(), &()) } {
            error!("Failed to initialize /dev/sar: {}", e);
            return battery;
        }

        battery
    }

    #[inline]
    fn charging(&self) -> Result<bool> {
        let gpio59 = Pin::new(59);
        gpio59.export()?;
        gpio59.set_direction(Direction::In)?;
        Ok(gpio59.get_value()? != 0)
    }
}

impl Battery for Miyoo283Battery {
    fn update(&mut self) -> Result<()> {
        if Battery::charging(self) {
            self.percentage = 100;
        } else {
            self.adc_value = read_adc_value(self.adc_value);
            self.percentage = battery_percentage(self.adc_value);
        }
        Ok(())
    }

    fn percentage(&self) -> i32 {
        self.percentage
    }

    fn charging(&self) -> bool {
        match self.charging() {
            Ok(charging) => charging,
            Err(e) => {
                error!("Failed to read charging status: {}", e);
                false
            }
        }
    }
}

#[inline]
fn read_adc_value(mut value: i32) -> i32 {
    let adc_config = AdcConfig {
        _channel: 0,
        adc_value: 0,
    };

    let sar_fd = match File::open("/dev/sar") {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to open /dev/sar: {}", e);
            return value;
        }
    };

    if let Err(e) = unsafe { sar_set_channel_read_value(sar_fd.as_raw_fd(), &adc_config) } {
        error!("Failed to read /dev/sar: {}", e);
        return value;
    }

    if value <= 100 {
        value = adc_config.adc_value
    } else if adc_config.adc_value > value {
        value += 1;
    } else if adc_config.adc_value < value {
        value -= 1;
    }

    value
}

#[inline]
fn battery_percentage(value: i32) -> i32 {
    match value {
        578..=i32::MAX => 100,
        528..=577 => value - 478,
        512..=527 => (value as f64 * 2.125 - 1068.0) as i32,
        480..=511 => (value as f64 * 0.51613 - 243.742) as i32,
        _ => 0,
    }
}
