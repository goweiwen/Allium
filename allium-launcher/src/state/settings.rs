use anyhow::Result;
use embedded_graphics::{
    prelude::*,
    primitives::{Primitive, PrimitiveStyle, Rectangle},
};
use serde::{Deserialize, Serialize};

use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::stylesheet::Stylesheet;

use crate::{command::AlliumCommand, state::State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsState {}

impl SettingsState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for SettingsState {
    fn enter(&mut self) -> Result<()> {
        Ok(())
    }

    fn leave(&mut self) -> Result<()> {
        Ok(())
    }

    fn draw(
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

    fn handle_key_event(&mut self, _key_event: KeyEvent) -> Result<(Option<AlliumCommand>, bool)> {
        Ok((None, false))
    }
}
