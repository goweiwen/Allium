use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::{
    CornerRadii, Primitive, PrimitiveStyle, Rectangle, RoundedRectangle,
};
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
    background_color: Option<StylesheetColor>,
    dirty: bool,
}

impl ScrollList {
    pub fn new(
        mut rect: Rect,
        items: Vec<String>,
        alignment: Alignment,
        entry_height: u32,
    ) -> Self {
        match alignment {
            Alignment::Left => {}
            Alignment::Center => {
                rect.x += rect.w as i32 / 2;
            }
            Alignment::Right => {
                rect.x = rect.x + rect.w as i32 - 1;
            }
        }
        let mut this = Self {
            rect,
            items: Vec::new(),
            children: Vec::new(),
            alignment,
            entry_height,
            top: 0,
            selected: 0,
            background_color: None,
            dirty: true,
        };

        this.set_items(items, false);

        this
    }

    pub fn set_items(&mut self, items: Vec<String>, preserve_selection: bool) {
        let selected = if preserve_selection {
            self.items
                .get(self.selected)
                .and_then(|selected| items.iter().position(|s| s == selected))
                .unwrap_or(0)
        } else {
            0
        };
        self.items = items;

        self.children.clear();
        let mut y = self.rect.y + 4;
        for i in 0..self.visible_count() {
            self.children.push(Label::new(
                Point::new(self.rect.x + 12 * self.alignment.sign(), y),
                self.items[i].to_owned(),
                self.alignment,
                Some(self.rect.w - 24),
            ));
            y += self.entry_height as i32;
        }

        if let Some(background_color) = self.background_color {
            for child in &mut self.children {
                child.set_background_color(background_color);
            }
        }

        self.select(selected);
        self.update_children();

        self.dirty = true;
    }

    pub fn select(&mut self, index: usize) {
        if self.visible_count() == 0 {
            return;
        }

        self.children.get_mut(self.selected - self.top).map(|c| {
            c.set_background_color(self.background_color.unwrap_or(StylesheetColor::Background))
        });

        if index >= self.top + self.visible_count() {
            self.top = (index - self.visible_count() + 1).min(self.items.len() - 1);
            self.update_children();
        } else if index < self.top {
            self.top = index;
            self.update_children();
        }

        self.selected = index;

        self.children
            .get_mut(self.selected - self.top)
            .map(|c| c.set_background_color(StylesheetColor::Highlight));

        self.dirty = true;
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn visible_count(&self) -> usize {
        (self.rect.h as usize / self.entry_height as usize).min(self.items.len())
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
            if let Some(color) = self.background_color {
                let mut rect = self
                    .children_mut()
                    .iter_mut()
                    .map(|v| v.bounding_box(styles))
                    .reduce(|acc, r| acc.union(&r))
                    .unwrap_or_default();
                rect.x -= 12;
                rect.w += 24;
                rect.y -= 4;
                rect.h += 8;
                RoundedRectangle::new(
                    rect.into(),
                    CornerRadii::new(Size::new_equal((styles.ui_font.size + 8) / 2)),
                )
                .into_styled(PrimitiveStyle::with_fill(color.to_color(styles)))
                .draw(display)?;
            } else {
                display.load(self.bounding_box(styles))?;
            }

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
        match self.alignment {
            Alignment::Left => self.rect,
            Alignment::Center => Rect::new(
                self.rect.x - self.rect.w as i32 / 2,
                self.rect.y,
                self.rect.w,
                self.rect.h,
            ),
            Alignment::Right => Rect::new(
                self.rect.x - self.rect.w as i32 + 1,
                self.rect.y,
                self.rect.w,
                self.rect.h,
            ),
        }
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

    fn set_background_color(&mut self, color: StylesheetColor) {
        self.background_color = Some(color);
        for child in &mut self.children {
            child.set_background_color(color);
        }
    }
}
