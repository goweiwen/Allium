use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::{fs, mem};

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::database::Database;
use common::display::Display;
use common::display::font::FontTextStyleBuilder;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, ButtonHints, Keyboard, View};
use log::{error, trace};
use tokio::sync::mpsc::Sender;

pub struct TextReader {
    rect: Rect,
    res: Resources,
    path: PathBuf,
    text: String,
    lowercase_text: String,
    cursor: usize,
    button_hints: ButtonHints<String>,
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

        let locale = res.get::<Locale>();
        let styles = res.get::<Stylesheet>();

        let button_hints = ButtonHints::new(
            res.clone(),
            vec![ButtonHint::new(
                res.clone(),
                Point::zero(),
                Key::Menu,
                locale.t("ingame-menu-continue"),
                Alignment::Left,
            )],
            vec![
                ButtonHint::new(
                    res.clone(),
                    Point::zero(),
                    Key::X,
                    locale.t("guide-button-search"),
                    Alignment::Right,
                ),
                ButtonHint::new(
                    res.clone(),
                    Point::zero(),
                    Key::B,
                    locale.t("button-back"),
                    Alignment::Right,
                ),
            ],
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

    fn visible_text(&self, styles: &Stylesheet, content_height: u32) -> Vec<&str> {
        let line_count = content_height / styles.menu.guide_font.size;
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
        let line_width =
            self.rect.w - styles.ui.margin_x as u32 * 2 - styles.ui.margin_y as u32 * 2;
        let text_style = FontTextStyleBuilder::new(styles.menu.guide_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size(styles.menu.guide_font.size)
            .background_color(styles.ui.background_color)
            .text_color(styles.ui.text_color)
            .build();
        let mut offset = self.text[cursor..]
            .find('\n')
            .or_else(|| self.text[..cursor].rfind('\n'))
            .unwrap_or_default();

        if cursor + offset >= self.text.len() {
            return &self.text[cursor..];
        }

        // Measure text to find line break point
        let mut text_slice = &self.text[cursor..cursor + offset];
        let mut text_size = text_style.measure(text_slice);

        while text_size.w > line_width || text_size.h > styles.menu.guide_font.size {
            offset -= 1;
            while !self.text.is_char_boundary(cursor + offset) {
                offset -= 1;
            }
            text_slice = &self.text[cursor..cursor + offset];
            text_size = text_style.measure(text_slice);
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

    fn search_forward_wrapping(&mut self, needle: String) -> bool {
        if self.search_forward(&needle) {
            self.last_searched = needle;
            return true;
        }
        let cursor = self.cursor;
        self.cursor = 0;
        if self.search_forward(&needle) {
            self.last_searched = needle;
            true
        } else {
            self.cursor = cursor;
            false
        }
    }

    fn search_backward_wrapping(&mut self, needle: String) -> bool {
        if self.search_backward(&needle) {
            self.last_searched = needle;
            return true;
        }
        let cursor = self.cursor;
        self.cursor = self.text.len();
        if self.search_backward(&needle) {
            self.last_searched = needle;
            true
        } else {
            self.cursor = cursor;
            false
        }
    }

    fn search_forward(&mut self, needle: &str) -> bool {
        let mut cursor = self.cursor;
        // Skip the current line
        cursor += self.text[cursor..].find('\n').unwrap_or_default();

        if let Some(location) = self.lowercase_text[cursor..].find(needle) {
            cursor += location;

            // Go back to the start of the line
            cursor = self.text[..cursor].rfind('\n').unwrap_or_default() + 1;
            cursor = cursor.clamp(0, self.text.len() - 1);
        } else {
            return false;
        }
        self.cursor = cursor;

        if self.button_hints.right().len() <= 2 {
            let locale = self.res.get::<Locale>();
            self.button_hints.right_mut().push(ButtonHint::new(
                self.res.clone(),
                Point::zero(),
                Key::L2,
                locale.t("guide-button-next"),
                Alignment::Right,
            ));
            self.button_hints.right_mut().push(ButtonHint::new(
                self.res.clone(),
                Point::zero(),
                Key::R2,
                locale.t("guide-button-prev"),
                Alignment::Right,
            ));
        }
        true
    }

    fn search_backward(&mut self, needle: &str) -> bool {
        let mut cursor = self.cursor;
        if let Some(location) = self.lowercase_text[..cursor].rfind(&needle) {
            cursor = location;

            // Go back to the start of the line
            cursor = self.text[..cursor].rfind('\n').unwrap_or_default() + 1;
            cursor = cursor.clamp(0, self.text.len() - 1);
            self.last_searched = needle.to_owned();
        } else {
            return false;
        }
        self.cursor = cursor;

        if self.button_hints.right().len() <= 2 {
            let locale = self.res.get::<Locale>();
            self.button_hints.right_mut().push(ButtonHint::new(
                self.res.clone(),
                Point::zero(),
                Key::L,
                locale.t("guide-button-next"),
                Alignment::Right,
            ));
            self.button_hints.right_mut().push(ButtonHint::new(
                self.res.clone(),
                Point::zero(),
                Key::R,
                locale.t("guide-button-prev"),
                Alignment::Right,
            ));
        }
        true
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
        self.set_should_draw();
    }

    fn move_forward_lines(&mut self, lines: usize) {
        let styles = self.res.get::<Stylesheet>();
        for _ in 0..lines {
            if self.cursor > self.text.len() {
                self.cursor = self.text.rfind('\n').map(|i| i + 1).unwrap_or_default();
                break;
            }
            if self.cursor != self.text.len() {
                let text = self.get_line(&styles, self.cursor);
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
        drop(styles);

        self.set_should_draw();
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
            let button_hints_rect = self.button_hints.bounding_box(styles);
            display.load(button_hints_rect)?;
            let content_top = self.rect.y;
            let content_height = (button_hints_rect.y - content_top) as u32;

            display.load(Rect::new(
                self.rect.x,
                content_top,
                self.rect.w,
                content_height,
            ))?;

            // Draw content background
            let content_rect = Rect::new(
                self.rect.x + styles.ui.margin_x,
                content_top,
                self.rect.w - styles.ui.margin_x as u32 * 2,
                content_height,
            );
            common::display::fill_rounded_rect(
                &mut display.pixmap_mut(),
                content_rect,
                8,
                styles.ui.background_color,
            );

            let text_style = FontTextStyleBuilder::new(styles.menu.guide_font.font())
                .font_fallback(styles.cjk_font.font())
                .font_size(styles.menu.guide_font.size)
                .background_color(styles.ui.background_color)
                .text_color(styles.ui.text_color)
                .build();

            // Draw visible text lines
            let visible_lines: Vec<&str> = self.visible_text(styles, content_height);
            let mut y = content_top;
            for line in visible_lines {
                let line_pos = Point::new(self.rect.x + styles.ui.margin_x + 12, y);
                text_style.draw(&mut display.pixmap_mut(), line, line_pos);
                y += styles.menu.guide_font.size as i32;
            }

            // Draw percentage indicator
            let percentage_text = format!(
                "{:.0}%",
                self.cursor as f32 / self.text.len().max(1) as f32 * 100.0
            );
            let percentage_size = text_style.measure(&percentage_text);
            let percentage_pos = Point::new(
                self.rect.x + self.rect.w as i32 - 16 - percentage_size.w as i32,
                content_top + content_height as i32
                    - styles.menu.guide_font.size as i32
                    - styles.ui.margin_y / 2,
            );
            text_style.draw(&mut display.pixmap_mut(), &percentage_text, percentage_pos);

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
                .is_some_and(common::view::View::should_draw)
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
                        self.search_forward(&std::mem::take(value).as_string().unwrap());
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
                    self.search_backward_wrapping(last_searched);
                    self.dirty = true;
                }
                KeyEvent::Pressed(Key::R2) => {
                    let last_searched = mem::take(&mut self.last_searched);
                    self.search_forward_wrapping(last_searched);
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
        let mut children: Vec<&dyn View> = vec![&self.button_hints];
        if let Some(ref keyboard) = self.keyboard {
            children.push(keyboard);
        }
        children
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        let mut children: Vec<&mut dyn View> = vec![&mut self.button_hints];
        if let Some(ref mut keyboard) = self.keyboard {
            children.push(keyboard);
        }
        children
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}
