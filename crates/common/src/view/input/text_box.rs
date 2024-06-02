use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::resources::Resources;
use crate::stylesheet::Stylesheet;
use crate::view::input::keyboard::Keyboard;
use crate::view::{Command, Label, View};

#[derive(Debug, Clone)]
pub struct TextBox {
    point: Point,
    res: Resources,
    value: String,
    is_password: bool,
    label: Label<String>,
    keyboard: Option<Keyboard>,
}

impl TextBox {
    pub fn new(
        point: Point,
        res: Resources,
        value: String,
        alignment: Alignment,
        is_password: bool,
    ) -> Self {
        let label = Label::new(
            Point::new(point.x, point.y),
            masked_value(&value, is_password),
            alignment,
            None,
        );

        Self {
            point,
            res,
            value,
            is_password,
            label,
            keyboard: None,
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
        let mut drawn = false;

        drawn |= self.label.should_draw() && self.label.draw(display, styles)?;

        if let Some(keyboard) = self.keyboard.as_mut() {
            drawn |= keyboard.should_draw() && keyboard.draw(display, styles)?;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.label.should_draw() || self.keyboard.as_ref().map_or(false, |k| k.should_draw())
    }

    fn set_should_draw(&mut self) {
        self.label.set_should_draw();
        if let Some(keyboard) = self.keyboard.as_mut() {
            keyboard.set_should_draw();
        }
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        command: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if let Some(keyboard) = self.keyboard.as_mut() {
            if keyboard.handle_key_event(event, command, bubble).await? {
                bubble.retain_mut(|cmd| match cmd {
                    Command::CloseView => {
                        self.keyboard = None;
                        *cmd = Command::Unfocus;
                        true
                    }
                    Command::ValueChanged(_, value) => {
                        self.value = value.clone().as_string().unwrap();
                        self.label
                            .set_text(masked_value(&self.value, self.is_password));
                        true
                    }
                    _ => true,
                });
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            self.keyboard = Some(Keyboard::new(
                self.res.clone(),
                self.value.clone(),
                self.is_password,
            ));
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
