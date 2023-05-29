use anyhow::Result;

pub mod framebuffer;

pub trait Display {
    /// Initialize the display for use. Will be run on boot.
    async fn init(&self) -> Result<()>;

    /// Flush the display, if applicable.
    fn flush(&mut self) -> Result<()>;
}
