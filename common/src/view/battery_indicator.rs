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
use crate::resources::Resources;
use crate::stylesheet::Stylesheet;
use crate::view::{Command, Label, View};

#[derive(Debug, Clone)]
pub struct BatteryIndicator<B>
where
    B: Battery + 'static,
{
    point: Point,
    last_updated: Instant,
    label: Option<Label<String>>,
    battery: B,
    dirty: bool,
}

impl<B> BatteryIndicator<B>
where
    B: Battery + 'static,
{
    pub fn new(res: Resources, point: Point, mut battery: B, show_percentage: bool) -> Self {
        battery.update().unwrap();

        let label = if show_percentage {
            let styles = res.get::<Stylesheet>();
            let mut label = Label::new(
                point,
                format_battery_percentage(battery.charging(), battery.percentage()),
                crate::geom::Alignment::Right,
                None,
            );
            label.font_size(styles.status_bar_font_size);
            Some(label)
        } else {
            None
        };

        Self {
            point,
            last_updated: Instant::now(),
            label,
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
        if let Some(ref mut label) = self.label {
            label.set_text(format_battery_percentage(
                self.battery.charging(),
                self.battery.percentage(),
            ));
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

            let label_w = if let Some(ref mut label) = self.label {
                label.bounding_box(styles).w as i32 + 8
            } else {
                0
            };
            let w = styles.status_bar_font_size() as u32;
            let h = (styles.status_bar_font_size() * 3.0 / 5.0) as u32;
            let y = styles.ui_font.size as i32 / 6 + 1;
            let margin = styles.ui_font.size as i32 * 2 / 28;
            let stroke = styles.ui_font.size as i32 * 3 / 28;
            let x = if self.battery.charging() {
                (-styles.status_bar_font_size() * 5.0 / 7.0) as i32 - label_w
            } else {
                -margin - label_w
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
                CornerRadii::new(Size::new_equal(stroke as u32 * 2)),
            )
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_color(styles.foreground_color)
                    .stroke_alignment(embedded_graphics::primitives::StrokeAlignment::Inside)
                    .stroke_width(stroke as u32)
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

                let x = self.point.x - label_w;
                let size = styles.status_bar_font_size();
                Triangle::new(
                    Point::new(
                        x + (-6.0 * size / 40.0) as i32,
                        self.point.y + (7.0 * size / 40.0) as i32,
                    )
                    .into(),
                    Point::new(
                        x + (-15.0 * size / 40.0) as i32,
                        self.point.y + (20.0 * size / 40.0) as i32,
                    )
                    .into(),
                    Point::new(
                        x + (-9.0 * size / 40.0) as i32,
                        self.point.y + (20.0 * size / 40.0) as i32,
                    )
                    .into(),
                )
                .into_styled(fill_style)
                .draw(display)?;
                Triangle::new(
                    Point::new(
                        x + (-12.0 * size / 40.0) as i32,
                        self.point.y + (31.0 * size / 40.0) as i32,
                    )
                    .into(),
                    Point::new(
                        x + (-3.0 * size / 40.0) as i32,
                        self.point.y + (18.0 * size / 40.0) as i32,
                    )
                    .into(),
                    Point::new(
                        x + (-9.0 * size / 40.0) as i32,
                        self.point.y + (18.0 * size / 40.0) as i32,
                    )
                    .into(),
                )
                .into_styled(fill_style)
                .draw(display)?;
            }

            if let Some(ref mut label) = self.label {
                label.draw(display, styles)?;
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
        if let Some(ref mut label) = self.label {
            label.set_should_draw()
        }
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
        let w = (styles.status_bar_font_size() * 3.0) as u32;
        let h = (styles.status_bar_font_size() * 6.0 / 5.0) as u32;

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

fn format_battery_percentage(charging: bool, percentage: i32) -> String {
    if charging {
        String::new()
    } else {
        format!("{}%", percentage)
    }
}
