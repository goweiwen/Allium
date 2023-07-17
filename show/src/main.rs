#![warn(clippy::all, rust_2018_idioms)]
#![feature(iter_array_chunks)]

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Parser;
use common::{display::color::Color, stylesheet::Stylesheet};
use framebuffer::Framebuffer;
use image::GenericImageView;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the image to display
    path: Option<PathBuf>,

    /// Whether to clear the screen
    #[arg(short, long)]
    clear: bool,

    /// Whether to darken the screen
    #[arg(short, long)]
    darken: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let styles = Stylesheet::load()?;
    let mut fb = Framebuffer::new("/dev/fb0")?;

    let vw = fb.var_screen_info.xres_virtual as usize;
    let vh = fb.var_screen_info.yres_virtual as usize;
    let bpp = fb.var_screen_info.bits_per_pixel as usize / 8;

    let mut frame = if !cli.clear {
        fb.read_frame().to_vec()
    } else {
        let r = styles.background_color.r();
        let g = styles.background_color.g();
        let b = styles.background_color.b();

        [b, g, r, 0xff]
            .into_iter()
            .cycle()
            .take(vw * vh * bpp)
            .collect()
    };

    if cli.darken {
        darken(&mut frame, styles.background_color, 192);
    }

    if let Some(path) = cli.path {
        show(&fb, &mut frame, path)?;
    }

    fb.write_frame(&frame);

    Ok(())
}

fn show(fb: &Framebuffer, frame: &mut [u8], path: impl AsRef<Path>) -> Result<()> {
    let x0 = fb.var_screen_info.xoffset as usize;
    let y0 = fb.var_screen_info.yoffset as usize;
    let w = fb.var_screen_info.xres as usize;
    let h = fb.var_screen_info.yres as usize;
    let bpp = fb.var_screen_info.bits_per_pixel as usize / 8;

    let mut image = image::io::Reader::open(path)?.decode()?;
    if image.width() != w as u32 || image.height() != h as u32 {
        let new_h = (h as u32).min(w as u32 * image.height() / image.width());
        image = image.resize_to_fill(w as u32, new_h, image::imageops::FilterType::Nearest);
    }

    for y in 0..h {
        for x in 0..w {
            let i = ((y0 + y) * w + (x0 + x)) * bpp;
            let pixel = image.get_pixel((w - x - 1) as u32, (h - y - 1) as u32);
            frame[i + 2] = pixel[0];
            frame[i + 1] = pixel[1];
            frame[i] = pixel[2];
        }
    }

    Ok(())
}

fn darken(frame: &mut [u8], color: Color, alpha: u8) {
    frame.iter_mut().array_chunks().for_each(|[b, g, r, _]| {
        let pixel = Color::new(*r, *g, *b);
        let color = pixel.blend(color.overlay(pixel), alpha);
        *b = color.b();
        *g = color.g();
        *r = color.r();
    });
}
