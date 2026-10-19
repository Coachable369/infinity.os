#[path = "../../kernel/runtime/native_arguments.rs"] mod arguments;
use arguments::Arguments;

// ------------------------=
// FUNC: main
// DESC: Verifies owned argv lifetime, empty arguments, capacity and embedded-NUL rejection.
// ------------------=
fn main() {
    let mut path = b"/documents/hello.py".to_vec();
    let args = Arguments::new(&[b"python", &path, b"", b"some value"]).unwrap();
    path.fill(0);
    let moved = args;
    assert_eq!(moved.count(), 4);
    assert_eq!(moved.get(1), Some(b"/documents/hello.py".as_slice()));
    assert_eq!(moved.get(2), Some(b"".as_slice()));
    assert_eq!(moved.get(3), Some(b"some value".as_slice()));
    assert_eq!(moved.get(4), None);
    assert!(Arguments::new(&[]).is_none());
    assert!(Arguments::new(&[b"bad\0argument"]).is_none());
    assert!(Arguments::new(&[&[1; 2049]]).is_none());
    assert!(Arguments::new(&[&[1; 2048]]).is_some());
    assert!(Arguments::new(&[b"".as_slice(); 33]).is_none());
    assert!(Arguments::new(&[b"".as_slice(); 32]).is_some());
}
