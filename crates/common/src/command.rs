use std::time::Duration;

use image::{ImageBuffer, Rgba};

use crate::display::color::Color;
use crate::locale::LocaleSettings;
use crate::retroarch::RetroArchCommand;
use crate::{display::settings::DisplaySettings, stylesheet::Stylesheet};

#[derive(Debug)]
pub enum Command {
    Exit,
    Exec(std::process::Command),
    ReloadStylesheet(Box<Stylesheet>),
    SaveDisplaySettings(Box<DisplaySettings>),
    SaveLocaleSettings(LocaleSettings),
    CloseView,
    ValueChanged(usize, Value),
    TrapFocus,
    Unfocus,
    Redraw,
    StartSearch,
    Search(String),
    Toast(String, Option<Duration>),
    ImageToast(ImageBuffer<Rgba<u8>, Vec<u8>>, String, Option<Duration>),
    DismissToast,
    PopulateDb,
    SaveStateScreenshot {
        path: String,
        core: String,
        slot: i8,
    },
    RetroArchCommand(RetroArchCommand),
}

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Int(i32),
    String(String),
    Color(Color),
    DateTime(chrono::NaiveDateTime),
}

impl Value {
    pub fn as_bool(self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_int(self) -> Option<i32> {
        match self {
            Value::Int(i) => Some(i),
            _ => None,
        }
    }

    pub fn as_string(self) -> Option<String> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_color(self) -> Option<Color> {
        match self {
            Value::Color(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_datetime(self) -> Option<chrono::NaiveDateTime> {
        match self {
            Value::DateTime(dt) => Some(dt),
            _ => None,
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::Bool(false)
    }
}
