#[cfg(target_arch = "arm")]
mod miyoo;
#[cfg(not(target_arch = "arm"))]
mod simulator;

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::pixelcolor;

#[cfg(target_arch = "arm")]
pub type DefaultPlatform = miyoo::MiyooPlatform;
#[cfg(target_arch = "arm")]
pub type Color = pixelcolor::Bgr888;

#[cfg(not(target_arch = "arm"))]
pub type DefaultPlatform = simulator::SimulatorPlatform;
#[cfg(not(target_arch = "arm"))]
pub type Color = pixelcolor::Rgb888;

// Platform is not threadsafe because it is ?Send
#[async_trait(?Send)]
pub trait Platform {
    type Display;
    type Battery;

    fn new() -> Result<Self>
    where
        Self: Sized;
    fn display(&mut self) -> Result<Self::Display>;
    fn battery(&self) -> Result<Self::Battery>;
    async fn poll(&mut self) -> Result<Option<KeyEvent>>;
    fn set_volume(&mut self, volume: i32) -> Result<()>;
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
