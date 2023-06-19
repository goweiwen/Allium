pub mod color;
pub mod font;
pub mod image;
pub mod settings;

use anyhow::Result;

use embedded_graphics::prelude::*;

use crate::display::color::Color;

use crate::geom::Rect;

pub trait Display:
    OriginDimensions + DrawTarget<Color = Color, Error = anyhow::Error> + Sized
{
    fn map_pixels<F>(&mut self, f: F) -> Result<()>
    where
        F: FnMut(Color) -> Color;

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    fn save(&mut self) -> Result<()>;
    fn load(&mut self, area: Rect) -> Result<()>;
}
