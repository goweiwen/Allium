use image::{Rgba, RgbaImage};

/// Draw rounded corners on an image.
pub fn round(image: &mut RgbaImage, color: Rgba<u8>, radius: u32) {
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
                let v = (radius_squared_1 - distance_squared) as f32
                    / (radius_squared_1 - radius_squared) as f32;
                image.put_pixel(x, y, blend(image.get_pixel(x, y), &color, v));
                image.put_pixel(
                    width - x - 1,
                    y,
                    blend(image.get_pixel(width - x - 1, y), &color, v),
                );
                image.put_pixel(
                    x,
                    height - y - 1,
                    blend(image.get_pixel(x, height - y - 1), &color, v),
                );
                image.put_pixel(
                    width - x - 1,
                    height - y - 1,
                    blend(image.get_pixel(width - x - 1, height - y - 1), &color, v),
                );
            }
        }
    }
}

fn blend(a: &Rgba<u8>, b: &Rgba<u8>, v: f32) -> Rgba<u8> {
    let v = v.clamp(0.0, 1.0);
    let r = (a[0] as f32 * v + b[0] as f32 * (1.0 - v)) as u8;
    let g = (a[1] as f32 * v + b[1] as f32 * (1.0 - v)) as u8;
    let b = (a[2] as f32 * v + b[2] as f32 * (1.0 - v)) as u8;
    Rgba([r, g, b, a[3]])
}
