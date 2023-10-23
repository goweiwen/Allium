#![warn(clippy::all, rust_2018_idioms)]

use std::{
    num::NonZeroU32,
    path::{Path, PathBuf},
};

use anyhow::Result;
use clap::Parser;
use framebuffer::Framebuffer;
use image::{imageops, Pixel, Rgb, RgbImage};
use sysfs_gpio::{Direction, Pin};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the image to display
    path: PathBuf,

    /// Whether to vibrate the device
    #[arg(short, long)]
    rumble: bool,

    /// Dimensions of the image
    #[arg(short, long)]
    width: Option<u32>,
    #[arg(short, long)]
    height: Option<u32>,

    /// Crop black borders
    #[arg(short, long)]
    crop: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.rumble {
        rumble(1)?;
    }

    if let Err(e) = screenshot(cli.path, cli.width, cli.height, cli.crop) {
        eprintln!("Error: {}", e);
    }

    if cli.rumble {
        rumble(0)?;
    }

    Ok(())
}

fn screenshot(
    path: impl AsRef<Path>,
    width: Option<u32>,
    height: Option<u32>,
    crop: bool,
) -> Result<()> {
    let fb = Framebuffer::new("/dev/fb0")?;

    let x0 = fb.var_screen_info.xoffset as usize;
    let y0 = fb.var_screen_info.yoffset as usize;
    let w = fb.var_screen_info.xres as usize;
    let h = fb.var_screen_info.yres as usize;
    let bpp = fb.var_screen_info.bits_per_pixel as usize / 8;

    let mut image = image::RgbImage::new(w as u32, h as u32);
    let frame = fb.read_frame();

    for y in 0..h {
        for x in 0..w {
            let i = ((y0 + y) * w + (x0 + x)) * bpp;
            let pixel = Rgb([frame[i + 2], frame[i + 1], frame[i]]);
            image.put_pixel((w - x - 1) as u32, (h - y - 1) as u32, pixel);
        }
    }

    if crop {
        let (x, y, w, h) = dbg!(cropped_bounding_box(&image));
        image = imageops::crop(&mut image, x, y, w, h).to_image();
    }

    let (width, height) = match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, w * image.height() / image.width()),
        (None, Some(h)) => (h * image.width() / image.height(), h),
        (None, None) => (image.width(), image.height()),
    };

    if width != image.width() || height != image.height() {
        let src_image = fast_image_resize::Image::from_vec_u8(
            NonZeroU32::new(image.width()).unwrap(),
            NonZeroU32::new(image.height()).unwrap(),
            image.into_vec(),
            fast_image_resize::PixelType::U8x3,
        )?;
        let mut dst_image = fast_image_resize::Image::new(
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
            src_image.pixel_type(),
        );
        let mut resizer = fast_image_resize::Resizer::new(
            fast_image_resize::ResizeAlg::Convolution(fast_image_resize::FilterType::Lanczos3),
        );
        resizer.resize(&src_image.view(), &mut dst_image.view_mut())?;
        let image = RgbImage::from_raw(width, height, dst_image.into_vec()).unwrap();
        image.save(path)?;
    } else {
        image.save(path)?;
    }

    Ok(())
}

fn rumble(val: u8) -> Result<()> {
    let pin = Pin::new(48);
    pin.export()?;
    pin.set_direction(Direction::Out)?;
    pin.set_value((val & 1) ^ 1)?;
    Ok(())
}

fn cropped_bounding_box(image: &RgbImage) -> (u32, u32, u32, u32) {
    if image.is_empty() {
        return (0, 0, 0, 0);
    }

    let mut top = 0;
    let mut left = 0;
    let mut right = 0;
    let mut bottom = 0;

    for y in 0..image.height() {
        for x in 0..image.width() {
            let pixel = image.get_pixel(x, y);
            if pixel.channels() != [0, 0, 0] && top == 0 {
                top = y;
                break;
            }
        }
    }

    for y in (0..image.height()).rev() {
        for x in (0..image.width()).rev() {
            let pixel = image.get_pixel(x, y);
            if pixel.channels() != [0, 0, 0] && bottom == 0 {
                bottom = y;
                break;
            }
        }
    }

    for x in 0..image.width() {
        for y in 0..image.height() {
            let pixel = image.get_pixel(x, y);
            if pixel.channels() != [0, 0, 0] && left == 0 {
                left = x;
                break;
            }
        }
    }

    for x in (0..image.width()).rev() {
        for y in (0..image.height()).rev() {
            let pixel = image.get_pixel(x, y);
            if pixel.channels() != [0, 0, 0] && right == 0 {
                right = x;
                break;
            }
        }
    }

    (left, top, right - left, bottom - top)
}
