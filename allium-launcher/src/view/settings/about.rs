use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::{ALLIUM_VERSION, SELECTION_MARGIN};
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, ButtonIcon, Label, Row, SettingsList, View};
use sysinfo::{DiskExt, SystemExt};
use tokio::sync::mpsc::Sender;

use crate::view::settings::{ChildState, SettingsChild};

pub struct About {
    rect: Rect,
    list: SettingsList,
    button_hints: Row<ButtonHint<String>>,
}

impl About {
    pub fn new(rect: Rect, res: Resources, state: Option<ChildState>) -> Self {
        let Rect { x, y, w, h } = rect;

        let firmware = DefaultPlatform::firmware();

        let mut sys = sysinfo::System::new();
        sys.refresh_disks_list();
        let disk = &sys.disks()[1];

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
                locale.t("settings-about-allium-version"),
                locale.t("settings-about-model-name"),
                locale.t("settings-about-firmware-version"),
                locale.t("settings-about-operating-system-version"),
                locale.t("settings-about-kernel-version"),
                locale.t("settings-about-storage-used"),
            ],
            vec![
                Box::new(Label::new(
                    Point::zero(),
                    format!("v{ALLIUM_VERSION}"),
                    Alignment::Right,
                    None,
                )),
                Box::new(Label::new(
                    Point::zero(),
                    DefaultPlatform::device_model(),
                    Alignment::Right,
                    None,
                )),
                Box::new(Label::new(Point::zero(), firmware, Alignment::Right, None)),
                Box::new(Label::new(
                    Point::zero(),
                    sys.long_os_version().map_or_else(
                        || locale.t("settings-about-unknown-value"),
                        |s| s.trim().to_owned(),
                    ),
                    Alignment::Right,
                    None,
                )),
                Box::new(Label::new(
                    Point::zero(),
                    sys.kernel_version()
                        .unwrap_or_else(|| locale.t("settings-about-unknown-value")),
                    Alignment::Right,
                    None,
                )),
                Box::new(Label::new(
                    Point::zero(),
                    format!(
                        "{}GB / {}GB",
                        (disk.total_space() - disk.available_space()) / (1024 * 1024 * 1024),
                        disk.total_space() / (1024 * 1024 * 1024)
                    ),
                    Alignment::Right,
                    None,
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
            list,
            button_hints,
        }
    }
}

#[async_trait(?Send)]
impl View for About {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let mut drawn = false;

        if self.list.should_draw() && self.list.draw(display, styles)? {
            drawn = true;
        }

        if self.button_hints.should_draw() && self.button_hints.draw(display, styles)? {
            drawn = true;
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
        match event {
            KeyEvent::Pressed(Key::B) => {
                bubble.push_back(Command::CloseView);
                Ok(true)
            }
            _ => self.list.handle_key_event(event, commands, bubble).await,
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

impl SettingsChild for About {
    fn save(&self) -> ChildState {
        ChildState {
            selected: self.list.selected(),
        }
    }
}
