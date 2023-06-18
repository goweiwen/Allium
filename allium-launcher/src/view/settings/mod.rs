mod display;
mod system;
mod theme;
mod wifi;

use self::display::Display;
use self::system::System;
use self::theme::Theme;
use self::wifi::Wifi;

use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::BUTTON_DIAMETER;
use common::display::Display as DisplayTrait;
use common::geom::{Alignment, Point, Rect};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, Label, List, Row, View};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    rect: Rect,
    list: List<Label<String>>,
    #[serde(skip)]
    child: Option<Box<dyn View>>,
    button_hints: Row<ButtonHint<String>>,
    #[serde(skip)]
    dirty: bool,
}

impl Settings {
    pub fn new(rect: Rect) -> Result<Self> {
        let Rect { x, y, w, h } = rect;

        let list = List::new(
            Rect::new(x + 12, y + 8, 110 + 12 + 12 - 24, h - 8 - 48),
            vec![
                Label::new(
                    Point::zero(),
                    "Wi-Fi".to_owned(),
                    Alignment::Left,
                    Some(110),
                ),
                Label::new(
                    Point::zero(),
                    "Display".to_owned(),
                    Alignment::Left,
                    Some(110),
                ),
                Label::new(
                    Point::zero(),
                    "Theme".to_owned(),
                    Alignment::Left,
                    Some(110),
                ),
                Label::new(
                    Point::zero(),
                    "System".to_owned(),
                    Alignment::Left,
                    Some(110),
                ),
            ],
            Alignment::Left,
            6,
        );

        let button_hints = Row::new(
            Point::new(x + w as i32 - 12, y + h as i32 - BUTTON_DIAMETER as i32 - 8),
            vec![
                ButtonHint::new(Point::zero(), Key::A, "Select".to_owned(), Alignment::Right),
                ButtonHint::new(Point::zero(), Key::B, "Back".to_owned(), Alignment::Right),
            ],
            Alignment::Right,
            12,
        );

        Ok(Self {
            rect,
            list,
            child: None,
            button_hints,
            dirty: true,
        })
    }

    async fn select_entry(&mut self, _commands: Sender<Command>) -> Result<()> {
        let rect = Rect::new(
            self.rect.x + 146,
            self.rect.y,
            self.rect.w - 146,
            self.rect.h,
        );
        match self.list.selected() {
            0 => self.child = Some(Box::new(Wifi::new(rect))),
            1 => self.child = Some(Box::new(Display::new(rect))),
            2 => self.child = Some(Box::new(Theme::new(rect))),
            3 => self.child = Some(Box::new(System::new(rect))),
            _ => unreachable!("Invalid index"),
        }
        self.dirty = true;
        Ok(())
    }
}

#[async_trait(?Send)]
impl View for Settings {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let mut drawn = false;

        if self.dirty {
            display.load(self.bounding_box(styles))?;
            self.dirty = false;
        }

        if self.list.should_draw() && self.list.draw(display, styles)? {
            drawn = true;
        }

        if let Some(ref mut child) = self.child {
            if child.draw(display, styles)? {
                drawn = true;
            }
        } else if self.button_hints.should_draw() && self.button_hints.draw(display, styles)? {
            drawn = true;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.child
            .as_ref()
            .map(|c| c.should_draw())
            .unwrap_or(false)
            || self.list.should_draw()
            || self.button_hints.should_draw()
    }

    fn set_should_draw(&mut self) {
        if let Some(ref mut child) = self.child {
            child.set_should_draw();
        }
        self.list.set_should_draw();
        self.button_hints.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if let Some(ref mut child) = self.child {
            let mut child_bubble = VecDeque::new();
            if child
                .handle_key_event(event, commands, &mut child_bubble)
                .await?
            {
                while let Some(command) = child_bubble.pop_front() {
                    match command {
                        Command::CloseView => {
                            self.dirty = true;
                            self.child = None;
                        }
                        _ => bubble.push_front(command),
                    }
                }
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            match event {
                KeyEvent::Pressed(Key::A) => {
                    self.select_entry(commands).await?;
                    Ok(true)
                }
                _ => self.list.handle_key_event(event, commands, bubble).await,
            }
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        let mut children: Vec<&dyn View> = vec![&self.list, &self.button_hints];
        if let Some(child) = self.child.as_deref() {
            children.push(child);
        }
        children
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        let mut children: Vec<&mut dyn View> = vec![&mut self.list, &mut self.button_hints];
        if let Some(child) = self.child.as_deref_mut() {
            children.push(child);
        }
        children
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}
