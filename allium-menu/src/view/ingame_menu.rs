use std::collections::VecDeque;
use std::fmt;

use anyhow::Result;
use async_trait::async_trait;
use common::battery::Battery;
use common::command::Command;
use common::constants::BUTTON_DIAMETER;
use common::geom::{Alignment, Point, Rect};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::retroarch::RetroArchCommand;
use common::stylesheet::Stylesheet;
use common::view::{BatteryIndicator, ButtonHint, Label, List, Row, View};
use strum::{EnumCount, EnumIter, FromRepr, IntoEnumIterator};
use tokio::sync::mpsc::Sender;

pub struct IngameMenu<B>
where
    B: Battery,
{
    rect: Rect,
    name: Label<String>,
    battery_indicator: BatteryIndicator<B>,
    menu: List<Label<String>>,
    button_hints: Row<ButtonHint<String>>,
}

impl<B> IngameMenu<B>
where
    B: Battery,
{
    pub fn new(rect: Rect, name: String, battery: B) -> Self {
        let Rect { x, y, w, h } = rect;

        let mut name = Label::new(Point::new(x + 12, y + 8), name, Alignment::Left, None);
        name.color(common::stylesheet::StylesheetColor::Highlight);

        let mut battery_indicator =
            BatteryIndicator::new(Point::new(w as i32 - 12, y + 8), Alignment::Right);
        battery_indicator.init(battery);

        let menu_w = 336;
        let menu = List::new(
            Rect::new(x + 24, y + 58, menu_w, h - 58),
            MenuEntry::iter()
                .map(|e| Label::new(Point::zero(), e.to_string(), Alignment::Left, Some(menu_w)))
                .collect(),
            Alignment::Left,
            6,
        );

        let button_hints = Row::new(
            Point::new(x + w as i32 - 12, y + h as i32 - BUTTON_DIAMETER as i32 - 8),
            vec![
                ButtonHint::new(Point::zero(), Key::A, "Select".to_owned(), Alignment::Right),
                ButtonHint::new(Point::zero(), Key::B, "Back".to_owned(), Alignment::Right),
            ],
            Alignment::Right,
            12,
        );

        Self {
            rect,
            name,
            battery_indicator,
            menu,
            button_hints,
        }
    }

    async fn select_entry(&mut self, commands: Sender<Command>) -> Result<bool> {
        let Some(selected) = MenuEntry::from_repr(self.menu.selected()) else {
            unreachable!("Invalid menu entry selected");
        };
        match selected {
            MenuEntry::Continue => {}
            MenuEntry::Save => {
                RetroArchCommand::SaveState.send().await?;
            }
            MenuEntry::Load => {
                RetroArchCommand::LoadState.send().await?;
            }
            MenuEntry::Reset => {
                RetroArchCommand::Reset.send().await?;
            }
            MenuEntry::Advanced => {
                RetroArchCommand::MenuToggle.send().await?;
            }
            MenuEntry::Quit => {
                RetroArchCommand::Quit.send().await?;
            }
        }
        commands.send(Command::Exit).await?;
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
        if self.name.should_draw() && self.name.draw(display, styles)? {
            drawn = true;
        }
        if self.battery_indicator.should_draw() && self.battery_indicator.draw(display, styles)? {
            drawn = true;
        }
        if self.menu.should_draw() && self.menu.draw(display, styles)? {
            drawn = true;
        }
        if self.button_hints.should_draw() && self.button_hints.draw(display, styles)? {
            drawn = true;
        }
        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.name.should_draw()
            || self.battery_indicator.should_draw()
            || self.menu.should_draw()
            || self.button_hints.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.name.set_should_draw();
        self.battery_indicator.set_should_draw();
        self.menu.set_should_draw();
        self.button_hints.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
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

#[derive(Debug, EnumIter, EnumCount, FromRepr, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MenuEntry {
    Continue,
    Save,
    Load,
    Reset,
    Advanced,
    Quit,
}

impl fmt::Display for MenuEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MenuEntry::Continue => "Continue",
                MenuEntry::Save => "Save",
                MenuEntry::Load => "Load",
                MenuEntry::Reset => "Reset",
                MenuEntry::Advanced => "Advanced",
                MenuEntry::Quit => "Quit",
            }
        )
    }
}
