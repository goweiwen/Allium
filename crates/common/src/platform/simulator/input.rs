use tokio::sync::mpsc::Sender;

use crate::input::{Key, KeyEvent};

pub struct SimulatorInput {
    tx: Sender<KeyEvent>,
}

impl SimulatorInput {
    pub fn new(tx: Sender<KeyEvent>) -> Self {
        Self { tx }
    }
}

impl minifb::InputCallback for SimulatorInput {
    fn add_char(&mut self, _uni_char: u32) {}

    fn set_key_state(&mut self, key: minifb::Key, state: bool) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let key = key.into();
            let event = match state {
                true => KeyEvent::Pressed(key),
                false => KeyEvent::Released(key),
            };
            tx.send(event).await.unwrap();
        });
    }
}

impl From<minifb::Key> for Key {
    fn from(value: minifb::Key) -> Self {
        match value {
            minifb::Key::Up => Key::Up,
            minifb::Key::Down => Key::Down,
            minifb::Key::Left => Key::Left,
            minifb::Key::Right => Key::Right,
            minifb::Key::Space => Key::A,
            minifb::Key::LeftCtrl => Key::B,
            minifb::Key::LeftShift => Key::X,
            minifb::Key::LeftAlt => Key::Y,
            minifb::Key::Enter => Key::Start,
            minifb::Key::RightCtrl => Key::Select,
            minifb::Key::E => Key::L,
            minifb::Key::T => Key::R,
            minifb::Key::Escape => Key::Menu,
            minifb::Key::Tab => Key::L2,
            minifb::Key::Backspace => Key::R2,
            // minifb::Key::Power => Key::Power,
            // minifb::Key::VolumeDown => Key::VolDown,
            // minifb::Key::VolumeUp => Key::VolUp,
            _ => Key::Unknown,
        }
    }
}
