//! Bounded native presentation damage; overflow merges the cheapest pair rather
//! than collapsing every region into a screen-spanning rectangle.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Region {
    pub left: usize,
    pub top: usize,
    pub right: usize,
    pub bottom: usize,
}
impl Region {
    // ------------------------=
    // FUNC: area
    // DESC: Returns bounded rectangular pixel coverage.
    // ------------------=
    pub fn area(self) -> usize {
        self.right
            .saturating_sub(self.left)
            .saturating_mul(self.bottom.saturating_sub(self.top))
    }
    // ------------------------=
    // FUNC: union
    // DESC: Returns the enclosing rectangle for two damage regions.
    // ------------------=
    pub fn union(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }
    // ------------------------=
    // FUNC: contains
    // DESC: Checks whether a rectangle fully covers another.
    // ------------------=
    pub fn contains(self, other: Self) -> bool {
        self.left <= other.left
            && self.top <= other.top
            && self.right >= other.right
            && self.bottom >= other.bottom
    }
}

// ------------------------=
// FUNC: insert
// DESC: Adds coverage, coalesces without area inflation, and bounds overflow to one minimum-cost merge.
// ------------------=
pub fn insert<const N: usize>(
    regions: &mut [Region; N],
    count: &mut usize,
    mut incoming: Region,
) -> bool {
    assert!(N > 0 && *count <= N);
    if incoming.area() == 0 {
        return false;
    }
    let mut index = 0;
    while index < *count {
        if regions[index].contains(incoming) {
            return false;
        }
        let union = regions[index].union(incoming);
        if union.area() <= regions[index].area().saturating_add(incoming.area()) {
            incoming = union;
            *count -= 1;
            regions[index] = regions[*count];
            index = 0;
        } else {
            index += 1;
        }
    }
    if *count < N {
        regions[*count] = incoming;
        *count += 1;
        return false;
    }
    let mut best = (usize::MAX, 0, N);
    for a in 0..N {
        for b in a + 1..=N {
            let second = if b == N { incoming } else { regions[b] };
            let cost = regions[a]
                .union(second)
                .area()
                .saturating_sub(regions[a].area().saturating_add(second.area()));
            if cost < best.0 {
                best = (cost, a, b);
            }
        }
    }
    if best.2 == N {
        regions[best.1] = regions[best.1].union(incoming);
    } else {
        regions[best.1] = regions[best.1].union(regions[best.2]);
        regions[best.2] = incoming;
    }
    true
}
