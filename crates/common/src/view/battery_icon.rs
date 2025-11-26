use std::collections::VecDeque;
use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use image::RgbaImage;
use log::debug;
use tiny_skia::{BlendMode, FillRule, Paint, PathBuilder, Transform};
use tokio::sync::mpsc::Sender;

use crate::constants::ALLIUM_THEMES_DIR;
use crate::display::Display;
use crate::geom::{Point, Rect};
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::stylesheet::{Stylesheet, Theme};
use crate::view::{Command, View};

/// Layout calculations for vector battery icon.
/// All positions are absolute screen coordinates.
struct VectorBatteryLayout {
    /// Outer battery body rectangle
    body: Rect,
    /// Inner fill rectangle (width varies with percentage)
    fill: Option<Rect>,
    /// Battery cap (positive terminal)
    cap: Rect,
    /// Charging indicator position (right edge of lightning bolt)
    charging_pos: Option<Point>,
    /// Corner radius for body
    body_radius: u32,
    /// Corner radius for fill and cap
    inner_radius: u32,
    /// Stroke width for body outline
    stroke_width: u32,
}

impl VectorBatteryLayout {
    fn calculate(point: Point, styles: &Stylesheet, charging: bool, percentage: i32) -> Self {
        let font_size = styles.status_bar_font_size();

        // Dimensions derived from font size
        let body_w = font_size as i32;
        let body_h = (font_size * 0.6) as i32;
        let stroke = font_size as i32 / 30 + 1;
        let cap_w = stroke * 2;
        let charging_w = if charging {
            (font_size * 5.0 / 7.0) as i32
        } else {
            0
        };

        // Vertical centering
        let y_offset = (font_size as i32 - body_h) / 2;

        // Layout from right to left:
        // [body] [cap] [charging indicator]
        //                                                    ^ point.x is here (right edge)

        let cap_right = point.x - charging_w - 1;
        let cap_left = cap_right - cap_w * 2 / 3;
        let body_right = cap_left;
        let body_left = body_right - body_w;
        let body_top = point.y + y_offset;

        let body = Rect::new(body_left, body_top, body_w as u32, body_h as u32);

        // Cap is to the right of body
        let cap_h = (4 * stroke).min(body_h / 4);
        let cap_top = body_top + (body_h - cap_h) / 2;
        let cap = Rect::new(cap_left, cap_top, cap_w as u32, cap_h as u32);

        // Fill inside body (only if percentage > 5%)
        let fill = if percentage > 5 {
            let fill_left = body_left + stroke * 2;
            let fill_top = body_top + stroke * 2;
            let max_fill_w = (body_w - 2 * stroke * 2).max(0);
            let fill_w = max_fill_w * (percentage - 5).clamp(0, 95) / 95;
            let fill_h = (body_h - 2 * stroke * 2).max(0);
            if fill_w > 0 && fill_h > 0 {
                Some(Rect::new(fill_left, fill_top, fill_w as u32, fill_h as u32))
            } else {
                None
            }
        } else {
            None
        };

        // Charging indicator position (right edge of bolt)
        let charging_pos = if charging {
            Some(Point::new(point.x, point.y))
        } else {
            None
        };

        Self {
            body,
            fill,
            cap,
            charging_pos,
            body_radius: (stroke * 4) as u32,
            inner_radius: (stroke * 2) as u32,
            stroke_width: stroke as u32,
        }
    }

    fn total_size(styles: &Stylesheet, charging: bool) -> Rect {
        let font_size = styles.status_bar_font_size();
        let body_w = font_size as i32;
        let stroke = font_size as i32 / 30 + 1;
        let cap_w = stroke * 2;
        let charging_w = if charging {
            (font_size * 5.0 / 7.0) as i32
        } else {
            0
        };

        // [margin] [body] [margin] [cap] [charging indicator]
        let total_w = body_w + cap_w + charging_w;
        let h = font_size as i32;

        Rect::new(0, 0, total_w as u32, h as u32)
    }
}

#[derive(Debug, Clone)]
enum BatteryIconVariant {
    Image {
        charging: RgbaImage,
        levels: Vec<RgbaImage>,
    },
    Vector,
}

#[derive(Debug, Clone)]
pub struct BatteryIcon {
    variant: BatteryIconVariant,
    point: Point,
    charging: bool,
    percentage: i32,
    dirty: bool,
}

impl BatteryIcon {
    pub fn new(point: Point) -> Self {
        Self {
            variant: Self::load_variant(),
            point,
            charging: false,
            percentage: 0,
            dirty: true,
        }
    }

    fn load_variant() -> BatteryIconVariant {
        let theme = Theme::load();
        let theme_dir = ALLIUM_THEMES_DIR.join(&theme.0);

        let resolve_icon_path = |icon_name: &str| -> PathBuf {
            let theme_icon = theme_dir.join("assets").join(icon_name);
            if theme_icon.exists() {
                return theme_icon;
            }
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
                return BatteryIconVariant::Vector;
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
                    return BatteryIconVariant::Vector;
                }
            }
            i += 1;
        }

        if levels.is_empty() {
            debug!("No battery level icons found. Falling back to primitive rendering.");
            return BatteryIconVariant::Vector;
        }

        BatteryIconVariant::Image { charging, levels }
    }

    pub fn set_state(&mut self, charging: bool, percentage: i32) {
        if self.charging != charging || self.percentage != percentage {
            self.charging = charging;
            self.percentage = percentage;
            self.dirty = true;
        }
    }

    fn icon_size(&self, styles: &Stylesheet) -> Rect {
        match &self.variant {
            BatteryIconVariant::Image {
                charging: charging_img,
                levels,
            } => {
                let img = if self.charging {
                    charging_img
                } else {
                    &levels[0]
                };
                Rect::new(0, 0, img.width(), img.height())
            }
            BatteryIconVariant::Vector => VectorBatteryLayout::total_size(styles, self.charging),
        }
    }

    fn draw_icon(
        &self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<()> {
        match &self.variant {
            BatteryIconVariant::Image {
                charging: charging_img,
                levels,
            } => {
                let image_to_draw = if self.charging {
                    charging_img
                } else {
                    let num_levels = levels.len();
                    let level = (self.percentage as usize * num_levels / 101).min(num_levels - 1);
                    &levels[level]
                };

                let icon_width = image_to_draw.width() as i32;
                let draw_point = Point::new(self.point.x - icon_width, self.point.y);

                crate::display::image::draw_image(
                    &mut display.pixmap_mut(),
                    image_to_draw,
                    draw_point,
                );
            }
            BatteryIconVariant::Vector => {
                let layout = VectorBatteryLayout::calculate(
                    self.point,
                    styles,
                    self.charging,
                    self.percentage,
                );

                let stroke_color = styles.status_bar.text_color;
                let fill_color = styles.status_bar.text_color;

                let mut pixmap = display.pixmap_mut();

                // Draw battery body (outline only)
                if let Some(path) =
                    crate::display::build_rounded_rect_path(layout.body, layout.body_radius)
                {
                    let paint = Paint {
                        shader: tiny_skia::Shader::SolidColor(stroke_color.into()),
                        blend_mode: BlendMode::SourceOver,
                        anti_alias: true,
                        ..Default::default()
                    };
                    let stroke = tiny_skia::Stroke {
                        width: layout.stroke_width as f32,
                        ..Default::default()
                    };
                    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                }

                // Draw battery fill
                if let Some(fill_rect) = layout.fill {
                    crate::display::fill_rounded_rect(
                        &mut pixmap,
                        fill_rect,
                        layout.inner_radius,
                        fill_color,
                    );
                }

                // Draw battery cap
                crate::display::fill_rounded_rect(
                    &mut pixmap,
                    layout.cap,
                    layout.body_radius,
                    fill_color,
                );

                // Draw charging indicator (lightning bolt)
                if let Some(pos) = layout.charging_pos {
                    self.draw_charging_indicator(display, pos, styles)?;
                }
            }
        }

        Ok(())
    }

    fn draw_charging_indicator(
        &self,
        display: &mut <DefaultPlatform as Platform>::Display,
        pos: Point,
        styles: &Stylesheet,
    ) -> Result<()> {
        let fill_color = styles.status_bar.text_color;
        let size = styles.status_bar_font_size();

        // Lightning bolt made of two triangles
        // Coordinates are relative to pos (right edge of bolt area)
        let scale = size / 30.0;

        // Upper triangle points
        let p1 = Point::new(pos.x - (4.0 * scale) as i32, pos.y + (5.0 * scale) as i32);
        let p2 = Point::new(pos.x - (12.0 * scale) as i32, pos.y + (16.0 * scale) as i32);
        let p3 = Point::new(pos.x - (6.0 * scale) as i32, pos.y + (16.0 * scale) as i32);

        // Lower triangle points
        let p4 = Point::new(pos.x - (8.0 * scale) as i32, pos.y + (25.0 * scale) as i32);
        let p5 = Point::new(pos.x - (0.0 * scale) as i32, pos.y + (14.0 * scale) as i32);
        let p6 = Point::new(pos.x - (6.0 * scale) as i32, pos.y + (14.0 * scale) as i32);

        let mut pixmap = display.pixmap_mut();

        let paint = Paint {
            shader: tiny_skia::Shader::SolidColor(fill_color.into()),
            blend_mode: BlendMode::SourceOver,
            anti_alias: true,
            ..Default::default()
        };

        let mut pb = PathBuilder::new();
        pb.move_to(p1.x as f32, p1.y as f32);
        pb.line_to(p2.x as f32, p2.y as f32);
        pb.line_to(p3.x as f32, p3.y as f32);
        pb.move_to(p4.x as f32, p4.y as f32);
        pb.line_to(p5.x as f32, p5.y as f32);
        pb.line_to(p6.x as f32, p6.y as f32);
        pb.close();
        if let Some(path) = pb.finish() {
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }

        Ok(())
    }
}

#[async_trait(?Send)]
impl View for BatteryIcon {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if !self.dirty {
            return Ok(false);
        }
        self.draw_icon(display, styles)?;
        self.dirty = false;
        Ok(true)
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
        let icon_size = self.icon_size(styles);
        let left = self.point.x - icon_size.w as i32;
        let top = self.point.y;

        Rect::new(left, top, icon_size.w, icon_size.h)
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.dirty = true;
    }
}
