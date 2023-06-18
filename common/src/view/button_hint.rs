use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::constants::BUTTON_DIAMETER;
use crate::display::Display;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::{ButtonIcon, Command, Label, View};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonHint<S>
where
    S: AsRef<str>,
{
    point: Point,
    button: ButtonIcon,
    label: Label<S>,
    alignment: Alignment,
    has_layout: bool,
    dirty: bool,
}

impl<S> ButtonHint<S>
where
    S: AsRef<str> + Send,
{
    pub fn new(point: Point, button: Key, text: S, alignment: Alignment) -> Self {
        let label = Label::new(Point::zero(), text, alignment, None);
        let button = ButtonIcon::new(Point::zero(), button, alignment);

        Self {
            point,
            button,
            label,
            alignment,
            has_layout: false,
            dirty: true,
        }
    }

    pub fn set_text(&mut self, text: S) {
        self.label.set_text(text);
        self.has_layout = false;
    }

    fn layout(&mut self, styles: &Stylesheet) {
        match self.alignment {
            Alignment::Left => self.layout_left(styles),
            Alignment::Center => unimplemented!("alignment should be Left or Right"),
            Alignment::Right => self.layout_right(styles),
        }
        self.has_layout = true;
    }

    fn layout_left(&mut self, _styles: &Stylesheet) {
        self.button.set_position(self.point);
        self.label.set_position(Point::new(
            self.point.x + BUTTON_DIAMETER as i32 + 8,
            self.point.y + 2,
        ));
    }

    fn layout_right(&mut self, styles: &Stylesheet) {
        self.label
            .set_position(Point::new(self.point.x, self.point.y + 2));
        self.button.set_position(Point::new(
            self.label.bounding_box(styles).x - 8,
            self.point.y,
        ));
    }
}

#[async_trait(?Send)]
impl<S> View for ButtonHint<S>
where
    S: AsRef<str> + Send,
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
            display.load(self.bounding_box(styles).into())?;
            drawn = true;
            self.dirty = false;
        }

        drawn |= self.label.should_draw() && self.button.draw(display, styles)?;
        drawn |= self.label.should_draw() && self.label.draw(display, styles)?;
        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.has_layout || self.button.should_draw() || self.label.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        self.button.set_should_draw();
        self.label.set_should_draw();
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
        vec![&self.button, &self.label]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.button, &mut self.label]
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        if !self.has_layout {
            self.layout(styles);
        }

        self.button
            .bounding_box(styles)
            .union(&self.label.bounding_box(styles))
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.has_layout = false;
    }
}
