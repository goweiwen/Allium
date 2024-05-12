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
    background_color: Option<StylesheetColor>,
    focused: bool,
    dirty: bool,
    has_layout: bool,
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
            background_color: None,
            dirty: true,
            has_layout: false,
        };

        this.set_items(left, right);

        this
    }

    pub fn set_background_color(&mut self, color: Option<StylesheetColor>) {
        self.background_color = color;
        self.dirty = true;
    }

    pub fn set_items(&mut self, left: Vec<String>, right: Vec<Box<dyn View>>) {
        self.labels = left;
        self.right = right;
        self.left.clear();

        let mut y = self.rect.y + 4;
        for i in 0..self.visible_count() {
            self.left.push(Label::new(
                Point::new(self.rect.x + 12, y),
                self.labels[i].to_owned(),
                Alignment::Left,
                Some((self.rect.w - 24) * 2 / 3),
            ));
            y += self.entry_height as i32;
        }

        self.top = 0;
        if self.selected >= self.top + self.visible_count() {
            self.top = self.selected;
        } else if self.selected < self.top {
            self.top = self.selected.min(self.labels.len() - self.visible_count());
        }

        self.has_layout = false;
        self.dirty = true;
    }

    pub fn set_right(&mut self, i: usize, right: Box<dyn View>) {
        self.right[i] = right;
        self.has_layout = false;
        self.dirty = true;
    }

    pub fn select(&mut self, index: usize) {
        if index >= self.top + self.visible_count() {
            self.top = index - self.visible_count() + 1;
            self.update_children();
            self.has_layout = false;
        } else if index < self.top {
            self.top = index;
            self.update_children();
            self.has_layout = false;
        }

        self.selected = index;

        self.dirty = true;
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn left(&self, i: usize) -> &str {
        &self.labels[i]
    }

    pub fn left_mut(&mut self, i: usize) -> &mut Label<String> {
        &mut self.left[i]
    }

    pub fn right(&self, i: usize) -> &dyn View {
        &self.right[i]
    }

    pub fn right_mut(&mut self, i: usize) -> &mut dyn View {
        &mut self.right[i]
    }

    pub fn visible_count(&self) -> usize {
        (self.rect.h as usize / self.entry_height as usize)
            .min(self.labels.len())
            .min(self.right.len())
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
        let mut drawn = false;

        if self.dirty {
            if !self.has_layout {
                for i in 0..self.visible_count() {
                    let child = &mut self.right[self.top + i];
                    child.set_position(Point::new(
                        self.rect.x + self.rect.w as i32 - 13,
                        self.rect.y + 4 + i as i32 * self.entry_height as i32,
                    ));
                    self.has_layout = true;
                }
            }

            display.load(self.bounding_box(styles))?;

            if !self.focused {
                let left = self
                    .left
                    .get_mut(self.selected - self.top)
                    .map(|s| s.bounding_box(styles))
                    .unwrap_or_default();
                let right = self
                    .right
                    .get_mut(self.selected)
                    .map(|s| s.bounding_box(styles))
                    .unwrap_or_default();

                // Highlight Background
                if right.w != 0 && right.h != 0 {
                    let rect = left.union(&right);
                    RoundedRectangle::with_equal_corners(
                        Rectangle::new(
                            embedded_graphics::prelude::Point::new(self.rect.x, rect.y - 4),
                            Size::new(self.rect.w, rect.h + 8),
                        ),
                        Size::new_equal(rect.h),
                    )
                    .into_styled(PrimitiveStyle::with_fill(
                        styles.highlight_color.with_a(128),
                    ))
                    .draw(display)?;
                }

                // Highlight
                let rect = if self.focused { right } else { left };
                RoundedRectangle::with_equal_corners(
                    Rectangle::new(
                        embedded_graphics::prelude::Point::new(rect.x - 12, rect.y - 4),
                        Size::new(rect.w + 24, rect.h + 8),
                    ),
                    Size::new_equal(rect.h),
                )
                .into_styled(PrimitiveStyle::with_fill(styles.highlight_color))
                .draw(display)?;

                for (i, left) in self.left.iter_mut().enumerate() {
                    left.set_should_draw();
                    let right = &mut self.right[self.top + i];
                    right.set_should_draw();
                }
            }
        }

        for (i, left) in self.left.iter_mut().enumerate() {
            let mut drawn_left = false;
            if (left.should_draw() || self.focused) && left.draw(display, styles)? {
                drawn = true;
                drawn_left = true;
            }
            let right = &mut self.right[self.top + i];
            if (drawn_left || right.should_draw()) && right.draw(display, styles)? {
                drawn = true;
            }
        }

        if self.focused {
            let right = &mut self.right[self.selected];
            right.set_should_draw();

            let left = self.left.get_mut(self.selected - self.top).unwrap();
            let left_rect = left.bounding_box(styles);
            let right_rect = right.bounding_box(styles);

            // Highlight Background
            if right_rect.w != 0 && right_rect.h != 0 {
                let rect = left_rect.union(&right_rect);
                RoundedRectangle::with_equal_corners(
                    Rectangle::new(
                        embedded_graphics::prelude::Point::new(self.rect.x, rect.y - 4),
                        Size::new(self.rect.w, rect.h + 8),
                    ),
                    Size::new_equal(rect.h),
                )
                .into_styled(PrimitiveStyle::with_fill(
                    styles.highlight_color.with_a(128),
                ))
                .draw(display)?;
            }

            // Highlight
            RoundedRectangle::with_equal_corners(
                Rectangle::new(
                    embedded_graphics::prelude::Point::new(right_rect.x - 12, right_rect.y - 4),
                    Size::new(right_rect.w + 24, right_rect.h + 8),
                ),
                Size::new_equal(right_rect.h),
            )
            .into_styled(PrimitiveStyle::with_fill(styles.highlight_color))
            .draw(display)?;

            left.draw(display, styles)?;
            right.draw(display, styles)?;
            drawn = true;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.dirty
            || self.left.iter().any(|c| c.should_draw())
            || self
                .right
                .iter()
                .skip(self.top)
                .take(self.visible_count())
                .any(|c| c.should_draw())
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        self.left.iter_mut().for_each(|c| c.set_should_draw());
        let visible_count = self.visible_count();
        self.right
            .iter_mut()
            .skip(self.top)
            .take(visible_count)
            .for_each(|c| c.set_should_draw());
    }
    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        command: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if self.focused {
            if let Some(selected) = self.right.get_mut(self.selected) {
                if selected.handle_key_event(event, command, bubble).await? {
                    bubble.retain_mut(|cmd| match cmd {
                        Command::TrapFocus => false,
                        Command::Unfocus => {
                            self.focused = false;
                            self.dirty = true;
                            false
                        }
                        Command::ValueChanged(i, _) => {
                            *i = self.selected;
                            true
                        }
                        _ => true,
                    });
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
                KeyEvent::Pressed(Key::L) | KeyEvent::Autorepeat(Key::L) => {
                    self.select(
                        (self.selected as isize - 5).clamp(0, self.right.len() as isize - 1)
                            as usize,
                    );
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::R) | KeyEvent::Autorepeat(Key::R) => {
                    self.select((self.selected + 5).clamp(0, self.right.len() - 1));
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::A) => {
                    if let Some(selected) = self.right.get_mut(self.selected) {
                        if selected.handle_key_event(event, command, bubble).await? {
                            bubble.retain_mut(|cmd| match cmd {
                                Command::TrapFocus => {
                                    self.focused = true;
                                    self.dirty = true;
                                    false
                                }
                                Command::ValueChanged(i, _) => {
                                    *i = self.selected;
                                    true
                                }
                                _ => true,
                            });
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
