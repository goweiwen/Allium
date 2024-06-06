use iced::{
    font::{Family, Weight},
    Color, Font, Pixels,
};
use serde::{Deserialize, Serialize};

pub trait Style {
    fn font(&self) -> Font;
    fn font_size(&self) -> impl Into<Pixels>;
    fn text_color(&self) -> Color;
    fn background_color(&self) -> Color;
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Stylesheet {}

impl Style for Stylesheet {
    fn font(&self) -> Font {
        Font {
            family: Family::SansSerif,
            weight: Weight::Bold,
            ..Default::default()
        }
    }

    fn font_size(&self) -> impl Into<Pixels> {
        16.0
    }

    fn text_color(&self) -> Color {
        Color::WHITE
    }

    fn background_color(&self) -> Color {
        Color::BLACK
    }
}
