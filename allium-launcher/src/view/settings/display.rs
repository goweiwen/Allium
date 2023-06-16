use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::{BUTTON_DIAMETER, SELECTION_HEIGHT};
use common::display::color::Color;
use common::display::settings::DisplaySettings;
use common::geom::{Alignment, Point, Rect};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, Label, Percentage, Row, SettingsList, View};
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::{Primitive, PrimitiveStyle, Rectangle};
use embedded_graphics::Drawable;
use tokio::sync::mpsc::Sender;

pub struct Display {
    rect: Rect,
    settings: DisplaySettings,
    list: SettingsList,
    restart_label: Label<String>,
    button_hints: Row<ButtonHint<String>>,
    has_changed: bool,
}

impl Display {
    pub fn new(rect: Rect) -> Self {
        let settings = DisplaySettings::load().unwrap();

        let list = SettingsList::new(
            Rect::new(rect.x, rect.y + 8, rect.w - 12, rect.h - 8),
            vec![
                "Brightness".to_string(),
                "Luminance".to_string(),
                "Hue".to_string(),
                "Saturation".to_string(),
                "Contrast".to_string(),
            ],
            vec![
                Box::new(Percentage::new(
                    Point::zero(),
                    settings.brightness as i32,
                    Alignment::Right,
                )),
                Box::new(Percentage::new(
                    Point::zero(),
                    settings.luminance as i32,
                    Alignment::Right,
                )),
                Box::new(Percentage::new(
                    Point::zero(),
                    settings.hue as i32,
                    Alignment::Right,
                )),
                Box::new(Percentage::new(
                    Point::zero(),
                    settings.saturation as i32,
                    Alignment::Right,
                )),
                Box::new(Percentage::new(
                    Point::zero(),
                    settings.contrast as i32,
                    Alignment::Right,
                )),
            ],
            SELECTION_HEIGHT,
        );

        let restart_label = Label::new(
            Point::new(
                rect.x + rect.w as i32 - 12,
                rect.y + rect.h as i32 - 58 - 30,
            ),
            "*Restart device to apply changes".to_owned(),
            Alignment::Right,
            None,
        );

        let button_hints = Row::new(
            Point::new(
                rect.x + rect.w as i32 - 12,
                rect.y + rect.h as i32 - BUTTON_DIAMETER as i32 - 8,
            ),
            vec![
                ButtonHint::new(Point::zero(), Key::A, "Edit".to_owned(), Alignment::Right),
                ButtonHint::new(Point::zero(), Key::B, "Back".to_owned(), Alignment::Right),
            ],
            Alignment::Right,
            12,
        );

        Self {
            rect,
            settings,
            list,
            restart_label,
            button_hints,
            has_changed: false,
        }
    }
}

#[async_trait(?Send)]
impl View for Display {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let mut drawn = false;

        if self.list.should_draw() && self.list.draw(display, styles)? {
            drawn = true;
        }

        if self.has_changed && self.restart_label.draw(display, styles)? {
            drawn = true;
        }

        if self.button_hints.should_draw() && self.button_hints.draw(display, styles)? {
            drawn = true;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.list.should_draw()
            || self.has_changed && self.restart_label.should_draw()
            || self.button_hints.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.list.set_should_draw();
        self.restart_label.set_should_draw();
        self.button_hints.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if self
            .list
            .handle_key_event(event, commands.clone(), bubble)
            .await?
        {
            while let Some(command) = bubble.pop_front() {
                match command {
                    Command::ValueChanged(i, val) => {
                        match i {
                            0 => self.settings.brightness = val.as_int().unwrap() as u8,
                            1 => self.settings.luminance = val.as_int().unwrap() as u8,
                            2 => self.settings.hue = val.as_int().unwrap() as u8,
                            3 => self.settings.saturation = val.as_int().unwrap() as u8,
                            4 => self.settings.contrast = val.as_int().unwrap() as u8,
                            _ => unreachable!("Invalid index"),
                        }

                        self.has_changed = true;

                        commands
                            .send(Command::SaveDisplaySettings(Box::new(
                                self.settings.clone(),
                            )))
                            .await?;
                    }
                    _ => {}
                }
            }
            return Ok(true);
        }

        match event {
            KeyEvent::Pressed(Key::B) => {
                bubble.push_back(Command::CloseView);
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.list, &self.restart_label, &self.button_hints]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![
            &mut self.list,
            &mut self.restart_label,
            &mut self.button_hints,
        ]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}
