use anyhow::Result;

pub trait Battery {
    fn update(&mut self) -> Result<()>;
    fn percentage(&self) -> i32;
    fn charging(&self) -> bool;
}
