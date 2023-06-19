use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::{BUTTON_DIAMETER, SELECTION_HEIGHT};
use common::display::Display;
use common::geom::{Alignment, Point, Rect};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, Label, Row, SettingsList, TextBox, Toggle, View};
use common::wifi::{self, WiFiSettings};
use tokio::sync::mpsc::Sender;

pub struct Wifi {
    rect: Rect,
    settings: WiFiSettings,
    list: SettingsList,
    has_ip_address: bool,
    ip_address_label: Label<String>,
    button_hints: Row<ButtonHint<String>>,
}

impl Wifi {
    pub fn new(rect: Rect) -> Self {
        let settings = WiFiSettings::load().unwrap();

        let list = SettingsList::new(
            Rect::new(rect.x, rect.y + 8, rect.w - 12, rect.h - 8 - 46 - 34 - 12),
            vec![
                "Wi-Fi Enabled".to_string(),
                "Wi-Fi Network Name".to_string(),
                "Wi-Fi Password".to_string(),
                "Telnet Enabled".to_string(),
                "FTP Enabled".to_string(),
            ],
            vec![
                Box::new(Toggle::new(Point::zero(), settings.wifi, Alignment::Right)),
                Box::new(TextBox::new(
                    Point::zero(),
                    settings.ssid.clone(),
                    Alignment::Right,
                    false,
                )),
                Box::new(TextBox::new(
                    Point::zero(),
                    settings.password.clone(),
                    Alignment::Right,
                    true,
                )),
                Box::new(Toggle::new(
                    Point::zero(),
                    settings.telnet,
                    Alignment::Right,
                )),
                Box::new(Toggle::new(Point::zero(), settings.ftp, Alignment::Right)),
            ],
            SELECTION_HEIGHT,
        );

        let ip_address_label = Label::new(
            Point::new(
                rect.x + rect.w as i32 - 12,
                rect.y + rect.h as i32 - 46 - 34,
            ),
            String::new(),
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
            has_ip_address: false,
            ip_address_label,
            button_hints,
        }
    }
}

#[async_trait(?Send)]
impl View for Wifi {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let mut drawn = false;

        if self.settings.wifi {
            if !self.has_ip_address {
                // Try to get the IP address if we don't have it yet
                if let Some(ip_address) = wifi::ip_address() {
                    self.has_ip_address = true;
                    display.load(self.ip_address_label.bounding_box(styles))?;
                    self.ip_address_label.set_text(ip_address);
                } else {
                    display.load(self.ip_address_label.bounding_box(styles))?;
                    self.ip_address_label.set_text("Connecting...".to_owned());
                }
            }
        } else if self.has_ip_address {
            display.load(self.ip_address_label.bounding_box(styles))?;
            self.ip_address_label.set_text("".to_owned());
        }

        drawn |=
            self.ip_address_label.should_draw() && self.ip_address_label.draw(display, styles)?;
        drawn |= self.button_hints.should_draw() && self.button_hints.draw(display, styles)?;
        drawn |= self.list.should_draw() && self.list.draw(display, styles)?;

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.list.should_draw()
            || self.ip_address_label.should_draw()
            || self.button_hints.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.list.set_should_draw();
        self.ip_address_label.set_should_draw();
        self.button_hints.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if self.list.handle_key_event(event, commands, bubble).await? {
            while let Some(command) = bubble.pop_front() {
                if let Command::ValueChanged(i, val) = command {
                    match i {
                        0 => self.settings.toggle_wifi(val.as_bool().unwrap())?,
                        1 => self.settings.ssid = val.as_string().unwrap().to_string(),
                        2 => self.settings.password = val.as_string().unwrap().to_string(),
                        3 => self.settings.toggle_telnet(val.as_bool().unwrap())?,
                        4 => self.settings.toggle_ftp(val.as_bool().unwrap())?,
                        _ => unreachable!("Invalid index"),
                    }
                }
                self.settings.save()?;
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
        vec![&self.list, &self.ip_address_label, &self.button_hints]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![
            &mut self.list,
            &mut self.ip_address_label,
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
