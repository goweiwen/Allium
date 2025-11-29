use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::ALLIUM_SD_ROOT;
use common::display::color::Color;
use common::display::{Display, fill_rect};
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::{Stylesheet, StylesheetColor};
use common::view::{ButtonHint, ButtonHints, Image, ImageMode, Label, View};
use tokio::sync::mpsc::Sender;

pub struct ScreenshotViewerView {
    rect: Rect,
    screenshots: Vec<PathBuf>,
    current_index: usize,
    ui_visible: bool,
    image: Option<Image>,
    label: Label<String>,
    button_hints: ButtonHints<String>,
    dirty: bool,
}

impl ScreenshotViewerView {
    pub fn new(rect: Rect, res: Resources) -> Result<Self> {
        let screenshots_dir = ALLIUM_SD_ROOT.join("Screenshots");
        let mut screenshots = Vec::new();

        if screenshots_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&screenshots_dir)
        {
            let mut valid_entries: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let path = e.path();
                    path.is_file()
                        && path
                            .extension()
                            .and_then(|s| s.to_str())
                            .map(|s| {
                                matches!(s.to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "bmp")
                            })
                            .unwrap_or(false)
                })
                .collect();

            // Sort by modification time (newest first)
            valid_entries.sort_by_key(|e| {
                e.metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .map(std::cmp::Reverse)
            });

            screenshots = valid_entries.iter().map(|e| e.path()).collect();
        }

        // Screenshot
        let image = if !screenshots.is_empty() {
            Some(Image::new(rect, screenshots[0].clone(), ImageMode::Contain))
        } else {
            None
        };

        // Filename
        let styles = res.get::<Stylesheet>();
        let label_text = screenshots
            .first()
            .map(|path| format_filename(path))
            .unwrap_or_default();

        let mut label = Label::new(
            Point::new(rect.x + styles.ui.margin_x, rect.y + styles.ui.margin_y),
            label_text,
            Alignment::Left,
            None,
        );
        label.color(StylesheetColor::Foreground);

        // Create button hints and empty state views
        let locale = res.get::<Locale>();
        let button_hints = ButtonHints::new(
            res.clone(),
            vec![],
            vec![
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
                    Key::Y,
                    locale.t("screenshot-viewer-button-hide-ui"),
                    Alignment::Right,
                ),
            ],
        );

        drop(styles);
        drop(locale);

        Ok(Self {
            rect,
            screenshots,
            current_index: 0,
            ui_visible: true,
            image,
            label,
            button_hints,
            dirty: true,
        })
    }

    fn navigate(&mut self, delta: isize) {
        if self.screenshots.is_empty() {
            return;
        }

        let len = self.screenshots.len() as isize;
        let mut new_index = self.current_index as isize + delta;

        // Wrap around
        if new_index < 0 {
            new_index = len - 1;
        } else if new_index >= len {
            new_index = 0;
        }

        self.current_index = new_index as usize;

        // Update image
        self.image = Some(Image::new(
            self.rect,
            self.screenshots[self.current_index].clone(),
            ImageMode::Contain,
        ));

        // Update label
        let label_text = format_filename(&self.screenshots[self.current_index]);
        self.label.set_text(label_text);

        self.set_should_draw();
    }

    fn toggle_ui(&mut self) {
        self.ui_visible = !self.ui_visible;
        self.set_should_draw();
    }
}

#[async_trait(?Send)]
impl View for ScreenshotViewerView {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if self.screenshots.is_empty() {
            // TODO
            return Ok(false);
        }

        let mut drawn = false;

        if !self.dirty {
            return Ok(false);
        }

        if let Some(ref mut image) = self.image {
            drawn |= image.should_draw() && image.draw(display, styles)?;
        }

        if self.ui_visible {
            if self.label.should_draw() {
                let rect = self.label.bounding_box(styles);
                let bg_rect = Rect::new(
                    rect.x - styles.ui.margin_x,
                    rect.y - styles.ui.margin_y,
                    rect.w + (styles.ui.margin_x * 2) as u32,
                    rect.h + (styles.ui.margin_y * 2) as u32,
                );
                let bg_color = styles.ui.background_color;
                fill_rect(
                    &mut display.pixmap_mut(),
                    bg_rect,
                    Color::rgba(bg_color.r(), bg_color.g(), bg_color.b(), 180),
                );
                drawn |= self.label.draw(display, styles)?;
            }

            if self.button_hints.should_draw() {
                let rect = self.button_hints.bounding_box(styles);
                let bg_rect = Rect::new(
                    rect.x - styles.ui.margin_x,
                    rect.y - styles.ui.margin_y,
                    rect.w + (styles.ui.margin_x * 2) as u32,
                    rect.h + (styles.ui.margin_y * 2) as u32,
                );
                let bg_color = styles.ui.background_color;
                fill_rect(
                    &mut display.pixmap_mut(),
                    bg_rect,
                    Color::rgba(bg_color.r(), bg_color.g(), bg_color.b(), 180),
                );
                drawn |= self.button_hints.draw(display, styles)?;
            }
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.dirty
            || (!self.screenshots.is_empty()
                && (self
                    .image
                    .as_ref()
                    .map(|i| i.should_draw())
                    .unwrap_or(false)
                    || (self.ui_visible
                        && (self.label.should_draw() || self.button_hints.should_draw()))))
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        if let Some(ref mut image) = self.image {
            image.set_should_draw();
        }
        self.label.set_should_draw();
        self.button_hints.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        command: Sender<Command>,
        _bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        match event {
            KeyEvent::Pressed(Key::Left) | KeyEvent::Autorepeat(Key::Left) => {
                self.navigate(-1);
            }
            KeyEvent::Pressed(Key::Right) | KeyEvent::Autorepeat(Key::Right) => {
                self.navigate(1);
            }
            KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                self.navigate(-1);
            }
            KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                self.navigate(1);
            }
            KeyEvent::Pressed(Key::Y) => {
                self.toggle_ui();
            }
            KeyEvent::Pressed(Key::B) => {
                command.send(Command::Exit).await?;
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
        self.rect
    }

    fn set_position(&mut self, point: Point) {
        self.rect.x = point.x;
        self.rect.y = point.y;
    }
}

fn format_filename(path: &Path) -> String {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let Some((date, app)) = name.rsplit_once('-') else {
        return name;
    };

    let Ok(date) = chrono::NaiveDateTime::parse_from_str(date.trim(), "%Y-%m-%d_%H-%M-%S") else {
        return name;
    };
    let date = date.format("%-d %b %Y %H:%M");

    format!("{} - {}", app.trim(), date)
}
