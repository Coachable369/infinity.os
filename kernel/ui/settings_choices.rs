//! The Settings presenter publishes labels before opening the list. Renderers only read this snapshot.
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
pub const CAPACITY: usize = 32;
pub const ITEM_HEIGHT: usize = 36;
const LABEL_BYTES: usize = 48;
static LABELS: [AtomicU8; CAPACITY * LABEL_BYTES] =
    [const { AtomicU8::new(0) }; CAPACITY * LABEL_BYTES];
static OWNER: AtomicUsize = AtomicUsize::new(0);
static COUNT: AtomicUsize = AtomicUsize::new(0);
static FOCUS: AtomicUsize = AtomicUsize::new(0);

// ------------------------=
// FUNC: open
// DESC: Snapshots the owner's available options without applying any setting.
// ------------------=
pub fn open(
    section: usize,
    row: usize,
    count: usize,
    selected: usize,
    mut label: impl FnMut(usize, &mut [u8; 48]),
) {
    close();
    if section >= 11 || row >= 8 {
        return;
    }
    let count = count.min(CAPACITY);
    for index in 0..count {
        let mut text = [0; LABEL_BYTES];
        label(index, &mut text);
        for (column, byte) in text.iter().enumerate() {
            LABELS[index * LABEL_BYTES + column].store(*byte, Ordering::Relaxed);
        }
    }
    COUNT.store(count, Ordering::Relaxed);
    FOCUS.store(selected.min(count.saturating_sub(1)), Ordering::Relaxed);
    OWNER.store(1 + section * 16 + row, Ordering::Release);
}

// ------------------------=
// FUNC: count
// DESC: Returns the item count only for the active list.
// ------------------=
pub fn count(section: usize, row: usize) -> usize {
    if section >= 11 || row >= 8 {
        return 0;
    }
    if OWNER.load(Ordering::Acquire) == 1 + section * 16 + row {
        COUNT.load(Ordering::Relaxed)
    } else {
        0
    }
}

// ------------------------=
// FUNC: label
// DESC: Copies one published option label for painting.
// ------------------=
pub fn label(index: usize, out: &mut [u8; 48]) -> usize {
    if index >= COUNT.load(Ordering::Acquire) {
        return 0;
    }
    for (i, b) in out.iter_mut().enumerate() {
        *b = LABELS[index * LABEL_BYTES + i].load(Ordering::Relaxed);
        if *b == 0 {
            return i;
        }
    }
    out.len()
}

// ------------------------=
// FUNC: focus
// DESC: Returns the keyboard-highlighted item without committing it.
// ------------------=
pub fn focus() -> usize {
    FOCUS.load(Ordering::Relaxed)
}

// ------------------------=
// FUNC: highlight
// DESC: Binds keyboard navigation to an available option.
// ------------------=
pub fn highlight(index: usize) {
    FOCUS.store(
        index.min(COUNT.load(Ordering::Relaxed).saturating_sub(1)),
        Ordering::Relaxed,
    );
}

// ------------------------=
// FUNC: close
// DESC: Dismisses the transient option snapshot.
// ------------------=
pub fn close() {
    OWNER.store(0, Ordering::Release);
}
