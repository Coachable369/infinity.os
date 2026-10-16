#[path="../kernel/runtime/native_console_input.rs"] mod native_input;
// ------------------------=
// FUNC: main
// DESC: Exercises parking, exact delivery, rejected copies, backpressure, line bounds, EOF and endpoint reset.
// ------------------=
fn main(){
    let mut input=native_input::Input::new();
    assert_eq!(input.read(10,|_|panic!()),-11);assert!(input.waiting);
    assert!(input.submit(b"2 + 3"));assert!(!input.waiting);
    assert!(!input.submit(b"lost"));
    assert_eq!(input.read(2,|_|false),-14);
    let mut received=Vec::new();
    assert_eq!(input.read(2,|b|{received.extend_from_slice(b);true}),2);
    assert_eq!(input.read(20,|b|{received.extend_from_slice(b);true}),4);
    assert_eq!(received,b"2 + 3\n");
    assert!(!input.submit(&[b'x';1024]));assert!(input.submit(b""));
    input.close();assert_eq!(input.read(2,|b|b==b"\n"),1);
    assert_eq!(input.read(2,|_|panic!()),0);assert!(!input.waiting);
    assert!(!input.submit(b"closed"));
    input=native_input::Input::new();assert!(input.submit(b"new session"));
}
