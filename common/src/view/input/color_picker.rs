use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::prelude::{Dimensions, Size};
use embedded_graphics::primitives::{Primitive, PrimitiveStyleBuilder, Rectangle, StrokeAlignment};
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use log::trace;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::command::Value;
use crate::display::color::Color;
use crate::display::font::{FontTextStyle, FontTextStyleBuilder};
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::{Stylesheet, StylesheetColor};
use crate::view::{Command, View};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPicker {
    point: Point,
    value: Color,
    alignment: Alignment,
    dirty: bool,
    #[serde(skip)]
    edit_state: Option<EditState>,
    background_color: StylesheetColor,
}

#[derive(Debug, Clone)]
struct EditState {
    selected: usize,
    value: Color,
}

impl ColorPicker {
    pub fn new(point: Point, value: Color, alignment: Alignment) -> Self {
        Self {
            point,
            value,
            alignment,
            dirty: true,
            edit_state: None,
            background_color: StylesheetColor::Background,
        }
    }

    pub fn set_value(&mut self, value: Color) {
        self.value = value;
        self.dirty = true;
    }

    pub fn value(&self) -> Color {
        self.value
    }
}

#[async_trait(?Send)]
impl View for ColorPicker {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if !self.dirty {
            return Ok(false);
        }

        let color = self
            .edit_state
            .as_ref()
            .map(|s| s.value)
            .unwrap_or(self.value);
        let edit_index = self.edit_state.as_ref().map(|s| s.selected);

        let fill_style = PrimitiveStyleBuilder::new()
            .fill_color(color)
            .stroke_color(styles.foreground_color)
            .stroke_alignment(StrokeAlignment::Inside)
            .stroke_width(1)
            .build();

        let w = styles.ui_font.size;
        Rectangle::new(
            Point::new(
                self.point.x - (w as i32 * (1 - self.alignment.sign()) / 2),
                self.point.y,
            )
            .into(),
            Size::new_equal(w),
        )
        .into_styled(fill_style)
        .draw(display)?;

        let text_style = FontTextStyleBuilder::new(styles.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size(styles.ui_font.size)
            .text_color(styles.foreground_color)
            .background_color(self.background_color.to_color(styles))
            .build();

        let focused_style = FontTextStyleBuilder::new(styles.ui_font.font())
            .font_size(styles.ui_font.size)
            .text_color(styles.foreground_color)
            .background_color(styles.highlight_color)
            .draw_background()
            .build();

        let selected_style = FontTextStyleBuilder::new(styles.ui_font.font())
            .font_size(styles.ui_font.size)
            .text_color(styles.foreground_color)
            .background_color(styles.highlight_color)
            .underline()
            .draw_background()
            .build();

        match self.alignment {
            Alignment::Right => {
                let mut x = self.point.x - w as i32 - 12;
                for i in (0..6).rev() {
                    let c = color.char(i);
                    let text = Text::with_alignment(
                        &c,
                        Point::new(x, self.point.y).into(),
                        if edit_index == Some(i) {
                            selected_style.clone()
                        } else if self.edit_state.is_some() {
                            focused_style.clone()
                        } else {
                            text_style.clone()
                        },
                        Alignment::Right.into(),
                    );
                    text.draw(display)?;
                    x = text.bounding_box().top_left.x - 1;
                }

                Text::with_alignment(
                    "#",
                    Point::new(x, self.point.y).into(),
                    if self.edit_state.is_some() {
                        focused_style
                    } else {
                        text_style
                    },
                    Alignment::Right.into(),
                )
                .draw(display)?;
            }
            Alignment::Center => unimplemented!("alignment should be left or right"),
            Alignment::Left => todo!(),
        }

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
        event: KeyEvent,
        _command: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        trace!(
            "color picker key event: {:?}, state: {:?}",
            event,
            self.edit_state
        );
        if let Some(state) = &mut self.edit_state {
            match event {
                KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                    state.value = match state.selected {
                        0 => state
                            .value
                            .with_r((state.value.r() as i32 + 16).rem_euclid(256) as u8),
                        1 => state.value.with_r(
                            (state.value.r() - state.value.r() % 16)
                                + (state.value.r() as i8 % 16 + 1).rem_euclid(16) as u8,
                        ),
                        2 => state
                            .value
                            .with_g((state.value.g() as i32 + 16).rem_euclid(256) as u8),
                        3 => state.value.with_g(
                            (state.value.g() - state.value.g() % 16)
                                + (state.value.g() as i8 % 16 + 1).rem_euclid(16) as u8,
                        ),
                        4 => state
                            .value
                            .with_b((state.value.b() as i32 + 16).rem_euclid(256) as u8),
                        5 => state.value.with_b(
                            (state.value.b() - state.value.b() % 16)
                                + (state.value.b() as i8 % 16 + 1).rem_euclid(16) as u8,
                        ),
                        _ => unreachable!(),
                    };
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                    state.value = match state.selected {
                        0 => state
                            .value
                            .with_r((state.value.r() as i32 - 16).rem_euclid(256) as u8),
                        1 => state.value.with_r(
                            (state.value.r() - state.value.r() % 16)
                                + (state.value.r() as i8 % 16 - 1).rem_euclid(16) as u8,
                        ),
                        2 => state
                            .value
                            .with_g((state.value.g() as i32 - 16).rem_euclid(256) as u8),
                        3 => state.value.with_g(
                            (state.value.g() - state.value.g() % 16)
                                + (state.value.g() as i8 % 16 - 1).rem_euclid(16) as u8,
                        ),
                        4 => state
                            .value
                            .with_b((state.value.b() as i32 - 16).rem_euclid(256) as u8),
                        5 => state.value.with_b(
                            (state.value.b() - state.value.b() % 16)
                                + (state.value.b() as i8 % 16 - 1).rem_euclid(16) as u8,
                        ),
                        _ => unreachable!(),
                    };
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                    state.selected = (state.selected as isize - 1).clamp(0, 5) as usize;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                    state.selected = (state.selected as isize + 1).clamp(0, 5) as usize;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::A) => {
                    self.value = state.value;
                    self.dirty = true;
                    self.edit_state = None;
                    bubble.push_back(Command::ValueChanged(0, Value::Color(self.value)));
                    bubble.push_back(Command::Unfocus);
                    Ok(true)
                }
                KeyEvent::Pressed(Key::B) => {
                    self.edit_state = None;
                    bubble.push_back(Command::Unfocus);
                    Ok(true)
                }
                _ => Ok(false),
            }
        } else {
            self.edit_state = Some(EditState {
                value: self.value,
                selected: 0,
            });
            bubble.push_back(Command::TrapFocus);
            Ok(true)
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![]
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        let text_style: FontTextStyle<Color> = FontTextStyleBuilder::new(styles.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size(styles.ui_font.size)
            .draw_background()
            .build();

        let mut x = self.point.x - 30 - 12;
        for i in (0..6).rev() {
            let c = self.value.char(i);
            let text = Text::with_alignment(
                &c,
                Point::new(x, self.point.y).into(),
                text_style.clone(),
                Alignment::Right.into(),
            );
            x = text.bounding_box().top_left.x - 1;
        }

        let rect: Rect = Text::with_alignment(
            "#",
            Point::new(x, self.point.y).into(),
            text_style,
            Alignment::Right.into(),
        )
        .bounding_box()
        .into();

        Rect::new(
            rect.x,
            self.point.y,
            (self.point.x - rect.x) as u32,
            rect.h + 1,
        )
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
    }

    fn set_background_color(&mut self, color: StylesheetColor) {
        self.background_color = color;
    }
}
