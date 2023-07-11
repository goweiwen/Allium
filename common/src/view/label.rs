use std::collections::VecDeque;

use crate::command::Command;
use crate::geom::{Alignment, Point, Rect};
use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::prelude::Dimensions;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::display::color::Color;
use crate::display::font::FontTextStyleBuilder;
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::stylesheet::{Stylesheet, StylesheetColor};
use crate::view::View;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label<S>
where
    S: AsRef<str> + PartialEq + Send,
{
    point: Point,
    text: S,
    alignment: Alignment,
    width: Option<u32>,
    truncated_text: Option<String>,
    color: StylesheetColor,
    background_color: StylesheetColor,
    dirty: bool,
}

impl<S> Label<S>
where
    S: AsRef<str> + PartialEq + Send,
{
    pub fn new(point: Point, text: S, alignment: Alignment, width: Option<u32>) -> Self {
        Self {
            point,
            text,
            alignment,
            width,
            truncated_text: None,
            color: StylesheetColor::Foreground,
            background_color: StylesheetColor::Background,
            dirty: true,
        }
    }

    pub fn color(&mut self, color: StylesheetColor) -> &mut Self {
        self.color = color;
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
            self.dirty = true;
        }
        self
    }

    fn layout(&mut self, styles: &Stylesheet) {
        let text_style = FontTextStyleBuilder::<Color>::new(styles.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size(styles.ui_font.size)
            .build();

        if self.truncated_text.is_none() {
            if let Some(width) = self.width {
                let mut text = Text::with_alignment(
                    self.truncated_text
                        .as_deref()
                        .unwrap_or_else(|| self.text.as_ref()),
                    self.point.into(),
                    text_style.clone(),
                    self.alignment.into(),
                );

                let ellipsis_width = Text::with_alignment(
                    "...",
                    self.point.into(),
                    text_style,
                    self.alignment.into(),
                )
                .bounding_box()
                .size
                .width;

                let mut truncated = false;
                while text.bounding_box().size.width + ellipsis_width > width {
                    let mut n = text.text.len() - 1;
                    while !text.text.is_char_boundary(n) {
                        n -= 1;
                    }
                    text.text = &text.text[..n];
                    truncated = true;
                }
                if truncated {
                    self.truncated_text = Some(format!("{}...", text.text.trim_end()));
                } else {
                    self.truncated_text = Some(text.text.to_string());
                }
            } else {
                self.truncated_text = Some(self.text.as_ref().to_owned());
            }
        }

        self.dirty = true;
    }
}

#[async_trait(?Send)]
impl<S> View for Label<S>
where
    S: AsRef<str> + PartialEq + Send,
{
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let text_style = FontTextStyleBuilder::new(styles.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .text_color(self.color.to_color(styles))
            .background_color(self.background_color.to_color(styles))
            .font_size(styles.ui_font.size)
            .build();

        if self.truncated_text.is_none() {
            self.layout(styles);
        }

        let text = Text::with_alignment(
            self.truncated_text.as_ref().unwrap(),
            self.point.into(),
            text_style,
            self.alignment.into(),
        );

        text.draw(display)?;

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
        if self.truncated_text.is_none() {
            self.layout(styles);
        }

        let text_style = FontTextStyleBuilder::<Color>::new(styles.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size(styles.ui_font.size)
            .build();

        let mut text = self.truncated_text.as_deref().unwrap();
        if text.is_empty() {
            text = " ";
        }
        Text::with_alignment(text, self.point.into(), text_style, self.alignment.into())
            .bounding_box()
            .into()
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.dirty = true;
    }
}
