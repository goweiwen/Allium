use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::{
    prelude::{OriginDimensions, Size},
    primitives::{Primitive, PrimitiveStyleBuilder, Rectangle, RoundedRectangle},
    text::Text,
    Drawable,
};
use strum::{EnumIter, FromRepr, IntoEnumIterator};
use tokio::sync::mpsc::Sender;

use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::stylesheet::Stylesheet;
use crate::view::View;
use crate::{
    command::{Command, Value},
    view::ButtonHint,
};
use crate::{display::font::FontTextStyleBuilder, view::Row};

#[derive(Debug, Clone)]
pub struct Keyboard {
    value: String,
    cursor: rusttype::Point<usize>,
    mode: KeyboardMode,
    is_password: bool,
    button_hints: Row<ButtonHint<&'static str>>,
    dirty: bool,
}

impl Keyboard {
    pub fn new(value: String, is_password: bool) -> Self {
        let button_hints = Row::new(
            Point::new(640 - 12, 480 - 8 - 30),
            vec![
                ButtonHint::new(Point::zero(), Key::Start, "Confirm", Alignment::Right),
                ButtonHint::new(Point::zero(), Key::Select, "Shift", Alignment::Right),
                ButtonHint::new(Point::zero(), Key::R, "Backspace", Alignment::Right),
            ],
            Alignment::Right,
            12,
        );

        Self {
            value,
            cursor: rusttype::Point { x: 0, y: 0 },
            mode: KeyboardMode::Lowercase,
            is_password,
            button_hints,
            dirty: true,
        }
    }
}

#[async_trait(?Send)]
impl View for Keyboard {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let mut drawn = false;
        if self.dirty {
            let text_style = FontTextStyleBuilder::new(styles.ui_font.clone())
                .font_size(styles.ui_font_size)
                .text_color(styles.foreground_color)
                .background_color(styles.background_color)
                .build();

            let selected_text_style = FontTextStyleBuilder::new(styles.ui_font.clone())
                .font_size(styles.ui_font_size)
                .text_color(styles.foreground_color)
                .background_color(styles.highlight_color)
                .build();

            let fill_style = PrimitiveStyleBuilder::new()
                .fill_color(styles.background_color)
                .stroke_width(1)
                .build();

            let selected_btn_style = PrimitiveStyleBuilder::new()
                .fill_color(styles.highlight_color)
                .stroke_width(1)
                .build();

            let key_size = 32_u32;
            let key_padding = 4;

            let w = key_size as i32 * KEYBOARD_COLUMNS + key_padding * 14;
            let h = key_size as i32 * KEYBOARD_ROWS + key_padding * 5;
            let x0 = (display.size().width as i32 - w) / 2;
            let y0 = display.size().height as i32 - h - 48;

            Rectangle::new(
                Point::new(0, y0 - styles.ui_font_size as i32 - 8).into(),
                Size::new(display.size().width, h as u32 + styles.ui_font_size + 8),
            )
            .into_styled(fill_style)
            .draw(display)?;

            for (i, key) in KeyboardKey::iter().enumerate() {
                let i = i as i32;
                let x = i % KEYBOARD_COLUMNS * w / KEYBOARD_COLUMNS;
                let y = i / KEYBOARD_COLUMNS * h / KEYBOARD_ROWS;

                let selected =
                    self.cursor.x + self.cursor.y * KEYBOARD_COLUMNS as usize == i as usize;

                if selected {
                    RoundedRectangle::with_equal_corners(
                        Rect::new(x0 + x, y0 + y, key_size, key_size).into(),
                        Size::new(12, 12),
                    )
                    .into_styled(selected_btn_style)
                    .draw(display)?;
                }

                Text::with_alignment(
                    key.key(self.mode),
                    Point::new(
                        x0 + x + key_size as i32 / 2,
                        y0 + y + key_size as i32 / 2 - styles.ui_font_size as i32 / 2,
                    )
                    .into(),
                    if selected {
                        selected_text_style.clone()
                    } else {
                        text_style.clone()
                    },
                    Alignment::Center.into(),
                )
                .draw(display)?;
            }

            Text::with_alignment(
                &masked_value(&self.value, self.is_password),
                Point::new(
                    display.size().width as i32 / 2,
                    display.size().height as i32 - h - 48 - styles.ui_font_size as i32 - 8,
                )
                .into(),
                text_style,
                Alignment::Center.into(),
            )
            .draw(display)?;

            drawn = true;
        }

        drawn |= self.button_hints.should_draw() && self.button_hints.draw(display, styles)?;

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.dirty || self.button_hints.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        self.button_hints.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        match event {
            KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                self.cursor.y = (self.cursor.y as i32 - 1).rem_euclid(KEYBOARD_ROWS) as usize;
            }
            KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                self.cursor.y = (self.cursor.y + 1).rem_euclid(KEYBOARD_ROWS as usize);
            }
            KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                self.cursor.x = (self.cursor.x as i32 - 1).rem_euclid(KEYBOARD_COLUMNS) as usize;
            }
            KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                self.cursor.x = (self.cursor.x + 1).rem_euclid(KEYBOARD_COLUMNS as usize);
            }
            KeyEvent::Pressed(Key::A) => {
                self.value += KeyboardKey::from_repr(
                    self.cursor.x + self.cursor.y * KEYBOARD_COLUMNS as usize,
                )
                .unwrap()
                .key(self.mode)
            }
            KeyEvent::Pressed(Key::R) => {
                self.value.pop();
            }
            KeyEvent::Pressed(Key::B) => {
                bubble.push_back(Command::CloseView);
                commands.send(Command::Redraw).await?;
            }
            KeyEvent::Pressed(Key::Select) => {
                self.mode = match self.mode {
                    KeyboardMode::Lowercase => KeyboardMode::Uppercase,
                    KeyboardMode::Uppercase => KeyboardMode::Symbols,
                    KeyboardMode::Symbols => KeyboardMode::Lowercase,
                }
            }
            KeyEvent::Pressed(Key::Start) => {
                bubble.push_back(Command::ValueChanged(0, Value::String(self.value.clone())));
                bubble.push_back(Command::CloseView);
                commands.send(Command::Redraw).await?;
                return Ok(true);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        let key_size = 32_u32;
        let key_padding = 4;

        let w = key_size * KEYBOARD_COLUMNS as u32 + key_padding * 14;
        let h = key_size * KEYBOARD_ROWS as u32 + key_padding * 5;
        let x = (640 - w as i32) / 2;
        let y = 480_i32 - h as i32;

        Rect::new(x, y, w, h)
    }

    fn set_position(&mut self, _point: crate::geom::Point) {}
}

#[rustfmt::skip]
#[derive(Debug, EnumIter, FromRepr)]
enum KeyboardKey {
    K1, K2, K3, K4, K5, K6, K7, K8,    K9,     K0,           Minus,
    Q,  W,  E,  R,  T,  Y,  U,  I,     O,      P,            Backslash,
    A,  S,  D,  F,  G,  H,  J,  K,     L,      Semicolon,    Quote,
    Z,  X,  C,  V,  B,  N,  M,  Comma, Period, QuestionMark, ExclamationMark
}

const KEYBOARD_COLUMNS: i32 = 11;
const KEYBOARD_ROWS: i32 = 4;

impl KeyboardKey {
    fn lowercase(&self) -> &str {
        use KeyboardKey::*;
        match self {
            K1 => "1",
            K2 => "2",
            K3 => "3",
            K4 => "4",
            K5 => "5",
            K6 => "6",
            K7 => "7",
            K8 => "8",
            K9 => "9",
            K0 => "0",
            Minus => "-",
            Q => "q",
            W => "w",
            E => "e",
            R => "r",
            T => "t",
            Y => "y",
            U => "u",
            I => "i",
            O => "o",
            P => "p",
            Backslash => "\\",
            A => "a",
            S => "s",
            D => "d",
            F => "f",
            G => "g",
            H => "h",
            J => "j",
            K => "k",
            L => "l",
            Semicolon => ";",
            Quote => "'",
            Z => "z",
            X => "x",
            C => "c",
            V => "v",
            B => "b",
            N => "n",
            M => "m",
            Comma => ",",
            Period => ".",
            QuestionMark => "?",
            ExclamationMark => "!",
        }
    }

    fn uppercase(&self) -> &str {
        use KeyboardKey::*;
        match self {
            K1 => "#",
            K2 => "[",
            K3 => "]",
            K4 => "$",
            K5 => "%",
            K6 => "^",
            K7 => "&",
            K8 => "*",
            K9 => "(",
            K0 => ")",
            Minus => "_",
            Q => "Q",
            W => "W",
            E => "E",
            R => "R",
            T => "T",
            Y => "Y",
            U => "U",
            I => "I",
            O => "O",
            P => "P",
            Backslash => "@",
            A => "A",
            S => "S",
            D => "D",
            F => "F",
            G => "G",
            H => "H",
            J => "J",
            K => "K",
            L => "L",
            Semicolon => ":",
            Quote => "\"",
            Z => "Z",
            X => "X",
            C => "C",
            V => "V",
            B => "B",
            N => "N",
            M => "M",
            Comma => "<",
            Period => ">",
            QuestionMark => "+",
            ExclamationMark => "=",
        }
    }

    fn symbol(&self) -> &str {
        use KeyboardKey::*;
        match self {
            K1 => "1",
            K2 => "2",
            K3 => "3",
            K4 => "4",
            K5 => "5",
            K6 => "6",
            K7 => "7",
            K8 => "8",
            K9 => "9",
            K0 => "0",
            Minus => "-",
            Q => "!",
            W => "@",
            E => "#",
            R => "$",
            T => "%",
            Y => "^",
            U => "&",
            I => "*",
            O => "(",
            P => ")",
            Backslash => "_",
            A => "~",
            S => "`",
            D => "=",
            F => "\\",
            G => "+",
            H => "{",
            J => "}",
            K => "|",
            L => "[",
            Semicolon => "]",
            Quote => " ",
            Z => "<",
            X => ">",
            C => ";",
            V => ":",
            B => "\"",
            N => "'",
            M => ",",
            Comma => ".",
            Period => "?",
            QuestionMark => "/",
            ExclamationMark => "~",
        }
    }

    fn key(&self, mode: KeyboardMode) -> &str {
        match mode {
            KeyboardMode::Lowercase => self.lowercase(),
            KeyboardMode::Uppercase => self.uppercase(),
            KeyboardMode::Symbols => self.symbol(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum KeyboardMode {
    Lowercase,
    Uppercase,
    Symbols,
}

fn masked_value(value: &str, is_password: bool) -> String {
    if is_password {
        "*".repeat(value.len())
    } else {
        value.to_owned()
    }
}
