use anyhow::Result;
use embedded_graphics::{prelude::*, primitives::Rectangle};

use common::stylesheet::Stylesheet;
use common::{
    display::Display,
    platform::{DefaultPlatform, KeyEvent, Platform},
};

#[derive(Debug, Clone)]
pub struct NetplayState {}

impl NetplayState {
    pub fn new() -> Self {
        Self {}
    }

    pub fn enter(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn leave(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        _styles: &Stylesheet,
    ) -> Result<()> {
        let Size { width, height } = display.size();
        display.load(Rectangle::new(
            Point::new(0, 46),
            Size::new(width, height - 46),
        ))?;
        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn handle_key_event(&mut self, _key_event: KeyEvent) -> Result<bool> {
        Ok(false)
    }
}
