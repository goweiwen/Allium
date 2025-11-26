//! Font rendering (ttf and otf) with tiny-skia.

use std::f32;
use std::vec::Vec;

use rusttype::Font;
use rusttype::GlyphId;
use rusttype::vector;
use tiny_skia::PixmapMut;

use crate::display::color::Color;
use crate::geom::{Point, Size};

/// Style properties for text using a ttf and otf font.
#[derive(Debug, Clone)]
pub struct FontTextStyle {
    /// Text color.
    pub text_color: Option<Color>,

    /// Background color.
    pub background_color: Option<Color>,

    /// Should draw background or skip.
    pub draw_background: bool,

    /// Underline color.
    pub underline_color: DecorationColor,

    /// Strikethrough color.
    pub strikethrough_color: DecorationColor,

    /// Stroke color.
    pub stroke_color: Option<Color>,

    /// Stroke width.
    pub stroke_width: u32,

    /// Font size.
    pub font_size: u32,

    /// Font.
    font: Font<'static>,

    /// Font fallback.
    font_fallback: Option<Font<'static>>,
}

/// Decoration color options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationColor {
    /// No decoration
    None,
    /// Use the text color
    TextColor,
    /// Custom color
    Custom(Color),
}

impl FontTextStyle {
    /// Creates a text style with transparent background.
    pub fn new(font: Font<'static>, text_color: Color, font_size: u32) -> Self {
        FontTextStyleBuilder::new(font)
            .text_color(text_color)
            .font_size(font_size)
            .build()
    }

    /// Creates a text style with a fallback font and transparent background.
    pub fn with_fallback(
        font: Font<'static>,
        text_color: Color,
        font_size: u32,
        font_fallback: Font<'static>,
    ) -> Self {
        FontTextStyleBuilder::new(font)
            .font_fallback(font_fallback)
            .text_color(text_color)
            .font_size(font_size)
            .build()
    }

    /// Resolves a decoration color.
    fn resolve_decoration_color(&self, color: DecorationColor) -> Option<Color> {
        match color {
            DecorationColor::None => None,
            DecorationColor::TextColor => self.text_color,
            DecorationColor::Custom(c) => Some(c),
        }
    }

    fn draw_background(&self, width: u32, position: Point, pixmap: &mut PixmapMut<'_>) {
        if width == 0 {
            return;
        }

        if let Some(background_color) = self.background_color {
            let rect = crate::geom::Rect {
                x: position.x,
                y: position.y,
                w: width,
                h: self.font_size,
            };
            crate::display::fill_rect(pixmap, rect, background_color);
        }
    }

    fn draw_strikethrough(&self, width: u32, position: Point, pixmap: &mut PixmapMut<'_>) {
        if let Some(strikethrough_color) = self.resolve_decoration_color(self.strikethrough_color) {
            let rect = crate::geom::Rect {
                x: position.x,
                y: position.y + self.font_size as i32 / 2,
                w: width,
                h: self.font_size / 12,
            };
            crate::display::fill_rect(pixmap, rect, strikethrough_color);
        }
    }

    fn draw_underline(&self, width: u32, position: Point, pixmap: &mut PixmapMut<'_>) {
        if let Some(underline_color) = self.resolve_decoration_color(self.underline_color) {
            let line_height = self.font_size / 12;
            let rect = crate::geom::Rect {
                x: position.x,
                y: position.y + self.font_size as i32 - line_height as i32,
                w: width,
                h: line_height,
            };
            crate::display::fill_rect(pixmap, rect, underline_color);
        }
    }

    /// Draw text to a pixmap
    pub fn draw(&self, pixmap: &mut PixmapMut<'_>, text: &str, position: Point) -> Point {
        // Handle multiline text
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return position;
        }

        let mut current_y = position.y;
        for line in lines {
            self.draw_line(pixmap, line, Point::new(position.x, current_y));
            current_y += self.font_size as i32;
        }

        Point {
            x: position.x,
            y: current_y,
        }
    }

    /// Draw a single line of text to a pixmap
    fn draw_line(&self, pixmap: &mut PixmapMut<'_>, text: &str, position: Point) -> Point {
        let scale = rusttype::Scale::uniform(self.font_size as f32);

        let v_metrics = self.font.v_metrics(scale);
        let start = rusttype::point(0.0, v_metrics.ascent);

        let glyphs: Vec<rusttype::PositionedGlyph<'_>> = text
            .chars()
            .map(|c| {
                let mut g = self.font.glyph(c);
                if g.id() == GlyphId(0)
                    && let Some(font_fallback) = self.font_fallback.as_ref()
                {
                    g = font_fallback.glyph(c);
                }
                g
            })
            .scan((None, self.stroke_width as f32 * 2.0), |(last, x), g| {
                let g = g.scaled(scale);
                if let Some(last) = last {
                    *x += self.font.pair_kerning(scale, *last, g.id());
                }
                let w = g.h_metrics().advance_width;
                let next = g.positioned(start + vector(*x, 0.0));
                *last = Some(next.id());
                *x += w;
                Some(next)
            })
            .collect();

        let width = glyphs
            .iter()
            .rev()
            .filter_map(|g| {
                g.pixel_bounding_box()
                    .map(|b| b.min.x as f32 + g.unpositioned().h_metrics().advance_width)
            })
            .next()
            .unwrap_or(0.0)
            .ceil() as i32
            + self.stroke_width as i32 * 2;

        let height = self.font_size as i32;

        // Create a buffer to hold the rasterized output
        // Each pixel stores RGBA color
        let buffer_width = width as usize;
        let buffer_height = height as usize;
        let mut buffer: Vec<Color> = vec![Color::rgba(0, 0, 0, 0); buffer_width * buffer_height];

        // Draw stroke first - render the glyph multiple times at different offsets
        // Skip if stroke color is transparent (alpha == 0)
        if let Some(stroke_color) = self.stroke_color
            && self.stroke_width > 0
            && stroke_color.a() > 0
        {
            // Draw the glyph at each offset position within stroke_width
            for dx in -(self.stroke_width as i32)..=(self.stroke_width as i32) {
                for dy in -(self.stroke_width as i32)..=(self.stroke_width as i32) {
                    // Skip the center (0,0) as that's where the actual text will be
                    if dx == 0 && dy == 0 {
                        continue;
                    }

                    for g in glyphs.iter() {
                        if let Some(bb) = g.pixel_bounding_box() {
                            g.draw(|off_x, off_y, v| {
                                let off_x = off_x as i32 + bb.min.x + dx;
                                let off_y = off_y as i32 + bb.min.y + dy;
                                // There's still a possibility that the glyph clips the boundaries of the bitmap
                                if off_x >= 0 && off_x < width && off_y >= 0 && off_y < height {
                                    let stroke_a = (v * stroke_color.a() as f32) as u8;
                                    if stroke_a > 0 {
                                        let idx = off_y as usize * buffer_width + off_x as usize;
                                        let existing = buffer[idx];
                                        // Take max alpha since we're drawing the same color
                                        let max_a = existing.a().max(stroke_a);
                                        buffer[idx] = Color::rgba(
                                            stroke_color.r(),
                                            stroke_color.g(),
                                            stroke_color.b(),
                                            max_a,
                                        );
                                    }
                                }
                            });
                        }
                    }
                }
            }
        }

        // Draw text on top of stroke
        if let Some(text_color) = self.text_color {
            for g in glyphs.iter() {
                if let Some(bb) = g.pixel_bounding_box() {
                    g.draw(|off_x, off_y, v| {
                        let off_x = off_x as i32 + bb.min.x;
                        let off_y = off_y as i32 + bb.min.y;
                        // There's still a possibility that the glyph clips the boundaries of the bitmap
                        if off_x >= 0 && off_x < width && off_y >= 0 && off_y < height {
                            let text_a = (v * text_color.a() as f32) as u8;

                            if text_a > 0 {
                                let idx = off_y as usize * buffer_width + off_x as usize;
                                let existing = buffer[idx];

                                // Alpha blend the text pixel
                                if existing.a() == 0 {
                                    buffer[idx] = Color::rgba(
                                        text_color.r(),
                                        text_color.g(),
                                        text_color.b(),
                                        text_a,
                                    );
                                } else {
                                    // Blend using alpha compositing
                                    let src_a = text_a as f32 / 255.0;
                                    let dst_a = existing.a() as f32 / 255.0;
                                    let out_a = src_a + dst_a * (1.0 - src_a);
                                    if out_a > 0.0 {
                                        let out_r = ((text_color.r() as f32 * src_a
                                            + existing.r() as f32 * dst_a * (1.0 - src_a))
                                            / out_a)
                                            as u8;
                                        let out_g = ((text_color.g() as f32 * src_a
                                            + existing.g() as f32 * dst_a * (1.0 - src_a))
                                            / out_a)
                                            as u8;
                                        let out_b = ((text_color.b() as f32 * src_a
                                            + existing.b() as f32 * dst_a * (1.0 - src_a))
                                            / out_a)
                                            as u8;
                                        buffer[idx] =
                                            Color::rgba(out_r, out_g, out_b, (out_a * 255.0) as u8);
                                    }
                                }
                            }
                        }
                    });
                }
            }
        }

        // Draw background if requested
        if self.draw_background {
            self.draw_background(width as u32, position, pixmap);
        }

        // Write buffer to pixmap
        let pixmap_width = pixmap.width() as i32;
        let pixmap_height = pixmap.height() as i32;

        for y in 0..buffer_height {
            for x in 0..buffer_width {
                let buffer_idx = y * buffer_width + x;
                let color = buffer[buffer_idx];

                if color.a() > 0 {
                    let px = position.x + x as i32;
                    let py = position.y + y as i32;

                    if px >= 0 && px < pixmap_width && py >= 0 && py < pixmap_height {
                        let pixmap_idx = (py * pixmap_width + px) as usize;

                        // Alpha blend with existing pixel
                        let pixels = pixmap.pixels_mut();
                        let existing: Color = pixels[pixmap_idx].into();
                        let blended = blend_colors(color, existing);
                        pixels[pixmap_idx] = blended.into();
                    }
                }
            }
        }

        self.draw_strikethrough(width as u32, position, pixmap);
        self.draw_underline(width as u32, position, pixmap);

        position
    }

    /// Draw whitespace (for decorations without text)
    pub fn draw_whitespace(
        &self,
        pixmap: &mut PixmapMut<'_>,
        width: u32,
        position: Point,
    ) -> Point {
        self.draw_background(width, position, pixmap);
        self.draw_strikethrough(width, position, pixmap);
        self.draw_underline(width, position, pixmap);

        Point {
            x: position.x + width as i32,
            y: position.y,
        }
    }

    /// Measure the size of rendered text
    pub fn measure(&self, text: &str) -> Size {
        // Handle multiline text
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return Size {
                w: 0,
                h: self.font_size,
            };
        }

        let scale = rusttype::Scale::uniform(self.font_size as f32);
        let v_metrics = self.font.v_metrics(scale);
        let start = rusttype::point(0.0, v_metrics.ascent);

        // Measure each line and find the widest
        let max_width = lines
            .iter()
            .map(|line| {
                let glyphs: Vec<rusttype::PositionedGlyph<'_>> = line
                    .chars()
                    .map(|c| {
                        let mut g = self.font.glyph(c);
                        if g.id() == GlyphId(0)
                            && let Some(font_fallback) = self.font_fallback.as_ref()
                        {
                            g = font_fallback.glyph(c);
                        }
                        g
                    })
                    .scan((None, 0.0), |(last, x), g| {
                        let g = g.scaled(scale);
                        if let Some(last) = last {
                            *x += self.font.pair_kerning(scale, *last, g.id());
                        }
                        let w = g.h_metrics().advance_width;
                        let next = g.positioned(start + vector(*x, 0.0));
                        *last = Some(next.id());
                        *x += w;
                        Some(next)
                    })
                    .collect();

                glyphs
                    .iter()
                    .rev()
                    .map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
                    .next()
                    .unwrap_or(0.0)
                    .ceil() as u32
            })
            .max()
            .unwrap_or(0);

        Size {
            w: max_width,
            h: self.font_size * lines.len() as u32,
        }
    }

    /// Get the line height for this font style
    pub fn line_height(&self) -> u32 {
        self.font_size
    }
}

/// Alpha blend two colors (src over dst)
fn blend_colors(src: Color, dst: Color) -> Color {
    let src_a = src.a() as f32 / 255.0;
    let dst_a = dst.a() as f32 / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);

    if out_a == 0.0 {
        Color::rgba(0, 0, 0, 0)
    } else {
        let out_r =
            ((src.r() as f32 * src_a + dst.r() as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
        let out_g =
            ((src.g() as f32 * src_a + dst.g() as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
        let out_b =
            ((src.b() as f32 * src_a + dst.b() as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
        Color::rgba(out_r, out_g, out_b, (out_a * 255.0) as u8)
    }
}

/// Text style builder for ttf and otf fonts.
pub struct FontTextStyleBuilder {
    style: FontTextStyle,
}

impl FontTextStyleBuilder {
    /// Creates a new text style builder.
    pub fn new(font: Font<'static>) -> Self {
        Self {
            style: FontTextStyle {
                font,
                font_fallback: None,
                background_color: None,
                font_size: 12,
                text_color: None,
                underline_color: DecorationColor::None,
                strikethrough_color: DecorationColor::None,
                stroke_color: None,
                stroke_width: 0,
                draw_background: false,
            },
        }
    }

    /// Builder method used to set the font size of the style.
    pub fn font_size(mut self, font_size: u32) -> Self {
        self.style.font_size = font_size;
        self
    }

    /// Builder method used to set the font fallback of the style.
    pub fn font_fallback(mut self, font_fallback: Font<'static>) -> Self {
        self.style.font_fallback = Some(font_fallback);
        self
    }

    /// Enables underline using the text color.
    pub fn underline(mut self) -> Self {
        self.style.underline_color = DecorationColor::TextColor;

        self
    }

    /// Enables strikethrough using the text color.
    pub fn strikethrough(mut self) -> Self {
        self.style.strikethrough_color = DecorationColor::TextColor;

        self
    }

    /// Sets the text color.
    pub fn text_color(mut self, text_color: Color) -> Self {
        self.style.text_color = Some(text_color);

        self
    }

    /// Sets the background color.
    pub fn background_color(mut self, background_color: Color) -> Self {
        self.style.background_color = Some(background_color);

        self
    }

    /// Enables underline with a custom color.
    pub fn underline_with_color(mut self, underline_color: Color) -> Self {
        self.style.underline_color = DecorationColor::Custom(underline_color);

        self
    }

    /// Enables strikethrough with a custom color.
    pub fn strikethrough_with_color(mut self, strikethrough_color: Color) -> Self {
        self.style.strikethrough_color = DecorationColor::Custom(strikethrough_color);

        self
    }

    pub fn draw_background(mut self) -> Self {
        self.style.draw_background = true;

        self
    }

    /// Sets the stroke color.
    pub fn stroke_color(mut self, stroke_color: Color) -> Self {
        self.style.stroke_color = Some(stroke_color);

        self
    }

    /// Sets the stroke width.
    pub fn stroke_width(mut self, stroke_width: u32) -> Self {
        self.style.stroke_width = stroke_width;

        self
    }

    /// Builds the text style.
    pub fn build(self) -> FontTextStyle {
        self.style
    }
}
