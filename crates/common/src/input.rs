use iced::keyboard::{Location, Modifiers};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Pressed(Key),
    Released(Key),
    Autorepeat(Key),
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

impl From<Key> for iced::keyboard::Key {
    fn from(value: Key) -> Self {
        use iced::keyboard::key::Key::*;
        use iced::keyboard::key::Named::*;
        match value {
            Key::Up => Named(ArrowUp),
            Key::Down => Named(ArrowDown),
            Key::Left => Named(ArrowLeft),
            Key::Right => Named(ArrowRight),
            Key::A => Character("a".into()),
            Key::B => Character("b".into()),
            Key::X => Character("x".into()),
            Key::Y => Character("y".into()),
            Key::Start => Named(Enter),
            Key::Select => Named(Control),
            Key::L => Character("l".into()),
            Key::R => Character("r".into()),
            Key::Menu => Named(Escape),
            Key::L2 => Named(Tab),
            Key::R2 => Named(Backspace),
            Key::Power => Named(Power),
            Key::VolDown => Named(AudioVolumeDown),
            Key::VolUp => Named(AudioVolumeUp),
            Key::Unknown => Unidentified,
        }
    }
}

impl From<iced::keyboard::Key> for Key {
    fn from(value: iced::keyboard::Key) -> Self {
        use iced::keyboard::key::Key::*;
        use iced::keyboard::key::Named::*;
        match value {
            Named(ArrowUp) => Key::Up,
            Named(ArrowDown) => Key::Down,
            Named(ArrowLeft) => Key::Left,
            Named(ArrowRight) => Key::Right,
            Character(c) if c.as_str() == "a" => Key::A,
            Character(c) if c.as_str() == "b" => Key::B,
            Character(c) if c.as_str() == "x" => Key::X,
            Character(c) if c.as_str() == "y" => Key::Y,
            Named(Enter) => Key::Start,
            Named(Control) => Key::Select,
            Character(c) if c.as_str() == "l" => Key::L,
            Character(c) if c.as_str() == "r" => Key::R,
            Named(Escape) => Key::Menu,
            Named(Tab) => Key::L2,
            Named(Backspace) => Key::R2,
            Named(Power) => Key::Power,
            Named(AudioVolumeDown) => Key::VolDown,
            Named(AudioVolumeUp) => Key::VolUp,
            _ => Key::Unknown,
        }
    }
}

impl From<KeyEvent> for Option<iced::Event> {
    fn from(event: KeyEvent) -> Self {
        Some(iced::Event::Keyboard(match event {
            KeyEvent::Pressed(key) => iced::keyboard::Event::KeyPressed {
                key: key.into(),
                location: Location::Standard,
                modifiers: Modifiers::default(),
                text: None,
            },
            KeyEvent::Released(key) => iced::keyboard::Event::KeyReleased {
                key: key.into(),
                location: Location::Standard,
                modifiers: Modifiers::default(),
            },
            KeyEvent::Autorepeat(_) => return None,
        }))
    }
}
