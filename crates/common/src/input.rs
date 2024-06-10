use iced::keyboard::{Location, Modifiers};
use serde::{Deserialize, Serialize};

use crate::app::Gamepad;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Pressed(Key),
    Released(Key),
    Autorepeat(Key),
}

impl KeyEvent {
    pub fn into_message<T: Gamepad>(self) -> Option<T> {
        match self {
            KeyEvent::Pressed(key) if key != Key::Unknown => Some(T::key_press(key)),
            KeyEvent::Released(key) if key != Key::Unknown => Some(T::key_release(key)),
            KeyEvent::Autorepeat(_) => None,
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    X,
    Y,
    Start,
    Select,
    L,
    R,
    Menu,
    L2,
    R2,
    Power,
    VolDown,
    VolUp,
    Unknown,
}
