use anyhow::Result;
use embedded_font::FontTextStyleBuilder;
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{
        Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, Triangle,
    },
    text::{Alignment, Text},
};
use rusttype::Font;

use crate::platform::{KeyEvent, Platform};

pub struct Allium {
    platform: Platform,
    dirty: bool,
}

impl Allium {
    pub fn new() -> Result<Allium> {
        Ok(Allium {
            platform: Platform::new()?,
            dirty: true,
        })
    }

    pub async fn init(&mut self) -> Result<()> {
        Platform::init().await
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        loop {
            if self.dirty {
                self.draw()?;
                // self.dirty = false;
            }

            match self.platform.poll().await? {
                Some(KeyEvent::Pressed(key)) => println!("down {:?}", key),
                Some(KeyEvent::Released(key)) => println!("up {:?}", key),
                None => (),
            }
        }
    }

    fn draw(&mut self) -> Result<()> {
        // Create styles used by the drawing operations.
        let thin_stroke = PrimitiveStyle::with_stroke(Rgb888::new(255, 255, 255), 1);
        let thick_stroke = PrimitiveStyle::with_stroke(Rgb888::new(255, 255, 255), 3);
        let border_stroke = PrimitiveStyleBuilder::new()
            .stroke_color(Rgb888::new(255, 255, 255))
            .stroke_width(3)
            .stroke_alignment(StrokeAlignment::Inside)
            .build();
        let fill = PrimitiveStyle::with_fill(Rgb888::new(255, 255, 255));
        let font =
            Font::try_from_bytes(include_bytes!("../assets/font/Sniglet/Sniglet-Regular.ttf"))
                .unwrap();
        let character_style = FontTextStyleBuilder::new(font)
            .font_size(32)
            .text_color(Rgb888::new(255, 255, 255))
            .build();
        // let character_style = MonoTextStyle::new(&FONT_6X10, Rgb888::new(255, 255, 255));

        let yoffset = 14;

        let display = self.platform.display()?;

        // Draw a 3px wide outline around the display.
        display
            .bounding_box()
            .into_styled(border_stroke)
            .draw(display)?;

        // Draw a triangle.
        Triangle::new(
            Point::new(16, 16 + yoffset),
            Point::new(16 + 16, 16 + yoffset),
            Point::new(16 + 8, yoffset),
        )
        .into_styled(thin_stroke)
        .draw(display)?;

        // Draw a filled square
        Rectangle::new(Point::new(52, yoffset), Size::new(16, 16))
            .into_styled(fill)
            .draw(display)?;

        // Draw a circle with a 3px wide stroke.
        Circle::new(Point::new(88, yoffset), 17)
            .into_styled(thick_stroke)
            .draw(display)?;

        // Draw centered text.
        let text = "hello world, from Allium!";
        Text::with_alignment(
            text,
            display.bounding_box().center() + Point::new(0, 15),
            character_style,
            Alignment::Center,
        )
        .draw(display)?;

        self.platform.flush()?;

        Ok(())
    }
}
