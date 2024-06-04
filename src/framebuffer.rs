use bytemuck::cast_slice;

use crate::{HEIGHT, WIDTH};

pub struct Framebuffer {
    fb: framebuffer::Framebuffer,
}

impl Framebuffer {
    pub fn new() -> Self {
        let fb = framebuffer::Framebuffer::new("/dev/fb0").unwrap();

        Self { fb }
    }

    pub fn draw(&mut self, buffer: &[u32]) {
        self.fb.frame[0..WIDTH as usize * HEIGHT as usize * 4].copy_from_slice(cast_slice(buffer));
    }
}
