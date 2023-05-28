use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{DrawTarget, OriginDimensions, RgbColor, Size},
    Pixel,
};
use framebuffer::Framebuffer;

pub struct FrameBufferDisplay {
    framebuffer: Vec<u8>,
    iface: Framebuffer,
}

impl FrameBufferDisplay {
    pub fn new() -> FrameBufferDisplay {
        let framebuffer = Framebuffer::new("/dev/fb0").unwrap();
        let h = framebuffer.var_screen_info.yres;
        let line_length = framebuffer.fix_screen_info.line_length;

        FrameBufferDisplay {
            framebuffer: vec![0u8; (line_length * h * 3) as usize],
            iface: Framebuffer::new("/dev/fb0").unwrap(),
        }
    }

    pub fn flush(&mut self) {
        self.iface.write_frame(&self.framebuffer);
    }
}

impl DrawTarget for FrameBufferDisplay {
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        let xres = self.iface.var_screen_info.xres as i32;
        let yres = self.iface.var_screen_info.yres as i32;
        let bytespp = self.iface.var_screen_info.bits_per_pixel / 8;

        for Pixel(coord, color) in pixels.into_iter() {
            // rotate 180 degrees
            let x: i32 = xres - coord.x;
            let y: i32 = yres - coord.y;
            if 0 <= x && x < xres && 0 <= y && y < yres {
                let index: u32 = (x as u32 + y as u32 * xres as u32) * bytespp;
                self.framebuffer[index as usize] = color.b();
                self.framebuffer[index as usize + 1] = color.g();
                self.framebuffer[index as usize + 2] = color.r();
            }
        }

        Ok(())
    }
}

impl OriginDimensions for FrameBufferDisplay {
    fn size(&self) -> Size {
        Size::new(
            self.iface.var_screen_info.xres,
            self.iface.var_screen_info.yres,
        )
    }
}
