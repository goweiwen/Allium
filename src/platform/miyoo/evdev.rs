use anyhow::Result;
use evdev::{Device, EventStream, EventType};

use crate::platform::{Key, KeyEvent};

impl From<evdev::Key> for Key {
    fn from(value: evdev::Key) -> Self {
        match value {
            evdev::Key::KEY_UP => Key::Up,
            evdev::Key::KEY_DOWN => Key::Down,
            evdev::Key::KEY_LEFT => Key::Left,
            evdev::Key::KEY_RIGHT => Key::Right,
            evdev::Key::KEY_SPACE => Key::A,
            evdev::Key::KEY_LEFTCTRL => Key::B,
            evdev::Key::KEY_LEFTSHIFT => Key::X,
            evdev::Key::KEY_LEFTALT => Key::Y,
            evdev::Key::KEY_ENTER => Key::Start,
            evdev::Key::KEY_RIGHTCTRL => Key::Select,
            evdev::Key::KEY_E => Key::L,
            evdev::Key::KEY_T => Key::R,
            evdev::Key::KEY_ESC => Key::Menu,
            evdev::Key::KEY_TAB => Key::L2,
            evdev::Key::KEY_BACKSPACE => Key::R2,
            evdev::Key::KEY_POWER => Key::Power,
            evdev::Key::KEY_LEFTMETA => Key::VolDown,
            evdev::Key::KEY_RIGHTMETA => Key::VolUp,
            _ => Key::Unknown,
        }
    }
}

pub struct EvdevKeys {
    pub events: EventStream,
}

impl EvdevKeys {
    pub fn new() -> Result<Self> {
        Ok(Self {
            events: Device::open("/dev/input/event0")
                .unwrap()
                .into_event_stream()?,
        })
    }

    pub async fn poll(&mut self) -> Result<Option<KeyEvent>> {
        let event = self.events.next_event().await?;
        match event.event_type() {
            EventType::KEY => {
                let key = event.code();
                let key: Key = evdev::Key(key).into();
                return Ok(Some(match event.value() {
                    0 => KeyEvent::Pressed(key),
                    _ => KeyEvent::Released(key),
                }));
            }
            _ => {}
        }
        Ok(None)
    }
}
