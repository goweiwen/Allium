use anyhow::{Result, anyhow, bail};
use framebuffer::Framebuffer;
use log::{trace, warn};
use tiny_skia::{Pixmap, PixmapMut, PixmapRef};

use crate::display::Display;
use crate::display::color::Color;
use crate::geom::Rect;

pub struct FramebufferDisplay {
    pixmap: Pixmap,
    iface: Framebuffer,
    saved: Vec<Pixmap>,
}

impl FramebufferDisplay {
    pub fn new() -> Result<FramebufferDisplay> {
        let iface = Framebuffer::new("/dev/fb0")?;
        trace!(
            "init fb: var_screen_info: {:?}, fix_screen_info: {:?}",
            iface.var_screen_info, iface.fix_screen_info,
        );

        let width = iface.var_screen_info.xres;
        let height = iface.var_screen_info.yres;

        let mut pixmap = Pixmap::new(width, height)
            .ok_or_else(|| anyhow!("Failed to create pixmap {}x{}", width, height))?;

        // Read initial framebuffer content
        let background = iface.read_frame();
        let (xoffset, yoffset) = (
            iface.var_screen_info.xoffset as usize,
            iface.var_screen_info.yoffset as usize,
        );
        let bytes_per_pixel = iface.var_screen_info.bits_per_pixel / 8;
        let location = (yoffset * width as usize + xoffset) * bytes_per_pixel as usize;

        // Copy initial background (need to convert from BGRA and unrotate)
        for y in 0..height {
            for x in 0..width {
                // Framebuffer is rotated 180°, so read from reversed position
                let fb_x = width - x - 1;
                let fb_y = height - y - 1;
                let fb_idx = location + ((fb_y * width + fb_x) as usize * bytes_per_pixel as usize);

                let b = background[fb_idx];
                let g = background[fb_idx + 1];
                let r = background[fb_idx + 2];
                let a = background[fb_idx + 3];

                let color = Color::rgba(r, g, b, a);
                let idx = (y * width + x) as usize;
                pixmap.pixels_mut()[idx] = color.into();
            }
        }

        Ok(FramebufferDisplay {
            pixmap,
            iface,
            saved: Vec::new(),
        })
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

        let background = self.iface.read_frame();

        // Re-read framebuffer content (convert from BGRA and unrotate)
        for y in 0..height {
            for x in 0..width {
                let fb_x = width - x - 1;
                let fb_y = height - y - 1;
                let fb_idx = location + (fb_y * width + fb_x) * bytes_per_pixel;

                let b = background[fb_idx];
                let g = background[fb_idx + 1];
                let r = background[fb_idx + 2];
                let a = background[fb_idx + 3];

                let color = Color::rgba(r, g, b, a);
                let idx = y * width + x;
                self.pixmap.pixels_mut()[idx] = color.into();
            }
        }

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
        let (xoffset, yoffset) = (
            self.iface.var_screen_info.xoffset as usize,
            self.iface.var_screen_info.yoffset as usize,
        );
        let width = self.width() as usize;
        let height = self.height() as usize;
        let bytes_per_pixel = (self.iface.var_screen_info.bits_per_pixel / 8) as usize;
        let location = (yoffset * width + xoffset) * bytes_per_pixel;

        // Write pixmap to framebuffer with 180° rotation and BGRA format
        for y in 0..height {
            for x in 0..width {
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

        Ok(())
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
