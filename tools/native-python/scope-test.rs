#[path = "../../kernel/runtime/native_python_scope.rs"] mod scope;

// ------------------------=
// FUNC: main
// DESC: Verifies owner isolation and the exact shared Documents compatibility boundary.
// ------------------=
fn main() {
    let user = [1; 16];
    let path = b"/home/default/documents/hello.py";
    assert!(scope::readable(user, user, true, path));
    assert!(scope::readable([0; 16], user, true, path));
    assert!(!scope::readable([2; 16], user, true, path));
    assert!(!scope::readable([0; 16], [0; 16], true, path));
    assert!(!scope::readable([0; 16], user, false, path));
    assert!(!scope::readable(user, user, false, path));
    assert!(!scope::readable([0; 16], user, true, b"/system/private"));
    assert!(!scope::readable([0; 16], user, true, b"/home/default/documents-other/hello.py"));
}
