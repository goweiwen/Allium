use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self::new(0, 0)
    }
}

impl Default for Point {
    fn default() -> Self {
        Self::zero()
    }
}

impl From<embedded_graphics::prelude::Point> for Point {
    fn from(point: embedded_graphics::prelude::Point) -> Self {
        Self::new(point.x, point.y)
    }
}

impl Into<embedded_graphics::prelude::Point> for Point {
    fn into(self) -> embedded_graphics::prelude::Point {
        embedded_graphics::prelude::Point::new(self.x, self.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    pub fn zero() -> Self {
        Self::new(0, 0, 0, 0)
    }

    pub fn top_left(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub fn union(&self, other: &Self) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let w = ((self.x + self.w as i32).max(other.x + other.w as i32) - x) as u32;
        let h = ((self.y + self.h as i32).max(other.y + other.h as i32) - y) as u32;
        Self::new(x, y, w, h)
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::zero()
    }
}

impl From<embedded_graphics::primitives::Rectangle> for Rect {
    fn from(rect: embedded_graphics::primitives::Rectangle) -> Self {
        Self::new(
            rect.top_left.x,
            rect.top_left.y,
            rect.size.width,
            rect.size.height,
        )
    }
}

impl Into<embedded_graphics::primitives::Rectangle> for Rect {
    fn into(self) -> embedded_graphics::primitives::Rectangle {
        embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::prelude::Point::new(self.x, self.y),
            embedded_graphics::geometry::Size::new(self.w, self.h),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

impl Alignment {
    pub fn sign(&self) -> i32 {
        match self {
            Self::Left => 1,
            Self::Center => 0,
            Self::Right => -1,
        }
    }
}

impl From<embedded_graphics::text::Alignment> for Alignment {
    fn from(alignment: embedded_graphics::text::Alignment) -> Self {
        match alignment {
            embedded_graphics::text::Alignment::Left => Self::Left,
            embedded_graphics::text::Alignment::Center => Self::Center,
            embedded_graphics::text::Alignment::Right => Self::Right,
        }
    }
}

impl Into<embedded_graphics::text::Alignment> for Alignment {
    fn into(self) -> embedded_graphics::text::Alignment {
        match self {
            Self::Left => embedded_graphics::text::Alignment::Left,
            Self::Center => embedded_graphics::text::Alignment::Center,
            Self::Right => embedded_graphics::text::Alignment::Right,
        }
    }
}
