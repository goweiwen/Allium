use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use embedded_graphics::{
    Drawable,
    prelude::{Dimensions, OriginDimensions, Size},
    primitives::{Primitive, PrimitiveStyleBuilder, Rectangle, RoundedRectangle},
    text::Text,
};
use strum::{EnumCount, EnumIter, FromRepr, IntoEnumIterator};
use tokio::sync::mpsc::Sender;

use crate::command::{Command, Value};
use crate::display::{Display, font::FontTextStyleBuilder};
use crate::geom::{self, Alignment, Point, Rect};
use crate::locale::Locale;
use crate::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use crate::resources::Resources;
use crate::stylesheet::Stylesheet;
use crate::view::{ButtonHint, ButtonHints, ButtonIcon, View};

#[derive(Debug, Clone)]
pub struct Keyboard {
    res: Resources,
    value: String,
    cursor: rusttype::Point<usize>,
    mode: KeyboardMode,
    is_password: bool,
    button_hints: ButtonHints<String>,
    dirty: bool,
}

impl Keyboard {
    pub fn new(res: Resources, value: String, is_password: bool) -> Self {
        let locale = res.get::<Locale>();
        let styles = res.get::<Stylesheet>();

        let button_hints = ButtonHints::new(
            res.clone(),
            vec![],
            vec![
                ButtonHint::new(
                    res.clone(),
                    Point::zero(),
                    Key::Start,
                    locale.t("button-confirm"),
                    Alignment::Right,
                ),
                ButtonHint::new(
                    res.clone(),
                    Point::zero(),
                    Key::B,
                    locale.t("button-back"),
                    Alignment::Right,
                ),
                ButtonHint::new(
                    res.clone(),
                    Point::zero(),
                    Key::R,
                    locale.t("keyboard-button-backspace"),
                    Alignment::Right,
                ),
            ],
        );

        drop(locale);
        drop(styles);

        Self {
            res,
            value,
            cursor: rusttype::Point { x: 5, y: 2 },
            mode: KeyboardMode::Lowercase,
            is_password,
            button_hints,
            dirty: true,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
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
            display.load(self.bounding_box(styles))?;
            let text_style = FontTextStyleBuilder::new(styles.ui_font.font())
                .font_fallback(styles.cjk_font.font())
                .font_size(styles.ui_font.size)
                .text_color(styles.foreground_color)
                .stroke_color(styles.stroke_color)
                .stroke_width(styles.stroke_width)
                .background_color(styles.background_color)
                .build();

            let selected_text_style = FontTextStyleBuilder::new(styles.ui_font.font())
                .font_fallback(styles.cjk_font.font())
                .font_size(styles.ui_font.size)
                .text_color(styles.highlight_text_color)
                .stroke_color(styles.highlight_text_stroke_color)
                .stroke_width(styles.stroke_width)
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

            let key_size = styles.ui_font.size;
            let key_padding = 0;

            let w = key_size as i32 * KEYBOARD_COLUMNS + key_padding * 14;
            let h = key_size as i32 * KEYBOARD_ROWS + key_padding * 5;
            let x0 = (display.size().width as i32 - w) / 2;
            let y0 = display.size().height as i32
                - h
                - ButtonIcon::diameter(styles) as i32
                - styles.margin_y
                - styles.margin_y;

            RoundedRectangle::with_equal_corners(
                Rectangle::new(
                    Point::new(8, y0 - styles.ui_font.size as i32 - styles.margin_y).into(),
                    Size::new(
                        display.size().width - 16,
                        h as u32 + styles.ui_font.size + 8,
                    ),
                ),
                Size::new_equal(8),
            )
            .into_styled(fill_style)
            .draw(display)?;

            for (i, key) in KeyboardKey::iter().enumerate().take(KeyboardKey::COUNT - 1) {
                let i = i as i32;
                let x = i % KEYBOARD_COLUMNS * w / KEYBOARD_COLUMNS;
                let y = i / KEYBOARD_COLUMNS * h / KEYBOARD_ROWS;

                let selected =
                    self.cursor.x + self.cursor.y * KEYBOARD_COLUMNS as usize == i as usize;
                if self.cursor.y < 4 && selected {
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
                        y0 + y + key_size as i32 / 2 - styles.ui_font.size as i32 / 2,
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

            // Spacebar
            {
                let y = 4 * h / KEYBOARD_ROWS;
                let selected = self.cursor.y == 4;
                if selected {
                    RoundedRectangle::with_equal_corners(
                        Rect::new(x0, y0 + y, w as u32, key_size).into(),
                        Size::new(12, 12),
                    )
                    .into_styled(selected_btn_style)
                    .draw(display)?;
                }

                Text::with_alignment(
                    "space",
                    Point::new(
                        x0 + w / 2,
                        y0 + y + key_size as i32 / 2 - styles.ui_font.size as i32 / 2,
                    )
                    .into(),
                    if selected {
                        selected_text_style
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
                    y0 - styles.margin_y - styles.ui_font.size as i32,
                )
                .into(),
                text_style,
                Alignment::Center.into(),
            )
            .draw(display)?;

            self.dirty = false;
            drawn = true;
        }

        if self.button_hints.should_draw() {
            display.load(Rect::new(
                display.bounding_box().top_left.x,
                display.bounding_box().top_left.y + display.bounding_box().size.height as i32 - 48,
                display.bounding_box().size.width,
                48,
            ))?;

            drawn |= self.button_hints.draw(display, styles)?;
        }

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
                self.dirty = true;
            }
            KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                self.cursor.y = (self.cursor.y + 1).rem_euclid(KEYBOARD_ROWS as usize);
                self.dirty = true;
            }
            KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                self.cursor.x = (self.cursor.x as i32 - 1).rem_euclid(KEYBOARD_COLUMNS) as usize;
                self.dirty = true;
            }
            KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                self.cursor.x = (self.cursor.x + 1).rem_euclid(KEYBOARD_COLUMNS as usize);
                self.dirty = true;
            }
            KeyEvent::Pressed(Key::A) => {
                if self.cursor.y == 4 {
                    self.value.push(' ');
                } else {
                    self.value += KeyboardKey::from_repr(
                        self.cursor.x + self.cursor.y * KEYBOARD_COLUMNS as usize,
                    )
                    .unwrap()
                    .key(self.mode)
                }
                self.dirty = true;
            }
            KeyEvent::Pressed(Key::R) | KeyEvent::Pressed(Key::L) => {
                self.value.pop();
                self.dirty = true;
            }
            KeyEvent::Pressed(Key::B) => {
                bubble.push_back(Command::CloseView);
                commands.send(Command::Redraw).await?;
            }
            KeyEvent::Pressed(Key::X) => {
                self.value.clear();
                self.dirty = true;
            }
            KeyEvent::Pressed(Key::Select) => {
                self.mode = match self.mode {
                    KeyboardMode::Lowercase => KeyboardMode::Uppercase,
                    KeyboardMode::Uppercase => KeyboardMode::Symbols,
                    KeyboardMode::Symbols => KeyboardMode::Lowercase,
                };
                self.dirty = true;
            }
            KeyEvent::Pressed(Key::Start) => {
                bubble.push_back(Command::ValueChanged(0, Value::String(self.value.clone())));
                bubble.push_back(Command::CloseView);
                commands.send(Command::Redraw).await?;
                return Ok(true);
            }
            _ => return Ok(true),
        }
        Ok(true)
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![]
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        let size = self.res.get::<geom::Size>();

        let key_size = styles.ui_font.size;
        let key_padding = 0;
        let h = key_size as i32 * KEYBOARD_ROWS + key_padding * 5 + 8;
        let y = size.h as i32
            - h
            - ButtonIcon::diameter(styles) as i32
            - styles.margin_x
            - styles.margin_x;

        Rect::new(
            0,
            y - styles.ui_font.size as i32 - styles.margin_x,
            size.w,
            size.h - y as u32,
        )
    }

    fn set_position(&mut self, _point: crate::geom::Point) {}

    fn focus(&mut self) {}

    fn blur(&mut self) {}
}

#[rustfmt::skip]
#[derive(Debug, EnumIter, EnumCount, FromRepr)]
enum KeyboardKey {
    K1, K2, K3, K4, K5, K6, K7, K8,    K9,     K0,           Minus,
    Q,  W,  E,  R,  T,  Y,  U,  I,     O,      P,            Backslash,
    A,  S,  D,  F,  G,  H,  J,  K,     L,      Semicolon,    Quote,
    Z,  X,  C,  V,  B,  N,  M,  Comma, Period, QuestionMark, ExclamationMark,
    Space,
}

const KEYBOARD_COLUMNS: i32 = 11;
const KEYBOARD_ROWS: i32 = 5;

impl KeyboardKey {
    fn lowercase(&self) -> &str {
        #[allow(clippy::enum_glob_use)]
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
            Space => " ",
        }
    }

    fn uppercase(&self) -> &str {
        #[allow(clippy::enum_glob_use)]
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
            Space => " ",
        }
    }

    fn symbol(&self) -> &str {
        #[allow(clippy::enum_glob_use)]
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
            Space => " ",
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
