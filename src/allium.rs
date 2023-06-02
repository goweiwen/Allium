use std::cmp::min;
use std::time::Duration;

use anyhow::Result;
use embedded_font::FontTextStyleBuilder;
use embedded_graphics::image::{Image, ImageRaw};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::{
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};
use rusttype::Font;
use tokio::process;
use tracing::{debug, trace};

use crate::constants::{
    BUTTON_DIAMETER, IMAGE_SIZE, LISTING_JUMP_SIZE, LISTING_SIZE, SELECTION_HEIGHT,
};
use crate::cores::CoreMapper;
use crate::launcher::{Directory, Entry, Game, Launcher};
use crate::platform::{Display, Key, KeyEvent, Platform};

pub struct Allium {
    platform: Platform,
    launcher: Launcher,
    core_mapper: CoreMapper,
    styles: Stylesheet,
    entries: Vec<Entry>,
    stack: Vec<Directory>,
    top: i32,
    selected: i32,
    core_handle: Option<process::Child>,
    dirty: bool,
}

pub struct Stylesheet {
    pub fg_color: Rgb888,
    pub bg_color: Rgb888,
    pub primary: Rgb888,
    pub button_a_color: Rgb888,
    pub button_b_color: Rgb888,
    pub button_x_color: Rgb888,
    pub button_y_color: Rgb888,
    pub ui_font: Font<'static>,
    pub ui_font_size: u32,
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
            ui_font_size: 24,
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
            entries: vec![],
            stack: vec![],
            top: 0,
            selected: 0,
            core_handle: None,
            dirty: true,
        })
    }

    fn directory(&self) -> Option<&Directory> {
        self.stack.last()
    }

    pub async fn init(&mut self) -> Result<()> {
        self.core_mapper.load_config()?;
        self.entries = self.launcher.entries(self.directory())?.collect();
        Ok(())
    }

    fn push_directory(&mut self, directory: Directory) -> Result<()> {
        self.stack.push(directory);
        self.entries = self.launcher.entries(self.directory())?.collect();
        self.top = 0;
        self.selected = 0;
        self.dirty = true;
        Ok(())
    }

    fn pop_directory(&mut self) -> Result<()> {
        self.stack.pop();
        self.entries = self.launcher.entries(self.directory())?.collect();
        self.top = 0;
        self.selected = 0;
        self.dirty = true;
        Ok(())
    }

    async fn launch_game(&mut self, game: &Game) -> Result<()> {
        self.platform.display().clear(self.styles.bg_color)?;
        self.platform.flush()?;
        let core = self.core_mapper.get_core(&game.extension);
        if let Some(core) = core {
            core.launch(&game.path).await?;
        }
        Ok(())
    }

    async fn select_entry(&mut self, entry: Entry) -> Result<()> {
        match entry {
            Entry::Directory(directory) => self.push_directory(directory)?,
            Entry::Game(game) => self.launch_game(&game).await?,
        }
        Ok(())
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        self.platform.update_battery()?;

        let mut last_updated_battery = std::time::Instant::now();

        loop {
            let now = std::time::Instant::now();

            // Update battery every 5 seconds
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
                Some(KeyEvent::Pressed(key)) => match key {
                    Key::Up => {
                        self.selected = (self.selected - 1).clamp(0, self.entries.len() as i32);
                        if self.selected < self.top {
                            self.top = self.selected
                        }
                        self.dirty = true;
                        trace!("selected: {}, top: {}", self.selected, self.top);
                    }
                    Key::Down => {
                        self.selected = (self.selected + 1).clamp(0, self.entries.len() as i32);
                        if self.selected - LISTING_SIZE >= self.top {
                            self.top = self.selected - LISTING_SIZE + 1;
                        }
                        self.dirty = true;
                        trace!("selected: {}, top: {}", self.selected, self.top);
                    }
                    Key::Left => {
                        self.selected =
                            (self.selected - LISTING_JUMP_SIZE).clamp(0, self.entries.len() as i32);
                        if self.selected < self.top {
                            self.top = self.selected
                        }
                        self.dirty = true;
                    }
                    Key::Right => {
                        self.selected =
                            (self.selected + LISTING_JUMP_SIZE).clamp(0, self.entries.len() as i32);
                        if self.selected - LISTING_SIZE >= self.top {
                            self.top = self.selected - LISTING_SIZE + 1;
                        }
                        self.dirty = true;
                    }
                    Key::A => {
                        if self.core_handle.is_none() {
                            let entry = self.entries.get(self.selected as usize);
                            if let Some(entry) = entry {
                                self.select_entry(entry.to_owned()).await?;
                            }
                        }
                    }
                    Key::B => self.pop_directory()?,
                    Key::Power => {
                        if let Some(mut handle) = self.core_handle.take() {
                            if let Some(id) = handle.id() {
                                #[cfg(target_os = "linux")]
                                unsafe {
                                    libc::kill(id as i32, libc::SIGTERM);
                                }
                            }
                            handle.wait().await?;
                        } else {
                            panic!("exiting");
                        }
                    }
                    _ => (),
                },
                Some(KeyEvent::Released(_)) => (),
                None => (),
            }
        }
    }

    fn draw(&mut self) -> Result<()> {
        let _yoffset = 14;

        let battery_percentage = self.platform.battery_percentage();
        let battery_is_charging = self.platform.battery_is_charging();

        let (width, height) = self.platform.display_size();
        let display = self.platform.display();

        let text_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(self.styles.ui_font_size)
            .text_color(self.styles.fg_color)
            .build();

        let primary_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(self.styles.ui_font_size)
            .text_color(self.styles.primary)
            .build();

        let selection_style = FontTextStyleBuilder::new(self.styles.ui_font.clone())
            .font_size(self.styles.ui_font_size)
            .text_color(self.styles.fg_color)
            .background_color(self.styles.primary)
            .build();

        // Draw battery percentage
        if battery_is_charging {
            Text::with_alignment(
                &format!("Charging: {}%", battery_percentage),
                Point { x: width - 8, y: 8 },
                text_style.clone(),
                Alignment::Right,
            )
            .draw(display)?;
        } else {
            Text::with_alignment(
                &format!("{}%", battery_percentage),
                Point { x: width - 8, y: 8 },
                text_style.clone(),
                Alignment::Right,
            )
            .draw(display)?;
        }

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

        let (x, mut y) = (24, 58);
        for i in (self.top as usize)
            ..min(
                self.entries.len(),
                self.top as usize + LISTING_SIZE as usize,
            )
        {
            let entry = &self.entries[i];

            // Clear previous selection
            let fill_style = PrimitiveStyle::with_fill(self.styles.bg_color);
            Rectangle::new(Point::new(x - 12, y - 4), Size::new(336, SELECTION_HEIGHT))
                .into_styled(fill_style.clone())
                .draw(display)?;

            if self.selected == i as i32 {
                if let Entry::Game(ref game) = entry {
                    if let Some(image) = &game.image {
                        let mut image = image::open(image)?;
                        if image.width() != IMAGE_SIZE.width || image.height() != IMAGE_SIZE.height
                        {
                            image = image.resize_to_fill(
                                IMAGE_SIZE.width,
                                IMAGE_SIZE.height,
                                image::imageops::FilterType::Triangle,
                            );
                        }
                        let mut image = image.to_rgb8();
                        crate::image::round(&mut image, image::Rgb([0u8; 3]), 12);
                        let image: ImageRaw<Rgb888> = ImageRaw::new(&image, IMAGE_SIZE.width);
                        let image = Image::new(
                            &image,
                            Point::new(width - IMAGE_SIZE.width as i32 - 24, 54),
                        );
                        image.draw(display)?;
                    } else {
                        Rectangle::new(
                            Point::new(width - IMAGE_SIZE.width as i32 - 24, 54),
                            IMAGE_SIZE.clone(),
                        )
                        .into_styled(fill_style)
                        .draw(display)?;
                    }
                }

                let text_width = display
                    .draw_text_ellipsis(
                        Point { x, y },
                        entry.name(),
                        selection_style.clone(),
                        Alignment::Left,
                        300,
                    )?
                    .size
                    .width;
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
                display
                    .draw_text_ellipsis(
                        Point { x, y },
                        entry.name(),
                        selection_style.clone(),
                        Alignment::Left,
                        300,
                    )?
                    .size
                    .width;
            } else {
                display.draw_text_ellipsis(
                    Point { x, y },
                    entry.name(),
                    text_style.clone(),
                    Alignment::Left,
                    300,
                )?;
            }
            y += 42;
        }

        // Draw button hints
        let y = height - BUTTON_DIAMETER as i32 - 8;
        let mut x = width as i32 - 12;

        x = display
            .draw_button_hint(
                Point::new(x, y),
                Key::A,
                text_style.clone(),
                "Start",
                &self.styles,
            )?
            .top_left
            .x
            - 18;
        display.draw_button_hint(
            Point::new(x, y),
            Key::B,
            text_style.clone(),
            "Back",
            &self.styles,
        )?;

        self.platform.flush()?;

        Ok(())
    }
}
