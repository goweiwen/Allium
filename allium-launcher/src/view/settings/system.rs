use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::{ALLIUM_VERSION, BUTTON_DIAMETER, SELECTION_HEIGHT};
use common::geom::{Alignment, Point, Rect};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, Label, Row, SettingsList, View};
use tokio::sync::mpsc::Sender;

pub struct System {
    rect: Rect,
    list: SettingsList,
    button_hints: Row<ButtonHint<String>>,
}

impl System {
    pub fn new(rect: Rect) -> Self {
        let firmware = DefaultPlatform::firmware();

        let list = SettingsList::new(
            Rect::new(rect.x, rect.y + 8, rect.w - 12, rect.h - 8 - 46),
            vec![
                "Version".to_string(),
                "Firmware".to_string(),
                "Device Model".to_string(),
            ],
            vec![
                Box::new(Label::new(
                    Point::zero(),
                    format!("Allium v{}", ALLIUM_VERSION),
                    Alignment::Right,
                    None,
                )),
                Box::new(Label::new(Point::zero(), firmware, Alignment::Right, None)),
                Box::new(Label::new(
                    Point::zero(),
                    DefaultPlatform::device_model(),
                    Alignment::Right,
                    None,
                )),
            ],
            SELECTION_HEIGHT,
        );

        let button_hints = Row::new(
            Point::new(
                rect.x + rect.w as i32 - 12,
                rect.y + rect.h as i32 - BUTTON_DIAMETER as i32 - 8,
            ),
            vec![ButtonHint::new(
                Point::zero(),
                Key::B,
                "Back".to_owned(),
                Alignment::Right,
            )],
            Alignment::Right,
            12,
        );

        Self {
            rect,
            list,
            button_hints,
        }
    }
}

#[async_trait(?Send)]
impl View for System {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let mut drawn = false;

        if self.list.should_draw() && self.list.draw(display, styles)? {
            drawn = true;
        }

        if self.button_hints.should_draw() && self.button_hints.draw(display, styles)? {
            drawn = true;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.list.should_draw() || self.button_hints.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.list.set_should_draw();
        self.button_hints.set_should_draw();
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
        vec![&self.list, &self.button_hints]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.list, &mut self.button_hints]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}
