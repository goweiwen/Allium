use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use crate::{
    command::Command,
    geom::Point,
    platform::{DefaultPlatform, KeyEvent, Platform},
    stylesheet::Stylesheet,
    view::View,
};

pub struct NullView;

#[async_trait(?Send)]
impl View for NullView {
    fn draw(
        &mut self,
        _display: &mut <DefaultPlatform as Platform>::Display,
        _styles: &Stylesheet,
    ) -> Result<bool> {
        Ok(false)
    }

    fn should_draw(&self) -> bool {
        false
    }

    fn set_should_draw(&mut self) {}

    async fn handle_key_event(
        &mut self,
        _event: KeyEvent,
        _commands: Sender<Command>,
        _bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        Ok(false)
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![]
    }

    fn set_position(&mut self, _point: Point) {}
}
