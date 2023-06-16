use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::{ALLIUM_VERSION, SELECTION_HEIGHT};
use common::geom::{Alignment, Point, Rect};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use common::view::{Label, SettingsList, View};
use tokio::sync::mpsc::Sender;

pub struct System {
    rect: Rect,
    list: SettingsList,
}

impl System {
    pub fn new(rect: Rect) -> Self {
        let list = SettingsList::new(
            rect,
            vec!["Version".to_string(), "Device Model".to_string()],
            vec![
                Box::new(Label::new(
                    Point::zero(),
                    format!("Allium v{}", ALLIUM_VERSION),
                    Alignment::Right,
                    None,
                )),
                Box::new(Label::new(
                    Point::zero(),
                    DefaultPlatform::device_model(),
                    Alignment::Right,
                    None,
                )),
            ],
            SELECTION_HEIGHT,
        );

        Self { rect, list }
    }
}

#[async_trait(?Send)]
impl View for System {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        self.list.draw(display, styles)
    }

    fn should_draw(&self) -> bool {
        self.list.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.list.set_should_draw()
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        match event {
            KeyEvent::Pressed(Key::B) => {
                bubble.push_back(Command::CloseView);
                Ok(true)
            }
            _ => {
                self.list.handle_key_event(event, commands, bubble).await?;
                Ok(true)
            }
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.list]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.list]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}
