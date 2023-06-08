use anyhow::Result;
use embedded_graphics::{
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};
use serde::{Deserialize, Serialize};

use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::stylesheet::Stylesheet;

use crate::{command::AlliumCommand, state::State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentsState {}

impl RecentsState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for RecentsState {
    fn enter(&mut self) -> Result<()> {
        Ok(())
    }

    fn leave(&mut self) -> Result<()> {
        Ok(())
    }

    fn draw(
        &self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<()> {
        let Size { width, height } = display.size();
        Rectangle::new(Point::new(0, 46), Size::new(width, height - 46))
            .into_styled(PrimitiveStyle::with_fill(styles.bg_color))
            .draw(display)?;
        Ok(())
    }

    fn handle_key_event(&mut self, _key_event: KeyEvent) -> Result<(Option<AlliumCommand>, bool)> {
        Ok((None, false))
    }
}
