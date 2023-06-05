use embedded_graphics::pixelcolor::Rgb888;
use rusttype::Font;

pub struct Stylesheet {
    pub fg_color: Rgb888,
    pub bg_color: Rgb888,
    pub primary: Rgb888,
    pub button_a_color: Rgb888,
    pub button_b_color: Rgb888,
    pub button_x_color: Rgb888,
    pub button_y_color: Rgb888,
    pub ui_font: Font<'static>,
    pub ui_font_size: u32,
}

impl Default for Stylesheet {
    fn default() -> Self {
        Self {
            fg_color: Rgb888::new(255, 255, 255),
            bg_color: Rgb888::new(0, 0, 0),
            primary: Rgb888::new(151, 135, 187),
            button_a_color: Rgb888::new(235, 26, 29),
            button_b_color: Rgb888::new(254, 206, 21),
            button_x_color: Rgb888::new(7, 73, 180),
            button_y_color: Rgb888::new(0, 141, 69),
            ui_font: Font::try_from_bytes(include_bytes!("../../assets/font/Lato/Lato-Bold.ttf"))
                .unwrap(),
            ui_font_size: 24,
        }
    }
}
