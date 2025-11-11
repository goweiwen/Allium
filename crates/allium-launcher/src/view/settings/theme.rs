use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use async_trait::async_trait;
use common::command::{Command, Value};
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::{Stylesheet, StylesheetFont};
use common::view::{
    ButtonHint, ButtonIcon, ColorPicker, Number, Percentage, Row, Select, SettingsList, Toggle,
    View,
};
use log::error;
use tokio::sync::mpsc::Sender;

use crate::view::settings::{ChildState, SettingsChild};

type Handler =
    Box<dyn Fn(&mut Stylesheet, &[PathBuf], &[String], Value, &Sender<Command>) -> Result<()>>;

pub struct Theme {
    rect: Rect,
    stylesheet: Stylesheet,
    themes: Vec<String>,
    fonts: Vec<PathBuf>,
    list: SettingsList,
    handlers: Vec<Handler>,
    left_button_hints: Row<ButtonHint<String>>,
    right_button_hints: Row<ButtonHint<String>>,
    restore_pressed: Option<Instant>,
}

impl Theme {
    pub fn new(rect: Rect, res: Resources, state: Option<ChildState>) -> Self {
        let Rect { x, y, w, h } = rect;

        let stylesheet = Stylesheet::load().unwrap();

        let locale = res.get::<Locale>();
        let styles = res.get::<Stylesheet>();

        let themes = Stylesheet::available_themes().unwrap_or_default();
        let current_theme = common::stylesheet::Theme::load();
        let current_theme_index = themes
            .iter()
            .position(|t| t == &current_theme.0)
            .unwrap_or(0);

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

        let items: Vec<(String, Box<dyn View>, Handler)> = vec![
            (
                locale.t("settings-theme-theme"),
                Box::new(Select::new(
                    Point::zero(),
                    current_theme_index,
                    themes.clone(),
                    Alignment::Right,
                )),
                Box::new(move |stylesheet, _fonts, themes, val, commands| {
                    let theme_index = val.as_int().unwrap() as usize;
                    if theme_index < themes.len() {
                        let theme_name = &themes[theme_index];
                        let theme_obj = common::stylesheet::Theme(theme_name.clone());
                        if let Err(e) = theme_obj.save() {
                            error!("failed to save theme: {}", e);
                        }
                        *stylesheet = Stylesheet::load_from_theme(&theme_obj)?;
                        commands
                            .try_send(Command::ReloadStylesheet(Box::new(stylesheet.clone())))?;
                    }
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-show-battery-level"),
                Box::new(Toggle::new(
                    Point::zero(),
                    stylesheet.show_battery_level,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, _val, _commands| {
                    stylesheet.toggle_battery_percentage();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-show-clock"),
                Box::new(Toggle::new(
                    Point::zero(),
                    stylesheet.show_clock,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, _val, _commands| {
                    stylesheet.toggle_clock();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-use-recents-carousel"),
                Box::new(Toggle::new(
                    Point::zero(),
                    stylesheet.use_recents_carousel,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, _val, _commands| {
                    stylesheet.use_recents_carousel = !stylesheet.use_recents_carousel;
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-boxart-width"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.boxart_width as i32,
                    0,
                    400,
                    10,
                    |px| {
                        if *px == 0 {
                            "Disabled".to_owned()
                        } else {
                            format!("{}px", px)
                        }
                    },
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.boxart_width = val.as_int().unwrap() as u32;
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-ui-font"),
                Box::new(Select::new(
                    Point::zero(),
                    fonts
                        .iter()
                        .position(|p| p.file_name() == stylesheet.ui_font.path.file_name())
                        .unwrap_or_default(),
                    font_names.clone(),
                    Alignment::Right,
                )),
                Box::new(move |stylesheet, fonts, _themes, val, _commands| {
                    stylesheet
                        .ui_font
                        .path
                        .clone_from(&fonts[val.as_int().unwrap() as usize]);
                    stylesheet.load_fonts()?;
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-ui-font-size"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.ui_font.size as i32,
                    10,
                    60,
                    5,
                    i32::to_string,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.ui_font.size = val.as_int().unwrap() as u32;
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-guide-font"),
                Box::new(Select::new(
                    Point::zero(),
                    fonts
                        .iter()
                        .position(|p| p.file_name() == stylesheet.guide_font.path.file_name())
                        .unwrap_or_default(),
                    font_names,
                    Alignment::Right,
                )),
                Box::new(move |stylesheet, fonts, _themes, val, _commands| {
                    stylesheet
                        .guide_font
                        .path
                        .clone_from(&fonts[val.as_int().unwrap() as usize]);
                    stylesheet.load_fonts()?;
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-guide-font-size"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.guide_font.size as i32,
                    10,
                    60,
                    5,
                    i32::to_string,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.guide_font.size = val.as_int().unwrap() as u32;
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-margin-x"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.margin_x,
                    0,
                    30,
                    5,
                    |x| format!("{x}px"),
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.margin_x = val.as_int().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-margin-y"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.margin_y,
                    0,
                    30,
                    5,
                    |x| format!("{x}px"),
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.margin_y = val.as_int().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-list-margin"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.list_margin,
                    0,
                    30,
                    5,
                    |x| format!("{x}px"),
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.list_margin = val.as_int().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-padding-x"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.padding_x,
                    0,
                    30,
                    5,
                    |x| format!("{x}px"),
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.padding_x = val.as_int().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-padding-y"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.padding_y,
                    0,
                    30,
                    5,
                    |x| format!("{x}px"),
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.padding_y = val.as_int().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-foreground-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.foreground_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.foreground_color = val.as_color().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-background-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.background_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.background_color = val.as_color().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-highlight-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.highlight_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.highlight_color = val.as_color().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-highlight-text-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.highlight_text_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.highlight_text_color = val.as_color().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-disabled-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.disabled_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.disabled_color = val.as_color().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-tab-font-size"),
                Box::new(Percentage::new(
                    Point::zero(),
                    (stylesheet.tab_font_size * 100.0) as i32,
                    0,
                    200,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.tab_font_size = val.as_int().unwrap() as f32 / 100.0;
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-tab-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.tab_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.tab_color = val.as_color().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-tab-selected-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.tab_selected_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.tab_selected_color = val.as_color().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-status-bar-font-size"),
                Box::new(Percentage::new(
                    Point::zero(),
                    (stylesheet.status_bar_font_size * 100.0) as i32,
                    0,
                    200,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.status_bar_font_size = val.as_int().unwrap() as f32 / 100.0;
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-status-bar-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.status_bar_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.status_bar_color = val.as_color().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-button-hint-font-size"),
                Box::new(Percentage::new(
                    Point::zero(),
                    (stylesheet.button_hint_font_size * 100.0) as i32,
                    0,
                    200,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.button_hint_font_size = val.as_int().unwrap() as f32 / 100.0;
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-button-a-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_a_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.button_a_color = val.as_color().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-button-b-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_b_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.button_b_color = val.as_color().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-button-x-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_x_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.button_x_color = val.as_color().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-button-y-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_y_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.button_y_color = val.as_color().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-button-text-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_text_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.button_text_color = val.as_color().unwrap();
                    Ok(())
                }),
            ),
            (
                locale.t("settings-theme-button-hint-text-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_hint_text_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _fonts, _themes, val, _commands| {
                    stylesheet.button_hint_text_color = val.as_color().unwrap();
                    Ok(())
                }),
            ),
        ];

        // Unzip into left, right, and handlers
        let (left, right, handlers): (Vec<_>, Vec<_>, Vec<_>) = items.into_iter().fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut left, mut right, mut handlers), (l, r, h)| {
                left.push(l);
                right.push(r);
                handlers.push(h);
                (left, right, handlers)
            },
        );

        let mut list = SettingsList::new(
            res.clone(),
            Rect::new(
                x + styles.margin_x,
                y,
                w - styles.margin_x as u32 * 2,
                h - ButtonIcon::diameter(&styles) - styles.margin_y as u32,
            ),
            left,
            right,
            res.get::<Stylesheet>().ui_font.size + styles.padding_y as u32,
        );
        if let Some(state) = state {
            list.select(state.selected);
        }

        let left_button_hints = Row::new(
            Point::new(
                rect.x + styles.margin_x,
                rect.y + rect.h as i32 - ButtonIcon::diameter(&styles) as i32 - styles.margin_y,
            ),
            vec![ButtonHint::new(
                res.clone(),
                Point::zero(),
                Key::X,
                locale.t("button-restore-defaults"),
                Alignment::Left,
            )],
            Alignment::Left,
            12,
        );

        let right_button_hints = Row::new(
            Point::new(
                rect.x + rect.w as i32 - styles.margin_y,
                rect.y + rect.h as i32 - ButtonIcon::diameter(&styles) as i32 - styles.margin_y,
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
            themes,
            fonts,
            list,
            handlers,
            left_button_hints,
            right_button_hints,
            restore_pressed: None,
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

        if self.left_button_hints.should_draw() && self.left_button_hints.draw(display, styles)? {
            drawn = true;
        }

        if self.right_button_hints.should_draw() && self.right_button_hints.draw(display, styles)? {
            drawn = true;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.list.should_draw()
            || self.left_button_hints.should_draw()
            || self.right_button_hints.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.list.set_should_draw();
        self.left_button_hints.set_should_draw();
        self.right_button_hints.set_should_draw();
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
                    self.handlers[i](
                        &mut self.stylesheet,
                        &self.fonts,
                        &self.themes,
                        val,
                        &commands,
                    )?;

                    self.stylesheet.save()?;
                    commands
                        .send(Command::ReloadStylesheet(Box::new(self.stylesheet.clone())))
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
            KeyEvent::Pressed(Key::X) => {
                if let Some(pressed_at) = self.restore_pressed {
                    // Check if within 3 seconds
                    if pressed_at.elapsed().as_secs() < 3 {
                        // Second press within window - dismiss toast and restore defaults
                        commands.send(Command::DismissToast).await?;
                        self.restore_pressed = None;
                        self.stylesheet.restore_defaults()?;
                        self.stylesheet.save()?;
                        commands
                            .send(Command::ReloadStylesheet(Box::new(self.stylesheet.clone())))
                            .await?;
                    } else {
                        // Expired, treat as first press
                        self.restore_pressed = Some(Instant::now());
                        commands
                            .send(Command::Toast(
                                "Press X again to restore defaults\nAll changes will be lost"
                                    .to_string(),
                                Some(std::time::Duration::from_secs(3)),
                            ))
                            .await?;
                    }
                } else {
                    // First press - show confirmation toast
                    self.restore_pressed = Some(Instant::now());
                    commands
                        .send(Command::Toast(
                            "Press X again to restore defaults\nAll changes will be lost"
                                .to_string(),
                            Some(std::time::Duration::from_secs(3)),
                        ))
                        .await?;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![
            &self.list,
            &self.left_button_hints,
            &self.right_button_hints,
        ]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![
            &mut self.list,
            &mut self.left_button_hints,
            &mut self.right_button_hints,
        ]
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
