use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::image::ImageRaw;
use embedded_graphics::Drawable;
use image::{GenericImageView, RgbImage};
use log::{error, trace};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::command::Command;
use crate::display::color::Color;
use crate::display::image::round;
use crate::display::Display;
use crate::geom::{Point, Rect};
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::stylesheet::{Stylesheet, StylesheetColor};
use crate::view::View;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ImageMode {
    /// Don't scale the image
    Raw,
    /// Scale the image to fill the rect, but maintain the aspect ratio.
    Cover,
    /// Scale the image to fit the rect, but maintain the aspect ratio.
    Contain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    rect: Rect,
    path: Option<PathBuf>,
    #[serde(skip)]
    image: Option<RgbImage>,
    mode: ImageMode,
    background_color: Option<StylesheetColor>,
    border_radius: u32,
    dirty: bool,
}

impl Image {
    pub fn new(rect: Rect, path: PathBuf, mode: ImageMode) -> Self {
        Self {
            rect,
            path: Some(path),
            image: None,
            mode,
            background_color: None,
            border_radius: 0,
            dirty: true,
        }
    }

    pub fn set_background_color(&mut self, color: StylesheetColor) -> &mut Self {
        self.background_color = Some(color);
        self.dirty = self.border_radius != 0;
        self
    }

    pub fn set_border_radius(&mut self, radius: u32) -> &mut Self {
        self.border_radius = radius;
        self.dirty = true;
        self
    }

    pub fn empty(rect: Rect, mode: ImageMode) -> Self {
        Self {
            rect,
            path: None,
            image: None,
            mode,
            background_color: None,
            border_radius: 0,
            dirty: true,
        }
    }

    pub fn set_path(&mut self, path: Option<PathBuf>) -> &mut Self {
        if path != self.path {
            self.image = None;
            self.dirty = true;
            self.path = path;
        }
        self
    }
}

#[async_trait(?Send)]
impl View for Image {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if self.image.is_none() {
            if let Some(ref path) = self.path {
                self.image = image(
                    path,
                    self.rect,
                    self.mode,
                    self.background_color.map(|c| c.to_color(styles)),
                    self.border_radius,
                );
            }
        }

        display.load(self.rect)?;
        if let Some(ref image) = self.image {
            let image: ImageRaw<'_, Color> = ImageRaw::new(image, self.rect.w);
            let image = embedded_graphics::image::Image::new(&image, self.rect.top_left().into());
            trace!("drawing image: {:?}", self.rect);
            image.draw(display)?;
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

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, point: Point) {
        self.rect.x = point.x;
        self.rect.y = point.y;
        self.dirty = true;
    }
}

fn image(
    path: &Path,
    rect: Rect,
    mode: ImageMode,
    background_color: Option<Color>,
    border_radius: u32,
) -> Option<RgbImage> {
    let mut image = ::image::open(path)
        .map_err(|e| error!("Failed to load image at {}: {}", path.display(), e))
        .ok()?;
    match mode {
        ImageMode::Raw => {}
        ImageMode::Cover => {
            image = image.resize_to_fill(rect.w, rect.h, image::imageops::FilterType::Nearest);
        }
        ImageMode::Contain => {
            let new_height = rect.h.min(rect.w * image.height() / image.width());
            image = image.resize_to_fill(rect.w, new_height, image::imageops::FilterType::Nearest);
        }
    }
    let mut image = image.to_rgb8();
    if border_radius != 0 {
        if let Some(background_color) = background_color {
            round(&mut image, background_color.into(), border_radius);
        }
    }
    Some(image)
}
