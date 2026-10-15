#[path = "../kernel/ui/native_development.rs"]
mod native_development;

use native_development::Command;

// ------------------------=
// FUNC: main
// DESC: Verifies native developer commands accept only exact verbs and one explicit ObjectStore path.
// ------------------=
fn main() {
    assert_eq!(native_development::parse(b"cc /home/default/hello.c"), Some(Command::Compile(b"/home/default/hello.c")));
    assert_eq!(native_development::parse(b"  run /home/default/hello.elf"), Some(Command::Run(b"/home/default/hello.elf")));
    assert_eq!(native_development::parse(b"cc-status"), Some(Command::Status));
    assert_eq!(native_development::parse(b" cc-cancel  "), Some(Command::Cancel));
    assert_eq!(native_development::parse(b"cc"), None);
    assert_eq!(native_development::parse(b"cc /home/default/a.c extra"), None);
    assert_eq!(native_development::parse(b"compile /home/default/a.c"), None);
    assert_eq!(native_development::parse(b"cc-status extra"), None);
    assert_eq!(native_development::parse(b"cc /home/default/a.c\0suffix"), None);
}
