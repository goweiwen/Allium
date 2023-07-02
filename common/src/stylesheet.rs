use std::fs::{self, File};
use std::io::Write;
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
    pub enable_box_art: bool,
    pub foreground_color: Color,
    pub background_color: Color,
    pub highlight_color: Color,
    pub disabled_color: Color,
    pub button_a_color: Color,
    pub button_b_color: Color,
    pub button_x_color: Color,
    pub button_y_color: Color,
    #[serde(default = "StylesheetFont::ui_font")]
    pub ui_font: StylesheetFont,
    #[serde(default = "StylesheetFont::guide_font")]
    pub guide_font: StylesheetFont,
    #[serde(skip, default = "StylesheetFont::cjk_font")]
    pub cjk_font: StylesheetFont,
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
}

impl Default for Stylesheet {
    fn default() -> Self {
        Self {
            enable_box_art: true,
            foreground_color: Color::new(255, 255, 255),
            background_color: Color::new(0, 0, 0),
            highlight_color: Color::new(151, 135, 187),
            disabled_color: Color::new(75, 75, 75),
            button_a_color: Color::new(235, 26, 29),
            button_b_color: Color::new(254, 206, 21),
            button_x_color: Color::new(7, 73, 180),
            button_y_color: Color::new(0, 141, 69),
            ui_font: StylesheetFont::ui_font(),
            guide_font: StylesheetFont::guide_font(),
            cjk_font: StylesheetFont::cjk_font(),
        }
    }
}
