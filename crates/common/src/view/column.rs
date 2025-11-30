use std::collections::VecDeque;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::command::Command;
use crate::geom::{Point, Rect};
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::View;

/// A vertical column of views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column<V>
where
    V: View,
{
    point: Point,
    children: Vec<V>,
    margin: i32,
}

impl<V> Column<V>
where
    V: View,
{
    pub fn new(point: Point, children: Vec<V>, margin: i32) -> Self {
        Self {
            point,
            children,
            margin,
        }
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&V> {
        self.children.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut V> {
        self.children.get_mut(index)
    }

    pub fn push(&mut self, view: V) {
        self.children.push(view);
        self.set_should_draw();
    }

    pub fn pop(&mut self) -> Option<V> {
        let view = self.children.pop();
        self.set_should_draw();
        view
    }

    pub fn remove(&mut self, index: usize) -> Option<V> {
        if index >= self.children.len() {
            return None;
        }
        let view = self.children.remove(index);
        self.set_should_draw();
        Some(view)
    }

    pub fn insert(&mut self, index: usize, view: V) {
        self.children.insert(index, view);
        self.set_should_draw();
    }

    fn layout(&mut self, styles: &Stylesheet) {
        let mut y = self.point.y;
        for entry in &mut self.children {
            let rect = entry.bounding_box(styles);
            entry.set_position(Point::new(self.point.x, y));
            if rect.h > 0 {
                y += rect.h as i32 + self.margin;
            }
        }
    }
}

// Display is PhantomData, so this is safe.
unsafe impl<V> Send for Column<V> where V: View {}

#[async_trait(?Send)]
impl<V> View for Column<V>
where
    V: View,
{
    fn update(&mut self, dt: Duration) {
        for child in self.children_mut() {
            child.update(dt);
        }
    }

    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        self.layout(styles);

        for entry in &mut self.children.iter_mut() {
            entry.draw(display, styles)?;
        }

        Ok(true)
    }

    fn should_draw(&self) -> bool {
        self.children.iter().any(|c| c.should_draw())
    }

    fn set_should_draw(&mut self) {
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
        self.layout(styles);
        self.children
            .iter_mut()
            .map(|c| c.bounding_box(styles))
            .reduce(|acc, b| acc.union(&b))
            .unwrap_or_default()
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.set_should_draw();
    }
}
