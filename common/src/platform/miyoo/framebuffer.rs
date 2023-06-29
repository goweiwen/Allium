use anyhow::{anyhow, bail, Result};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::Pixel;
use framebuffer::Framebuffer;
use log::trace;

use crate::display::color::Color;
use crate::display::Display;
use crate::geom::Rect;

pub struct Buffer {
    buffer: Vec<u8>,
    size: Size,
    bytes_per_pixel: u32,
}

pub struct FramebufferDisplay {
    framebuffer: Buffer,
    iface: Framebuffer,
    saved: Option<Vec<u8>>,
}

impl FramebufferDisplay {
    pub fn new() -> Result<FramebufferDisplay> {
        let iface = Framebuffer::new("/dev/fb0")?;
        trace!(
            "init fb: var_screen_info: {:?}, fix_screen_info: {:?}",
            iface.var_screen_info,
            iface.fix_screen_info,
        );

        let background = iface.read_frame();
        let size = Size::new(iface.var_screen_info.xres, iface.var_screen_info.yres);

        let (xoffset, yoffset) = (
            iface.var_screen_info.xoffset as usize,
            iface.var_screen_info.yoffset as usize,
        );
        let width = size.width as usize;
        let height = size.height as usize;
        let bytes_per_pixel = iface.var_screen_info.bits_per_pixel / 8;
        let mut buffer = vec![0; width * height * bytes_per_pixel as usize];
        let buffer_size = buffer.len();
        let location = (yoffset * width + xoffset) * bytes_per_pixel as usize;
        buffer[..].copy_from_slice(&background[location..location + buffer_size]);

        Ok(FramebufferDisplay {
            framebuffer: Buffer {
                buffer,
                size,
                bytes_per_pixel,
            },
            iface,
            saved: None,
        })
    }
}

impl Display for FramebufferDisplay {
    fn map_pixels<F>(&mut self, mut f: F) -> Result<()>
    where
        F: FnMut(Color) -> Color,
    {
        self.framebuffer.buffer = self
            .framebuffer
            .buffer
            .chunks(self.framebuffer.bytes_per_pixel as usize)
            .flat_map(|raw| {
                // framebuffer should be divisible by bytespp, we don't have to worry about out of bounds
                let pixel = f(Color::new(raw[2], raw[1], raw[0]));
                [pixel.b(), pixel.g(), pixel.r(), raw[3]]
            })
            .collect();
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        let (xoffset, yoffset) = (
            self.iface.var_screen_info.xoffset as usize,
            self.iface.var_screen_info.yoffset as usize,
        );
        let width = self.framebuffer.size.width as usize;
        let location = (yoffset * width + xoffset) * self.framebuffer.bytes_per_pixel as usize;
        self.iface.frame[location..location + self.framebuffer.buffer.len()]
            .copy_from_slice(&self.framebuffer.buffer);
        Ok(())
    }

    fn save(&mut self) -> Result<()> {
        self.saved = Some(self.framebuffer.buffer.clone());
        Ok(())
    }

    fn load(&mut self, rect: Rect) -> Result<()> {
        let Some(ref saved) = self.saved else {
             bail!("No saved image");
        };

        let Rect {
            mut x,
            mut y,
            mut w,
            mut h,
        } = rect;
        x = x.max(0);
        y = y.max(0);
        w = w.min(self.framebuffer.size.width - x as u32);
        h = h.min(self.framebuffer.size.height - y as u32);

        for y in (y as u32)..(y as u32 + h) {
            let from = (y as u32 * self.framebuffer.size.width + x as u32) as usize
                * self.framebuffer.bytes_per_pixel as usize;
            let to = from + w as usize * self.framebuffer.bytes_per_pixel as usize;
            self.framebuffer.buffer[from..to].copy_from_slice(&saved[from..to]);
        }

        Ok(())
    }
}

impl DrawTarget for FramebufferDisplay {
    type Color = Color;
    type Error = anyhow::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<()>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        self.framebuffer
            .draw_iter(pixels)
            .map_err(|e| anyhow!("failed to draw: {}", e))
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<()>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        self.framebuffer
            .fill_contiguous(area, colors)
            .map_err(|e| anyhow!("failed to draw: {}", e))
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<()> {
        self.framebuffer
            .fill_solid(area, color)
            .map_err(|e| anyhow!("failed to draw: {}", e))
    }

    fn clear(&mut self, color: Self::Color) -> Result<()> {
        self.framebuffer
            .clear(color)
            .map_err(|e| anyhow!("failed to draw: {}", e))
    }
}

impl OriginDimensions for FramebufferDisplay {
    fn size(&self) -> Size {
        self.framebuffer.size
    }
}

impl DrawTarget for Buffer {
    type Color = Color;
    type Error = anyhow::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<()>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        let width = self.size.width as i32;
        let height = self.size.height as i32;
        let bytespp = self.bytes_per_pixel;

        for Pixel(coord, color) in pixels.into_iter() {
            let Point { x, y } = coord;
            if 0 <= x && x < width && 0 <= y && y < height {
                let index: u32 = (x as u32 + y as u32 * width as u32) * bytespp;
                self.buffer[index as usize] = color.b();
                self.buffer[index as usize + 1] = color.g();
                self.buffer[index as usize + 2] = color.r();
            }
        }

        Ok(())
    }
}

impl OriginDimensions for Buffer {
    fn size(&self) -> Size {
        self.size
    }
}
