#[path = "../kernel/runtime/native_python_inputs.rs"] mod inputs;
use inputs::{Inputs, Metadata, Receipt, Error, BYTES, FILE_BYTES, COUNT};
static mut INPUTS: Inputs = Inputs::new();

// ------------------------=
// FUNC: metadata
// DESC: Builds a typed owned immutable version used by the actual packed snapshot implementation.
// ------------------=
fn metadata(index: u32, length: usize) -> Metadata {
    let mut object = [0;16]; object[..4].copy_from_slice(&index.to_le_bytes());
    Metadata { object, version: 7, owner: [1;16], personal: true, length }
}

// ------------------------=
// FUNC: receipt
// DESC: Returns the exact object/version/length receipt a version-pinned storage read must corroborate.
// ------------------=
fn receipt(metadata: Metadata) -> Receipt {
    Receipt { object: metadata.object, version: metadata.version, length: metadata.length, current: metadata }
}

// ------------------------=
// FUNC: main
// DESC: Exercises many small sources, immutable native bytes, bounded admission and failure-atomic owner/version reads without host or guest text oracles.
// ------------------=
fn main() {
    let inputs = unsafe { &mut *(&raw mut INPUTS) };
    for index in 1..=24 {
        let path = format!("/home/default/documents/application{index}.py");
        let source = format!("answer = {}\n", index * 7);
        let meta = metadata(index, source.len());
        inputs.capture([1;16], path.as_bytes(), meta, |target| {
            target.copy_from_slice(source.as_bytes()); Ok(receipt(meta))
        }).unwrap();
    }
    assert_eq!(inputs.count(), 24);
    for index in 0..24 {
        let file = inputs.get(index).unwrap();
        assert_eq!(file.path, format!("/home/default/documents/application{}.py", index+1).as_bytes());
        assert_eq!(file.bytes, format!("answer = {}\n", (index+1)*7).as_bytes());
        assert_eq!(file.object, metadata(index as u32 + 1, 0).object);
        assert_eq!(file.version, 7);
    }
    let extension = [0x7f, b'E', b'L', b'F', 2, 1, 1, 0];
    let meta = metadata(25, extension.len());
    inputs.capture([1;16], b"/home/default/documents/native.so", meta, |target| {
        target.copy_from_slice(&extension); Ok(receipt(meta))
    }).unwrap();
    let address = inputs.get(24).unwrap().bytes.as_ptr();
    let count = inputs.count(); let used = inputs.used();
    for mut changed in [metadata(26, 8); 4].into_iter().enumerate() {
        let original = changed.1;
        match changed.0 { 0 => changed.1.version += 1, 1 => changed.1.owner = [2;16],
            2 => changed.1.object = [9;16], _ => changed.1.personal = false }
        assert_eq!(inputs.capture([1;16], b"/home/default/documents/new.py", original, |target| {
            target.fill(0xff); Ok(Receipt { current: changed.1, ..receipt(original) })
        }), Err(Error::Changed));
        assert_eq!((inputs.count(), inputs.used()), (count, used));
        assert_eq!(inputs.get(24).unwrap().bytes, extension);
        assert_eq!(inputs.get(24).unwrap().bytes.as_ptr(), address);
    }
    let mut foreign = metadata(26, 1); foreign.owner = [2;16];
    assert_eq!(inputs.capture([1;16], b"/home/default/documents/new.py", foreign, |_| panic!("unauthorized reader invoked")), Err(Error::Authority));
    assert_eq!(inputs.capture([1;16], b"/home/default/documents/native.so", meta, |_| panic!("duplicate reader invoked")), Err(Error::Duplicate));
    assert_eq!(inputs.capture([1;16], b"/home/default/documents/new.py", metadata(26, FILE_BYTES+1), |_| panic!("oversized reader invoked")), Err(Error::FileBudget));
    assert_eq!(inputs.capture([1;16], b"/home/default/documents/new.py", metadata(26, 8), |target| { target.fill(0xa5); Err(()) }), Err(Error::Read));
    assert_eq!((inputs.count(), inputs.used()), (count, used));
    for bad in [b"relative.py".as_slice(), b"/home/../secret.py", b"/home//a.py", b"/home/a.py\0"] {
        assert_eq!(inputs.capture([1;16], bad, metadata(26, 1), |_| panic!("bad path reader invoked")), Err(Error::Path));
    }
    inputs.clear();
    for index in 1..=BYTES/FILE_BYTES {
        let path = format!("/home/default/documents/big{index}.py"); let meta = metadata(index as u32, FILE_BYTES);
        inputs.capture([1;16], path.as_bytes(), meta, |target| { target.fill(index as u8); Ok(receipt(meta)) }).unwrap();
    }
    assert_eq!(inputs.used(), BYTES);
    assert_eq!(inputs.capture([1;16], b"/home/default/documents/overflow.py", metadata(20, 1), |_| panic!("overflow reader invoked")), Err(Error::ByteBudget));
    assert_eq!(inputs.get(0).unwrap().bytes[0], 1);
    assert_eq!(inputs.get(7).unwrap().bytes[FILE_BYTES-1], 8);
    inputs.clear();
    for index in 1..=COUNT {
        let path = format!("/home/default/documents/empty{index}.py"); let meta = metadata(index as u32, 0);
        inputs.capture([1;16], path.as_bytes(), meta, |_| Ok(receipt(meta))).unwrap();
    }
    assert_eq!(inputs.capture([1;16], b"/home/default/documents/extra.py", metadata(1000, 0), |_| panic!("descriptor overflow reader invoked")), Err(Error::CountBudget));
    inputs.clear(); assert_eq!((inputs.count(), inputs.used()), (0,0)); assert!(inputs.get(0).is_none());
}
