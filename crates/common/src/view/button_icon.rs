use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::Drawable;
use embedded_graphics::image::{Image, ImageRaw};
use embedded_graphics::prelude::{Dimensions, Size};
use embedded_graphics::primitives::{
    Circle, CornerRadii, CornerRadiiBuilder, Primitive, PrimitiveStyle, Rectangle, RoundedRectangle,
};
use embedded_graphics::text::{Text, TextStyleBuilder};
use image::RgbaImage;
use log::debug;
use tokio::sync::mpsc::Sender;

use crate::constants::ALLIUM_THEMES_DIR;
use crate::display::color::Color;
use crate::display::font::FontTextStyleBuilder;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::{Stylesheet, Theme};
use crate::view::{Command, View};

#[derive(Debug, Clone)]
struct ButtonIcons {
    images: HashMap<Key, RgbaImage>,
}

impl ButtonIcons {
    fn load() -> Self {
        let theme = Theme::load();
        let theme_dir = ALLIUM_THEMES_DIR.join(&theme.0);

        let resolve_icon_path = |icon_name: &str| -> PathBuf {
            let theme_icon = theme_dir.join("assets").join(icon_name);
            if theme_icon.exists() {
                return theme_icon;
            }
            // Fallback to default theme
            ALLIUM_THEMES_DIR
                .join("Allium")
                .join("assets")
                .join(icon_name)
        };

        let button_keys = [
            (Key::A, "button-a.png"),
            (Key::B, "button-b.png"),
            (Key::X, "button-x.png"),
            (Key::Y, "button-y.png"),
            (Key::Up, "button-up.png"),
            (Key::Down, "button-down.png"),
            (Key::Left, "button-left.png"),
            (Key::Right, "button-right.png"),
            (Key::Start, "button-start.png"),
            (Key::Select, "button-select.png"),
            (Key::L, "button-l.png"),
            (Key::R, "button-r.png"),
            (Key::L2, "button-l2.png"),
            (Key::R2, "button-r2.png"),
            (Key::Menu, "button-menu.png"),
            (Key::Power, "button-power.png"),
            (Key::VolDown, "button-voldown.png"),
            (Key::VolUp, "button-volup.png"),
            (Key::LidClose, "button-lid.png"),
        ];

        let mut images = HashMap::new();
        for (key, filename) in button_keys {
            let path = resolve_icon_path(filename);
            if !path.exists() {
                debug!(
                    "Button icon {} not found. Using vector rendering for this button.",
                    filename
                );
                continue;
            }
            match image::open(path) {
                Ok(img) => {
                    images.insert(key, img.to_rgba8());
                }
                Err(e) => {
                    debug!(
                        "Failed to load button icon {}: {}. Using vector rendering for this button.",
                        filename, e
                    );
                }
            }
        }

        ButtonIcons { images }
    }

    fn bounding_box(&self, styles: &Stylesheet, button: Key) -> Rect {
        if let Some(img) = self.images.get(&button) {
            Rect::new(0, 0, img.width(), img.height())
        } else {
            // Fall back to vector dimensions if image not found
            Self::vector_bounding_box(styles, button)
        }
    }

    fn vector_bounding_box(styles: &Stylesheet, button: Key) -> Rect {
        let text = Self::button_text(button);
        let diameter = styles.button_hint_font_size() as u32;

        let w = match button {
            Key::A
            | Key::B
            | Key::X
            | Key::Y
            | Key::L
            | Key::L2
            | Key::R
            | Key::R2
            | Key::Up
            | Key::Right
            | Key::Down
            | Key::Left => diameter,
            _ => {
                let text_style = FontTextStyleBuilder::new(styles.ui_font.font())
                    .font_fallback(styles.cjk_font.font())
                    .font_size(diameter * 3 / 4)
                    .text_color(styles.background_color)
                    .build();
                let text = Text::with_text_style(
                    text,
                    embedded_graphics::prelude::Point::zero(),
                    text_style,
                    TextStyleBuilder::new()
                        .alignment(Alignment::Center.into())
                        .build(),
                );
                text.bounding_box().size.width + 8
            }
        };

        Rect::new(0, 0, w, diameter + 4)
    }

    fn button_text(button: Key) -> &'static str {
        match button {
            Key::A => "A",
            Key::B => "B",
            Key::X => "X",
            Key::Y => "Y",
            Key::Up => "",
            Key::Down => "",
            Key::Left => "",
            Key::Right => "",
            Key::Start => "START",
            Key::Select => "SELECT",
            Key::L => "L",
            Key::R => "R",
            Key::Menu => "MENU",
            Key::L2 => "L2",
            Key::R2 => "R2",
            Key::Power => "POWER",
            Key::VolDown => "VOL-",
            Key::VolUp => "VOL+",
            Key::LidClose => "LID",
            Key::Unknown => unimplemented!("unknown button"),
        }
    }

    fn draw(
        &self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
        point: embedded_graphics::prelude::Point,
        button: Key,
    ) -> Result<()> {
        if let Some(img) = self.images.get(&button) {
            let raw_image: ImageRaw<'_, Color> = ImageRaw::new(img, img.width());
            let image = Image::new(&raw_image, point);
            image.draw(display)?;
        } else {
            // Fall back to vector drawing if image not found
            Self::draw_vector(display, styles, point, button)?;
        }
        Ok(())
    }

    fn draw_vector(
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
        point: embedded_graphics::prelude::Point,
        button: Key,
    ) -> Result<()> {
        let (color, text) = match button {
            Key::A => (styles.button_a_color, "A"),
            Key::B => (styles.button_b_color, "B"),
            Key::X => (styles.button_x_color, "X"),
            Key::Y => (styles.button_y_color, "Y"),
            Key::Up => (styles.button_bg_color, ""),
            Key::Down => (styles.button_bg_color, ""),
            Key::Left => (styles.button_bg_color, ""),
            Key::Right => (styles.button_bg_color, ""),
            Key::Start => (styles.button_bg_color, "START"),
            Key::Select => (styles.button_bg_color, "SELECT"),
            Key::L => (styles.button_bg_color, "L"),
            Key::R => (styles.button_bg_color, "R"),
            Key::Menu => (styles.button_bg_color, "MENU"),
            Key::L2 => (styles.button_bg_color, "L2"),
            Key::R2 => (styles.button_bg_color, "R2"),
            Key::Power => (styles.button_bg_color, "POWER"),
            Key::VolDown => (styles.button_bg_color, "VOL-"),
            Key::VolUp => (styles.button_bg_color, "VOL+"),
            Key::LidClose => (styles.button_bg_color, "LID"),
            Key::Unknown => unimplemented!("unknown button"),
        };

        let diameter = styles.button_hint_font_size() as u32;

        let text_style = FontTextStyleBuilder::new(styles.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size(diameter * 3 / 4)
            .text_color(styles.button_text_color)
            .build();
        let mut text = Text::with_text_style(
            text,
            embedded_graphics::prelude::Point::new(
                point.x + diameter as i32 / 2,
                point.y + diameter as i32 / 8,
            ),
            text_style,
            TextStyleBuilder::new()
                .alignment(Alignment::Center.into())
                .build(),
        );

        let mut draw_bg = false;
        let rect = match button {
            Key::A | Key::B | Key::X | Key::Y => {
                Circle::new(point, diameter)
                    .into_styled(PrimitiveStyle::with_fill(color))
                    .draw(display)?;
                Rect::new(point.x, point.y, diameter, diameter)
            }
            Key::Up | Key::Right | Key::Down | Key::Left => {
                RoundedRectangle::with_equal_corners(
                    Rectangle::new(
                        Point::new(point.x, point.y + diameter as i32 * 2 / 7 + 1).into(),
                        Size::new(diameter, diameter * 3 / 7),
                    ),
                    Size::new_equal(4),
                )
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(display)?;
                RoundedRectangle::with_equal_corners(
                    Rectangle::new(
                        Point::new(point.x + diameter as i32 * 2 / 7 + 1, point.y).into(),
                        Size::new(diameter * 3 / 7, diameter),
                    ),
                    Size::new_equal(4),
                )
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(display)?;
                match button {
                    Key::Up => RoundedRectangle::new(
                        Rectangle::new(
                            Point::new(
                                point.x + diameter as i32 * 5 / 14 + 1,
                                point.y + diameter as i32 / 14,
                            )
                            .into(),
                            Size::new(diameter * 2 / 7, diameter * 3 / 7),
                        ),
                        CornerRadii::new(Size::new_equal(4)),
                    ),
                    Key::Right => RoundedRectangle::new(
                        Rectangle::new(
                            Point::new(
                                point.x + diameter as i32 * 7 / 14 + 1,
                                point.y + diameter as i32 * 5 / 14 + 1,
                            )
                            .into(),
                            Size::new(diameter * 3 / 7, diameter * 2 / 7),
                        ),
                        CornerRadii::new(Size::new_equal(4)),
                    ),
                    Key::Down => RoundedRectangle::new(
                        Rectangle::new(
                            Point::new(
                                point.x + diameter as i32 * 5 / 14 + 1,
                                point.y + diameter as i32 * 7 / 14 + 1,
                            )
                            .into(),
                            Size::new(diameter * 2 / 7, diameter * 3 / 7),
                        ),
                        CornerRadii::new(Size::new_equal(4)),
                    ),
                    Key::Left => RoundedRectangle::new(
                        Rectangle::new(
                            Point::new(
                                point.x + diameter as i32 / 14,
                                point.y + diameter as i32 * 5 / 14 + 1,
                            )
                            .into(),
                            Size::new(diameter * 3 / 7, diameter * 2 / 7),
                        ),
                        CornerRadii::new(Size::new_equal(4)),
                    ),
                    _ => unreachable!(),
                }
                .into_styled(PrimitiveStyle::with_fill(styles.foreground_color))
                .draw(display)?;
                Rect::new(point.x, point.y, diameter, diameter)
            }
            Key::L | Key::L2 => {
                RoundedRectangle::new(
                    Rectangle::new(
                        Point::new(point.x, point.y + diameter as i32 / 8).into(),
                        Size::new(diameter, diameter * 3 / 4),
                    ),
                    CornerRadiiBuilder::new()
                        .all(Size::new_equal(8))
                        .top_left(Size::new_equal(16))
                        .build(),
                )
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(display)?;
                Rect::new(point.x, point.y, diameter, diameter)
            }
            Key::R | Key::R2 => {
                RoundedRectangle::new(
                    Rectangle::new(
                        Point::new(point.x, point.y + diameter as i32 / 8).into(),
                        Size::new(diameter, diameter * 3 / 4),
                    ),
                    CornerRadiiBuilder::new()
                        .all(Size::new_equal(8))
                        .top_right(Size::new_equal(16))
                        .build(),
                )
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(display)?;
                Rect::new(point.x, point.y, diameter, diameter)
            }
            _ => {
                draw_bg = true;
                text.position.x = point.x + 4;
                text.text_style.alignment = Alignment::Left.into();
                let rect = text.bounding_box();
                Rect::new(
                    rect.top_left.x - 4,
                    rect.top_left.y - 2,
                    rect.size.width + 8,
                    rect.size.height + 4,
                )
            }
        };

        if draw_bg {
            let fill_style = PrimitiveStyle::with_fill(color);
            RoundedRectangle::new(rect.into(), CornerRadii::new(Size::new_equal(8)))
                .into_styled(fill_style)
                .draw(display)?;
        }

        text.draw(display)?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ButtonIcon {
    point: Point,
    button: Key,
    alignment: Alignment,
    icons: ButtonIcons,
    dirty: bool,
}

impl ButtonIcon {
    pub fn new(point: Point, button: Key, alignment: Alignment) -> Self {
        Self {
            point,
            button,
            alignment,
            icons: ButtonIcons::load(),
            dirty: true,
        }
    }

    pub fn diameter(styles: &Stylesheet) -> u32 {
        styles.button_hint_font_size() as u32
    }
}

#[async_trait(?Send)]
impl View for ButtonIcon {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let diameter = Self::diameter(styles);

        let point = match self.alignment {
            Alignment::Left => self.point.into(),
            Alignment::Center => embedded_graphics::prelude::Point::new(
                self.point.x - (diameter / 2) as i32,
                self.point.y,
            ),
            Alignment::Right => {
                let width = self.bounding_box(styles).w;
                embedded_graphics::prelude::Point::new(self.point.x - width as i32, self.point.y)
            }
        };

        self.icons.draw(display, styles, point, self.button)?;

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
        let icon_size = self.icons.bounding_box(styles, self.button);
        let w = icon_size.w;

        let x = match self.alignment {
            Alignment::Left => self.point.x,
            Alignment::Center => self.point.x - (w / 2) as i32,
            Alignment::Right => self.point.x - w as i32,
        };

        Rect::new(x, self.point.y - 1, w, icon_size.h)
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.dirty = true;
    }
}
