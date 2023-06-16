use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::{Primitive, PrimitiveStyleBuilder, Rectangle, StrokeAlignment};
use embedded_graphics::Drawable;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::command::Value;
use crate::display::color::Color;
use crate::display::font::FontTextStyleBuilder;
use crate::display::Display;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::{Command, Label, View};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPicker {
    point: Point,
    value: Color,
    label: Label<String>,
    alignment: Alignment,
    dirty: bool,
    #[serde(skip)]
    edit_state: Option<EditState>,
}

#[derive(Debug, Clone)]
struct EditState {
    selected: usize,
    original: Color,
    value: Color,
}

impl ColorPicker {
    pub fn new(point: Point, value: Color, alignment: Alignment) -> Self {
        let label = Label::new(
            Point::new(point.x + (30 + 12) * alignment.sign(), point.y),
            format!("#{:X}", value),
            alignment,
            None,
        );

        Self {
            point,
            value,
            label,
            alignment,
            dirty: true,
            edit_state: None,
        }
    }

    pub fn set_value(&mut self, value: Color) {
        self.value = value;
        self.label.set_text(format!("#{:X}", value));
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
        let mut drawn = false;

        if self.dirty {
            let fill_style = PrimitiveStyleBuilder::new()
                .fill_color(
                    self.edit_state
                        .as_ref()
                        .map(|s| s.value)
                        .unwrap_or(self.value)
                        .into(),
                )
                .stroke_color(styles.foreground_color)
                .stroke_alignment(StrokeAlignment::Inside)
                .stroke_width(1)
                .build();

            Rectangle::new(
                Point::new(
                    self.point.x - (30 * (1 - self.alignment.sign()) / 2),
                    self.point.y,
                )
                .into(),
                Size::new_equal(30),
            )
            .into_styled(fill_style)
            .draw(display)?;
            drawn = true;

            if let Some(state) = self.edit_state.as_ref() {
                let text_style = FontTextStyleBuilder::new(styles.ui_font.clone())
                    .font_size(styles.ui_font_size)
                    .text_color(styles.foreground_color)
                    .background_color(styles.highlight_color)
                    .draw_background()
                    .build();

                let selected_style = FontTextStyleBuilder::new(styles.ui_font.clone())
                    .font_size(styles.ui_font_size)
                    .text_color(styles.foreground_color)
                    .background_color(styles.highlight_color)
                    .draw_background()
                    .underline()
                    .build();

                match self.alignment {
                    Alignment::Right => {
                        let mut x = self.point.x - 30 - 12;
                        for i in (0..6).rev() {
                            let rect = display.draw_text(
                                Point::new(x, self.point.y).into(),
                                &state.value.char(i),
                                if i == state.selected {
                                    selected_style.clone()
                                } else {
                                    text_style.clone()
                                },
                                Alignment::Right.into(),
                            )?;
                            x = rect.top_left.x - 1;
                        }
                        display.draw_text(
                            Point::new(x, self.point.y).into(),
                            "#",
                            text_style,
                            Alignment::Right.into(),
                        )?;
                    }
                    Alignment::Center => unimplemented!("alignment should be left or right"),
                    Alignment::Left => todo!(),
                }
            }
        } else {
            if self.label.should_draw() && self.label.draw(display, styles)? {
                drawn = true;
            }
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.dirty || self.label.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.label.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        _command: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
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
                    self.label.set_text(format!("#{:X}", self.value));
                    self.dirty = true;
                    self.edit_state = None;
                    bubble.push_back(Command::ValueChanged(0, Value::Color(self.value)));
                    bubble.push_back(Command::Unfocus);
                    Ok(true)
                }
                KeyEvent::Pressed(Key::B) => {
                    self.value = state.original;
                    self.label.set_text(format!("#{:X}", self.value));
                    self.dirty = true;
                    self.edit_state = None;
                    bubble.push_back(Command::ValueChanged(0, Value::Color(self.value)));
                    bubble.push_back(Command::Unfocus);
                    Ok(true)
                }
                _ => Ok(false),
            }
        } else {
            self.edit_state = Some(EditState {
                original: self.value,
                value: self.value,
                selected: 0,
            });
            bubble.push_back(Command::TrapFocus);
            Ok(true)
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.label]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.label]
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        self.label.bounding_box(styles).union(&Rect::new(
            self.point.x - (30 * (1 - self.alignment.sign()) / 2),
            self.point.y,
            30,
            30,
        ))
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.label.set_position(Point::new(
            point.x + (30 + 12) * self.alignment.sign(),
            point.y,
        ));
    }
}
