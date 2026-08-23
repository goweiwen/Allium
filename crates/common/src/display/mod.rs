pub mod color;
pub mod font;
pub mod image;
pub mod settings;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use tiny_skia::{
    BlendMode, FillRule, Paint, Path, PathBuilder, PixmapMut, PixmapRef, Stroke, Transform,
};

use crate::display::color::Color;
use crate::geom::{Point, Rect, Size};

/// A thread stamping a region over a repainting foreground app; it runs until this is dropped
pub struct RectHold {
    stop: Arc<AtomicBool>,
}

impl RectHold {
    /// Runs `stamp` on a thread until the returned hold is dropped; it polls the flag it is handed
    pub fn spawn(stamp: impl FnOnce(&AtomicBool) + Send + 'static) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        std::thread::spawn({
            let stop = Arc::clone(&stop);
            move || stamp(&stop)
        });
        Self { stop }
    }
}

impl Drop for RectHold {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub trait Display: Sized {
    /// Get the width of the display in pixels
    fn width(&self) -> u32;

    /// Get the height of the display in pixels
    fn height(&self) -> u32;

    /// Get the size of the display
    fn size(&self) -> Size {
        Size::new(self.width(), self.height())
    }

    /// Get the bounding box of the display (entire screen area)
    fn bounding_box(&self) -> Rect {
        Rect::new(0, 0, self.width(), self.height())
    }

    /// Get a reference to the underlying pixmap
    fn pixmap(&self) -> PixmapRef<'_>;

    /// Get a mutable reference to the underlying pixmap
    fn pixmap_mut(&mut self) -> PixmapMut<'_>;

    /// Apply a function to all pixels
    fn map_pixels<F>(&mut self, f: F) -> Result<()>
    where
        F: FnMut(Color) -> Color;

    /// Clear the display with a solid color
    fn clear(&mut self, color: Color) -> Result<()> {
        let rect = self.bounding_box();
        fill_rect(&mut self.pixmap_mut(), rect, color);
        Ok(())
    }

    /// Flush any pending changes to the display
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    /// Flush a region of the display; defaults to a full flush
    fn flush_rect(&mut self, _area: Rect) -> Result<()> {
        self.flush()
    }

    /// Keep `area`, already drawn by the caller, restamped over an app that repaints the
    /// screen, until the returned hold is dropped. `None` when the platform cannot stamp
    fn hold_rect(&mut self, _area: Rect, _corner_radius: u32) -> Result<Option<RectHold>> {
        Ok(None)
    }

    /// Sync with the display hardware
    fn sync(&mut self) -> Result<()> {
        Ok(())
    }

    /// Save the current display state
    fn save(&mut self) -> Result<()>;

    /// Load a previously saved display state
    fn load(&mut self, area: Rect) -> Result<()>;

    /// Pop the most recent saved state
    fn pop(&mut self) -> bool;
}

// Primitive drawing helpers

/// Fill a rectangle on the pixmap
pub fn fill_rect(pixmap: &mut PixmapMut<'_>, rect: Rect, color: Color) {
    let paint = Paint {
        shader: tiny_skia::Shader::SolidColor(color.into()),
        blend_mode: BlendMode::SourceOver,
        anti_alias: false,
        ..Default::default()
    };

    if let Some(ts_rect) =
        tiny_skia::Rect::from_xywh(rect.x as f32, rect.y as f32, rect.w as f32, rect.h as f32)
    {
        let path = PathBuilder::from_rect(ts_rect);
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

/// Fill a rounded rectangle on the pixmap
pub fn fill_rounded_rect(pixmap: &mut PixmapMut<'_>, rect: Rect, radius: u32, color: Color) {
    let paint = Paint {
        shader: tiny_skia::Shader::SolidColor(color.into()),
        blend_mode: BlendMode::SourceOver,
        anti_alias: true, // Enable AA for rounded corners
        ..Default::default()
    };

    if let Some(path) = build_rounded_rect_path(rect, radius) {
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

/// Fill a circle on the pixmap
pub fn fill_circle(pixmap: &mut PixmapMut<'_>, center: Point, radius: u32, color: Color) {
    let paint = Paint {
        shader: tiny_skia::Shader::SolidColor(color.into()),
        blend_mode: BlendMode::SourceOver,
        anti_alias: true, // Enable AA for circles
        ..Default::default()
    };

    if let Some(path) = PathBuilder::from_circle(center.x as f32, center.y as f32, radius as f32) {
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

/// Stroke a rectangle on the pixmap
pub fn stroke_rect(pixmap: &mut PixmapMut<'_>, rect: Rect, stroke_width: f32, color: Color) {
    let paint = Paint {
        shader: tiny_skia::Shader::SolidColor(color.into()),
        blend_mode: BlendMode::SourceOver,
        anti_alias: false,
        ..Default::default()
    };

    let stroke = Stroke {
        width: stroke_width,
        ..Default::default()
    };

    if let Some(ts_rect) =
        tiny_skia::Rect::from_xywh(rect.x as f32, rect.y as f32, rect.w as f32, rect.h as f32)
    {
        let path = PathBuilder::from_rect(ts_rect);
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

/// Draw a speaker-with-waves volume icon inside `rect`
pub fn draw_speaker_icon(pixmap: &mut PixmapMut<'_>, rect: Rect, color: Color) {
    let s = rect.w.min(rect.h) as f32;
    let (x, y) = (rect.x as f32, rect.y as f32);
    let paint = icon_paint(color);

    // Speaker body and cone as one polygon
    let mut pb = PathBuilder::new();
    pb.move_to(x + 0.08 * s, y + 0.36 * s);
    pb.line_to(x + 0.26 * s, y + 0.36 * s);
    pb.line_to(x + 0.48 * s, y + 0.16 * s);
    pb.line_to(x + 0.48 * s, y + 0.84 * s);
    pb.line_to(x + 0.26 * s, y + 0.64 * s);
    pb.line_to(x + 0.08 * s, y + 0.64 * s);
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

    // Two sound waves to the right of the cone
    let stroke = icon_stroke(s);
    let (cx, cy) = (x + 0.52 * s, y + 0.5 * s);
    let sweep = std::f32::consts::PI * 0.6;
    const SEGMENTS: u32 = 12;
    for radius in [0.22 * s, 0.36 * s] {
        let mut pb = PathBuilder::new();
        for i in 0..=SEGMENTS {
            let angle = -sweep / 2.0 + sweep * i as f32 / SEGMENTS as f32;
            let px = cx + radius * angle.cos();
            let py = cy + radius * angle.sin();
            if i == 0 {
                pb.move_to(px, py);
            } else {
                pb.line_to(px, py);
            }
        }
        if let Some(path) = pb.finish() {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }
}

/// Draw a sun brightness icon inside `rect`
pub fn draw_sun_icon(pixmap: &mut PixmapMut<'_>, rect: Rect, color: Color) {
    let s = rect.w.min(rect.h) as f32;
    let (cx, cy) = (rect.x as f32 + 0.5 * s, rect.y as f32 + 0.5 * s);
    let paint = icon_paint(color);

    if let Some(path) = PathBuilder::from_circle(cx, cy, 0.18 * s) {
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    // Eight rays around the core
    let stroke = icon_stroke(s);
    let mut pb = PathBuilder::new();
    for i in 0..8 {
        let (sin, cos) = (std::f32::consts::FRAC_PI_4 * i as f32).sin_cos();
        pb.move_to(cx + 0.30 * s * cos, cy + 0.30 * s * sin);
        pb.line_to(cx + 0.44 * s * cos, cy + 0.44 * s * sin);
    }
    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

fn icon_paint(color: Color) -> Paint<'static> {
    Paint {
        shader: tiny_skia::Shader::SolidColor(color.into()),
        blend_mode: BlendMode::SourceOver,
        anti_alias: true,
        ..Default::default()
    }
}

fn icon_stroke(icon_side: f32) -> Stroke {
    Stroke {
        width: (icon_side / 12.0).max(1.0),
        line_cap: tiny_skia::LineCap::Round,
        ..Default::default()
    }
}

/// Build a path for a rounded rectangle
pub fn build_rounded_rect_path(rect: Rect, radius: u32) -> Option<Path> {
    let x = rect.x as f32;
    let y = rect.y as f32;
    let w = rect.w as f32;
    let h = rect.h as f32;
    let r = radius.min(rect.w / 2).min(rect.h / 2) as f32;

    // Bezier control point offset for 90° arc: 4/3 * tan(π/8)
    const K: f32 = 0.552_284_8;
    let k = r * K;

    let mut pb = PathBuilder::new();

    // Start at top-left corner (after the radius)
    pb.move_to(x + r, y);

    // Top edge
    pb.line_to(x + w - r, y);

    // Top-right corner
    pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);

    // Right edge
    pb.line_to(x + w, y + h - r);

    // Bottom-right corner
    pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);

    // Bottom edge
    pb.line_to(x + r, y + h);

    // Bottom-left corner
    pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);

    // Left edge
    pb.line_to(x, y + r);

    // Top-left corner
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);

    pb.close();
    pb.finish()
}
