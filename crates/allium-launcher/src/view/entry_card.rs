use std::collections::VecDeque;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::display::Display;
use common::geom::{Alignment, Point, Rect};
use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use common::view::{Column, Image, ImageMode, Label, View};
use tokio::sync::mpsc::Sender;

use crate::entry::lazy_image::LazyImage;

pub struct EntryCard {
    point: Point,
    image: LazyImage,
    title: String,
    subtitle: String,
    column: Column<Box<dyn View>>,
    image_rect: Option<Rect>,
    focused: bool,
    dirty: bool,
}

impl EntryCard {
    pub fn new(point: Point, image: LazyImage, title: String, subtitle: String) -> Self {
        Self {
            point,
            image,
            title,
            subtitle,
            column: Column::new(point, vec![], 0),
            image_rect: None,
            focused: false,
            dirty: true,
        }
    }

    fn layout(&mut self, styles: &Stylesheet) {
        if !self.dirty {
            return;
        }

        let boxart_width = styles.games.boxart_width;

        let content_x = self.point.x + 4;
        let content_y = self.point.y + 4;

        // Determine image height based on actual image dimensions
        let image_height = if let Some(path) = self.image.try_image() {
            // Try to load image to get dimensions
            if let Ok(img) = image::open(path) {
                let width = img.width();
                let height = img.height();
                // Scale height to fit within boxart_width while maintaining aspect ratio
                let scaled_height = (boxart_width as f32 * height as f32 / width as f32) as u32;
                scaled_height
            } else {
                0
            }
        } else {
            0
        };

        // Create image view only if we have a height > 0
        let mut views: Vec<Box<dyn View>> = Vec::new();

        if image_height > 0 {
            let image_rect = Rect::new(content_x, content_y, boxart_width, image_height);
            self.image_rect = Some(image_rect);

            let mut image_view = Image::empty(image_rect, ImageMode::Cover);
            if let Some(path) = self.image.try_image() {
                image_view.set_path(Some(path.to_path_buf()));
            }
            image_view.set_border_radius(12);
            image_view.set_alignment(Alignment::Center);
            views.push(Box::new(image_view) as Box<dyn View>);
        } else {
            self.image_rect = None;
        }

        // Create title label
        let title_y = content_y + image_height as i32;
        let mut title_label = Label::new(
            Point::new(content_x, title_y),
            self.title.clone(),
            Alignment::Left,
            Some(boxart_width),
        );
        title_label.font_size(0.8);
        let title_height = title_label.bounding_box(styles).h;

        views.push(Box::new(title_label) as Box<dyn View>);

        // Create subtitle label
        let subtitle_y = title_y + title_height as i32;
        let mut subtitle_label = Label::new(
            Point::new(content_x, subtitle_y),
            self.subtitle.clone(),
            Alignment::Left,
            Some(boxart_width),
        );
        subtitle_label.font_size(0.6);
        views.push(Box::new(subtitle_label) as Box<dyn View>);

        // Build column (also offset by 4px)
        self.column = Column::new(Point::new(content_x, content_y), views, 4);

        self.dirty = false;
    }
}

#[async_trait(?Send)]
impl View for EntryCard {
    fn update(&mut self, dt: Duration) {
        self.column.update(dt);
    }

    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        self.layout(styles);
        self.column.draw(display, styles)?;

        // Draw focus stroke if focused
        if self.focused {
            if let Some(image_rect) = self.image_rect {
                let stroke_rect = Rect::new(
                    image_rect.x - 3,
                    image_rect.y - 3,
                    image_rect.w + 6,
                    image_rect.h + 6,
                );

                common::display::stroke_rounded_rect(
                    &mut display.pixmap_mut(),
                    stroke_rect,
                    12.0 + 2.0, // border radius + gap
                    2.0,        // stroke width
                    styles.ui.text_color,
                );
            }
        }

        Ok(true)
    }

    fn should_draw(&self) -> bool {
        self.dirty || self.column.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        self.column.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        command: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        self.column.handle_key_event(event, command, bubble).await
    }

    fn children(&self) -> Vec<&dyn View> {
        self.column.children()
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        self.column.children_mut()
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        self.layout(styles);

        const STROKE_OFFSET: i32 = 5;
        let column_bbox = self.column.bounding_box(styles);

        // The bounding box should start at self.point and extend by STROKE_OFFSET on all sides
        Rect::new(
            self.point.x,
            self.point.y,
            (column_bbox.w as i32 + STROKE_OFFSET * 2) as u32,
            (column_bbox.h as i32 + STROKE_OFFSET * 2) as u32,
        )
    }

    fn set_position(&mut self, point: Point) {
        if self.point == point {
            return;
        }
        self.point = point;
        self.dirty = true;
    }

    fn focus(&mut self) {
        self.focused = true;
        self.dirty = true;
    }

    fn blur(&mut self) {
        self.focused = false;
        self.dirty = true;
    }
}
