use std::fs::File;
use std::io::Read;
use std::time::Duration;

use anyhow::Result;
use evdev::{Device, EventStream, EventType};
use log::info;

use crate::constants::MAXIMUM_FRAME_TIME;
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};

impl From<u16> for Key {
    fn from(code: u16) -> Self {
        use evdev::KeyCode;
        match KeyCode(code) {
            KeyCode::KEY_UP => Key::Up,
            KeyCode::KEY_DOWN => Key::Down,
            KeyCode::KEY_LEFT => Key::Left,
            KeyCode::KEY_RIGHT => Key::Right,
            KeyCode::KEY_SPACE => Key::A,
            KeyCode::KEY_LEFTCTRL => Key::B,
            KeyCode::KEY_LEFTSHIFT => Key::X,
            KeyCode::KEY_LEFTALT => Key::Y,
            KeyCode::KEY_ENTER => Key::Start,
            KeyCode::KEY_RIGHTCTRL => Key::Select,
            KeyCode::KEY_E => Key::L,
            KeyCode::KEY_T => Key::R,
            KeyCode::KEY_ESC => Key::Menu,
            KeyCode::KEY_TAB => Key::L2,
            KeyCode::KEY_BACKSPACE => Key::R2,
            KeyCode::KEY_POWER => Key::Power,
            KeyCode::KEY_VOLUMEDOWN => Key::VolDown,
            KeyCode::KEY_VOLUMEUP => Key::VolUp,
            _ => Key::Unknown,
        }
    }
}

pub struct EvdevKeys {
    pub events: EventStream,
    lid_switch_poller: Option<LidSwitchPoller>,
}

impl EvdevKeys {
    pub fn new() -> Result<Self> {
        Ok(Self {
            events: Device::open("/dev/input/event0")
                .unwrap()
                .into_event_stream()?,
            lid_switch_poller: DefaultPlatform::has_lid().then(LidSwitchPoller::new),
        })
    }

    pub async fn poll(&mut self) -> KeyEvent {
        loop {
            if let Some(lid_event) = self.lid_switch_poller.as_mut().and_then(|lid| lid.poll()) {
                info!("Lid event detected: {:?}", lid_event);
                return lid_event;
            }

            let timeout =
                tokio::time::timeout(Duration::from_millis(500), self.events.next_event());
            let Ok(result) = timeout.await else {
                continue;
            };
            let event = result.unwrap();
            if event.event_type() == EventType::KEY {
                let key = event.code();
                let key: Key = key.into();
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

struct LidSwitchPoller {
    is_lid_open: bool,
}

impl LidSwitchPoller {
    fn new() -> Self {
        let is_lid_open = read_is_lid_open().expect("Failed to read lid switch state");
        Self { is_lid_open }
    }

    fn poll(&mut self) -> Option<KeyEvent> {
        let is_lid_open = read_is_lid_open().expect("Failed to read lid switch state");
        if is_lid_open != self.is_lid_open {
            self.is_lid_open = is_lid_open;
            if is_lid_open {
                Some(KeyEvent::Released(Key::LidClose))
            } else {
                Some(KeyEvent::Pressed(Key::LidClose))
            }
        } else {
            None
        }
    }
}

fn read_is_lid_open() -> Result<bool> {
    let mut file = File::open("/sys/devices/soc0/soc/soc:hall-mh248/hallvalue")
        .expect("Failed to open /sys/devices/soc0/soc/soc:hall-mh248/hallvalue");
    let mut buffer = [0u8; 2];
    file.read_exact(&mut buffer)?;
    Ok(buffer[0] == b'1')
}
