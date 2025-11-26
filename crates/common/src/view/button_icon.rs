use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use image::RgbaImage;
use log::debug;
use tokio::sync::mpsc::Sender;

use crate::constants::ALLIUM_THEMES_DIR;
use crate::display::Display;
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
        let diameter = styles.button_size() as u32;

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
                let text_style = FontTextStyleBuilder::new(styles.ui.ui_font.font())
                    .font_fallback(styles.cjk_font.font())
                    .font_size(styles.button_text_font_size() as u32)
                    .text_color(styles.ui.background_color)
                    .build();
                let text_size = text_style.measure(text);
                text_size.w + 8
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
        point: Point,
        button: Key,
    ) -> Result<()> {
        if let Some(img) = self.images.get(&button) {
            crate::display::image::draw_image(&mut display.pixmap_mut(), img, point);
        } else {
            // Fall back to vector drawing if image not found
            Self::draw_vector(display, styles, point, button)?;
        }
        Ok(())
    }

    fn draw_vector(
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
        point: Point,
        button: Key,
    ) -> Result<()> {
        let (color, text_str) = match button {
            Key::A => (styles.button_hints.button_a_color, "A"),
            Key::B => (styles.button_hints.button_b_color, "B"),
            Key::X => (styles.button_hints.button_x_color, "X"),
            Key::Y => (styles.button_hints.button_y_color, "Y"),
            Key::Up => (styles.button_hints.button_bg_color, ""),
            Key::Down => (styles.button_hints.button_bg_color, ""),
            Key::Left => (styles.button_hints.button_bg_color, ""),
            Key::Right => (styles.button_hints.button_bg_color, ""),
            Key::Start => (styles.button_hints.button_bg_color, "START"),
            Key::Select => (styles.button_hints.button_bg_color, "SELECT"),
            Key::L => (styles.button_hints.button_bg_color, "L"),
            Key::R => (styles.button_hints.button_bg_color, "R"),
            Key::Menu => (styles.button_hints.button_bg_color, "MENU"),
            Key::L2 => (styles.button_hints.button_bg_color, "L2"),
            Key::R2 => (styles.button_hints.button_bg_color, "R2"),
            Key::Power => (styles.button_hints.button_bg_color, "POWER"),
            Key::VolDown => (styles.button_hints.button_bg_color, "VOL-"),
            Key::VolUp => (styles.button_hints.button_bg_color, "VOL+"),
            Key::LidClose => (styles.button_hints.button_bg_color, "LID"),
            Key::Unknown => unimplemented!("unknown button"),
        };

        let diameter = styles.button_size() as u32;
        let font_size = styles.button_text_font_size() as u32;

        let text_style = FontTextStyleBuilder::new(styles.ui.ui_font.font())
            .font_fallback(styles.cjk_font.font())
            .font_size(font_size)
            .text_color(styles.button_hints.button_text_color)
            .build();

        let mut pixmap = display.pixmap_mut();

        let mut text_pos = Point::new(
            point.x + diameter as i32 / 2,
            point.y + (diameter as i32 - font_size as i32) / 2,
        );
        let mut text_alignment = Alignment::Center;
        let mut draw_bg = false;
        let mut bg_rect = Rect::new(0, 0, 0, 0);

        match button {
            Key::A | Key::B | Key::X | Key::Y => {
                // Draw circle
                crate::display::fill_circle(
                    &mut pixmap,
                    Point::new(point.x + diameter as i32 / 2, point.y + diameter as i32 / 2),
                    diameter / 2,
                    color,
                );
            }
            Key::Up | Key::Right | Key::Down | Key::Left => {
                // Draw D-pad cross (two rounded rectangles)
                crate::display::fill_rounded_rect(
                    &mut pixmap,
                    Rect::new(
                        point.x,
                        point.y + diameter as i32 * 2 / 7 + 1,
                        diameter,
                        diameter * 3 / 7,
                    ),
                    4,
                    color,
                );
                crate::display::fill_rounded_rect(
                    &mut pixmap,
                    Rect::new(
                        point.x + diameter as i32 * 2 / 7 + 1,
                        point.y,
                        diameter * 3 / 7,
                        diameter,
                    ),
                    4,
                    color,
                );

                // Draw directional indicator
                let indicator_rect = match button {
                    Key::Up => Rect::new(
                        point.x + diameter as i32 * 5 / 14 + 1,
                        point.y + diameter as i32 / 14,
                        diameter * 2 / 7,
                        diameter * 3 / 7,
                    ),
                    Key::Right => Rect::new(
                        point.x + diameter as i32 * 7 / 14 + 1,
                        point.y + diameter as i32 * 5 / 14 + 1,
                        diameter * 3 / 7,
                        diameter * 2 / 7,
                    ),
                    Key::Down => Rect::new(
                        point.x + diameter as i32 * 5 / 14 + 1,
                        point.y + diameter as i32 * 7 / 14 + 1,
                        diameter * 2 / 7,
                        diameter * 3 / 7,
                    ),
                    Key::Left => Rect::new(
                        point.x + diameter as i32 / 14,
                        point.y + diameter as i32 * 5 / 14 + 1,
                        diameter * 3 / 7,
                        diameter * 2 / 7,
                    ),
                    _ => unreachable!(),
                };
                crate::display::fill_rounded_rect(
                    &mut pixmap,
                    indicator_rect,
                    4,
                    styles.ui.text_color,
                );
            }
            Key::L | Key::L2 | Key::R | Key::R2 => {
                // L/R buttons with custom corner radii - use simple rounded rect for now
                // TODO: Implement proper asymmetric corner radii if needed
                crate::display::fill_rounded_rect(
                    &mut pixmap,
                    Rect::new(
                        point.x,
                        point.y + diameter as i32 / 8,
                        diameter,
                        diameter * 3 / 4,
                    ),
                    8,
                    color,
                );
            }
            _ => {
                // Other buttons: draw background later after measuring text
                draw_bg = true;
                text_alignment = Alignment::Left;
                text_pos.x = point.x + 4;

                let text_size = text_style.measure(text_str);
                bg_rect = Rect::new(
                    point.x,
                    point.y + (diameter as i32 - font_size as i32) / 2 - 2,
                    text_size.w + 8,
                    text_size.h + 4,
                );
            }
        }

        if draw_bg {
            crate::display::fill_rounded_rect(&mut pixmap, bg_rect, 8, color);
        }

        // Draw text
        if !text_str.is_empty() {
            let text_width = text_style.measure(text_str).w;
            let draw_pos = match text_alignment {
                Alignment::Left => text_pos,
                Alignment::Center => Point::new(text_pos.x - text_width as i32 / 2, text_pos.y),
                Alignment::Right => Point::new(text_pos.x - text_width as i32, text_pos.y),
            };
            text_style.draw(&mut display.pixmap_mut(), text_str, draw_pos);
        }

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
        styles.button_size() as u32
    }
}

#[async_trait(?Send)]
impl View for ButtonIcon {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let icon_size = self.icons.bounding_box(styles, self.button);
        let w = icon_size.w as i32;

        let point = match self.alignment {
            Alignment::Left => self.point,
            Alignment::Center => Point::new(self.point.x - w / 2, self.point.y),
            Alignment::Right => Point::new(self.point.x - w, self.point.y),
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
        let w = icon_size.w as i32;

        let x = match self.alignment {
            Alignment::Left => self.point.x,
            Alignment::Center => self.point.x - w / 2,
            Alignment::Right => self.point.x - w,
        };

        Rect::new(x, self.point.y, w as u32, icon_size.h)
    }

    fn set_position(&mut self, point: Point) {
        self.point = point;
        self.dirty = true;
    }
}
