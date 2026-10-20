//! Interior-only editor appearance, persisted independently of system window chrome.
use core::sync::atomic::{AtomicU32, Ordering};
pub const SIZES: [usize; 9] = [8, 10, 12, 14, 16, 18, 20, 22, 24];
pub const SIZE_NAMES: [&[u8]; 9] = [b"8 px", b"10 px", b"12 px", b"14 px", b"16 px", b"18 px", b"20 px", b"22 px", b"24 px"];
pub const DEFAULT_SIZE: u8 = 3;
pub const FONTS: [&[u8]; 4] = [
    b"JetBrains Mono",
    b"Fira Code",
    b"IBM Plex Mono",
    b"Inconsolata",
];
pub const FONT_IDS: [u8; 4] = [8, 7, 10, 24];
pub const NAMES: [&[u8]; 10] = [
    b"Infinity",
    b"Charcoal",
    b"Monokai Night",
    b"Dracula Night",
    b"Nord",
    b"Solar Dark",
    b"Solar Light",
    b"Paper",
    b"Ocean",
    b"Forest",
];
pub const PALETTES: [[u32; 9]; 10] = [
    [
        0x090f26, 0x101b3a, 0x1a2e63, 0x244a86, 0xffffff, 0x86c7ff, 0xc2dbf5, 0x6d99ff, 0xbebebe,
    ],
    [
        0x202124, 0x292b2f, 0x30343b, 0x405474, 0xe8eaed, 0x8ab4f8, 0x81c995, 0xfdd663, 0x9aa0a6,
    ],
    [
        0x272822, 0x20211c, 0x33352c, 0x49483e, 0xf8f8f2, 0xf92672, 0xe6db74, 0xae81ff, 0xa6a98c,
    ],
    [
        0x282a36, 0x21222c, 0x343746, 0x44475a, 0xf8f8f2, 0xff79c6, 0xf1fa8c, 0xbd93f9, 0x99a9d0,
    ],
    [
        0x2e3440, 0x272d38, 0x3b4252, 0x434c5e, 0xeceff4, 0x88c0d0, 0xa3be8c, 0xb48ead, 0xa5b4ca,
    ],
    [
        0x2b36, 0x212b, 0x73642, 0x164b55, 0x93a1a1, 0x2aa198, 0xb6c53b, 0xd0a244, 0x839496,
    ],
    [
        0xfdf6e3, 0xeee8d5, 0xf4edda, 0xd8dfc5, 0x374f59, 0x6f84, 0x566900, 0x934f00, 0x657b83,
    ],
    [
        0xfafafa, 0xeeeeee, 0xeef2f8, 0xcbdcf5, 0x252a34, 0x8b238f, 0x28642e, 0x8c4a00, 0x606a78,
    ],
    [
        0xf2433, 0xb1c28, 0x163548, 0x24556c, 0xd9edf7, 0x66d9ef, 0xb3e88d, 0xffc46b, 0x89aabb,
    ],
    [
        0x15241e, 0x101c17, 0x20372a, 0x35543c, 0xe3ecd9, 0xa7d785, 0xf0d890, 0xeaa77c, 0x94ad9d,
    ],
];
const RECORD: usize = 4 + 24 * 32 * 95;
const DATA: &[u8; RECORD * FONTS.len() * SIZES.len()] = include_bytes!("../../assets/fonts/InfinityEditor-library.bin");
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preferences {
    pub scheme: u8,
    pub font: u8,
    pub size: u8,
}
static CURRENT: AtomicU32 = AtomicU32::new((DEFAULT_SIZE as u32) << 16);
impl Preferences {
    // ------------------------=
    // FUNC: new
    // DESC: Returns the kit's default fourteen-pixel JetBrains Mono on Infinity.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            scheme: 0,
            font: 0,
            size: DEFAULT_SIZE,
        }
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes a bounded versioned per-profile appearance record.
    // ------------------=
    pub fn encode(self) -> [u8; 8] {
        [b'I', b'E', b'D', 2, self.scheme, self.font, self.size, 0]
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Migrates prior size indices without changing their pixel size and rejects unknown versions or invalid appearance values.
    // ------------------=
    pub fn decode(b: &[u8]) -> Option<Self> {
        if b.len() != 8
            || b[..3] != [b'I', b'E', b'D']
            || !matches!(b[3],1|2)
            || b[4] >= 10
            || b[5] >= 4
            || b[6] as usize >= if b[3]==1 {6} else {SIZES.len()}
            || b[7] != 0
        {
            return None;
        }
        Some(Self {
            scheme: b[4],
            font: b[5],
            size: if b[3]==1 {[2,3,4,5,6,8][b[6] as usize]} else {b[6]},
        })
    }
    // ------------------------=
    // FUNC: zoom_shortcut
    // DESC: Resolves repeated Control or Command zoom keys through every native size, retaining the selected scheme and font.
    // ------------------=
    pub fn zoom_shortcut(self,key:u8) -> Option<Self> {
        let size=match key {
            b'+'|b'='=>(self.size as usize+1).min(SIZES.len()-1) as u8,
            b'-'|b'_' => self.size.saturating_sub(1),
            b'0'=>DEFAULT_SIZE,
            _=>return None,
        };
        Some(Self{size,..self})
    }
    // ------------------------=
    // FUNC: record
    // DESC: Resolves exact-size native glyph pixels without decoding during paint.
    // ------------------=
    pub fn record(self) -> &'static [u8] {
        let start = (self.font.min(3) as usize * SIZES.len() + (self.size as usize).min(SIZES.len()-1)) * RECORD;
        &DATA[start..start + RECORD]
    }
    // ------------------------=
    // FUNC: cell
    // DESC: Returns the same cell width used by selection and rasterization.
    // ------------------=
    pub fn cell(self) -> usize {
        self.record()[0] as usize
    }
    // ------------------------=
    // FUNC: line
    // DESC: Adds four pixels of consistent leading to actual glyph height.
    // ------------------=
    pub fn line(self) -> usize {
        self.record()[1] as usize + 4
    }
    // ------------------------=
    // FUNC: color
    // DESC: Resolves an interior semantic color from the selected scheme.
    // ------------------=
    pub fn color(self, role: usize) -> (u8, u8, u8) {
        let c = PALETTES[self.scheme.min(9) as usize][role.min(8)];
        ((c >> 16) as u8, (c >> 8) as u8, c as u8)
    }
}
// ------------------------=
// FUNC: current
// DESC: Reads the serialized UI thread's active appearance snapshot.
// ------------------=
pub fn current() -> Preferences {
    let n = CURRENT.load(Ordering::Acquire);
    Preferences {
        scheme: n as u8,
        font: (n >> 8) as u8,
        size: (n >> 16) as u8,
    }
}
// ------------------------=
// FUNC: publish
// DESC: Publishes only valid preference records after durable commit.
// ------------------=
pub fn publish(p: Preferences) {
    if Preferences::decode(&p.encode()).is_some() {
        CURRENT.store(
            p.scheme as u32 | ((p.font as u32) << 8) | ((p.size as u32) << 16),
            Ordering::Release,
        );
    }
}
