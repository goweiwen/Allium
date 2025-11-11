use std::collections::VecDeque;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::display::Display;
use common::display::color::Color;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, ButtonIcon, Label, Row, SettingsList, TextBox, Toggle, View};
use common::wifi::{self, WiFiSettings};
use log::warn;
use qrcode::QrCode;
use tokio::sync::mpsc::Sender;

use crate::view::settings::{ChildState, SettingsChild};

pub struct Wifi {
    rect: Rect,
    res: Resources,
    settings: WiFiSettings,
    list: SettingsList,
    has_ip_address: bool,
    check_ip_delay: Duration,
    button_hints: Row<ButtonHint<String>>,
    edit_button: Option<ButtonHint<String>>,
}

impl Wifi {
    pub fn new(rect: Rect, res: Resources, state: Option<ChildState>) -> Self {
        let Rect { x, y, w, h } = rect;

        let settings = WiFiSettings::load().unwrap();

        let locale = res.get::<Locale>();
        let styles = res.get::<Stylesheet>();

        let mut list = SettingsList::new(
            res.clone(),
            Rect::new(
                x + styles.margin_x,
                y,
                w - styles.margin_x as u32 * 2,
                h - ButtonIcon::diameter(&styles) - styles.margin_y as u32,
            ),
            vec![
                locale.t("settings-wifi-wifi-enabled"),
                locale.t("settings-wifi-ip-address"),
                locale.t("settings-wifi-wifi-network"),
                locale.t("settings-wifi-wifi-password"),
                locale.t("settings-wifi-ntp-enabled"),
                locale.t("settings-wifi-web-file-explorer"),
                locale.t("settings-wifi-telnet-enabled"),
                locale.t("settings-wifi-ftp-enabled"),
                locale.t("settings-wifi-syncthing"),
            ],
            vec![
                Box::new(Toggle::new(Point::zero(), settings.wifi, Alignment::Right)),
                Box::new(Label::new(
                    Point::zero(),
                    String::new(),
                    Alignment::Right,
                    None,
                )),
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
                Box::new(Toggle::new(Point::zero(), settings.ntp, Alignment::Right)),
                Box::new(Toggle::new(
                    Point::zero(),
                    settings.web_file_browser,
                    Alignment::Right,
                )),
                Box::new(Toggle::new(
                    Point::zero(),
                    settings.telnet,
                    Alignment::Right,
                )),
                Box::new(Toggle::new(Point::zero(), settings.ftp, Alignment::Right)),
                Box::new(Toggle::new(
                    Point::zero(),
                    settings.syncthing,
                    Alignment::Right,
                )),
            ],
            res.get::<Stylesheet>().ui_font.size + styles.padding_y as u32,
        );
        if let Some(state) = state {
            list.select(state.selected);
        }

        let button_hints = Row::new(
            Point::new(
                rect.x + rect.w as i32 - styles.margin_y,
                rect.y + rect.h as i32 - ButtonIcon::diameter(&styles) as i32 - styles.margin_y,
            ),
            vec![
                ButtonHint::new(
                    res.clone(),
                    Point::zero(),
                    Key::A,
                    locale.t("button-edit"),
                    Alignment::Right,
                ),
                ButtonHint::new(
                    res.clone(),
                    Point::zero(),
                    Key::B,
                    locale.t("button-back"),
                    Alignment::Right,
                ),
            ],
            Alignment::Right,
            12,
        );
        let edit_button = Some(ButtonHint::new(
            res.clone(),
            Point::zero(),
            Key::A,
            locale.t("button-edit"),
            Alignment::Right,
        ));

        drop(locale);
        drop(styles);

        Self {
            rect,
            res,
            settings,
            list,
            has_ip_address: false,
            check_ip_delay: Duration::ZERO,
            button_hints,
            edit_button,
        }
    }
}

#[async_trait(?Send)]
impl View for Wifi {
    fn update(&mut self, dt: Duration) {
        if self.settings.wifi {
            if !self.has_ip_address {
                if self.check_ip_delay > dt {
                    self.check_ip_delay -= dt;
                    return;
                }
                self.check_ip_delay = Duration::from_secs(1);
                // Try to get the IP address if we don't have it yet
                if let Some(ip_address) = wifi::ip_address() {
                    self.has_ip_address = true;
                    self.list.set_right(
                        1,
                        Box::new(Label::new(
                            Point::zero(),
                            ip_address,
                            Alignment::Right,
                            None,
                        )),
                    );
                } else {
                    let locale = self.res.get::<Locale>();
                    self.list.set_right(
                        1,
                        Box::new(Label::new(
                            Point::zero(),
                            locale.t("settings-wifi-connecting"),
                            Alignment::Right,
                            None,
                        )),
                    );
                }
            }
        } else if self.has_ip_address {
            self.has_ip_address = false;
            self.list.set_right(
                1,
                Box::new(Label::new(
                    Point::zero(),
                    String::new(),
                    Alignment::Right,
                    None,
                )),
            );
        }
    }

    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let mut drawn = false;

        drawn |= self.list.should_draw() && self.list.draw(display, styles)?;

        if self.button_hints.should_draw() {
            display.load(Rect::new(
                self.rect.x,
                self.rect.y + self.rect.h as i32 - ButtonIcon::diameter(styles) as i32 - 8,
                self.rect.w,
                ButtonIcon::diameter(styles),
            ))?;
            drawn |= self.button_hints.draw(display, styles)?;
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
        if self
            .list
            .handle_key_event(event, commands.clone(), bubble)
            .await?
        {
            if self.list.selected() == 1 && self.button_hints.len() == 2 {
                self.edit_button = self.button_hints.remove(0);
            } else if let Some(button) = self.edit_button.take()
                && self.button_hints.len() == 1
            {
                self.button_hints.insert(0, button);
            }
            while let Some(command) = bubble.pop_front() {
                if let Command::ValueChanged(i, val) = command {
                    match i {
                        0 => {
                            self.settings.set_wifi(val.as_bool().unwrap())?;
                            let commands = commands.clone();
                            tokio::spawn(async move {
                                if wifi::wait_for_wifi().await.is_ok() {
                                    commands.send(Command::Redraw).await.ok();
                                }
                            });
                        }
                        1 => {} // ip address
                        2 => self
                            .settings
                            .set_ssid(val.as_string().unwrap().to_string())?,
                        3 => self
                            .settings
                            .set_password(val.as_string().unwrap().to_string())?,
                        4 => self.settings.toggle_ntp(val.as_bool().unwrap())?,
                        5 => {
                            let enabled = val.as_bool().unwrap();
                            self.settings.toggle_web_file_browser(enabled)?;
                            if enabled {
                                let fg_color = self.res.get::<Stylesheet>().foreground_color;
                                let commands = commands.clone();
                                tokio::spawn(async move {
                                    if wifi::wait_for_wifi().await.is_ok()
                                        && let Some(ip_address) = wifi::ip_address()
                                    {
                                        let url = format!("http://{ip_address}/");
                                        let Ok(code) = QrCode::new(url.as_bytes()) else {
                                            warn!(
                                                "Failed to generate QR code for web file explorer"
                                            );
                                            return;
                                        };
                                        let image = code
                                            .render::<image::Rgba<u8>>()
                                            .dark_color(fg_color.into())
                                            .light_color(Color::rgba(0, 0, 0, 0).into())
                                            .min_dimensions(300, 300)
                                            .build();
                                        commands
                                            .send(Command::ImageToast(image, url, None))
                                            .await
                                            .ok();
                                    }
                                });
                            } else {
                                commands.send(Command::DismissToast).await.ok();
                            }
                        }
                        6 => self.settings.toggle_telnet(val.as_bool().unwrap())?,
                        7 => self.settings.toggle_ftp(val.as_bool().unwrap())?,
                        8 => {
                            let enabled = val.as_bool().unwrap();
                            self.settings.toggle_syncthing(enabled)?;
                            if enabled {
                                let fg_color = self.res.get::<Stylesheet>().foreground_color;
                                let commands = commands.clone();
                                tokio::spawn(async move {
                                    if wifi::wait_for_wifi().await.is_ok()
                                        && let Some(ip_address) = wifi::ip_address()
                                    {
                                        let url = format!("http://{ip_address}:8384/");
                                        let Ok(code) = QrCode::new(url.as_bytes()) else {
                                            warn!(
                                                "Failed to generate QR code for web file explorer"
                                            );
                                            return;
                                        };
                                        let image = code
                                            .render::<image::Rgba<u8>>()
                                            .dark_color(fg_color.into())
                                            .light_color(Color::rgba(0, 0, 0, 0).into())
                                            .min_dimensions(300, 300)
                                            .build();
                                        commands
                                            .send(Command::ImageToast(image, url, None))
                                            .await
                                            .ok();
                                    }
                                });
                            } else {
                                commands.send(Command::DismissToast).await.ok();
                            }
                        }
                        _ => unreachable!("Invalid index"),
                    }
                }
                self.settings.save()?;
            }
            return Ok(true);
        }

        match event {
            KeyEvent::Pressed(Key::B) => {
                commands.send(Command::DismissToast).await.ok();
                bubble.push_back(Command::CloseView);
                Ok(true)
            }
            _ => Ok(false),
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

impl SettingsChild for Wifi {
    fn save(&self) -> ChildState {
        ChildState {
            selected: self.list.selected(),
        }
    }
}
