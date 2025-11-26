use image::{Rgba, RgbaImage};
use tiny_skia::PixmapMut;

use crate::display::color::Color;
use crate::geom::Point;

/// Draw an RGBA image to a pixmap with alpha blending.
///
/// This function performs proper alpha compositing:
/// - Opaque pixels (alpha=255) are copied directly
/// - Transparent pixels (alpha=0) are skipped
/// - Semi-transparent pixels are blended with the destination
pub fn draw_image(pixmap: &mut PixmapMut<'_>, image: &RgbaImage, point: Point) {
    let pixmap_width = pixmap.width() as i32;
    let pixmap_height = pixmap.height() as i32;

    for y in 0..image.height() {
        for x in 0..image.width() {
            let px = point.x + x as i32;
            let py = point.y + y as i32;

            if px >= 0 && px < pixmap_width && py >= 0 && py < pixmap_height {
                let pixel = image.get_pixel(x, y);
                let src = Color::rgba(pixel[0], pixel[1], pixel[2], pixel[3]);

                if src.a() > 0 {
                    let pixmap_idx = (py * pixmap_width + px) as usize;
                    let pixels = pixmap.pixels_mut();

                    if src.a() == 255 {
                        // Opaque, just write directly
                        pixels[pixmap_idx] = src.into();
                    } else {
                        // Alpha blend with existing pixel
                        let dst: Color = pixels[pixmap_idx].into();

                        let src_a = src.a() as u32;
                        let dst_a = dst.a() as u32;
                        let inv_src_a = 255 - src_a;

                        let out_a = src_a + (dst_a * inv_src_a) / 255;

                        if out_a == 0 {
                            pixels[pixmap_idx] = Color::rgba(0, 0, 0, 0).into();
                        } else {
                            let out_r = ((src.r() as u32 * src_a
                                + dst.r() as u32 * dst_a * inv_src_a / 255)
                                / out_a) as u8;
                            let out_g = ((src.g() as u32 * src_a
                                + dst.g() as u32 * dst_a * inv_src_a / 255)
                                / out_a) as u8;
                            let out_b = ((src.b() as u32 * src_a
                                + dst.b() as u32 * dst_a * inv_src_a / 255)
                                / out_a) as u8;

                            pixels[pixmap_idx] =
                                Color::rgba(out_r, out_g, out_b, out_a as u8).into();
                        }
                    }
                }
            }
        }
    }
}

/// Draw rounded corners on an image.
pub fn round(image: &mut RgbaImage, radius: u32) {
    let color = Rgba([0, 0, 0, 0]);

    let (width, height) = image.dimensions();

    let radius_squared = radius.pow(2) as i32;
    let radius_squared_1 = (radius + 1).pow(2) as i32;

    // Draw the corners.
    for x in 0..radius + 1 {
        for y in 0..radius + 1 {
            let distance_squared =
                (x as i32 - radius as i32).pow(2) + (y as i32 - radius as i32).pow(2);
            if distance_squared > radius_squared_1 {
                image.put_pixel(x, y, color);
                image.put_pixel(width - x - 1, y, color);
                image.put_pixel(x, height - y - 1, color);
                image.put_pixel(width - x - 1, height - y - 1, color);
            } else if distance_squared > radius_squared {
                // Rough approximation of the coverage of the pixel by the circle.
                let v = (radius_squared_1 - distance_squared) * 255
                    / (radius_squared_1 - radius_squared);

                let pixel = image.get_pixel(x, y);
                image.put_pixel(
                    x,
                    y,
                    Rgba([
                        pixel[0],
                        pixel[1],
                        pixel[2],
                        (v * pixel[3] as i32 / 255) as u8,
                    ]),
                );

                {
                    let x = width - x - 1;
                    let pixel = image.get_pixel(x, y);
                    image.put_pixel(
                        x,
                        y,
                        Rgba([
                            pixel[0],
                            pixel[1],
                            pixel[2],
                            (v * pixel[3] as i32 / 255) as u8,
                        ]),
                    );
                }

                {
                    let y = height - y - 1;
                    let pixel = image.get_pixel(x, y);
                    image.put_pixel(
                        x,
                        y,
                        Rgba([
                            pixel[0],
                            pixel[1],
                            pixel[2],
                            (v * pixel[3] as i32 / 255) as u8,
                        ]),
                    );
                }

                {
                    let x = width - x - 1;
                    let y = height - y - 1;
                    let pixel = image.get_pixel(x, y);
                    image.put_pixel(
                        x,
                        y,
                        Rgba([
                            pixel[0],
                            pixel[1],
                            pixel[2],
                            (v * pixel[3] as i32 / 255) as u8,
                        ]),
                    );
                }
            }
        }
    }
}
