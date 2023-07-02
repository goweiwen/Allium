use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::command::Value;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::{Command, Label, View};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Number {
    point: Point,
    value: i32,
    min: i32,
    max: i32,
    alignment: Alignment,
    label: Label<String>,
    #[serde(skip)]
    edit_state: Option<i32>,
}

impl Number {
    pub fn new(point: Point, value: i32, min: i32, max: i32, alignment: Alignment) -> Self {
        let label = Label::new(
            Point::new(point.x, point.y),
            format!("{}", value),
            alignment,
            None,
        );

        Self {
            point,
            value,
            min,
            max,
            alignment,
            label,
            edit_state: None,
        }
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    pub fn set_value(&mut self, value: i32) {
        self.value = value;
        self.label.set_text(format!("{}", self.value));
    }
}

#[async_trait(?Send)]
impl View for Number {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        self.label.draw(display, styles)
    }

    fn should_draw(&self) -> bool {
        self.label.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.label.set_should_draw()
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        _command: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if let Some(value) = &mut self.edit_state {
            match event {
                KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                    *value = (*value + 1).clamp(self.min, self.max);
                    self.label.set_text(format!("{}", *value));
                    return Ok(true);
                }
                KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                    *value = (*value - 1).clamp(self.min, self.max);
                    self.label.set_text(format!("{}", *value));
                    return Ok(true);
                }
                KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                    *value = (*value - 5).clamp(self.min, self.max);
                    self.label.set_text(format!("{}", *value));
                    return Ok(true);
                }
                KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                    *value = (*value + 5).clamp(self.min, self.max);
                    self.label.set_text(format!("{}", *value));
                    return Ok(true);
                }
                KeyEvent::Pressed(Key::A) => {
                    self.value = *value;
                    self.edit_state = None;
                    bubble.push_back(Command::ValueChanged(0, Value::Int(self.value)));
                    bubble.push_back(Command::Unfocus);
                    Ok(true)
                }
                KeyEvent::Pressed(Key::B) => {
                    self.edit_state = None;
                    self.label.set_text(format!("{}", self.value));
                    bubble.push_back(Command::Unfocus);
                    Ok(true)
                }
                _ => Ok(false),
            }
        } else {
            self.edit_state = Some(self.value);
            bubble.push_back(Command::TrapFocus);
            Ok(true)
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.label]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.label]
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        self.label.bounding_box(styles)
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.label.set_position(point)
    }
}
