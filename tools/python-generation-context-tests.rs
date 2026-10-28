//! Behavioral selection, prompt capacity and exact supplied-objective preservation.
use crate::ui::app_assistant::{Panel, editor_compile_intent::SourceLanguage,
    python_generation_context::{self as context, Family, Interface}};

// ------------------------=
// FUNC: selected
// DESC: Checks typed functional selection independently of generated prompt wording.
// ------------------=
fn selected() {
    for (request, expected, interface) in [
        (b"Create Hello world in Python".as_slice(), [None, None], Interface::Terminal),
        (b"Create a Python arithmetic calculator", [Some(Family::Arithmetic), None], Interface::Terminal),
        (b"Create a graphical Python calendar app", [Some(Family::Dates), None], Interface::NativeUi),
        (b"Create a Python CSV parser", [Some(Family::Data), Some(Family::Text)], Interface::Terminal),
        (b"Create a Python compression tool", [Some(Family::Compression), None], Interface::Terminal),
        (b"Create a Python game", [Some(Family::Models), None], Interface::Terminal),
        (b"Create a graphical Python game and play me", [Some(Family::Models), None], Interface::NativeUi),
        (b"Create a Python SQL database app", [Some(Family::InMemorySql), None], Interface::Terminal),
    ] {
        let choice = context::select(request, b"");
        assert_eq!(choice.families, expected);
        assert_eq!(choice.interface, interface);
        assert_eq!(choice.game, request == b"Create a Python game"
            || request == b"Create a graphical Python game and play me");
        assert_eq!(choice.opponent, request == b"Create a graphical Python game and play me");
        let mut bytes = [0xa5; context::CONTEXT_BYTES + 8];
        let size = context::format(choice, &mut bytes[..context::CONTEXT_BYTES]).unwrap();
        assert!(size <= if interface == Interface::Terminal { if choice.game { 800 } else { 600 } } else { 1850 });
        assert!(bytes[size..].iter().all(|b| *b == 0xa5));
        for capacity in 0..size {
            let mut short = [0xa5; context::CONTEXT_BYTES];
            assert!(context::format(choice, &mut short[..capacity]).is_none());
            assert!(short.iter().all(|b| *b == 0xa5));
        }
    }
    let preserved = context::select(b"Fix the code", b"import infinity as ui\nfrom datetime import date\nimport csv\n");
    assert_eq!(preserved.interface, Interface::NativeUi);
    assert_eq!(preserved.families, [Some(Family::Dates), Some(Family::Data)]);
    assert!(!preserved.opponent);
    assert!(!preserved.game);
    let bounded = context::select(b"Create Python arithmetic calendar csv text compression sql app", b"");
    assert_eq!(bounded.families, [Some(Family::Arithmetic), Some(Family::Dates)]);
    assert_eq!(context::select(b"Create a graphical terminal Python app", b"").interface, Interface::Terminal);
    // Changing an unrelated description does not drop an existing native UI.
    assert_eq!(context::select(b"Fix the terminal output", b"import infinity\n").interface, Interface::NativeUi);
}

// ------------------------=
// FUNC: literal_imports
// DESC: Distinguishes real import roots from comments, strings, aliases and names imported from unrelated packages.
// ------------------=
fn literal_imports() {
    for source in [b"import infinity as ui\n".as_slice(), b"from infinity import button\n",
        b"import os, infinity\n", b"import os, \\\n infinity\n", b"def f():\n import infinity\n",
        b"import os; import infinity\n"] {
        assert!(context::imports(source, b"infinity"));
    }
    for source in [b"# import infinity\n".as_slice(), b"message='import infinity'\n",
        b"message=\"\"\"from infinity import button\n\"\"\"\n",
        b"from example import infinity\n", b"import example as infinity\n",
        b"from .infinity import button\n", b"import infinity_other\n",
        b"f'from infinity import button'\n", b"\xff\xfe", b""] {
        assert!(!context::imports(source, b"infinity"));
        assert_eq!(context::select(b"Fix code", source).interface, Interface::Terminal);
    }
    assert!(context::imports(b"from json.decoder import JSONDecoder\n", b"json"));
    assert!(!context::imports(b"from json.decoder import JSONDecoder\n", b"JSONDecoder"));
}

// ------------------------=
// FUNC: prompt_directory
// DESC: Exercises production generation, repair and update assembly while checking exact original data and atomic bounded insertion.
// ------------------=
fn prompt_directory() {
    let source = b"import calendar\nprint(calendar.month(2026, 10))\n";
    for (request, mode) in [(b"Create a Python calendar app".as_slice(), 0),
        (b"Fix the code", 1), (b"Update the calendar app and add week numbers", 2)] {
        let mut panel = Panel::new();
        panel.source_language = SourceLanguage::Python;
        panel.input[..request.len()].copy_from_slice(request); panel.length = request.len();
        panel.request[..request.len()].copy_from_slice(request); panel.request_len = request.len();
        let mut output = [0xa5; 6144];
        let size = match mode {
            0 => panel.generation_prompt(true, b"", &mut output),
            1 => panel.compiler_repair_prompt(source, &mut output),
            _ => panel.update_prompt(source, &mut output),
        }.unwrap();
        let before = output;
        // Binary fixture models sanitized directory data; it is not a prose oracle.
        let directory = [0x31, 0x32, 0x33, 0x34];
        let end = context::append_directory(&mut output, size, request, &directory).unwrap();
        assert_eq!(end, size + directory.len());
        assert_eq!(&output[end - request.len()..end], request);
        assert!(output[end..].iter().all(|b| *b == 0xa5));
        let insertion = size - request.len() - b"\nUSER REQUEST:\n".len();
        assert_eq!(&output[..insertion], &before[..insertion]);
        assert_eq!(&output[insertion..insertion + directory.len()], &directory);
        assert_eq!(&output[insertion + directory.len()..end], &before[insertion..size]);
        for bad_request in [b"other".as_slice(), &request[..request.len() - 1]] {
            let snapshot = output;
            assert!(context::append_directory(&mut output, end, bad_request, &directory).is_none());
            assert_eq!(output, snapshot);
        }
        let snapshot = output;
        assert!(context::append_directory(&mut output[..end], end, request, &directory).is_none());
        assert_eq!(output, snapshot);
        assert!(context::append_directory(&mut output, end, request, &[0; 513]).is_none());
        assert_eq!(output, snapshot);
    }
}

// ------------------------=
// FUNC: repair_source_boundaries
// DESC: Preserves complete repair bytes, including an unterminated final line, between distinct prompt sections without merging policy into code.
// ------------------=
fn repair_source_boundaries() {
    let mut panel = Panel::new();
    panel.source_language = SourceLanguage::Python;
    let request = b"Fix the code";
    panel.request[..request.len()].copy_from_slice(request);
    panel.request_len = request.len();
    for source in [b"print(42)".as_slice(), b"print(42)\n", b"print(42)\n\n"] {
        let mut output = [0xa5; 6144];
        let length = panel.compiler_repair_prompt(source, &mut output).unwrap();
        let positions: Vec<_> = output[..length].windows(source.len())
            .enumerate().filter_map(|(offset, bytes)| (bytes == source).then_some(offset)).collect();
        assert_eq!(positions.len(), 1);
        let position = positions[0];
        assert_eq!(output[position - 1], b'\n');
        assert_eq!(output[position + source.len()], b'\n');
        assert_eq!(&output[length - request.len()..length], request);
        assert!(output[length..].iter().all(|byte| *byte == 0xa5));
    }
}

// ------------------------=
// FUNC: run
// DESC: Runs bounded library selection and production prompt assembly without an LLM, imports or filesystem effects.
// ------------------=
pub fn run() { selected(); literal_imports(); prompt_directory(); repair_source_boundaries(); }
