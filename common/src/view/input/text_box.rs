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
pub struct TextBox {
    point: Point,
    value: String,
    alignment: Alignment,
    is_password: bool,
    label: Label<String>,
    #[serde(skip)]
    edit_state: Option<String>,
}

impl TextBox {
    pub fn new(point: Point, value: String, alignment: Alignment, is_password: bool) -> Self {
        let label = Label::new(
            Point::new(point.x, point.y),
            masked_value(&value, is_password),
            alignment,
            None,
        );

        Self {
            point,
            value,
            alignment,
            is_password,
            label,
            edit_state: None,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn set_value(&mut self, value: String) {
        self.value = value;
        self.label
            .set_text(masked_value(&self.value, self.is_password));
    }
}

#[async_trait(?Send)]
impl View for TextBox {
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
        if self.edit_state.is_some() {
            match event {
                KeyEvent::Pressed(Key::A) => {
                    self.value = self.edit_state.take().unwrap();
                    bubble.push_back(Command::ValueChanged(0, Value::String(self.value.clone())));
                    bubble.push_back(Command::Unfocus);
                    Ok(true)
                }
                KeyEvent::Pressed(Key::B) => {
                    self.edit_state = None;
                    self.label.set_text(format!("{}%", self.value));
                    bubble.push_back(Command::Unfocus);
                    Ok(true)
                }
                _ => Ok(false),
            }
        } else {
            self.edit_state = Some(self.value.clone());
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

fn masked_value(value: &str, is_password: bool) -> String {
    if is_password {
        "*".repeat(value.len())
    } else {
        value.to_owned()
    }
}
