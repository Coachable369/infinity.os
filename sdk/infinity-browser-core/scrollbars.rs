//! Root-page scrollbar geometry and pointer policy, shared by native Servo and tests.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect { pub x: f32, pub y: f32, pub width: f32, pub height: f32 }
impl Rect {
    // ------------------------=
    // FUNC: contains
    // DESC: Uses half-open bounds shared by painting and pointer hit testing.
    // ------------------=
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis { Horizontal, Vertical }
#[derive(Clone, Copy, Debug)]
pub struct Rail { pub axis: Axis, pub track: Rect, pub thumb: Rect, pub maximum: f32, pub offset: f32, pub page: f32 }
impl Rail {
    // ------------------------=
    // FUNC: coordinate
    // DESC: Projects a pointer onto this rail's scrolling axis.
    // ------------------=
    fn coordinate(self, x: f32, y: f32) -> f32 { if self.axis == Axis::Vertical { y } else { x } }
    // ------------------------=
    // FUNC: start
    // DESC: Returns the leading coordinate of a rectangle along this rail.
    // ------------------=
    fn start(self, rect: Rect) -> f32 { if self.axis == Axis::Vertical { rect.y } else { rect.x } }
    // ------------------------=
    // FUNC: length
    // DESC: Returns a rectangle's extent along this rail.
    // ------------------=
    fn length(self, rect: Rect) -> f32 { if self.axis == Axis::Vertical { rect.height } else { rect.width } }
    // ------------------------=
    // FUNC: drag_offset
    // DESC: Maps a clamped thumb position to the engine's full scroll range without a press jump.
    // ------------------=
    fn drag_offset(self, position: f32, grab: f32) -> f32 {
        let travel = self.length(self.track) - self.length(self.thumb);
        if travel <= 0.0 { return self.offset; }
        ((position - self.start(self.track) - grab) / travel).clamp(0.0, 1.0) * self.maximum
    }
}

// ------------------------=
// FUNC: layout
// DESC: Produces persistent rails only for input-scrollable overflow; both axes leave a shared corner free.
// ------------------=
pub fn layout(view: Rect, page: [f32; 2], maximum: [f32; 2], offset: [f32; 2], scale: f32) -> [Option<Rail>; 2] {
    layout_styled(view, page, maximum, offset, scale, false)
}
// ------------------------=
// FUNC: layout_styled
// DESC: Applies the root author's thin-width choice while retaining the normal thumb minimum and bounds.
// ------------------=
pub fn layout_styled(view: Rect, page: [f32; 2], maximum: [f32; 2], offset: [f32; 2], scale: f32, thin: bool) -> [Option<Rail>; 2] {
    if ![view.x, view.y, view.width, view.height, scale].iter().all(|n| n.is_finite()) || scale <= 0.0 { return [None, None]; }
    let thickness = if thin { 8.0 } else { 14.0 } * scale;
    let visible = [maximum[0].is_finite() && maximum[0] > 0.0 && page[0] > 0.0,
        maximum[1].is_finite() && maximum[1] > 0.0 && page[1] > 0.0];
    let mut rails = [None, None];
    for i in 0..2 {
        if !visible[i] { continue; }
        let vertical = i == 1;
        let length = (if vertical { view.height } else { view.width }) - if visible[1-i] { thickness } else { 0.0 };
        if length < thickness * 2.0 || view.width < thickness || view.height < thickness { continue; }
        let track = if vertical { Rect { x: view.x + view.width - thickness, y: view.y, width: thickness, height: length } }
            else { Rect { x: view.x, y: view.y + view.height - thickness, width: length, height: thickness } };
        let thumb_length = (length * page[i] / (page[i] + maximum[i])).max(24.0 * scale).min(length);
        let position = if offset[i].is_finite() { offset[i].clamp(0.0, maximum[i]) } else { 0.0 };
        let start = (length - thumb_length) * position / maximum[i];
        let thumb = if vertical { Rect { y: track.y + start, height: thumb_length, ..track } }
            else { Rect { x: track.x + start, width: thumb_length, ..track } };
        rails[i] = Some(Rail { axis: if vertical { Axis::Vertical } else { Axis::Horizontal }, track, thumb,
            maximum: maximum[i], offset: position, page: page[i] });
    }
    rails
}

#[derive(Clone, Copy)]
pub enum Pointer { Down, Move, Up, Leave, Cancel }
#[derive(Default)]
pub struct Interaction { pub hover: Option<Axis>, pub pressed: Option<Axis>, grab: Option<f32> }
#[derive(Default, Debug)]
pub struct Outcome { pub consumed: bool, pub repaint: bool, pub scroll: Option<(Axis, f32)> }
impl Interaction {
    // ------------------------=
    // FUNC: pointer
    // DESC: Captures thumb drags and track presses, consumes their release, and keeps page input outside rails unchanged.
    // ------------------=
    pub fn pointer(&mut self, rails: [Option<Rail>; 2], event: Pointer, x: f32, y: f32) -> Outcome {
        let old = (self.hover, self.pressed);
        let mut result = Outcome::default();
        let hit = rails.iter().flatten().find(|rail| rail.track.contains(x, y)).copied();
        self.hover = hit.map(|rail| rail.axis);
        match event {
            Pointer::Cancel | Pointer::Leave => { self.hover = None; self.pressed = None; self.grab = None; }
            Pointer::Down => {
                if let Some(rail) = hit {
                    self.pressed = Some(rail.axis);
                    let at = rail.coordinate(x, y);
                    if at >= rail.start(rail.thumb) && at < rail.start(rail.thumb) + rail.length(rail.thumb) {
                        self.grab = Some(at - rail.start(rail.thumb));
                    } else {
                        self.grab = None;
                        let direction = if at < rail.start(rail.thumb) { -1.0 } else { 1.0 };
                        result.scroll = Some((rail.axis, (rail.offset + direction * rail.page * 0.9).clamp(0.0, rail.maximum)));
                    }
                    result.consumed = true;
                }
            }
            Pointer::Move => {
                if let Some(axis) = self.pressed {
                    result.consumed = true;
                    if let (Some(grab), Some(rail)) = (self.grab, rails.iter().flatten().find(|rail| rail.axis == axis)) {
                        result.scroll = Some((axis, rail.drag_offset(rail.coordinate(x, y), grab)));
                    }
                } else { result.consumed = hit.is_some(); }
            }
            Pointer::Up => {
                result.consumed = self.pressed.is_some() || hit.is_some();
                self.pressed = None; self.grab = None;
            }
        }
        result.repaint = old != (self.hover, self.pressed) || result.scroll.is_some();
        result
    }
}
