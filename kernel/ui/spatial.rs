//! Bounded, user-owned spatial metadata. References never convey object authority.
use super::session_state::{self, DesktopSessionLayout};

// ------------------------=
// FUNC: refresh_due
// DESC: Defers queued application refreshes until a finite retained-scene transition has settled.
// ------------------=
pub fn refresh_due(requested: bool, elapsed_ms: u64, moving: bool, closing: bool) -> bool {
    requested && elapsed_ms >= 100 && !moving && !closing
}

// ------------------------=
// FUNC: drag_position
// DESC: Resolves final placement from press and current coordinates independently of intermediate event delivery.
// ------------------=
pub fn drag_position(origin: (usize, usize), press: (i32, i32), current: (i32, i32)) -> (u16, u16) {
    (
        (origin.0 as i64 + i64::from(current.0) - i64::from(press.0)).clamp(80, 710) as u16,
        (origin.1 as i64 + i64::from(current.1) - i64::from(press.1)).clamp(230, 620) as u16,
    )
}

pub const STATE_BYTES: usize = 8192;
pub const ITEM_COUNT: usize = 16;
pub const WORLD_COUNT: usize = 4;
static DESKTOP_WORLD: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(255);
static WORLD_DIRTY: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
// ------------------------=
// FUNC: publish_world
// DESC: Publishes a wallpaper identity and invalidates the retained desktop only when it changes.
// ------------------=
pub fn publish_world(world: u8) {
    use core::sync::atomic::Ordering;
    if DESKTOP_WORLD.swap(world, Ordering::Relaxed) != world {
        WORLD_DIRTY.store(true, Ordering::Relaxed);
    }
}
// ------------------------=
// FUNC: desktop_world
// DESC: Reads the selected packaged world wallpaper without a storage query.
// ------------------=
pub fn desktop_world() -> u8 {
    DESKTOP_WORLD.load(core::sync::atomic::Ordering::Relaxed)
}
// ------------------------=
// FUNC: take_world_damage
// DESC: Consumes one structural repaint for an explicit world change.
// ------------------=
pub fn take_world_damage() -> bool {
    WORLD_DIRTY.swap(false, core::sync::atomic::Ordering::Relaxed)
}
// ------------------------=
// FUNC: ring_radius
// DESC: Scales each category orbit independently while keeping it inside the spatial stage.
// ------------------=
pub fn ring_radius(state: &SpatialState, group: usize) -> usize {
    ((100 + group * 85) * (192 + state.ring_zoom[group] as usize) / 320).min(390)
}
// ------------------------=
// FUNC: ring_card
// DESC: Places category ideas on the same ellipse drawn by the native renderer.
// ------------------=
pub fn ring_card(state: &SpatialState, index: usize) -> (usize, usize, usize, usize) {
    let group = state.items[index]
        .map(|i| i.collection as usize)
        .unwrap_or(0);
    let count = (0..ITEM_COUNT)
        .filter(|&i| {
            state.visible(i) && state.items[i].is_some_and(|v| v.collection as usize == group)
        })
        .count()
        .max(1);
    let ordinal = (0..index)
        .filter(|&i| {
            state.visible(i) && state.items[i].is_some_and(|v| v.collection as usize == group)
        })
        .count();
    const POINTS: [(i32, i32); 16] = [
        (1000, 0),
        (924, 383),
        (707, 707),
        (383, 924),
        (0, 1000),
        (-383, 924),
        (-707, 707),
        (-924, 383),
        (-1000, 0),
        (-924, -383),
        (-707, -707),
        (-383, -924),
        (0, -1000),
        (383, -924),
        (707, -707),
        (924, -383),
    ];
    let (x, y) = POINTS[(ordinal * 16 / count).min(15)];
    let radius = ring_radius(state, group).min(390) as i32;
    (
        (500 + x * radius / 1000 - 70).max(65) as usize,
        (475 + y * radius / 2200 - 40).max(250) as usize,
        140,
        90,
    )
}
// ------------------------=
// FUNC: ring_hit
// DESC: Selects the closest category ellipse within a generous pointer tolerance.
// ------------------=
pub fn ring_hit(state: &SpatialState, x: i32, y: i32) -> Option<u8> {
    let dx = i64::from(x - 500);
    let dy = i64::from((y - 475) * 2);
    (0..4)
        .filter(|&i| !state.categories[i].get().is_empty())
        .min_by_key(|&i| (dx * dx + dy * dy - (ring_radius(state, i) as i64).pow(2)).abs())
        .filter(|&i| {
            (dx * dx + dy * dy - (ring_radius(state, i) as i64).pow(2)).abs()
                < ring_radius(state, i) as i64 * 28
        })
        .map(|i| i as u8)
}
// ------------------------=
// FUNC: collection_card
// DESC: Shares collection geometry between painting, clicks, and drops.
// ------------------=
pub fn collection_card(index: usize) -> (usize, usize, usize, usize) {
    (80 + index * 210, 730, 190, 60)
}
// ------------------------=
// FUNC: collection_hit
// DESC: Resolves named destinations without accepting gutters.
// ------------------=
pub fn collection_hit(x: i32, y: i32) -> Option<u8> {
    (0..WORLD_COUNT)
        .find(|&i| contains(collection_card(i), x, y))
        .map(|i| i as u8)
}
// ------------------------=
// FUNC: collection_name
// DESC: Uses saved collection names consistently across controls and confirmations.
// ------------------=
pub fn collection_name(state: &SpatialState, group: usize) -> &[u8] {
    let Some(name) = state.categories.get(group) else {
        return b"";
    };
    if name.get().is_empty() {
        b"+ Category"
    } else {
        name.get()
    }
}
pub const OVERVIEW_COUNT: usize = 10;
pub type OverviewBounds = (usize, usize, usize, usize);

#[derive(Clone, Copy)]
pub struct OverviewFrame {
    pub bounds: [OverviewBounds; OVERVIEW_COUNT],
    pub order: [usize; OVERVIEW_COUNT],
    pub count: usize,
}
impl OverviewFrame {
    // ------------------------=
    // FUNC: new
    // DESC: Samples curved carousel geometry and sorts its cards back to front from their visible size.
    // ------------------=
    pub fn new(
        from: Option<&[OverviewBounds; OVERVIEW_COUNT]>,
        focus: usize,
        zoom: u8,
        count: usize,
        progress: u8,
    ) -> Self {
        let count = count.min(OVERVIEW_COUNT);
        let mut frame = Self {
            bounds: [(0, 0, 0, 0); OVERVIEW_COUNT],
            order: [0; OVERVIEW_COUNT],
            count,
        };
        let t = usize::from(progress);
        for i in 0..count {
            let target = overview_bounds(i, focus, zoom, count);
            let origin = from.map_or(target, |r| r[i]);
            let mix = |a: usize, b: usize| (a * (255 - t) + b * t) / 255;
            let arc = origin.0.abs_diff(target.0).min(400) * t * (255 - t) / (255 * 255 * 8);
            frame.bounds[i] = (
                mix(origin.0, target.0),
                mix(origin.1, target.1).saturating_sub(arc),
                mix(origin.2, target.2),
                mix(origin.3, target.3),
            );
            frame.order[i] = i;
        }
        frame.order[..count]
            .sort_unstable_by_key(|i| (frame.bounds[*i].2 * frame.bounds[*i].3, *i == focus));
        frame
    }
    // ------------------------=
    // FUNC: hit
    // DESC: Resolves the topmost visible animated card, matching the renderer's exact depth order.
    // ------------------=
    pub fn hit(&self, x: i32, y: i32) -> Option<usize> {
        self.order[..self.count]
            .iter()
            .rev()
            .copied()
            .find(|i| contains(self.bounds[*i], x, y))
    }
}

// ------------------------=
// FUNC: overview_activates
// DESC: Activates only a second selection of the settled foreground card, never an in-flight side selection.
// ------------------=
pub fn overview_activates(hit: usize, focus: usize, moving: bool) -> bool {
    hit == focus && !moving
}

// ------------------------=
// FUNC: next_reference
// DESC: Traverses only real references, preserving keyboard navigation after removals leave empty slots.
// ------------------=
pub fn next_reference(state: &SpatialState, current: usize, reverse: bool) -> usize {
    for step in 1..=ITEM_COUNT {
        let index = (current.min(ITEM_COUNT - 1) + if reverse { ITEM_COUNT - step } else { step })
            % ITEM_COUNT;
        if state.items[index].is_some() {
            return index;
        }
    }
    0
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DropTarget {
    Collection(u8),
    Editor,
    Folder(Label<96>),
}
#[derive(Clone, Copy)]
pub struct DropRequest {
    pub index: usize,
    pub target: DropTarget,
}
impl DropRequest {
    // ------------------------=
    // FUNC: new
    // DESC: Rejects nonexistent references and malformed destinations before showing a drop proposal.
    // ------------------=
    pub fn new(state: &SpatialState, index: usize, target: DropTarget) -> Option<Self> {
        let item = state.items.get(index)?.as_ref()?;
        match target {
            DropTarget::Collection(group) if group >= WORLD_COUNT as u8 => return None,
            DropTarget::Folder(path)
                if item.object == [0; 16] || path.get().first() != Some(&b'/') =>
            {
                return None
            }
            _ => {}
        }
        Some(Self { index, target })
    }
}
#[derive(Clone, Copy)]
pub struct Preview {
    pub slot: usize,
    pub app: u8,
    pub navigator: Option<usize>,
    pub visible: bool,
    pub label: Label<96>,
}
impl Preview {
    pub const EMPTY: Self = Self {
        slot: 0,
        app: 0,
        navigator: None,
        visible: false,
        label: Label::empty(),
    };
}
pub const TABS: [&[u8]; 5] = [
    b"Holographic",
    b"Worldshift",
    b"Gravity Wall",
    b"Matter Shelf",
    b"Constellations",
];

// ------------------------=
// FUNC: card
// DESC: Shares normalized interactive card geometry between painting and hit testing.
// ------------------=
pub fn card(index: usize) -> (usize, usize, usize, usize) {
    (80 + (index % 4) * 210, 230 + (index / 4) * 130, 190, 110)
}
// ------------------------=
// FUNC: shelf_card
// DESC: Keeps four shelf slots visible per keyboard-selected page without overlapping app content.
// ------------------=
pub fn shelf_card(index: usize, focus: usize) -> Option<(usize, usize, usize, usize)> {
    if index >= ITEM_COUNT || index / 4 != focus / 4 {
        return None;
    }
    Some((80 + index % 4 * 210, 690, 190, 100))
}
// ------------------------=
// FUNC: overview_card
// DESC: Interpolates a retained window island into an aspect-preserving inspection stage.
// ------------------=
pub fn overview_card(index: usize, focus: usize, zoom: u8) -> (usize, usize, usize, usize) {
    let normal = (80 + (index % 3) * 280, 230 + (index / 3) * 255, 260, 225);
    if index != focus {
        return normal;
    }
    let t = usize::from(zoom);
    let mix = |a: usize, b: usize| (a * (255 - t) + b * t) / 255;
    (
        mix(normal.0, 110),
        mix(normal.1, 230),
        mix(normal.2, 780),
        mix(normal.3, 480),
    )
}
// ------------------------=
// FUNC: overview_bounds
// DESC: Depth-stages small sessions and uses an accessible filmstrip for larger sessions above the action strip.
// ------------------=
pub fn overview_bounds(
    index: usize,
    focus: usize,
    zoom: u8,
    count: usize,
) -> (usize, usize, usize, usize) {
    // The normal five-window desktop uses the kit's depth-staged composition.
    // Larger sessions retain a bounded accessible filmstrip, never offscreen cards.
    if count <= 5 {
        if index == focus {
            let t = usize::from(zoom);
            return (
                240usize.saturating_sub(60 * t / 255),
                250 - 20 * t / 255,
                520 + 120 * t / 255,
                480 + 20 * t / 255,
            );
        }
        let rank = if index < focus {
            index
        } else {
            index.saturating_sub(1)
        };
        let rear = rank / 2;
        return (
            if rank % 2 == 0 {
                90 + rear * 20
            } else {
                680 - rear * 20
            },
            315usize.saturating_sub(rear * 65),
            230,
            360usize.saturating_sub(rear * 35),
        );
    }
    // A large live foreground surface and a reachable filmstrip replace the
    // equal-weight application grid. Even ten independent windows stay visible.
    if index != focus {
        let rank = if index < focus {
            index
        } else {
            index.saturating_sub(1)
        };
        let slots = count.saturating_sub(1).max(1);
        let width = (820 / slots).min(175);
        let total = slots * width;
        return (
            90 + (820 - total) / 2 + rank * width,
            695,
            width.saturating_sub(10),
            90,
        );
    }
    let t = usize::from(zoom);
    let mix = |a: usize, b: usize| (a * (255 - t) + b * t) / 255;
    (mix(180, 130), 230, mix(640, 740), mix(425, 445))
}
// ------------------------=
// FUNC: world_card
// DESC: Shares tall environment-card geometry with input dispatch.
// ------------------=
pub fn world_card(index: usize) -> (usize, usize, usize, usize) {
    (
        80 + index * 210,
        [300, 245, 270, 335][index.min(3)],
        190,
        350,
    )
}
// ------------------------=
// FUNC: contains
// DESC: Tests half-open logical bounds, excluding adjacent gutters.
// ------------------=
pub fn contains(rect: (usize, usize, usize, usize), x: i32, y: i32) -> bool {
    x >= rect.0 as i32
        && y >= rect.1 as i32
        && x < (rect.0 + rect.2) as i32
        && y < (rect.1 + rect.3) as i32
}

// ------------------------=
// FUNC: hit_card
// DESC: Resolves only visible bounded cards, keeping gutters noninteractive.
// ------------------=
pub fn hit_card(x: i32, y: i32, count: usize) -> Option<usize> {
    (0..count.min(ITEM_COUNT)).find(|i| {
        let (a, b, w, h) = card(*i);
        x >= a as i32 && x < (a + w) as i32 && y >= b as i32 && y < (b + h) as i32
    })
}
// ------------------------=
// FUNC: item_card
// DESC: Applies persisted placement with clamping to the interactive canvas.
// ------------------=
pub fn item_card(index: usize, item: &Item) -> (usize, usize, usize, usize) {
    if item.x == 0 && item.y == 0 {
        card(index)
    } else {
        (
            (item.x as usize).clamp(80, 710),
            (item.y as usize).clamp(230, 620),
            190,
            110,
        )
    }
}
// ------------------------=
// FUNC: hit_item
// DESC: Selects the topmost placed reference without making empty slots clickable.
// ------------------=
pub fn hit_item(state: &SpatialState, x: i32, y: i32) -> Option<usize> {
    (0..ITEM_COUNT).rev().find(|i| {
        state.items[*i]
            .map(|item| {
                let (a, b, w, h) = item_card(*i, &item);
                x >= a as i32 && x < (a + w) as i32 && y >= b as i32 && y < (b + h) as i32
            })
            .unwrap_or(false)
    })
}
const WORLD_BYTES: usize = 400;
const ITEM_BYTES: usize = 352;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Label<const N: usize> {
    bytes: [u8; N],
    len: u8,
}
impl<const N: usize> Label<N> {
    // ------------------------=
    // FUNC: empty
    // DESC: Creates empty bounded text without allocation.
    // ------------------=
    pub const fn empty() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }
    // ------------------------=
    // FUNC: set
    // DESC: Rejects oversized or control-containing labels rather than truncating identity.
    // ------------------=
    pub fn set(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() > N || bytes.len() > 255 || bytes.iter().any(|b| *b < 32 || *b > 126) {
            return false;
        }
        self.bytes.fill(0);
        self.bytes[..bytes.len()].copy_from_slice(bytes);
        self.len = bytes.len() as u8;
        true
    }
    // ------------------------=
    // FUNC: get
    // DESC: Returns the validated visible prefix.
    // ------------------=
    pub fn get(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len).min(N)]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct World {
    pub name: Label<24>,
    pub location: Label<96>,
    pub editor: Label<96>,
    pub layout: Option<DesktopSessionLayout>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item {
    pub object: [u8; 16],
    pub path: Label<96>,
    pub name: Label<32>,
    pub text: Label<192>,
    pub collection: u8,
    pub x: u16,
    pub y: u16,
    pub links: u16,
    pub parent: Option<u8>,
    pub expanded: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Owner,
    Capacity,
    Invalid,
    Missing,
    Corrupt,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpatialState {
    owner: [u8; 16],
    pub worlds: [World; WORLD_COUNT],
    pub items: [Option<Item>; ITEM_COUNT],
    pub active_world: u8,
    pub reduced_motion: bool,
    pub categories: [Label<24>; 4],
    pub selected_ring: u8,
    pub ring_zoom: [u8; 4],
    pub world_icons: [u8; 4],
    pub world_accents: [u32; 4],
    pub world_primary: [u32; 4],
    pub world_skin: [u8; 4],
    pub world_enabled: bool,
}
impl SpatialState {
    // ------------------------=
    // FUNC: branch
    // DESC: Returns the bounded subtree mask including its root, without allocating.
    // ------------------=
    pub fn branch(&self, root: usize) -> u16 {
        if root >= ITEM_COUNT || self.items[root].is_none() {
            return 0;
        }
        let mut mask = 1u16 << root;
        for _ in 0..ITEM_COUNT {
            for (i, item) in self.items.iter().enumerate() {
                if item
                    .and_then(|v| v.parent)
                    .is_some_and(|p| p < 16 && mask & (1 << p) != 0)
                {
                    mask |= 1 << i;
                }
            }
        }
        mask
    }
    // ------------------------=
    // FUNC: visible
    // DESC: Hides descendants of collapsed ancestors and rejects malformed ancestry.
    // ------------------=
    pub fn visible(&self, index: usize) -> bool {
        let Some(mut item) = self.items.get(index).copied().flatten() else {
            return false;
        };
        for _ in 0..ITEM_COUNT {
            let Some(parent) = item.parent else {
                return true;
            };
            let Some(ancestor) = self.items.get(parent as usize).copied().flatten() else {
                return false;
            };
            if !ancestor.expanded {
                return false;
            }
            item = ancestor;
        }
        false
    }
    // ------------------------=
    // FUNC: add_category
    // DESC: Allocates a named category and its own selectable ring, rejecting duplicates and capacity overflow.
    // ------------------=
    pub fn add_category(&mut self, owner: [u8; 16], name: &[u8]) -> Result<usize, Error> {
        self.authorize(owner)?;
        if name.is_empty()
            || self
                .categories
                .iter()
                .any(|c| c.get().eq_ignore_ascii_case(name))
        {
            return Err(Error::Invalid);
        }
        let index = self
            .categories
            .iter()
            .position(|c| c.get().is_empty())
            .ok_or(Error::Capacity)?;
        if !self.categories[index].set(name) {
            return Err(Error::Invalid);
        }
        self.selected_ring = index as u8;
        Ok(index)
    }
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty per-user workspace without seeded fake documents.
    // ------------------=
    pub const fn new(owner: [u8; 16]) -> Self {
        Self {
            owner,
            worlds: [World {
                name: Label::empty(),
                location: Label::empty(),
                editor: Label::empty(),
                layout: None,
            }; WORLD_COUNT],
            items: [None; ITEM_COUNT],
            active_world: 0,
            reduced_motion: false,
            categories: [Label::empty(); 4],
            selected_ring: 0,
            ring_zoom: [128; 4],
            world_icons: [3, 0, 1, 2],
            world_accents: [0x69d7ff, 0xffb56b, 0x9baeff, 0x70ead1],
            world_primary: [0x031422, 0x20130b, 0x101329, 0x06241e],
            world_skin: [0; 4],
            world_enabled: false,
        }
    }
    // ------------------------=
    // FUNC: authorize
    // DESC: Separates user metadata access from underlying object authorization.
    // ------------------=
    fn authorize(&self, owner: [u8; 16]) -> Result<(), Error> {
        if owner == [0; 16] || owner != self.owner {
            Err(Error::Owner)
        } else {
            Ok(())
        }
    }
    // ------------------------=
    // FUNC: gather
    // DESC: Adds an explicit file reference or short text clipping with no source mutation.
    // ------------------=
    pub fn gather(&mut self, owner: [u8; 16], mut item: Item) -> Result<usize, Error> {
        self.authorize(owner)?;
        if item.collection >= 4
            || item.x > 1000
            || item.y > 1000
            || item.name.get().is_empty()
            || (item.object == [0; 16] && item.text.get().is_empty())
        {
            return Err(Error::Invalid);
        }
        let index = self
            .items
            .iter()
            .position(Option::is_none)
            .ok_or(Error::Capacity)?;
        item.links = 0;
        if let Some(parent) = item.parent {
            if !self
                .items
                .get(parent as usize)
                .copied()
                .flatten()
                .is_some_and(|p| p.collection == item.collection)
            {
                return Err(Error::Invalid);
            }
        }
        if self.categories[item.collection as usize].get().is_empty() {
            self.categories[item.collection as usize].set(
                [
                    b"Ideas".as_slice(),
                    b"Category 2",
                    b"Category 3",
                    b"Category 4",
                ][item.collection as usize],
            );
        }
        self.items[index] = Some(item);
        Ok(index)
    }
    // ------------------------=
    // FUNC: remove
    // DESC: Removes only spatial metadata and incoming relationships, never the referenced file.
    // ------------------=
    pub fn remove(&mut self, owner: [u8; 16], index: usize) -> Result<(), Error> {
        self.authorize(owner)?;
        let mask = self.branch(index);
        if mask == 0 {
            return Err(Error::Missing);
        }
        for i in 0..ITEM_COUNT {
            if mask & (1 << i) != 0 {
                self.items[i] = None;
            }
        }
        for item in self.items.iter_mut().flatten() {
            item.links &= !mask;
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: connect
    // DESC: Toggles a symmetric user-authored relationship between two existing objects.
    // ------------------=
    pub fn connect(&mut self, owner: [u8; 16], a: usize, b: usize) -> Result<(), Error> {
        self.authorize(owner)?;
        if a == b
            || a >= ITEM_COUNT
            || b >= ITEM_COUNT
            || self.items[a].is_none()
            || self.items[b].is_none()
        {
            return Err(Error::Missing);
        }
        self.items[a].as_mut().unwrap().links ^= 1 << b;
        self.items[b].as_mut().unwrap().links ^= 1 << a;
        Ok(())
    }
    // ------------------------=
    // FUNC: place
    // DESC: Repositions and regroups a reference using bounded logical coordinates.
    // ------------------=
    pub fn place(
        &mut self,
        owner: [u8; 16],
        index: usize,
        group: u8,
        x: u16,
        y: u16,
    ) -> Result<(), Error> {
        self.authorize(owner)?;
        if group >= 4 || x > 1000 || y > 1000 {
            return Err(Error::Invalid);
        }
        let branch = self.branch(index);
        let changed_group = self
            .items
            .get(index)
            .copied()
            .flatten()
            .is_some_and(|item| item.collection != group);
        for i in 0..ITEM_COUNT {
            if branch & (1 << i) != 0 {
                self.items[i].as_mut().unwrap().collection = group;
            }
        }
        let item = self
            .items
            .get_mut(index)
            .and_then(Option::as_mut)
            .ok_or(Error::Missing)?;
        item.collection = group;
        if changed_group {
            item.parent = None;
        }
        item.x = x;
        item.y = y;
        Ok(())
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Serializes versioned metadata in an architecture-neutral checksummed object.
    // ------------------=
    pub fn encode(&self, owner: [u8; 16]) -> Result<[u8; STATE_BYTES], Error> {
        self.authorize(owner)?;
        if self.active_world >= WORLD_COUNT as u8 {
            return Err(Error::Invalid);
        }
        let mut out = [0u8; STATE_BYTES];
        out[..8].copy_from_slice(b"INFSPC01");
        out[8..24].copy_from_slice(&owner);
        out[24] = self.active_world;
        out[25] = u8::from(self.reduced_motion);
        out[26] = 2;
        out[27] = self.selected_ring;
        out[28] = u8::from(self.world_enabled);
        for i in 0..4 {
            put_label(&mut out, 7264 + i * 25, &self.categories[i]);
            out[7364 + i] = self.ring_zoom[i];
            out[7368 + i] = self.world_icons[i];
            out[7372 + i * 4..7376 + i * 4].copy_from_slice(&self.world_accents[i].to_le_bytes());
            out[7388 + i * 4..7392 + i * 4].copy_from_slice(&self.world_primary[i].to_le_bytes());
            out[7404 + i] = self.world_skin[i];
        }
        for (i, world) in self.worlds.iter().enumerate() {
            let at = 32 + i * WORLD_BYTES;
            put_label(&mut out, at, &world.name);
            put_label(&mut out, at + 25, &world.location);
            put_label(&mut out, at + 300, &world.editor);
            if let Some(layout) = world.layout {
                out[at + 122] = 1;
                session_state::write_layout(&mut out, at + 123, layout);
                out[at + 282..at + 284]
                    .copy_from_slice(&layout.app_drawer_floating[0].to_le_bytes());
                out[at + 284..at + 286]
                    .copy_from_slice(&layout.app_drawer_floating[1].to_le_bytes());
            }
        }
        for (i, item) in self.items.iter().enumerate() {
            let Some(item) = item else { continue };
            let at = 32 + WORLD_COUNT * WORLD_BYTES + i * ITEM_BYTES;
            out[at] = 1;
            out[at + 1..at + 17].copy_from_slice(&item.object);
            put_label(&mut out, at + 17, &item.path);
            put_label(&mut out, at + 114, &item.name);
            put_label(&mut out, at + 147, &item.text);
            out[at + 340] = item.collection;
            out[at + 341..at + 343].copy_from_slice(&item.x.to_le_bytes());
            out[at + 343..at + 345].copy_from_slice(&item.y.to_le_bytes());
            out[at + 345..at + 347].copy_from_slice(&item.links.to_le_bytes());
            out[at + 347] = item.parent.map_or(0, |p| p + 1);
            out[at + 348] = u8::from(item.expanded);
        }
        let sum = checksum(&out[..STATE_BYTES - 4]);
        out[STATE_BYTES - 4..].copy_from_slice(&sum.to_le_bytes());
        Ok(out)
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Rejects corrupt, cross-user, dangling or noncanonical spatial state before publication.
    // ------------------=
    pub fn decode(owner: [u8; 16], bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != STATE_BYTES || &bytes[..8] != b"INFSPC01" {
            return Err(Error::Corrupt);
        }
        if owner == [0; 16] || bytes[8..24] != owner {
            return Err(Error::Owner);
        }
        if checksum(&bytes[..STATE_BYTES - 4])
            != u32::from_le_bytes(bytes[STATE_BYTES - 4..].try_into().unwrap())
            || bytes[24] >= 4
            || bytes[25] > 1
        {
            return Err(Error::Corrupt);
        }
        let mut state = Self::new(owner);
        state.active_world = bytes[24];
        state.reduced_motion = bytes[25] != 0;
        if bytes[26] == 1 || bytes[26] == 2 {
            if bytes[27] >= 4 || bytes[28] > 1 {
                return Err(Error::Corrupt);
            }
            state.selected_ring = bytes[27];
            state.world_enabled = bytes[28] != 0;
            for i in 0..4 {
                state.categories[i] = read_label(bytes, 7264 + i * 25)?;
                state.ring_zoom[i] = bytes[7364 + i];
                if bytes[7368 + i] >= 4 {
                    return Err(Error::Corrupt);
                }
                state.world_icons[i] = bytes[7368 + i];
                if bytes[26] == 2 {
                    state.world_accents[i] =
                        u32::from_le_bytes(bytes[7372 + i * 4..7376 + i * 4].try_into().unwrap());
                    state.world_primary[i] =
                        u32::from_le_bytes(bytes[7388 + i * 4..7392 + i * 4].try_into().unwrap());
                    if bytes[7404 + i] > 2 {
                        return Err(Error::Corrupt);
                    }
                    state.world_skin[i] = bytes[7404 + i];
                }
            }
        }
        for (i, world) in state.worlds.iter_mut().enumerate() {
            let at = 32 + i * WORLD_BYTES;
            world.name = read_label(bytes, at)?;
            world.location = read_label(bytes, at + 25)?;
            world.editor = read_label(bytes, at + 300)?;
            world.layout = match bytes[at + 122] {
                0 => None,
                1 => Some(session_state::read_layout(bytes, at + 123).ok_or(Error::Corrupt)?),
                _ => return Err(Error::Corrupt),
            };
            if let Some(layout) = world.layout.as_mut() {
                layout.app_drawer_floating = [
                    u16::from_le_bytes(bytes[at + 282..at + 284].try_into().unwrap()).min(900),
                    u16::from_le_bytes(bytes[at + 284..at + 286].try_into().unwrap()).min(341),
                ];
            }
        }
        for i in 0..ITEM_COUNT {
            let at = 32 + WORLD_COUNT * WORLD_BYTES + i * ITEM_BYTES;
            if bytes[at] == 0 {
                continue;
            }
            if bytes[at] != 1 {
                return Err(Error::Corrupt);
            }
            let item = Item {
                object: bytes[at + 1..at + 17].try_into().unwrap(),
                path: read_label(bytes, at + 17)?,
                name: read_label(bytes, at + 114)?,
                text: read_label(bytes, at + 147)?,
                collection: bytes[at + 340],
                x: u16::from_le_bytes(bytes[at + 341..at + 343].try_into().unwrap()),
                y: u16::from_le_bytes(bytes[at + 343..at + 345].try_into().unwrap()),
                links: u16::from_le_bytes(bytes[at + 345..at + 347].try_into().unwrap()),
                parent: bytes[at + 347].checked_sub(1),
                expanded: bytes[at + 348] == 1,
            };
            if item.collection >= 4
                || item.x > 1000
                || item.y > 1000
                || item.links & (1 << i) != 0
                || item.name.get().is_empty()
                || (item.object == [0; 16] && item.text.get().is_empty())
            {
                return Err(Error::Corrupt);
            }
            state.items[i] = Some(item);
        }
        for (i, item) in state.items.iter().enumerate() {
            if let Some(item) = item {
                let mut parent = item.parent;
                let mut seen = 1u16 << i;
                while let Some(p) = parent {
                    if p >= 16 || seen & (1 << p) != 0 {
                        return Err(Error::Corrupt);
                    }
                    seen |= 1 << p;
                    let ancestor = state.items[p as usize].ok_or(Error::Corrupt)?;
                    if ancestor.collection != item.collection {
                        return Err(Error::Corrupt);
                    }
                    parent = ancestor.parent;
                }
                for j in 0..ITEM_COUNT {
                    if item.links & (1 << j) != 0
                        && !state.items[j].is_some_and(|other| other.links & (1 << i) != 0)
                    {
                        return Err(Error::Corrupt);
                    }
                }
            }
        }
        let mut canonical = state.encode(owner)?;
        if bytes[26] < 2 {
            canonical[26] = bytes[26];
            if bytes[26] == 0 {
                canonical[27..29].fill(0);
                canonical[7264..7372].fill(0);
            }
            canonical[7372..7408].fill(0);
            let sum = checksum(&canonical[..STATE_BYTES - 4]);
            canonical[STATE_BYTES - 4..].copy_from_slice(&sum.to_le_bytes());
        }
        if canonical != bytes {
            return Err(Error::Corrupt);
        }
        if bytes[26] == 0 {
            for item in state.items.iter().flatten() {
                state.categories[item.collection as usize].set(
                    [
                        b"Ideas".as_slice(),
                        b"Category 2",
                        b"Category 3",
                        b"Category 4",
                    ][item.collection as usize],
                );
            }
        }
        Ok(state)
    }
}
// ------------------------=
// FUNC: put_label
// DESC: Copies bounded labels with deterministic zero padding.
// ------------------=
fn put_label<const N: usize>(out: &mut [u8], at: usize, label: &Label<N>) {
    out[at] = label.len;
    out[at + 1..at + 1 + N].copy_from_slice(&label.bytes);
}
// ------------------------=
// FUNC: read_label
// DESC: Validates label bounds and printable content before exposing a decoded value.
// ------------------=
fn read_label<const N: usize>(bytes: &[u8], at: usize) -> Result<Label<N>, Error> {
    let len = usize::from(bytes[at]);
    if len > N {
        return Err(Error::Corrupt);
    }
    let mut label = Label::empty();
    if !label.set(&bytes[at + 1..at + 1 + len]) {
        return Err(Error::Corrupt);
    }
    Ok(label)
}
// ------------------------=
// FUNC: checksum
// DESC: Computes corruption detection over serialized metadata, not an authorization token.
// ------------------=
fn checksum(bytes: &[u8]) -> u32 {
    bytes.iter().fold(2166136261u32, |value, byte| {
        (value ^ u32::from(*byte)).wrapping_mul(16777619)
    })
}
