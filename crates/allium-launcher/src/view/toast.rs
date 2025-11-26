use std::collections::VecDeque;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;

use common::command::Command;
use common::display::Display;
use common::display::font::FontTextStyleBuilder;
use common::geom::{Point, Rect};
use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::stylesheet::{Stylesheet, StylesheetColor};
use common::view::View;
use image::{ImageBuffer, Rgba};
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct Toast {
    image: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    text: String,
    expires: Option<Instant>,
}

impl Toast {
    pub fn new(text: String, duration: Option<Duration>) -> Self {
        Self {
            image: None,
            text,
            expires: duration.map(|duration| Instant::now() + duration),
        }
    }

    pub fn with_image(
        image: ImageBuffer<Rgba<u8>, Vec<u8>>,
        text: String,
        duration: Option<Duration>,
    ) -> Self {
        Self {
            image: Some(image),
            text,
            expires: duration.map(|duration| Instant::now() + duration),
        }
    }

    pub fn has_expired(&self) -> bool {
        if let Some(expires) = self.expires {
            Instant::now() > expires
        } else {
            false
        }
    }
}

#[async_trait(?Send)]
impl View for Toast {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let w = display.size().w;
        let h = display.size().h;

        let lines = self.text.lines().count() as u32;
        let mut text_y = (h - styles.ui.ui_font.size * lines) as i32 / 2;

        let image_rect = if let Some(image) = &self.image {
            let image_w = image.width();
            let image_h = image.height();
            let x = (w - image_w) as i32 / 2;
            let y = (h - image_h) as i32 / 2 - styles.ui.margin_y - styles.ui.ui_font.size as i32;

            text_y = y + image_h as i32 + styles.ui.margin_y;

            Some(Rect::new(x, y, image_w, image_h))
        } else {
            None
        };

        let bg_color = StylesheetColor::BackgroundHighlightBlend.to_color(styles);

        let text_style = FontTextStyleBuilder::new(styles.ui.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size(styles.ui.ui_font.size)
            .background_color(bg_color)
            .text_color(styles.ui.text_color)
            .build();

        // Measure text to calculate background size
        let text_size = text_style.measure(&self.text);
        let text_x = w as i32 / 2 - text_size.w as i32 / 2;

        let mut bounds_rect = Rect::new(text_x, text_y, text_size.w, text_size.h);
        if let Some(image_rect) = image_rect {
            bounds_rect = Rect::union(&bounds_rect, &image_rect);
        }

        // Draw rounded background
        let bg_rect = Rect::new(
            bounds_rect.x - styles.ui.margin_x,
            bounds_rect.y - styles.ui.margin_y,
            bounds_rect.w + styles.ui.margin_x as u32 * 2,
            bounds_rect.h + styles.ui.margin_y as u32 * 2,
        );
        common::display::fill_rounded_rect(
            &mut display.pixmap_mut(),
            bg_rect,
            styles.ui.margin_x as u32,
            bg_color,
        );

        // Draw optional image
        if let Some(ref image) = self.image
            && let Some(image_rect) = image_rect
        {
            common::display::image::draw_image(
                &mut display.pixmap_mut(),
                image,
                Point::new(image_rect.x, image_rect.y),
            );
        }

        // Draw text
        text_style.draw(
            &mut display.pixmap_mut(),
            &self.text,
            Point::new(text_x, text_y),
        );

        Ok(true)
    }

    fn should_draw(&self) -> bool {
        true
    }

    fn set_should_draw(&mut self) {}

    async fn handle_key_event(
        &mut self,
        _event: KeyEvent,
        _commands: Sender<Command>,
        _bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        Ok(false)
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        Rect::zero()
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}
