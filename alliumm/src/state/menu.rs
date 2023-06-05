use std::process;

use anyhow::Result;
use embedded_font::FontTextStyleBuilder;
use embedded_graphics::{prelude::*, primitives::Rectangle, text::Alignment};
use strum::{Display, EnumCount, EnumIter, IntoEnumIterator};

use common::display::Display;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use common::{
    constants::{BUTTON_DIAMETER, LISTING_SIZE, SELECTION_HEIGHT, SELECTION_MARGIN},
    retroarch::RetroArchCommand,
};

#[derive(Debug, Clone)]
pub struct MenuState {
    selected: MenuEntry,
}

impl MenuState {
    pub fn new() -> Result<MenuState> {
        Ok(MenuState {
            selected: MenuEntry::Continue,
        })
    }

    pub fn enter(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn leave(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<()> {
        let Size { width, height } = display.size();

        let text_style = FontTextStyleBuilder::new(styles.ui_font.clone())
            .font_size(styles.ui_font_size)
            .text_color(styles.fg_color)
            .build();

        let selection_style = FontTextStyleBuilder::new(styles.ui_font.clone())
            .font_size(styles.ui_font_size)
            .text_color(styles.fg_color)
            .background_color(styles.primary)
            .build();

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
                if self.selected == entry {
                    selection_style.clone()
                } else {
                    text_style.clone()
                },
                Alignment::Left,
                300,
                true,
            )?;
            y += (SELECTION_HEIGHT + SELECTION_MARGIN) as i32;
        }

        // Draw button hints
        let y = height as i32 - BUTTON_DIAMETER as i32 - 8;
        let mut x = width as i32 - 12;

        x = display
            .draw_button_hint(
                Point::new(x, y),
                Key::A,
                text_style.clone(),
                "Select",
                styles,
            )?
            .top_left
            .x
            - 18;
        display.draw_button_hint(Point::new(x, y), Key::B, text_style, "Back", styles)?;

        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        Ok(())
    }

    pub async fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<bool> {
        Ok(match key_event {
            KeyEvent::Released(key) => match key {
                Key::Up => {
                    self.selected = self.selected.prev();
                    true
                }
                Key::Down => {
                    self.selected = self.selected.next();
                    true
                }
                Key::Left => {
                    self.selected = MenuEntry::Continue;
                    true
                }
                Key::Right => {
                    self.selected = MenuEntry::Quit;
                    true
                }
                Key::A => {
                    self.select_entry().await?;
                    true
                }
                Key::B => {
                    self.selected = MenuEntry::Continue;
                    self.select_entry().await?;
                    true
                }
                _ => false,
            },
            _ => false,
        })
    }

    async fn select_entry(&mut self) -> Result<()> {
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
        process::exit(0);
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
