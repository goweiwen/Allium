use std::process::Command;

use anyhow::Result;
use log::debug;
use serde::Deserialize;

use crate::battery::Battery;

#[derive(Deserialize)]
struct BatteryCommandOutput {
    battery: i32,
    #[allow(dead_code)]
    voltage: i32,
    charging: i32,
}

pub struct Miyoo354Battery {
    charging: bool,
    percentage: i32,
}

impl Miyoo354Battery {
    pub fn new() -> Miyoo354Battery {
        Miyoo354Battery {
            charging: false,
            percentage: 0,
        }
    }
}

impl Battery for Miyoo354Battery {
    fn update(&mut self) -> Result<()> {
        let output = Command::new("/customer/app/axp_test").output()?;
        let output = String::from_utf8(output.stdout)?;
        let output: BatteryCommandOutput = serde_json::from_str(&output)?;
        self.percentage = output.battery;
        self.charging = output.charging == 3;

        debug!("battery: {}%", self.percentage);
        Ok(())
    }

    fn percentage(&self) -> i32 {
        self.percentage
    }

    fn charging(&self) -> bool {
        self.charging
    }
}
