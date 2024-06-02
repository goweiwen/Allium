use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::SELECTION_MARGIN;

use common::display::Display as DisplayTrait;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::power::{PowerButtonAction, PowerSettings};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, ButtonIcon, Number, Row, Select, SettingsList, Toggle, View};

use tokio::sync::mpsc::Sender;

use crate::view::settings::{ChildState, SettingsChild};

pub struct Power {
    rect: Rect,
    power_settings: PowerSettings,
    list: SettingsList,
    button_hints: Row<ButtonHint<String>>,
}

impl Power {
    pub fn new(rect: Rect, res: Resources, state: Option<ChildState>) -> Self {
        let Rect { x, y, w, h } = rect;

        let locale = res.get::<Locale>();
        let styles = res.get::<Stylesheet>();
        let power_settings = PowerSettings::load().unwrap_or_default();

        let auto_sleep_duration_disabled_label =
            locale.t("settings-power-auto-sleep-duration-disabled");
        let mut list = SettingsList::new(
            Rect::new(
                x + 12,
                y + 8,
                w - 24,
                h - 8 - ButtonIcon::diameter(&styles) - 8,
            ),
            vec![
                locale.t("settings-power-power-button-action"),
                locale.t("settings-power-auto-sleep-when-charging"),
                locale.t("settings-power-auto-sleep-duration-minutes"),
            ],
            vec![
                Box::new(Select::new(
                    Point::zero(),
                    power_settings.power_button_action as usize,
                    vec![
                        locale.t("settings-power-power-button-action-suspend"),
                        locale.t("settings-power-power-button-action-shutdown"),
                        locale.t("settings-power-power-button-action-nothing"),
                    ],
                    Alignment::Right,
                )),
                Box::new(Toggle::new(
                    Point::zero(),
                    power_settings.auto_sleep_when_charging,
                    Alignment::Right,
                )),
                Box::new(Number::new(
                    Point::zero(),
                    power_settings.auto_sleep_duration_minutes,
                    0,
                    60,
                    move |x: &i32| {
                        if *x == 0 {
                            auto_sleep_duration_disabled_label.clone()
                        } else {
                            x.to_string()
                        }
                    },
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

        Self {
            rect,
            power_settings,
            list,
            button_hints,
        }
    }
}

#[async_trait(?Send)]
impl View for Power {
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
            while let Some(command) = bubble.pop_front() {
                if let Command::ValueChanged(i, val) = command {
                    match i {
                        0 => {
                            self.power_settings.power_button_action =
                                PowerButtonAction::from_repr(val.as_int().unwrap() as usize)
                                    .unwrap_or_default()
                        }
                        1 => self.power_settings.auto_sleep_when_charging = val.as_bool().unwrap(),
                        2 => {
                            self.power_settings.auto_sleep_duration_minutes = val.as_int().unwrap()
                        }
                        _ => unreachable!("Invalid index"),
                    }
                    self.power_settings.save()?;
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

impl SettingsChild for Power {
    fn save(&self) -> ChildState {
        ChildState {
            selected: self.list.selected(),
        }
    }
}
