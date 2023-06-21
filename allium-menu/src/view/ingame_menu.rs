use std::collections::VecDeque;
use std::fs;
use std::fs::File;

use anyhow::Result;
use async_trait::async_trait;
use common::battery::Battery;
use common::command::Command;
use common::constants::{ALLIUM_MENU_STATE, BUTTON_DIAMETER};
use common::database::Database;
use common::display::Display;
use common::game_info::GameInfo;
use common::geom::{Alignment, Point, Rect};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::retroarch::RetroArchCommand;
use common::stylesheet::Stylesheet;
use common::view::{BatteryIndicator, ButtonHint, Label, List, Row, View};
use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumIter, IntoEnumIterator};
use tokio::sync::mpsc::Sender;
use tracing::warn;

use crate::view::TextReader;

#[derive(Serialize, Deserialize, Default)]
pub struct IngameMenuState {
    is_text_reader_open: bool,
}

pub struct IngameMenu<B>
where
    B: Battery,
{
    rect: Rect,
    game_info: GameInfo,
    database: Database,
    name: Label<String>,
    battery_indicator: BatteryIndicator<B>,
    menu: List<Label<&'static str>>,
    child: Option<TextReader>,
    button_hints: Row<ButtonHint<&'static str>>,
    dirty: bool,
}

impl<B> IngameMenu<B>
where
    B: Battery,
{
    pub fn new(
        rect: Rect,
        state: IngameMenuState,
        game_info: GameInfo,
        battery: B,
        database: Database,
    ) -> Self {
        let Rect { x, y, w, h } = rect;

        let mut name = Label::new(
            Point::new(x + 12, y + 8),
            game_info.name.to_owned(),
            Alignment::Left,
            None,
        );
        name.color(common::stylesheet::StylesheetColor::Highlight);

        let mut battery_indicator =
            BatteryIndicator::new(Point::new(w as i32 - 12, y + 8), Alignment::Right);
        battery_indicator.init(battery);

        let menu_w = 336;
        let menu = List::new(
            Rect::new(x + 24, y + 58, menu_w, h - 58),
            MenuEntry::iter()
                .filter(|e| match e {
                    MenuEntry::Guide => game_info.guide.is_some(),
                    _ => true,
                })
                .map(|e| Label::new(Point::zero(), e.as_str(), Alignment::Left, Some(menu_w)))
                .collect(),
            Alignment::Left,
            6,
        );

        let button_hints = Row::new(
            Point::new(x + w as i32 - 12, y + h as i32 - BUTTON_DIAMETER as i32 - 8),
            vec![
                ButtonHint::new(Point::zero(), Key::A, "Select", Alignment::Right),
                ButtonHint::new(Point::zero(), Key::B, "Back", Alignment::Right),
            ],
            Alignment::Right,
            12,
        );

        let mut child = None;
        if state.is_text_reader_open {
            if let Some(guide) = game_info.guide.as_ref() {
                child = Some(TextReader::new(rect, guide.to_path_buf(), database.clone()));
            }
        }

        Self {
            rect,
            game_info,
            database,
            name,
            battery_indicator,
            menu,
            child,
            button_hints,
            dirty: false,
        }
    }

    pub fn load_or_new(
        rect: Rect,
        game_info: GameInfo,
        battery: B,
        database: Database,
    ) -> Result<Self> {
        if ALLIUM_MENU_STATE.exists() {
            let file = File::open(ALLIUM_MENU_STATE.as_path())?;
            if let Ok(state) = serde_json::from_reader::<_, IngameMenuState>(file) {
                return Ok(Self::new(rect, state, game_info, battery, database));
            }
            warn!("failed to deserialize state file, deleting");
            fs::remove_file(ALLIUM_MENU_STATE.as_path())?;
        }

        Ok(Self::new(
            rect,
            Default::default(),
            game_info,
            battery,
            database,
        ))
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
        let selected = match self.menu.selected() {
            0 => MenuEntry::Continue,
            1 => MenuEntry::Save,
            2 => MenuEntry::Load,
            3 => MenuEntry::Reset,
            4 => MenuEntry::Advanced,
            5 => {
                if self.game_info.guide.is_some() {
                    MenuEntry::Guide
                } else {
                    MenuEntry::Quit
                }
            }
            6 => MenuEntry::Quit,
            _ => unreachable!(),
        };
        match selected {
            MenuEntry::Continue => {
                commands.send(Command::Exit).await?;
            }
            MenuEntry::Save => {
                RetroArchCommand::SaveState.send().await?;
                commands.send(Command::Exit).await?;
            }
            MenuEntry::Load => {
                RetroArchCommand::LoadState.send().await?;
                commands.send(Command::Exit).await?;
            }
            MenuEntry::Reset => {
                RetroArchCommand::Reset.send().await?;
                commands.send(Command::Exit).await?;
            }
            MenuEntry::Advanced => {
                RetroArchCommand::MenuToggle.send().await?;
                commands.send(Command::Exit).await?;
            }
            MenuEntry::Guide => {
                if let Some(guide) = self.game_info.guide.as_ref() {
                    self.child = Some(TextReader::new(
                        self.rect,
                        guide.to_path_buf(),
                        self.database.clone(),
                    ));
                }
            }
            MenuEntry::Quit => {
                RetroArchCommand::Quit.send().await?;
                commands.send(Command::Exit).await?;
            }
        }
        Ok(true)
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
        self.dirty
            || self.name.should_draw()
            || self.battery_indicator.should_draw()
            || self
                .child
                .as_ref()
                .map_or_else(|| self.menu.should_draw(), |c| c.should_draw())
            || self.button_hints.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        self.name.set_should_draw();
        self.battery_indicator.set_should_draw();
        if let Some(child) = self.child.as_mut() {
            child.set_should_draw();
        } else {
            self.menu.set_should_draw();
        }
        self.button_hints.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if let Some(child) = self.child.as_mut() {
            let mut child_bubble = VecDeque::new();
            if child
                .handle_key_event(event, commands.clone(), &mut child_bubble)
                .await?
            {
                for command in child_bubble {
                    match command {
                        Command::CloseView => {
                            self.child = None;
                            self.set_should_draw();
                        }
                        _ => bubble.push_back(command),
                    }
                }
                return Ok(true);
            }
        }

        match event {
            KeyEvent::Pressed(common::platform::Key::A) => self.select_entry(commands).await,
            event => self.menu.handle_key_event(event, commands, bubble).await,
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

#[derive(Debug, EnumIter, EnumCount, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MenuEntry {
    Continue,
    Save,
    Load,
    Reset,
    Advanced,
    Guide,
    Quit,
}

impl MenuEntry {
    fn as_str(&self) -> &'static str {
        match self {
            MenuEntry::Continue => "Continue",
            MenuEntry::Save => "Save",
            MenuEntry::Load => "Load",
            MenuEntry::Reset => "Reset",
            MenuEntry::Advanced => "Advanced",
            MenuEntry::Guide => "Guide",
            MenuEntry::Quit => "Quit",
        }
    }
}
