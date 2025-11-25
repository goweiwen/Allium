use std::collections::VecDeque;
use std::time::Duration;

use crate::command::Command;
use crate::geom::{Alignment, Point, Rect};
use anyhow::Result;
use async_trait::async_trait;
use log::trace;
use tokio::sync::mpsc::Sender;

use crate::display::Display;
use crate::display::font::FontTextStyleBuilder;
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::stylesheet::{Stylesheet, StylesheetColor};
use crate::view::View;

#[derive(Debug, Clone)]
struct Scrolling {
    offset: usize,
    dt: Duration,
}

#[derive(Debug, Clone)]
pub struct Label<S>
where
    S: AsRef<str> + PartialEq + Send,
{
    rect: Option<Rect>,
    point: Point,
    text: S,
    alignment: Alignment,
    width: Option<u32>,
    truncated_text: Option<String>,
    color: StylesheetColor,
    stroke_color: StylesheetColor,
    font_size: f32,
    scrolling: Option<Scrolling>,
    dirty: bool,
}

const SCROLL_DELAY: Duration = Duration::from_millis(1000);
const SCROLL_INTERVAL: Duration = Duration::from_micros(166_667);

impl<S> Label<S>
where
    S: AsRef<str> + PartialEq + Send,
{
    pub fn new(point: Point, text: S, alignment: Alignment, width: Option<u32>) -> Self {
        Self {
            rect: None,
            point,
            text,
            alignment,
            width,
            truncated_text: None,
            color: StylesheetColor::Foreground,
            stroke_color: StylesheetColor::Stroke,
            font_size: 1.0,
            scrolling: None,
            dirty: true,
        }
    }

    pub fn scroll(&mut self, enabled: bool) -> &mut Self {
        if enabled && self.width.is_some() {
            self.scrolling = Some(Scrolling {
                offset: 0,
                dt: Duration::from_millis(0),
            });
            self.truncated_text = None;
        } else {
            self.truncated_text = None;
            self.scrolling = None;
        }
        self
    }

    pub fn color(&mut self, color: StylesheetColor) -> &mut Self {
        self.color = color;
        self.dirty = true;
        self
    }

    pub fn stroke_color(&mut self, stroke_color: StylesheetColor) -> &mut Self {
        self.stroke_color = stroke_color;
        self.dirty = true;
        self
    }

    pub fn text(&self) -> &str {
        self.text.as_ref()
    }

    pub fn set_text(&mut self, text: S) -> &mut Self {
        if self.text != text {
            self.text = text;
            self.truncated_text = None;
            self.rect = None;
            self.dirty = true;
        }
        self
    }

    pub fn font_size(&mut self, font_size: f32) -> &mut Self {
        self.font_size = font_size;
        self
    }

    fn layout(&mut self, styles: &Stylesheet) {
        if self.truncated_text.is_some() {
            return;
        }

        self.dirty = true;

        let text_style = FontTextStyleBuilder::new(styles.ui.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size((styles.ui.ui_font.size as f32 * self.font_size) as u32)
            .build();

        let size = text_style.measure(self.text.as_ref());
        let rect = Rect {
            x: self.point.x,
            y: self.point.y,
            w: size.w,
            h: size.h,
        };
        self.rect = Some(rect);

        if let Some(width) = self.width {
            if let Some(scrolling) = self.scrolling.as_ref() {
                let scroll_text = self
                    .text
                    .as_ref()
                    .chars()
                    .chain("     ".chars())
                    .chain(self.text.as_ref().chars().take(scrolling.offset))
                    .skip(scrolling.offset)
                    .collect::<String>();

                let mut truncated = scroll_text.clone();
                while text_style.measure(&truncated).w > width && !truncated.is_empty() {
                    let mut n = truncated.len() - 1;
                    while !truncated.is_char_boundary(n) {
                        n -= 1;
                    }
                    truncated = truncated[..n].to_string();
                }
                self.truncated_text = Some(truncated.trim_end().to_string());
            } else {
                let ellipsis_width = text_style.measure("...").w;

                let text_width = text_style.measure(self.text.as_ref()).w;
                let mut truncated = false;
                let text_str = self.text.as_ref();

                if text_width > width {
                    let mut current = text_str.to_string();
                    while text_style.measure(&current).w + ellipsis_width > width
                        && !current.is_empty()
                    {
                        let mut n = current.len() - 1;
                        while !current.is_char_boundary(n) {
                            n -= 1;
                        }
                        current = current[..n].to_string();
                        truncated = true;
                    }
                    if truncated {
                        self.truncated_text = Some(format!("{}...", current.trim_end()));
                    } else {
                        self.truncated_text = Some(text_str.to_string());
                    }
                } else {
                    self.truncated_text = Some(text_str.to_string());
                }
            }
        } else {
            self.truncated_text = Some(self.text.as_ref().to_owned());
        }
    }

    /// Calculate the drawing position based on alignment
    fn get_draw_position(&self, text_width: u32) -> Point {
        match self.alignment {
            Alignment::Left => self.point,
            Alignment::Center => Point {
                x: self.point.x - text_width as i32 / 2,
                y: self.point.y,
            },
            Alignment::Right => Point {
                x: self.point.x - text_width as i32,
                y: self.point.y,
            },
        }
    }
}

#[async_trait(?Send)]
impl<S> View for Label<S>
where
    S: AsRef<str> + PartialEq + Send,
{
    fn update(&mut self, dt: Duration) {
        let Some(scrolling) = self.scrolling.as_mut() else {
            return;
        };
        let Some(rect) = self.rect else {
            trace!("haven't calculated rect, skip for now");
            return;
        };
        let Some(width) = self.width else {
            trace!("we don't have any width, we don't need to scroll");
            self.scroll(false);
            return;
        };

        if rect.w < width {
            trace!("text is smaller than width, we don't need to scroll");
            self.scroll(false);
            return;
        }

        scrolling.dt += dt;

        let offset = scrolling.offset;
        while scrolling.dt > SCROLL_DELAY {
            scrolling.dt -= SCROLL_INTERVAL;
            scrolling.offset += 1;
        }

        if offset >= self.text.as_ref().chars().count() + 5 {
            scrolling.offset = 0;
        }

        if scrolling.offset != offset {
            self.truncated_text = None;
            self.set_should_draw();
        }
    }

    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let text_style = FontTextStyleBuilder::new(styles.ui.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .text_color(self.color.to_color(styles))
            .font_size((styles.ui.ui_font.size as f32 * self.font_size) as u32)
            .stroke_width(styles.ui.stroke_width)
            .stroke_color(self.stroke_color.to_color(styles))
            .build();

        if self.truncated_text.is_none() {
            self.layout(styles);
        }

        let truncated_text = self.truncated_text.as_ref().unwrap();
        if !truncated_text.is_empty() {
            let text_width = text_style.measure(truncated_text).w;
            let draw_pos = self.get_draw_position(text_width);

            text_style.draw(&mut display.pixmap_mut(), truncated_text, draw_pos);
        }

        self.dirty = false;
        Ok(true)
    }

    fn should_draw(&self) -> bool {
        self.dirty
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
    }

    async fn handle_key_event(
        &mut self,
        _event: KeyEvent,
        _command: Sender<Command>,
        _bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        Ok(false)
    }

    fn children(&self) -> Vec<&dyn View> {
        Vec::new()
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        Vec::new()
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        let text_style = FontTextStyleBuilder::new(styles.ui.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size((styles.ui.ui_font.size as f32 * self.font_size) as u32)
            .build();

        let size = text_style.measure(self.text.as_ref());
        let mut w = size.w;

        if let Some(width) = self.width {
            w = w.min(width);
        }

        // Adjust x position based on alignment (same as draw)
        let x = match self.alignment {
            Alignment::Left => self.point.x,
            Alignment::Center => self.point.x - w as i32 / 2,
            Alignment::Right => self.point.x - w as i32,
        };

        Rect {
            x,
            y: self.point.y,
            w,
            h: size.h,
        }
    }

    fn set_position(&mut self, point: Point) {
        if self.point == point {
            return;
        }
        self.point = point;
        self.dirty = true;
    }

    fn focus(&mut self) {
        self.color = StylesheetColor::HighlightText;
        self.stroke_color = StylesheetColor::HighlightTextStroke;
        self.dirty = true;
    }

    fn blur(&mut self) {
        self.color = StylesheetColor::Foreground;
        self.stroke_color = StylesheetColor::Stroke;
        self.dirty = true;
    }
}
