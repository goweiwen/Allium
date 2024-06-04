use std::{cell::OnceCell, env};

use bytemuck::cast_slice;

use crate::{HEIGHT, WIDTH};

pub struct Framebuffer {
    fb: framebuffer::Framebuffer,
    rotated_buf: Vec<u32>,
    should_rotate: OnceCell<bool>,
}

impl Framebuffer {
    pub fn new() -> Self {
        let fb = framebuffer::Framebuffer::new("/dev/fb0").unwrap();
        let should_rotate = OnceCell::new();
        let rotated_buf = if *should_rotate.get_or_init(|| env::var("ROTATE").is_ok()) {
            vec![0; WIDTH as usize * HEIGHT as usize]
        } else {
            vec![]
        };

        Self {
            fb,
            rotated_buf,
            should_rotate,
        }
    }

    pub fn draw(&mut self, buffer: &[u32]) {
        let buf = if *self
            .should_rotate
            .get_or_init(|| env::var("ROTATE").is_ok())
        {
            self.rotated_buf.copy_from_slice(buffer);
            self.rotated_buf.reverse();
            &self.rotated_buf
        } else {
            buffer
        };
        self.fb.frame[0..WIDTH as usize * HEIGHT as usize * 4].copy_from_slice(cast_slice(buf));
    }
}
