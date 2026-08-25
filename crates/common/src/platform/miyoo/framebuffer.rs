use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use framebuffer::Framebuffer;
use log::{debug, trace, warn};
use tiny_skia::{Pixmap, PixmapMut, PixmapRef};

use crate::display::color::Color;
use crate::display::{Display, RectHold};
use crate::geom::Rect;

/// Pause between stamping passes, matching Onion; probing or yielding instead costs whole
/// frames on this dual-core SoC
const STAMP_PAUSE: Duration = Duration::from_micros(100);

pub struct FramebufferDisplay {
    pixmap: Pixmap,
    iface: Framebuffer,
    saved: Vec<Pixmap>,
}

impl FramebufferDisplay {
    pub fn new() -> Result<FramebufferDisplay> {
        let mut display = Self::blank()?;
        let frame = display.bounding_box();
        display.read_frame_rect(frame);
        Ok(display)
    }

    /// A display whose pixmap starts empty; the caller reads what it needs with `read_rect`
    pub fn blank() -> Result<FramebufferDisplay> {
        let iface = Framebuffer::new("/dev/fb0")?;
        trace!(
            "init fb: var_screen_info: {:?}, fix_screen_info: {:?}",
            iface.var_screen_info, iface.fix_screen_info,
        );

        let width = iface.var_screen_info.xres;
        let height = iface.var_screen_info.yres;
        let pixmap = Pixmap::new(width, height)
            .ok_or_else(|| anyhow!("Failed to create pixmap {}x{}", width, height))?;

        Ok(FramebufferDisplay {
            pixmap,
            iface,
            saved: Vec::new(),
        })
    }

    /// Copies `rect` of the visible frame into the pixmap, unrotating it and BGRA to RGBA
    fn read_frame_rect(&mut self, rect: Rect) {
        let width = self.pixmap.width() as usize;
        let height = self.pixmap.height() as usize;
        let bytes_per_pixel = (self.iface.var_screen_info.bits_per_pixel / 8) as usize;
        let xoffset = self.iface.var_screen_info.xoffset as usize;
        let yoffset = self.iface.var_screen_info.yoffset as usize;
        let location = (yoffset * width + xoffset) * bytes_per_pixel;

        let x0 = rect.x.max(0) as usize;
        let y0 = rect.y.max(0) as usize;
        let x1 = (rect.right().max(0) as usize).min(width);
        let y1 = (rect.bottom().max(0) as usize).min(height);

        let frame = self.iface.read_frame();
        let pixels = self.pixmap.pixels_mut();
        for y in y0..y1 {
            // The framebuffer is rotated 180 degrees, so both axes run backwards
            let fb_y = height - 1 - y;
            for x in x0..x1 {
                let fb_x = width - 1 - x;
                let fb_idx = location + (fb_y * width + fb_x) * bytes_per_pixel;
                let color = Color::rgba(
                    frame[fb_idx + 2],
                    frame[fb_idx + 1],
                    frame[fb_idx],
                    frame[fb_idx + 3],
                );
                pixels[y * width + x] = color.into();
            }
        }
    }

    /// Packs `area` into fb-ordered rows, so the stamping thread only does memcpy
    fn stamp(&self, area: Rect, corner_radius: u32) -> Option<Stamp> {
        let width = self.width() as usize;
        let height = self.height() as usize;
        let bytes_per_pixel = (self.iface.var_screen_info.bits_per_pixel / 8) as usize;

        let x0 = area.x.max(0) as usize;
        let y0 = area.y.max(0) as usize;
        let x1 = (area.right().max(0) as usize).min(width);
        let y1 = (area.bottom().max(0) as usize).min(height);
        if x0 >= x1 || y0 >= y1 {
            return None;
        }

        // Trim to the rounding so the corners keep the app's pixels, not a frozen frame
        let radius = (corner_radius as usize)
            .min((x1 - x0) / 2)
            .min((y1 - y0) / 2) as f32;
        let row_h = (y1 - y0) as f32;
        let mut rows = Vec::with_capacity(y1 - y0);
        let mut bytes = Vec::with_capacity((y1 - y0) * (x1 - x0) * bytes_per_pixel);
        for y in y0..y1 {
            let dy = (y - y0) as f32 + 0.5;
            let dist = (radius - dy).max(dy - (row_h - radius)).max(0.0);
            let inset = (radius - (radius * radius - dist * dist).sqrt()).ceil() as usize;
            let (rx0, rx1) = (x0 + inset, x1 - inset);
            if rx0 >= rx1 {
                continue;
            }
            let start = bytes.len();
            // Reversed x, and fb_y below, are the two halves of the 180 degree rotation
            for x in (rx0..rx1).rev() {
                let pixel = self.pixmap.pixels()[y * width + x];
                bytes.extend_from_slice(&[pixel.blue(), pixel.green(), pixel.red(), pixel.alpha()]);
            }
            let fb_y = height - 1 - y;
            let fb_x0 = width - rx1;
            rows.push(StampRow {
                offset: (fb_y * width + fb_x0) * bytes_per_pixel,
                start,
                len: bytes.len() - start,
            });
        }
        if rows.is_empty() {
            return None;
        }

        let pages = (self.iface.var_screen_info.yres_virtual as usize / height.max(1)).clamp(1, 3);
        debug!(
            "holding {}x{} rect over fb {}x{} (virtual {}, stride {}, {} bpp, {} pages)",
            x1 - x0,
            y1 - y0,
            width,
            height,
            self.iface.var_screen_info.yres_virtual,
            self.iface.fix_screen_info.line_length,
            bytes_per_pixel * 8,
            pages,
        );
        Some(Stamp {
            bytes: bytes.into_boxed_slice(),
            rows: rows.into_boxed_slice(),
            pages,
            page_stride: width * height * bytes_per_pixel,
        })
    }

    fn write_rect(&mut self, rect: Rect) {
        let yoffset = self.iface.var_screen_info.yoffset as usize;
        self.write_rect_at(rect, yoffset);
    }

    fn write_rect_at(&mut self, rect: Rect, yoffset: usize) {
        let xoffset = self.iface.var_screen_info.xoffset as usize;
        let width = self.width() as usize;
        let height = self.height() as usize;
        let bytes_per_pixel = (self.iface.var_screen_info.bits_per_pixel / 8) as usize;
        let location = (yoffset * width + xoffset) * bytes_per_pixel;

        if location + height * width * bytes_per_pixel > self.iface.frame.len() {
            return;
        }

        let x0 = rect.x.max(0) as usize;
        let y0 = rect.y.max(0) as usize;
        let x1 = (rect.right().max(0) as usize).min(width);
        let y1 = (rect.bottom().max(0) as usize).min(height);

        // Write pixmap to framebuffer with 180° rotation and BGRA format
        for y in y0..y1 {
            for x in x0..x1 {
                let idx = y * width + x;
                let pixel = self.pixmap.pixels()[idx];

                // Apply 180° rotation when writing to framebuffer
                let fb_x = width - x - 1;
                let fb_y = height - y - 1;
                let fb_idx = location + (fb_y * width + fb_x) * bytes_per_pixel;

                // Write as BGRA (use premultiplied values directly)
                self.iface.frame[fb_idx] = pixel.blue();
                self.iface.frame[fb_idx + 1] = pixel.green();
                self.iface.frame[fb_idx + 2] = pixel.red();
                self.iface.frame[fb_idx + 3] = pixel.alpha();
            }
        }
    }
}

/// One row of the stamp: where it goes within a page, and its slice of `Stamp::bytes`
struct StampRow {
    offset: usize,
    start: usize,
    len: usize,
}

/// Rows of a rect in fb byte order, to be repeated across every page the app may flip to
struct Stamp {
    /// Every row back to back, so a pass reads the source linearly
    bytes: Box<[u8]>,
    rows: Box<[StampRow]>,
    pages: usize,
    page_stride: usize,
}

impl Stamp {
    fn blit(&self, frame: &mut [u8]) {
        for page in 0..self.pages {
            let base = page * self.page_stride;
            for row in &self.rows {
                let at = base + row.offset;
                if let Some(dst) = frame.get_mut(at..at + row.len) {
                    dst.copy_from_slice(&self.bytes[row.start..row.start + row.len]);
                }
            }
        }
    }
}

impl Display for FramebufferDisplay {
    fn width(&self) -> u32 {
        self.pixmap.width()
    }

    fn height(&self) -> u32 {
        self.pixmap.height()
    }

    fn pixmap(&self) -> PixmapRef<'_> {
        self.pixmap.as_ref()
    }

    fn pixmap_mut(&mut self) -> PixmapMut<'_> {
        self.pixmap.as_mut()
    }

    fn sync(&mut self) -> Result<()> {
        self.iface.var_screen_info = Framebuffer::get_var_screeninfo(&self.iface.device)
            .map_err(|e| anyhow!("failed to get var_screen_info: {}", e))?;

        let xoffset = self.iface.var_screen_info.xoffset as usize;
        let yoffset = self.iface.var_screen_info.yoffset as usize;
        let width = self.width() as usize;
        let height = self.height() as usize;
        let bytes_per_pixel = (self.iface.var_screen_info.bits_per_pixel / 8) as usize;
        let location = (yoffset * width + xoffset) * bytes_per_pixel;

        let frame = self.bounding_box();
        self.read_frame_rect(frame);

        if yoffset != 0 {
            let frame_size = width * height * bytes_per_pixel;
            self.iface
                .frame
                .copy_within(location..location + frame_size, 0);
            self.iface.var_screen_info.yoffset = 0;
            Framebuffer::put_var_screeninfo(&self.iface.device, &self.iface.var_screen_info)
                .map_err(|e| anyhow!("failed to set var_screen_info: {}", e))?;
        }

        Ok(())
    }

    fn map_pixels<F>(&mut self, mut f: F) -> Result<()>
    where
        F: FnMut(Color) -> Color,
    {
        for pixel in self.pixmap.pixels_mut() {
            let color: Color = (*pixel).into();
            *pixel = f(color).into();
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.write_rect(self.bounding_box());
        Ok(())
    }

    fn flush_rect(&mut self, area: Rect) -> Result<()> {
        // Re-read offsets: the foreground app may have moved yoffset since creation
        self.iface.var_screen_info = Framebuffer::get_var_screeninfo(&self.iface.device)
            .map_err(|e| anyhow!("failed to get var_screen_info: {}", e))?;
        self.write_rect(area);
        Ok(())
    }

    fn read_rect(&mut self, area: Rect) -> Result<()> {
        // Re-read offsets: the foreground app may have moved yoffset since creation
        self.iface.var_screen_info = Framebuffer::get_var_screeninfo(&self.iface.device)
            .map_err(|e| anyhow!("failed to get var_screen_info: {}", e))?;
        self.read_frame_rect(area);
        Ok(())
    }

    fn hold_rect(&mut self, area: Rect, corner_radius: u32) -> Result<Option<RectHold>> {
        self.flush_rect(area)?;
        let Some(stamp) = self.stamp(area, corner_radius) else {
            return Ok(None);
        };

        Ok(Some(RectHold::spawn(move |stop| {
            let Ok(mut iface) = Framebuffer::new("/dev/fb0") else {
                return;
            };
            while !stop.load(Ordering::Relaxed) {
                stamp.blit(&mut iface.frame);
                std::thread::sleep(STAMP_PAUSE);
            }
        })))
    }

    fn save(&mut self) -> Result<()> {
        self.saved.push(self.pixmap.clone());
        Ok(())
    }

    fn load(&mut self, mut rect: Rect) -> Result<()> {
        let Some(saved) = self.saved.last() else {
            bail!("No saved image");
        };

        let size = self.size();
        if rect.x < 0
            || rect.y < 0
            || rect.x as u32 + rect.w > size.w
            || rect.y as u32 + rect.h > size.h
        {
            warn!(
                "Area exceeds display bounds: x: {}, y: {}, w: {}, h: {}",
                rect.x, rect.y, rect.w, rect.h,
            );
            rect.x = rect.x.max(0);
            rect.y = rect.y.max(0);
            rect.w = rect.w.min(size.w - rect.x as u32);
            rect.h = rect.h.min(size.h - rect.y as u32);
        }

        // Copy saved region to current pixmap
        let width = self.width() as usize;
        for dy in 0..rect.h {
            for dx in 0..rect.w {
                let x = (rect.x + dx as i32) as usize;
                let y = (rect.y + dy as i32) as usize;
                let idx = y * width + x;
                self.pixmap.pixels_mut()[idx] = saved.pixels()[idx];
            }
        }

        Ok(())
    }

    fn pop(&mut self) -> bool {
        self.saved.pop();
        !self.saved.is_empty()
    }
}
