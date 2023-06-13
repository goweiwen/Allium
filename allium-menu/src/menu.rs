use anyhow::Result;
use embedded_graphics::{prelude::*, primitives::Rectangle, text::Alignment};
use strum::{Display, EnumCount, EnumIter, IntoEnumIterator};

use common::display::Display;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use common::{
    constants::{BUTTON_DIAMETER, LISTING_SIZE, SELECTION_HEIGHT, SELECTION_MARGIN},
    retroarch::RetroArchCommand,
};

use crate::command::AlliumMenuCommand;

#[derive(Debug, Clone)]
pub struct Menu {
    selected: MenuEntry,
}

impl Menu {
    pub fn new() -> Result<Menu> {
        Ok(Menu {
            selected: MenuEntry::Continue,
        })
    }

    pub fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<()> {
        let Size { width, height } = display.size();

        // Draw menu
        let (x, mut y) = (24, 58);

        // Clear previous selection
        display.load(Rectangle::new(
            Point::new(x - 12, y - 4),
            Size::new(
                336,
                LISTING_SIZE as u32 * (SELECTION_HEIGHT + SELECTION_MARGIN),
            ),
        ))?;

        for entry in MenuEntry::iter() {
            display.draw_entry(
                Point { x, y },
                &entry.to_string(),
                styles,
                Alignment::Left,
                300,
                self.selected == entry,
                true,
                0,
            )?;
            y += (SELECTION_HEIGHT + SELECTION_MARGIN) as i32;
        }

        // Draw button hints
        let y = height as i32 - BUTTON_DIAMETER as i32 - 8;
        let mut x = width as i32 - 12;

        x = display
            .draw_button_hint(Point::new(x, y), Key::A, "Select", styles, Alignment::Right)?
            .top_left
            .x
            - 18;
        display.draw_button_hint(Point::new(x, y), Key::B, "Back", styles, Alignment::Right)?;

        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        Ok(())
    }

    pub async fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
    ) -> Result<(Option<AlliumMenuCommand>, bool)> {
        Ok(match key_event {
            // we intentionally ignore autorepeat in menu to avoid accidental actions
            KeyEvent::Pressed(Key::Up) => {
                self.selected = self.selected.prev();
                (None, true)
            }
            KeyEvent::Pressed(Key::Down) => {
                self.selected = self.selected.next();
                (None, true)
            }
            KeyEvent::Pressed(Key::Left) => {
                self.selected = MenuEntry::Continue;
                (None, true)
            }
            KeyEvent::Pressed(Key::Right) => {
                self.selected = MenuEntry::Quit;
                (None, true)
            }
            KeyEvent::Pressed(Key::A) => (self.select_entry().await?, true),
            KeyEvent::Pressed(Key::B) => (Some(AlliumMenuCommand::Close), true),
            _ => (None, false),
        })
    }

    async fn select_entry(&mut self) -> Result<Option<AlliumMenuCommand>> {
        match self.selected {
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
        Ok(Some(AlliumMenuCommand::Close))
    }
}

#[derive(Debug, Display, EnumIter, EnumCount, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MenuEntry {
    Continue,
    Save,
    Load,
    Reset,
    Advanced,
    Quit,
}

impl MenuEntry {
    fn next(&self) -> MenuEntry {
        match self {
            MenuEntry::Continue => MenuEntry::Save,
            MenuEntry::Save => MenuEntry::Load,
            MenuEntry::Load => MenuEntry::Reset,
            MenuEntry::Reset => MenuEntry::Advanced,
            MenuEntry::Advanced => MenuEntry::Quit,
            MenuEntry::Quit => MenuEntry::Continue,
        }
    }

    fn prev(&self) -> MenuEntry {
        match self {
            MenuEntry::Continue => MenuEntry::Quit,
            MenuEntry::Save => MenuEntry::Continue,
            MenuEntry::Load => MenuEntry::Save,
            MenuEntry::Reset => MenuEntry::Load,
            MenuEntry::Advanced => MenuEntry::Reset,
            MenuEntry::Quit => MenuEntry::Advanced,
        }
    }
}
