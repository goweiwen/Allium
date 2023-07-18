use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{Label, View};
use tokio::sync::mpsc::Sender;

use crate::view::button_chord::ButtonChordHint;

pub struct Hotkeys {
    rect: Rect,
    global_label: Label<String>,
    global_hotkeys: Vec<ButtonChordHint<String>>,
    ingame_label: Label<String>,
    ingame_hotkeys: Vec<ButtonChordHint<String>>,
    dirty: bool,
}

impl Hotkeys {
    pub fn new(rect: Rect, res: Resources) -> Self {
        let locale = res.get::<Locale>();
        let styles = res.get::<Stylesheet>();

        let x = 100;
        let mut y = 40;

        let global_label = Label::new(
            Point::new(x, y),
            locale.t("hotkeys-global"),
            Alignment::Left,
            None,
        );
        y += styles.ui_font.size as i32 + 8;

        let mut global_hotkeys = Vec::with_capacity(5);
        let global_hotkeys_data = [
            (Key::Power, locale.t("hotkeys-screenshot")),
            (Key::Up, locale.t("hotkeys-brightness-up")),
            (Key::Down, locale.t("hotkeys-brightness-down")),
            (Key::Right, locale.t("hotkeys-volume-up")),
            (Key::Left, locale.t("hotkeys-volume-down")),
        ];
        for (key, label) in global_hotkeys_data {
            global_hotkeys.push(ButtonChordHint::new(
                Point::new(x, y),
                vec![Key::Menu, key],
                label,
                Alignment::Left,
            ));
            y += styles.ui_font.size as i32 + 8;
        }

        y += 16;

        let ingame_label = Label::new(
            Point::new(x, y),
            locale.t("hotkeys-ingame"),
            Alignment::Left,
            None,
        );
        y += styles.ui_font.size as i32 + 8;

        let mut ingame_hotkeys = Vec::with_capacity(2);
        let ingame_hotkeys_data = [
            (Key::Start, locale.t("hotkeys-toggle-aspect-ratio")),
            (Key::X, locale.t("hotkeys-toggle-fps")),
        ];
        for (key, label) in ingame_hotkeys_data {
            ingame_hotkeys.push(ButtonChordHint::new(
                Point::new(x, y),
                vec![Key::Menu, key],
                label,
                Alignment::Left,
            ));
            y += styles.ui_font.size as i32 + 8;
        }

        drop(locale);
        drop(styles);

        Self {
            rect,
            global_label,
            global_hotkeys,
            ingame_label,
            ingame_hotkeys,
            dirty: true,
        }
    }
}

#[async_trait(?Send)]
impl View for Hotkeys {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let mut drawn = false;

        drawn |= self.global_label.draw(display, styles)?;
        for hotkey in self.global_hotkeys.iter_mut() {
            if hotkey.should_draw() {
                drawn |= hotkey.draw(display, styles)?;
            }
        }

        drawn |= self.ingame_label.draw(display, styles)?;
        for hotkey in self.ingame_hotkeys.iter_mut() {
            if hotkey.should_draw() {
                drawn |= hotkey.draw(display, styles)?;
            }
        }

        self.dirty = false;
        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.dirty
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        _bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if matches!(event, KeyEvent::Released(Key::Menu)) {
            commands.send(Command::Exit).await?;
        }
        Ok(true)
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}
