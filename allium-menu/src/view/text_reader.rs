use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::{fs, mem};

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::database::Database;
use common::display::font::FontTextStyleBuilder;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, Row, View};
use common::view::{ButtonIcon, Keyboard};
use embedded_graphics::prelude::{Dimensions, Size};
use embedded_graphics::primitives::{Primitive, PrimitiveStyle, Rectangle, RoundedRectangle};
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use log::{error, trace};
use tokio::sync::mpsc::Sender;

pub struct TextReader {
    rect: Rect,
    res: Resources,
    path: PathBuf,
    text: String,
    lowercase_text: String,
    cursor: usize,
    button_hints: Row<ButtonHint<String>>,
    keyboard: Option<Keyboard>,
    last_searched: String,
    dirty: bool,
}

impl TextReader {
    #[must_use]
    pub fn new(rect: Rect, res: Resources, path: PathBuf) -> Self {
        let text = fs::read_to_string(&path)
            .map_err(|e| error!("failed to load guide file: {}", e))
            .unwrap_or_default();
        let lowercase_text = text.to_lowercase();

        let mut cursor = if text.is_empty() {
            0
        } else {
            load_cursor(&res.get::<Database>(), path.as_path()).clamp(0, text.len() - 1)
        };
        while !text.is_char_boundary(cursor) && cursor > 0 {
            cursor -= 1;
        }

        let Rect { x, y, w, h } = rect;

        let locale = res.get::<Locale>();
        let styles = res.get::<Stylesheet>();

        let button_hints = Row::new(
            Point::new(
                x + w as i32 - 12,
                y + h as i32 - ButtonIcon::diameter(&styles) as i32 - 8,
            ),
            vec![
                ButtonHint::new(
                    Point::zero(),
                    Key::X,
                    locale.t("guide-button-search"),
                    Alignment::Right,
                ),
                ButtonHint::new(
                    Point::zero(),
                    Key::B,
                    locale.t("button-back"),
                    Alignment::Right,
                ),
            ],
            Alignment::Right,
            12,
        );

        drop(locale);
        drop(styles);

        Self {
            rect,
            res,
            path,
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
        self.res
            .get::<Database>()
            .update_guide_cursor(&self.path, self.cursor as u64)
            .map_err(|e| error!("failed to update guide cursor to database: {}", e))
            .ok();
    }

    fn visible_text(&self, styles: &Stylesheet) -> Vec<&str> {
        let line_count =
            (self.rect.h - 12 - 8 - ButtonIcon::diameter(styles) - 8) / styles.guide_font.size;
        let mut lines = Vec::with_capacity(line_count as usize);
        let mut cursor = self.cursor;
        for _ in 0..line_count {
            let line = self.get_line(styles, cursor);
            lines.push(line);
            cursor += line.len();
            if self.text.is_char_boundary(cursor)
                && self.text[cursor..]
                    .chars()
                    .next()
                    .map(|c| c == '\n')
                    .unwrap_or_default()
            {
                cursor += 1;
            }
        }

        lines
    }

    fn get_line(&self, styles: &Stylesheet, cursor: usize) -> &str {
        let line_width = self.rect.w - 24 - 24;
        let text_style = FontTextStyleBuilder::new(styles.guide_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size(styles.guide_font.size)
            .background_color(styles.background_color)
            .text_color(styles.foreground_color)
            .build();
        let mut offset = self.text[cursor..]
            .find('\n')
            .or_else(|| self.text[..cursor].rfind('\n'))
            .unwrap_or_default();

        if cursor + offset >= self.text.len() {
            return &self.text[cursor..];
        }

        let mut text = Text::new(
            &self.text[cursor..cursor + offset],
            Point::zero().into(),
            text_style,
        );

        while text.bounding_box().size.width > line_width
            || text.bounding_box().size.height > styles.guide_font.size
        {
            offset -= 1;
            while !self.text.is_char_boundary(cursor + offset) {
                offset -= 1;
            }
            text.text = &self.text[cursor..cursor + offset];
        }

        let offset_without_word_wrap = offset;

        // If not linebreak, we try to break at the start of the word
        if offset > 0
            && self.text[cursor + offset..]
                .chars()
                .next()
                .unwrap_or_default()
                .is_alphanumeric()
        {
            offset -= 1;
            while !self.text.is_char_boundary(cursor + offset) {
                offset -= 1;
            }
            if &self.text[cursor + offset..cursor + offset] != "\n" {
                while self.text[cursor + offset..]
                    .chars()
                    .next()
                    .unwrap_or_default()
                    .is_alphanumeric()
                {
                    offset -= 1;
                    while !self.text.is_char_boundary(cursor + offset) {
                        offset -= 1;
                    }

                    if offset == 0 {
                        offset = offset_without_word_wrap;
                        break;
                    }
                }
                offset += 1;
                while !self.text.is_char_boundary(cursor + offset) {
                    offset += 1;
                }
            }
        }

        &self.text[cursor..cursor + offset]
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
            let locale = self.res.get::<Locale>();
            self.button_hints.push(ButtonHint::new(
                Point::zero(),
                Key::L2,
                locale.t("guide-next"),
                Alignment::Right,
            ));
            self.button_hints.push(ButtonHint::new(
                Point::zero(),
                Key::R2,
                locale.t("guide-prev"),
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
            let locale = self.res.get::<Locale>();
            self.button_hints.push(ButtonHint::new(
                Point::zero(),
                Key::L,
                locale.t("guide-next"),
                Alignment::Right,
            ));
            self.button_hints.push(ButtonHint::new(
                Point::zero(),
                Key::R,
                locale.t("guide-prev"),
                Alignment::Right,
            ));
        }
    }

    fn move_back_lines(&mut self, lines: usize) {
        let styles = self.res.get::<Stylesheet>();

        // Keep moving back until we've moved back the requested number of lines
        let mut cursor;
        let mut lines = lines as isize;
        while lines > 0 {
            if self.cursor == 0 {
                lines = 0;
                break;
            }

            // Move to the start of the previous line
            self.cursor -= 1;
            while !self.text.is_char_boundary(self.cursor) {
                self.cursor -= 1;
            }

            cursor = self.cursor;
            self.cursor = self.text[..cursor]
                .rfind('\n')
                .map(|i| i + 1)
                .unwrap_or_default();

            // Count the number of lines we moved back
            let mut line = self.get_line(&styles, self.cursor);
            if line.is_empty() {
                lines -= 1;
                continue;
            }
            let mut mid = self.cursor;
            while mid < cursor {
                mid += line.len();
                line = self.get_line(&styles, mid);
                lines -= 1;
            }
        }
        drop(styles);

        // If we overshot, move forward as many times as necessary
        if lines < 0 {
            self.move_forward_lines(-lines as usize);
        }
        self.dirty = true;
    }

    fn move_forward_lines(&mut self, lines: usize) {
        let styles = &self.res.get::<Stylesheet>();
        for _ in 0..lines {
            if self.cursor > self.text.len() {
                self.cursor = self.text.rfind('\n').map(|i| i + 1).unwrap_or_default();
                break;
            }
            if self.cursor != self.text.len() {
                let text = self.get_line(styles, self.cursor);
                self.cursor += text.len();
                if self.text.is_char_boundary(self.cursor)
                    && self.text[self.cursor..]
                        .chars()
                        .next()
                        .map(|c| c == '\n')
                        .unwrap_or_default()
                {
                    self.cursor += 1;
                }
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
                    self.rect.h - 12 - 8 - ButtonIcon::diameter(styles) - 8,
                )),
                Size::new_equal(8),
            )
            .into_styled(PrimitiveStyle::with_fill(styles.background_color))
            .draw(display)?;

            let text_style = FontTextStyleBuilder::new(styles.guide_font.font())
                .font_fallback(styles.cjk_font.font())
                .font_size(styles.guide_font.size)
                .background_color(styles.background_color)
                .text_color(styles.foreground_color)
                .build();

            let mut y = self.rect.y + 12 + 8;
            for line in self.visible_text(styles) {
                let text = Text::new(
                    line,
                    Point::new(self.rect.x + 12 + 12, y).into(),
                    text_style.clone(),
                );
                text.draw(display)?;
                y += styles.guide_font.size as i32;
            }

            Text::with_alignment(
                &format!(
                    "{:.0}%",
                    self.cursor as f32 / self.text.len().max(1) as f32 * 100.0
                ),
                Point::new(
                    self.rect.x + self.rect.w as i32 - 16,
                    self.rect.y + self.rect.h as i32
                        - styles.guide_font.size as i32
                        - 8
                        - ButtonIcon::diameter(styles) as i32
                        - 8,
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
            || self
                .keyboard
                .as_ref()
                .map_or(false, common::view::View::should_draw)
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
            if keyboard.handle_key_event(event, commands, bubble).await? {
                bubble.retain_mut(|cmd| match cmd {
                    Command::CloseView => {
                        self.keyboard = None;
                        false
                    }
                    Command::ValueChanged(_, value) => {
                        self.search_forward(std::mem::take(value).as_string().unwrap());
                        false
                    }
                    _ => true,
                });
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            match event {
                KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                    self.move_back_lines(1);
                }
                KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                    self.move_forward_lines(1);
                }
                KeyEvent::Pressed(Key::L) | KeyEvent::Autorepeat(Key::L) => {
                    self.move_back_lines(10);
                }
                KeyEvent::Pressed(Key::R) | KeyEvent::Autorepeat(Key::R) => {
                    self.move_forward_lines(10);
                }
                KeyEvent::Pressed(Key::L2) => {
                    let last_searched = mem::take(&mut self.last_searched);
                    self.search_backward(last_searched);
                    self.dirty = true;
                }
                KeyEvent::Pressed(Key::R2) => {
                    let last_searched = mem::take(&mut self.last_searched);
                    self.search_forward(last_searched);
                    self.dirty = true;
                }
                KeyEvent::Pressed(Key::B) => {
                    self.save_cursor();
                    bubble.push_back(Command::CloseView);
                }
                KeyEvent::Pressed(Key::X) => {
                    self.keyboard = Some(Keyboard::new(
                        self.res.clone(),
                        mem::take(&mut self.last_searched),
                        false,
                    ));
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
