use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::command::Command;
use crate::display::Display;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::View;

/// A horizontal row of views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row<V>
where
    V: View,
{
    point: Point,
    children: Vec<V>,
    alignment: Alignment,
    margin: i32,
    dirty: bool,
    has_layout: bool,
}

impl<V> Row<V>
where
    V: View,
{
    pub fn new(point: Point, children: Vec<V>, alignment: Alignment, margin: i32) -> Self {
        Self {
            point,
            children,
            alignment,
            margin,
            dirty: true,
            has_layout: false,
        }
    }

    pub fn get(&self, index: usize) -> Option<&V> {
        self.children.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut V> {
        self.children.get_mut(index)
    }

    fn layout(&mut self, styles: &Stylesheet) {
        match self.alignment {
            Alignment::Left => self.layout_left(styles),
            Alignment::Center => unimplemented!("alignment should be Left or Right"),
            Alignment::Right => self.layout_right(styles),
        }
        self.has_layout = true;
        self.dirty = true;
    }

    fn layout_left(&mut self, styles: &Stylesheet) {
        let mut x = self.point.x;
        for entry in &mut self.children {
            let rect = entry.bounding_box(styles);
            entry.set_position(Point::new(x, self.point.y));
            x += rect.w as i32 + self.margin;
        }
    }

    fn layout_right(&mut self, styles: &Stylesheet) {
        let mut x = self.point.x;
        for entry in self.children.iter_mut() {
            entry.set_position(Point::new(x, self.point.y));
            let rect = entry.bounding_box(styles);
            x -= rect.w as i32 + self.margin;
        }
    }
}

// Display is PhantomData, so this is safe.
unsafe impl<V> Send for Row<V> where V: View {}

#[async_trait(?Send)]
impl<V> View for Row<V>
where
    V: View,
{
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if !self.has_layout {
            self.layout(styles);
        }

        let mut drawn = false;

        if self.dirty {
            display.load(self.bounding_box(styles))?;
            drawn = true;
            self.dirty = false;
        }

        for entry in &mut self.children.iter_mut() {
            if entry.should_draw() && entry.draw(display, styles)? {
                drawn = true;
            }
        }
        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.dirty || self.children.iter().any(|c| c.should_draw())
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        for entry in &mut self.children {
            entry.set_should_draw();
        }
    }

    async fn handle_key_event(
        &mut self,
        _event: KeyEvent,
        _command: Sender<Command>,
        _bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        Ok(false)
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

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        if !self.has_layout {
            self.layout(styles);
        }

        self.children
            .iter_mut()
            .map(|c| c.bounding_box(styles))
            .reduce(|acc, b| acc.union(&b))
            .unwrap_or_default()
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.has_layout = false;
        self.dirty = true;
    }
}
