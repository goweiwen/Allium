use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::{Primitive, PrimitiveStyle, Rectangle, RoundedRectangle};
use embedded_graphics::Drawable;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::display::Display;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::{Stylesheet, StylesheetColor};
use crate::view::{Command, Label, View};

/// A listing of selectable entries. Assumes that all entries have the same size.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScrollList {
    rect: Rect,
    /// All entries.
    items: Vec<String>,
    /// Visible entries.
    children: Vec<Label<String>>,
    alignment: Alignment,
    entry_height: u32,
    top: usize,
    selected: usize,
    dirty: bool,
}

impl ScrollList {
    pub fn new(rect: Rect, items: Vec<String>, alignment: Alignment, entry_height: u32) -> Self {
        let mut this = Self {
            rect,
            items: Vec::new(),
            children: Vec::new(),
            alignment,
            entry_height,
            top: 0,
            selected: 0,
            dirty: true,
        };

        this.set_items(items, false);

        this
    }

    pub fn set_items(&mut self, items: Vec<String>, preserve_selection: bool) {
        self.items = items;
        self.children.clear();

        let mut y = self.rect.y + 8;
        for i in 0..self.visible_count() {
            self.children.push(Label::new(
                Point::new(self.rect.x + 12, y),
                self.items[i].to_owned(),
                self.alignment,
                Some(self.rect.w - 24),
            ));
            y += self.entry_height as i32;
        }

        self.selected = if preserve_selection {
            self.items
                .get(self.selected)
                .and_then(|selected| self.items.iter().position(|s| s == selected))
                .unwrap_or(0)
        } else {
            0
        };

        self.children
            .get_mut(self.selected)
            .map(|c| c.set_background_color(StylesheetColor::Highlight));

        self.top = 0;
        if self.selected >= self.top + self.visible_count() {
            self.top = self.selected;
        } else if self.selected < self.top {
            self.top = self.selected.min(self.items.len() - self.visible_count());
        }

        self.dirty = true;
    }

    pub fn select(&mut self, index: usize) {
        self.children[self.selected - self.top].set_background_color(StylesheetColor::Background);

        if index >= self.top + self.visible_count() {
            self.top = index - self.visible_count() + 1;
            self.update_children();
        } else if index < self.top {
            self.top = index;
            self.update_children();
        }

        self.selected = index;

        self.children[self.selected - self.top].set_background_color(StylesheetColor::Highlight);

        self.dirty = true;
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn visible_count(&self) -> usize {
        ((self.rect.h as usize - 16) / self.entry_height as usize).min(self.items.len())
    }

    fn update_children(&mut self) {
        for (i, child) in self.children.iter_mut().enumerate() {
            child.set_text(self.items[self.top + i].to_owned());
        }
    }
}

#[async_trait(?Send)]
impl View for ScrollList {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if self.dirty {
            display.load(self.bounding_box(styles))?;

            if let Some(selected) = self.children.get_mut(self.selected - self.top) {
                let rect = selected.bounding_box(styles);

                let fill_style = PrimitiveStyle::with_fill(styles.highlight_color);
                RoundedRectangle::with_equal_corners(
                    Rectangle::new(
                        embedded_graphics::prelude::Point::new(rect.x - 12, rect.y - 4),
                        Size::new(rect.w + 24, rect.h + 8),
                    ),
                    Size::new_equal(rect.h),
                )
                .into_styled(fill_style)
                .draw(display)?;
            }

            for child in self.children.iter_mut() {
                child.draw(display, styles)?;
            }

            self.dirty = false;

            return Ok(true);
        }

        let mut drawn = false;
        for child in self.children.iter_mut() {
            if child.should_draw() && child.draw(display, styles)? {
                drawn = true;
            }
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.dirty
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        for entry in &mut self.children {
            entry.set_should_draw();
        }
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        _command: Sender<Command>,
        _bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if !self.items.is_empty() {
            match event {
                KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                    self.select(
                        (self.selected as isize - 1).rem_euclid(self.items.len() as isize) as usize,
                    );
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                    self.select((self.selected + 1).rem_euclid(self.items.len()));
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                    self.select(
                        (self.selected as isize - 5).clamp(0, self.items.len() as isize - 1)
                            as usize,
                    );
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                    self.select((self.selected + 5).clamp(0, self.items.len() - 1));
                    self.dirty = true;
                    Ok(true)
                }
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        self.children.iter().map(|c| c as &dyn View).collect()
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        self.children
            .iter_mut()
            .map(|c| c as &mut dyn View)
            .collect()
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, point: Point) {
        self.rect.x = point.x;
        self.rect.y = point.y;
        for (i, child) in self.children.iter_mut().enumerate() {
            child.set_position(Point::new(
                point.x + 12,
                point.y + 8 + i as i32 * self.entry_height as i32,
            ));
        }

        self.dirty = true;
    }
}
