mod battery;
mod display;
mod input;

use std::process;

use anyhow::Result;
use async_trait::async_trait;
use minifb::{Window, WindowOptions};
use tokio::sync::mpsc::{channel, Receiver};

use crate::platform::simulator::input::SimulatorInput;
use crate::platform::{Display, KeyEvent, Platform};
use battery::SimulatorBattery;
use display::SimulatorDisplay;

pub const SCREEN_WIDTH: u32 = 640;
pub const SCREEN_HEIGHT: u32 = 480;

pub struct SimulatorPlatform {
    display: SimulatorDisplay,
    inputs: Receiver<KeyEvent>,
}

#[async_trait(?Send)]
impl Platform for SimulatorPlatform {
    type Display = SimulatorDisplay;
    type Battery = SimulatorBattery;

    fn new() -> Result<SimulatorPlatform> {
        let mut window = Window::new(
            "Allium",
            SCREEN_WIDTH as usize,
            SCREEN_HEIGHT as usize,
            WindowOptions::default(),
        )?;

        let (tx, rx) = channel(128);
        let input = SimulatorInput::new(tx);
        window.set_input_callback(Box::new(input));

        let display = SimulatorDisplay::new(window);

        Ok(Self {
            display,
            inputs: rx,
        })
    }

    async fn poll(&mut self) -> KeyEvent {
        self.inputs.recv().await.unwrap()
    }

    fn battery(&self) -> Result<SimulatorBattery> {
        Ok(SimulatorBattery::new())
    }

    fn shutdown(&self) -> Result<()> {
        process::exit(0);
    }

    fn device_model() -> String {
        "Simulator".to_string()
    }

    fn firmware() -> String {
        "00000000".to_string()
    }

    fn has_wifi() -> bool {
        true
    }
}

impl Default for SimulatorPlatform {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl Display for SimulatorPlatform {
    fn draw(&mut self, buffer: &[u32]) -> Result<()> {
        self.display.draw(buffer)
    }
}
