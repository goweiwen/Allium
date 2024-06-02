use std::collections::VecDeque;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use chrono::Local;

use tokio::sync::mpsc::Sender;

use crate::constants::CLOCK_UPDATE_INTERVAL;
use crate::display::Display;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::{Command, Label, View};

#[derive(Debug, Clone)]
pub struct Clock {
    label: Label<String>,
    point: Point,
    last_updated: Instant,
}

impl Clock {
    pub fn new(point: Point, alignment: Alignment) -> Self {
        let label = Label::new(point, text(), alignment, None);

        Self {
            label,
            point,
            last_updated: Instant::now(),
        }
    }
}

#[async_trait(?Send)]
impl View for Clock {
    fn update(&mut self, _dt: Duration) {
        if self.last_updated.elapsed() >= CLOCK_UPDATE_INTERVAL {
            self.label.set_text(text());
            self.last_updated = Instant::now();
        }
    }

    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        display.load(self.bounding_box(styles))?;
        self.label.draw(display, styles)
    }

    fn should_draw(&self) -> bool {
        self.label.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.label.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        _event: KeyEvent,
        _commands: Sender<Command>,
        _bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        Ok(false)
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.label]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.label]
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        self.label.bounding_box(styles)
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.label.set_position(point);
    }
}

fn text() -> String {
    format!("{}", Local::now().format("%H:%M"))
}
