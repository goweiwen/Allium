use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::command::Value;
use crate::display::Display;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::{Command, View};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        let h = styles.ui.ui_font.size;
        let w = h * 3 / 2;
        let margin = h as i32 / 6;

        let mut pixmap = display.pixmap_mut();

        // Draw toggle background
        let bg_color = match self.value {
            true => styles.ui.highlight_color,
            false => styles.ui.disabled_color,
        };
        let bg_rect = Rect::new(
            self.point.x - (w as i32 * (1 - self.alignment.sign()) / 2),
            self.point.y,
            w,
            h,
        );
        crate::display::fill_rounded_rect(&mut pixmap, bg_rect, h, bg_color);

        // Draw toggle circle
        let circle_x = self.point.x - (w as i32 * (1 - self.alignment.sign()) / 2)
            + match self.value {
                true => w as i32 - h as i32 + margin,
                false => margin,
            };
        let circle_y = self.point.y + margin;
        let circle_diameter = h - margin as u32 - margin as u32;
        let circle_center = Point::new(
            circle_x + circle_diameter as i32 / 2,
            circle_y + circle_diameter as i32 / 2,
        );

        crate::display::fill_circle(
            &mut pixmap,
            circle_center,
            circle_diameter / 2,
            styles.ui.highlight_text_color,
        );

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

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        let h = styles.ui.ui_font.size;
        let w = h * 3 / 2;
        Rect::new(
            self.point.x - (w as i32 * (1 - self.alignment.sign()) / 2),
            self.point.y,
            w,
            h,
        )
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.dirty = true;
    }
}
