use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::prelude::*;

use crate::battery::Battery;
use crate::display::color::Color;
use crate::display::settings::DisplaySettings;
use crate::display::Display;
use crate::geom::Rect;
use crate::platform::{KeyEvent, Platform};

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

    fn display(&mut self) -> Result<Self::Display> {
        Ok(MockDisplay)
    }

    fn battery(&self) -> Result<Self::Battery> {
        Ok(MockBattery)
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn set_volume(&mut self, _volume: i32) -> Result<()> {
        Ok(())
    }

    fn get_brightness(&self) -> Result<u8> {
        Ok(50)
    }

    fn set_brightness(&mut self, _brightness: u8) -> Result<()> {
        Ok(())
    }

    fn set_display_settings(&mut self, _settings: &DisplaySettings) -> Result<()> {
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
    fn map_pixels<F>(&mut self, _f: F) -> Result<()>
    where
        F: FnMut(Color) -> Color,
    {
        Ok(())
    }

    fn save(&mut self) -> Result<()> {
        Ok(())
    }

    fn load(&mut self, _area: Rect) -> Result<()> {
        Ok(())
    }
}

impl DrawTarget for MockDisplay {
    type Color = Color;

    type Error = anyhow::Error;

    fn draw_iter<I>(&mut self, _pixels: I) -> Result<()>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        Ok(())
    }
}

impl OriginDimensions for MockDisplay {
    fn size(&self) -> Size {
        Size::new(SCREEN_WIDTH, SCREEN_HEIGHT)
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
