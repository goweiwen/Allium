use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::geom::{Point, Rect};
use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use common::view::View;
use tokio::sync::mpsc::Sender;

pub struct Display {
    rect: Rect,
}

impl Display {
    pub fn new(rect: Rect) -> Self {
        Self { rect }
    }
}

#[async_trait(?Send)]
impl View for Display {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        todo!()
    }

    fn should_draw(&self) -> bool {
        todo!()
    }

    fn set_should_draw(&mut self) {
        todo!()
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        todo!()
    }

    fn children(&self) -> Vec<&dyn View> {
        todo!()
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        todo!()
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, point: Point) {
        unimplemented!()
    }
}
