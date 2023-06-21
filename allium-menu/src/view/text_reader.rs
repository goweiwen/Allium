use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::{fs, mem};

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::BUTTON_DIAMETER;
use common::database::Database;
use common::display::font::FontTextStyleBuilder;
use common::geom::{Alignment, Point, Rect};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use common::view::keyboard::Keyboard;
use common::view::{ButtonHint, Row, View};
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::{Primitive, PrimitiveStyle, Rectangle, RoundedRectangle};
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use embedded_text::alignment::HorizontalAlignment;
use embedded_text::style::{HeightMode, TextBoxStyleBuilder, VerticalOverdraw};
use embedded_text::TextBox;
use tokio::sync::mpsc::Sender;
use tracing::{error, trace};

pub struct TextReader {
    rect: Rect,
    path: PathBuf,
    database: Database,
    text: String,
    lowercase_text: String,
    cursor: usize,
    button_hints: Row<ButtonHint<&'static str>>,
    keyboard: Option<Keyboard>,
    last_searched: String,
    dirty: bool,
}

impl TextReader {
    pub fn new(rect: Rect, path: PathBuf, database: Database) -> Self {
        let text = fs::read_to_string(&path)
            .map_err(|e| error!("failed to load guide file: {}", e))
            .unwrap_or_default();
        let lowercase_text = text.to_lowercase();

        let cursor = load_cursor(&database, path.as_path()).clamp(0, text.len());

        let Rect { x, y, w, h } = rect;
        let button_hints = Row::new(
            Point::new(x + w as i32 - 12, y + h as i32 - BUTTON_DIAMETER as i32 - 8),
            vec![
                ButtonHint::new(Point::zero(), Key::X, "Search", Alignment::Right),
                ButtonHint::new(Point::zero(), Key::B, "Back", Alignment::Right),
            ],
            Alignment::Right,
            12,
        );

        Self {
            rect,
            path,
            database,
            text,
            lowercase_text,
            cursor,
            button_hints,
            keyboard: None,
            dirty: true,
            last_searched: String::new(),
        }
    }

    pub fn save_cursor(&self) {
        self.database
            .update_guide_cursor(&self.path, self.cursor as u64)
            .map_err(|e| error!("failed to update guide cursor to database: {}", e))
            .ok();
    }

    fn visible_text(&self, styles: &Stylesheet) -> &str {
        let text = &self.text[self.cursor..];

        let mut end = 0;
        let line_count = self.rect.h / styles.mono_font_size;
        for _ in 0..line_count {
            if end >= text.len() {
                break;
            }
            if let Some(pos) = text[end..].find('\n') {
                end += 1 + pos;
            } else {
                return text;
            }
        }

        &text[..end]
    }

    fn search_forward(&mut self, needle: String) {
        // Skip the current line
        self.cursor += self.text[self.cursor..].find('\n').unwrap_or_default();

        if let Some(location) = self.lowercase_text[self.cursor..].find(&needle) {
            self.cursor += location;

            // Go back to the start of the line
            self.cursor = self.text[..self.cursor].rfind('\n').unwrap_or_default() + 1;
            self.cursor = self.cursor.clamp(0, self.text.len() - 1);
            self.last_searched = needle;
        } else {
            self.cursor = 0;
            self.search_forward(needle);
        }

        if self.button_hints.children().len() <= 2 {
            self.button_hints.push(ButtonHint::new(
                Point::zero(),
                Key::L,
                "Next",
                Alignment::Right,
            ));
            self.button_hints.push(ButtonHint::new(
                Point::zero(),
                Key::R,
                "Prev",
                Alignment::Right,
            ));
        }
    }

    fn search_backward(&mut self, needle: String) {
        if let Some(location) = self.lowercase_text[..self.cursor].rfind(&needle) {
            self.cursor = location;

            // Go back to the start of the line
            self.cursor = self.text[..self.cursor].rfind('\n').unwrap_or_default() + 1;
            self.cursor = self.cursor.clamp(0, self.text.len() - 1);
            self.last_searched = needle;
        } else {
            self.cursor = self.text.len();
            self.search_backward(needle);
        }

        if self.button_hints.children().len() <= 2 {
            self.button_hints.push(ButtonHint::new(
                Point::zero(),
                Key::L,
                "Next",
                Alignment::Right,
            ));
            self.button_hints.push(ButtonHint::new(
                Point::zero(),
                Key::R,
                "Prev",
                Alignment::Right,
            ));
        }
    }

    fn advance_cursor(&mut self) {
        self.cursor = (self.cursor + 1).min(self.text.len());
        while !self.text.is_char_boundary(self.cursor) {
            self.cursor += 1;
        }
    }

    fn move_back_lines(&mut self, lines: usize) {
        for _ in 0..lines {
            if self.cursor > 0 {
                self.cursor = self.text[..self.cursor - 1].rfind('\n').unwrap_or_default();
                self.advance_cursor();
            }
        }
        self.dirty = true;
    }

    fn move_forward_lines(&mut self, lines: usize) {
        for _ in 0..lines {
            if self.cursor > self.text.len() {
                self.cursor = self.text.rfind('\n').map(|i| i + 1).unwrap_or_default();
                break;
            }
            if self.cursor != self.text.len() {
                self.advance_cursor();
                self.cursor += self.text[self.cursor..]
                    .find('\n')
                    .or_else(|| self.text[..self.cursor].rfind('\n'))
                    .unwrap_or_default();
                self.advance_cursor();
            }
        }
        self.dirty = true;
    }
}

fn load_cursor(database: &Database, path: &Path) -> usize {
    database
        .get_guide_cursor(path)
        .map_err(|e| error!("failed to load guide cursor from database: {}", e))
        .unwrap_or_default() as usize
}

#[async_trait(?Send)]
impl View for TextReader {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let mut drawn = false;

        if self.dirty {
            RoundedRectangle::with_equal_corners(
                <Rect as Into<Rectangle>>::into(Rect::new(
                    self.rect.x + 12,
                    self.rect.y + 12,
                    self.rect.w - 24,
                    self.rect.h - 48 - 12,
                )),
                Size::new_equal(8),
            )
            .into_styled(PrimitiveStyle::with_fill(styles.background_color))
            .draw(display)?;

            let text_style = FontTextStyleBuilder::new(styles.mono_font.clone())
                .font_size(styles.mono_font_size)
                .background_color(styles.background_color)
                .text_color(styles.foreground_color)
                .draw_background()
                .build();

            let text_box_style = TextBoxStyleBuilder::new()
                .height_mode(HeightMode::Exact(VerticalOverdraw::Hidden))
                .alignment(HorizontalAlignment::Left)
                .build();

            TextBox::with_textbox_style(
                self.visible_text(styles),
                Rect::new(
                    self.rect.x + 24,
                    self.rect.y + 16,
                    self.rect.w - 48,
                    self.rect.h - 48 - 12 - 4,
                )
                .into(),
                text_style.clone(),
                text_box_style,
            )
            .draw(display)?;

            Text::with_alignment(
                &format!(
                    "{:.0}%",
                    self.cursor as f32 / self.text.len().max(1) as f32 * 100.0
                ),
                Point::new(
                    self.rect.x + self.rect.w as i32 - 16,
                    self.rect.y + self.rect.h as i32 - styles.mono_font_size as i32 - 48 - 8,
                )
                .into(),
                text_style,
                Alignment::Right.into(),
            )
            .draw(display)?;

            self.dirty = false;

            trace!("drawing text reader");

            drawn = true;
        }

        drawn |= self.button_hints.draw(display, styles)?;

        if let Some(keyboard) = self.keyboard.as_mut() {
            drawn |= keyboard.draw(display, styles)?;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.dirty
            || self.button_hints.should_draw()
            || self.keyboard.as_ref().map_or(false, |k| k.should_draw())
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        self.button_hints.set_should_draw();
        if let Some(keyboard) = self.keyboard.as_mut() {
            keyboard.set_should_draw();
        }
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if let Some(keyboard) = self.keyboard.as_mut() {
            let mut child_bubble = VecDeque::new();
            if keyboard
                .handle_key_event(event, commands, &mut child_bubble)
                .await?
            {
                while let Some(cmd) = child_bubble.pop_front() {
                    match cmd {
                        Command::CloseView => {
                            self.keyboard = None;
                        }
                        Command::ValueChanged(_, value) => {
                            self.search_forward(value.as_string().unwrap());
                        }
                        cmd => bubble.push_back(cmd),
                    }
                }
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            match event {
                KeyEvent::Pressed(Key::Up) => {
                    self.move_back_lines(1);
                }
                KeyEvent::Autorepeat(Key::Up) => {
                    self.move_back_lines(3);
                }
                KeyEvent::Pressed(Key::Down) => {
                    self.move_forward_lines(1);
                }
                KeyEvent::Autorepeat(Key::Down) => {
                    self.move_forward_lines(3);
                }
                KeyEvent::Pressed(Key::Left) => {
                    self.move_back_lines(10);
                }
                KeyEvent::Autorepeat(Key::Left) => {
                    self.move_back_lines(30);
                }
                KeyEvent::Pressed(Key::Right) => {
                    self.move_forward_lines(10);
                }
                KeyEvent::Autorepeat(Key::Right) => {
                    self.move_forward_lines(30);
                }
                KeyEvent::Pressed(Key::L) => {
                    let last_searched = mem::take(&mut self.last_searched);
                    self.search_backward(last_searched);
                    self.dirty = true;
                }
                KeyEvent::Pressed(Key::R) => {
                    let last_searched = mem::take(&mut self.last_searched);
                    self.search_forward(last_searched);
                    self.dirty = true;
                }
                KeyEvent::Pressed(Key::B) => {
                    self.save_cursor();
                    bubble.push_back(Command::CloseView);
                }
                KeyEvent::Pressed(Key::X) => {
                    self.keyboard = Some(Keyboard::new(mem::take(&mut self.last_searched), false));
                }
                _ => return Ok(false),
            }
            Ok(true)
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![]
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}
