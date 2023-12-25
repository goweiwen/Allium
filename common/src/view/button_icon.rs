use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::prelude::{Dimensions, Size};
use embedded_graphics::primitives::{
    Circle, CornerRadii, CornerRadiiBuilder, Primitive, PrimitiveStyle, Rectangle, RoundedRectangle,
};
use embedded_graphics::text::{Text, TextStyleBuilder};
use embedded_graphics::Drawable;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::display::font::FontTextStyleBuilder;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::{Command, View};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ButtonIcon {
    point: Point,
    button: Key,
    alignment: Alignment,
    dirty: bool,
}

impl ButtonIcon {
    pub fn new(point: Point, button: Key, alignment: Alignment) -> Self {
        Self {
            point,
            button,
            alignment,
            dirty: true,
        }
    }

    pub fn diameter(styles: &Stylesheet) -> u32 {
        styles.ui_font.size
    }
}

#[async_trait(?Send)]
impl View for ButtonIcon {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let (color, text) = match self.button {
            Key::A => (styles.button_a_color, "A"),
            Key::B => (styles.button_b_color, "B"),
            Key::X => (styles.button_x_color, "X"),
            Key::Y => (styles.button_y_color, "Y"),
            Key::Up => (styles.disabled_color, ""),
            Key::Down => (styles.disabled_color, ""),
            Key::Left => (styles.disabled_color, ""),
            Key::Right => (styles.disabled_color, ""),
            Key::Start => (styles.disabled_color, "START"),
            Key::Select => (styles.disabled_color, "SELECT"),
            Key::L => (styles.disabled_color, "L"),
            Key::R => (styles.disabled_color, "R"),
            Key::Menu => (styles.disabled_color, "MENU"),
            Key::L2 => (styles.disabled_color, "L2"),
            Key::R2 => (styles.disabled_color, "R2"),
            Key::Power => (styles.disabled_color, "POWER"),
            Key::VolDown => (styles.disabled_color, "VOL-"),
            Key::VolUp => (styles.disabled_color, "VOL+"),
            Key::Unknown => unimplemented!("unknown button"),
        };

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

        let text_style = FontTextStyleBuilder::new(styles.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size(diameter * 3 / 4)
            .text_color(styles.foreground_color)
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
        let rect = match self.button {
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
                match self.button {
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
        let text = match self.button {
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
            Key::Unknown => unimplemented!("unknown button"),
        };

        let w = match self.button {
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
            | Key::Left => Self::diameter(styles),
            _ => {
                let text_style = FontTextStyleBuilder::new(styles.ui_font.font())
                    .font_fallback(styles.cjk_font.font())
                    .font_size(Self::diameter(styles) * 3 / 4)
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

        let x = match self.alignment {
            Alignment::Left => self.point.x,
            Alignment::Center => self.point.x - (w / 2) as i32,
            Alignment::Right => self.point.x - w as i32,
        };

        Rect::new(x, self.point.y - 1, w, Self::diameter(styles) + 4)
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.dirty = true;
    }
}
