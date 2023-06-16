use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::command::Value;
use crate::geom::{Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::{Command, View};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Button<V>
where
    V: View,
{
    view: V,
}

impl<V> Button<V>
where
    V: View,
{
    pub fn new(view: V) -> Self {
        Self { view }
    }
}

#[async_trait(?Send)]
impl<V> View for Button<V>
where
    V: View,
{
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        self.view.draw(display, styles)
    }

    fn should_draw(&self) -> bool {
        self.view.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.view.set_should_draw()
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        _command: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        match event {
            KeyEvent::Pressed(Key::A) => {
                bubble.push_back(Command::ValueChanged(0, Value::Bool(true)));
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.view]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.view]
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        self.view.bounding_box(styles)
    }

    fn set_position(&mut self, point: Point) {
        self.view.set_position(point)
    }
}
