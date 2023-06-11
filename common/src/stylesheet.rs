use std::{
    fs::{self, File},
    io::Write,
};

use anyhow::Result;
use rusttype::Font;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};

use crate::{constants::ALLIUM_STYLESHEET, display::color::Color};

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
    pub ui_font_size: u32,
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
        Ok(())
    }

    fn font() -> Font<'static> {
        trace!("loading font");
        Font::try_from_bytes(include_bytes!("../../assets/font/Lato/Lato-Bold.ttf")).unwrap()
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
            ui_font_size: 24,
        }
    }
}
