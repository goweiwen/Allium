use anyhow::Result;

use crate::battery::Battery;

pub struct Miyoo283Battery {
    charging: bool,
    percentage: i32,
}

impl Miyoo283Battery {
    pub fn new() -> Miyoo283Battery {
        Miyoo283Battery {
            charging: false,
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

    fn charging(&self) -> bool {
        self.charging
    }
}
