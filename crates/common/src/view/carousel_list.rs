use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use crate::display::Display;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::resources::Resources;
use crate::stylesheet::{Stylesheet, StylesheetColor};
use crate::view::{Command, View};

/// A horizontal carousel of selectable entries with constant width.
/// Renders overflow elements for seamless appearance.
#[derive(Debug)]
pub struct CarouselList<V>
where
    V: View,
{
    rect: Rect,
    res: Resources,
    /// All entries.
    children: Vec<V>,
    alignment: Alignment,
    entry_width: u32,
    margin: u32,
    left: usize,
    selected: usize,
    background_color: Option<StylesheetColor>,
    dirty: bool,
}

impl<V> CarouselList<V>
where
    V: View,
{
    pub fn new(
        rect: Rect,
        res: Resources,
        children: Vec<V>,
        alignment: Alignment,
        entry_width: u32,
        margin: u32,
    ) -> Self {
        let mut this = Self {
            rect,
            res,
            children,
            alignment,
            entry_width,
            margin,
            left: 0,
            selected: 0,
            background_color: None,
            dirty: true,
        };

        if !this.children.is_empty() {
            this.layout_children();
            // Focus the initially selected child
            if let Some(child) = this.children.get_mut(this.selected) {
                child.focus();
                child.set_should_draw();
            }
            this.dirty = true;
        }

        this
    }

    pub fn set_background_color(&mut self, color: Option<StylesheetColor>) {
        self.background_color = color;
        self.dirty = true;
    }

    pub fn select(&mut self, mut index: usize) {
        if self.children.is_empty() {
            return;
        }

        if let Some(child) = self.children.get_mut(self.selected) {
            child.blur();
        }

        index = index.clamp(0, self.children.len() - 1);
        let old_left = self.left;

        if index >= self.left + self.visible_count() {
            self.left = (index - self.visible_count() + 1).min(self.children.len() - 1);
        } else if index < self.left {
            self.left = index;
        }

        self.selected = index;

        if self.left != old_left {
            self.layout_children();
        }

        if let Some(child) = self.children.get_mut(self.selected) {
            child.focus();
        }

        self.dirty = true;
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn visible_count(&self) -> usize {
        ((self.rect.w as usize) / (self.entry_width as usize + self.margin as usize))
            .min(self.children.len())
    }

    fn layout_children(&mut self) {
        let visible = self.visible_count();
        // Render one extra element for seamless overflow
        let render_count = (visible + 1).min(self.children.len());

        let styles = self.res.get::<Stylesheet>();
        let mut x = self.rect.x + styles.ui.margin_x;
        for i in 0..render_count {
            let item_index = self.left + i;
            if item_index >= self.children.len() {
                break;
            }

            let y = match self.alignment {
                Alignment::Left => self.rect.y,
                Alignment::Center => self.rect.y + self.rect.h as i32 / 2,
                Alignment::Right => self.rect.y + self.rect.h as i32,
            };

            self.children[item_index].set_position(Point::new(x, y));
            x += self.entry_width as i32 + self.margin as i32;
        }
    }

    fn visible_range(&self) -> (usize, usize) {
        let visible = self.visible_count();
        let render_count = (visible + 1).min(self.children.len());
        let end = (self.left + render_count).min(self.children.len());
        (self.left, end)
    }
}

#[async_trait(?Send)]
impl<V> View for CarouselList<V>
where
    V: View,
{
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if self.should_draw() {
            display.load(self.bounding_box(styles))?;

            // Draw optional background
            if let Some(bg_color) = self.background_color {
                let mut pixmap = display.pixmap_mut();
                crate::display::fill_rect(&mut pixmap, self.rect, bg_color.to_color(styles));
            }

            // Draw visible children only
            let (start, end) = self.visible_range();
            for child in &mut self.children[start..end] {
                child.draw(display, styles)?;
            }

            self.dirty = false;

            return Ok(true);
        }

        let mut drawn = false;
        let (start, end) = self.visible_range();
        for child in &mut self.children[start..end] {
            if child.should_draw() && child.draw(display, styles)? {
                drawn = true;
            }
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.dirty || self.children.iter().any(|v| v.should_draw())
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
        if !self.children.is_empty() {
            match event {
                KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                    self.select(self.selected.saturating_sub(1));
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                    self.select((self.selected + 1).min(self.children.len() - 1));
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::L) | KeyEvent::Autorepeat(Key::L) => {
                    self.select(
                        (self.selected as isize - 5).clamp(0, self.children.len() as isize - 1)
                            as usize,
                    );
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::R) | KeyEvent::Autorepeat(Key::R) => {
                    self.select((self.selected + 5).clamp(0, self.children.len() - 1));
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
        self.layout_children();
        self.dirty = true;
    }
}
