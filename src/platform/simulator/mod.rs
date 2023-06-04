use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::{DrawTarget, OriginDimensions, Size};
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use sdl2::keyboard::Keycode;

use crate::battery::Battery;
use crate::display::Display;
use crate::platform::{Key, KeyEvent, Platform};

pub const SCREEN_WIDTH: u32 = 640;
pub const SCREEN_HEIGHT: u32 = 480;

pub struct SimulatorPlatform {
    window: Rc<RefCell<Window>>,
}

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
        match self.window.borrow_mut().events().next() {
            Some(SimulatorEvent::KeyDown { keycode, .. }) => {
                Ok(Some(KeyEvent::Pressed(Key::from(keycode))))
            }
            Some(SimulatorEvent::KeyUp { keycode, .. }) => {
                Ok(Some(KeyEvent::Released(Key::from(keycode))))
            }
            Some(_) => {
                // Ignore other events
                Ok(None)
            }
            None => Ok(None),
        }
    }

    fn display(&mut self) -> Result<SimulatorWindow> {
        let display = SimulatorDisplay::new(Size::new(SCREEN_WIDTH, SCREEN_HEIGHT));
        self.window.borrow_mut().update(&display);
        Ok(SimulatorWindow {
            window: Rc::clone(&self.window),
            display,
        })
    }

    fn battery(&self) -> Result<SimulatorBattery> {
        Ok(SimulatorBattery::new())
    }

    fn set_volume(&mut self, _volume: u8) -> Result<()> {
        Ok(())
    }
}

impl Default for SimulatorPlatform {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

pub struct SimulatorWindow {
    window: Rc<RefCell<Window>>,
    display: SimulatorDisplay<Rgb888>,
}

impl Display<<SimulatorWindow as DrawTarget>::Error> for SimulatorWindow {
    fn flush(&mut self) -> Result<()> {
        self.window.borrow_mut().update(&self.display);
        Ok(())
    }
}

impl DrawTarget for SimulatorWindow {
    type Color = Rgb888;
    type Error = <SimulatorDisplay<Rgb888> as DrawTarget>::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> std::result::Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        self.display.draw_iter(pixels)
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
