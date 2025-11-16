#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use clap::Parser;
use common::{
    display::{Display, color::Color, font::FontTextStyleBuilder},
    platform::{DefaultPlatform, Platform},
    stylesheet::Stylesheet,
};
use embedded_graphics::{
    Drawable,
    prelude::{Dimensions, OriginDimensions, Point, Size},
    primitives::{CornerRadii, Primitive, PrimitiveStyle, RoundedRectangle},
    text::{Alignment, Text},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Text to display
    text: String,

    /// Whether to draw a box behind the text
    #[arg(short, long)]
    bg: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Err(e) = say(&cli.text, cli.bg) {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

fn say(text: &str, bg: bool) -> Result<()> {
    let mut platform = DefaultPlatform::new()?;
    let mut display = platform.display()?;
    let styles = Stylesheet::load()?;

    let text_style = FontTextStyleBuilder::<Color>::new(styles.ui.ui_font.font())
        .text_color(styles.ui.text_color)
        .font_fallback(styles.cjk_font.font())
        .font_size(styles.ui.ui_font.size)
        .build();

    let w = display.size().width;
    let h = display.size().height;
    let height = text.lines().count() as u32 * styles.ui.ui_font.size;

    let text = Text::with_alignment(
        text,
        Point::new(w as i32 / 2, (h - height) as i32 / 2),
        text_style,
        Alignment::Center,
    );

    if bg {
        let mut rect = text.bounding_box();
        rect.top_left.x -= 12;
        rect.top_left.y -= 8;
        rect.size.width += 24;
        rect.size.height += 16;
        RoundedRectangle::new(
            rect,
            CornerRadii::new(Size::new_equal((styles.ui.ui_font.size + 8) / 2)),
        )
        .into_styled(PrimitiveStyle::with_fill(styles.ui.highlight_color))
        .draw(&mut display)?;
    }

    text.draw(&mut display)?;
    display.flush()?;

    Ok(())
}
