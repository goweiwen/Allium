#[cfg(target_arch = "arm")]
mod miyoo;
#[cfg(not(target_arch = "arm"))]
mod simulator;

use std::error::Error;

use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{DrawTarget, OriginDimensions},
};

#[cfg(target_arch = "arm")]
pub type Platform = miyoo::MiyooPlatform;
#[cfg(not(target_arch = "arm"))]
pub type Platform = simulator::SimulatorPlatform;

pub trait Display<E: Error>: OriginDimensions + DrawTarget<Color = Rgb888, Error = E> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Pressed(Key),
    Released(Key),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
