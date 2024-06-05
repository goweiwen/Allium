use anyhow::Result;
use log::trace;

use crate::battery::Battery;

pub struct SimulatorBattery {
    percentage: i32,
    charging: bool,
}

impl SimulatorBattery {
    pub fn new() -> SimulatorBattery {
        SimulatorBattery {
            percentage: 100,
            charging: false,
        }
    }
}

impl Default for SimulatorBattery {
    fn default() -> Self {
        Self::new()
    }
}

impl Battery for SimulatorBattery {
    fn update(&mut self) -> Result<()> {
        trace!("Updating battery");
        if self.percentage > 0 {
            self.percentage -= 5
        }
        self.charging = !self.charging;
        Ok(())
    }

    fn percentage(&self) -> i32 {
        self.percentage
    }

    fn charging(&self) -> bool {
        self.charging
    }
}
