use std::collections::VecDeque;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use crate::constants::WIFI_UPDATE_INTERVAL;
use crate::geom::{Point, Rect};
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::resources::Resources;
use crate::stylesheet::Stylesheet;
use crate::view::{Command, View, WifiIcon};
use crate::wifi;

#[derive(Debug, Clone)]
pub struct WifiIndicator {
    point: Point,
    connected: bool,
    enabled: bool,
    last_updated: Instant,
    icon: WifiIcon,
}

impl WifiIndicator {
    pub fn new(_res: Resources, point: Point) -> Self {
        let enabled = wifi::WiFiSettings::load().map(|s| s.wifi).unwrap_or(false);
        let connected = enabled && wifi::ip_address().is_some();
        let mut icon = WifiIcon::new(point);
        icon.set_connected(connected);

        Self {
            point,
            connected,
            enabled,
            last_updated: Instant::now(),
            icon,
        }
    }
}

#[async_trait(?Send)]
impl View for WifiIndicator {
    fn update(&mut self, _dt: Duration) {
        // Update WiFi status periodically (every 1 second for responsiveness)
        if self.last_updated.elapsed() < WIFI_UPDATE_INTERVAL {
            return;
        }
        self.last_updated = Instant::now();

        let enabled = wifi::WiFiSettings::load().map(|s| s.wifi).unwrap_or(false);
        let connected = enabled && wifi::ip_address().is_some();

        if enabled != self.enabled || connected != self.connected {
            self.enabled = enabled;
            self.connected = connected;
            self.icon.set_connected(connected);
            self.set_should_draw();
        }
    }

    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if !self.enabled {
            return Ok(false);
        }
        self.icon.draw(display, styles)
    }

    fn should_draw(&self) -> bool {
        self.enabled && self.icon.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.icon.set_should_draw();
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
        vec![&self.icon]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.icon]
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        if !self.enabled {
            return Rect::zero();
        }
        self.icon.bounding_box(styles)
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.icon.set_position(point);
    }
}
