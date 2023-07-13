#![warn(clippy::all, rust_2018_idioms)]

use std::env;

use anyhow::Result;
use common::{
    display::{color::Color, font::FontTextStyleBuilder, Display},
    platform::{DefaultPlatform, Platform},
    stylesheet::Stylesheet,
};
use embedded_graphics::{
    prelude::{Dimensions, OriginDimensions, Point, Size},
    primitives::{CornerRadii, Primitive, PrimitiveStyle, RoundedRectangle},
    text::{Alignment, Text},
    Drawable,
};

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let text = match args.next() {
        Some(text) => text,
        None => {
            eprintln!("Usage: say <text>");
            return Ok(());
        }
    };

    if let Err(e) = say(&text) {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

fn say(text: &str) -> Result<()> {
    let mut platform = DefaultPlatform::new()?;
    let mut display = platform.display()?;
    let styles = Stylesheet::load()?;

    let text_style = FontTextStyleBuilder::<Color>::new(styles.ui_font.font())
        .text_color(styles.foreground_color)
        .font_fallback(styles.cjk_font.font())
        .font_size(styles.ui_font.size)
        .build();

    let w = display.size().width;
    let h = display.size().height;
    let height = text.lines().count() as u32 * styles.ui_font.size;

    let text = Text::with_alignment(
        text,
        Point::new(w as i32 / 2, (h - height) as i32 / 2),
        text_style,
        Alignment::Center,
    );

    let mut rect = text.bounding_box();
    rect.top_left.x -= 12;
    rect.top_left.y -= 8;
    rect.size.width += 24;
    rect.size.height += 16;
    RoundedRectangle::new(
        rect,
        CornerRadii::new(Size::new_equal((styles.ui_font.size + 8) / 2)),
    )
    .into_styled(PrimitiveStyle::with_fill(styles.highlight_color))
    .draw(&mut display)?;

    text.draw(&mut display)?;
    display.flush()?;

    Ok(())
}
