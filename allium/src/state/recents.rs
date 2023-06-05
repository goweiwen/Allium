use anyhow::Result;
use embedded_graphics::{
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};
use serde::{Deserialize, Serialize};

use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::stylesheet::Stylesheet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentsState {}

impl RecentsState {
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
        styles: &Stylesheet,
    ) -> Result<()> {
        let Size { width, height } = display.size();
        Rectangle::new(Point::new(0, 46), Size::new(width, height - 46))
            .into_styled(PrimitiveStyle::with_fill(styles.bg_color))
            .draw(display)?;
        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn handle_key_event(&mut self, _key_event: KeyEvent) -> Result<bool> {
        Ok(false)
    }
}
