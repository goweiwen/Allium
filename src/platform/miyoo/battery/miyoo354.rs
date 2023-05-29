use std::process::Command;

use anyhow::Result;
use serde::Deserialize;

use crate::platform::miyoo::battery::Battery;

#[derive(Deserialize)]
struct BatteryCommandOutput {
    battery: i32,
    voltage: i32,
    charging: i32,
}

pub struct Miyoo354Battery {
    is_charging: bool,
    percentage: i32,
}

impl Miyoo354Battery {
    pub fn new() -> Miyoo354Battery {
        Miyoo354Battery {
            is_charging: false,
            percentage: 100,
        }
    }
}

impl Battery for Miyoo354Battery {
    fn update(&mut self) -> Result<()> {
        let output = Command::new("./axp_test")
            .current_dir("/customer/app/")
            .output()?;
        let output = String::from_utf8(output.stdout)?;
        let output: BatteryCommandOutput = serde_json::from_str(&output)?;
        self.percentage = map_battery_percentage(output.battery);
        self.is_charging = output.charging != 0;
        Ok(())
    }

    fn percentage(&self) -> i32 {
        self.percentage
    }

    fn is_charging(&self) -> bool {
        self.is_charging
    }
}

fn map_battery_percentage(value: i32) -> i32 {
    if value == 100 {
        500
    } else if value >= 578 {
        100
    } else if value >= 528 {
        value - 478
    } else if value >= 512 {
        (value as f32 * 2.125 - 1068.0) as i32
    } else if value >= 480 {
        (value as f32 * 0.51613 - 243.742) as i32
    } else {
        0
    }
}
