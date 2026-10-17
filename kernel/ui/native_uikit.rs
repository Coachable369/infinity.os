//! Bounded declarative UI Kit state for hardware-isolated native C applications.
//! The OS owns styling and hit testing; applications submit only semantic elements.

pub const MAX_ELEMENTS: usize = 32;
pub const TITLE_CAPACITY: usize = 48;
pub const LABEL_CAPACITY: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementKind {
    Label = 1,
    Display = 2,
    Button = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Element {
    pub id: u32,
    pub kind: ElementKind,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub label: [u8; LABEL_CAPACITY],
    pub label_length: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub kind: u32,
    pub id: u32,
    pub value: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Presentation {
    pub active: bool,
    pub width: u32,
    pub height: u32,
    pub title: [u8; TITLE_CAPACITY],
    pub title_length: usize,
    pub elements: [Option<Element>; MAX_ELEMENTS],
    pub revision: u64,
}

impl Presentation {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an inactive bounded application presentation.
    // ------------------=
    pub const fn new() -> Self {
        Self { active: false, width: 480, height: 420, title: [0; TITLE_CAPACITY],
            title_length: 0, elements: [None; MAX_ELEMENTS], revision: 0 }
    }
}

struct State {
    presentation: Presentation,
    events: [Option<Event>; 16],
    event_head: usize,
    event_length: usize,
}

impl State {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty UI Kit state and event queue.
    // ------------------=
    const fn new() -> Self {
        Self { presentation: Presentation::new(), events: [None; 16], event_head: 0,
            event_length: 0 }
    }

    // ------------------------=
    // FUNC: push
    // DESC: Queues one semantic event without overwriting an unconsumed interaction.
    // ------------------=
    fn push(&mut self, event: Event) -> bool {
        if self.event_length == self.events.len() { return false; }
        let index = (self.event_head + self.event_length) % self.events.len();
        self.events[index] = Some(event);
        self.event_length += 1;
        true
    }
}

static mut STATE: State = State::new();

#[no_mangle]
pub static mut INFINITY_NATIVE_UI_SNAPSHOT: [u64; 11] =
    [0x494e4655494b4954, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0];

// ------------------------=
// FUNC: publish
// DESC: Publishes a seqlocked semantic UI state for installed-runtime behavioral acceptance.
// ------------------=
fn publish(presentation: Presentation, event: Option<Event>) { unsafe {
    let sequence = (&raw const INFINITY_NATIVE_UI_SNAPSHOT[2]).read_volatile()
        .wrapping_add(2) & !1;
    (&raw mut INFINITY_NATIVE_UI_SNAPSHOT[2]).write_volatile(sequence | 1);
    (&raw mut INFINITY_NATIVE_UI_SNAPSHOT[3]).write_volatile(presentation.active as u64);
    (&raw mut INFINITY_NATIVE_UI_SNAPSHOT[4]).write_volatile(presentation.width as u64);
    (&raw mut INFINITY_NATIVE_UI_SNAPSHOT[5]).write_volatile(presentation.height as u64);
    (&raw mut INFINITY_NATIVE_UI_SNAPSHOT[6]).write_volatile(
        presentation.elements.iter().filter(|element| element.is_some()).count() as u64);
    (&raw mut INFINITY_NATIVE_UI_SNAPSHOT[7]).write_volatile(presentation.revision);
    (&raw mut INFINITY_NATIVE_UI_SNAPSHOT[8]).write_volatile(event.map_or(0, |value| value.kind as u64));
    (&raw mut INFINITY_NATIVE_UI_SNAPSHOT[9]).write_volatile(event.map_or(0, |value| value.id as u64));
    (&raw mut INFINITY_NATIVE_UI_SNAPSHOT[10]).write_volatile(sequence);
    (&raw mut INFINITY_NATIVE_UI_SNAPSHOT[2]).write_volatile(sequence);
}}

// ------------------------=
// FUNC: open
// DESC: Opens one theme-owned native app canvas with validated logical geometry.
// ------------------=
pub fn open(width: u32, height: u32, title: &[u8]) -> bool { unsafe {
    if !(320..=960).contains(&width) || !(240..=720).contains(&height)
        || title.is_empty() || title.len() > TITLE_CAPACITY || core::str::from_utf8(title).is_err() {
        return false;
    }
    let state = &mut *(&raw mut STATE);
    state.presentation = Presentation::new();
    state.presentation.active = true;
    state.presentation.width = width;
    state.presentation.height = height;
    state.presentation.title[..title.len()].copy_from_slice(title);
    state.presentation.title_length = title.len();
    state.presentation.revision = 1;
    state.events = [None; 16]; state.event_head = 0; state.event_length = 0;
    publish(state.presentation, None);
    true
}}

// ------------------------=
// FUNC: upsert
// DESC: Creates or updates one semantic UI Kit element by stable application ID.
// ------------------=
pub fn upsert(element: Element) -> bool { unsafe {
    let state = &mut *(&raw mut STATE);
    let presentation = &mut state.presentation;
    if !presentation.active || element.id == 0 || element.width == 0 || element.height == 0
        || element.label_length > LABEL_CAPACITY
        || element.x.saturating_add(element.width) > presentation.width
        || element.y.saturating_add(element.height) > presentation.height
        || core::str::from_utf8(&element.label[..element.label_length]).is_err() { return false; }
    let index = presentation.elements.iter().position(|slot|slot.is_some_and(|item|item.id==element.id))
        .or_else(||presentation.elements.iter().position(Option::is_none));
    let Some(index) = index else { return false; };
    presentation.elements[index] = Some(element);
    presentation.revision = presentation.revision.wrapping_add(1).max(1);
    publish(*presentation, None);
    true
}}

// ------------------------=
// FUNC: activate
// DESC: Converts one logical pointer activation into a stable button event.
// ------------------=
pub fn activate(x: u32, y: u32) -> bool { unsafe {
    let state = &mut *(&raw mut STATE);
    let Some(element) = state.presentation.elements.iter().rev().flatten().find(|element|
        element.kind == ElementKind::Button && x >= element.x && y >= element.y
            && x < element.x.saturating_add(element.width)
            && y < element.y.saturating_add(element.height)).copied() else { return false; };
    let event = Event { kind: 1, id: element.id, value: 0 };
    let accepted = state.push(event);
    if accepted { publish(state.presentation, Some(event)); }
    accepted
}}

// ------------------------=
// FUNC: request_close
// DESC: Queues a close request for the owning native application.
// ------------------=
pub fn request_close() -> bool { unsafe {
    let state = &mut *(&raw mut STATE);
    let event = Event { kind: 2, id: 0, value: 0 };
    let accepted = state.presentation.active && state.push(event);
    if accepted { publish(state.presentation, Some(event)); }
    accepted
}}

// ------------------------=
// FUNC: poll
// DESC: Removes and returns the oldest pending UI interaction.
// ------------------=
pub fn poll() -> Option<Event> { unsafe {
    let state = &mut *(&raw mut STATE);
    if state.event_length == 0 { return None; }
    let event = state.events[state.event_head].take();
    state.event_head = (state.event_head + 1) % state.events.len();
    state.event_length -= 1;
    event
}}

// ------------------------=
// FUNC: presentation
// DESC: Returns a copyable render snapshot without exposing mutable application state.
// ------------------=
pub fn presentation() -> Presentation { unsafe { (&*(&raw const STATE)).presentation } }

// ------------------------=
// FUNC: close
// DESC: Clears UI elements and pending input when the native process terminates.
// ------------------=
pub fn close() { unsafe {
    *(&mut *(&raw mut STATE)) = State::new();
    publish(Presentation::new(), None);
} }
