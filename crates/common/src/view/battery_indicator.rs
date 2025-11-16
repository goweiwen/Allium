use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::Drawable;
use embedded_graphics::image::{Image, ImageRaw};
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::{
    CornerRadii, Primitive, PrimitiveStyleBuilder, RoundedRectangle, Triangle,
};
use image::RgbaImage;
use log::{debug, error};
use tokio::sync::mpsc::Sender;

use crate::battery::Battery;
use crate::constants::{ALLIUM_THEMES_DIR, BATTERY_UPDATE_INTERVAL};
use crate::display::Display;
use crate::display::color::Color;
use crate::geom::{Point, Rect};
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::resources::Resources;
use crate::stylesheet::{Stylesheet, Theme};
use crate::view::{Command, Label, View};

#[derive(Debug, Clone)]
enum BatteryIcons {
    Image {
        charging: RgbaImage,
        levels: Vec<RgbaImage>,
    },
    Vector,
}

impl BatteryIcons {
    fn load() -> Self {
        let theme = Theme::load();
        let theme_dir = ALLIUM_THEMES_DIR.join(&theme.0);

        let resolve_icon_path = |icon_name: &str| -> PathBuf {
            let theme_icon = theme_dir.join("assets").join(icon_name);
            if theme_icon.exists() {
                return theme_icon;
            }
            // Fallback to default theme
            ALLIUM_THEMES_DIR
                .join("Allium")
                .join("assets")
                .join(icon_name)
        };

        let charging_path = resolve_icon_path("battery-charging.png");
        let charging = match image::open(charging_path) {
            Ok(img) => img.to_rgba8(),
            Err(e) => {
                debug!(
                    "Failed to load battery charging icon: {}. Falling back to primitive rendering.",
                    e
                );
                return BatteryIcons::Vector;
            }
        };

        let mut levels = Vec::new();
        let mut i = 0;
        loop {
            let level_path = resolve_icon_path(&format!("battery-{}.png", i));
            if !level_path.exists() {
                break;
            }
            match image::open(level_path) {
                Ok(level_image) => levels.push(level_image.to_rgba8()),
                Err(e) => {
                    debug!(
                        "Failed to load battery level {} icon: {}. Falling back to primitive rendering.",
                        i, e
                    );
                    return BatteryIcons::Vector;
                }
            }
            i += 1;
        }

        if levels.is_empty() {
            debug!("No battery level icons found. Falling back to primitive rendering.");
            return BatteryIcons::Vector;
        }

        BatteryIcons::Image { charging, levels }
    }

    fn bounding_box(&self, styles: &Stylesheet, charging: bool) -> Rect {
        match self {
            BatteryIcons::Image {
                charging: charging_img,
                levels,
            } => {
                let img = if charging {
                    charging_img
                } else {
                    &levels[0] // All level images should have the same dimensions
                };
                Rect::new(0, 0, img.width(), img.height())
            }
            BatteryIcons::Vector => {
                let font_size = styles.status_bar_font_size() as u32;
                let margin = styles.status_bar_font_size() as i32 * 2 / 28;
                let stroke = styles.status_bar_font_size() as i32 * 3 / 28;

                // Battery width
                let battery_w = font_size as i32 + stroke + margin * 3;

                // Charging indicator width
                let charging_w = if charging {
                    (styles.status_bar_font_size() * 5.0 / 7.0) as i32 + margin * 3
                } else {
                    0
                };

                let total_w = battery_w + charging_w;
                let h = (styles.status_bar_font_size() * 3.0 / 5.0) as i32;

                Rect::new(0, 0, total_w as u32, h as u32)
            }
        }
    }

    fn draw(
        &self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
        point: Point,
        label_w: i32,
        charging: bool,
        percentage: i32,
    ) -> Result<()> {
        match self {
            BatteryIcons::Image {
                charging: charging_img,
                levels,
            } => {
                let image_to_draw = if charging {
                    charging_img
                } else {
                    let num_levels = levels.len();
                    let level = (percentage as usize * num_levels / 101).min(num_levels - 1);
                    &levels[level]
                };

                let icon_width = image_to_draw.width() as i32;
                let draw_point = Point::new(point.x - icon_width - label_w, point.y);

                let raw_image: ImageRaw<'_, Color> =
                    ImageRaw::new(image_to_draw, image_to_draw.width());
                let image = Image::new(&raw_image, draw_point.into());
                image.draw(display)?;
            }
            BatteryIcons::Vector => {
                let w = styles.status_bar_font_size() as u32;
                let h = (styles.status_bar_font_size() * 3.0 / 5.0) as u32;
                let y = styles.status_bar_font_size() as i32 / 6 + 1;
                let margin = styles.status_bar_font_size() as i32 * 2 / 28;
                let stroke = styles.status_bar_font_size() as i32 * 3 / 28;
                let x = if charging {
                    (-styles.status_bar_font_size() * 5.0 / 7.0) as i32 - label_w
                } else {
                    -margin - label_w
                };

                // Outer battery stroke
                if styles.stroke_width > 0 && styles.status_bar_stroke_color.a() > 0 {
                    for dx in -(styles.stroke_width as i32)..=(styles.stroke_width as i32) {
                        for dy in -(styles.stroke_width as i32)..=(styles.stroke_width as i32) {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            RoundedRectangle::new(
                                Rect::new(
                                    x + point.x - w as i32 - margin - margin + dx,
                                    y + point.y + dy,
                                    w,
                                    h,
                                )
                                .into(),
                                CornerRadii::new(Size::new_equal(stroke as u32 * 2)),
                            )
                            .into_styled(
                                PrimitiveStyleBuilder::new()
                                    .stroke_color(styles.status_bar_stroke_color)
                                    .stroke_alignment(
                                        embedded_graphics::primitives::StrokeAlignment::Inside,
                                    )
                                    .stroke_width(stroke as u32)
                                    .build(),
                            )
                            .draw(display)?;
                        }
                    }
                }

                // Outer battery
                RoundedRectangle::new(
                    Rect::new(x + point.x - w as i32 - margin - margin, y + point.y, w, h).into(),
                    CornerRadii::new(Size::new_equal(stroke as u32 * 2)),
                )
                .into_styled(
                    PrimitiveStyleBuilder::new()
                        .stroke_color(styles.status_bar_color)
                        .stroke_alignment(embedded_graphics::primitives::StrokeAlignment::Inside)
                        .stroke_width(stroke as u32)
                        .build(),
                )
                .draw(display)?;

                // Inner battery stroke
                if percentage > 5 {
                    if styles.stroke_width > 0 && styles.status_bar_stroke_color.a() > 0 {
                        for dx in -(styles.stroke_width as i32)..=(styles.stroke_width as i32) {
                            for dy in -(styles.stroke_width as i32)..=(styles.stroke_width as i32) {
                                if dx == 0 && dy == 0 {
                                    continue;
                                }
                                RoundedRectangle::new(
                                    Rect::new(
                                        x + point.x - w as i32 + stroke - margin + dx,
                                        y + point.y + stroke + margin + dy,
                                        w.saturating_sub(2 * (stroke + margin) as u32)
                                            * (percentage - 5).max(0) as u32
                                            / 90,
                                        h.saturating_sub(2 * (stroke + margin) as u32),
                                    )
                                    .into(),
                                    CornerRadii::new(Size::new_equal(stroke as u32)),
                                )
                                .into_styled(
                                    PrimitiveStyleBuilder::new()
                                        .fill_color(styles.status_bar_stroke_color)
                                        .build(),
                                )
                                .draw(display)?;
                            }
                        }
                    }

                    // Inner battery
                    RoundedRectangle::new(
                        Rect::new(
                            x + point.x - w as i32 + stroke - margin,
                            y + point.y + stroke + margin,
                            w.saturating_sub(2 * (stroke + margin) as u32)
                                * (percentage - 5).max(0) as u32
                                / 90,
                            h.saturating_sub(2 * (stroke + margin) as u32),
                        )
                        .into(),
                        CornerRadii::new(Size::new_equal(stroke as u32)),
                    )
                    .into_styled(
                        PrimitiveStyleBuilder::new()
                            .fill_color(styles.status_bar_color)
                            .build(),
                    )
                    .draw(display)?;
                }

                // Battery cap stroke
                if styles.stroke_width > 0 && styles.status_bar_stroke_color.a() > 0 {
                    for dx in -(styles.stroke_width as i32)..=(styles.stroke_width as i32) {
                        for dy in -(styles.stroke_width as i32)..=(styles.stroke_width as i32) {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            RoundedRectangle::new(
                                Rect::new(
                                    x + point.x - margin + dx,
                                    y + point.y + stroke + margin + dy,
                                    stroke as u32,
                                    h.saturating_sub(2 * (stroke + margin) as u32),
                                )
                                .into(),
                                CornerRadii::new(Size::new_equal(stroke as u32)),
                            )
                            .into_styled(
                                PrimitiveStyleBuilder::new()
                                    .fill_color(styles.status_bar_stroke_color)
                                    .build(),
                            )
                            .draw(display)?;
                        }
                    }
                }

                // Battery cap
                RoundedRectangle::new(
                    Rect::new(
                        x + point.x - margin,
                        y + point.y + stroke + margin,
                        stroke as u32,
                        h.saturating_sub(2 * (stroke + margin) as u32),
                    )
                    .into(),
                    CornerRadii::new(Size::new_equal(stroke as u32)),
                )
                .into_styled(
                    PrimitiveStyleBuilder::new()
                        .fill_color(styles.status_bar_color)
                        .build(),
                )
                .draw(display)?;

                // Charging indicator
                if charging {
                    let stroke_style = PrimitiveStyleBuilder::new()
                        .fill_color(styles.status_bar_stroke_color)
                        .build();
                    let fill_style = PrimitiveStyleBuilder::new()
                        .fill_color(styles.status_bar_color)
                        .build();

                    let x = point.x - label_w;
                    let size = styles.status_bar_font_size();

                    // First triangle stroke
                    if styles.stroke_width > 0 && styles.status_bar_stroke_color.a() > 0 {
                        for dx in -(styles.stroke_width as i32)..=(styles.stroke_width as i32) {
                            for dy in -(styles.stroke_width as i32)..=(styles.stroke_width as i32) {
                                if dx == 0 && dy == 0 {
                                    continue;
                                }
                                Triangle::new(
                                    Point::new(
                                        x + (-6.0 * size / 40.0) as i32 + dx,
                                        point.y + (7.0 * size / 40.0) as i32 + dy,
                                    )
                                    .into(),
                                    Point::new(
                                        x + (-15.0 * size / 40.0) as i32 + dx,
                                        point.y + (20.0 * size / 40.0) as i32 + dy,
                                    )
                                    .into(),
                                    Point::new(
                                        x + (-9.0 * size / 40.0) as i32 + dx,
                                        point.y + (20.0 * size / 40.0) as i32 + dy,
                                    )
                                    .into(),
                                )
                                .into_styled(stroke_style)
                                .draw(display)?;
                            }
                        }
                    }

                    // Second triangle stroke
                    if styles.stroke_width > 0 && styles.status_bar_stroke_color.a() > 0 {
                        for dx in -(styles.stroke_width as i32)..=(styles.stroke_width as i32) {
                            for dy in -(styles.stroke_width as i32)..=(styles.stroke_width as i32) {
                                if dx == 0 && dy == 0 {
                                    continue;
                                }
                                Triangle::new(
                                    Point::new(
                                        x + (-12.0 * size / 40.0) as i32 + dx,
                                        point.y + (31.0 * size / 40.0) as i32 + dy,
                                    )
                                    .into(),
                                    Point::new(
                                        x + (-3.0 * size / 40.0) as i32 + dx,
                                        point.y + (18.0 * size / 40.0) as i32 + dy,
                                    )
                                    .into(),
                                    Point::new(
                                        x + (-9.0 * size / 40.0) as i32 + dx,
                                        point.y + (18.0 * size / 40.0) as i32 + dy,
                                    )
                                    .into(),
                                )
                                .into_styled(stroke_style)
                                .draw(display)?;
                            }
                        }
                    }

                    // First triangle
                    Triangle::new(
                        Point::new(
                            x + (-6.0 * size / 40.0) as i32,
                            point.y + (7.0 * size / 40.0) as i32,
                        )
                        .into(),
                        Point::new(
                            x + (-15.0 * size / 40.0) as i32,
                            point.y + (20.0 * size / 40.0) as i32,
                        )
                        .into(),
                        Point::new(
                            x + (-9.0 * size / 40.0) as i32,
                            point.y + (20.0 * size / 40.0) as i32,
                        )
                        .into(),
                    )
                    .into_styled(fill_style)
                    .draw(display)?;

                    // Second triangle
                    Triangle::new(
                        Point::new(
                            x + (-12.0 * size / 40.0) as i32,
                            point.y + (31.0 * size / 40.0) as i32,
                        )
                        .into(),
                        Point::new(
                            x + (-3.0 * size / 40.0) as i32,
                            point.y + (18.0 * size / 40.0) as i32,
                        )
                        .into(),
                        Point::new(
                            x + (-9.0 * size / 40.0) as i32,
                            point.y + (18.0 * size / 40.0) as i32,
                        )
                        .into(),
                    )
                    .into_styled(fill_style)
                    .draw(display)?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BatteryIndicator<B>
where
    B: Battery + 'static,
{
    point: Point,
    last_updated: Instant,
    label: Option<Label<String>>,
    battery: B,
    icons: BatteryIcons,
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
            label.color(crate::stylesheet::StylesheetColor::StatusBar);
            label.stroke_color(crate::stylesheet::StylesheetColor::StatusBarStroke);
            Some(label)
        } else {
            None
        };

        let icons = BatteryIcons::load();

        Self {
            point,
            last_updated: Instant::now(),
            label,
            battery,
            icons,
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

            self.icons.draw(
                display,
                styles,
                self.point,
                label_w,
                self.battery.charging(),
                self.battery.percentage(),
            )?;

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
        let label_w = if let Some(ref mut label) = self.label {
            label.bounding_box(styles).w as i32 + 8
        } else {
            0
        };

        let icon_size = self.icons.bounding_box(styles, self.battery.charging());
        let total_width = icon_size.w as i32 + label_w;
        let left = self.point.x - total_width;
        let top = self.point.y;

        Rect::new(left, top, total_width as u32, icon_size.h)
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        if let Some(ref mut label) = self.label {
            label.set_position(point);
        }
    }
}

fn format_battery_percentage(charging: bool, percentage: i32) -> String {
    if charging {
        String::new()
    } else {
        format!("{}%", percentage)
    }
}
