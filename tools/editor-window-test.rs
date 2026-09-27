#![allow(dead_code)]
#[path = "../kernel/ui/mod.rs"]
mod ui;
use ui::geometry::{Point, Rect};
#[path = "storage.rs"]
mod storage;
use std::{cell::RefCell, rc::Rc};
use storage::{
    object::{ObjectStore, ObjectType, Space},
    BlockDevice,
};
#[derive(Clone)]
struct MemoryDisk(Rc<RefCell<Vec<[u8; 512]>>>);

impl MemoryDisk {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a deterministic in-memory block device for behavioral Object Store tests.
    // ------------------=
    fn new(sectors: usize) -> Self {
        Self(Rc::new(RefCell::new(vec![[0; 512]; sectors])))
    }
}

impl BlockDevice for MemoryDisk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Returns the mock device capacity.
    // ------------------=
    fn block_count(&self) -> u64 {
        self.0.borrow().len() as u64
    }

    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads one exact mock sector.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        let Some(sector) = self.0.borrow().get(lba as usize).copied() else {
            return false;
        };
        *out = sector;
        true
    }

    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes one exact mock sector.
    // ------------------=
    fn write_sector(&mut self, lba: u64, input: &[u8; 512]) -> bool {
        let mut disk = self.0.borrow_mut();
        let Some(sector) = disk.get_mut(lba as usize) else {
            return false;
        };
        *sector = *input;
        true
    }

    // ------------------------=
    // FUNC: flush
    // DESC: Commits immediately in the deterministic mock device.
    // ------------------=
    fn flush(&mut self) -> bool {
        true
    }
}

// ------------------------=
// FUNC: editor_pool_roundtrip
// DESC: Saves distinct filenames and locations, browses real references, edits content, and verifies a remounted Pool.
// ------------------=
fn editor_pool_roundtrip() {
    let sectors = storage::object::STORE_RELATIVE_LBA as usize + 32_768;
    let disk = MemoryDisk::new(sectors);
    let mut store = ObjectStore::format(disk.clone(), 0, sectors as u64, [0x41; 16]).unwrap();
    let mut picker = ui::object_picker::Picker::new();
    assert!(picker.set_location(b"/personal/projects"));
    let path = picker.destination(b"report.txt").unwrap();
    let id = store
        .create_attached(
            b"report.txt",
            ObjectType::Text,
            Space::Personal,
            b"first",
            path.bytes(),
        )
        .unwrap();
    assert!(picker.set_location(b"/personal/archive"));
    let other_path = picker.destination(b"report.txt").unwrap();
    let other = store
        .create_attached(
            b"report.txt",
            ObjectType::Text,
            Space::Personal,
            b"independent",
            other_path.bytes(),
        )
        .unwrap();
    assert_ne!(id, other);
    assert!(store
        .create_attached(
            b"report.txt",
            ObjectType::Text,
            Space::Personal,
            b"replacement",
            other_path.bytes()
        )
        .is_err());
    store.write(id, b"edited and saved").unwrap();
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    assert!(picker.set_location(b"/personal/projects"));
    for index in 0..256 {
        let Some(entry) = store.namespace_list_nth(picker.location.bytes(), index) else {
            break;
        };
        picker.add(&entry.path[..entry.path_len as usize], false);
    }
    assert_eq!(picker.count, 1);
    let reopened = store.resolve(picker.entries[0].bytes()).unwrap();
    assert_eq!(reopened, id);
    let mut bytes = [0; 128];
    let result = store.read(reopened, None, &mut bytes).unwrap();
    assert_eq!(&bytes[..result], b"edited and saved");
    let result = store.read(other, None, &mut bytes).unwrap();
    assert_eq!(&bytes[..result], b"independent");
}
// ------------------------=
// FUNC: main
// DESC: Exercises Pool path creation, folder browsing, bounds, and shared window hit/raise ordering without text or source oracles.
// ------------------=
fn main() {
    editor_pool_roundtrip();
    let mut p = ui::object_picker::Picker::new();
    assert!(p.set_location(b"/"));
    p.add(b"/personal/documents/first.txt", false);
    p.add(b"/personal/documents/second.txt", false);
    p.add(b"/home/default", true);
    assert_eq!(p.count, 2);
    assert!(p.entries[0].folder);
    let folder = p.entries[0];
    assert!(p.set_location(folder.bytes()));
    p.add(b"/personal/documents/first.txt", false);
    assert_eq!(p.count, 1);
    assert!(p.entries[0].folder);
    let folder = p.entries[0];
    assert!(p.set_location(folder.bytes()));
    p.add(b"/personal/documents/first.txt", false);
    assert!(!p.entries[0].folder);
    assert_eq!(
        p.destination(b"unique.txt").unwrap().bytes(),
        b"/personal/documents/unique.txt"
    );
    assert!(p.destination(b"../escape").is_none());
    assert!(!p.set_location(b"/personal/../system"));
    p.parent();
    assert_eq!(p.location.bytes(), b"/personal");
    p.parent();
    p.parent();
    assert_eq!(p.location.bytes(), b"/");
    assert_eq!(p.destination(b"root.txt").unwrap().bytes(), b"/root.txt");
    for scale in 1..=3 {
        let content = Rect {
            x: 50,
            y: 60,
            width: 650 * scale,
            height: 500 * scale,
        };
        for save in [true, false] {
            let g = ui::object_picker::geometry(content, scale as usize, save);
            for r in [
                g.location, g.parent, g.list, g.previous, g.next, g.cancel, g.accept,
            ] {
                assert!(
                    r.x >= g.sheet.x
                        && r.y >= g.sheet.y
                        && r.right() <= g.sheet.right()
                        && r.bottom() <= g.sheet.bottom()
                );
            }
            assert!(g.list.bottom() <= g.previous.y);
        }
    }
    let mut stack = ui::desktop_stack::Stack::new();
    let bounds = [Rect {
        x: 0,
        y: 0,
        width: 100,
        height: 100,
    }; 5];
    for active in [4, 2, 1, 0, 3, 4, 1] {
        stack.sync([true; 5], active, 6);
        assert_eq!(stack.hit(bounds, Point { x: 50, y: 50 }), Some(active));
        let order = stack.order;
        stack.sync([true; 5], active, 6);
        assert_eq!(stack.order, order);
        assert_eq!(stack.hit(bounds, Point { x: 200, y: 200 }), None);
    }
    let before = stack.order;
    let mut visible = [true; 5];
    visible[1] = false;
    stack.sync(visible, 1, 0);
    assert_eq!(stack.order, before);
    assert_ne!(stack.hit(bounds, Point { x: 50, y: 50 }), Some(1));
    let bounds=[bounds[0];ui::desktop_stack::SURFACES];
    let mut visible=[true;ui::desktop_stack::SURFACES];
    stack.sync(visible,ui::desktop_stack::BROWSER,6);
    assert_eq!(stack.hit(bounds,Point{x:50,y:50}),Some(ui::desktop_stack::BROWSER));
    stack.sync(visible,2,6);
    assert_eq!(stack.hit(bounds,Point{x:50,y:50}),Some(2));
    stack.sync(visible,ui::desktop_stack::BROWSER,6);
    visible[ui::desktop_stack::BROWSER]=false;
    stack.sync(visible,ui::desktop_stack::BROWSER,6);
    assert_eq!(stack.hit(bounds,Point{x:50,y:50}),Some(2));
    // Existing five-surface callers must neither expose nor hit the new slot.
    stack.sync([true;5],4,6);
    assert!(!stack.visible[ui::desktop_stack::BROWSER]);
    assert_eq!(stack.hit(bounds,Point{x:50,y:50}),Some(4));
}
