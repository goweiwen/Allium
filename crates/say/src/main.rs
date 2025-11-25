#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use clap::Parser;
use common::{
    display::{Display, font::FontTextStyleBuilder},
    geom::{Point, Rect},
    platform::{DefaultPlatform, Platform},
    stylesheet::Stylesheet,
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

    let text_color = if bg {
        styles.ui.highlight_text_color
    } else {
        styles.ui.text_color
    };
    let text_style = FontTextStyleBuilder::new(styles.ui.ui_font.font())
        .text_color(text_color)
        .font_fallback(styles.cjk_font.font())
        .font_size(styles.ui.ui_font.size)
        .build();

    let w = display.size().w;
    let h = display.size().h;
    let height = text.lines().count() as u32 * styles.ui.ui_font.size;

    // Measure text for centered positioning
    let text_size = text_style.measure(text);
    let text_pos = Point::new(
        w as i32 / 2 - text_size.w as i32 / 2,
        (h - height) as i32 / 2,
    );

    if bg {
        let bg_rect = Rect::new(
            text_pos.x - 12,
            text_pos.y - 8,
            text_size.w + 24,
            text_size.h + 16,
        );
        let radius = (styles.ui.ui_font.size + 8) / 2;
        common::display::fill_rounded_rect(
            &mut display.pixmap_mut(),
            bg_rect,
            radius,
            styles.ui.highlight_color,
        );
    }

    text_style.draw(&mut display.pixmap_mut(), text, text_pos);
    display.flush()?;

    Ok(())
}
