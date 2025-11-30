use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::display::Display;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, ButtonHints, ScrollList, View};
use tokio::sync::mpsc::Sender;

use crate::view::text_reader::TextReader;

pub struct GuideSelector {
    rect: Rect,
    res: Resources,
    list: ScrollList,
    button_hints: ButtonHints<String>,
    guides: Vec<PathBuf>,
    text_reader: Option<Box<TextReader>>,
}

impl GuideSelector {
    pub fn new(
        rect: Rect,
        res: Resources,
        guides: Vec<PathBuf>,
        selected: usize,
        text_reader_open: bool,
    ) -> Self {
        let styles = res.get::<Stylesheet>();
        let locale = res.get::<Locale>();

        let guide_names: Vec<String> = guides
            .iter()
            .filter_map(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            })
            .collect();

        let mut button_hints = ButtonHints::new(
            res.clone(),
            vec![ButtonHint::new(
                res.clone(),
                Point::zero(),
                Key::Menu,
                locale.t("ingame-menu-continue"),
                Alignment::Left,
            )],
            vec![
                ButtonHint::new(
                    res.clone(),
                    Point::zero(),
                    Key::A,
                    locale.t("button-select"),
                    Alignment::Right,
                ),
                ButtonHint::new(
                    res.clone(),
                    Point::zero(),
                    Key::B,
                    locale.t("button-back"),
                    Alignment::Right,
                ),
            ],
        );

        let button_hints_rect = button_hints.bounding_box(&styles);
        let list_rect = Rect::new(
            rect.x + styles.ui.margin_x,
            rect.y,
            rect.w - (styles.ui.margin_x * 2) as u32,
            button_hints_rect.y as u32 - rect.y as u32,
        );

        let mut list = ScrollList::new(
            res.clone(),
            list_rect,
            guide_names,
            Alignment::Left,
            styles.ui.ui_font.size + styles.ui.padding_y as u32,
        );

        list.select(selected);

        let text_reader = if (text_reader_open || guides.len() == 1)
            && let Some(path) = guides.get(selected)
        {
            Some(Box::new(TextReader::new(rect, res.clone(), path.clone())))
        } else {
            None
        };

        drop(locale);
        drop(styles);

        Self {
            rect,
            res,
            list,
            button_hints,
            guides,
            text_reader,
        }
    }

    #[allow(dead_code)]
    pub fn selected(&self) -> usize {
        self.list.selected()
    }

    pub fn selected_path(&self) -> Option<&Path> {
        self.guides.get(self.list.selected()).map(|p| p.as_path())
    }

    pub fn is_text_reader_open(&self) -> bool {
        self.text_reader.is_some()
    }

    pub fn text_reader(&self) -> Option<&TextReader> {
        self.text_reader.as_deref()
    }

    fn open_text_reader(&mut self) {
        if let Some(path) = self.guides.get(self.list.selected()) {
            self.text_reader = Some(Box::new(TextReader::new(
                self.rect,
                self.res.clone(),
                path.clone(),
            )));
        }
    }

    fn close_text_reader(&mut self) {
        if let Some(reader) = self.text_reader.take() {
            reader.save_cursor();
        }
        self.set_should_draw();
    }
}

#[async_trait(?Send)]
impl View for GuideSelector {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if let Some(reader) = &mut self.text_reader {
            reader.draw(display, styles)
        } else {
            let mut drawn = false;
            drawn |= self.list.should_draw() && self.list.draw(display, styles)?;
            if self.button_hints.should_draw() {
                display.load(self.button_hints.bounding_box(styles))?;
                drawn |= self.button_hints.draw(display, styles)?;
            }
            Ok(drawn)
        }
    }

    fn should_draw(&self) -> bool {
        if let Some(reader) = &self.text_reader {
            reader.should_draw()
        } else {
            self.list.should_draw() || self.button_hints.should_draw()
        }
    }

    fn set_should_draw(&mut self) {
        if let Some(reader) = &mut self.text_reader {
            reader.set_should_draw();
        } else {
            self.list.set_should_draw();
            self.button_hints.set_should_draw();
        }
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if let Some(reader) = &mut self.text_reader {
            if reader.handle_key_event(event, commands, bubble).await? {
                // Check if reader requested to close
                bubble.retain(|cmd| match cmd {
                    Command::CloseView => {
                        self.close_text_reader();
                        false
                    }
                    _ => true,
                });
                return Ok(true);
            }
            return Ok(false);
        }

        // Handle list navigation
        if self.list.handle_key_event(event, commands, bubble).await? {
            return Ok(true);
        }

        match event {
            KeyEvent::Pressed(Key::A) => {
                self.open_text_reader();
                Ok(true)
            }
            KeyEvent::Pressed(Key::B) => {
                bubble.push_back(Command::CloseView);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        if let Some(reader) = &self.text_reader {
            vec![reader.as_ref()]
        } else {
            vec![&self.list]
        }
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        if let Some(reader) = &mut self.text_reader {
            vec![reader.as_mut()]
        } else {
            vec![&mut self.list]
        }
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: common::geom::Point) {
        unimplemented!()
    }
}
