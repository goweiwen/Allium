use anyhow::Result;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::Size;
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use sdl2::keyboard::Keycode;

use crate::platform::{Key, KeyEvent};

pub const SCREEN_WIDTH: u32 = 640;
pub const SCREEN_HEIGHT: u32 = 480;

pub struct SimulatorPlatform {
    display: SimulatorDisplay<Rgb888>,
    window: Window,
}

impl SimulatorPlatform {
    pub fn new() -> Result<SimulatorPlatform> {
        let display = SimulatorDisplay::new(Size::new(SCREEN_WIDTH, SCREEN_HEIGHT));
        let output_settings = OutputSettingsBuilder::new().build();
        let mut window = Window::new("Hello World", &output_settings);

        window.update(&display);
        Ok(SimulatorPlatform { display, window })
    }

    pub async fn init() -> Result<()> {
        Ok(())
    }

    pub async fn poll(&mut self) -> Result<Option<KeyEvent>> {
        match self.window.events().next() {
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

    pub fn display(&mut self) -> Result<&mut SimulatorDisplay<Rgb888>> {
        Ok(&mut self.display)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.window.update(&self.display);
        Ok(())
    }

    pub fn display_size(&self) -> (i32, i32) {
        (640, 480)
    }

    pub fn update_battery(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn battery_percentage(&self) -> i32 {
        100
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
