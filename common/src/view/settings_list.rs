use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::{Primitive, PrimitiveStyle, Rectangle, RoundedRectangle};
use embedded_graphics::Drawable;
use tokio::sync::mpsc::Sender;

use crate::display::Display;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::{Stylesheet, StylesheetColor};
use crate::view::{Command, Label, View};

/// A listing of selectable entries. Assumes that all entries have the same size.
#[derive(Debug)]
pub struct SettingsList {
    rect: Rect,
    labels: Vec<String>,
    left: Vec<Label<String>>,
    right: Vec<Box<dyn View>>,
    entry_height: u32,
    top: usize,
    selected: usize,
    focused: bool,
    dirty: bool,
}

impl SettingsList {
    pub fn new(
        rect: Rect,
        left: Vec<String>,
        right: Vec<Box<dyn View>>,
        entry_height: u32,
    ) -> Self {
        let mut this = Self {
            rect,
            labels: Vec::new(),
            left: Vec::new(),
            right: Vec::new(),
            entry_height,
            top: 0,
            selected: 0,
            focused: false,
            dirty: true,
        };

        this.set_items(left, right);

        this
    }

    pub fn set_items(&mut self, left: Vec<String>, right: Vec<Box<dyn View>>) {
        self.labels = left;
        self.right = right;
        self.left.clear();

        let mut y = self.rect.y + 8;
        for i in 0..self.visible_count() {
            self.left.push(Label::new(
                Point::new(self.rect.x + 12, y),
                self.labels[i].to_owned(),
                Alignment::Left,
                Some(self.rect.w - 24),
            ));
            y += self.entry_height as i32;
        }
        self.left
            .get_mut(0)
            .map(|c| c.set_background_color(StylesheetColor::Highlight));
        if let Some(c) = self.right.get_mut(0) {
            c.set_background_color(StylesheetColor::Highlight)
        }

        self.top = 0;
        if self.selected >= self.top + self.visible_count() {
            self.top = self.selected;
        } else if self.selected < self.top {
            self.top = self.selected.min(self.labels.len() - self.visible_count());
        }

        self.dirty = true;
    }

    pub fn select(&mut self, index: usize) {
        self.left[self.selected - self.top].set_background_color(StylesheetColor::Background);
        self.right[self.selected].set_background_color(StylesheetColor::Background);

        if index >= self.top + self.visible_count() {
            self.top = index - self.visible_count() + 1;
            self.update_children();
        } else if index < self.top {
            self.top = index;
            self.update_children();
        }

        self.selected = index;

        self.left[self.selected - self.top].set_background_color(StylesheetColor::Highlight);
        self.right[self.selected].set_background_color(StylesheetColor::Highlight);

        self.dirty = true;
    }

    pub fn visible_count(&self) -> usize {
        ((self.rect.h as usize - 16) / self.entry_height as usize)
            .min(self.labels.len())
            .min(self.right.len())
    }

    pub fn set_child(&mut self, index: usize, child: Box<dyn View>) {
        self.right[index] = child;
        self.dirty = true;
    }

    fn update_children(&mut self) {
        for (i, child) in self.left.iter_mut().enumerate() {
            child.set_text(self.labels[self.top + i].to_owned());
        }
    }
}

#[async_trait(?Send)]
impl View for SettingsList {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if self.dirty
            || (self.focused
                && self
                    .right
                    .get(self.selected - self.top)
                    .map(|s| s.should_draw())
                    .unwrap_or(false))
        {
            display.load(self.bounding_box(styles))?;

            let rect = if self.focused {
                self.right
                    .get_mut(self.selected)
                    .map(|s| s.bounding_box(styles))
            } else {
                self.left
                    .get_mut(self.selected - self.top)
                    .map(|s| s.bounding_box(styles))
            }
            .unwrap_or_default();

            RoundedRectangle::with_equal_corners(
                Rectangle::new(
                    embedded_graphics::prelude::Point::new(self.rect.x, rect.y - 4),
                    Size::new(self.rect.w, rect.h + 8),
                ),
                Size::new_equal(rect.h),
            )
            .into_styled(PrimitiveStyle::with_fill(
                styles.highlight_color.blend(styles.background_color, 128),
            ))
            .draw(display)?;

            RoundedRectangle::with_equal_corners(
                Rectangle::new(
                    embedded_graphics::prelude::Point::new(rect.x - 12, rect.y - 4),
                    Size::new(rect.w + 24, rect.h + 8),
                ),
                Size::new_equal(rect.h),
            )
            .into_styled(PrimitiveStyle::with_fill(styles.highlight_color))
            .draw(display)?;

            for child in self.left.iter_mut() {
                child.draw(display, styles)?;
            }

            for i in 0..self.visible_count() {
                self.right[self.top + i].set_position(Point::new(
                    self.rect.x + self.rect.w as i32 - 13,
                    self.rect.y + 8 + i as i32 * self.entry_height as i32,
                ));
                self.right[self.top + i].draw(display, styles)?;
            }

            self.dirty = false;

            return Ok(true);
        }

        let mut drawn = false;
        for child in self.left.iter_mut() {
            if child.should_draw() && child.draw(display, styles)? {
                drawn = true;
            }
        }
        for i in 0..self.visible_count() {
            let child = &mut self.right[self.top + i];
            if child.should_draw() && child.draw(display, styles)? {
                drawn = true;
            }
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.dirty
            || self.left.iter().any(|c| c.should_draw())
            || self.right.iter().any(|c| c.should_draw())
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        self.left.iter_mut().for_each(|c| c.set_should_draw());
        self.right.iter_mut().for_each(|c| c.set_should_draw());
    }
    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        command: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if self.focused {
            if let Some(selected) = self.right.get_mut(self.selected) {
                let mut child_bubble = VecDeque::new();
                if selected
                    .handle_key_event(event, command, &mut child_bubble)
                    .await?
                {
                    while let Some(cmd) = child_bubble.pop_front() {
                        match cmd {
                            Command::TrapFocus => (),
                            Command::Unfocus => {
                                self.focused = false;
                                self.dirty = true;
                            }
                            Command::ValueChanged(_, val) => {
                                bubble.push_back(Command::ValueChanged(self.selected, val))
                            }
                            cmd => bubble.push_back(cmd),
                        }
                    }
                    return Ok(true);
                }
            }
            Ok(false)
        } else if !self.left.is_empty() {
            match event {
                KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                    self.select(
                        (self.selected as isize - 1).rem_euclid(self.right.len() as isize) as usize,
                    );
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                    self.select((self.selected + 1).rem_euclid(self.right.len()));
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                    self.select(
                        (self.selected as isize - 5).clamp(0, self.right.len() as isize - 1)
                            as usize,
                    );
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                    self.select((self.selected + 5).clamp(0, self.right.len() - 1));
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::A) => {
                    if let Some(selected) = self.right.get_mut(self.selected) {
                        let mut child_bubble = VecDeque::new();
                        if selected
                            .handle_key_event(event, command, &mut child_bubble)
                            .await?
                        {
                            while let Some(command) = child_bubble.pop_front() {
                                match command {
                                    Command::TrapFocus => {
                                        self.focused = true;
                                        self.dirty = true;
                                    }
                                    Command::ValueChanged(_, val) => {
                                        bubble.push_back(Command::ValueChanged(self.selected, val))
                                    }
                                    command => bubble.push_back(command),
                                }
                            }
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        self.left
            .iter()
            .map(|c| c as &dyn View)
            .chain(self.right.iter().map(|c| c.as_ref() as &dyn View))
            .collect()
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        self.left
            .iter_mut()
            .map(|c| c as &mut dyn View)
            .chain(self.right.iter_mut().map(|c| c.as_mut() as &mut dyn View))
            .collect()
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, point: Point) {
        self.rect.x = point.x;
        self.rect.y = point.y;
        for (i, child) in self.left.iter_mut().enumerate() {
            child.set_position(Point::new(
                point.x + 12,
                point.y + 8 + i as i32 * self.entry_height as i32,
            ));
        }

        self.dirty = true;
    }
}
