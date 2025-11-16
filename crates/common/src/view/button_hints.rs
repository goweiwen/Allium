use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::geometry::Dimensions;
use tokio::sync::mpsc::Sender;

use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::resources::Resources;
use crate::stylesheet::Stylesheet;
use crate::view::{ButtonHint, ButtonIcon, Command, Row, View};

#[derive(Debug, Clone)]
pub struct ButtonHints<S>
where
    S: AsRef<str> + PartialEq + Send + Clone,
{
    res: Resources,
    left: Vec<ButtonHint<S>>,
    right: Vec<ButtonHint<S>>,
    left_row: Option<Row<ButtonHint<S>>>,
    right_row: Option<Row<ButtonHint<S>>>,
    dirty: bool,
}

impl<S> ButtonHints<S>
where
    S: AsRef<str> + PartialEq + Send + Clone,
{
    pub fn new(res: Resources, left: Vec<ButtonHint<S>>, right: Vec<ButtonHint<S>>) -> Self {
        Self {
            res,
            left,
            right,
            left_row: None,
            right_row: None,
            dirty: true,
        }
    }

    fn ensure_layout(&mut self, display: &<DefaultPlatform as Platform>::Display) {
        if self.left_row.is_some() && self.right_row.is_some() {
            return;
        }

        let styles = self.res.get::<Stylesheet>();
        let Rect { x, y, w, h } = display.bounding_box().into();

        self.left_row = Some(Row::new(
            Point::new(
                x + styles.margin_x,
                y + h as i32 - ButtonIcon::diameter(&styles) as i32 - styles.margin_x,
            ),
            self.left.clone(),
            Alignment::Left,
            12,
        ));

        self.right_row = Some(Row::new(
            Point::new(
                x + w as i32 - styles.margin_y,
                y + h as i32 - ButtonIcon::diameter(&styles) as i32 - styles.margin_x,
            ),
            self.right.clone(),
            Alignment::Right,
            12,
        ));
    }

    pub fn left(&self) -> &Vec<ButtonHint<S>> {
        &self.left
    }

    pub fn left_mut(&mut self) -> &mut Vec<ButtonHint<S>> {
        self.left_row = None; // Invalidate layout
        &mut self.left
    }

    pub fn right(&self) -> &Vec<ButtonHint<S>> {
        &self.right
    }

    pub fn right_mut(&mut self) -> &mut Vec<ButtonHint<S>> {
        self.right_row = None; // Invalidate layout
        &mut self.right
    }
}

#[async_trait(?Send)]
impl<S> View for ButtonHints<S>
where
    S: AsRef<str> + PartialEq + Send + Clone,
{
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        self.ensure_layout(display);

        let mut drawn = false;

        if self.should_draw() {
            self.dirty = false;
            if let Some(ref mut left) = self.left_row {
                left.draw(display, styles)?;
            }
            if let Some(ref mut right) = self.right_row {
                right.draw(display, styles)?;
            }
            drawn = true;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.dirty
            || self.left_row.as_ref().is_some_and(|r| r.should_draw())
            || self.right_row.as_ref().is_some_and(|r| r.should_draw())
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        if let Some(ref mut left) = self.left_row {
            left.set_should_draw();
        }
        if let Some(ref mut right) = self.right_row {
            right.set_should_draw();
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
        let mut children = Vec::new();
        if let Some(ref left) = self.left_row {
            children.push(left as &dyn View);
        }
        if let Some(ref right) = self.right_row {
            children.push(right as &dyn View);
        }
        children
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        let mut children = Vec::new();
        if let Some(ref mut left) = self.left_row {
            children.push(left as &mut dyn View);
        }
        if let Some(ref mut right) = self.right_row {
            children.push(right as &mut dyn View);
        }
        children
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        let mut bbox = Rect::zero();
        if let Some(ref mut left) = self.left_row {
            bbox = bbox.union(&left.bounding_box(styles));
        }
        if let Some(ref mut right) = self.right_row {
            bbox = bbox.union(&right.bounding_box(styles));
        }

        // Extend the bounding box to cover the entire width
        if bbox.w > 0 && bbox.h > 0 {
            let left_x = if let Some(ref mut left) = self.left_row {
                left.bounding_box(styles).x
            } else {
                bbox.x
            };

            let right_bbox = if let Some(ref mut right) = self.right_row {
                right.bounding_box(styles)
            } else {
                bbox
            };
            let right_edge = right_bbox.x + right_bbox.w as i32;

            let full_width = (right_edge - left_x) as u32;
            bbox = Rect::new(left_x, bbox.y, full_width, bbox.h);
        }

        bbox
    }

    fn set_position(&mut self, _point: Point) {
        // Position is controlled by the left and right rows individually
    }
}
