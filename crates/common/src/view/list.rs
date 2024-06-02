use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::{Primitive, PrimitiveStyle, Rectangle, RoundedRectangle};
use embedded_graphics::Drawable;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::command::Command;
use crate::display::Display;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::View;

/// A listing of selectable entries. Assumes that all entries have the same size.
#[derive(Debug, Serialize, Deserialize)]
pub struct List<V>
where
    V: View,
{
    rect: Rect,
    children: Vec<V>,
    alignment: Alignment,
    margin: u32,
    selected: usize,
    dirty: bool,
    has_layout: bool,
}

impl<V> List<V>
where
    V: View,
{
    pub fn new(rect: Rect, children: Vec<V>, alignment: Alignment, margin: u32) -> Self {
        Self {
            rect,
            children,
            alignment,
            margin,
            selected: 0,
            dirty: true,
            has_layout: false,
        }
    }

    pub fn select(&mut self, index: usize) {
        self.selected = index;
        self.dirty = true;
    }

    pub fn selected(&self) -> usize {
        self.selected
    }
}

// Display is PhantomData, so this is safe.
unsafe impl<V> Send for List<V> where V: View {}

#[async_trait(?Send)]
impl<V> View for List<V>
where
    V: View,
{
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if !self.has_layout {
            let mut y = self.rect.y + 8;
            for child in &mut self.children {
                let rect = child.bounding_box(styles);
                child.set_position(Point::new(self.rect.x + 12, y));
                y += rect.h as i32 + self.margin as i32 + 8;
            }
            self.has_layout = true;
            self.dirty = true;
        }

        if self.dirty {
            display.load(self.bounding_box(styles))?;

            let selected = &mut self.children[self.selected];

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

            for child in &mut self.children.iter_mut() {
                child.draw(display, styles)?;
            }
            return Ok(true);
        }

        let mut drawn = false;
        for child in &mut self.children.iter_mut() {
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
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        _command: Sender<Command>,
        _bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        match event {
            KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                self.select(
                    (self.selected as isize - 1).rem_euclid(self.children.len() as isize) as usize,
                );
                self.dirty = true;
                Ok(true)
            }
            KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                self.select((self.selected + 1).rem_euclid(self.children.len()));
                self.dirty = true;
                Ok(true)
            }
            KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                self.select(
                    (self.selected as isize - 5).clamp(0, self.children.len() as isize - 1)
                        as usize,
                );
                self.dirty = true;
                Ok(true)
            }
            KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                self.select((self.selected + 5).clamp(0, self.children.len() - 1));
                self.dirty = true;
                Ok(true)
            }
            _ => Ok(false),
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
        self.has_layout = false;
        self.dirty = true;
    }
}
