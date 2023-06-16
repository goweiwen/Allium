use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::SELECTION_HEIGHT;
use common::geom::{Alignment, Point, Rect};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use common::view::{ColorPicker, SettingsList, Toggle, View};
use tokio::sync::mpsc::Sender;

pub struct Theme {
    rect: Rect,
    stylesheet: Stylesheet,
    list: SettingsList,
}

impl Theme {
    pub fn new(rect: Rect) -> Self {
        let stylesheet = Stylesheet::load().unwrap();

        let list = SettingsList::new(
            Rect::new(rect.x, rect.y + 8, rect.w, rect.h - 16),
            vec![
                "Dark Mode".to_string(),
                "Enable Box Art".to_string(),
                "Highlight Color".to_string(),
                "Foreground Color".to_string(),
                "Background Color".to_string(),
                "Disabled Color".to_string(),
                "Button A Color".to_string(),
                "Button B Color".to_string(),
                "Button X Color".to_string(),
                "Button Y Color".to_string(),
            ],
            vec![
                Box::new(Toggle::new(
                    Point::zero(),
                    stylesheet.background_color.is_dark(),
                    Alignment::Right,
                )),
                Box::new(Toggle::new(
                    Point::zero(),
                    stylesheet.enable_box_art,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.highlight_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.foreground_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.background_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.disabled_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_a_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_b_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_x_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_y_color,
                    Alignment::Right,
                )),
            ],
            SELECTION_HEIGHT,
        );

        Self {
            rect,
            stylesheet,
            list,
        }
    }
}

#[async_trait(?Send)]
impl View for Theme {
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
        if self
            .list
            .handle_key_event(event, commands.clone(), bubble)
            .await?
        {
            while let Some(command) = bubble.pop_front() {
                match command {
                    Command::ValueChanged(i, val) => {
                        match i {
                            0 => todo!(),
                            1 => self.stylesheet.enable_box_art = val.as_bool().unwrap(),
                            2 => self.stylesheet.highlight_color = val.as_color().unwrap(),
                            3 => self.stylesheet.foreground_color = val.as_color().unwrap(),
                            4 => self.stylesheet.background_color = val.as_color().unwrap(),
                            5 => self.stylesheet.disabled_color = val.as_color().unwrap(),
                            6 => self.stylesheet.button_a_color = val.as_color().unwrap(),
                            7 => self.stylesheet.button_b_color = val.as_color().unwrap(),
                            8 => self.stylesheet.button_x_color = val.as_color().unwrap(),
                            9 => self.stylesheet.button_y_color = val.as_color().unwrap(),
                            _ => unreachable!("Invalid index"),
                        }

                        commands
                            .send(Command::SaveStylesheet(Box::new(self.stylesheet.clone())))
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
