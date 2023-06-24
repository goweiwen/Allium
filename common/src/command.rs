use crate::display::color::Color;
use crate::locale::LocaleSettings;
use crate::{display::settings::DisplaySettings, stylesheet::Stylesheet};

#[derive(Debug)]
pub enum Command {
    Exit,
    Exec(std::process::Command),
    SaveStylesheet(Box<Stylesheet>),
    SaveDisplaySettings(Box<DisplaySettings>),
    SaveLocaleSettings(LocaleSettings),
    CloseView,
    ValueChanged(usize, Value),
    TrapFocus,
    Unfocus,
    Redraw,
}

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Int(i32),
    String(String),
    Color(Color),
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
}

impl Default for Value {
    fn default() -> Self {
        Self::Bool(false)
    }
}
