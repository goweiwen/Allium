use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const fn zero() -> Self {
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

impl From<Point> for embedded_graphics::prelude::Point {
    fn from(val: Point) -> Self {
        embedded_graphics::prelude::Point::new(val.x, val.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    pub w: u32,
    pub h: u32,
}

impl Size {
    pub const fn new(w: u32, h: u32) -> Self {
        Self { w, h }
    }

    pub const fn zero() -> Self {
        Self::new(0, 0)
    }
}

impl Default for Size {
    fn default() -> Self {
        Self::zero()
    }
}

impl From<embedded_graphics::prelude::Size> for Size {
    fn from(size: embedded_graphics::prelude::Size) -> Self {
        Self::new(size.width, size.height)
    }
}

impl From<Size> for embedded_graphics::prelude::Size {
    fn from(val: Size) -> Self {
        embedded_graphics::prelude::Size::new(val.w, val.h)
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
    pub const fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    pub const fn zero() -> Self {
        Self::new(0, 0, 0, 0)
    }

    pub const fn top_left(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub const fn size(&self) -> Size {
        Size::new(self.w, self.h)
    }

    pub const fn right(&self) -> i32 {
        self.x + self.w as i32
    }

    pub const fn bottom(&self) -> i32 {
        self.y + self.h as i32
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

impl From<Rect> for embedded_graphics::primitives::Rectangle {
    fn from(val: Rect) -> Self {
        embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::prelude::Point::new(val.x, val.y),
            embedded_graphics::geometry::Size::new(val.w, val.h),
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
    pub const fn sign(&self) -> i32 {
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

impl From<Alignment> for embedded_graphics::text::Alignment {
    fn from(val: Alignment) -> Self {
        match val {
            Alignment::Left => embedded_graphics::text::Alignment::Left,
            Alignment::Center => embedded_graphics::text::Alignment::Center,
            Alignment::Right => embedded_graphics::text::Alignment::Right,
        }
    }
}
