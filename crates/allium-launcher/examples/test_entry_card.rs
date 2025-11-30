//! Test EntryCard view example with CarouselList
//!
//! This example tests the EntryCard view in a horizontal CarouselList with:
//! - LazyImage display with boxart width
//! - Title at 0.8x font size
//! - Subtitle at 0.6x font size
//! - Vertical column layout
//! - Left/Right navigation
//! - Seamless overflow rendering
//!
//! Run with: cargo run --example test_entry_card --features simulator

use std::collections::VecDeque;
use std::path::PathBuf;

use allium_launcher::entry::lazy_image::LazyImage;
use allium_launcher::view::EntryCard;
use anyhow::Result;
use common::display::color::Color;
use common::display::{Display, fill_rect};
use common::geom::{self, Alignment, Point, Rect};
use common::locale::{Locale, LocaleSettings};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{CarouselList, Label, View};
use type_map::TypeMap;

#[tokio::main]
async fn main() -> Result<()> {
    let mut platform = DefaultPlatform::new()?;
    let mut display = platform.display()?;
    let styles = Stylesheet::load()?;

    let mut res = TypeMap::new();
    res.insert(Stylesheet::load()?);
    res.insert(Locale::new(&LocaleSettings::load()?.lang));
    res.insert(Into::<geom::Size>::into(display.size()));
    let res = Resources::new(res);

    // Clear background
    let (width, height) = (display.width(), display.height());
    fill_rect(
        &mut display.pixmap_mut(),
        Rect::new(0, 0, width, height),
        Color::new(40, 44, 52), // Dark blue-gray background
    );

    // Save initial state
    display.save()?;

    // Create sample EntryCard instances
    let boxart_width = styles.games.boxart_width;

    let cards = vec![
        EntryCard::new(
            Point::new(0, 50),
            LazyImage::Found(PathBuf::from("simulator/bg-640x480.png")),
            "Sample Game 1".to_string(),
            "Platform: NES".to_string(),
        ),
        EntryCard::new(
            Point::new(0, 50),
            LazyImage::Found(PathBuf::from("simulator/bg-640x480.png")),
            "Sample Game 2".to_string(),
            "Platform: SNES".to_string(),
        ),
        EntryCard::new(
            Point::new(0, 50),
            LazyImage::Found(PathBuf::from("simulator/bg-752x560.png")),
            "Long Game Title".to_string(),
            "Genre: RPG".to_string(),
        ),
        EntryCard::new(
            Point::new(0, 50),
            LazyImage::NotFound,
            "Another Game".to_string(),
            "Platform: GBA".to_string(),
        ),
        EntryCard::new(
            Point::new(0, 50),
            LazyImage::NotFound,
            "Final Game".to_string(),
            "Platform: PS1".to_string(),
        ),
    ];

    // Create carousel list with entry cards
    let carousel_rect = Rect::new(0, 50, width, 300);
    let mut carousel = CarouselList::new(
        carousel_rect,
        res.clone(),
        cards,
        Alignment::Left,
        boxart_width,
        styles.ui.margin_x as u32,
    );

    carousel.draw(&mut display, &styles)?;

    // Instructions
    let mut instruction_text = Label::new(
        Point::new(20, height as i32 - 30),
        "Left/Right: Navigate | L/R: Skip 5 | ESC: Exit",
        Alignment::Left,
        None,
    );
    instruction_text.draw(&mut display, &styles)?;

    display.flush()?;

    // Event loop
    loop {
        let event = platform.poll().await;

        match event {
            KeyEvent::Pressed(Key::Menu) => break,
            _ => {
                if carousel
                    .handle_key_event(event, tokio::sync::mpsc::channel(1).0, &mut VecDeque::new())
                    .await?
                {
                    carousel.draw(&mut display, &styles)?;
                    display.flush()?;
                }
            }
        }
    }

    Ok(())
}
