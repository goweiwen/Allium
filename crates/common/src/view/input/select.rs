use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;

use log::trace;
use tokio::sync::mpsc::Sender;

use crate::command::Value;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::{Command, Label, View};

#[derive(Debug, Clone)]
pub struct Select {
    point: Point,
    value: usize,
    values: Vec<String>,
    label: Label<String>,
    edit_state: Option<usize>,
}

impl Select {
    pub fn new(point: Point, value: usize, values: Vec<String>, alignment: Alignment) -> Self {
        let label = Label::new(
            Point::new(point.x, point.y),
            values
                .get(value)
                .map_or_else(|| "".to_owned(), String::to_owned),
            alignment,
            None,
        );

        Self {
            point,
            value,
            values,
            label,
            edit_state: None,
        }
    }

    pub fn value(&self) -> &str {
        &self.values[self.value]
    }

    pub fn set_value(&mut self, selected: usize) {
        self.value = selected;
        self.label.set_text(self.values[self.value].clone());
    }
}

#[async_trait(?Send)]
impl View for Select {
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
        trace!("selection: {:?}", self.values[self.value]);
        if let Some(value) = &mut self.edit_state {
            match event {
                KeyEvent::Pressed(Key::Up | Key::Left)
                | KeyEvent::Autorepeat(Key::Up | Key::Left) => {
                    *value = (*value as isize - 1).rem_euclid(self.values.len() as isize) as usize;
                    self.label.set_text(self.values[*value].clone());
                    return Ok(true);
                }
                KeyEvent::Pressed(Key::Down | Key::Right)
                | KeyEvent::Autorepeat(Key::Down | Key::Right) => {
                    *value = (*value + 1).rem_euclid(self.values.len());
                    self.label.set_text(self.values[*value].clone());
                    return Ok(true);
                }
                KeyEvent::Pressed(Key::A) => {
                    self.value = *value;
                    self.edit_state = None;
                    bubble.push_back(Command::ValueChanged(0, Value::Int(self.value as i32)));
                    bubble.push_back(Command::Unfocus);
                    Ok(true)
                }
                KeyEvent::Pressed(Key::B) => {
                    self.edit_state = None;
                    self.label.set_text(self.values[self.value].clone());
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
