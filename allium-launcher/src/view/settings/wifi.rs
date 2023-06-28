use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::{BUTTON_DIAMETER, SELECTION_MARGIN};
use common::display::Display;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, Label, Row, SettingsList, TextBox, Toggle, View};
use common::wifi::{self, WiFiSettings};
use tokio::sync::mpsc::Sender;

pub struct Wifi {
    rect: Rect,
    res: Resources,
    settings: WiFiSettings,
    list: SettingsList,
    has_ip_address: bool,
    ip_address_label: Label<String>,
    button_hints: Row<ButtonHint<String>>,
}

impl Wifi {
    pub fn new(rect: Rect, res: Resources) -> Self {
        let Rect { x, y, w, h } = rect;

        let settings = WiFiSettings::load().unwrap();

        let locale = res.get::<Locale>();

        let list = SettingsList::new(
            Rect::new(
                x + 12,
                y + 8,
                w - 24,
                h - 8 - 48 - 12 - res.get::<Stylesheet>().ui_font.size - 12,
            ),
            vec![
                locale.t("settings-wifi-wifi-enabled"),
                locale.t("settings-wifi-wifi-network"),
                locale.t("settings-wifi-wifi-password"),
                locale.t("settings-wifi-telnet-enabled"),
                locale.t("settings-wifi-ftp-enabled"),
            ],
            vec![
                Box::new(Toggle::new(Point::zero(), settings.wifi, Alignment::Right)),
                Box::new(TextBox::new(
                    Point::zero(),
                    res.clone(),
                    settings.ssid.clone(),
                    Alignment::Right,
                    false,
                )),
                Box::new(TextBox::new(
                    Point::zero(),
                    res.clone(),
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
            res.get::<Stylesheet>().ui_font.size + SELECTION_MARGIN,
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
                ButtonHint::new(
                    Point::zero(),
                    Key::A,
                    locale.t("button-edit"),
                    Alignment::Right,
                ),
                ButtonHint::new(
                    Point::zero(),
                    Key::B,
                    locale.t("button-back"),
                    Alignment::Right,
                ),
            ],
            Alignment::Right,
            12,
        );

        std::mem::drop(locale);

        Self {
            rect,
            res,
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

        let locale = self.res.get::<Locale>();
        if self.settings.wifi {
            if !self.has_ip_address {
                // Try to get the IP address if we don't have it yet
                if let Some(ip_address) = wifi::ip_address() {
                    self.has_ip_address = true;
                    display.load(self.ip_address_label.bounding_box(styles))?;
                    self.ip_address_label.set_text(ip_address);
                } else {
                    self.ip_address_label
                        .set_text(locale.t("settings-wifi-connecting"));
                }
            }
        } else if self.has_ip_address {
            display.load(self.ip_address_label.bounding_box(styles))?;
            self.ip_address_label.set_text(String::new());
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
