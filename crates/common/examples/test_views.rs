//! Test views example for the tiny-skia migration
//!
//! This example tests the View trait and common views:
//! - Label with different alignments and colors
//! - BatteryIcon with different charge levels
//! - Row for horizontal composition
//!
//! Run with: cargo run --example test_views --features simulator

use anyhow::Result;
use common::display::color::Color;
use common::display::{Display, fill_rect};
use common::geom::{Alignment, Point, Rect};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use common::view::{BatteryIcon, Label, Row, View};

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

    // Save initial state for views that need to restore background
    display.save()?;

    // === Test 1: Labels with different alignments ===
    let mut title = Label::new(
        Point::new(width as i32 / 2, 30),
        "View Tests",
        Alignment::Center,
        None,
    );
    title.font_size(1.5);
    title.draw(&mut display, &styles)?;

    // Left-aligned label
    let mut label_left = Label::new(Point::new(20, 80), "Left Aligned", Alignment::Left, None);
    label_left.draw(&mut display, &styles)?;

    // Center-aligned label
    let mut label_center = Label::new(
        Point::new(width as i32 / 2, 110),
        "Center Aligned",
        Alignment::Center,
        None,
    );
    label_center.draw(&mut display, &styles)?;

    // Right-aligned label
    let mut label_right = Label::new(
        Point::new(width as i32 - 20, 140),
        "Right Aligned",
        Alignment::Right,
        None,
    );
    label_right.draw(&mut display, &styles)?;

    // === Test 2: Truncated labels with max width ===
    let mut truncated_label = Label::new(
        Point::new(20, 180),
        "This is a very long text that should be truncated with ellipsis at the end",
        Alignment::Left,
        Some(300),
    );
    truncated_label.draw(&mut display, &styles)?;

    // === Test 3: Battery icons at different levels ===
    let battery_levels = [100, 75, 50, 25, 10, 5];
    let mut y = 230;

    for level in battery_levels {
        // Battery icon
        let mut battery = BatteryIcon::new(Point::new(80, y));
        battery.set_state(false, level);
        battery.draw(&mut display, &styles)?;

        // Label showing level
        let mut level_label = Label::new(
            Point::new(100, y),
            format!("{}%", level),
            Alignment::Left,
            None,
        );
        level_label.draw(&mut display, &styles)?;

        y += 35;
    }

    // Charging battery
    let mut charging_battery = BatteryIcon::new(Point::new(250, 230));
    charging_battery.set_state(true, 60);
    charging_battery.draw(&mut display, &styles)?;

    let mut charging_label = Label::new(Point::new(270, 230), "Charging", Alignment::Left, None);
    charging_label.draw(&mut display, &styles)?;

    // === Test 4: Row of labels ===
    let row_labels = vec![
        Label::new(Point::new(0, 0), "Item 1", Alignment::Left, None),
        Label::new(Point::new(0, 0), "Item 2", Alignment::Left, None),
        Label::new(Point::new(0, 0), "Item 3", Alignment::Left, None),
    ];
    let mut row: Row<Label<&str>> = Row::new(Point::new(300, 300), row_labels, Alignment::Left, 20);
    row.draw(&mut display, &styles)?;

    // === Test 5: Focused/Blurred labels ===
    let mut normal_label = Label::new(Point::new(20, 420), "Normal", Alignment::Left, None);
    normal_label.draw(&mut display, &styles)?;

    let mut focused_label = Label::new(Point::new(120, 420), "Focused", Alignment::Left, None);
    focused_label.focus();
    focused_label.draw(&mut display, &styles)?;

    // Success message
    let mut success = Label::new(
        Point::new(20, 480),
        "Press ESC to exit",
        Alignment::Left,
        None,
    );
    success.draw(&mut display, &styles)?;

    display.flush()?;

    // Event loop with update for scrolling labels
    let mut last_update = std::time::Instant::now();
    loop {
        let event = platform.poll().await;

        // Update views
        let dt = last_update.elapsed();
        last_update = std::time::Instant::now();
        truncated_label.update(dt);

        if let KeyEvent::Pressed(Key::Menu) = event {
            break;
        }
    }

    Ok(())
}
