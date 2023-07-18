use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;

use common::command::Command;
use tokio::sync::mpsc::Sender;

use common::display::Display;
use common::geom::{Alignment, Point, Rect};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use common::view::{ButtonIcon, Label, View};

#[derive(Debug, Clone)]
pub struct ButtonChordHint<S>
where
    S: AsRef<str> + PartialEq + Send,
{
    point: Point,
    buttons: Vec<ButtonIcon>,
    button_pluses: Vec<Label<&'static str>>,
    label: Label<S>,
    alignment: Alignment,
    has_layout: bool,
    dirty: bool,
}

impl<S> ButtonChordHint<S>
where
    S: AsRef<str> + PartialEq + Send,
{
    pub fn new(point: Point, buttons: Vec<Key>, text: S, alignment: Alignment) -> Self {
        let label = Label::new(Point::zero(), text, alignment, None);
        let button_pluses = (0..buttons.len() - 1)
            .map(|_| Label::new(Point::zero(), "+", alignment, None))
            .collect();
        let buttons = buttons
            .iter()
            .map(|k| ButtonIcon::new(Point::zero(), *k, alignment))
            .collect();

        Self {
            point,
            buttons,
            button_pluses,
            label,
            alignment,
            has_layout: false,
            dirty: true,
        }
    }

    pub fn set_text(&mut self, text: S) {
        self.label.set_text(text);
        self.has_layout = false;
        self.dirty = true;
    }

    fn layout(&mut self, styles: &Stylesheet) {
        match self.alignment {
            Alignment::Left => self.layout_left(styles),
            Alignment::Center => unimplemented!("alignment should be Left or Right"),
            Alignment::Right => self.layout_right(styles),
        }
        self.has_layout = true;
    }

    fn layout_left(&mut self, styles: &Stylesheet) {
        let mut x = self.point.x;
        for i in 0..self.buttons.len() {
            let button = &mut self.buttons[i];
            let button_plus = self.button_pluses.get_mut(i);
            button.set_position(Point::new(x, self.point.y));
            x += button.bounding_box(styles).w as i32 + 2;
            if let Some(button_plus) = button_plus {
                button_plus.set_position(Point::new(x, self.point.y + 2));
                x += button_plus.bounding_box(styles).w as i32 + 2;
            }
        }
        x += 12;
        self.label.set_position(Point::new(x, self.point.y + 2));
    }

    fn layout_right(&mut self, styles: &Stylesheet) {
        self.label.set_position(self.point);
        let mut x = self.label.bounding_box(styles).w as i32 - 8;
        for button in &mut self.buttons {
            button.set_position(Point::new(x, self.point.y));
            x -= ButtonIcon::diameter(styles) as i32 + 8;
        }
    }
}

#[async_trait(?Send)]
impl<S> View for ButtonChordHint<S>
where
    S: AsRef<str> + PartialEq + Send,
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

        for v in &mut self.button_pluses {
            drawn |= v.should_draw() && v.draw(display, styles)?;
        }
        for v in &mut self.buttons {
            drawn |= v.should_draw() && v.draw(display, styles)?;
        }
        drawn |= self.label.should_draw() && self.label.draw(display, styles)?;
        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.dirty || self.buttons.iter().any(|b| b.should_draw()) || self.label.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        for button in self.buttons.iter_mut() {
            button.set_should_draw();
        }
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
        std::iter::once(&self.label as &dyn View)
            .chain(self.buttons.iter().map(|b| b as &dyn View))
            .collect()
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        std::iter::once(&mut self.label as &mut dyn View)
            .chain(self.buttons.iter_mut().map(|b| b as &mut dyn View))
            .collect()
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        if !self.has_layout {
            self.layout(styles);
        }

        self.buttons
            .iter_mut()
            .map(|b| b.bounding_box(styles))
            .reduce(|a, b| a.union(&b))
            .unwrap()
            .union(&self.label.bounding_box(styles))
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.has_layout = false;
    }
}
