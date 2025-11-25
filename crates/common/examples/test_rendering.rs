//! Test rendering example for the tiny-skia migration
//!
//! This example tests various rendering primitives:
//! - Text rendering with different alignments
//! - Rounded rectangles with anti-aliasing
//! - Circles with anti-aliasing
//! - Regular rectangles
//! - Image rendering with alpha blending
//!
//! Run with: cargo run --example test_rendering --features simulator

use anyhow::Result;
use common::display::color::Color;
use common::display::font::FontTextStyleBuilder;
use common::display::{Display, fill_circle, fill_rect, fill_rounded_rect};
use common::geom::{Point, Rect};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;

#[tokio::main]
async fn main() -> Result<()> {
    let mut platform = DefaultPlatform::new()?;
    let mut display = platform.display()?;
    let styles = Stylesheet::load()?;

    // Clear background
    let (width, height) = (display.width(), display.height());
    fill_rect(
        &mut display.pixmap_mut(),
        Rect::new(0, 0, width, height),
        Color::new(40, 44, 52), // Dark blue-gray background
    );

    // Test 1: Text rendering
    let text_style = FontTextStyleBuilder::new(styles.ui.ui_font.font())
        .font_size(24)
        .text_color(Color::new(255, 255, 255))
        .build();

    text_style.draw(
        &mut display.pixmap_mut(),
        "tiny-skia Migration Test",
        Point::new(20, 30),
    );

    // Test 2: Rounded rectangles with different radii
    let colors = [
        Color::new(224, 108, 117), // Red
        Color::new(152, 195, 121), // Green
        Color::new(97, 175, 239),  // Blue
        Color::new(198, 120, 221), // Purple
    ];

    for (i, color) in colors.iter().enumerate() {
        let y = 80 + i as i32 * 70;
        fill_rounded_rect(
            &mut display.pixmap_mut(),
            Rect::new(20, y, 200, 50),
            (5 + i * 5) as u32,
            *color,
        );
    }

    // Test 3: Circles with anti-aliasing
    let circle_colors = [
        Color::new(209, 154, 102), // Orange
        Color::new(86, 182, 194),  // Cyan
        Color::new(229, 192, 123), // Yellow
    ];

    for (i, color) in circle_colors.iter().enumerate() {
        let x = 300 + i as i32 * 80;
        fill_circle(&mut display.pixmap_mut(), Point::new(x, 150), 30, *color);
    }

    // Test 4: Regular rectangles (no AA)
    fill_rect(
        &mut display.pixmap_mut(),
        Rect::new(300, 250, 150, 100),
        Color::rgba(255, 255, 255, 128), // Semi-transparent white
    );

    // Test 5: Small text labels
    let small_text = FontTextStyleBuilder::new(styles.ui.ui_font.font())
        .font_size(16)
        .text_color(Color::new(171, 178, 191))
        .build();

    small_text.draw(
        &mut display.pixmap_mut(),
        "Rounded Rects",
        Point::new(20, 400),
    );
    small_text.draw(&mut display.pixmap_mut(), "Circles", Point::new(300, 220));
    small_text.draw(
        &mut display.pixmap_mut(),
        "Alpha Blend",
        Point::new(300, 370),
    );

    // Test 6: Success message
    let success_style = FontTextStyleBuilder::new(styles.ui.ui_font.font())
        .font_size(20)
        .text_color(Color::new(152, 195, 121))
        .build();

    success_style.draw(
        &mut display.pixmap_mut(),
        "✓ Rendering tests complete!",
        Point::new(20, 450),
    );

    small_text.draw(
        &mut display.pixmap_mut(),
        "Press ESC to exit",
        Point::new(20, 475),
    );

    display.flush()?;

    // Event loop - wait for exit key
    loop {
        let event = platform.poll().await;
        if let KeyEvent::Pressed(Key::Menu) = event {
            break;
        }
    }

    Ok(())
}
