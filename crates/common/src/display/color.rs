use std::fmt;

use bytemuck::{Pod, Zeroable};
use image::Rgba;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tiny_skia::PremultipliedColorU8;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Pod, Zeroable)]
pub struct Color(u32);

impl Color {
    #[inline]
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 0xFF)
    }

    #[inline]
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self(r as u32 | (g as u32) << 8 | (b as u32) << 16 | (a as u32) << 24)
    }

    #[inline]
    pub fn r(&self) -> u8 {
        self.0 as u8
    }

    #[inline]
    pub fn g(&self) -> u8 {
        (self.0 >> 8) as u8
    }

    #[inline]
    pub fn b(&self) -> u8 {
        (self.0 >> 16) as u8
    }

    #[inline]
    pub fn a(&self) -> u8 {
        (self.0 >> 24) as u8
    }

    #[inline]
    pub fn with_r(&self, r: u8) -> Self {
        Self((r as u32) | self.0 & 0xFFFFFF00)
    }

    #[inline]
    pub fn with_g(&self, g: u8) -> Self {
        Self((g as u32) << 8 | self.0 & 0xFFFF00FF)
    }

    #[inline]
    pub fn with_b(&self, b: u8) -> Self {
        Self((b as u32) << 16 | self.0 & 0xFF00FFFF)
    }

    #[inline]
    pub fn with_a(&self, a: u8) -> Self {
        Self((a as u32) << 24 | self.0 & 0x00FFFFFF)
    }

    pub fn char(&self, i: usize) -> String {
        format!(
            "{:X}",
            match i {
                0 => self.r() / 16,
                1 => self.r() % 16,
                2 => self.g() / 16,
                3 => self.g() % 16,
                4 => self.b() / 16,
                5 => self.b() % 16,
                6 => self.a() / 16,
                7 => self.a() % 16,
                _ => unreachable!(),
            }
        )
    }

    pub fn is_dark(&self) -> bool {
        self.r() < 128 && self.g() < 128 && self.b() < 128
    }

    pub fn invert(&self) -> Self {
        Self::new(255 - self.r(), 255 - self.g(), 255 - self.b())
    }

    pub fn blend(&self, other: Self, alpha: u8) -> Self {
        Self::new(
            ((self.r() as i32 * (255 - alpha as i32) + other.r() as i32 * alpha as i32) / 255)
                as u8,
            ((self.g() as i32 * (255 - alpha as i32) + other.g() as i32 * alpha as i32) / 255)
                as u8,
            ((self.b() as i32 * (255 - alpha as i32) + other.b() as i32 * alpha as i32) / 255)
                as u8,
        )
    }

    pub fn overlay(&self, other: Self) -> Self {
        Self::new(
            overlay(self.r(), other.r()),
            overlay(self.g(), other.g()),
            overlay(self.b(), other.b()),
        )
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (r, g, b, a) = (self.r(), self.g(), self.b(), self.a());
        let hex = if a < 255 {
            format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
        } else {
            format!("#{:02x}{:02x}{:02x}", r, g, b)
        };
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
        Ok(if hex.len() == 8 {
            let a = u8::from_str_radix(&hex[6..8], 16).map_err(serde::de::Error::custom)?;
            Color::rgba(r, g, b, a)
        } else {
            Color::new(r, g, b)
        })
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (r, g, b) = (self.r(), self.g(), self.b());
        write!(f, "#{:02x}{:02x}{:02x}", r, g, b)
    }
}

impl fmt::UpperHex for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (r, g, b) = (self.r(), self.g(), self.b());
        write!(f, "{:02X}{:02X}{:02X}", r, g, b)
    }
}

impl From<Color> for Rgba<u8> {
    fn from(color: Color) -> Self {
        Rgba([
            color.0 as u8,
            (color.0 >> 8) as u8,
            (color.0 >> 16) as u8,
            (color.0 >> 24) as u8,
        ])
    }
}

impl From<Color> for PremultipliedColorU8 {
    #[inline]
    fn from(color: Color) -> Self {
        let a = color.a();
        if a == 0 {
            PremultipliedColorU8::from_rgba(0, 0, 0, 0).unwrap()
        } else if a == 255 {
            // Zero-cost conversion: Color and PremultipliedColorU8 have identical RGBA layout
            bytemuck::cast(color)
        } else {
            // Premultiply RGB by alpha
            let r = ((color.r() as u16 * a as u16) / 255) as u8;
            let g = ((color.g() as u16 * a as u16) / 255) as u8;
            let b = ((color.b() as u16 * a as u16) / 255) as u8;
            PremultipliedColorU8::from_rgba(r, g, b, a).unwrap()
        }
    }
}

impl From<PremultipliedColorU8> for Color {
    #[inline]
    fn from(color: PremultipliedColorU8) -> Self {
        let a = color.alpha();
        if a == 0 {
            Self::rgba(0, 0, 0, 0)
        } else if a == 255 {
            // Zero-cost conversion: Color and PremultipliedColorU8 have identical RGBA layout
            bytemuck::cast(color)
        } else {
            // Un-premultiply RGB by alpha
            let r = ((color.red() as u16 * 255) / a as u16) as u8;
            let g = ((color.green() as u16 * 255) / a as u16) as u8;
            let b = ((color.blue() as u16 * 255) / a as u16) as u8;
            Self::rgba(r, g, b, a)
        }
    }
}

impl From<Color> for tiny_skia::Color {
    #[inline]
    fn from(color: Color) -> Self {
        tiny_skia::Color::from_rgba8(color.r(), color.g(), color.b(), color.a())
    }
}

fn overlay(a: u8, b: u8) -> u8 {
    if a < 128 {
        (a as i32 * b as i32 / 255) as u8
    } else {
        255 - ((255 - a as i32) * (255 - b as i32) / 255) as u8
    }
}
