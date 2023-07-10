use std::collections::VecDeque;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;

use common::command::Command;
use common::display::font::FontTextStyleBuilder;
use common::geom::{Point, Rect};
use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use common::view::View;
use embedded_graphics::prelude::{Dimensions, OriginDimensions, Size};
use embedded_graphics::primitives::{
    CornerRadii, Primitive, PrimitiveStyle, Rectangle, RoundedRectangle,
};
use embedded_graphics::text::{Alignment, Text};
use embedded_graphics::Drawable;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct Toast {
    text: String,
    expires: Option<Instant>,
}

impl Toast {
    pub fn new(text: String, duration: Option<Duration>) -> Self {
        Self {
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

        let text_style = FontTextStyleBuilder::new(styles.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size(styles.ui_font.size)
            .background_color(styles.highlight_color)
            .text_color(styles.foreground_color)
            .build();

        let lines = self.text.lines().count() as u32;

        let text = Text::with_alignment(
            &self.text,
            Point::new(w as i32 / 2, (h - styles.ui_font.size * lines) as i32 / 2).into(),
            text_style,
            Alignment::Center,
        );

        let rect = text.bounding_box();
        let x = rect.top_left.x;
        let y = rect.top_left.y;
        let Size { width, height } = rect.size;
        RoundedRectangle::new(
            Rectangle::new(
                Point::new(x - 12, y - 8).into(),
                Size::new(width + 24, height + 16),
            ),
            CornerRadii::new(Size::new_equal(12)),
        )
        .into_styled(PrimitiveStyle::with_fill(styles.highlight_color))
        .draw(display)?;

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
