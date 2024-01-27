use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::SELECTION_MARGIN;

use common::display::settings::DisplaySettings;
use common::display::Display as DisplayTrait;
use common::geom::{Alignment, Point, Rect, Size};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, ButtonIcon, Label, Percentage, Row, SettingsList, View};

use tokio::sync::mpsc::Sender;

use crate::view::settings::{ChildState, SettingsChild};

pub struct Display {
    rect: Rect,
    settings: DisplaySettings,
    list: SettingsList,
    button_hints: Row<ButtonHint<String>>,
    edit_button: Option<ButtonHint<String>>,
}

impl Display {
    pub fn new(rect: Rect, res: Resources, state: Option<ChildState>) -> Self {
        let Rect { x, y, w, h } = rect;

        let settings = DisplaySettings::load().unwrap();

        let locale = res.get::<Locale>();
        let styles = res.get::<Stylesheet>();

        let mut list = SettingsList::new(
            Rect::new(
                x + 12,
                y + 8,
                w - 24,
                h - 8 - ButtonIcon::diameter(&styles) - 8,
            ),
            vec![
                locale.t("settings-display-screen-resolution"),
                locale.t("settings-display-luminance"),
                locale.t("settings-display-hue"),
                locale.t("settings-display-saturation"),
                locale.t("settings-display-contrast"),
                locale.t("settings-display-red"),
                locale.t("settings-display-green"),
                locale.t("settings-display-blue"),
            ],
            vec![
                Box::new(Label::new(
                    Point::zero(),
                    {
                        let size = res.get::<Size>();
                        format!("{}x{}", size.w, size.h)
                    },
                    Alignment::Right,
                    None,
                )),
                Box::new(Percentage::new(
                    Point::zero(),
                    0,
                    100,
                    i32::from(settings.luminance),
                    Alignment::Right,
                )),
                Box::new(Percentage::new(
                    Point::zero(),
                    0,
                    100,
                    i32::from(settings.hue),
                    Alignment::Right,
                )),
                Box::new(Percentage::new(
                    Point::zero(),
                    0,
                    100,
                    i32::from(settings.saturation),
                    Alignment::Right,
                )),
                Box::new(Percentage::new(
                    Point::zero(),
                    0,
                    100,
                    i32::from(settings.contrast),
                    Alignment::Right,
                )),
                Box::new(Percentage::new(
                    Point::zero(),
                    0,
                    100,
                    i32::from(settings.r),
                    Alignment::Right,
                )),
                Box::new(Percentage::new(
                    Point::zero(),
                    0,
                    100,
                    i32::from(settings.g),
                    Alignment::Right,
                )),
                Box::new(Percentage::new(
                    Point::zero(),
                    0,
                    100,
                    i32::from(settings.b),
                    Alignment::Right,
                )),
            ],
            styles.ui_font.size + SELECTION_MARGIN,
        );
        if let Some(state) = state {
            list.select(state.selected);
        }

        let button_hints = Row::new(
            Point::new(
                rect.x + rect.w as i32 - 12,
                rect.y + rect.h as i32 - ButtonIcon::diameter(&styles) as i32 - 8,
            ),
            vec![ButtonHint::new(
                res.clone(),
                Point::zero(),
                Key::B,
                locale.t("button-back"),
                Alignment::Right,
            )],
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

        Self {
            rect,
            settings,
            list,
            button_hints,
            edit_button,
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
            if self.list.selected() == 0 && self.button_hints.len() == 2 {
                self.edit_button = self.button_hints.remove(0);
            } else if let Some(button) = self.edit_button.take() {
                if self.button_hints.len() == 1 {
                    self.button_hints.insert(0, button);
                }
            }
            while let Some(command) = bubble.pop_front() {
                if let Command::ValueChanged(i, val) = command {
                    match i {
                        0 => {}
                        1 => self.settings.luminance = val.as_int().unwrap() as u8,
                        2 => self.settings.hue = val.as_int().unwrap() as u8,
                        3 => self.settings.saturation = val.as_int().unwrap() as u8,
                        4 => self.settings.contrast = val.as_int().unwrap() as u8,
                        5 => self.settings.r = val.as_int().unwrap() as u8,
                        6 => self.settings.g = val.as_int().unwrap() as u8,
                        7 => self.settings.b = val.as_int().unwrap() as u8,
                        _ => unreachable!("Invalid index"),
                    }

                    commands
                        .send(Command::SaveDisplaySettings(Box::new(
                            self.settings.clone(),
                        )))
                        .await?;
                }
            }
            return Ok(true);
        }

        match event {
            KeyEvent::Pressed(Key::B) => {
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

impl SettingsChild for Display {
    fn save(&self) -> ChildState {
        ChildState {
            selected: self.list.selected(),
        }
    }
}
