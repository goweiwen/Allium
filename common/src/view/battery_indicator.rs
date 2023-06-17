use std::collections::VecDeque;
use std::time::Instant;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tracing::error;

use crate::battery::Battery;
use crate::constants::BATTERY_UPDATE_INTERVAL;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::{Command, Label, View};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryIndicator<B>
where
    B: Battery,
{
    label: Label<String>,
    #[serde(skip)]
    battery: Option<B>,
    point: Point,
    #[serde(skip, default = "Instant::now")]
    last_updated: Instant,
}

impl<B> BatteryIndicator<B>
where
    B: Battery,
{
    pub fn new(point: Point, alignment: Alignment) -> Self {
        let label = Label::new(point, "".to_owned(), alignment, None);

        Self {
            label,
            battery: None,
            point,
            last_updated: Instant::now(),
        }
    }

    pub fn init(&mut self, battery: B) {
        self.label.set_text(text(&battery));
        self.battery = Some(battery);
    }
}

#[async_trait(?Send)]
impl<B> View for BatteryIndicator<B>
where
    B: Battery,
{
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if self.last_updated.elapsed() >= BATTERY_UPDATE_INTERVAL {
            if let Some(ref mut battery) = self.battery {
                if let Err(e) = battery.update() {
                    error!("Failed to update battery: {}", e);
                }
                self.label.set_text(text(battery));
            }
        }

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

fn text(battery: &impl Battery) -> String {
    if battery.charging() {
        format!("Charging: {}%", battery.percentage())
    } else {
        format!("{}%", battery.percentage())
    }
}
