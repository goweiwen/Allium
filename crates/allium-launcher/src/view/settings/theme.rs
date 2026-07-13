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
    ButtonHint, ButtonHints, ColorPicker, Number, Percentage, Select, SettingsList, Toggle, View,
};
use log::error;
use tokio::sync::mpsc::Sender;

use crate::view::settings::{ChildState, SettingsChild};

struct ThemeContext {
    fonts: Vec<PathBuf>,
    wallpapers: Vec<PathBuf>,
    themes: Vec<String>,
}

type Handler = Box<dyn Fn(&mut Stylesheet, &ThemeContext, Value, &Sender<Command>) -> Result<bool>>;

pub struct Theme {
    rect: Rect,
    stylesheet: Stylesheet,
    context: ThemeContext,
    list: SettingsList,
    handlers: Vec<Handler>,
    button_hints: ButtonHints<String>,
    restore_pressed: Option<Instant>,
}

impl Theme {
    pub fn new(rect: Rect, res: Resources, state: Option<ChildState>) -> Self {
        let Rect { x, y, w, .. } = rect;

        let stylesheet = Stylesheet::load().unwrap();

        let locale = res.get::<Locale>();
        let styles = res.get::<Stylesheet>();

        let context = ThemeContext {
            fonts: StylesheetFont::available_fonts().unwrap_or_default(),
            wallpapers: Stylesheet::available_wallpapers().unwrap_or_default(),
            themes: Stylesheet::available_themes().unwrap_or_default(),
        };

        let font_names: Vec<String> = context
            .fonts
            .iter()
            .map(|p| {
                p.file_stem()
                    .and_then(std::ffi::OsStr::to_str)
                    .unwrap_or("Unknown")
                    .replace(['_', '-'], " ")
            })
            .collect();

        let mut wallpaper_names: Vec<String> = vec!["None".to_string()];
        wallpaper_names.extend(context.wallpapers.iter().map(|p| {
            p.file_name()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("Unknown")
                .to_string()
        }));
        let current_wallpaper_index = if let Some(ref wp) = stylesheet.wallpaper {
            context
                .wallpapers
                .iter()
                .position(|w| w.file_name() == wp.file_name())
                .map(|i| i + 1)
                .unwrap_or(0)
        } else {
            0
        };

        let current_theme = common::stylesheet::Theme::load();
        let current_theme_index = context
            .themes
            .iter()
            .position(|t| t == &current_theme.0)
            .unwrap_or(0);

        let items: Vec<(String, Box<dyn View>, Handler)> = vec![
            (
                locale.t("settings-theme-theme"),
                Box::new(Select::new(
                    Point::zero(),
                    current_theme_index,
                    context.themes.clone(),
                    Alignment::Right,
                )),
                Box::new(|stylesheet, ctx, val, _commands| {
                    let theme_index = val.as_int().unwrap() as usize;
                    if theme_index < ctx.themes.len() {
                        let theme_name = &ctx.themes[theme_index];
                        let theme_obj = common::stylesheet::Theme(theme_name.clone());
                        if let Err(e) = theme_obj.save() {
                            error!("failed to save theme: {}", e);
                        }
                        *stylesheet = Stylesheet::load_from_theme(&theme_obj)?;
                    }
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-wallpaper"),
                Box::new(Select::new(
                    Point::zero(),
                    current_wallpaper_index,
                    wallpaper_names.clone(),
                    Alignment::Right,
                )),
                Box::new(|stylesheet, ctx, val, _commands| {
                    let wallpaper_index = val.as_int().unwrap() as usize;
                    if wallpaper_index == 0 {
                        stylesheet.wallpaper = None;

                        if stylesheet.ui.background_color.a() != 255 {
                            stylesheet.ui.background_color =
                                stylesheet.ui.background_color.with_a(255);
                        }
                    } else if wallpaper_index - 1 < ctx.wallpapers.len() {
                        stylesheet.wallpaper = Some(ctx.wallpapers[wallpaper_index - 1].clone());

                        if stylesheet.ui.background_color.a() == 255 {
                            stylesheet.ui.background_color =
                                stylesheet.ui.background_color.with_a(0);
                        }
                    }
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-show-battery-level"),
                Box::new(Toggle::new(
                    Point::zero(),
                    stylesheet.status_bar.show_battery_level,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, _val, _commands| {
                    stylesheet.toggle_battery_percentage();
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-show-clock"),
                Box::new(Toggle::new(
                    Point::zero(),
                    stylesheet.status_bar.show_clock,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, _val, _commands| {
                    stylesheet.toggle_clock();
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-show-wifi"),
                Box::new(Toggle::new(
                    Point::zero(),
                    stylesheet.status_bar.show_wifi,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, _val, _commands| {
                    stylesheet.toggle_wifi();
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-use-recents-carousel"),
                Box::new(Toggle::new(
                    Point::zero(),
                    stylesheet.recents.use_recents_carousel,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, _val, _commands| {
                    stylesheet.recents.use_recents_carousel =
                        !stylesheet.recents.use_recents_carousel;
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-boxart-width"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.games.boxart_width as i32,
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
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.games.boxart_width = val.as_int().unwrap() as u32;
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-boxart-underlay"),
                Box::new(Toggle::new(
                    Point::zero(),
                    stylesheet.games.boxart_underlay,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, _val, _commands| {
                    stylesheet.games.boxart_underlay = !stylesheet.games.boxart_underlay;
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-boxart-border-radius"),
                Box::new(Number::new(
                    Point::zero(),
                    (stylesheet.games.boxart_border_radius * 100.0) as i32,
                    0,
                    100,
                    5,
                    |v| format!("{}%", v),
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.games.boxart_border_radius = val.as_int().unwrap() as f32 / 100.0;
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-ui-font"),
                Box::new(Select::new(
                    Point::zero(),
                    context
                        .fonts
                        .iter()
                        .position(|p| p.file_name() == stylesheet.ui.ui_font.path.file_name())
                        .unwrap_or_default(),
                    font_names.clone(),
                    Alignment::Right,
                )),
                Box::new(|stylesheet, ctx, val, _commands| {
                    stylesheet
                        .ui
                        .ui_font
                        .path
                        .clone_from(&ctx.fonts[val.as_int().unwrap() as usize]);
                    stylesheet.load_fonts()?;
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-ui-font-size"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.ui.ui_font.size as i32,
                    10,
                    60,
                    5,
                    i32::to_string,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.ui.ui_font.size = val.as_int().unwrap() as u32;
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-guide-font"),
                Box::new(Select::new(
                    Point::zero(),
                    context
                        .fonts
                        .iter()
                        .position(|p| p.file_name() == stylesheet.menu.guide_font.path.file_name())
                        .unwrap_or_default(),
                    font_names,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, ctx, val, _commands| {
                    stylesheet
                        .menu
                        .guide_font
                        .path
                        .clone_from(&ctx.fonts[val.as_int().unwrap() as usize]);
                    stylesheet.load_fonts()?;
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-guide-font-size"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.menu.guide_font.size as i32,
                    10,
                    60,
                    5,
                    i32::to_string,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.menu.guide_font.size = val.as_int().unwrap() as u32;
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-margin-x"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.ui.margin_x,
                    0,
                    30,
                    5,
                    |x| format!("{x}px"),
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.ui.margin_x = val.as_int().unwrap();
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-margin-y"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.ui.margin_y,
                    0,
                    30,
                    5,
                    |x| format!("{x}px"),
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.ui.margin_y = val.as_int().unwrap();
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-list-margin"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.ui.list_margin,
                    0,
                    30,
                    5,
                    |x| format!("{x}px"),
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.ui.list_margin = val.as_int().unwrap();
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-padding-x"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.ui.padding_x,
                    0,
                    30,
                    5,
                    |x| format!("{x}px"),
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.ui.padding_x = val.as_int().unwrap();
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-padding-y"),
                Box::new(Number::new(
                    Point::zero(),
                    stylesheet.ui.padding_y,
                    0,
                    30,
                    5,
                    |x| format!("{x}px"),
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.ui.padding_y = val.as_int().unwrap();
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-foreground-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.ui.text_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.ui.text_color = val.as_color().unwrap();
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-background-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.ui.background_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.ui.background_color = val.as_color().unwrap();
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-highlight-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.ui.highlight_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.ui.highlight_color = val.as_color().unwrap();
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-highlight-text-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.ui.highlight_text_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.ui.highlight_text_color = val.as_color().unwrap();
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-disabled-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.ui.disabled_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.ui.disabled_color = val.as_color().unwrap();
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-tab-font-size"),
                Box::new(Percentage::new(
                    Point::zero(),
                    (stylesheet.ui.tab_font_size * 100.0) as i32,
                    0,
                    200,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.ui.tab_font_size = val.as_int().unwrap() as f32 / 100.0;
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-tab-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.ui.tab_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.ui.tab_color = val.as_color().unwrap();
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-tab-selected-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.ui.tab_selected_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.ui.tab_selected_color = val.as_color().unwrap();
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-status-bar-font-size"),
                Box::new(Percentage::new(
                    Point::zero(),
                    (stylesheet.status_bar.font_size * 100.0) as i32,
                    0,
                    200,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.status_bar.font_size = val.as_int().unwrap() as f32 / 100.0;
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-status-bar-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.status_bar.text_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.status_bar.text_color = val.as_color().unwrap();
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-status-backdrop"),
                Box::new(Toggle::new(
                    Point::zero(),
                    stylesheet.status_bar.status_backdrop,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, _val, _commands| {
                    stylesheet.status_bar.status_backdrop = !stylesheet.status_bar.status_backdrop;
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-status-backdrop-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.status_bar.status_backdrop_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.status_bar.status_backdrop_color = val.as_color().unwrap();
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-button-hint-font-size"),
                Box::new(Percentage::new(
                    Point::zero(),
                    (stylesheet.button_hints.button_hint_font_size * 100.0) as i32,
                    0,
                    200,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.button_hints.button_hint_font_size =
                        val.as_int().unwrap() as f32 / 100.0;
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-button-size"),
                Box::new(Percentage::new(
                    Point::zero(),
                    (stylesheet.button_hints.button_size * 100.0) as i32,
                    0,
                    200,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.button_hints.button_size = val.as_int().unwrap() as f32 / 100.0;
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-button-text-font-size"),
                Box::new(Percentage::new(
                    Point::zero(),
                    (stylesheet.button_hints.button_text_font_size * 100.0) as i32,
                    0,
                    200,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.button_hints.button_text_font_size =
                        val.as_int().unwrap() as f32 / 100.0;
                    Ok(true)
                }),
            ),
            (
                locale.t("settings-theme-button-a-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_hints.button_a_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.button_hints.button_a_color = val.as_color().unwrap();
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-button-b-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_hints.button_b_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.button_hints.button_b_color = val.as_color().unwrap();
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-button-x-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_hints.button_x_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.button_hints.button_x_color = val.as_color().unwrap();
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-button-y-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_hints.button_y_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.button_hints.button_y_color = val.as_color().unwrap();
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-button-text-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_hints.button_text_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.button_hints.button_text_color = val.as_color().unwrap();
                    Ok(false)
                }),
            ),
            (
                locale.t("settings-theme-button-hint-text-color"),
                Box::new(ColorPicker::new(
                    Point::zero(),
                    stylesheet.button_hints.text_color,
                    Alignment::Right,
                )),
                Box::new(|stylesheet, _ctx, val, _commands| {
                    stylesheet.button_hints.text_color = val.as_color().unwrap();
                    Ok(false)
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

        let mut button_hints = ButtonHints::new(
            res.clone(),
            vec![ButtonHint::new(
                res.clone(),
                Point::zero(),
                Key::X,
                locale.t("button-restore-defaults"),
                Alignment::Left,
            )],
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
        );

        let button_hints_rect = button_hints.bounding_box(&styles);
        let list_height = (button_hints_rect.y - y) as u32;

        let mut list = SettingsList::new(
            res.clone(),
            Rect::new(
                x + styles.ui.margin_x,
                y,
                w - styles.ui.margin_x as u32 * 2,
                list_height,
            ),
            left,
            right,
            res.get::<Stylesheet>().ui.ui_font.size + styles.ui.padding_y as u32,
        );
        if let Some(state) = state {
            list.select(state.selected);
        }

        drop(styles);
        drop(locale);

        Self {
            rect,
            stylesheet,
            context,
            list,
            handlers,
            button_hints,
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

        drawn |= self.list.should_draw() && self.list.draw(display, styles)?;
        drawn |= self.button_hints.should_draw() && self.button_hints.draw(display, styles)?;

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
                    let needs_relayout =
                        self.handlers[i](&mut self.stylesheet, &self.context, val, &commands)?;

                    self.stylesheet.save()?;
                    commands
                        .send(Command::ReloadStylesheet(
                            Box::new(self.stylesheet.clone()),
                            needs_relayout,
                        ))
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
                            .send(Command::ReloadStylesheet(
                                Box::new(self.stylesheet.clone()),
                                true,
                            ))
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
