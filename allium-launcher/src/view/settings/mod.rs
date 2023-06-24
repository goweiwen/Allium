mod about;
mod display;
mod language;
mod theme;
mod wifi;

use self::about::About;
use self::display::Display;
use self::language::Language;
use self::theme::Theme;
use self::wifi::Wifi;

use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::{ALLIUM_TOOLS_DIR, BUTTON_DIAMETER, SELECTION_MARGIN};
use common::display::Display as DisplayTrait;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, Row, ScrollList, View};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsState {
    selected: usize,
    has_child: bool,
}

#[derive(Debug)]
pub struct Settings {
    rect: Rect,
    res: Resources,
    list: ScrollList,
    child: Option<Box<dyn View>>,
    button_hints: Row<ButtonHint<String>>,
    dirty: bool,
}

impl Settings {
    pub fn new(rect: Rect, res: Resources, state: SettingsState) -> Result<Self> {
        let Rect { x, y, w, h } = rect;

        let locale = res.get::<Locale>();

        let mut list = ScrollList::new(
            Rect::new(x + 12, y + 8, w - 24, h - 8 - 48),
            vec![
                locale.t("settings-wifi"),
                locale.t("settings-display"),
                locale.t("settings-theme"),
                locale.t("settings-language"),
                locale.t("settings-files"),
                locale.t("settings-about"),
            ],
            Alignment::Left,
            res.get::<Stylesheet>().ui_font.size + SELECTION_MARGIN,
        );
        list.select(state.selected);

        let child: Option<Box<dyn View>> = if state.has_child {
            match state.selected {
                0 => Some(Box::new(Wifi::new(rect, res.clone()))),
                1 => Some(Box::new(Display::new(rect, res.clone()))),
                2 => Some(Box::new(Theme::new(rect, res.clone()))),
                3 => Some(Box::new(Language::new(rect, res.clone()))),
                4 => None,
                5 => Some(Box::new(About::new(rect, res.clone()))),
                _ => None,
            }
        } else {
            None
        };

        let button_hints = Row::new(
            Point::new(x + w as i32 - 12, y + h as i32 - BUTTON_DIAMETER as i32 - 8),
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
            dirty: true,
        })
    }

    pub fn save(&self) -> SettingsState {
        SettingsState {
            selected: self.list.selected(),
            has_child: self.child.is_some(),
        }
    }

    async fn select_entry(&mut self, commands: Sender<Command>) -> Result<()> {
        match self.list.selected() {
            0 => self.child = Some(Box::new(Wifi::new(self.rect, self.res.clone()))),
            1 => self.child = Some(Box::new(Display::new(self.rect, self.res.clone()))),
            2 => self.child = Some(Box::new(Theme::new(self.rect, self.res.clone()))),
            3 => self.child = Some(Box::new(Language::new(self.rect, self.res.clone()))),
            4 => {
                let path = ALLIUM_TOOLS_DIR.join("Files.pak");
                let mut command = std::process::Command::new(path.join("launch.sh"));
                command.current_dir(path);
                commands.send(Command::Exec(command)).await?;
            }
            5 => self.child = Some(Box::new(About::new(self.rect, self.res.clone()))),
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

        if let Some(ref mut child) = self.child {
            return child.draw(display, styles);
        }

        if self.dirty {
            display.load(self.bounding_box(styles))?;
            self.dirty = false;
        }

        drawn |= self.list.should_draw() && self.list.draw(display, styles)?;
        drawn |= self.button_hints.should_draw() && self.button_hints.draw(display, styles)?;

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
            if child.handle_key_event(event, commands, bubble).await? {
                bubble.retain(|cmd| match cmd {
                    Command::CloseView => {
                        self.set_should_draw();
                        self.child = None;
                        true
                    }
                    _ => false,
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
