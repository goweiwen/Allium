#[cfg(target_arch = "arm")]
mod miyoo;
#[cfg(not(target_arch = "arm"))]
mod simulator;

use anyhow::Result;

#[cfg(target_arch = "arm")]
pub type DefaultPlatform = miyoo::MiyooPlatform;
#[cfg(not(target_arch = "arm"))]
pub type DefaultPlatform = simulator::SimulatorPlatform;

pub trait Platform {
    type Display;
    type Battery;

    fn new() -> Result<Self>
    where
        Self: Sized;
    fn display(&mut self) -> Result<Self::Display>;
    fn battery(&self) -> Result<Self::Battery>;
    async fn poll(&mut self) -> Result<Option<KeyEvent>>;
}

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
