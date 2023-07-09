#![warn(clippy::all, rust_2018_idioms)]

use std::{env, path::Path};

use anyhow::Result;
use framebuffer::Framebuffer;
use image::Rgb;
use sysfs_gpio::{Direction, Pin};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("Usage: screenshot <path>");
            return Ok(());
        }
    };

    rumble(1)?;
    if let Err(e) = screenshot(path) {
        eprintln!("Error: {}", e);
    }
    rumble(0)?;

    Ok(())
}

fn screenshot(path: impl AsRef<Path>) -> Result<()> {
    let fb = Framebuffer::new("/dev/fb0")?;

    let x0 = fb.var_screen_info.xoffset as usize;
    let y0 = fb.var_screen_info.yoffset as usize;
    let x = 0;
    let y = 0;
    let w = fb.var_screen_info.xres as usize;
    let h = fb.var_screen_info.yres as usize;
    let bpp = fb.var_screen_info.bits_per_pixel as usize / 8;

    let mut image = image::RgbImage::new(w as u32, h as u32);
    let frame = fb.read_frame();

    for y in y..y + h {
        for x in x..x + w {
            let i = ((y0 + y) * w + (x0 + x)) * bpp;
            let pixel = Rgb([frame[i + 2], frame[i + 1], frame[i]]);
            image.put_pixel((w - x - 1) as u32, (h - y - 1) as u32, pixel);
        }
    }

    image.save(path)?;

    Ok(())
}

fn rumble(val: u8) -> Result<()> {
    let pin = Pin::new(48);
    pin.export()?;
    pin.set_direction(Direction::Out)?;
    pin.set_value((val & 1) ^ 1)?;
    Ok(())
}
