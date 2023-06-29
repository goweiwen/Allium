use std::collections::VecDeque;
use std::time::Instant;

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::{
    CornerRadii, Primitive, PrimitiveStyleBuilder, RoundedRectangle, Triangle,
};
use embedded_graphics::Drawable;
use tokio::sync::mpsc::Sender;

use crate::battery::Battery;
use crate::constants::{BATTERY_SIZE, BATTERY_UPDATE_INTERVAL};
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

    pub fn update(&mut self) -> Result<()> {
        self.battery.update()?;
        self.dirty = true;
        Ok(())
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
            self.last_updated = Instant::now();
            self.update()?;
        }

        let mut drawn = false;

        if self.dirty {
            display.load(self.bounding_box(styles))?;

            let offset = if self.battery.charging() { -22 } else { 0 };

            // Outer battery
            RoundedRectangle::new(
                Rect::new(offset + self.point.x + -38, self.point.y + 12, 31, 17).into(),
                CornerRadii::new(Size::new_equal(4)),
            )
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(styles.background_color)
                    .stroke_color(styles.foreground_color)
                    .stroke_width(3)
                    .build(),
            )
            .draw(display)?;

            // Inner battery
            let percentage = self.battery.percentage();
            if percentage > 5 {
                RoundedRectangle::new(
                    Rect::new(
                        offset + self.point.x + -34,
                        self.point.y + 16,
                        23 * (percentage - 5).clamp(0, 90) as u32 / 90,
                        9,
                    )
                    .into(),
                    CornerRadii::new(Size::new_equal(2)),
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
                Rect::new(offset + self.point.x + -4, self.point.y + 16, 4, 9).into(),
                CornerRadii::new(Size::new_equal(2)),
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

                Triangle::new(
                    Point::new(self.point.x + -6, self.point.y + 8).into(),
                    Point::new(self.point.x + -15, self.point.y + 21).into(),
                    Point::new(self.point.x + -9, self.point.y + 21).into(),
                )
                .into_styled(fill_style)
                .draw(display)?;
                Triangle::new(
                    Point::new(self.point.x + -12, self.point.y + 32).into(),
                    Point::new(self.point.x + -3, self.point.y + 19).into(),
                    Point::new(self.point.x + -9, self.point.y + 19).into(),
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
        self.dirty || self.last_updated.elapsed() >= BATTERY_UPDATE_INTERVAL
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

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        Rect::new(
            self.point.x - BATTERY_SIZE.w as i32,
            self.point.y,
            BATTERY_SIZE.w,
            BATTERY_SIZE.h,
        )
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
    }
}
