use std::process::Command;

use anyhow::Result;
use serde::Deserialize;

use crate::platform::miyoo::battery::Battery;

pub struct Miyoo283Battery {
    is_charging: bool,
    percentage: i32,
}

impl Miyoo283Battery {
    pub fn new() -> Miyoo283Battery {
        Miyoo283Battery {
            is_charging: false,
            percentage: 100,
        }
    }
}

impl Battery for Miyoo283Battery {
    fn update(&mut self) -> Result<()> {
        // TODO
        Ok(())
    }

    fn percentage(&self) -> i32 {
        self.percentage
    }

    fn is_charging(&self) -> bool {
        self.is_charging
    }
}
