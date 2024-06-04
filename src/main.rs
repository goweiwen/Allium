mod counter;

use iced::mouse::Cursor;
use iced::Element;
use iced::Font;
use iced::Pixels;
use iced::Theme;
use iced_runtime::core::clipboard;
use iced_runtime::core::mouse;
use iced_runtime::core::renderer;
use iced_runtime::core::Size;
use iced_runtime::user_interface::{self, UserInterface};
use iced_tiny_skia::Renderer;

use counter::Counter;

fn main() {
    let mut counter = Counter::new();
    let mut cache = user_interface::Cache::new();
    // let mut renderer = inySkiaRenderer, TinySkiaRenderer>();
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
    let mut window_size = Size::new(640.0, 480.0);
    let mut clipboard = clipboard::Null;

    let mut events = Vec::new();
    let mut messages = Vec::new();
    let mut theme = Theme::default();

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
        let event_statuses = user_interface.update(
            &events,
            cursor,
            &mut renderer,
            &mut clipboard,
            &mut messages,
        );

        // Draw the user interface
        let mouse_interaction =
            user_interface.draw(&mut renderer, &theme, &renderer::Style::default(), cursor);

        cache = user_interface.into_cache();

        for message in messages.drain(..) {
            counter.update(message);
        }

        // Update mouse cursor icon...
        // Flush rendering operations...
    }
}
