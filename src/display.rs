use std::error::Error;

use anyhow::Result;
use embedded_font::{FontTextStyle, FontTextStyleBuilder};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};

use crate::allium::Stylesheet;
use crate::constants::BUTTON_DIAMETER;
use crate::platform::Key;

pub trait Display<E: Error + Send + Sync + 'static>:
    OriginDimensions + DrawTarget<Color = Rgb888, Error = E> + Sized
{
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    fn draw_text(
        &mut self,
        point: Point,
        text: &str,
        style: FontTextStyle<Rgb888>,
        alignment: Alignment,
    ) -> Result<Rectangle> {
        let text = Text::with_alignment(text, point, style, alignment);
        text.draw(self)?;
        Ok(text.bounding_box())
    }

    /// Renders string truncated to fit within width, adding ellipsis if necessary
    fn draw_text_ellipsis(
        &mut self,
        point: Point,
        text: &str,
        style: FontTextStyle<Rgb888>,
        alignment: Alignment,
        width: u32,
    ) -> Result<Rectangle> {
        let mut text = Text::with_alignment(text, point, style.clone(), alignment);
        let ellipsis_width = Text::with_alignment("...", point, style, alignment)
            .bounding_box()
            .size
            .width;

        // TODO: binary search?
        let mut ellipsis = false;
        let mut text_width = text.bounding_box().size.width;
        while text_width + ellipsis_width > width {
            let mut chars = text.text.chars();
            chars.next_back();
            ellipsis = true;
            text.text = chars.as_str();
            text_width = text.bounding_box().size.width;
        }
        let ellipsis_text = format!("{}...", text.text);
        if ellipsis {
            text.text = &ellipsis_text;
        }
        text.draw(self)?;
        Ok(text.bounding_box())
    }

    fn draw_button_hint(
        &mut self,
        point: Point,
        button: Key,
        style: FontTextStyle<Rgb888>,
        text: &str,
        styles: &Stylesheet,
    ) -> Result<Rectangle> {
        let x = point.x
            - self
                .draw_text(
                    Point::new(point.x, point.y + 4),
                    text,
                    style,
                    Alignment::Right,
                )?
                .size
                .width as i32
            - 4;
        self.draw_button(
            Point::new(x - BUTTON_DIAMETER as i32, point.y),
            button,
            styles,
        )?;
        Ok(Rectangle::new(
            Point::new(x - BUTTON_DIAMETER as i32, point.y),
            Size::new(
                (point.x - (x - BUTTON_DIAMETER as i32)) as u32,
                BUTTON_DIAMETER,
            ),
        ))
    }

    fn draw_button(&mut self, point: Point, button: Key, styles: &Stylesheet) -> Result<()> {
        let (color, text) = match button {
            Key::A => (styles.button_a_color, "A"),
            Key::B => (styles.button_b_color, "B"),
            Key::X => (styles.button_x_color, "X"),
            Key::Y => (styles.button_y_color, "Y"),
            _ => (styles.primary, "?"),
        };

        Circle::new(point, BUTTON_DIAMETER)
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(self)?;

        let button_style = FontTextStyleBuilder::new(styles.ui_font.clone())
            .font_size(styles.ui_font_size)
            .text_color(styles.fg_color)
            .background_color(color)
            .build();
        Text::with_text_style(
            text,
            Point::new(point.x + (BUTTON_DIAMETER / 2) as i32, point.y + 4),
            button_style,
            TextStyleBuilder::new()
                .alignment(Alignment::Center)
                .baseline(Baseline::Middle)
                .build(),
        )
        .draw(self)?;

        Ok(())
    }
}
