use std::cell::RefCell;
use std::process;
use std::rc::Rc;
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::{Result, bail};
use async_trait::async_trait;
use log::{trace, warn};
use softbuffer::Surface;
use tiny_skia::{Pixmap, PixmapMut, PixmapRef};
use tokio::sync::mpsc;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent as WinitKeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::window::{Window as WinitWindow, WindowId};

use crate::battery::Battery;
use crate::display::Display;
use crate::display::color::Color;
use crate::display::settings::DisplaySettings;
use crate::geom::Rect;
use crate::platform::{Key, KeyEvent, Platform};

static SCREEN_WIDTH: LazyLock<u32> = LazyLock::new(|| {
    std::env::var("WIDTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(752)
});

static SCREEN_HEIGHT: LazyLock<u32> = LazyLock::new(|| {
    std::env::var("HEIGHT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(560)
});

static SCREEN_SCALE: LazyLock<u32> = LazyLock::new(|| {
    std::env::var("SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
});

pub struct SimulatorPlatform {
    event_loop: EventLoop<()>,
    window: Option<Rc<WinitWindow>>,
    key_event_rx: mpsc::UnboundedReceiver<KeyEvent>,
    key_event_tx: mpsc::UnboundedSender<KeyEvent>,
}

struct SimulatorApp {
    window: Option<Rc<WinitWindow>>,
    surface: Option<Surface<Rc<WinitWindow>, Rc<WinitWindow>>>,
    key_event_tx: mpsc::UnboundedSender<KeyEvent>,
    got_event: bool,
}

impl ApplicationHandler for SimulatorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            trace!("Creating window in resumed()");
            let window_attrs = WinitWindow::default_attributes()
                .with_title("Allium Simulator")
                .with_inner_size(PhysicalSize::new(
                    *SCREEN_WIDTH * *SCREEN_SCALE,
                    *SCREEN_HEIGHT * *SCREEN_SCALE,
                ))
                .with_resizable(false);

            let window = event_loop
                .create_window(window_attrs)
                .expect("Failed to create window");
            let window = Rc::new(window);

            let context = softbuffer::Context::new(window.clone())
                .expect("Failed to create softbuffer context");
            let surface = softbuffer::Surface::new(&context, window.clone())
                .expect("Failed to create softbuffer surface");

            self.window = Some(window);
            self.surface = Some(surface);
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Exit if we received an event
        if self.got_event {
            self.got_event = false;
            event_loop.exit();
        } else {
            // Wait for up to 16ms for events, then exit to yield to tokio
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                std::time::Instant::now() + Duration::from_millis(16),
            ));
        }
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        // Exit on timeout to yield to tokio for View::update
        if matches!(cause, winit::event::StartCause::ResumeTimeReached { .. }) {
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    WinitKeyEvent {
                        physical_key: PhysicalKey::Code(keycode),
                        state,
                        repeat,
                        ..
                    },
                ..
            } => {
                trace!("KeyboardInput: {:?} {:?}", keycode, state);
                if keycode == KeyCode::KeyQ && state == ElementState::Pressed {
                    event_loop.exit();
                    return;
                }

                let key = Key::from(keycode);
                let key_event = match (state, repeat) {
                    (ElementState::Pressed, true) => KeyEvent::Autorepeat(key),
                    (ElementState::Pressed, false) => KeyEvent::Pressed(key),
                    (ElementState::Released, _) => KeyEvent::Released(key),
                };

                trace!("Sending key event: {:?}", key_event);
                let _ = self.key_event_tx.send(key_event);
                self.got_event = true;
            }
            WindowEvent::RedrawRequested => {
                // We manage our own display updates, no action needed
            }
            _ => {}
        }
    }
}

#[async_trait(?Send)]
impl Platform for SimulatorPlatform {
    type Display = SimulatorWindow;
    type Battery = SimulatorBattery;
    type SuspendContext = ();

    fn new() -> Result<SimulatorPlatform> {
        let event_loop = EventLoop::new()?;

        let (tx, rx) = mpsc::unbounded_channel();

        Ok(SimulatorPlatform {
            event_loop,
            window: None,
            key_event_rx: rx,
            key_event_tx: tx,
        })
    }

    async fn poll(&mut self) -> KeyEvent {
        loop {
            // Try to receive from channel (non-blocking)
            if let Ok(event) = self.key_event_rx.try_recv() {
                trace!("Returning queued event: {:?}", event);
                return event;
            }

            // Process window events - will wait for keyboard event then return
            let window_ref = self.window.clone();
            let tx = self.key_event_tx.clone();

            let mut app = SimulatorApp {
                window: window_ref,
                surface: None,
                key_event_tx: tx,
                got_event: false,
            };

            // run_app_on_demand waits for events; about_to_wait exits when we get one
            self.event_loop.run_app_on_demand(&mut app).ok();

            // Check again after processing events
            if let Ok(event) = self.key_event_rx.try_recv() {
                trace!("Returning event after processing: {:?}", event);
                return event;
            }

            // Yield to tokio briefly
            tokio::task::yield_now().await;
        }
    }

    fn display(&mut self) -> Result<SimulatorWindow> {
        trace!("display() called");
        // Initialize window if not already done
        if self.window.is_none() {
            trace!("Window is None, initializing...");
            let tx = self.key_event_tx.clone();
            let mut app = SimulatorApp {
                window: None,
                surface: None,
                key_event_tx: tx,
                got_event: false,
            };
            trace!("Running event loop to create window");
            self.event_loop.run_app_on_demand(&mut app).ok();
            // Capture the window created by the event loop
            self.window = app.window.clone();
            trace!("Window after event loop: {:?}", self.window.is_some());
        }

        // Load background image if available
        let bg_path = format!("simulator/bg-{}x{}.png", *SCREEN_WIDTH, *SCREEN_HEIGHT);
        let mut pixmap = if let Ok(img) = image::open(&bg_path) {
            let img = img.to_rgba8();
            let mut pixmap =
                Pixmap::new(*SCREEN_WIDTH, *SCREEN_HEIGHT).expect("Failed to create pixmap");

            for (x, y, pixel) in img.enumerate_pixels() {
                if x < *SCREEN_WIDTH && y < *SCREEN_HEIGHT {
                    let idx = (y * *SCREEN_WIDTH + x) as usize;
                    let color = Color::rgba(pixel[0], pixel[1], pixel[2], pixel[3]);
                    pixmap.pixels_mut()[idx] = color.into();
                }
            }
            pixmap
        } else {
            warn!(
                "Failed to load background image '{}', using black background",
                bg_path
            );
            Pixmap::new(*SCREEN_WIDTH, *SCREEN_HEIGHT).expect("Failed to create pixmap")
        };

        // Fill with black if no background
        if pixmap.pixels().iter().all(|p| {
            let c: Color = (*p).into();
            c.r() == 0 && c.g() == 0 && c.b() == 0 && c.a() == 0
        }) {
            pixmap.fill(tiny_skia::Color::BLACK);
        }

        let window = self.window.clone().expect("Window not initialized");
        let context =
            softbuffer::Context::new(window.clone()).expect("Failed to create softbuffer context");
        let surface = softbuffer::Surface::new(&context, window.clone())
            .expect("Failed to create softbuffer surface");

        Ok(SimulatorWindow {
            window,
            surface: Rc::new(RefCell::new(surface)),
            pixmap,
            saved: Vec::new(),
        })
    }

    fn battery(&self) -> Result<SimulatorBattery> {
        Ok(SimulatorBattery::new())
    }

    fn shutdown(&self) -> Result<()> {
        process::exit(0);
    }

    fn suspend(&self) -> Result<Self::SuspendContext> {
        Ok(())
    }

    fn unsuspend(&self, _ctx: Self::SuspendContext) -> Result<()> {
        Ok(())
    }

    fn set_volume(&mut self, _volume: i32) -> Result<()> {
        Ok(())
    }

    fn get_brightness(&self) -> Result<u8> {
        Ok(50)
    }

    fn set_brightness(&mut self, _brightness: u8) -> Result<()> {
        Ok(())
    }

    fn set_display_settings(&mut self, _settings: &mut DisplaySettings) -> Result<()> {
        Ok(())
    }

    fn device_model() -> String {
        "Simulator".into()
    }

    fn firmware() -> String {
        "00000000".to_string()
    }

    fn has_wifi() -> bool {
        true
    }

    fn has_lid() -> bool {
        true
    }
}

impl Default for SimulatorPlatform {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

pub struct SimulatorWindow {
    window: Rc<WinitWindow>,
    surface: Rc<RefCell<Surface<Rc<WinitWindow>, Rc<WinitWindow>>>>,
    pixmap: Pixmap,
    saved: Vec<Vec<u8>>,
}

impl Display for SimulatorWindow {
    fn width(&self) -> u32 {
        *SCREEN_WIDTH
    }

    fn height(&self) -> u32 {
        *SCREEN_HEIGHT
    }

    fn pixmap(&self) -> PixmapRef<'_> {
        self.pixmap.as_ref()
    }

    fn pixmap_mut(&mut self) -> PixmapMut<'_> {
        self.pixmap.as_mut()
    }

    fn map_pixels<F>(&mut self, mut f: F) -> Result<()>
    where
        F: FnMut(Color) -> Color,
    {
        for pixel in self.pixmap.pixels_mut() {
            let color: Color = (*pixel).into();
            *pixel = f(color).into();
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        let width = *SCREEN_WIDTH as usize;
        let height = *SCREEN_HEIGHT as usize;
        let scale = *SCREEN_SCALE as usize;

        let mut surface = self.surface.borrow_mut();
        surface
            .resize(
                std::num::NonZeroU32::new((width * scale) as u32).unwrap(),
                std::num::NonZeroU32::new((height * scale) as u32).unwrap(),
            )
            .expect("Failed to resize surface");

        let mut buffer = surface.buffer_mut().expect("Failed to get buffer");

        // Convert pixmap to softbuffer format (u32 RGB)
        // Apply scaling if needed
        for y in 0..height * scale {
            for x in 0..width * scale {
                let src_x = x / scale;
                let src_y = y / scale;
                let src_idx = src_y * width + src_x;

                let pixel = self.pixmap.pixels()[src_idx];
                // Use premultiplied values directly for RGB (tiny-skia uses premultiplied alpha)
                let rgb = (pixel.red() as u32) << 16
                    | (pixel.green() as u32) << 8
                    | (pixel.blue() as u32);

                let dst_idx = y * width * scale + x;
                buffer[dst_idx] = rgb;
            }
        }

        buffer.present().expect("Failed to present buffer");
        self.window.request_redraw();

        Ok(())
    }

    fn save(&mut self) -> Result<()> {
        // Save pixmap as raw RGBA bytes
        let mut buffer = Vec::with_capacity(self.pixmap.pixels().len() * 4);
        for pixel in self.pixmap.pixels() {
            let color: Color = (*pixel).into();
            buffer.push(color.r());
            buffer.push(color.g());
            buffer.push(color.b());
            buffer.push(color.a());
        }
        self.saved.push(buffer);
        Ok(())
    }

    fn load(&mut self, mut rect: Rect) -> Result<()> {
        let Some(saved) = &self.saved.last() else {
            bail!("No saved image");
        };

        let size = self.size();
        if rect.x < 0
            || rect.y < 0
            || rect.x as u32 + rect.w > size.w
            || rect.y as u32 + rect.h > size.h
        {
            warn!(
                "Area exceeds display bounds: x: {}, y: {}, w: {}, h: {}",
                rect.x, rect.y, rect.w, rect.h,
            );
            rect.x = rect.x.max(0);
            rect.y = rect.y.max(0);
            rect.w = rect.w.min(size.w - rect.x as u32);
            rect.h = rect.h.min(size.h - rect.y as u32);
        }

        // Copy saved region back to pixmap
        let width = *SCREEN_WIDTH as usize;
        for dy in 0..rect.h {
            for dx in 0..rect.w {
                let x = (rect.x + dx as i32) as usize;
                let y = (rect.y + dy as i32) as usize;
                let idx = y * width + x;
                let src_idx = idx * 4;

                if src_idx + 3 < saved.len() {
                    let color = Color::rgba(
                        saved[src_idx],
                        saved[src_idx + 1],
                        saved[src_idx + 2],
                        saved[src_idx + 3],
                    );
                    self.pixmap.pixels_mut()[idx] = color.into();
                }
            }
        }

        Ok(())
    }

    fn pop(&mut self) -> bool {
        self.saved.pop();
        !self.saved.is_empty()
    }
}

impl From<KeyCode> for Key {
    fn from(value: KeyCode) -> Self {
        match value {
            KeyCode::ArrowUp => Key::Up,
            KeyCode::ArrowDown => Key::Down,
            KeyCode::ArrowLeft => Key::Left,
            KeyCode::ArrowRight => Key::Right,
            KeyCode::Space => Key::A,
            KeyCode::ControlLeft => Key::B,
            KeyCode::ShiftLeft => Key::X,
            KeyCode::AltLeft => Key::Y,
            KeyCode::Enter => Key::Start,
            KeyCode::ControlRight => Key::Select,
            KeyCode::KeyE => Key::L,
            KeyCode::KeyT => Key::R,
            KeyCode::Escape => Key::Menu,
            KeyCode::Tab => Key::L2,
            KeyCode::Backspace => Key::R2,
            KeyCode::Power => Key::Power,
            KeyCode::SuperLeft => Key::VolDown,
            KeyCode::SuperRight => Key::VolUp,
            _ => Key::Unknown,
        }
    }
}

pub struct SimulatorBattery {
    percentage: i32,
    charging: bool,
}

impl SimulatorBattery {
    pub fn new() -> SimulatorBattery {
        SimulatorBattery {
            percentage: 100,
            charging: std::env::var("SIMULATOR_CHARGING").unwrap_or_else(|_| "0".to_string())
                == "1",
        }
    }
}

impl Default for SimulatorBattery {
    fn default() -> Self {
        Self::new()
    }
}

impl Battery for SimulatorBattery {
    fn update(&mut self) -> Result<()> {
        trace!("Updating battery");
        if self.percentage > 0 {
            self.percentage -= 5
        }
        Ok(())
    }

    fn percentage(&self) -> i32 {
        self.percentage
    }

    fn charging(&self) -> bool {
        self.charging
    }

    fn update_led(_enabled: bool) {
        // Simulator doesn't have LED
    }
}
