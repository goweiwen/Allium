mod fonts;
pub mod widgets;

use anyhow::Result;
use bytemuck::{cast_slice, cast_slice_mut};
use iced::{mouse::Cursor, program, Event, Point, Rectangle, Size};
use iced_core::clipboard;
use iced_graphics::Viewport;
use iced_runtime::{user_interface, UserInterface};
use log::{info, trace};
use tiny_skia::{Mask, PixmapMut};
use tokio::time::Interval;

use crate::platform::{DefaultPlatform, Display, Platform};

pub type Element<'a, Message> =
    iced_core::Element<'a, Message, iced::Theme, iced_tiny_skia::Renderer>;

pub type Theme = iced::Theme;
pub type Renderer = iced_tiny_skia::Renderer;

pub async fn run<App, State, Theme, Message, Style>(app: App, style: Style) -> Result<()>
where
    App: program::Title<State>
        + program::Update<State, Message>
        + for<'a> program::View<'a, State, Message, Theme, Renderer>
        + 'static,
    State: Default,
    Theme: Default,
    Style: crate::style::Style + Default,
{
    fonts::load_fonts();

    let mut state = State::default();
    info!("{}", app.title(&state));

    let mut platform = DefaultPlatform::new()?;

    let mut cache = user_interface::Cache::new();
    let mut renderer = Renderer::new(style.font(), style.font_size().into());
    let window_size = Size::new(640.0, 480.0);
    let mut clipboard = clipboard::Null;

    let mut messages = Vec::new();
    let theme = Theme::default();

    let cursor = Cursor::Unavailable;

    // Rendering
    let mut buffer: Vec<u32> = vec![0; window_size.width as usize * window_size.height as usize];
    let mut pixels = PixmapMut::from_bytes(
        cast_slice_mut(&mut buffer),
        window_size.width as u32,
        window_size.height as u32,
    )
    .unwrap();
    let mut mask = Mask::new(window_size.width as u32, window_size.height as u32).unwrap();
    let viewport = Viewport::with_physical_size(
        Size::new(window_size.width as u32, window_size.height as u32),
        1.0,
    );

    #[cfg(unix)]
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

    let mut frame_interval =
        tokio::time::interval(tokio::time::Duration::from_secs_f64(1.0 / 60.0));
    frame_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut events;
    loop {
        tokio::select! {
            _ = sigterm.recv() => {
                std::process::exit(0);
            }
            e = poll_until(&mut platform, &mut frame_interval) => {
                events = e;
            }
        }

        let mut user_interface = UserInterface::<'_, _, _, Renderer>::build(
            app.view(&state),
            window_size,
            cache,
            &mut renderer,
        );

        // Update the user interface
        let (_, event_statuses) = user_interface.update(
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

        // Draw the user interface
        let _ = user_interface.draw(
            &mut renderer,
            &theme,
            &iced_core::renderer::Style {
                text_color: style.text_color(),
            },
            cursor,
        );

        cache = user_interface.into_cache();

        for message in messages.drain(..) {
            app.update(&mut state, message);
        }

        // Render to framebuffer
        renderer.draw::<&str>(
            &mut pixels,
            &mut mask,
            &viewport,
            &[Rectangle::new(Point::new(0.0, 0.0), window_size)],
            style.background_color(),
            &[],
        );

        // Flush rendering operations...
        platform.draw(cast_slice(pixels.as_ref().data()))?;
    }
}

async fn poll_until(platform: &mut impl Platform, interval: &mut Interval) -> Vec<Event> {
    let mut events = Vec::new();
    // loop {
    trace!("poll");
    tokio::select! {
        event = platform.poll() => {
            trace!("key");
            if let Some(event) = event.into() {
                events.push(event);
            }
        }
        _ = interval.tick() => {
            trace!("tick");
            return events;
        }
    }
    events
    // }
}

trait Focus {
    fn focus() -> Self {}
    fn unfocus() -> Self {}
}
