use linuxfb::Framebuffer;
use slint::{
    platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType, Rgb565Pixel},
        Platform,
    },
    PhysicalSize,
};
use std::rc::Rc;
use std::time::Duration;

slint::include_modules!();

struct FramebufferPlatform {
    window: Rc<MinimalSoftwareWindow>,
    fb: Framebuffer,
    stride: usize,
}

impl FramebufferPlatform {
    fn new(fb: Framebuffer) -> Self {
        let size = fb.get_size();
        let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
        window.set_size(PhysicalSize::new(size.0, size.1));
        Self {
            window,
            fb,
            stride: size.0 as usize,
        }
    }
}

impl Platform for FramebufferPlatform {
    fn create_window_adapter(
        &self,
    ) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn run_event_loop(&self) -> Result<(), slint::PlatformError> {
        loop {
            slint::platform::update_timers_and_animations();

            self.window.draw_if_needed(|renderer| {
                let mut frame = self.fb.map().unwrap();
                let (_, pixels, _) = unsafe { frame.align_to_mut::<Rgb565Pixel>() };
                renderer.render(pixels, self.stride);
            });

            if !self.window.has_active_animations() {
                std::thread::sleep(
                    slint::platform::duration_until_next_timer_update()
                        .unwrap_or(Duration::from_secs(1)),
                );
            }
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let fb_path = "/dev/fb0";

    slint::platform::set_platform(Box::new(FramebufferPlatform::new(
        Framebuffer::new(fb_path).expect("open framebuffer"),
    )))
    .expect("set platform");

    let ui = Main::new()?;

    let ui_handle = ui.as_weak();
    ui.on_request_increase_value(move || {
        let ui = ui_handle.unwrap();
        ui.set_counter(ui.get_counter() + 1);
    });

    ui.run()
}
