use std::fs::{self, File};
use std::io::Write;
use std::mem;
use std::path::PathBuf;

use anyhow::Result;
use log::{debug, error, warn};
use rusttype::Font;
use serde::{Deserialize, Serialize};

use crate::{
    constants::{ALLIUM_FONTS_DIR, ALLIUM_STYLESHEET},
    display::color::Color,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StylesheetColor {
    Foreground,
    Background,
    Highlight,
    Disabled,
    ButtonA,
    ButtonB,
    ButtonX,
    ButtonY,
    BackgroundHighlightBlend,
}

impl StylesheetColor {
    pub fn to_color(&self, stylesheet: &Stylesheet) -> Color {
        match self {
            Self::Foreground => stylesheet.foreground_color,
            Self::Background => stylesheet.background_color,
            Self::Highlight => stylesheet.highlight_color,
            Self::Disabled => stylesheet.disabled_color,
            Self::ButtonA => stylesheet.button_a_color,
            Self::ButtonB => stylesheet.button_b_color,
            Self::ButtonX => stylesheet.button_x_color,
            Self::ButtonY => stylesheet.button_y_color,
            Self::BackgroundHighlightBlend => stylesheet
                .background_color
                .blend(stylesheet.highlight_color, 128),
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

                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with('.') {
                        return None;
                    }
                }

                if let Some(ext) = path.extension() {
                    if ext == "ttf" || ext == "otf" || ext == "ttc" {
                        return Some(path);
                    }
                }
                None
            })
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
        Self::new(PathBuf::from("/customer/app/wqy-microhei.ttc"), 32)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stylesheet {
    pub wallpaper: Option<PathBuf>,
    pub enable_box_art: bool,
    pub show_battery_level: bool,
    #[serde(default = "Stylesheet::default_foreground_color")]
    pub foreground_color: Color,
    #[serde(default = "Stylesheet::default_background_color")]
    pub background_color: Color,
    #[serde(default = "Stylesheet::default_highlight_color")]
    pub highlight_color: Color,
    #[serde(default = "Stylesheet::default_disabled_color")]
    pub disabled_color: Color,
    #[serde(default = "Stylesheet::default_button_a_color")]
    pub button_a_color: Color,
    #[serde(default = "Stylesheet::default_button_b_color")]
    pub button_b_color: Color,
    #[serde(default = "Stylesheet::default_button_x_color")]
    pub button_x_color: Color,
    #[serde(default = "Stylesheet::default_button_y_color")]
    pub button_y_color: Color,
    #[serde(default = "StylesheetFont::ui_font")]
    pub ui_font: StylesheetFont,
    #[serde(default = "StylesheetFont::guide_font")]
    pub guide_font: StylesheetFont,
    #[serde(skip, default = "StylesheetFont::cjk_font")]
    pub cjk_font: StylesheetFont,
    #[serde(default = "Stylesheet::default_title_font_size")]
    pub title_font_size: f32,
    #[serde(default = "Stylesheet::default_status_bar_font_size")]
    pub status_bar_font_size: f32,
    #[serde(default = "Stylesheet::default_button_hint_font_size")]
    pub button_hint_font_size: f32,
    #[serde(default = "Stylesheet::default_alt_foreground_color")]
    alt_foreground_color: Color,
    #[serde(default = "Stylesheet::default_alt_background_color")]
    alt_background_color: Color,
    #[serde(default = "Stylesheet::default_alt_highlight_color")]
    alt_highlight_color: Color,
    #[serde(default = "Stylesheet::default_alt_disabled_color")]
    alt_disabled_color: Color,
    #[serde(default = "Stylesheet::default_alt_button_a_color")]
    alt_button_a_color: Color,
    #[serde(default = "Stylesheet::default_alt_button_b_color")]
    alt_button_b_color: Color,
    #[serde(default = "Stylesheet::default_alt_button_x_color")]
    alt_button_x_color: Color,
    #[serde(default = "Stylesheet::default_alt_button_y_color")]
    alt_button_y_color: Color,
}

impl Stylesheet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load() -> Result<Self> {
        if ALLIUM_STYLESHEET.exists() {
            debug!("found state, loading from file");
            if let Ok(json) = fs::read_to_string(ALLIUM_STYLESHEET.as_path()) {
                if let Ok(mut styles) = serde_json::from_str::<Self>(&json) {
                    styles.load_fonts()?;
                    return Ok(styles);
                }
            }
            warn!("failed to read state file, removing");
            fs::remove_file(ALLIUM_STYLESHEET.as_path())?;
        }

        let mut styles = Self::default();
        styles.load_fonts()?;
        Ok(styles)
    }

    pub fn load_fonts(&mut self) -> Result<()> {
        if let Err(e) = self.ui_font.load() {
            error!(
                "failed to load UI font: {}, {}",
                self.ui_font.path.display(),
                e
            );
            self.ui_font = StylesheetFont::ui_font();
            self.ui_font.load()?;
        }
        if let Err(e) = self.guide_font.load() {
            error!(
                "failed to load guide font: {} ({})",
                self.guide_font.path.display(),
                e
            );
            self.guide_font = StylesheetFont::guide_font();
            self.guide_font.load()?;
        }
        if let Err(e) = self.cjk_font.load() {
            error!(
                "failed to load CJK font: {} ({})",
                self.cjk_font.path.display(),
                e
            );
            self.cjk_font = StylesheetFont::guide_font();
            self.cjk_font.load()?;
        }
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string(&self).unwrap();
        File::create(ALLIUM_STYLESHEET.as_path())?.write_all(json.as_bytes())?;
        if let Err(e) = self.patch_ra_config() {
            warn!("failed to patch RA config: {}", e);
        }
        Ok(())
    }

    pub fn toggle_dark_mode(&mut self) {
        mem::swap(&mut self.foreground_color, &mut self.alt_foreground_color);
        mem::swap(&mut self.background_color, &mut self.alt_background_color);
        mem::swap(&mut self.highlight_color, &mut self.alt_highlight_color);
        mem::swap(&mut self.disabled_color, &mut self.alt_disabled_color);
        mem::swap(&mut self.button_a_color, &mut self.alt_button_a_color);
        mem::swap(&mut self.button_b_color, &mut self.alt_button_b_color);
        mem::swap(&mut self.button_x_color, &mut self.alt_button_x_color);
        mem::swap(&mut self.button_y_color, &mut self.alt_button_y_color);
    }

    pub fn toggle_battery_percentage(&mut self) {
        self.show_battery_level = !self.show_battery_level;
    }

    fn patch_ra_config(&self) -> Result<()> {
        let mut file = File::create("/mnt/SDCARD/RetroArch/.retroarch/assets/rgui/Allium.cfg")?;
        write!(
            file,
            r#"rgui_entry_normal_color = "0xFF{foreground:X}"
rgui_entry_hover_color = "0xFF{highlight:X}"
rgui_title_color = "0xFF{highlight:X}"
rgui_bg_dark_color = "0xFF{background:X}"
rgui_bg_light_color = "0xFF{background:X}"
rgui_border_dark_color = "0xFF{background:X}"
rgui_border_light_color = "0xFF{background:X}"
rgui_shadow_color = "0xFF{background:X}"
rgui_particle_color = "0xFF{highlight:X}"
"#,
            foreground = self.foreground_color,
            highlight = self.highlight_color,
            background = self.background_color
        )?;
        Ok(())
    }

    #[inline]
    fn default_title_font_size() -> f32 {
        1.3
    }

    #[inline]
    fn default_status_bar_font_size() -> f32 {
        1.0
    }

    #[inline]
    fn default_button_hint_font_size() -> f32 {
        1.0
    }

    #[inline]
    fn default_foreground_color() -> Color {
        Color::new(255, 255, 255)
    }

    #[inline]
    fn default_background_color() -> Color {
        Color::rgba(0, 0, 0, 0)
    }

    #[inline]
    fn default_highlight_color() -> Color {
        Color::new(114, 135, 253)
    }

    #[inline]
    fn default_disabled_color() -> Color {
        Color::new(88, 91, 112)
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
    fn default_alt_foreground_color() -> Color {
        Color::new(41, 44, 60)
    }

    #[inline]
    fn default_alt_background_color() -> Color {
        Color::new(239, 241, 245)
    }

    #[inline]
    fn default_alt_highlight_color() -> Color {
        Color::new(114, 135, 253)
    }

    #[inline]
    fn default_alt_disabled_color() -> Color {
        Color::new(124, 127, 147)
    }

    #[inline]
    fn default_alt_button_a_color() -> Color {
        Color::new(243, 139, 168)
    }

    #[inline]
    fn default_alt_button_b_color() -> Color {
        Color::new(249, 226, 175)
    }

    #[inline]
    fn default_alt_button_x_color() -> Color {
        Color::new(137, 180, 250)
    }

    #[inline]
    fn default_alt_button_y_color() -> Color {
        Color::new(148, 226, 213)
    }
}

impl Default for Stylesheet {
    fn default() -> Self {
        Self {
            wallpaper: None,
            enable_box_art: true,
            show_battery_level: false,
            foreground_color: Self::default_foreground_color(),
            background_color: Self::default_background_color(),
            highlight_color: Self::default_highlight_color(),
            disabled_color: Self::default_disabled_color(),
            button_a_color: Self::default_button_a_color(),
            button_b_color: Self::default_button_b_color(),
            button_x_color: Self::default_button_x_color(),
            button_y_color: Self::default_button_y_color(),
            ui_font: StylesheetFont::ui_font(),
            guide_font: StylesheetFont::guide_font(),
            cjk_font: StylesheetFont::cjk_font(),
            title_font_size: Self::default_title_font_size(),
            status_bar_font_size: Self::default_status_bar_font_size(),
            button_hint_font_size: Self::default_button_hint_font_size(),
            alt_foreground_color: Self::default_alt_foreground_color(),
            alt_background_color: Self::default_alt_background_color(),
            alt_highlight_color: Self::default_alt_highlight_color(),
            alt_disabled_color: Self::default_alt_disabled_color(),
            alt_button_a_color: Self::default_alt_button_a_color(),
            alt_button_b_color: Self::default_alt_button_b_color(),
            alt_button_x_color: Self::default_alt_button_x_color(),
            alt_button_y_color: Self::default_alt_button_y_color(),
        }
    }
}
