pub mod color;
pub mod font;
pub mod image;
pub mod settings;

use anyhow::Result;
use tiny_skia::{
    BlendMode, FillRule, Paint, Path, PathBuilder, PixmapMut, PixmapRef, Stroke, Transform,
};

use crate::display::color::Color;
use crate::geom::{Point, Rect, Size};

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
