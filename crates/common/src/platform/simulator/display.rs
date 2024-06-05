use anyhow::Result;
use minifb::Window;

use crate::platform::simulator::{SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::platform::Display;

pub struct SimulatorDisplay {
    window: Window,
}

impl SimulatorDisplay {
    pub fn new(window: Window) -> Self {
        Self { window }
    }
}

impl Display for SimulatorDisplay {
    fn draw(&mut self, buffer: &[u32]) -> Result<()> {
        Ok(self
            .window
            .update_with_buffer(buffer, SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize)?)
    }
}
