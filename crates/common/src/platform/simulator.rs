use std::cell::RefCell;
use std::process;
use std::rc::Rc;
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::{Result, bail};
use async_trait::async_trait;
use embedded_graphics::image::{Image, ImageRaw};
use embedded_graphics::pixelcolor::raw::BigEndian;
use embedded_graphics::prelude::*;
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use image::buffer::ConvertBuffer;
use image::{ImageBuffer, Rgba};
use itertools::iproduct;
use log::{info, trace, warn};
use sdl2::keyboard::Keycode;

use crate::battery::Battery;
use crate::display::Display;
use crate::display::color::Color;
use crate::display::settings::DisplaySettings;
use crate::geom::Rect;
use crate::platform::{Key, KeyEvent, Platform};

static SCREEN_WIDTH: LazyLock<u32> = LazyLock::new(|| {
    std::env::var("WIDTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(752)
});

static SCREEN_HEIGHT: LazyLock<u32> = LazyLock::new(|| {
    std::env::var("HEIGHT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(560)
});

pub struct SimulatorPlatform {
    window: Rc<RefCell<Window>>,
}

#[async_trait(?Send)]
impl Platform for SimulatorPlatform {
    type Display = SimulatorWindow;
    type Battery = SimulatorBattery;
    type SuspendContext = ();

    fn new() -> Result<SimulatorPlatform> {
        let output_settings = OutputSettingsBuilder::new().scale(1).build();
        let window = Window::new("Allium Simulator", &output_settings);
        Ok(SimulatorPlatform {
            window: Rc::new(RefCell::new(window)),
        })
    }

    async fn poll(&mut self) -> KeyEvent {
        loop {
            let event = self.window.borrow_mut().events().next();
            if let Some(event) = event {
                match event {
                    SimulatorEvent::KeyDown {
                        keycode, repeat, ..
                    } => {
                        if keycode == Keycode::Q {
                            process::exit(0);
                        }
                        return if repeat {
                            KeyEvent::Autorepeat(Key::from(keycode))
                        } else {
                            KeyEvent::Pressed(Key::from(keycode))
                        };
                    }
                    SimulatorEvent::KeyUp { keycode, .. } => {
                        return KeyEvent::Released(Key::from(keycode));
                    }
                    SimulatorEvent::Quit => {
                        process::exit(0);
                    }
                    _ => {}
                }
            } else {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }

    fn display(&mut self) -> Result<SimulatorWindow> {
        let bg_path = format!("simulator/bg-{}x{}.png", *SCREEN_WIDTH, *SCREEN_HEIGHT);
        let display = SimulatorDisplay::load_png(&bg_path).unwrap_or_else(|_| {
            SimulatorDisplay::with_default_color(
                Size::new(*SCREEN_WIDTH, *SCREEN_HEIGHT),
                Color::new(0, 0, 0),
            )
        });
        Ok(SimulatorWindow {
            window: Rc::clone(&self.window),
            display,
            saved: Vec::new(),
        })
    }

    fn battery(&self) -> Result<SimulatorBattery> {
        Ok(SimulatorBattery::new())
    }

    fn shutdown(&self) -> Result<()> {
        process::exit(0);
    }

    fn suspend(&self) -> Result<Self::SuspendContext> {
        Ok(())
    }

    fn unsuspend(&self, _ctx: Self::SuspendContext) -> Result<()> {
        Ok(())
    }

    fn set_volume(&mut self, _volume: i32) -> Result<()> {
        Ok(())
    }

    fn get_brightness(&self) -> Result<u8> {
        Ok(50)
    }

    fn set_brightness(&mut self, _brightness: u8) -> Result<()> {
        Ok(())
    }

    fn set_display_settings(&mut self, _settings: &mut DisplaySettings) -> Result<()> {
        Ok(())
    }

    fn device_model() -> String {
        "Simulator".into()
    }

    fn firmware() -> String {
        "00000000".to_string()
    }

    fn has_wifi() -> bool {
        true
    }

    fn has_lid() -> bool {
        true
    }
}

impl Default for SimulatorPlatform {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

pub struct SimulatorWindow {
    window: Rc<RefCell<Window>>,
    display: SimulatorDisplay<Color>,
    saved: Vec<(Vec<u8>, u32)>,
}

impl Display for SimulatorWindow {
    fn map_pixels<F>(&mut self, mut f: F) -> Result<()>
    where
        F: FnMut(Color) -> Color,
    {
        let Size { width, height } = self.display.size();
        let pixels = iproduct!(0..width as i32, 0..height as i32)
            .map(|(x, y)| {
                let point = Point::new(x, y);
                let color = self.display.get_pixel(point);
                Pixel(point, f(color))
            })
            .collect::<Vec<_>>();
        self.display.draw_iter(pixels)?;
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.window.borrow_mut().update(&self.display);
        Ok(())
    }

    fn save(&mut self) -> Result<()> {
        let image = self
            .display
            .to_rgb_output_image(&OutputSettingsBuilder::new().build());
        let size = image.size();
        let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> = image.as_image_buffer().convert();
        self.saved.push((buffer.as_raw().to_vec(), size.width));
        Ok(())
    }

    fn load(&mut self, mut rect: Rect) -> Result<()> {
        let Some(saved) = &self.saved.last() else {
            bail!("No saved image");
        };

        let size = self.size();
        if rect.x < 0
            || rect.y < 0
            || rect.x as u32 + rect.w > size.width
            || rect.y as u32 + rect.h > size.height
        {
            warn!(
                "Area exceeds display bounds: x: {}, y: {}, w: {}, h: {}",
                rect.x, rect.y, rect.w, rect.h,
            );
            rect.x = rect.x.max(0);
            rect.y = rect.y.max(0);
            rect.w = rect.w.min(size.width - rect.x as u32);
            rect.h = rect.h.min(size.height - rect.h);
        }

        let image: ImageRaw<'_, _, BigEndian> = ImageRaw::new(&saved.0, saved.1);
        let image = image.sub_image(&rect.into());
        let image = Image::new(&image, rect.top_left().into());
        image.draw(&mut self.display)?;

        Ok(())
    }

    fn pop(&mut self) -> bool {
        self.saved.pop();
        !self.saved.is_empty()
    }
}

impl DrawTarget for SimulatorWindow {
    type Color = Color;
    type Error = anyhow::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> std::result::Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        let pixels: Vec<_> = pixels
            .into_iter()
            .filter_map(|p| {
                if p.0.x < 0
                    || p.0.y < 0
                    || p.0.x >= *SCREEN_WIDTH as i32
                    || p.0.y >= *SCREEN_HEIGHT as i32
                {
                    warn!("Pixel out of bounds: {:?}", p);
                    return None;
                }
                let curr = self.display.get_pixel(p.0);
                let color = p.1;

                let a = color.a() as u32;
                let a_inv = 255 - a;

                let b = (curr.b() as u32 * a_inv + color.b() as u32 * a) / 255;
                let g = (curr.g() as u32 * a_inv + color.g() as u32 * a) / 255;
                let r = (curr.r() as u32 * a_inv + color.r() as u32 * a) / 255;

                Some(Pixel(p.0, Color::new(r as u8, g as u8, b as u8)))
            })
            .collect();
        Ok(self.display.draw_iter(pixels)?)
    }
}

impl OriginDimensions for SimulatorWindow {
    fn size(&self) -> Size {
        self.display.size()
    }
}

impl From<Keycode> for Key {
    fn from(value: Keycode) -> Self {
        match value {
            Keycode::Up => Key::Up,
            Keycode::Down => Key::Down,
            Keycode::Left => Key::Left,
            Keycode::Right => Key::Right,
            Keycode::Space => Key::A,
            Keycode::LCtrl => Key::B,
            Keycode::LShift => Key::X,
            Keycode::LAlt => Key::Y,
            Keycode::Return => Key::Start,
            Keycode::RCtrl => Key::Select,
            Keycode::E => Key::L,
            Keycode::T => Key::R,
            Keycode::Escape => Key::Menu,
            Keycode::Tab => Key::L2,
            Keycode::Backspace => Key::R2,
            Keycode::Power => Key::Power,
            Keycode::LGui => Key::VolDown,
            Keycode::RGui => Key::VolUp,
            _ => Key::Unknown,
        }
    }
}

pub struct SimulatorBattery {
    percentage: i32,
    charging: bool,
}

impl SimulatorBattery {
    pub fn new() -> SimulatorBattery {
        SimulatorBattery {
            percentage: 100,
            charging: true,
        }
    }
}

impl Default for SimulatorBattery {
    fn default() -> Self {
        Self::new()
    }
}

impl Battery for SimulatorBattery {
    fn update(&mut self) -> Result<()> {
        trace!("Updating battery");
        if self.percentage > 0 {
            self.percentage -= 5
        }
        self.charging = !self.charging;
        Ok(())
    }

    fn percentage(&self) -> i32 {
        self.percentage
    }

    fn charging(&self) -> bool {
        self.charging
    }

    fn update_led(enabled: bool) {
        info!(
            "[Simulator] Charging LED: {}",
            if enabled { "ON" } else { "OFF" }
        );
    }
}
