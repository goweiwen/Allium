use std::collections::VecDeque;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::{
    CornerRadii, Primitive, PrimitiveStyleBuilder, RoundedRectangle, Triangle,
};
use embedded_graphics::Drawable;
use log::error;
use tokio::sync::mpsc::Sender;

use crate::battery::Battery;
use crate::constants::BATTERY_UPDATE_INTERVAL;
use crate::display::Display;
use crate::geom::{Point, Rect};
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::{Command, View};

#[derive(Debug, Clone)]
pub struct BatteryIndicator<B>
where
    B: Battery + 'static,
{
    point: Point,
    last_updated: Instant,
    battery: B,
    dirty: bool,
}

impl<B> BatteryIndicator<B>
where
    B: Battery + 'static,
{
    pub fn new(point: Point, mut battery: B) -> Self {
        battery.update().unwrap();
        Self {
            point,
            last_updated: Instant::now(),
            battery,
            dirty: true,
        }
    }
}

#[async_trait(?Send)]
impl<B> View for BatteryIndicator<B>
where
    B: Battery,
{
    fn update(&mut self, _dt: Duration) {
        if self.last_updated.elapsed() < BATTERY_UPDATE_INTERVAL {
            return;
        }
        self.last_updated = Instant::now();
        if let Err(e) = self.battery.update() {
            error!("Failed to update battery: {}", e);
        }
        self.dirty = true;
    }

    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let mut drawn = false;

        if self.dirty {
            display.load(self.bounding_box(styles))?;

            let w = styles.ui_font.size;
            let h = styles.ui_font.size * 3 / 5;
            let y = styles.ui_font.size as i32 / 6;
            let margin = styles.ui_font.size as i32 * 2 / 28;
            let stroke = styles.ui_font.size as i32 * 3 / 28;
            let x = if self.battery.charging() {
                -(styles.ui_font.size as i32) * 5 / 7
            } else {
                -margin
            };

            // Outer battery
            RoundedRectangle::new(
                Rect::new(
                    x + self.point.x - w as i32 - margin - margin,
                    y + self.point.y,
                    w,
                    h,
                )
                .into(),
                CornerRadii::new(Size::new_equal(stroke as u32)),
            )
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(styles.foreground_color)
                    .build(),
            )
            .draw(display)?;

            // Inner battery outline
            RoundedRectangle::new(
                Rect::new(
                    x + self.point.x - w as i32 + stroke - margin - margin,
                    y + self.point.y + stroke,
                    w - 2 * stroke as u32,
                    h - 2 * stroke as u32,
                )
                .into(),
                CornerRadii::new(Size::new_equal(stroke as u32)),
            )
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(styles.background_color)
                    .build(),
            )
            .draw(display)?;

            // Inner battery
            let percentage = self.battery.percentage();
            if percentage > 5 {
                RoundedRectangle::new(
                    Rect::new(
                        x + self.point.x - w as i32 + stroke - margin,
                        y + self.point.y + stroke + margin,
                        (w - 2 * (stroke + margin) as u32) * (percentage - 5).max(0) as u32 / 90,
                        h - 2 * (stroke + margin) as u32,
                    )
                    .into(),
                    CornerRadii::new(Size::new_equal(stroke as u32)),
                )
                .into_styled(
                    PrimitiveStyleBuilder::new()
                        .fill_color(styles.foreground_color)
                        .build(),
                )
                .draw(display)?;
            }

            // Battery cap
            RoundedRectangle::new(
                Rect::new(
                    x + self.point.x - margin,
                    y + self.point.y + stroke + margin,
                    stroke as u32,
                    h - 2 * (stroke + margin) as u32,
                )
                .into(),
                CornerRadii::new(Size::new_equal(stroke as u32)),
            )
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(styles.foreground_color)
                    .build(),
            )
            .draw(display)?;

            // Charging indicator
            if self.battery.charging() {
                let fill_style = PrimitiveStyleBuilder::new()
                    .fill_color(styles.foreground_color)
                    .build();

                let size = styles.ui_font.size;
                Triangle::new(
                    Point::new(
                        self.point.x + -6 * size as i32 / 40,
                        self.point.y + 7 * size as i32 / 40,
                    )
                    .into(),
                    Point::new(
                        self.point.x + -15 * size as i32 / 40,
                        self.point.y + 20 * size as i32 / 40,
                    )
                    .into(),
                    Point::new(
                        self.point.x + -9 * size as i32 / 40,
                        self.point.y + 20 * size as i32 / 40,
                    )
                    .into(),
                )
                .into_styled(fill_style)
                .draw(display)?;
                Triangle::new(
                    Point::new(
                        self.point.x + -12 * size as i32 / 40,
                        self.point.y + 31 * size as i32 / 40,
                    )
                    .into(),
                    Point::new(
                        self.point.x + -3 * size as i32 / 40,
                        self.point.y + 18 * size as i32 / 40,
                    )
                    .into(),
                    Point::new(
                        self.point.x + -9 * size as i32 / 40,
                        self.point.y + 18 * size as i32 / 40,
                    )
                    .into(),
                )
                .into_styled(fill_style)
                .draw(display)?;
            }

            self.dirty = false;
            drawn = true;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.dirty
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
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
        vec![]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![]
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        let w = styles.ui_font.size * 2;
        let h = w * 3 / 5;
        Rect::new(
            self.point.x - w as i32,
            styles.ui_font.size as i32 / 6,
            w,
            h,
        )
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
    }
}
