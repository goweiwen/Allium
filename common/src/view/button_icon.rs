use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::{
    Circle, CornerRadiiBuilder, Primitive, PrimitiveStyle, Rectangle, RoundedRectangle,
};
use embedded_graphics::text::{Text, TextStyleBuilder};
use embedded_graphics::Drawable;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::display::font::FontTextStyleBuilder;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::{Command, View};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ButtonIcon {
    point: Point,
    button: Key,
    alignment: Alignment,
    dirty: bool,
}

impl ButtonIcon {
    pub fn new(point: Point, button: Key, alignment: Alignment) -> Self {
        Self {
            point,
            button,
            alignment,
            dirty: true,
        }
    }

    pub fn diameter(styles: &Stylesheet) -> u32 {
        styles.ui_font.size
    }
}

#[async_trait(?Send)]
impl View for ButtonIcon {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let (color, text) = match self.button {
            Key::A => (styles.button_a_color, "A"),
            Key::B => (styles.button_b_color, "B"),
            Key::X => (styles.button_x_color, "X"),
            Key::Y => (styles.button_y_color, "Y"),
            Key::Up => (styles.disabled_color, "▲"),
            Key::Down => (styles.disabled_color, "▼"),
            Key::Left => (styles.disabled_color, "◀"),
            Key::Right => (styles.disabled_color, "▶"),
            Key::Start => (styles.disabled_color, "+"),
            Key::Select => (styles.disabled_color, "-"),
            Key::L => (styles.disabled_color, "L"),
            Key::R => (styles.disabled_color, "R"),
            Key::Menu => (styles.disabled_color, "M"),
            Key::L2 => (styles.disabled_color, "L2"),
            Key::R2 => (styles.disabled_color, "R2"),
            Key::Power => (styles.disabled_color, "Power"),
            Key::VolDown => (styles.disabled_color, "Vol-"),
            Key::VolUp => (styles.disabled_color, "Vol+"),
            Key::Unknown => unimplemented!("unknown button"),
        };

        let diameter = Self::diameter(styles);

        let point = match self.alignment {
            Alignment::Left => self.point.into(),
            Alignment::Center => embedded_graphics::prelude::Point::new(
                self.point.x - (diameter / 2) as i32,
                self.point.y,
            ),
            Alignment::Right => {
                embedded_graphics::prelude::Point::new(self.point.x - diameter as i32, self.point.y)
            }
        };

        match self.button {
            Key::A | Key::B | Key::X | Key::Y | Key::Menu => {
                Circle::new(point, diameter)
                    .into_styled(PrimitiveStyle::with_fill(color))
                    .draw(display)?;
            }
            Key::Start | Key::Select => {
                RoundedRectangle::with_equal_corners(
                    Rectangle::new(
                        Point::new(point.x, point.y + diameter as i32 / 5 + 1).into(),
                        Size::new(diameter, diameter * 3 / 5),
                    ),
                    Size::new_equal(8),
                )
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(display)?;
            }
            Key::L | Key::L2 => {
                RoundedRectangle::new(
                    Rectangle::new(
                        Point::new(point.x, point.y + diameter as i32 / 8).into(),
                        Size::new(diameter, diameter * 3 / 4),
                    ),
                    CornerRadiiBuilder::new()
                        .all(Size::new_equal(8))
                        .top_left(Size::new_equal(16))
                        .build(),
                )
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(display)?;
            }
            Key::R | Key::R2 => {
                RoundedRectangle::new(
                    Rectangle::new(
                        Point::new(point.x, point.y + diameter as i32 / 8).into(),
                        Size::new(diameter, diameter * 3 / 4),
                    ),
                    CornerRadiiBuilder::new()
                        .all(Size::new_equal(8))
                        .top_right(Size::new_equal(16))
                        .build(),
                )
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(display)?;
            }
            _ => {}
        }

        let text_style = FontTextStyleBuilder::new(styles.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size(diameter * 3 / 4)
            .text_color(styles.foreground_color)
            .background_color(color)
            .build();
        Text::with_text_style(
            text,
            embedded_graphics::prelude::Point::new(
                point.x + diameter as i32 / 2,
                point.y + diameter as i32 / 8,
            ),
            text_style,
            TextStyleBuilder::new()
                .alignment(Alignment::Center.into())
                .build(),
        )
        .draw(display)?;

        self.dirty = false;

        Ok(true)
    }

    fn should_draw(&self) -> bool {
        self.dirty
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
    }

    async fn handle_key_event(
        &mut self,
        _event: KeyEvent,
        _command: Sender<Command>,
        _bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        Ok(false)
    }

    fn children(&self) -> Vec<&dyn View> {
        Vec::new()
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        Vec::new()
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        let diameter = Self::diameter(styles);
        let x = match self.alignment {
            Alignment::Left => self.point.x,
            Alignment::Center => self.point.x - (diameter / 2) as i32,

            Alignment::Right => self.point.x - diameter as i32,
        };

        Rect::new(x, self.point.y - 1, diameter, diameter)
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.dirty = true;
    }
}
