use std::ffi::OsStr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use anyhow::Result;
use embedded_font::FontTextStyleBuilder;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::{
    prelude::*,
    primitives::{
        Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, Triangle,
    },
    text::{Alignment, Text},
};
use rusttype::Font;
use tokio::process;
use tracing::debug;

use crate::cores::CoreMapper;
use crate::launcher::Launcher;
use crate::platform::{Key, KeyEvent, Platform};

pub struct Allium {
    platform: Platform,
    launcher: Launcher,
    core_mapper: CoreMapper,
    styles: Stylesheet,
    core_handle: Option<process::Child>,
    dirty: bool,
}

pub struct Stylesheet {
    fg_color: Rgb888,
    bg_color: Rgb888,
    primary: Rgb888,
    ui_font: Font<'static>,
}

impl Default for Stylesheet {
    fn default() -> Self {
        let fg_color = Rgb888::new(255, 255, 255);
        let bg_color = Rgb888::new(0, 0, 0);
        let primary = Rgb888::new(237, 90, 134);
        let ui_font =
            Font::try_from_bytes(include_bytes!("../assets/font/Sniglet/Sniglet-Regular.ttf"))
                .unwrap();

        Self {
            ui_font,
            fg_color,
            bg_color,
            primary,
        }
    }
}

pub const BATTERY_UPDATE_INTERVAL: Duration = Duration::from_secs(5);

impl Allium {
    pub fn new() -> Result<Allium> {
        Ok(Allium {
            platform: Platform::new()?,
            launcher: Launcher::new(),
            core_mapper: CoreMapper::new()?,
            styles: Default::default(),
            core_handle: None,
            dirty: true,
        })
    }

    pub async fn init(&mut self) -> Result<()> {
        Platform::init().await?;
        self.core_mapper.load_config()?;
        Ok(())
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        self.platform.update_battery()?;

        let mut last_updated_battery = std::time::Instant::now();

        loop {
            let now = std::time::Instant::now();

            // Update battery every 5 seconds
            debug!(
                "last updated: {}s",
                now.duration_since(last_updated_battery).as_secs()
            );
            if now.duration_since(last_updated_battery) > BATTERY_UPDATE_INTERVAL {
                self.platform.update_battery()?;
                last_updated_battery = now;
                self.dirty = true;
            }

            if self.core_handle.is_none() {
                if self.dirty {
                    self.draw()?;
                    self.dirty = false;
                }
            }

            match self.platform.poll().await? {
                Some(KeyEvent::Pressed(key)) => {
                    println!("down {:?}", key);
                    match key {
                        Key::A => {
                            if self.core_handle.is_none() {
                                // Testing RetroArch launching
                                let rom = PathBuf::from_str(
                                    "/mnt/SDCARD/Roms/GBC/001 Dragon Warrior I and II.gbc",
                                )
                                .unwrap();
                                let core = self
                                    .core_mapper
                                    .get_core(rom.extension().unwrap().to_str().unwrap())
                                    .unwrap();
                                self.platform.display()?.clear(self.styles.bg_color)?;
                                self.platform.flush()?;
                                self.core_handle = Some(core.launch(&rom).await?);
                            }
                        }
                        Key::Power => {
                            if let Some(mut handle) = self.core_handle.take() {
                                handle.kill().await?;
                            } else {
                                panic!("exiting");
                            }
                        }
                        _ => (),
                    }
                }
                Some(KeyEvent::Released(key)) => println!("up {:?}", key),
                None => (),
            }
        }
    }

    fn draw(&mut self) -> Result<()> {
        let yoffset = 14;

        let battery_percentage = self.platform.battery_percentage();

        let (width, _height) = self.platform.display_size();
        let display = self.platform.display()?;

        // Draw a 3px wide outline around the display.
        display
            .bounding_box()
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_color(self.styles.primary)
                    .stroke_width(3)
                    .stroke_alignment(StrokeAlignment::Inside)
                    .build(),
            )
            .draw(display)?;

        // Draw a triangle.
        Triangle::new(
            Point::new(16, 16 + yoffset),
            Point::new(16 + 16, 16 + yoffset),
            Point::new(16 + 8, yoffset),
        )
        .into_styled(PrimitiveStyle::with_stroke(self.styles.primary, 1))
        .draw(display)?;

        // Draw a filled square
        Rectangle::new(Point::new(52, yoffset), Size::new(16, 16))
            .into_styled(PrimitiveStyle::with_fill(self.styles.primary))
            .draw(display)?;

        // Draw a circle with a 3px wide stroke.
        Circle::new(Point::new(88, yoffset), 17)
            .into_styled(PrimitiveStyle::with_stroke(self.styles.primary, 3))
            .draw(display)?;

        // Draw battery percentage
        let text = format!("{}%", battery_percentage);

        let character_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(24)
            .text_color(self.styles.fg_color)
            .build();

        Text::with_alignment(
            &text,
            Point { x: width - 8, y: 8 },
            character_style,
            Alignment::Right,
        )
        .draw(display)?;

        // Draw centered text.
        let text = "hello world, from Allium!";

        let character_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(24)
            .text_color(self.styles.fg_color)
            .build();

        Text::with_alignment(
            text,
            Point {
                x: width / 2,
                y: 16,
            },
            character_style.clone(),
            Alignment::Center,
        )
        .draw(display)?;

        // Draw roms
        let roms = self
            .launcher
            .roms()?
            .flat_map(|r| r.ok())
            .collect::<Vec<_>>();

        let (x, y) = (16, 64);
        let mut i = 0;
        for rom in roms {
            if let Some(text) = rom.file_name().and_then(OsStr::to_str) {
                Text::with_alignment(
                    text,
                    Point { x, y: i * 24 + y },
                    character_style.clone(),
                    Alignment::Left,
                )
                .draw(display)?;
                i += 1;
            }
        }

        self.platform.flush()?;

        Ok(())
    }
}
