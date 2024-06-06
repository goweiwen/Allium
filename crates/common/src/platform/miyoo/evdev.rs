use anyhow::Result;
use evdev::{Device, EventStream, EventType};

use crate::constants::MAXIMUM_FRAME_TIME;
use crate::input::{Key, KeyEvent};

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
            evdev::Key::KEY_VOLUMEDOWN => Key::VolDown,
            evdev::Key::KEY_VOLUMEUP => Key::VolUp,
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

    pub async fn poll(&mut self) -> KeyEvent {
        loop {
            let event = self.events.next_event().await.unwrap();
            if event.event_type() == EventType::KEY {
                let key = event.code();
                let key: Key = evdev::Key(key).into();
                if event.timestamp().elapsed().unwrap() > MAXIMUM_FRAME_TIME {
                    continue;
                }
                return match event.value() {
                    0 => KeyEvent::Released(key),
                    1 => KeyEvent::Pressed(key),
                    2 => KeyEvent::Autorepeat(key),
                    _ => unreachable!(),
                };
            }
        }
    }
}
