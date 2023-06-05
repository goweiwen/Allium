use rusttype::Font;

use crate::platform::Color;

pub struct Stylesheet {
    pub fg_color: Color,
    pub bg_color: Color,
    pub primary: Color,
    pub button_a_color: Color,
    pub button_b_color: Color,
    pub button_x_color: Color,
    pub button_y_color: Color,
    pub ui_font: Font<'static>,
    pub ui_font_size: u32,
}

impl Default for Stylesheet {
    fn default() -> Self {
        Self {
            fg_color: Color::new(255, 255, 255),
            bg_color: Color::new(0, 0, 0),
            primary: Color::new(151, 135, 187),
            button_a_color: Color::new(235, 26, 29),
            button_b_color: Color::new(254, 206, 21),
            button_x_color: Color::new(7, 73, 180),
            button_y_color: Color::new(0, 141, 69),
            ui_font: Font::try_from_bytes(include_bytes!("../../assets/font/Lato/Lato-Bold.ttf"))
                .unwrap(),
            ui_font_size: 24,
        }
    }
}
