#![warn(clippy::all, rust_2018_idioms)]

use std::{env, path::Path};

use anyhow::Result;
use framebuffer::Framebuffer;
use image::GenericImageView;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let path = args.next();

    match path {
        Some(path) => {
            if let Err(e) = show(path) {
                eprintln!("Error: {}", e);
            }
        }
        None => {
            if let Err(e) = clear() {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}

fn show(path: impl AsRef<Path>) -> Result<()> {
    let mut fb = Framebuffer::new("/dev/fb0")?;

    let x0 = fb.var_screen_info.xoffset as usize;
    let y0 = fb.var_screen_info.yoffset as usize;
    let w = fb.var_screen_info.xres as usize;
    let h = fb.var_screen_info.yres as usize;
    let vw = fb.var_screen_info.xres_virtual as usize;
    let vh = fb.var_screen_info.yres_virtual as usize;
    let bpp = fb.var_screen_info.bits_per_pixel as usize / 8;

    let mut image = image::io::Reader::open(path)?.decode()?;
    if image.width() != w as u32 || image.height() != h as u32 {
        let new_h = (h as u32).min(w as u32 * image.height() / image.width());
        image = image.resize_to_fill(w as u32, new_h, image::imageops::FilterType::Nearest);
    }

    let mut frame = vec![0u8; vw * vh * bpp];

    for y in 0..h {
        for x in 0..w {
            let i = ((y0 + y) * w + (x0 + x)) * bpp;
            let pixel = image.get_pixel((w - x - 1) as u32, (h - y - 1) as u32);
            frame[i + 2] = pixel[0];
            frame[i + 1] = pixel[1];
            frame[i] = pixel[2];
        }
    }

    fb.write_frame(&frame);

    Ok(())
}

fn clear() -> Result<()> {
    let mut fb = Framebuffer::new("/dev/fb0")?;

    let vw = fb.var_screen_info.xres_virtual as usize;
    let vh = fb.var_screen_info.yres_virtual as usize;
    let bpp = fb.var_screen_info.bits_per_pixel as usize / 8;

    let frame = vec![0u8; vw * vh * bpp];

    fb.write_frame(&frame);

    Ok(())
}
