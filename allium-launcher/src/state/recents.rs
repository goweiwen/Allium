use std::{cmp::min, rc::Rc};

use anyhow::Result;
use embedded_graphics::{
    image::{Image, ImageRaw},
    prelude::*,
    primitives::Rectangle,
    text::Alignment,
};
use serde::{Deserialize, Serialize};

use common::{
    constants::{
        BUTTON_DIAMETER, IMAGE_SIZE, LISTING_JUMP_SIZE, LISTING_SIZE, RECENT_GAMES_LIMIT,
        SELECTION_HEIGHT, SELECTION_MARGIN,
    },
    display::{color::Color, Display},
    platform::Key,
    stylesheet::Stylesheet,
};
use common::{
    database::Database,
    platform::{DefaultPlatform, KeyEvent, Platform},
};
use tracing::trace;

use crate::{
    command::AlliumCommand,
    cores::{CoreMapper, Game},
    state::State,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentsState {
    top: i32,
    selected: i32,
    entries: Vec<Game>,
    #[serde(skip)]
    database: Database,
    #[serde(skip)]
    core_mapper: Option<Rc<CoreMapper>>,
    sort: Sort,
}

impl RecentsState {
    pub fn new() -> Self {
        Self {
            top: 0,
            selected: 0,
            entries: vec![],
            database: Default::default(),
            core_mapper: None,
            sort: Sort::LastPlayed,
        }
    }

    pub fn init(&mut self, core_mapper: Rc<CoreMapper>, database: Database) {
        self.database = database;
        self.core_mapper = Some(core_mapper);
    }

    fn select_entry(&mut self, selected: i32) -> Result<Option<AlliumCommand>> {
        let game = &mut self.entries[selected as usize];
        self.core_mapper
            .as_ref()
            .unwrap()
            .launch_game(&self.database, game)
    }

    fn load_entries(&mut self) -> Result<()> {
        let games = match self.sort {
            Sort::LastPlayed => self.database.select_last_played(RECENT_GAMES_LIMIT)?,
            Sort::MostPlayed => self.database.select_most_played(RECENT_GAMES_LIMIT)?,
        };

        self.entries = games
            .into_iter()
            .map(|game| Game::new(game.name, game.path))
            .collect();

        Ok(())
    }
}

impl State for RecentsState {
    fn enter(&mut self) -> Result<()> {
        self.load_entries()?;
        Ok(())
    }

    fn leave(&mut self) -> Result<()> {
        Ok(())
    }

    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<()> {
        let Size { width, height } = display.size();

        // Draw game list
        let (x, mut y) = (24, 58);

        // Clear previous selection
        display.load(Rectangle::new(
            Point::new(x - 12, y - 4),
            Size::new(
                if styles.enable_box_art {
                    324 + 12 * 2
                } else {
                    640 - 12 * 2
                },
                LISTING_SIZE as u32 * (SELECTION_HEIGHT + SELECTION_MARGIN),
            ),
        ))?;

        for i in (self.top as usize)
            ..std::cmp::min(
                self.entries.len(),
                self.top as usize + LISTING_SIZE as usize,
            )
        {
            let entry = &mut self.entries[i];

            if self.selected == i as i32 {
                if styles.enable_box_art {
                    if let Some(image) = entry.image().as_deref() {
                        let mut image = image::open(image)?;
                        if image.width() != IMAGE_SIZE.width || image.height() > IMAGE_SIZE.height {
                            let new_height = min(
                                IMAGE_SIZE.height,
                                IMAGE_SIZE.width * image.height() / image.width(),
                            );
                            image = image.resize_to_fill(
                                IMAGE_SIZE.width,
                                new_height,
                                image::imageops::FilterType::Triangle,
                            );
                        }
                        display.load(Rectangle::new(
                            Point::new(
                                width as i32 - IMAGE_SIZE.width as i32 - 24,
                                54 + image.height() as i32,
                            ),
                            Size::new(IMAGE_SIZE.width, IMAGE_SIZE.height - image.height()),
                        ))?;

                        let mut image = image.to_rgb8();
                        common::display::image::round(
                            &mut image,
                            styles.background_color.into(),
                            12,
                        );
                        let image: ImageRaw<Color> = ImageRaw::new(&image, IMAGE_SIZE.width);
                        let image = Image::new(
                            &image,
                            Point::new(width as i32 - IMAGE_SIZE.width as i32 - 24, 54),
                        );
                        image.draw(display)?;
                    } else {
                        display.load(Rectangle::new(
                            Point::new(width as i32 - IMAGE_SIZE.width as i32 - 24, 54),
                            IMAGE_SIZE,
                        ))?;
                    }
                }

                display.draw_entry(
                    Point { x, y },
                    &entry.name,
                    styles,
                    Alignment::Left,
                    if styles.enable_box_art { 324 } else { 592 },
                    true,
                    true,
                    0,
                )?;
            } else {
                display.draw_entry(
                    Point { x, y },
                    &entry.name,
                    styles,
                    Alignment::Left,
                    if styles.enable_box_art { 324 } else { 592 },
                    false,
                    true,
                    0,
                )?;
            }
            y += (SELECTION_HEIGHT + SELECTION_MARGIN) as i32;
        }

        // Draw button hints
        let y = height as i32 - BUTTON_DIAMETER as i32 - 8;
        let x = width as i32 - 12;

        display.load(Rectangle::new(
            Point::new(360, y),
            Size::new(width - 360, BUTTON_DIAMETER),
        ))?;

        let x = display
            .draw_button_hint(Point::new(x, y), Key::A, "Start", styles, Alignment::Right)?
            .top_left
            .x
            - 18;
        display.draw_button_hint(
            Point::new(x, y),
            Key::Y,
            self.sort.button_hint(),
            styles,
            Alignment::Right,
        )?;

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<(Option<AlliumCommand>, bool)> {
        Ok(match key_event {
            KeyEvent::Pressed(Key::A) => {
                let entry = self.entries.get(self.selected as usize);
                if entry.is_some() {
                    (self.select_entry(self.selected)?, true)
                } else {
                    (None, false)
                }
            }
            KeyEvent::Pressed(Key::Y) => {
                self.sort = self.sort.next();
                self.load_entries()?;
                self.top = 0;
                self.selected = 0;
                (None, true)
            }
            KeyEvent::Pressed(key) | KeyEvent::Autorepeat(key) => match key {
                Key::Up => {
                    let len = self.entries.len() as i32;
                    self.selected = (self.selected - 1).rem_euclid(len);
                    if self.selected < self.top {
                        self.top = self.selected;
                    }
                    if self.selected - LISTING_SIZE >= self.top {
                        self.top = len - LISTING_SIZE;
                    }
                    trace!("selected: {}, top: {}", self.selected, self.top);
                    (None, true)
                }
                Key::Down => {
                    let len = self.entries.len() as i32;
                    self.selected = (self.selected + 1).rem_euclid(len);
                    if self.selected < self.top {
                        self.top = 0;
                    }
                    if self.selected - LISTING_SIZE >= self.top {
                        self.top = self.selected - LISTING_SIZE + 1;
                    }
                    trace!("selected: {}, top: {}", self.selected, self.top);
                    (None, true)
                }
                Key::Left => {
                    let len = self.entries.len() as i32;
                    self.selected = (self.selected - LISTING_JUMP_SIZE).clamp(0, len - 1);
                    if self.selected < self.top {
                        self.top = self.selected;
                    }
                    (None, true)
                }
                Key::Right => {
                    let len = self.entries.len() as i32;
                    self.selected = (self.selected + LISTING_JUMP_SIZE).clamp(0, len - 1);
                    if self.selected - LISTING_SIZE >= self.top {
                        self.top = self.selected - LISTING_SIZE + 1;
                    }
                    (None, true)
                }
                _ => (None, false),
            },
            _ => (None, false),
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum Sort {
    LastPlayed,
    MostPlayed,
}

impl Sort {
    fn button_hint(&self) -> &'static str {
        match self {
            Sort::LastPlayed => "Sort: Last Played",
            Sort::MostPlayed => "Sort: Most Played",
        }
    }

    fn next(self) -> Self {
        match self {
            Sort::LastPlayed => Sort::MostPlayed,
            Sort::MostPlayed => Sort::LastPlayed,
        }
    }
}
