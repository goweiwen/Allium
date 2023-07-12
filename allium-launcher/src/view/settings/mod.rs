mod about;
mod clock;
mod display;
mod language;
mod theme;
mod wifi;

use crate::view::settings::clock::Clock;

use self::about::About;
use self::display::Display;
use self::language::Language;
use self::theme::Theme;
use self::wifi::Wifi;

use std::collections::VecDeque;
use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::SELECTION_MARGIN;
use common::display::Display as DisplayTrait;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, ButtonIcon, Row, ScrollList, View};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsState {
    selected: usize,
    child: Option<ChildState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChildState {
    selected: usize,
}

trait SettingsChild: View {
    fn save(&self) -> ChildState;
}

impl Debug for dyn SettingsChild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("dyn SettingsChild").finish()
    }
}

#[derive(Debug)]
pub struct Settings {
    rect: Rect,
    res: Resources,
    list: ScrollList,
    child: Option<Box<dyn SettingsChild>>,
    button_hints: Row<ButtonHint<String>>,
    has_wifi: bool,
    dirty: bool,
}

impl Settings {
    pub fn new(rect: Rect, res: Resources, state: SettingsState) -> Result<Self> {
        let Rect { x, y, w, h } = rect;

        let locale = res.get::<Locale>();
        let styles = res.get::<Stylesheet>();

        let has_wifi = DefaultPlatform::has_wifi();
        let mut labels = Vec::with_capacity(7);
        if has_wifi {
            labels.push(locale.t("settings-wifi"));
        }
        labels.push(locale.t("settings-clock"));
        labels.push(locale.t("settings-display"));
        labels.push(locale.t("settings-theme"));
        labels.push(locale.t("settings-language"));
        labels.push(locale.t("settings-about"));

        let mut list = ScrollList::new(
            Rect::new(x + 12, y + 8, w - 24, h - 8 - styles.ui_font.size - 8),
            labels,
            Alignment::Left,
            styles.ui_font.size + SELECTION_MARGIN,
        );
        list.select(state.selected);

        let child: Option<Box<dyn SettingsChild>> = if let Some(child) = state.child {
            let mut selected = state.selected;
            if !has_wifi {
                selected += 1;
            };
            match selected {
                0 => Some(Box::new(Wifi::new(rect, res.clone(), Some(child)))),
                1 => Some(Box::new(Clock::new(rect, res.clone(), Some(child)))),
                2 => Some(Box::new(Display::new(rect, res.clone(), Some(child)))),
                3 => Some(Box::new(Theme::new(rect, res.clone(), Some(child)))),
                4 => Some(Box::new(Language::new(rect, res.clone(), Some(child)))),
                5 => Some(Box::new(About::new(rect, res.clone(), Some(child)))),
                _ => None,
            }
        } else {
            None
        };

        let button_hints = Row::new(
            Point::new(
                x + w as i32 - 12,
                y + h as i32 - ButtonIcon::diameter(&styles) as i32 - 8,
            ),
            vec![
                ButtonHint::new(
                    Point::zero(),
                    Key::A,
                    locale.t("button-select"),
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

        Ok(Self {
            rect,
            res: res.clone(),
            list,
            child,
            button_hints,
            has_wifi,
            dirty: true,
        })
    }

    pub fn save(&self) -> SettingsState {
        SettingsState {
            selected: self.list.selected(),
            child: self.child.as_ref().map(|c| c.save()),
        }
    }

    async fn select_entry(&mut self, _commands: Sender<Command>) -> Result<()> {
        let mut selected = self.list.selected();
        if !self.has_wifi {
            selected += 1
        };
        match selected {
            0 => self.child = Some(Box::new(Wifi::new(self.rect, self.res.clone(), None))),
            1 => self.child = Some(Box::new(Clock::new(self.rect, self.res.clone(), None))),
            2 => self.child = Some(Box::new(Display::new(self.rect, self.res.clone(), None))),
            3 => self.child = Some(Box::new(Theme::new(self.rect, self.res.clone(), None))),
            4 => self.child = Some(Box::new(Language::new(self.rect, self.res.clone(), None))),
            5 => self.child = Some(Box::new(About::new(self.rect, self.res.clone(), None))),
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

        if let Some(ref mut child) = self.child {
            return child.draw(display, styles);
        }

        drawn |= self.list.should_draw() && self.list.draw(display, styles)?;
        drawn |= self.button_hints.should_draw() && self.button_hints.draw(display, styles)?;

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        if let Some(child) = self.child.as_ref() {
            child.should_draw()
        } else {
            self.list.should_draw() || self.button_hints.should_draw()
        }
    }

    fn set_should_draw(&mut self) {
        if let Some(ref mut child) = self.child {
            child.set_should_draw();
        } else {
            self.list.set_should_draw();
            self.button_hints.set_should_draw();
        }
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if let Some(ref mut child) = self.child {
            if child.handle_key_event(event, commands, bubble).await? {
                bubble.retain(|cmd| match cmd {
                    Command::CloseView => {
                        self.child = None;
                        self.set_should_draw();
                        false
                    }
                    _ => true,
                });
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
        if let Some(child) = self.child.as_deref() {
            vec![child as &dyn View]
        } else {
            vec![&self.list, &self.button_hints]
        }
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        if let Some(child) = self.child.as_deref_mut() {
            vec![child as &mut dyn View]
        } else {
            vec![&mut self.list, &mut self.button_hints]
        }
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}
