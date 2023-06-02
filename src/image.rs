use image::{ImageBuffer, Rgb};

/// Draw rounded corners on an image.
pub fn round(image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, color: Rgb<u8>, radius: u32) {
    let (width, height) = image.dimensions();

    let radius_squared = radius.pow(2) as i32;

    // Draw the corners.
    for x in 0..radius {
        for y in 0..radius {
            if (x as i32 - radius as i32).pow(2) + (y as i32 - radius as i32).pow(2)
                > radius_squared
            {
                image.put_pixel(x, y, color);
                image.put_pixel(width - x - 1, y, color);
                image.put_pixel(x, height - y - 1, color);
                image.put_pixel(width - x - 1, height - y - 1, color);
            }
        }
    }
}
