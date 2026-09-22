//! Bounded, user-owned spatial metadata. References never convey object authority.
use super::session_state::{self, DesktopSessionLayout};

pub const STATE_BYTES: usize = 8192;
pub const ITEM_COUNT: usize = 16;
pub const WORLD_COUNT: usize = 4;
pub const OVERVIEW_COUNT: usize = 10;
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
    b"Gravity Well",
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
// DESC: Fits all independent windows without allowing an extra row to overlap the action strip.
// ------------------=
pub fn overview_bounds(
    index: usize,
    focus: usize,
    zoom: u8,
    count: usize,
) -> (usize, usize, usize, usize) {
    if count <= 5 {
        return overview_card(index, focus, zoom);
    }
    let normal = (80 + index % 4 * 210, 230 + index / 4 * 175, 195, 160);
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
// FUNC: world_card
// DESC: Shares tall environment-card geometry with input dispatch.
// ------------------=
pub fn world_card(index: usize) -> (usize, usize, usize, usize) {
    (80 + index * 210, 245, 190, 450)
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
}
impl SpatialState {
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
        self.items[index] = Some(item);
        Ok(index)
    }
    // ------------------------=
    // FUNC: remove
    // DESC: Removes only spatial metadata and incoming relationships, never the referenced file.
    // ------------------=
    pub fn remove(&mut self, owner: [u8; 16], index: usize) -> Result<(), Error> {
        self.authorize(owner)?;
        self.items
            .get_mut(index)
            .ok_or(Error::Missing)?
            .take()
            .ok_or(Error::Missing)?;
        for item in self.items.iter_mut().flatten() {
            item.links &= !(1u16 << index);
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
        let item = self
            .items
            .get_mut(index)
            .and_then(Option::as_mut)
            .ok_or(Error::Missing)?;
        item.collection = group;
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
        for (i, world) in self.worlds.iter().enumerate() {
            let at = 32 + i * WORLD_BYTES;
            put_label(&mut out, at, &world.name);
            put_label(&mut out, at + 25, &world.location);
            put_label(&mut out, at + 300, &world.editor);
            if let Some(layout) = world.layout {
                out[at + 122] = 1;
                session_state::write_layout(&mut out, at + 123, layout);
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
                for j in 0..ITEM_COUNT {
                    if item.links & (1 << j) != 0
                        && !state.items[j].is_some_and(|other| other.links & (1 << i) != 0)
                    {
                        return Err(Error::Corrupt);
                    }
                }
            }
        }
        if state.encode(owner)? != bytes {
            return Err(Error::Corrupt);
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
