use std::collections::VecDeque;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::image::ImageRaw;
use embedded_graphics::Drawable;
use image::{imageops, GenericImageView, RgbaImage};
use log::{error, trace};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::command::Command;
use crate::display::color::Color;
use crate::display::image::round;
use crate::display::Display;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
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
    image: Option<RgbaImage>,
    mode: ImageMode,
    border_radius: u32,
    alignment: Alignment,
    dirty: bool,
}

impl Image {
    pub fn new(rect: Rect, path: PathBuf, mode: ImageMode) -> Self {
        Self {
            rect,
            path: Some(path),
            image: None,
            mode,
            border_radius: 0,
            alignment: Alignment::Left,
            dirty: true,
        }
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
            border_radius: 0,
            alignment: Alignment::Left,
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

    pub fn set_alignment(&mut self, alignment: Alignment) -> &mut Self {
        self.alignment = alignment;
        self
    }

    fn image(
        &self,
        path: &Path,
        rect: Rect,
        mode: ImageMode,
        border_radius: u32,
    ) -> Option<RgbaImage> {
        let image = ::image::open(path)
            .map_err(|e| error!("Failed to load image at {}: {}", path.display(), e))
            .ok()?;
        let mut image = match mode {
            ImageMode::Raw => image.to_rgba8(),
            ImageMode::Cover => {
                if image.width() == rect.w && image.height() == rect.h {
                    image.to_rgba8()
                } else {
                    let src_image = fast_image_resize::Image::from_vec_u8(
                        NonZeroU32::new(image.width())?,
                        NonZeroU32::new(image.height())?,
                        image.to_rgba8().into_raw(),
                        fast_image_resize::PixelType::U8x3,
                    )
                    .map_err(|e| error!("Failed to load image at {}: {}", path.display(), e))
                    .ok()?;
                    let mut dst_image = fast_image_resize::Image::new(
                        NonZeroU32::new(rect.w)?,
                        NonZeroU32::new(rect.h)?,
                        src_image.pixel_type(),
                    );
                    let mut resizer =
                        fast_image_resize::Resizer::new(fast_image_resize::ResizeAlg::Nearest);
                    resizer
                        .resize(&src_image.view(), &mut dst_image.view_mut())
                        .ok()?;
                    RgbaImage::from_raw(rect.w, rect.h, dst_image.into_vec())?
                }
            }
            ImageMode::Contain => {
                if image.width() == rect.w && image.height() == rect.h {
                    image.to_rgba8()
                } else {
                    let new_height = rect.h.min(rect.w * image.height() / image.width());
                    let new_width = rect.w.min(rect.h * image.width() / image.height());
                    let src_image = fast_image_resize::Image::from_vec_u8(
                        NonZeroU32::new(image.width())?,
                        NonZeroU32::new(image.height())?,
                        image.to_rgba8().into_raw(),
                        fast_image_resize::PixelType::U8x4,
                    )
                    .map_err(|e| error!("Failed to load image at {}: {}", path.display(), e))
                    .ok()?;
                    let mut dst_image = fast_image_resize::Image::new(
                        NonZeroU32::new(new_width)?,
                        NonZeroU32::new(new_height)?,
                        src_image.pixel_type(),
                    );
                    let mut resizer =
                        fast_image_resize::Resizer::new(fast_image_resize::ResizeAlg::Nearest);
                    resizer
                        .resize(&src_image.view(), &mut dst_image.view_mut())
                        .ok()?;
                    RgbaImage::from_raw(new_width, new_height, dst_image.into_vec())?
                }
            }
        };
        let (w, h) = image.dimensions();
        if border_radius != 0 {
            let border_radius = border_radius.min(w / 2).min(h / 2);
            round(&mut image, border_radius);
        }
        let image = if w != rect.w || h != rect.h {
            let mut bg = RgbaImage::new(rect.w, rect.h);
            let x = match self.alignment {
                Alignment::Left => 0,
                Alignment::Center => rect.w.saturating_sub(w) / 2,
                Alignment::Right => rect.w.saturating_sub(w),
            };
            // vertical align top
            imageops::overlay(&mut bg, &image, x, 0);
            bg
        } else {
            image
        };

        Some(image)
    }
}

#[async_trait(?Send)]
impl View for Image {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        _styles: &Stylesheet,
    ) -> Result<bool> {
        if self.image.is_none() {
            if let Some(ref path) = self.path {
                self.image = self.image(path, self.rect, self.mode, self.border_radius);
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
