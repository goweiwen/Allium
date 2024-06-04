mod counter;
mod fonts;
mod framebuffer;
// mod minifb;

use bytemuck::cast_slice;
use bytemuck::cast_slice_mut;
use iced::font::Family;
use iced::font::Weight;
use iced::mouse::Cursor;
use iced::Color;
use iced::Font;
use iced::Pixels;
use iced::Point;
use iced::Rectangle;
use iced::Theme;
use iced_graphics::Viewport;
use iced_runtime::core::clipboard;
use iced_runtime::core::renderer;
use iced_runtime::core::Size;
use iced_runtime::user_interface::{self, UserInterface};
use iced_tiny_skia::Renderer;
use tiny_skia::Mask;
use tiny_skia::PixmapMut;

use crate::counter::Counter;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;

fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();

    fonts::load_fonts();

    let mut platform = framebuffer::Framebuffer::new();
    // let mut platform = minifb::MiniFb::new();
    let mut buffer: Vec<u32> = vec![0; WIDTH as usize * HEIGHT as usize];
    let mut pixels = PixmapMut::from_bytes(cast_slice_mut(&mut buffer), WIDTH, HEIGHT).unwrap();
    let mut mask = Mask::new(WIDTH, HEIGHT).unwrap();
    let viewport = Viewport::with_physical_size(Size::new(WIDTH, HEIGHT), 1.0);

    let mut counter = Counter::new();
    let mut cache = user_interface::Cache::new();
    let mut renderer = Renderer::new(
        Font {
            family: Family::SansSerif,
            weight: Weight::Bold,
            ..Default::default()
        },
        Pixels(16.0),
    );
    let window_size = Size::new(640.0, 480.0);
    let mut clipboard = clipboard::Null;

    let mut events = Vec::new();
    let mut messages = Vec::new();
    let theme = Theme::default();

    let cursor = Cursor::Unavailable;

    loop {
        // Obtain system events...

        let mut user_interface = UserInterface::<'_, _, _, Renderer>::build(
            counter.view(),
            window_size,
            cache,
            &mut renderer,
        );

        // Update the user interface
        let (state, mut event_statuses) = user_interface.update(
            &events,
            cursor,
            &mut renderer,
            &mut clipboard,
            &mut messages,
        );
        for (i, status) in event_statuses.into_iter().enumerate() {
            if let iced::event::Status::Ignored = status {
                println!("Ignored event: {:?}", events[i]);
            }
        }
        events.clear();

        // Draw the user interface
        let _ = user_interface.draw(&mut renderer, &theme, &renderer::Style::default(), cursor);

        cache = user_interface.into_cache();

        for message in messages.drain(..) {
            counter.update(message);
        }

        // Render to framebuffer
        renderer.draw::<&str>(
            &mut pixels,
            &mut mask,
            &viewport,
            &[Rectangle::new(
                Point::new(0.0, 0.0),
                Size::new(WIDTH as f32, HEIGHT as f32),
            )],
            Color::WHITE,
            &[],
        );

        // Flush rendering operations...
        platform.draw(cast_slice(pixels.as_ref().data()));
    }
}
