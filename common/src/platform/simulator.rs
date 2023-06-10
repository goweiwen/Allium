use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{bail, Result};
use async_trait::async_trait;
use embedded_graphics::image::{Image, ImageRaw};
use embedded_graphics::pixelcolor::raw::BigEndian;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use itertools::iproduct;
use sdl2::keyboard::Keycode;

use crate::battery::Battery;
use crate::display::color::Color;
use crate::display::Display;
use crate::platform::{Key, KeyEvent, Platform};

pub const SCREEN_WIDTH: u32 = 640;
pub const SCREEN_HEIGHT: u32 = 480;

pub struct SimulatorPlatform {
    window: Rc<RefCell<Window>>,
}

#[async_trait(?Send)]
impl Platform for SimulatorPlatform {
    type Display = SimulatorWindow;
    type Battery = SimulatorBattery;

    fn new() -> Result<SimulatorPlatform> {
        let output_settings = OutputSettingsBuilder::new().scale(1).build();
        let window = Window::new("Allium Simulator", &output_settings);
        Ok(SimulatorPlatform {
            window: Rc::new(RefCell::new(window)),
        })
    }

    async fn poll(&mut self) -> Result<Option<KeyEvent>> {
        let event = self.window.borrow_mut().events().next();
        if let Some(event) = event {
            match event {
                SimulatorEvent::KeyDown {
                    keycode, repeat, ..
                } => Ok(Some(if repeat {
                    KeyEvent::Autorepeat(Key::from(keycode))
                } else {
                    KeyEvent::Pressed(Key::from(keycode))
                })),
                SimulatorEvent::KeyUp { keycode, .. } => {
                    Ok(Some(KeyEvent::Released(Key::from(keycode))))
                }
                _ => {
                    // Ignore other events
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    fn display(&mut self) -> Result<SimulatorWindow> {
        let display =
            SimulatorDisplay::load_png("assets/simulator/ingame.png").unwrap_or_else(|_| {
                SimulatorDisplay::with_default_color(
                    Size::new(SCREEN_WIDTH, SCREEN_HEIGHT),
                    Color::new(0, 0, 0),
                )
            });
        self.window.borrow_mut().update(&display);
        Ok(SimulatorWindow {
            window: Rc::clone(&self.window),
            display,
            saved: None,
        })
    }

    fn battery(&self) -> Result<SimulatorBattery> {
        Ok(SimulatorBattery::new())
    }

    fn set_volume(&mut self, _volume: i32) -> Result<()> {
        Ok(())
    }

    fn set_brightness(&mut self, _brightness: u8) -> Result<()> {
        Ok(())
    }

    fn device_model() -> String {
        "Simulator".to_string()
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
    saved: Option<(Vec<u8>, u32)>,
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
        self.display.draw_iter(pixels.into_iter())?;
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
        let buffer = image.as_image_buffer();
        self.saved = Some((buffer.as_raw().to_vec(), size.width));
        Ok(())
    }

    fn load(&mut self, area: Rectangle) -> Result<()> {
        let Some(saved) = &self.saved else {
            bail!("No saved image");
        };

        let image: ImageRaw<_, BigEndian> = ImageRaw::new(&saved.0, saved.1);
        let image = image.sub_image(&area);
        let image = Image::new(&image, area.top_left);
        image.draw(&mut self.display)?;

        Ok(())
    }
}

impl DrawTarget for SimulatorWindow {
    type Color = Color;
    type Error = <SimulatorDisplay<Color> as DrawTarget>::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> std::result::Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        self.display.draw_iter(pixels)
    }

    fn fill_contiguous<I>(
        &mut self,
        area: &Rectangle,
        colors: I,
    ) -> std::result::Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        self.display.fill_contiguous(area, colors)
    }

    fn fill_solid(
        &mut self,
        area: &Rectangle,
        color: Self::Color,
    ) -> std::result::Result<(), Self::Error> {
        self.display.fill_solid(area, color)
    }

    fn clear(&mut self, color: Self::Color) -> std::result::Result<(), Self::Error> {
        self.display.clear(color)
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
            charging: false,
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
        Ok(())
    }

    fn percentage(&self) -> i32 {
        self.percentage
    }

    fn charging(&self) -> bool {
        self.charging
    }
}
