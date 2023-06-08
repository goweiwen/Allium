use embedded_graphics::pixelcolor::{raw::RawU24, Rgb888};
use embedded_graphics::prelude::{PixelColor, RgbColor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Color(Rgb888);

impl Color {
    #[inline]
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self(Rgb888::new(r, g, b))
    }

    #[inline]
    pub fn r(&self) -> u8 {
        self.0.r()
    }

    #[inline]
    pub fn g(&self) -> u8 {
        self.0.g()
    }

    #[inline]
    pub fn b(&self) -> u8 {
        self.0.b()
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (r, g, b) = (self.r(), self.g(), self.b());
        let hex = format!("#{:02x}{:02x}{:02x}", r, g, b);
        serializer.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let hex = String::deserialize(deserializer)?;
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).map_err(serde::de::Error::custom)?;
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(serde::de::Error::custom)?;
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(serde::de::Error::custom)?;
        Ok(Color(Rgb888::new(r, g, b)))
    }
}

impl PixelColor for Color {
    type Raw = RawU24;
}

impl From<Rgb888> for Color {
    fn from(rgb: Rgb888) -> Self {
        Color(rgb)
    }
}

impl From<Color> for Rgb888 {
    fn from(color: Color) -> Self {
        color.0
    }
}

impl From<RawU24> for Color {
    fn from(raw: RawU24) -> Self {
        Color(Rgb888::from(raw))
    }
}
