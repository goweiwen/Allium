use std::{
    fs::{self, File},
    io::Write,
};

use anyhow::Result;
use rusttype::Font;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};

use crate::{constants::ALLIUM_STYLESHEET, display::color::Color};

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
        }
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
    #[serde(skip, default = "Stylesheet::font")]
    pub ui_font: Font<'static>,
    #[serde(skip, default = "Stylesheet::font_size")]
    pub ui_font_size: u32,
    #[serde(skip, default = "Stylesheet::mono_font")]
    pub mono_font: Font<'static>,
    #[serde(skip, default = "Stylesheet::mono_font_size")]
    pub mono_font_size: u32,
}

impl Stylesheet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load() -> Result<Self> {
        if ALLIUM_STYLESHEET.exists() {
            debug!("found state, loading from file");
            if let Ok(json) = fs::read_to_string(ALLIUM_STYLESHEET.as_path()) {
                if let Ok(json) = serde_json::from_str(&json) {
                    return Ok(json);
                }
            }
            warn!("failed to read state file, removing");
            fs::remove_file(ALLIUM_STYLESHEET.as_path())?;
        }
        Ok(Self::new())
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

    fn font() -> Font<'static> {
        trace!("loading font");
        Font::try_from_bytes(include_bytes!("../../assets/font/Nunito/Nunito-Bold.ttf")).unwrap()
    }

    fn font_size() -> u32 {
        32
    }

    fn mono_font() -> Font<'static> {
        trace!("loading mono font");
        Font::try_from_bytes(include_bytes!(
            "../../assets/font/Martian Mono/MartianMono_Condensed-Regular.ttf"
        ))
        .unwrap()
    }

    fn mono_font_size() -> u32 {
        16
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
            ui_font: Self::font(),
            ui_font_size: Self::font_size(),
            mono_font: Self::mono_font(),
            mono_font_size: Self::mono_font_size(),
        }
    }
}
