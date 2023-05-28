use anyhow::Result;
use embedded_font::FontTextStyleBuilder;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{
        Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, Triangle,
    },
    text::{Alignment, Text},
};
use rusttype::Font;

use backend::FrameBufferDisplay;
use wifi::Wifi;

mod backend;
mod wifi;

fn main() -> Result<()> {
    let mut display = FrameBufferDisplay::new();

    // Create styles used by the drawing operations.
    let thin_stroke = PrimitiveStyle::with_stroke(Rgb888::new(255, 255, 255), 1);
    let thick_stroke = PrimitiveStyle::with_stroke(Rgb888::new(255, 255, 255), 3);
    let border_stroke = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb888::new(255, 255, 255))
        .stroke_width(3)
        .stroke_alignment(StrokeAlignment::Inside)
        .build();
    let fill = PrimitiveStyle::with_fill(Rgb888::new(255, 255, 255));
    let font = Font::try_from_bytes(include_bytes!("../assets/font/Jua/Jua-Regular.ttf")).unwrap();
    let character_style = FontTextStyleBuilder::new(font)
        .font_size(32)
        .text_color(Rgb888::new(255, 255, 255))
        .build();
    // let character_style = MonoTextStyle::new(&FONT_6X10, Rgb888::new(255, 255, 255));

    let yoffset = 14;

    // Draw a 3px wide outline around the display.
    display
        .bounding_box()
        .into_styled(border_stroke)
        .draw(&mut display)?;

    // Draw a triangle.
    Triangle::new(
        Point::new(16, 16 + yoffset),
        Point::new(16 + 16, 16 + yoffset),
        Point::new(16 + 8, yoffset),
    )
    .into_styled(thin_stroke)
    .draw(&mut display)?;

    // Draw a filled square
    Rectangle::new(Point::new(52, yoffset), Size::new(16, 16))
        .into_styled(fill)
        .draw(&mut display)?;

    // Draw a circle with a 3px wide stroke.
    Circle::new(Point::new(88, yoffset), 17)
        .into_styled(thick_stroke)
        .draw(&mut display)?;

    // Draw centered text.
    let text = "embedded-graphics";
    Text::with_alignment(
        text,
        display.bounding_box().center() + Point::new(0, 15),
        character_style,
        Alignment::Center,
    )
    .draw(&mut display)?;

    display.flush();

    Ok(())
}
