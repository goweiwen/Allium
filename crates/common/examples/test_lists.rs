//! Test ScrollList and SettingsList example for the tiny-skia migration
//!
//! This example tests:
//! - ScrollList with scrollable items
//! - SettingsList with labels and toggle controls
//! - Keyboard navigation (Up/Down arrows, L/R for page navigation)
//!
//! Run with: cargo run --example test_lists --features simulator

use anyhow::Result;
use common::display::color::Color;
use common::display::{Display, fill_rect};
use common::geom::{Alignment, Point, Rect};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{Label, ScrollList, SettingsList, Toggle, View};
use type_map::TypeMap;

#[tokio::main]
async fn main() -> Result<()> {
    let mut platform = DefaultPlatform::new()?;
    let mut display = platform.display()?;
    let styles = Stylesheet::load()?;

    // Create Resources with Stylesheet
    let mut type_map = TypeMap::new();
    type_map.insert(styles.clone());
    let res = Resources::new(type_map);

    // Clear background
    let (width, height) = (display.width(), display.height());
    fill_rect(
        &mut display.pixmap_mut(),
        Rect::new(0, 0, width, height),
        Color::new(40, 44, 52),
    );

    // Save initial state for views that need to restore background
    display.save()?;

    // === ScrollList on the left side ===
    let scroll_items: Vec<String> = (1..=20).map(|i| format!("Item {}", i)).collect();

    let mut scroll_list = ScrollList::new(
        res.clone(),
        Rect::new(20, 60, 300, 400),
        scroll_items,
        Alignment::Left,
        32,
    );

    // === SettingsList on the right side ===
    let settings_labels = vec![
        "Enable Feature".to_string(),
        "Dark Mode".to_string(),
        "Notifications".to_string(),
        "Auto-save".to_string(),
        "Debug Mode".to_string(),
    ];

    let settings_controls: Vec<Box<dyn View>> = vec![
        Box::new(Toggle::new(Point::zero(), true, Alignment::Right)),
        Box::new(Toggle::new(Point::zero(), false, Alignment::Right)),
        Box::new(Toggle::new(Point::zero(), true, Alignment::Right)),
        Box::new(Toggle::new(Point::zero(), false, Alignment::Right)),
        Box::new(Toggle::new(Point::zero(), false, Alignment::Right)),
    ];

    let mut settings_list = SettingsList::new(
        res.clone(),
        Rect::new(380, 60, 350, 300),
        settings_labels,
        settings_controls,
        32,
    );

    // Draw title
    let mut title = Label::new(
        Point::new(width as i32 / 2, 20),
        "List Tests",
        Alignment::Center,
        None,
    );
    title.font_size(1.5);
    title.draw(&mut display, &styles)?;

    // Draw section labels
    let mut scroll_label = Label::new(Point::new(170, 45), "ScrollList", Alignment::Center, None);
    scroll_label.draw(&mut display, &styles)?;

    let mut settings_label =
        Label::new(Point::new(555, 45), "SettingsList", Alignment::Center, None);
    settings_label.draw(&mut display, &styles)?;

    // Draw initial state
    scroll_list.draw(&mut display, &styles)?;
    settings_list.draw(&mut display, &styles)?;

    // Instructions
    let mut instructions = Label::new(
        Point::new(20, height as i32 - 30),
        "Up/Down: Navigate | L/R: Page | ESC: Exit",
        Alignment::Left,
        None,
    );
    instructions.draw(&mut display, &styles)?;

    display.flush()?;

    // Event loop
    let (tx, _rx) = tokio::sync::mpsc::channel(16);
    let mut bubble = std::collections::VecDeque::new();
    let mut active_list = 0; // 0 = scroll_list, 1 = settings_list

    loop {
        let event = platform.poll().await;

        match event {
            KeyEvent::Pressed(Key::Menu) => break,
            KeyEvent::Pressed(Key::Left) | KeyEvent::Pressed(Key::Right) => {
                active_list = 1 - active_list;
            }
            _ => {
                if active_list == 0 {
                    scroll_list
                        .handle_key_event(event, tx.clone(), &mut bubble)
                        .await?;
                } else {
                    settings_list
                        .handle_key_event(event, tx.clone(), &mut bubble)
                        .await?;
                }
            }
        }

        // Redraw if needed
        if scroll_list.should_draw() || settings_list.should_draw() {
            scroll_list.draw(&mut display, &styles)?;
            settings_list.draw(&mut display, &styles)?;
            display.flush()?;
        }
    }

    Ok(())
}
