use std::collections::VecDeque;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;

use common::command::Command;
use common::display::color::Color;
use common::display::font::FontTextStyleBuilder;
use common::geom::{Point, Rect};
use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::stylesheet::{Stylesheet, StylesheetColor};
use common::view::View;
use embedded_graphics::Drawable;
use embedded_graphics::image::ImageRaw;
use embedded_graphics::prelude::{Dimensions, OriginDimensions, Size};
use embedded_graphics::primitives::{
    CornerRadii, Primitive, PrimitiveStyle, Rectangle, RoundedRectangle,
};
use embedded_graphics::text::{Alignment, Text};
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
        let w = display.size().width;
        let h = display.size().height;

        let lines = self.text.lines().count() as u32;
        let mut text_y = (h - styles.ui_font.size * lines) as i32 / 2;

        let image_rect = if let Some(image) = &self.image {
            let image_w = image.width();
            let image_h = image.height();
            let x = (w - image_w) as i32 / 2;
            let y = (h - image_h) as i32 / 2 - styles.margin_y - styles.ui_font.size as i32;

            text_y = y + image_h as i32 + styles.margin_y;

            Some(Rect::new(x, y, image_w, image_h))
        } else {
            None
        };

        let bg_color = StylesheetColor::BackgroundHighlightBlend.to_color(styles);

        let text_style = FontTextStyleBuilder::new(styles.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size(styles.ui_font.size)
            .background_color(bg_color)
            .text_color(styles.foreground_color)
            .build();

        let text = Text::with_alignment(
            &self.text,
            Point::new(w as i32 / 2, text_y).into(),
            text_style,
            Alignment::Center,
        );

        let mut rect = text.bounding_box();
        if let Some(image_rect) = image_rect {
            rect = common::geom::Rect::union(&rect.into(), &image_rect).into();
        }

        let x = rect.top_left.x;
        let y = rect.top_left.y;
        let Size { width, height } = rect.size;
        RoundedRectangle::new(
            Rectangle::new(
                Point::new(x - styles.margin_x, y - styles.margin_y).into(),
                Size::new(
                    width + styles.margin_x as u32 * 2,
                    height + styles.margin_y as u32 * 2,
                ),
            ),
            CornerRadii::new(Size::new_equal(styles.margin_x as u32)),
        )
        .into_styled(PrimitiveStyle::with_fill(bg_color))
        .draw(display)?;

        if let Some(ref image) = self.image
            && let Some(image_rect) = image_rect
        {
            let image_raw: ImageRaw<'_, Color> = ImageRaw::new(image, image_rect.w);
            let image = embedded_graphics::image::Image::new(
                &image_raw,
                embedded_graphics::geometry::Point::new(image_rect.x, image_rect.y),
            );
            image.draw(display)?;
        }

        text.draw(display)?;

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
