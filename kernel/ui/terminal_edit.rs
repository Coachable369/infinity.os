//! Local terminal editor presentation, reusing the native document engine and font kit.
use super::text_editor::TextDocument;
// Opaque object bytes plus the version fence; UI never owns storage authority.
pub type Revision = Option<([u8;16],u32)>;
pub struct State {
    pub active:bool, pub closing_window:bool, pub owner:[u8;16], pub document:TextDocument,
    pub path:[u8;96], pub path_len:usize, pub top:usize,
    pub revision:Revision,
    pub memory:Revision,
    pub prompt:u8, pub query:[u8;96], pub query_len:usize,
    pub cut:[u8;4096], pub cut_len:usize, pub notice:&'static [u8],
}
impl State {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves bounded session-private editor state without allocating or invoking a host editor.
    // ------------------=
    pub const fn new()->Self {Self{active:false,closing_window:false,owner:[0;16],document:TextDocument::new(),path:[0;96],path_len:0,top:0,revision:None,memory:None,prompt:0,query:[0;96],query_len:0,cut:[0;4096],cut_len:0,notice:b""}}
}
static mut STATE:State=State::new();
// ------------------------=
// FUNC: inspect
// DESC: Borrows the BSP-owned editor for painting without copying its undo history onto the stack.
// ------------------=
pub fn inspect<T>(f:impl FnOnce(&State)->T)->T{unsafe{f(&*core::ptr::addr_of!(STATE))}}
// ------------------------=
// FUNC: mutate
// DESC: Serializes editor mutation on the same UI thread as the graphical document controller.
// ------------------=
pub fn mutate<T>(f:impl FnOnce(&mut State)->T)->T{unsafe{f(&mut *core::ptr::addr_of_mut!(STATE))}}
// ------------------------=
// FUNC: session
// DESC: Clears document, undo history and cut buffer when a different authenticated session takes ownership.
// ------------------=
pub fn session(owner:[u8;16]) {mutate(|s|{if s.owner!=owner {s.document.erase_private();s.cut.fill(0);s.query.fill(0);s.path.fill(0);s.active=false;s.owner=owner;s.path_len=0;s.cut_len=0;s.query_len=0;s.prompt=0;}});}
