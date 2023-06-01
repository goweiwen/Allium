use std::ffi::OsStr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use anyhow::Result;
use embedded_font::{FontTextStyle, FontTextStyleBuilder};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::text::{Baseline, TextStyleBuilder};
use embedded_graphics::{
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};
use rusttype::Font;
use tokio::process;
use tracing::debug;

use crate::cores::CoreMapper;
use crate::launcher::Launcher;
use crate::platform::{Key, KeyEvent, Platform};

const BUTTON_DIAMETER: u32 = 34;
const SELECTION_HEIGHT: u32 = 34;

pub struct Allium {
    platform: Platform,
    launcher: Launcher,
    core_mapper: CoreMapper,
    styles: Stylesheet,
    selected: i32,
    core_handle: Option<process::Child>,
    dirty: bool,
}

pub struct Stylesheet {
    fg_color: Rgb888,
    bg_color: Rgb888,
    primary: Rgb888,
    button_a_color: Rgb888,
    button_b_color: Rgb888,
    button_x_color: Rgb888,
    button_y_color: Rgb888,
    ui_font: Font<'static>,
}

impl Default for Stylesheet {
    fn default() -> Self {
        Self {
            fg_color: Rgb888::new(255, 255, 255),
            bg_color: Rgb888::new(0, 0, 0),
            primary: Rgb888::new(151, 135, 187),
            button_a_color: Rgb888::new(235, 26, 29),
            button_b_color: Rgb888::new(254, 206, 21),
            button_x_color: Rgb888::new(7, 73, 180),
            button_y_color: Rgb888::new(0, 141, 69),
            ui_font: Font::try_from_bytes(include_bytes!("../assets/font/Lato/Lato-Bold.ttf"))
                .unwrap(),
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
            selected: 0,
            core_handle: None,
            dirty: true,
        })
    }

    pub async fn init(&mut self) -> Result<()> {
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

            if self.core_handle.is_none() && self.dirty {
                self.draw()?;
                self.dirty = false;
            }

            match self.platform.poll().await? {
                Some(KeyEvent::Pressed(key)) => {
                    match key {
                        Key::Up => {
                            self.selected = (self.selected - 1).clamp(0, 10);
                            self.dirty = true;
                        }
                        Key::Down => {
                            self.selected = (self.selected + 1).clamp(0, 10);
                            self.dirty = true;
                        }
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
                                self.platform.display().clear(self.styles.bg_color)?;
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
                Some(KeyEvent::Released(_)) => (),
                None => (),
            }
        }
    }

    fn draw(&mut self) -> Result<()> {
        let _yoffset = 14;

        let battery_percentage = self.platform.battery_percentage();

        let (width, height) = self.platform.display_size();
        let display = self.platform.display();

        let text_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(24)
            .text_color(self.styles.fg_color)
            .build();

        let primary_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(24)
            .text_color(self.styles.primary)
            .build();

        let selection_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(24)
            .text_color(self.styles.fg_color)
            .background_color(self.styles.primary)
            .build();

        // Draw battery percentage
        Text::with_alignment(
            &format!("{}%", battery_percentage),
            Point { x: width - 8, y: 8 },
            text_style.clone(),
            Alignment::Right,
        )
        .draw(display)?;

        // Draw header navigation
        let mut x = 12;
        for (i, text) in ["Games", "Recents", "Settings"].iter().enumerate() {
            let text = Text::with_alignment(
                text,
                Point { x, y: 8 },
                if i == 0 {
                    primary_style.clone()
                } else {
                    text_style.clone()
                },
                Alignment::Left,
            );
            x += text.bounding_box().size.width as i32 + 12;
            text.draw(display)?;
        }

        // Draw game list
        let roms = self
            .launcher
            .roms()?
            .flat_map(|r| r.ok())
            .collect::<Vec<_>>();

        let (x, mut y) = (24, 58);
        for (i, rom) in roms.iter().enumerate() {
            if let Some(text) = rom.file_name().and_then(OsStr::to_str) {
                if self.selected == i as i32 {
                    let text = Text::with_alignment(
                        text,
                        Point { x, y },
                        selection_style.clone(),
                        Alignment::Left,
                    );
                    let text_width = text.bounding_box().size.width;
                    let fill_style = PrimitiveStyle::with_fill(self.styles.primary);
                    Circle::new(Point::new(x - 12, y - 4), SELECTION_HEIGHT)
                        .into_styled(fill_style)
                        .draw(display)?;
                    Circle::new(
                        Point::new(x + text_width as i32 - SELECTION_HEIGHT as i32 + 12, y - 4),
                        SELECTION_HEIGHT,
                    )
                    .into_styled(fill_style)
                    .draw(display)?;
                    Rectangle::new(
                        Point::new(x - 12 + SELECTION_HEIGHT as i32 / 2, y - 4),
                        Size::new(text_width - 24 + SELECTION_HEIGHT / 2, SELECTION_HEIGHT),
                    )
                    .into_styled(fill_style)
                    .draw(display)?;
                    text.draw(display)?;
                } else {
                    // Clear previous selection
                    let fill_style = PrimitiveStyle::with_fill(self.styles.bg_color);
                    Rectangle::new(Point::new(x - 12, y - 4), Size::new(336, SELECTION_HEIGHT))
                        .into_styled(fill_style)
                        .draw(display)?;
                    let text = Text::with_alignment(
                        text,
                        Point { x, y },
                        text_style.clone(),
                        Alignment::Left,
                    );
                    text.draw(display)?;
                }
                y += 42;
            }
        }

        // Draw button hints
        let y = height - BUTTON_DIAMETER as i32 - 8;
        let mut x = width as i32 - 12;

        x = self
            .draw_button_hint(Point::new(x, y), Key::A, text_style.clone(), "Start")?
            .top_left
            .x
            - 18;
        self.draw_button_hint(Point::new(x, y), Key::B, text_style.clone(), "Back")?;

        self.platform.flush()?;

        Ok(())
    }

    fn draw_text(
        &mut self,
        point: Point,
        text: &str,
        style: FontTextStyle<Rgb888>,
        alignment: Alignment,
    ) -> Result<Rectangle> {
        let text = Text::with_alignment(text, point, style, alignment);
        text.draw(self.platform.display())?;
        Ok(text.bounding_box())
    }

    fn draw_button_hint(
        &mut self,
        point: Point,
        button: Key,
        style: FontTextStyle<Rgb888>,
        text: &str,
    ) -> Result<Rectangle> {
        let x = point.x
            - self
                .draw_text(
                    Point::new(point.x, point.y + 4),
                    text,
                    style,
                    Alignment::Right,
                )?
                .size
                .width as i32
            - 4;
        self.draw_button(Point::new(x - BUTTON_DIAMETER as i32, point.y), button)?;
        Ok(Rectangle::new(
            Point::new(x - BUTTON_DIAMETER as i32, point.y),
            Size::new(
                (point.x - (x - BUTTON_DIAMETER as i32)) as u32,
                BUTTON_DIAMETER,
            ),
        ))
    }

    fn draw_button(&mut self, point: Point, button: Key) -> Result<()> {
        let display = self.platform.display();
        let (color, text) = match button {
            Key::A => (self.styles.button_a_color, "A"),
            Key::B => (self.styles.button_b_color, "B"),
            Key::X => (self.styles.button_x_color, "X"),
            Key::Y => (self.styles.button_y_color, "Y"),
            _ => (self.styles.primary, "?"),
        };

        Circle::new(point, BUTTON_DIAMETER)
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(display)?;

        let button_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(24)
            .text_color(self.styles.fg_color)
            .background_color(color)
            .build();
        Text::with_text_style(
            text,
            Point::new(point.x + (BUTTON_DIAMETER / 2) as i32, point.y + 4),
            button_style.clone(),
            TextStyleBuilder::new()
                .alignment(Alignment::Center)
                .baseline(Baseline::Middle)
                .build(),
        )
        .draw(display)?;

        Ok(())
    }
}
