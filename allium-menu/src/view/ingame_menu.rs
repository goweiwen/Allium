use std::collections::{HashMap, VecDeque};
use std::fs;
use std::fs::File;

use anyhow::Result;
use async_trait::async_trait;
use common::battery::Battery;
use common::command::Command;
use common::constants::{ALLIUM_MENU_STATE, SELECTION_MARGIN};
use common::display::Display;
use common::game_info::GameInfo;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::retroarch::RetroArchCommand;
use common::stylesheet::Stylesheet;
use common::view::{
    BatteryIndicator, ButtonHint, ButtonIcon, Label, NullView, Row, SettingsList, View,
};
use log::warn;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::retroarch_info::RetroArchInfo;
use crate::view::text_reader::TextReader;

#[derive(Serialize, Deserialize, Default)]
pub struct IngameMenuState {
    is_text_reader_open: bool,
}

pub struct IngameMenu<B>
where
    B: Battery + 'static,
{
    rect: Rect,
    res: Resources,
    name: Label<String>,
    battery_indicator: BatteryIndicator<B>,
    menu: SettingsList,
    child: Option<TextReader>,
    button_hints: Row<ButtonHint<String>>,
    entries: Vec<MenuEntry>,
    info: Option<RetroArchInfo>,
    dirty: bool,
}

impl<B> IngameMenu<B>
where
    B: Battery + 'static,
{
    pub fn new(
        rect: Rect,
        state: IngameMenuState,
        res: Resources,
        battery: B,
        info: Option<RetroArchInfo>,
    ) -> Self {
        let Rect { x, y, w, h } = rect;

        let game_info = res.get::<GameInfo>();
        let locale = res.get::<Locale>();
        let styles = res.get::<Stylesheet>();

        let mut name = Label::new(
            Point::new(x + 12, y + 8),
            game_info.name.clone(),
            Alignment::Left,
            None,
        );
        name.color(common::stylesheet::StylesheetColor::Highlight);

        let battery_indicator = BatteryIndicator::new(Point::new(w as i32 - 12, y + 8), battery);

        let entries = MenuEntry::entries(&info);
        let mut menu = SettingsList::new(
            Rect::new(
                x + 24,
                y + 8 + styles.ui_font.size as i32 + 8,
                w / 2 - 48,
                h - 8 - styles.ui_font.size - 8,
            ),
            entries.iter().map(|e| e.as_str(&locale)).collect(),
            entries
                .iter()
                .map(|_| Box::new(NullView) as Box<dyn View>)
                .collect(),
            styles.ui_font.size + SELECTION_MARGIN,
        );
        if let Some(info) = info.as_ref() {
            if info.max_disk_slots > 1 && !state.is_text_reader_open {
                let mut map = HashMap::new();
                map.insert("disk".to_string(), (info.disk_slot + 1).into());
                menu.set_right(
                    MenuEntry::Continue as usize,
                    Box::new(Label::new(
                        Point::zero(),
                        locale.ta("ingame-menu-disk", &map),
                        Alignment::Right,
                        None,
                    )),
                );
            }
        }

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

        let mut child = None;
        if state.is_text_reader_open {
            if let Some(guide) = game_info.guide.as_ref() {
                menu.select(MenuEntry::Guide as usize);
                child = Some(TextReader::new(rect, res.clone(), guide.clone()));
            }
        }

        drop(game_info);
        drop(locale);
        drop(styles);

        Self {
            rect,
            res,
            name,
            battery_indicator,
            menu,
            child,
            button_hints,
            entries,
            info,
            dirty: false,
        }
    }

    pub async fn load_or_new(
        rect: Rect,
        res: Resources,
        battery: B,
        info: Option<RetroArchInfo>,
    ) -> Result<Self> {
        if ALLIUM_MENU_STATE.exists() {
            let file = File::open(ALLIUM_MENU_STATE.as_path())?;
            if let Ok(state) = serde_json::from_reader::<_, IngameMenuState>(file) {
                return Ok(Self::new(rect, state, res, battery, info));
            }
            warn!("failed to deserialize state file, deleting");
            fs::remove_file(ALLIUM_MENU_STATE.as_path())?;
        }

        Ok(Self::new(rect, Default::default(), res, battery, info))
    }

    pub fn save(&self) -> Result<()> {
        let file = File::create(ALLIUM_MENU_STATE.as_path())?;
        let state = IngameMenuState {
            is_text_reader_open: self.child.is_some(),
        };
        if let Some(child) = self.child.as_ref() {
            child.save_cursor();
        }
        serde_json::to_writer(file, &state)?;
        Ok(())
    }

    async fn select_entry(&mut self, commands: Sender<Command>) -> Result<bool> {
        let selected = self.entries[self.menu.selected()];
        match selected {
            MenuEntry::Continue => {
                commands.send(Command::Exit).await?;
            }
            MenuEntry::Save => {
                RetroArchCommand::SaveStateSlot(self.info.as_ref().unwrap().state_slot.unwrap())
                    .send()
                    .await?;
                commands.send(Command::Exit).await?;
            }
            MenuEntry::Load => {
                RetroArchCommand::LoadStateSlot(self.info.as_ref().unwrap().state_slot.unwrap())
                    .send()
                    .await?;
                commands.send(Command::Exit).await?;
            }
            MenuEntry::Reset => {
                RetroArchCommand::Reset.send().await?;
                commands.send(Command::Exit).await?;
            }
            MenuEntry::Guide => {
                if let Some(guide) = self.res.get::<GameInfo>().guide.as_ref() {
                    self.child = Some(TextReader::new(self.rect, self.res.clone(), guide.clone()));
                }
            }
            MenuEntry::Settings => {
                RetroArchCommand::Unpause.send().await?;
                RetroArchCommand::MenuToggle.send().await?;
                commands.send(Command::Exit).await?;
            }
            MenuEntry::Quit => {
                if self.info.is_some() {
                    RetroArchCommand::Quit.send().await?;
                } else {
                    tokio::process::Command::new("pkill")
                        .arg("retroarch")
                        .spawn()?
                        .wait()
                        .await?;
                }
                commands.send(Command::Exit).await?;
            }
        }
        Ok(true)
    }

    fn update_state_slot_label(&mut self, state_slot: i8) {
        if state_slot == -1 {
            self.menu.set_right(
                self.menu.selected(),
                Box::new(Label::new(
                    Point::zero(),
                    self.res.get::<Locale>().t("ingame-menu-slot-auto"),
                    Alignment::Right,
                    None,
                )),
            );
        } else {
            let mut map = HashMap::new();
            map.insert("slot".to_string(), state_slot.into());
            self.menu.set_right(
                self.menu.selected(),
                Box::new(Label::new(
                    Point::zero(),
                    self.res.get::<Locale>().ta("ingame-menu-slot", &map),
                    Alignment::Right,
                    None,
                )),
            );
        }
    }
}

#[async_trait(?Send)]
impl<B> View for IngameMenu<B>
where
    B: Battery,
{
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let mut drawn = false;

        if self.dirty {
            display.load(self.rect)?;
            self.dirty = false;
        }

        if let Some(child) = self.child.as_mut() {
            drawn |= child.should_draw() && child.draw(display, styles)?;
        } else {
            drawn |= self.name.should_draw() && self.name.draw(display, styles)?;
            drawn |= self.battery_indicator.should_draw()
                && self.battery_indicator.draw(display, styles)?;
            drawn |= self.menu.should_draw() && self.menu.draw(display, styles)?;
            drawn |= self.button_hints.should_draw() && self.button_hints.draw(display, styles)?;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        if let Some(child) = self.child.as_ref() {
            self.dirty || child.should_draw()
        } else {
            self.dirty
                || self.name.should_draw()
                || self.battery_indicator.should_draw()
                || self.menu.should_draw()
                || self.button_hints.should_draw()
        }
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        if let Some(child) = self.child.as_mut() {
            child.set_should_draw();
        } else {
            self.name.set_should_draw();
            self.battery_indicator.set_should_draw();
            self.menu.set_should_draw();
            self.button_hints.set_should_draw();
        }
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if let Some(child) = self.child.as_mut() {
            if child
                .handle_key_event(event, commands.clone(), bubble)
                .await?
            {
                bubble.retain(|cmd| match cmd {
                    Command::CloseView => {
                        self.child = None;
                        self.set_should_draw();
                        false
                    }
                    _ => true,
                });
                return Ok(true);
            }
        }

        let selected = self.menu.selected();

        // Handle disk slot selection
        if let Some(info) = self.info.as_mut() {
            if info.max_disk_slots > 1 && selected == MenuEntry::Continue as usize {
                match event {
                    KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                        info.disk_slot = info.disk_slot.saturating_sub(1);
                        RetroArchCommand::SetDiskSlot(info.disk_slot).send().await?;

                        let mut map = HashMap::new();
                        map.insert("disk".to_string(), (info.disk_slot + 1).into());
                        self.menu.set_right(
                            self.menu.selected(),
                            Box::new(Label::new(
                                Point::zero(),
                                self.res.get::<Locale>().ta("ingame-menu-disk", &map),
                                Alignment::Right,
                                None,
                            )),
                        );
                        return Ok(true);
                    }
                    KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                        info.disk_slot = (info.disk_slot + 1).min(info.max_disk_slots - 1);
                        RetroArchCommand::SetDiskSlot(info.disk_slot).send().await?;

                        let mut map = HashMap::new();
                        map.insert("disk".to_string(), (info.disk_slot + 1).into());
                        self.menu.set_right(
                            self.menu.selected(),
                            Box::new(Label::new(
                                Point::zero(),
                                self.res.get::<Locale>().ta("ingame-menu-disk", &map),
                                Alignment::Right,
                                None,
                            )),
                        );
                        return Ok(true);
                    }
                    _ => {}
                }
            }

            // Handle state slot selection
            if let Some(state_slot) = info.state_slot.as_mut() {
                if selected == MenuEntry::Save as usize || selected == MenuEntry::Load as usize {
                    match event {
                        KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                            *state_slot = (*state_slot - 1).max(-1);
                            let state_slot = *state_slot;
                            RetroArchCommand::SetStateSlot(state_slot).send().await?;
                            self.update_state_slot_label(state_slot);
                            return Ok(true);
                        }
                        KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                            *state_slot = state_slot.saturating_add(1);
                            let state_slot = *state_slot;
                            RetroArchCommand::SetStateSlot(state_slot).send().await?;
                            self.update_state_slot_label(state_slot);
                            return Ok(true);
                        }
                        _ => {}
                    }
                }
            }
        }

        match event {
            KeyEvent::Pressed(Key::A) => self.select_entry(commands).await,
            KeyEvent::Pressed(Key::Left | Key::Right)
            | KeyEvent::Autorepeat(Key::Left | Key::Right) => {
                // Don't scroll with left/right
                Ok(true)
            }
            event => {
                let prev = self.menu.selected();
                let consumed = self.menu.handle_key_event(event, commands, bubble).await?;
                let curr = self.menu.selected();
                if consumed && prev != curr {
                    if let Some(info) = self.info.as_ref() {
                        if info.max_disk_slots > 1 {
                            if prev == MenuEntry::Continue as usize {
                                self.menu.set_right(prev, Box::new(NullView));
                            }
                            if curr == MenuEntry::Continue as usize {
                                let mut map = HashMap::new();
                                map.insert("disk".to_string(), (info.disk_slot + 1).into());
                                self.menu.set_right(
                                    curr,
                                    Box::new(Label::new(
                                        Point::zero(),
                                        self.res.get::<Locale>().ta("ingame-menu-disk", &map),
                                        Alignment::Right,
                                        None,
                                    )),
                                );
                            }
                        }

                        if let Some(state_slot) = info.state_slot {
                            if prev == MenuEntry::Save as usize || prev == MenuEntry::Load as usize
                            {
                                self.menu.set_right(prev, Box::new(NullView));
                            }
                            if curr == MenuEntry::Save as usize || curr == MenuEntry::Load as usize
                            {
                                self.update_state_slot_label(state_slot);
                            }
                        }
                    }
                }
                Ok(consumed)
            }
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![
            &self.name,
            &self.battery_indicator,
            &self.menu,
            &self.button_hints,
        ]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![
            &mut self.name,
            &mut self.battery_indicator,
            &mut self.menu,
            &mut self.button_hints,
        ]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, point: Point) {
        self.rect.x = point.x;
        self.rect.y = point.y;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MenuEntry {
    Continue,
    Save,
    Load,
    Reset,
    Guide,
    Settings,
    Quit,
}

impl MenuEntry {
    fn as_str(&self, locale: &Locale) -> String {
        match self {
            MenuEntry::Continue => locale.t("ingame-menu-continue"),
            MenuEntry::Save => locale.t("ingame-menu-save"),
            MenuEntry::Load => locale.t("ingame-menu-load"),
            MenuEntry::Reset => locale.t("ingame-menu-reset"),
            MenuEntry::Guide => locale.t("ingame-menu-guide"),
            MenuEntry::Settings => locale.t("ingame-menu-settings"),
            MenuEntry::Quit => locale.t("ingame-menu-quit"),
        }
    }

    fn entries(info: &Option<RetroArchInfo>) -> Vec<Self> {
        match info {
            Some(RetroArchInfo {
                state_slot: Some(_),
                ..
            }) => vec![
                MenuEntry::Continue,
                MenuEntry::Save,
                MenuEntry::Load,
                MenuEntry::Reset,
                MenuEntry::Guide,
                MenuEntry::Settings,
                MenuEntry::Quit,
            ],
            Some(_) => vec![
                MenuEntry::Continue,
                MenuEntry::Reset,
                MenuEntry::Guide,
                MenuEntry::Settings,
                MenuEntry::Quit,
            ],
            None => vec![MenuEntry::Continue, MenuEntry::Guide, MenuEntry::Quit],
        }
    }
}
