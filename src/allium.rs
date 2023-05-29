use std::time::Duration;

use anyhow::Result;
use embedded_font::{FontTextStyle, FontTextStyleBuilder};
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{
        Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, Triangle,
    },
    text::{Alignment, Text},
};
use rusttype::Font;
use tracing::debug;

use crate::platform::{Key, KeyEvent, Platform};

pub struct Allium {
    platform: Platform,
    dirty: bool,
    styles: Stylesheet,
}

pub struct Stylesheet {
    fg_color: Rgb888,
    bg_color: Rgb888,
    primary: Rgb888,
    ui_font: Font<'static>,
}

impl Default for Stylesheet {
    fn default() -> Self {
        let fg_color = Rgb888::new(255, 255, 255);
        let bg_color = Rgb888::new(0, 0, 0);
        let primary = Rgb888::new(237, 90, 134);
        let ui_font =
            Font::try_from_bytes(include_bytes!("../assets/font/Sniglet/Sniglet-Regular.ttf"))
                .unwrap();

        Self {
            ui_font,
            fg_color,
            bg_color,
            primary,
        }
    }
}

pub const BATTERY_UPDATE_INTERVAL: Duration = Duration::from_secs(5);

impl Allium {
    pub fn new() -> Result<Allium> {
        Ok(Allium {
            platform: Platform::new()?,
            dirty: true,
            styles: Default::default(),
        })
    }

    pub async fn init(&mut self) -> Result<()> {
        Platform::init().await
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        self.platform.update_battery()?;
        let mut last_updated_battery = std::time::Instant::now();

        loop {
            let now = std::time::Instant::now();

            debug!(
                "last updated: {}s",
                now.duration_since(last_updated_battery).as_secs()
            );
            if now.duration_since(last_updated_battery) > BATTERY_UPDATE_INTERVAL {
                self.platform.update_battery()?;
                last_updated_battery = now;
                self.dirty = true;
            }

            if self.dirty {
                self.draw()?;
                // self.dirty = false;
            }

            match self.platform.poll().await? {
                Some(KeyEvent::Pressed(key)) => {
                    if key == Key::Power {
                        panic!("exiting");
                    }
                    println!("down {:?}", key);
                }
                Some(KeyEvent::Released(key)) => println!("up {:?}", key),
                None => (),
            }
        }
    }

    fn draw(&mut self) -> Result<()> {
        let yoffset = 14;

        let battery_percentage = self.platform.battery_percentage();

        let (width, height) = self.platform.display_size();
        let display = self.platform.display()?;

        // Draw a 3px wide outline around the display.
        display
            .bounding_box()
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_color(self.styles.primary)
                    .stroke_width(3)
                    .stroke_alignment(StrokeAlignment::Inside)
                    .build(),
            )
            .draw(display)?;

        // Draw a triangle.
        Triangle::new(
            Point::new(16, 16 + yoffset),
            Point::new(16 + 16, 16 + yoffset),
            Point::new(16 + 8, yoffset),
        )
        .into_styled(PrimitiveStyle::with_stroke(self.styles.primary, 1))
        .draw(display)?;

        // Draw a filled square
        Rectangle::new(Point::new(52, yoffset), Size::new(16, 16))
            .into_styled(PrimitiveStyle::with_fill(self.styles.primary))
            .draw(display)?;

        // Draw a circle with a 3px wide stroke.
        Circle::new(Point::new(88, yoffset), 17)
            .into_styled(PrimitiveStyle::with_stroke(self.styles.primary, 3))
            .draw(display)?;

        // Draw battery percentage
        let text = format!("{}%", battery_percentage);

        let character_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(24)
            .text_color(self.styles.fg_color)
            .build();

        Text::with_alignment(
            &text,
            Point { x: width - 8, y: 8 },
            character_style,
            Alignment::Right,
        )
        .draw(display)?;

        // Draw centered text.
        let text = "hello world, from Allium!";

        let character_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(32)
            .text_color(self.styles.fg_color)
            .build();

        Text::with_alignment(
            text,
            display.bounding_box().center(),
            character_style,
            Alignment::Center,
        )
        .draw(display)?;

        self.platform.flush()?;

        Ok(())
    }
}
