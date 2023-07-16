use std::io::Read;
use std::process::Command;
use std::time::Duration;
use wait_timeout::ChildExt;

use anyhow::Result;
use log::trace;
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
            percentage: 100,
        }
    }
}

impl Battery for Miyoo354Battery {
    fn update(&mut self) -> Result<()> {
        let mut child = Command::new("/customer/app/axp_test").spawn()?;
        let mut stdout = child.stdout.take().unwrap();
        match child.wait_timeout(Duration::from_millis(100))? {
            Some(_) => (),
            None => {
                child.kill()?;
                child.wait()?;
                return Ok(());
            }
        }
        let mut output = Vec::with_capacity(128);
        stdout.read_to_end(&mut output)?;
        let output = String::from_utf8(output)?;
        let output: BatteryCommandOutput = serde_json::from_str(&output)?;
        self.percentage = output.battery;
        self.charging = output.charging == 3;

        trace!("battery: {}%", self.percentage);
        Ok(())
    }

    fn percentage(&self) -> i32 {
        self.percentage
    }

    fn charging(&self) -> bool {
        self.charging
    }
}
