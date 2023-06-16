use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::{Circle, Primitive, PrimitiveStyle, RoundedRectangle};
use embedded_graphics::Drawable;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::command::Value;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::{Command, View};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Toggle {
    point: Point,
    value: bool,
    alignment: Alignment,
    dirty: bool,
}

impl Toggle {
    pub fn new(point: Point, value: bool, alignment: Alignment) -> Self {
        Self {
            point,
            value,
            alignment,
            dirty: true,
        }
    }

    pub fn value(&self) -> bool {
        self.value
    }

    pub fn set_value(&mut self, value: bool) {
        self.value = value;
        self.dirty = true;
    }
}

#[async_trait(?Send)]
impl View for Toggle {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        RoundedRectangle::with_equal_corners(
            Rect::new(
                self.point.x - (44 * (1 - self.alignment.sign()) / 2),
                self.point.y,
                44,
                28,
            )
            .into(),
            Size::new_equal(14),
        )
        .into_styled(PrimitiveStyle::with_fill(match self.value {
            true => styles.highlight_color,
            false => styles.disabled_color,
        }))
        .draw(display)?;

        Circle::new(
            Point::new(
                self.point.x - (44 * (1 - self.alignment.sign()) / 2)
                    + match self.value {
                        true => 20,
                        false => 4,
                    },
                self.point.y + 4,
            )
            .into(),
            20,
        )
        .into_styled(PrimitiveStyle::with_fill(styles.foreground_color))
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
        event: KeyEvent,
        _command: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        match event {
            KeyEvent::Pressed(Key::A) => {
                self.value = !self.value;
                self.dirty = true;
                bubble.push_back(Command::ValueChanged(0, Value::Bool(self.value)));
                return Ok(true);
            }
            _ => Ok(false),
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        Vec::new()
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        Vec::new()
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        Rect::new(
            self.point.x - (44 * (1 - self.alignment.sign())),
            self.point.y,
            44,
            24,
        )
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.dirty = true;
    }
}
