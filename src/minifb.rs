use minifb::{Window, WindowOptions};

use crate::{HEIGHT, WIDTH};

pub struct MiniFb {
    window: Window,
}

impl MiniFb {
    pub fn new() -> Self {
        let window = Window::new(
            "Test",
            WIDTH as usize,
            HEIGHT as usize,
            WindowOptions::default(),
        )
        .unwrap();

        Self { window }
    }

    pub fn draw(&mut self, buffer: &[u32]) {
        self.window
            .update_with_buffer(buffer, WIDTH as usize, HEIGHT as usize)
            .unwrap();
    }
}
