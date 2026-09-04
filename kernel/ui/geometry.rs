//! Device-independent geometry and deterministic fixed-point scaling.

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Insets {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Scale(pub u16);

impl Scale {
    pub const ONE: Self = Self(100);
    pub const ONE_QUARTER: Self = Self(125);
    pub const ONE_HALF: Self = Self(150);
    pub const TWO: Self = Self(200);

    // ------------------------=
    // FUNC: valid
    // DESC: Accepts only the logical scale factors supported by InfinityUI v1.
    // ------------------=
    pub const fn valid(self) -> bool {
        matches!(self.0, 100 | 125 | 150 | 200)
    }

    // ------------------------=
    // FUNC: pixels
    // DESC: Converts logical units into rounded physical pixels without floating point.
    // ------------------=
    pub const fn pixels(self, logical: u32) -> u32 {
        logical.saturating_mul(self.0 as u32).saturating_add(50) / 100
    }
}

impl Rect {
    // ------------------------=
    // FUNC: right
    // DESC: Returns the saturated exclusive right edge.
    // ------------------=
    pub const fn right(self) -> i32 {
        self.x.saturating_add(self.width as i32)
    }

    // ------------------------=
    // FUNC: bottom
    // DESC: Returns the saturated exclusive bottom edge.
    // ------------------=
    pub const fn bottom(self) -> i32 {
        self.y.saturating_add(self.height as i32)
    }

    // ------------------------=
    // FUNC: contains
    // DESC: Tests whether a physical point lies inside the rectangle.
    // ------------------=
    pub const fn contains(self, point: Point) -> bool {
        point.x >= self.x && point.y >= self.y && point.x < self.right() && point.y < self.bottom()
    }

    // ------------------------=
    // FUNC: intersects
    // DESC: Tests whether two rectangles share at least one pixel.
    // ------------------=
    pub const fn intersects(self, other: Rect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    // ------------------------=
    // FUNC: union
    // DESC: Computes the smallest rectangle containing both inputs.
    // ------------------=
    pub const fn union(self, other: Rect) -> Rect {
        let left = if self.x < other.x { self.x } else { other.x };
        let top = if self.y < other.y { self.y } else { other.y };
        let right = if self.right() > other.right() { self.right() } else { other.right() };
        let bottom = if self.bottom() > other.bottom() { self.bottom() } else { other.bottom() };
        Rect {
            x: left,
            y: top,
            width: right.saturating_sub(left) as u32,
            height: bottom.saturating_sub(top) as u32,
        }
    }

    // ------------------------=
    // FUNC: inset
    // DESC: Applies logical content insets while preventing negative geometry.
    // ------------------=
    pub const fn inset(self, insets: Insets) -> Rect {
        let horizontal = insets.left as u32 + insets.right as u32;
        let vertical = insets.top as u32 + insets.bottom as u32;
        Rect {
            x: self.x.saturating_add(insets.left as i32),
            y: self.y.saturating_add(insets.top as i32),
            width: self.width.saturating_sub(horizontal),
            height: self.height.saturating_sub(vertical),
        }
    }
}

// ------------------------=
// FUNC: fit_cover
// DESC: Calculates a centered cover rectangle for a source image inside a viewport.
// ------------------=
pub const fn fit_cover(source: Size, viewport: Rect) -> Rect {
    if source.width == 0 || source.height == 0 || viewport.width == 0 || viewport.height == 0 {
        return viewport;
    }
    let by_width_height = viewport.width.saturating_mul(source.height) / source.width;
    if by_width_height >= viewport.height {
        Rect {
            x: viewport.x,
            y: viewport.y.saturating_sub(((by_width_height - viewport.height) / 2) as i32),
            width: viewport.width,
            height: by_width_height,
        }
    } else {
        let width = viewport.height.saturating_mul(source.width) / source.height;
        Rect {
            x: viewport.x.saturating_sub(((width - viewport.width) / 2) as i32),
            y: viewport.y,
            width,
            height: viewport.height,
        }
    }
}
