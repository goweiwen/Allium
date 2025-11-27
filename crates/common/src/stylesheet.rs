use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use itertools::Itertools;
use log::{debug, error, warn};
use rusttype::Font;
use serde::{Deserialize, Serialize};

use crate::constants::ALLIUM_SD_ROOT;
use crate::{
    constants::{ALLIUM_FONTS_DIR, ALLIUM_THEME_STATE, ALLIUM_THEMES_DIR},
    display::color::Color,
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum StylesheetColor {
    Foreground,
    Background,
    Highlight,
    HighlightText,
    Disabled,
    Tab,
    TabSelected,
    ButtonA,
    ButtonB,
    ButtonX,
    ButtonY,
    ButtonBackground,
    ButtonText,
    ButtonHintText,
    BackgroundHighlightBlend,
    Stroke,
    HighlightTextStroke,
    TabStroke,
    TabSelectedStroke,
    StatusBar,
    StatusBarStroke,
    MenuBackground,
}

impl StylesheetColor {
    pub fn to_color(&self, stylesheet: &Stylesheet) -> Color {
        match self {
            Self::Foreground => stylesheet.ui.text_color,
            Self::Background => stylesheet.ui.background_color,
            Self::Highlight => stylesheet.ui.highlight_color,
            Self::HighlightText => stylesheet.ui.highlight_text_color,
            Self::Disabled => stylesheet.ui.disabled_color,
            Self::Tab => stylesheet.ui.tab_color,
            Self::TabSelected => stylesheet.ui.tab_selected_color,
            Self::ButtonA => stylesheet.button_hints.button_a_color,
            Self::ButtonB => stylesheet.button_hints.button_b_color,
            Self::ButtonX => stylesheet.button_hints.button_x_color,
            Self::ButtonY => stylesheet.button_hints.button_y_color,
            Self::ButtonBackground => stylesheet.button_hints.button_bg_color,
            Self::ButtonText => stylesheet.button_hints.button_text_color,
            Self::ButtonHintText => stylesheet.button_hints.text_color,
            Self::BackgroundHighlightBlend => stylesheet
                .ui
                .background_color
                .blend(stylesheet.ui.highlight_color, 128),
            Self::Stroke => stylesheet.ui.text_stroke_color,
            Self::HighlightTextStroke => stylesheet.ui.highlight_text_stroke_color,
            Self::TabStroke => stylesheet.ui.tab_stroke_color,
            Self::TabSelectedStroke => stylesheet.ui.tab_selected_stroke_color,
            Self::StatusBar => stylesheet.status_bar.text_color,
            Self::StatusBarStroke => stylesheet.status_bar.text_stroke_color,
            Self::MenuBackground => stylesheet.menu.background_color,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylesheetFont {
    pub path: PathBuf,
    pub size: u32,
    #[serde(skip)]
    pub font: Option<Font<'static>>,
}

impl StylesheetFont {
    pub fn new(path: PathBuf, size: u32) -> Self {
        Self {
            path,
            size,
            font: None,
        }
    }

    /// Returns an owned font. Panics if the font has not been loaded.
    pub fn font(&self) -> Font<'static> {
        self.font.as_ref().unwrap().clone()
    }

    /// Loads the font from disk if it has not already been loaded.
    pub fn load(&mut self) -> Result<()> {
        let bytes = fs::read(&self.path)?;
        self.font = Font::try_from_vec(bytes);
        if self.font.is_none() {
            error!("failed to load font from {:?}", self.path);
        }
        Ok(())
    }

    pub fn available_fonts() -> Result<Vec<PathBuf>> {
        Ok(fs::read_dir(ALLIUM_FONTS_DIR.as_path())?
            .filter_map(|entry| {
                if let Err(e) = entry {
                    warn!("failed to read font directory: {}", e);
                    return None;
                }

                let entry = entry.unwrap();
                let path = entry.path();

                if let Some(name) = path.file_name()
                    && name.to_string_lossy().starts_with('.')
                {
                    return None;
                }

                if let Some(ext) = path.extension()
                    && (ext == "ttf" || ext == "otf" || ext == "ttc")
                {
                    return path.file_name().map(PathBuf::from);
                }
                None
            })
            .sorted_unstable()
            .collect())
    }

    /// Default UI font.
    pub fn ui_font() -> Self {
        Self::new(ALLIUM_FONTS_DIR.join("Nunito.ttf"), 36)
    }

    /// Default guide font.
    pub fn guide_font() -> Self {
        Self::new(ALLIUM_FONTS_DIR.join("Nunito.ttf"), 28)
    }

    /// Default CJK font.
    pub fn cjk_font() -> Self {
        Self::new(ALLIUM_FONTS_DIR.join("NotoSansCJK.otf"), 32)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme(pub String);

impl Theme {
    pub fn load() -> Self {
        let theme = fs::read_to_string(ALLIUM_THEME_STATE.as_path())
            .unwrap_or_else(|_| "Allium".to_owned());

        if let Ok(themes) = Stylesheet::available_themes()
            && themes.contains(&theme)
        {
            return Self(theme);
        }

        Self("Allium".to_owned())
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = ALLIUM_THEME_STATE.parent() {
            fs::create_dir_all(parent)?;
        }
        File::create(ALLIUM_THEME_STATE.as_path())?.write_all(self.0.as_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylesheetUi {
    #[serde(default = "Stylesheet::default_margin_x")]
    pub margin_x: i32,
    #[serde(default = "Stylesheet::default_margin_y")]
    pub margin_y: i32,
    #[serde(default = "Stylesheet::default_list_margin")]
    pub list_margin: i32,
    #[serde(default = "Stylesheet::default_padding_x")]
    pub padding_x: i32,
    #[serde(default = "Stylesheet::default_padding_y")]
    pub padding_y: i32,
    #[serde(default = "StylesheetFont::ui_font")]
    pub ui_font: StylesheetFont,
    #[serde(default = "Stylesheet::default_text_color")]
    pub text_color: Color,
    #[serde(default = "Stylesheet::default_text_stroke_color")]
    pub text_stroke_color: Color,
    #[serde(default = "Stylesheet::default_background_color")]
    pub background_color: Color,
    #[serde(default = "Stylesheet::default_highlight_color")]
    pub highlight_color: Color,
    #[serde(default = "Stylesheet::default_highlight_text_color")]
    pub highlight_text_color: Color,
    #[serde(default = "Stylesheet::default_highlight_text_stroke_color")]
    pub highlight_text_stroke_color: Color,
    #[serde(default = "Stylesheet::default_disabled_color")]
    pub disabled_color: Color,
    #[serde(default = "Stylesheet::default_tab_font_size")]
    pub tab_font_size: f32,
    #[serde(default = "Stylesheet::default_tab_color")]
    pub tab_color: Color,
    #[serde(default = "Stylesheet::default_tab_stroke_color")]
    pub tab_stroke_color: Color,
    #[serde(default = "Stylesheet::default_tab_selected_color")]
    pub tab_selected_color: Color,
    #[serde(default = "Stylesheet::default_tab_selected_stroke_color")]
    pub tab_selected_stroke_color: Color,
    #[serde(default = "Stylesheet::default_stroke_width")]
    pub stroke_width: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylesheetStatusBar {
    #[serde(default)]
    pub show_battery_level: bool,
    #[serde(default = "Stylesheet::default_show_clock")]
    pub show_clock: bool,
    #[serde(default)]
    pub show_wifi: bool,
    #[serde(default = "Stylesheet::default_status_bar_font_size")]
    pub font_size: f32,
    #[serde(default = "Stylesheet::default_status_bar_color")]
    pub text_color: Color,
    #[serde(default = "Stylesheet::default_status_bar_stroke_color")]
    pub text_stroke_color: Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylesheetButtonHints {
    #[serde(default = "Stylesheet::default_button_hint_font_size")]
    pub button_hint_font_size: f32,
    #[serde(default = "Stylesheet::default_button_size")]
    pub button_size: f32,
    #[serde(default = "Stylesheet::default_button_text_font_size")]
    pub button_text_font_size: f32,
    #[serde(default = "Stylesheet::default_button_a_color")]
    pub button_a_color: Color,
    #[serde(default = "Stylesheet::default_button_b_color")]
    pub button_b_color: Color,
    #[serde(default = "Stylesheet::default_button_x_color")]
    pub button_x_color: Color,
    #[serde(default = "Stylesheet::default_button_y_color")]
    pub button_y_color: Color,
    #[serde(default = "Stylesheet::default_button_bg_color")]
    pub button_bg_color: Color,
    #[serde(default = "Stylesheet::default_button_text_color")]
    pub button_text_color: Color,
    #[serde(default = "Stylesheet::default_button_hint_text_color")]
    pub text_color: Color,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StylesheetRecents {
    #[serde(default)]
    pub use_recents_carousel: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylesheetGames {
    #[serde(default = "Stylesheet::default_boxart_width")]
    pub boxart_width: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylesheetMenu {
    #[serde(default = "Stylesheet::default_menu_background_color")]
    pub background_color: Color,
    #[serde(default = "StylesheetFont::guide_font")]
    pub guide_font: StylesheetFont,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stylesheet {
    pub wallpaper: Option<PathBuf>,
    #[serde(default = "StylesheetUi::default")]
    pub ui: StylesheetUi,
    #[serde(default = "StylesheetStatusBar::default")]
    pub status_bar: StylesheetStatusBar,
    #[serde(default = "StylesheetButtonHints::default")]
    pub button_hints: StylesheetButtonHints,
    #[serde(default = "StylesheetRecents::default")]
    pub recents: StylesheetRecents,
    #[serde(default = "StylesheetGames::default")]
    pub games: StylesheetGames,
    #[serde(default = "StylesheetMenu::default")]
    pub menu: StylesheetMenu,
    #[serde(skip, default = "StylesheetFont::cjk_font")]
    pub cjk_font: StylesheetFont,
}

impl Stylesheet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn available_themes() -> Result<Vec<String>> {
        if !ALLIUM_THEMES_DIR.exists() {
            return Ok(Vec::new());
        }

        Ok(fs::read_dir(ALLIUM_THEMES_DIR.as_path())?
            .filter_map(|entry| {
                if let Err(e) = entry {
                    warn!("failed to read themes directory: {}", e);
                    return None;
                }

                let entry = entry.unwrap();
                let path = entry.path();

                // Skip hidden directories
                if let Some(name) = path.file_name()
                    && name.to_string_lossy().starts_with('.')
                {
                    return None;
                }

                // Check if it's a directory and contains stylesheet.json
                if path.is_dir() && path.join("stylesheet.json").exists() {
                    return path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string());
                }
                None
            })
            .sorted_unstable()
            .collect())
    }

    pub fn load_from_theme(theme: &Theme) -> Result<Self> {
        let stylesheet_path = ALLIUM_THEMES_DIR.join(&theme.0).join("stylesheet.json");
        if !stylesheet_path.exists() {
            return Err(anyhow::anyhow!(
                "Theme '{}' does not have a stylesheet.json",
                theme.0
            ));
        }

        debug!("loading theme from {}", stylesheet_path.display());
        let json = fs::read_to_string(&stylesheet_path)?;
        let mut styles = serde_json::from_str::<Self>(&json)?;

        #[cfg(feature = "simulator")]
        {
            // Write default missing fields to original stylesheet.json
            if let Ok(file) = File::create(
                PathBuf::from("../Allium-Themes/Themes")
                    .join(&theme.0)
                    .join("stylesheet.json"),
            ) {
                serde_json::to_writer_pretty(&file, &styles)?;
            }
        }

        // Load override file if it exists
        let override_path = ALLIUM_THEMES_DIR
            .join(&theme.0)
            .join("stylesheet.override.json");
        if override_path.exists() {
            debug!("loading theme overrides from {}", override_path.display());
            if let Ok(override_json) = fs::read_to_string(&override_path)
                && let Ok(override_styles) = serde_json::from_str::<Self>(&override_json)
            {
                styles.merge(override_styles);
            }
        }

        styles.load_fonts()?;
        Ok(styles)
    }

    /// Merge another stylesheet into this one, taking values from `other` where present
    pub fn merge(&mut self, other: Self) {
        if other.wallpaper.is_some() {
            self.wallpaper = other.wallpaper;
        }
        self.ui = other.ui;
        self.status_bar = other.status_bar;
        self.button_hints = other.button_hints;
        self.recents = other.recents;
        self.games = other.games;
        self.menu = other.menu;
    }

    pub fn load() -> Result<Self> {
        let theme = Theme::load();

        // Try loading from the theme
        if let Ok(styles) = Self::load_from_theme(&theme) {
            return Ok(styles);
        }

        // Fall back to built-in defaults
        debug!("using built-in default stylesheet");
        let mut styles = Self::default();
        styles.load_fonts()?;
        Ok(styles)
    }

    pub fn load_fonts(&mut self) -> Result<()> {
        let theme_dir = ALLIUM_THEMES_DIR.join(Theme::load().0);
        let resolve_font_path = |font: &PathBuf| -> PathBuf {
            if font.is_absolute() && font.exists() {
                debug!("using absolute font path: {}", font.display());
                return font.clone();
            }
            let theme_font = theme_dir.join(font);
            if theme_font.exists() {
                debug!("using theme font path: {}", theme_font.display());
                return theme_font;
            }
            debug!("using default font path: {}", font.display());
            ALLIUM_FONTS_DIR.join(font)
        };

        self.ui.ui_font.path = resolve_font_path(&self.ui.ui_font.path);
        if let Err(e) = self.ui.ui_font.load() {
            error!(
                "failed to load UI font: {}, {}",
                self.ui.ui_font.path.display(),
                e
            );
            self.ui.ui_font = StylesheetFont::ui_font();
            self.ui.ui_font.load()?;
        }

        self.menu.guide_font.path = resolve_font_path(&self.menu.guide_font.path);
        if let Err(e) = self.menu.guide_font.load() {
            error!(
                "failed to load guide font: {} ({})",
                self.menu.guide_font.path.display(),
                e
            );
            self.menu.guide_font = StylesheetFont::guide_font();
            self.menu.guide_font.load()?;
        }

        self.cjk_font.path = resolve_font_path(&self.cjk_font.path);
        if let Err(e) = self.cjk_font.load() {
            error!(
                "failed to load CJK font: {} ({})",
                self.cjk_font.path.display(),
                e
            );
            self.cjk_font = StylesheetFont::cjk_font();
            self.cjk_font.load()?;
        }

        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let theme = Theme::load();
        let override_path = ALLIUM_THEMES_DIR
            .join(&theme.0)
            .join("stylesheet.override.json");
        debug!("saving stylesheet to {}", override_path.display());

        if let Some(parent) = override_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = File::create(&override_path)?;
        serde_json::to_writer_pretty(&file, &self)?;

        if let Err(e) = self.set_retroarch_theme() {
            warn!("failed to patch RA config: {}", e);
        }
        Ok(())
    }

    pub fn restore_defaults(&mut self) -> Result<()> {
        let theme = Theme::load();
        let override_path = ALLIUM_THEMES_DIR
            .join(&theme.0)
            .join("stylesheet.override.json");
        if override_path.exists() {
            debug!(
                "removing theme override file at {}",
                override_path.display()
            );
            fs::remove_file(&override_path)?;
        }

        // Reload the theme from defaults
        *self = Self::load_from_theme(&theme)?;

        Ok(())
    }

    pub fn toggle_battery_percentage(&mut self) {
        self.status_bar.show_battery_level = !self.status_bar.show_battery_level;
    }

    pub fn toggle_clock(&mut self) {
        self.status_bar.show_clock = !self.status_bar.show_clock;
    }

    pub fn toggle_wifi(&mut self) {
        self.status_bar.show_wifi = !self.status_bar.show_wifi;
    }

    #[inline]
    pub fn tab_font_size(&self) -> f32 {
        self.ui.ui_font.size as f32 * self.ui.tab_font_size
    }

    #[inline]
    pub fn button_hint_font_size(&self) -> f32 {
        self.ui.ui_font.size as f32 * self.button_hints.button_hint_font_size
    }

    #[inline]
    pub fn button_size(&self) -> f32 {
        self.button_hint_font_size() * self.button_hints.button_size
    }

    #[inline]
    pub fn button_text_font_size(&self) -> f32 {
        self.button_size() * self.button_hints.button_text_font_size
    }

    #[inline]
    pub fn status_bar_font_size(&self) -> f32 {
        self.ui.ui_font.size as f32 * self.status_bar.font_size
    }

    fn set_retroarch_theme(&self) -> Result<()> {
        let retroarch_wallpaper_path =
            PathBuf::from("/mnt/SDCARD/RetroArch/.retroarch/assets/rgui/Allium.png");
        if let Some(wallpaper) = &self.wallpaper {
            let path = Self::resolve_wallpaper(wallpaper);

            if !path.exists() {
                return Ok(());
            }

            let mut image = ::image::open(path)?;

            let (w, h) = (320, 240);
            if image.width() != w || image.height() != h {
                let new_h = h.min(w * image.height() / image.width());
                image = image.resize_to_fill(w, new_h, image::imageops::FilterType::Nearest);
            }

            let mut image = image.into_rgba8();

            let bg_color = self.ui.background_color;
            if bg_color.a() != 0 {
                for p in image.pixels_mut() {
                    let alpha = bg_color.a() as u32;
                    p[0] =
                        ((p[0] as u32 * (255 - alpha) + bg_color.r() as u32 * alpha) / 255) as u8;
                    p[1] =
                        ((p[1] as u32 * (255 - alpha) + bg_color.g() as u32 * alpha) / 255) as u8;
                    p[2] =
                        ((p[2] as u32 * (255 - alpha) + bg_color.b() as u32 * alpha) / 255) as u8;
                }
            }

            image.save(&retroarch_wallpaper_path)?;
        } else {
            let _ = fs::remove_file(retroarch_wallpaper_path);
        }

        let mut file = File::create("/mnt/SDCARD/RetroArch/.retroarch/assets/rgui/Allium.cfg")?;
        write!(
            file,
            r#"rgui_entry_normal_color = "0xFF{tab_color:X}"
rgui_entry_hover_color = "0xFF{tab_selected_color:X}"
rgui_title_color = "0xFF{highlight:X}"
rgui_bg_dark_color = "0xFF{background:X}"
rgui_bg_light_color = "0xFF{background:X}"
rgui_border_dark_color = "0xFF{background:X}"
rgui_border_light_color = "0xFF{background:X}"
rgui_shadow_color = "0xFF{background:X}"
rgui_particle_color = "0xFF{highlight:X}"
rgui_wallpaper = "/mnt/SDCARD/RetroArch/.retroarch/assets/rgui/Allium.png"
"#,
            tab_color = self.ui.tab_color,
            tab_selected_color = self.ui.tab_selected_color,
            // foreground = self.ui.text_color,
            highlight = self.ui.highlight_color,
            background = self.ui.background_color,
        )?;
        Ok(())
    }

    fn resolve_wallpaper(wallpaper: &Path) -> PathBuf {
        // If wallpaper path is absolute, use it as-is
        if wallpaper.is_absolute() {
            return wallpaper.to_path_buf();
        }

        // Load the current theme and check if wallpaper exists in theme directory
        let theme = crate::stylesheet::Theme::load();
        let theme_wallpaper = ALLIUM_THEMES_DIR.join(&theme.0).join(wallpaper);
        if theme_wallpaper.exists() {
            return theme_wallpaper;
        }

        // Fall back to SD root
        ALLIUM_SD_ROOT.join(wallpaper)
    }

    #[inline]
    fn default_tab_font_size() -> f32 {
        1.0
    }

    #[inline]
    fn default_status_bar_font_size() -> f32 {
        1.0
    }

    #[inline]
    fn default_button_hint_font_size() -> f32 {
        0.9
    }

    #[inline]
    fn default_button_size() -> f32 {
        1.0
    }

    #[inline]
    fn default_button_text_font_size() -> f32 {
        0.75
    }

    #[inline]
    fn default_margin_x() -> i32 {
        12
    }

    #[inline]
    fn default_margin_y() -> i32 {
        8
    }

    #[inline]
    fn default_list_margin() -> i32 {
        4
    }

    #[inline]
    fn default_padding_x() -> i32 {
        12
    }

    #[inline]
    fn default_padding_y() -> i32 {
        4
    }

    #[inline]
    fn default_boxart_width() -> u32 {
        250
    }

    #[inline]
    fn default_text_color() -> Color {
        Color::new(255, 255, 255)
    }

    #[inline]
    fn default_background_color() -> Color {
        Color::new(0, 0, 0)
    }

    #[inline]
    fn default_highlight_color() -> Color {
        Color::new(114, 135, 253)
    }

    #[inline]
    fn default_highlight_text_color() -> Color {
        Color::new(255, 255, 255)
    }

    #[inline]
    fn default_disabled_color() -> Color {
        Color::new(88, 91, 112)
    }

    #[inline]
    fn default_tab_color() -> Color {
        Color::rgba(255, 255, 255, 112)
    }

    #[inline]
    fn default_tab_selected_color() -> Color {
        Color::new(255, 255, 255)
    }

    #[inline]
    fn default_button_a_color() -> Color {
        Color::new(235, 26, 29)
    }

    #[inline]
    fn default_button_b_color() -> Color {
        Color::new(254, 206, 21)
    }

    #[inline]
    fn default_button_x_color() -> Color {
        Color::new(7, 73, 180)
    }

    #[inline]
    fn default_button_y_color() -> Color {
        Color::new(0, 141, 69)
    }

    #[inline]
    fn default_button_bg_color() -> Color {
        Stylesheet::default_disabled_color()
    }

    #[inline]
    fn default_button_text_color() -> Color {
        Stylesheet::default_text_color()
    }

    #[inline]
    fn default_button_hint_text_color() -> Color {
        Stylesheet::default_text_color()
    }

    #[inline]
    fn default_text_stroke_color() -> Color {
        Color::rgba(0, 0, 0, 0)
    }

    #[inline]
    fn default_highlight_text_stroke_color() -> Color {
        Color::rgba(0, 0, 0, 0)
    }

    #[inline]
    fn default_tab_stroke_color() -> Color {
        Color::rgba(0, 0, 0, 0)
    }

    #[inline]
    fn default_tab_selected_stroke_color() -> Color {
        Color::rgba(0, 0, 0, 0)
    }

    #[inline]
    fn default_status_bar_color() -> Color {
        Stylesheet::default_text_color()
    }

    #[inline]
    fn default_status_bar_stroke_color() -> Color {
        Color::rgba(0, 0, 0, 0)
    }

    #[inline]
    fn default_menu_background_color() -> Color {
        Stylesheet::default_background_color()
    }

    #[inline]
    fn default_stroke_width() -> u32 {
        0
    }

    #[inline]
    fn default_show_clock() -> bool {
        true
    }
}

impl Default for StylesheetUi {
    fn default() -> Self {
        Self {
            margin_x: Stylesheet::default_margin_x(),
            margin_y: Stylesheet::default_margin_y(),
            list_margin: Stylesheet::default_list_margin(),
            padding_x: Stylesheet::default_padding_x(),
            padding_y: Stylesheet::default_padding_y(),
            ui_font: StylesheetFont::ui_font(),
            text_color: Stylesheet::default_text_color(),
            text_stroke_color: Stylesheet::default_text_stroke_color(),
            background_color: Stylesheet::default_background_color(),
            highlight_color: Stylesheet::default_highlight_color(),
            highlight_text_color: Stylesheet::default_highlight_text_color(),
            highlight_text_stroke_color: Stylesheet::default_highlight_text_stroke_color(),
            disabled_color: Stylesheet::default_disabled_color(),
            tab_font_size: Stylesheet::default_tab_font_size(),
            tab_color: Stylesheet::default_tab_color(),
            tab_stroke_color: Stylesheet::default_tab_stroke_color(),
            tab_selected_color: Stylesheet::default_tab_selected_color(),
            tab_selected_stroke_color: Stylesheet::default_tab_selected_stroke_color(),
            stroke_width: Stylesheet::default_stroke_width(),
        }
    }
}

impl Default for StylesheetStatusBar {
    fn default() -> Self {
        Self {
            show_battery_level: false,
            show_clock: Stylesheet::default_show_clock(),
            show_wifi: false,
            font_size: Stylesheet::default_status_bar_font_size(),
            text_color: Stylesheet::default_status_bar_color(),
            text_stroke_color: Stylesheet::default_status_bar_stroke_color(),
        }
    }
}

impl Default for StylesheetButtonHints {
    fn default() -> Self {
        Self {
            button_hint_font_size: Stylesheet::default_button_hint_font_size(),
            button_size: Stylesheet::default_button_size(),
            button_text_font_size: Stylesheet::default_button_text_font_size(),
            button_a_color: Stylesheet::default_button_a_color(),
            button_b_color: Stylesheet::default_button_b_color(),
            button_x_color: Stylesheet::default_button_x_color(),
            button_y_color: Stylesheet::default_button_y_color(),
            button_bg_color: Stylesheet::default_button_bg_color(),
            button_text_color: Stylesheet::default_button_text_color(),
            text_color: Stylesheet::default_button_hint_text_color(),
        }
    }
}

impl Default for StylesheetGames {
    fn default() -> Self {
        Self {
            boxart_width: Stylesheet::default_boxart_width(),
        }
    }
}

impl Default for StylesheetMenu {
    fn default() -> Self {
        Self {
            background_color: Stylesheet::default_background_color(),
            guide_font: StylesheetFont::guide_font(),
        }
    }
}

impl Default for Stylesheet {
    fn default() -> Self {
        Self {
            wallpaper: None,
            ui: StylesheetUi::default(),
            status_bar: StylesheetStatusBar::default(),
            button_hints: StylesheetButtonHints::default(),
            recents: StylesheetRecents::default(),
            games: StylesheetGames::default(),
            menu: StylesheetMenu::default(),
            cjk_font: StylesheetFont::cjk_font(),
        }
    }
}
