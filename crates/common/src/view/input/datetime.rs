use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{Days, Duration, Months, NaiveDateTime};
use log::trace;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::command::Value;
use crate::display::Display;
use crate::display::font::FontTextStyleBuilder;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::{Stylesheet, StylesheetColor};
use crate::view::{Command, View};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateTime {
    point: Point,
    value: NaiveDateTime,
    alignment: Alignment,
    dirty: bool,
    #[serde(skip)]
    edit_state: Option<EditState>,
    text_color: StylesheetColor,
}

#[derive(Debug, Clone)]
struct EditState {
    selected: usize,
    value: NaiveDateTime,
}

impl DateTime {
    pub fn new(point: Point, value: NaiveDateTime, alignment: Alignment) -> Self {
        Self {
            point,
            value,
            alignment,
            dirty: true,
            edit_state: None,
            text_color: StylesheetColor::Foreground,
        }
    }

    pub fn set_value(&mut self, value: NaiveDateTime) {
        self.value = value;
        self.dirty = true;
    }

    pub fn value(&self) -> NaiveDateTime {
        self.value
    }
}

#[async_trait(?Send)]
impl View for DateTime {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if !self.dirty {
            return Ok(false);
        }
        self.dirty = false;

        let datetime = self
            .edit_state
            .as_ref()
            .map(|s| s.value)
            .unwrap_or(self.value);
        let edit_index = self.edit_state.as_ref().map(|s| s.selected);

        let text_color = self.text_color.to_color(styles);
        let stroke_color = if self.text_color == StylesheetColor::HighlightText {
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

        let year = datetime.format("%Y").to_string();
        let month = datetime.format("%m").to_string();
        let day = datetime.format("%d").to_string();
        let hour = datetime.format("%H").to_string();
        let minute = datetime.format("%M").to_string();
        let second = datetime.format("%S").to_string();
        let fields = [
            &year, "-", &month, "-", &day, " ", &hour, ":", &minute, ":", &second,
        ];
        let mut x = self.point.x;
        match self.alignment {
            Alignment::Right => {
                for (i, field) in fields.iter().enumerate().rev() {
                    let field_style = if edit_index == Some(i) {
                        &selected_style
                    } else if self.edit_state.is_some() {
                        &focused_style
                    } else {
                        &text_style
                    };

                    let field_size = field_style.measure(field);
                    let field_pos = Point::new(x - field_size.w as i32, self.point.y);
                    field_style.draw(&mut display.pixmap_mut(), field, field_pos);

                    x = field_pos.x - 1;
                }
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
            "datetime key event: {:?}, state: {:?}",
            event, self.edit_state
        );
        if let Some(state) = &mut self.edit_state {
            match event {
                KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                    state.value = match state.selected {
                        0 => state
                            .value
                            .checked_add_months(Months::new(12))
                            .unwrap_or(state.value),
                        2 => state
                            .value
                            .checked_add_months(Months::new(1))
                            .unwrap_or(state.value),
                        4 => state
                            .value
                            .checked_add_days(Days::new(1))
                            .unwrap_or(state.value),
                        6 => state
                            .value
                            .checked_add_signed(Duration::hours(1))
                            .unwrap_or(state.value),
                        8 => state
                            .value
                            .checked_add_signed(Duration::minutes(1))
                            .unwrap_or(state.value),
                        10 => state
                            .value
                            .checked_add_signed(Duration::seconds(1))
                            .unwrap_or(state.value),
                        _ => unreachable!(),
                    };
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                    state.value = match state.selected {
                        0 => state
                            .value
                            .checked_sub_months(Months::new(12))
                            .unwrap_or(state.value),
                        2 => state
                            .value
                            .checked_sub_months(Months::new(1))
                            .unwrap_or(state.value),
                        4 => state
                            .value
                            .checked_sub_days(Days::new(1))
                            .unwrap_or(state.value),
                        6 => state
                            .value
                            .checked_sub_signed(Duration::hours(1))
                            .unwrap_or(state.value),
                        8 => state
                            .value
                            .checked_sub_signed(Duration::minutes(1))
                            .unwrap_or(state.value),
                        10 => state
                            .value
                            .checked_sub_signed(Duration::seconds(1))
                            .unwrap_or(state.value),
                        _ => unreachable!(),
                    };
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                    state.selected = (state.selected as isize - 1).clamp(0, 10) as usize;
                    if state.selected == 1
                        || state.selected == 3
                        || state.selected == 5
                        || state.selected == 7
                        || state.selected == 9
                    {
                        state.selected -= 1;
                    }
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                    state.selected = (state.selected as isize + 1).clamp(0, 10) as usize;
                    if state.selected == 1
                        || state.selected == 3
                        || state.selected == 5
                        || state.selected == 7
                        || state.selected == 9
                    {
                        state.selected += 1;
                    }
                    self.dirty = true;
                    Ok(true)
                }
                KeyEvent::Pressed(Key::A) => {
                    self.value = state.value;
                    self.dirty = true;
                    self.edit_state = None;
                    bubble.push_back(Command::ValueChanged(0, Value::DateTime(self.value)));
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
                selected: 6,
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

        // Measure the full datetime string to calculate bounding box
        let datetime_str = self.value.format("%Y-%m-%d %H:%M:%S").to_string();
        let full_size = text_style.measure(&datetime_str);

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
