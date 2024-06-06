use anyhow::Result;
use async_trait::async_trait;

use crate::battery::Battery;
use crate::platform::{Display, KeyEvent, Platform};

pub const SCREEN_WIDTH: u32 = 640;
pub const SCREEN_HEIGHT: u32 = 480;

pub struct MockPlatform;

#[async_trait(?Send)]
impl Platform for MockPlatform {
    type Display = MockDisplay;
    type Battery = MockBattery;

    fn new() -> Result<MockPlatform> {
        Ok(MockPlatform)
    }

    async fn poll(&mut self) -> KeyEvent {
        std::future::pending().await
    }

    fn battery(&self) -> Result<Self::Battery> {
        Ok(MockBattery)
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn device_model() -> String {
        "Mock".to_string()
    }

    fn firmware() -> String {
        "00000000".to_string()
    }

    fn has_wifi() -> bool {
        false
    }
}

impl Default for MockPlatform {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

pub struct MockDisplay;

impl Display for MockDisplay {
    fn draw(&mut self, _pixels: &[u32]) -> Result<()> {
        Ok(())
    }
}

pub struct MockBattery;

impl Battery for MockBattery {
    fn update(&mut self) -> Result<()> {
        Ok(())
    }

    fn percentage(&self) -> i32 {
        50
    }

    fn charging(&self) -> bool {
        false
    }
}
