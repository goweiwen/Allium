use std::{cell::OnceCell, env};

use anyhow::Result;
use bytemuck::cast_slice;

use crate::platform::Display;

pub const SCREEN_WIDTH: u32 = 640;
pub const SCREEN_HEIGHT: u32 = 480;

pub struct FramebufferDisplay {
    fb: framebuffer::Framebuffer,
    rotated_buf: Vec<u32>,
    should_rotate: OnceCell<bool>,
}

impl Default for FramebufferDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl FramebufferDisplay {
    pub fn new() -> Self {
        let fb = framebuffer::Framebuffer::new("/dev/fb0").unwrap();
        let should_rotate = OnceCell::new();
        let rotated_buf = if *should_rotate.get_or_init(|| env::var("ROTATE").is_ok()) {
            vec![0; SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize]
        } else {
            vec![]
        };

        Self {
            fb,
            rotated_buf,
            should_rotate,
        }
    }
}

impl Display for FramebufferDisplay {
    fn draw(&mut self, buffer: &[u32]) -> Result<()> {
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
        self.fb.frame[0..SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize * 4]
            .copy_from_slice(cast_slice(buf));
        Ok(())
    }
}
