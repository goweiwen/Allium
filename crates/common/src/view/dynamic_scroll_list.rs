use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use crate::command::Command;
use crate::display::Display;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::View;

/// A listing of selectable entries with dynamic heights. Only renders visible entries.
#[derive(Debug)]
pub struct DynamicScrollList<V>
where
    V: View,
{
    rect: Rect,
    children: Vec<V>,
    alignment: Alignment,
    margin: u32,
    /// Cached heights per entry
    entry_heights: Vec<u32>,
    /// Cumulative heights for O(1) range queries
    cumulative_heights: Vec<u32>,
    /// Pixel offset from top
    scroll_offset: u32,
    selected: usize,
    dirty: bool,
    has_layout: bool,
}

impl<V> DynamicScrollList<V>
where
    V: View,
{
    pub fn new(
        rect: Rect,
        children: Vec<V>,
        alignment: Alignment,
        margin: u32,
    ) -> Self {
        let len = children.len();
        Self {
            rect,
            children,
            alignment,
            margin,
            entry_heights: vec![0; len],
            cumulative_heights: vec![0; len + 1],
            scroll_offset: 0,
            selected: 0,
            dirty: true,
            has_layout: false,
        }
    }

    pub fn select(&mut self, index: usize) {
        if self.children.is_empty() {
            return;
        }

        let old_selected = self.selected;
        self.selected = index.clamp(0, self.children.len() - 1);

        if old_selected != self.selected {
            self.adjust_scroll();
            self.dirty = true;
        }
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Recalculate heights from current children. Call when child content changes.
    pub fn recalculate_heights(&mut self, styles: &Stylesheet) {
        self.entry_heights.clear();
        self.entry_heights.reserve(self.children.len());

        for child in &mut self.children {
            let height = child.bounding_box(styles).h;
            self.entry_heights.push(height);
        }

        self.update_cumulative_heights(styles);
        self.has_layout = false;
        self.dirty = true;
    }

    fn update_cumulative_heights(&mut self, styles: &Stylesheet) {
        self.cumulative_heights.clear();
        self.cumulative_heights.push(0);

        let mut sum = 0u32;
        for &height in &self.entry_heights {
            sum += height + self.margin + styles.ui.margin_y as u32;
            self.cumulative_heights.push(sum);
        }
    }

    fn adjust_scroll(&mut self) {
        if self.children.is_empty() || self.cumulative_heights.len() <= 1 {
            return;
        }

        let selected_top = self.cumulative_heights[self.selected];
        let selected_bottom = self.cumulative_heights[self.selected + 1];

        // Scroll down if selected is below viewport
        if selected_bottom > self.scroll_offset + self.rect.h {
            self.scroll_offset = selected_bottom.saturating_sub(self.rect.h);
        }
        // Scroll up if selected is above viewport
        else if selected_top < self.scroll_offset {
            self.scroll_offset = selected_top;
        }
    }

    fn visible_range(&self) -> (usize, usize) {
        if self.children.is_empty() {
            return (0, 0);
        }

        let viewport_top = self.scroll_offset;
        let viewport_bottom = self.scroll_offset + self.rect.h;

        // Binary search for first visible
        let start = self
            .cumulative_heights
            .partition_point(|&h| h < viewport_top)
            .saturating_sub(1)
            .min(self.children.len());

        // Binary search for last visible
        let end = self
            .cumulative_heights
            .partition_point(|&h| h < viewport_bottom)
            .min(self.children.len());

        (start, end)
    }

    fn layout(&mut self, styles: &Stylesheet) {
        if self.has_layout {
            return;
        }

        // Calculate heights if not yet done
        if self.entry_heights.iter().all(|&h| h == 0) {
            self.recalculate_heights(styles);
        }

        // Position all children
        let mut y = self.rect.y + styles.ui.margin_y;
        for (i, child) in self.children.iter_mut().enumerate() {
            let x = match self.alignment {
                Alignment::Left => self.rect.x + styles.ui.margin_x,
                Alignment::Center => self.rect.x + self.rect.w as i32 / 2,
                Alignment::Right => self.rect.x + self.rect.w as i32 - styles.ui.margin_x,
            };

            let offset_y = y - self.scroll_offset as i32;
            child.set_position(Point::new(x, offset_y));

            y += self.entry_heights[i] as i32 + self.margin as i32 + styles.ui.margin_y;
        }

        self.has_layout = true;
    }
}

// Display is PhantomData, so this is safe.
unsafe impl<V> Send for DynamicScrollList<V> where V: View {}

#[async_trait(?Send)]
impl<V> View for DynamicScrollList<V>
where
    V: View,
{
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        self.layout(styles);

        if self.dirty {
            display.load(self.bounding_box(styles))?;

            // Draw highlight for selected
            if !self.children.is_empty() {
                let selected = &mut self.children[self.selected];
                let rect = selected.bounding_box(styles);

                let highlight_rect = Rect::new(
                    rect.x - styles.ui.margin_x,
                    rect.y - styles.ui.margin_y / 2,
                    rect.w + styles.ui.margin_x as u32 * 2,
                    rect.h + styles.ui.margin_y as u32,
                );
                crate::display::fill_rounded_rect(
                    &mut display.pixmap_mut(),
                    highlight_rect,
                    rect.h,
                    styles.ui.highlight_color,
                );
            }

            // Only draw visible children
            let (start, end) = self.visible_range();
            for child in &mut self.children[start..end] {
                child.draw(display, styles)?;
            }

            self.dirty = false;
            return Ok(true);
        }

        // Incremental draw - only visible children
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
        self.dirty || self.children.iter().any(|c| c.should_draw())
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        for child in &mut self.children {
            child.set_should_draw();
        }
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        _command: Sender<Command>,
        _bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if self.children.is_empty() {
            return Ok(false);
        }

        match event {
            KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                self.select(
                    (self.selected as isize - 1).rem_euclid(self.children.len() as isize) as usize,
                );
                Ok(true)
            }
            KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                self.select((self.selected + 1).rem_euclid(self.children.len()));
                Ok(true)
            }
            KeyEvent::Pressed(Key::L) | KeyEvent::Autorepeat(Key::L) => {
                self.select(
                    (self.selected as isize - 5).clamp(0, self.children.len() as isize - 1)
                        as usize,
                );
                Ok(true)
            }
            KeyEvent::Pressed(Key::R) | KeyEvent::Autorepeat(Key::R) => {
                self.select((self.selected + 5).clamp(0, self.children.len() - 1));
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
