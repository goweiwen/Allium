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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    #[inline]
    pub const fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    #[inline]
    pub const fn zero() -> Self {
        Self::new(0, 0, 0, 0)
    }

    #[inline]
    pub const fn top_left(&self) -> Point {
        Point::new(self.x, self.y)
    }

    #[inline]
    pub const fn size(&self) -> Size {
        Size::new(self.w, self.h)
    }

    #[inline]
    pub const fn right(&self) -> i32 {
        self.x + self.w as i32
    }

    #[inline]
    pub const fn bottom(&self) -> i32 {
        self.y + self.h as i32
    }

    pub fn union(&self, other: &Self) -> Self {
        if self.w == 0 || self.h == 0 {
            return *other;
        } else if other.w == 0 || other.h == 0 {
            return *self;
        }

        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let w = ((self.x + self.w as i32).max(other.x + other.w as i32) - x) as u32;
        let h = ((self.y + self.h as i32).max(other.y + other.h as i32) - y) as u32;
        Self::new(x, y, w, h)
    }

    pub fn intersection(&self, other: &Self) -> Self {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let w = ((self.x + self.w as i32).min(other.x + other.w as i32) - x) as u32;
        let h = ((self.y + self.h as i32).min(other.y + other.h as i32) - y) as u32;
        Self::new(x, y, w, h)
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::zero()
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
