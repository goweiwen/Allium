mod miyoo283;
mod miyoo354;

use anyhow::Result;

pub use miyoo283::Miyoo283Battery;
pub use miyoo354::Miyoo354Battery;

pub trait Battery {
    fn update(&mut self) -> Result<()>;
    fn percentage(&self) -> i32;
    fn is_charging(&self) -> bool;
}
