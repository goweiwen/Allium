use anyhow::Result;
use async_trait::async_trait;
use tiny_skia::{Pixmap, PixmapMut, PixmapRef};

use crate::battery::Battery;
use crate::display::Display;
use crate::display::color::Color;
use crate::display::settings::DisplaySettings;
use crate::geom::Rect;
use crate::platform::{KeyEvent, Platform};

pub const SCREEN_WIDTH: u32 = 640;
pub const SCREEN_HEIGHT: u32 = 480;

pub struct MockPlatform;

#[async_trait(?Send)]
impl Platform for MockPlatform {
    type Display = MockDisplay;
    type Battery = MockBattery;
    type SuspendContext = ();

    fn new() -> Result<MockPlatform> {
        Ok(MockPlatform)
    }

    async fn poll(&mut self) -> KeyEvent {
        std::future::pending().await
    }

    fn display(&mut self) -> Result<Self::Display> {
        Ok(MockDisplay::new())
    }

    fn battery(&self) -> Result<Self::Battery> {
        Ok(MockBattery)
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn suspend(&self) -> Result<Self::SuspendContext> {
        Ok(())
    }

    fn unsuspend(&self, _ctx: Self::SuspendContext) -> Result<()> {
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

    fn set_display_settings(&mut self, _settings: &mut DisplaySettings) -> Result<()> {
        Ok(())
    }

    fn device_model() -> String {
        "Mock".into()
    }

    fn firmware() -> String {
        "00000000".to_string()
    }

    fn has_wifi() -> bool {
        false
    }

    fn has_lid() -> bool {
        false
    }
}

impl Default for MockPlatform {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

pub struct MockDisplay {
    pixmap: Pixmap,
}

impl MockDisplay {
    pub fn new() -> Self {
        Self {
            pixmap: Pixmap::new(SCREEN_WIDTH, SCREEN_HEIGHT).expect("Failed to create mock pixmap"),
        }
    }
}

impl Default for MockDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for MockDisplay {
    fn width(&self) -> u32 {
        SCREEN_WIDTH
    }

    fn height(&self) -> u32 {
        SCREEN_HEIGHT
    }

    fn pixmap(&self) -> PixmapRef<'_> {
        self.pixmap.as_ref()
    }

    fn pixmap_mut(&mut self) -> PixmapMut<'_> {
        self.pixmap.as_mut()
    }

    fn map_pixels<F>(&mut self, mut f: F) -> Result<()>
    where
        F: FnMut(Color) -> Color,
    {
        for pixel in self.pixmap.pixels_mut() {
            let color: Color = (*pixel).into();
            *pixel = f(color).into();
        }
        Ok(())
    }

    fn save(&mut self) -> Result<()> {
        Ok(())
    }

    fn load(&mut self, _area: Rect) -> Result<()> {
        Ok(())
    }

    fn pop(&mut self) -> bool {
        true
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
