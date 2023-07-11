use image::{Rgba, RgbaImage};

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
