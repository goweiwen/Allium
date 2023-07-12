use std::collections::VecDeque;
use std::env;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Local;
use common::command::Command;
use common::constants::{ALLIUM_TIMEZONE, SELECTION_MARGIN};

use common::display::Display as DisplayTrait;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, ButtonIcon, DateTime, Row, Select, SettingsList, View};

use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::Sender;

use crate::view::settings::{ChildState, SettingsChild};

pub struct Clock {
    rect: Rect,
    timezone: usize,
    list: SettingsList,
    button_hints: Row<ButtonHint<String>>,
}

// POSIX TZ offset are opposite of UTC naming convention:
// https://unix.stackexchange.com/questions/104088/why-does-tz-utc-8-produce-dates-that-are-utc8
const TIMEZONE_VALUES: [&str; 39] = [
    "UTC-0",
    "UTC-1",
    "UTC-2",
    "UTC-3",
    "UTC-3:30",
    "UTC-4",
    "UTC-4:30",
    "UTC-5",
    "UTC-5:30",
    "UTC-5:45",
    "UTC-6",
    "UTC-6:30",
    "UTC-7",
    "UTC-8",
    "UTC-8:45",
    "UTC-9",
    "UTC-9:30",
    "UTC-10",
    "UTC-10:30",
    "UTC-11",
    "UTC-12",
    "UTC-12:45",
    "UTC-13",
    "UTC-13:45",
    "UTC-14",
    "UTC+12",
    "UTC+11",
    "UTC+10",
    "UTC+9:30",
    "UTC+9",
    "UTC+8",
    "UTC+7",
    "UTC+6",
    "UTC+5",
    "UTC+4",
    "UTC+3",
    "UTC+3:30",
    "UTC+2",
    "UTC+1",
];

const TIMEZONE_NAMES: [&str; 39] = [
    "UTC+0",
    "UTC+1",
    "UTC+2",
    "UTC+3",
    "UTC+3:30",
    "UTC+4",
    "UTC+4:30",
    "UTC+5",
    "UTC+5:30",
    "UTC+5:45",
    "UTC+6",
    "UTC+6:30",
    "UTC+7",
    "UTC+8",
    "UTC+8:45",
    "UTC+9",
    "UTC+9:30",
    "UTC+10",
    "UTC+10:30",
    "UTC+11",
    "UTC+12",
    "UTC+12:45",
    "UTC+13",
    "UTC+13:45",
    "UTC+14",
    "UTC-12",
    "UTC-11",
    "UTC-10",
    "UTC-9:30",
    "UTC-9",
    "UTC-8",
    "UTC-7",
    "UTC-6",
    "UTC-5",
    "UTC-4",
    "UTC-3",
    "UTC-3:30",
    "UTC-2",
    "UTC-1",
];

impl Clock {
    pub fn new(rect: Rect, res: Resources, state: Option<ChildState>) -> Self {
        let Rect { x, y, w, h } = rect;

        let timezone = env::var("TZ")
            .map(|tz| TIMEZONE_VALUES.iter().position(|&s| s == tz).unwrap_or(0))
            .unwrap_or(0);
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
                locale.t("settings-clock-datetime"),
                locale.t("settings-clock-timezone"),
            ],
            vec![
                Box::new(DateTime::new(
                    Point::zero(),
                    Local::now().naive_local(),
                    Alignment::Right,
                )),
                Box::new(Select::new(
                    Point::zero(),
                    timezone,
                    TIMEZONE_NAMES.iter().map(|s| s.to_string()).collect(),
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
            timezone,
            list,
            button_hints,
        }
    }
}

#[async_trait(?Send)]
impl View for Clock {
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
                            let datetime = val.as_datetime().unwrap();
                            let datetime = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
                            tokio::process::Command::new("date")
                                .args(["-s", &datetime])
                                .spawn()?
                                .wait()
                                .await?;
                        }
                        1 => {
                            self.timezone = val.as_int().unwrap() as usize;
                            let timezone = TIMEZONE_VALUES[self.timezone];
                            File::create(ALLIUM_TIMEZONE.as_path())
                                .await?
                                .write_all(timezone.as_bytes())
                                .await?;
                            env::set_var("TZ", timezone);
                            self.list.set_right(
                                0,
                                Box::new(DateTime::new(
                                    Point::zero(),
                                    Local::now().naive_local(),
                                    Alignment::Right,
                                )),
                            );
                        }
                        _ => unreachable!("Invalid index"),
                    }
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

impl SettingsChild for Clock {
    fn save(&self) -> ChildState {
        ChildState {
            selected: self.list.selected(),
        }
    }
}
