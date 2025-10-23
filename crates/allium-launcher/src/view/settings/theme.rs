use std::collections::VecDeque;
use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::SELECTION_MARGIN;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::{Stylesheet, StylesheetFont};
use common::view::{
    ButtonHint, ButtonIcon, ColorPicker, Number, Percentage, Row, Select, SettingsList, Toggle,
    View,
};
use tokio::sync::mpsc::Sender;

use crate::view::settings::{ChildState, SettingsChild};

pub struct Theme {
    rect: Rect,
    stylesheet: Stylesheet,
    fonts: Vec<PathBuf>,
    list: SettingsList,
    button_hints: Row<ButtonHint<String>>,
}

impl Theme {
    pub fn new(rect: Rect, res: Resources, state: Option<ChildState>) -> Self {
        let Rect { x, y, w, h } = rect;

        let stylesheet = Stylesheet::load().unwrap();

        let locale = res.get::<Locale>();
        let styles = res.get::<Stylesheet>();

        let fonts = StylesheetFont::available_fonts().unwrap_or_default();
        let font_names: Vec<String> = fonts
            .iter()
            .map(|p| {
                p.file_stem()
                    .and_then(std::ffi::OsStr::to_str)
                    .unwrap_or("Unknown")
                    .replace(['_', '-'], " ")
            })
            .collect();

        let mut list = SettingsList::new(
            Rect::new(
                x + 12,
                y + 8,
                w - 24,
                h - 8 - ButtonIcon::diameter(&styles) - 8,
            ),
            vec![
                locale.t("settings-theme-dark-mode"),
                locale.t("settings-theme-show-battery-level"),
                locale.t("settings-theme-show-clock"),
                locale.t("settings-theme-use-recents-carousel"),
                locale.t("settings-theme-ui-font"),
                locale.t("settings-theme-ui-font-size"),
                locale.t("settings-theme-guide-font"),
                locale.t("settings-theme-guide-font-size"),
                locale.t("settings-theme-tab-font-size"),
                locale.t("settings-theme-status-bar-font-size"),
                locale.t("settings-theme-button-hint-font-size"),
                locale.t("settings-theme-highlight-color"),
                locale.t("settings-theme-foreground-color"),
                locale.t("settings-theme-background-color"),
                locale.t("settings-theme-disabled-color"),
                locale.t("settings-theme-tab-color"),
                locale.t("settings-theme-tab-selected-color"),
                locale.t("settings-theme-button-a-color"),
                locale.t("settings-theme-button-b-color"),
                locale.t("settings-theme-button-x-color"),
                locale.t("settings-theme-button-y-color"),
            ],
            vec![
                Box::new(Toggle::new(
                    Point::zero(),
                    stylesheet.background_color.is_dark(),
                    Alignment::Right,
                )),
                Box::new(Toggle::new(
                    Point::zero(),
                    stylesheet.show_battery_level,
                    Alignment::Right,
                )),
                Box::new(Toggle::new(
                    Point::zero(),
                    stylesheet.show_clock,
                    Alignment::Right,
                )),
                Box::new(Toggle::new(
                    Point::zero(),
                    stylesheet.use_recents_carousel,
                    Alignment::Right,
                )),
                Box::new(Select::new(
                    Point::zero(),
                    fonts
                        .iter()
                        .position(|p| *p == stylesheet.ui_font.path)
                        .unwrap_or_default(),
                    font_names.clone(),
                    Alignment::Right,
                )),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.ui_font.size as i32,
                    20,
                    60,
                    i32::to_string,
                    Alignment::Right,
                )),
                Box::new(Select::new(
                    Point::zero(),
                    fonts
                        .iter()
                        .position(|p| *p == stylesheet.guide_font.path)
                        .unwrap_or_default(),
                    font_names,
                    Alignment::Right,
                )),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.guide_font.size as i32,
                    20,
                    60,
                    i32::to_string,
                    Alignment::Right,
                )),
                Box::new(Percentage::new(
                    Point::zero(),
                    (stylesheet.tab_font_size * 100.0) as i32,
                    50,
                    200,
                    Alignment::Right,
                )),
                Box::new(Percentage::new(
                    Point::zero(),
                    (stylesheet.status_bar_font_size * 100.0) as i32,
                    50,
                    200,
                    Alignment::Right,
                )),
                Box::new(Percentage::new(
                    Point::zero(),
                    (stylesheet.button_hint_font_size * 100.0) as i32,
                    50,
                    200,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.highlight_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.foreground_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.background_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.disabled_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.tab_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.tab_selected_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_a_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_b_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_x_color,
                    Alignment::Right,
                )),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_y_color,
                    Alignment::Right,
                )),
            ],
            res.get::<Stylesheet>().ui_font.size + SELECTION_MARGIN,
        );
        if let Some(state) = state {
            list.select(state.selected);
        }

        let button_hints = Row::new(
            Point::new(
                rect.x + rect.w as i32 - 12,
                rect.y + rect.h as i32 - ButtonIcon::diameter(&styles) as i32 - 8,
            ),
            vec![
                ButtonHint::new(
                    res.clone(),
                    Point::zero(),
                    Key::A,
                    locale.t("button-edit"),
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
            Alignment::Right,
            12,
        );

        Self {
            rect,
            stylesheet,
            fonts,
            list,
            button_hints,
        }
    }
}

#[async_trait(?Send)]
impl View for Theme {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let mut drawn = false;

        if self.list.should_draw() && self.list.draw(display, styles)? {
            drawn = true;
        }

        if self.button_hints.should_draw() && self.button_hints.draw(display, styles)? {
            drawn = true;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.list.should_draw() || self.button_hints.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.list.set_should_draw();
        self.button_hints.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if self
            .list
            .handle_key_event(event, commands.clone(), bubble)
            .await?
        {
            while let Some(command) = bubble.pop_front() {
                if let Command::ValueChanged(i, val) = command {
                    match i {
                        0 => {
                            self.stylesheet.toggle_dark_mode();
                            self.list.set_right(
                                11,
                                Box::new(ColorPicker::new(
                                    Point::zero(),
                                    self.stylesheet.foreground_color,
                                    Alignment::Right,
                                )),
                            );
                            self.list.set_right(
                                12,
                                Box::new(ColorPicker::new(
                                    Point::zero(),
                                    self.stylesheet.background_color,
                                    Alignment::Right,
                                )),
                            );
                            self.list.set_right(
                                13,
                                Box::new(ColorPicker::new(
                                    Point::zero(),
                                    self.stylesheet.disabled_color,
                                    Alignment::Right,
                                )),
                            );
                            self.list.set_right(
                                14,
                                Box::new(ColorPicker::new(
                                    Point::zero(),
                                    self.stylesheet.tab_color,
                                    Alignment::Right,
                                )),
                            );
                            self.list.set_right(
                                15,
                                Box::new(ColorPicker::new(
                                    Point::zero(),
                                    self.stylesheet.tab_selected_color,
                                    Alignment::Right,
                                )),
                            );
                            self.list.set_right(
                                16,
                                Box::new(ColorPicker::new(
                                    Point::zero(),
                                    self.stylesheet.button_a_color,
                                    Alignment::Right,
                                )),
                            );
                            self.list.set_right(
                                17,
                                Box::new(ColorPicker::new(
                                    Point::zero(),
                                    self.stylesheet.button_b_color,
                                    Alignment::Right,
                                )),
                            );
                            self.list.set_right(
                                18,
                                Box::new(ColorPicker::new(
                                    Point::zero(),
                                    self.stylesheet.button_x_color,
                                    Alignment::Right,
                                )),
                            );
                            self.list.set_right(
                                19,
                                Box::new(ColorPicker::new(
                                    Point::zero(),
                                    self.stylesheet.button_y_color,
                                    Alignment::Right,
                                )),
                            );
                        }
                        1 => self.stylesheet.toggle_battery_percentage(),
                        2 => self.stylesheet.toggle_clock(),
                        3 => {
                            self.stylesheet.use_recents_carousel =
                                !self.stylesheet.use_recents_carousel
                        }
                        4 => self
                            .stylesheet
                            .ui_font
                            .path
                            .clone_from(&self.fonts[val.as_int().unwrap() as usize]),
                        5 => self.stylesheet.ui_font.size = val.as_int().unwrap() as u32,
                        6 => self
                            .stylesheet
                            .guide_font
                            .path
                            .clone_from(&self.fonts[val.as_int().unwrap() as usize]),
                        7 => self.stylesheet.guide_font.size = val.as_int().unwrap() as u32,
                        8 => self.stylesheet.tab_font_size = val.as_int().unwrap() as f32 / 100.0,
                        9 => {
                            self.stylesheet.status_bar_font_size =
                                val.as_int().unwrap() as f32 / 100.0
                        }
                        10 => {
                            self.stylesheet.button_hint_font_size =
                                val.as_int().unwrap() as f32 / 100.0
                        }
                        11 => self.stylesheet.highlight_color = val.as_color().unwrap(),
                        12 => self.stylesheet.foreground_color = val.as_color().unwrap(),
                        13 => self.stylesheet.background_color = val.as_color().unwrap(),
                        14 => self.stylesheet.disabled_color = val.as_color().unwrap(),
                        15 => self.stylesheet.tab_color = val.as_color().unwrap(),
                        16 => self.stylesheet.tab_selected_color = val.as_color().unwrap(),
                        17 => self.stylesheet.button_a_color = val.as_color().unwrap(),
                        18 => self.stylesheet.button_b_color = val.as_color().unwrap(),
                        19 => self.stylesheet.button_x_color = val.as_color().unwrap(),
                        20 => self.stylesheet.button_y_color = val.as_color().unwrap(),
                        _ => unreachable!("Invalid index"),
                    }

                    commands
                        .send(Command::SaveStylesheet(Box::new(self.stylesheet.clone())))
                        .await?;
                }
            }
            return Ok(true);
        }

        match event {
            KeyEvent::Pressed(Key::B) => {
                bubble.push_back(Command::CloseView);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.list, &self.button_hints]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.list, &mut self.button_hints]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}

impl SettingsChild for Theme {
    fn save(&self) -> ChildState {
        ChildState {
            selected: self.list.selected(),
        }
    }
}
