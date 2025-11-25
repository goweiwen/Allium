use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use log::trace;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::command::Value;
use crate::display::Display;
use crate::display::color::Color;
use crate::display::font::FontTextStyleBuilder;
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
    text_color: StylesheetColor,
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
            text_color: StylesheetColor::Foreground,
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
        self.dirty = false;

        let color = self
            .edit_state
            .as_ref()
            .map(|s| s.value)
            .unwrap_or(self.value);
        let edit_index = self.edit_state.as_ref().map(|s| s.selected);

        let w = styles.ui.ui_font.size;
        let color_box_rect = Rect::new(
            self.point.x - (w as i32 * (1 - self.alignment.sign()) / 2),
            self.point.y,
            w,
            w,
        );

        // Draw color preview box with border
        let mut pixmap = display.pixmap_mut();
        crate::display::fill_rect(&mut pixmap, color_box_rect, color);

        // Draw 1px border (simplified - not using stroke_path for performance)
        let border_color = styles.ui.text_color;
        for x in color_box_rect.x..(color_box_rect.x + color_box_rect.w as i32) {
            if x >= 0 && x < pixmap.width() as i32 {
                // Top border
                if color_box_rect.y >= 0 && color_box_rect.y < pixmap.height() as i32 {
                    let idx = (color_box_rect.y * pixmap.width() as i32 + x) as usize;
                    pixmap.pixels_mut()[idx] = border_color.into();
                }
                // Bottom border
                let bottom_y = color_box_rect.y + color_box_rect.h as i32 - 1;
                if bottom_y >= 0 && bottom_y < pixmap.height() as i32 {
                    let idx = (bottom_y * pixmap.width() as i32 + x) as usize;
                    pixmap.pixels_mut()[idx] = border_color.into();
                }
            }
        }
        for y in color_box_rect.y..(color_box_rect.y + color_box_rect.h as i32) {
            if y >= 0 && y < pixmap.height() as i32 {
                // Left border
                if color_box_rect.x >= 0 && color_box_rect.x < pixmap.width() as i32 {
                    let idx = (y * pixmap.width() as i32 + color_box_rect.x) as usize;
                    pixmap.pixels_mut()[idx] = border_color.into();
                }
                // Right border
                let right_x = color_box_rect.x + color_box_rect.w as i32 - 1;
                if right_x >= 0 && right_x < pixmap.width() as i32 {
                    let idx = (y * pixmap.width() as i32 + right_x) as usize;
                    pixmap.pixels_mut()[idx] = border_color.into();
                }
            }
        }

        let text_color = self.text_color.to_color(styles);
        let stroke_color = if self.text_color == crate::stylesheet::StylesheetColor::HighlightText {
            styles.ui.highlight_text_stroke_color
        } else {
            styles.ui.text_stroke_color
        };

        let text_style = FontTextStyleBuilder::new(styles.ui.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size(styles.ui.ui_font.size)
            .text_color(text_color)
            .stroke_color(stroke_color)
            .stroke_width(styles.ui.stroke_width)
            .build();

        let focused_style = FontTextStyleBuilder::new(styles.ui.ui_font.font())
            .font_size(styles.ui.ui_font.size)
            .text_color(text_color)
            .stroke_color(stroke_color)
            .stroke_width(styles.ui.stroke_width)
            .draw_background()
            .build();

        let selected_style = FontTextStyleBuilder::new(styles.ui.ui_font.font())
            .font_size(styles.ui.ui_font.size)
            .text_color(text_color)
            .stroke_color(stroke_color)
            .stroke_width(styles.ui.stroke_width)
            .underline()
            .draw_background()
            .build();

        match self.alignment {
            Alignment::Right => {
                let mut x = self.point.x - w as i32 - styles.ui.margin_y;

                // Draw each hex digit
                for i in (0..8).rev() {
                    let c = color.char(i);
                    let char_style = if edit_index == Some(i) {
                        &selected_style
                    } else if self.edit_state.is_some() {
                        &focused_style
                    } else {
                        &text_style
                    };

                    let char_size = char_style.measure(&c);
                    let char_pos = Point::new(x - char_size.w as i32, self.point.y);
                    char_style.draw(&mut display.pixmap_mut(), &c, char_pos);

                    x = char_pos.x - 1;
                }

                // Draw "#" prefix
                let hash_style = if self.edit_state.is_some() {
                    &focused_style
                } else {
                    &text_style
                };
                let hash_text = "#";
                let hash_size = hash_style.measure(hash_text);
                let hash_pos = Point::new(x - hash_size.w as i32, self.point.y);
                hash_style.draw(&mut display.pixmap_mut(), hash_text, hash_pos);
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
            event, self.edit_state
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
                        6 => state
                            .value
                            .with_a((state.value.a() as i32 + 16).rem_euclid(256) as u8),
                        7 => state.value.with_a(
                            (state.value.a() - state.value.a() % 16)
                                + (state.value.a() as i8 % 16 + 1).rem_euclid(16) as u8,
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
                        6 => state
                            .value
                            .with_a((state.value.a() as i32 - 16).rem_euclid(256) as u8),
                        7 => state.value.with_a(
                            (state.value.a() - state.value.a() % 16)
                                + (state.value.a() as i8 % 16 - 1).rem_euclid(16) as u8,
                        ),
                        _ => unreachable!(),
                    };
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                    state.selected = (state.selected as isize - 1).clamp(0, 7) as usize;
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                    state.selected = (state.selected as isize + 1).clamp(0, 7) as usize;
                    self.dirty = true;
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
        let text_style = FontTextStyleBuilder::new(styles.ui.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size(styles.ui.ui_font.size)
            .draw_background()
            .build();

        // Measure the full color hex string to calculate bounding box
        let mut full_text = String::with_capacity(9);
        for i in (0..8).rev() {
            full_text.push_str(&self.value.char(i));
        }
        full_text.insert(0, '#');
        let full_size = text_style.measure(&full_text);

        Rect::new(
            self.point.x - full_size.w as i32 - 30 - styles.ui.margin_y,
            self.point.y,
            full_size.w + 30 + styles.ui.margin_y as u32,
            full_size.h + 1,
        )
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
    }

    fn focus(&mut self) {
        self.text_color = StylesheetColor::HighlightText;
        self.dirty = true;
    }

    fn blur(&mut self) {
        self.text_color = StylesheetColor::Foreground;
        self.dirty = true;
    }
}
