//! Validates packaged Studio records with the same bounded decoders used by the OS.
#![allow(dead_code)]

#[path = "../kernel/ui/bitmap.rs"]
mod bitmap;
#[path = "../kernel/ui/installer_template.rs"]
mod installer_template;
#[path = "../kernel/ui/kit_tokens.rs"]
mod kit_tokens;
#[path = "../kernel/ui/panel_recipe.rs"]
mod panel_recipe;

use std::process::ExitCode;
use std::io::Read;

// ------------------------=
// FUNC: valid_payload
// DESC: Dispatches each supported record kind directly to its production native decoder without duplicating validation rules.
// ------------------=
fn valid_payload(kind: &str, bytes: &[u8]) -> bool {
    match kind {
        "desktop" => installer_template::InstallerTemplate::parse_desktop(bytes).is_ok(),
        "login" => installer_template::InstallerTemplate::parse_login(bytes).is_ok(),
        "boot" => installer_template::InstallerTemplate::parse_boot(bytes).is_ok(),
        "template" => installer_template::InstallerTemplate::parse(bytes).is_ok(),
        "settings" => installer_template::InstallerTemplate::parse_settings(bytes).is_ok(),
        "tokens" => kit_tokens::decode(bytes).is_some(),
        "styles" => panel_recipe::decode_styles(bytes).is_some(),
        _ => false,
    }
}

// ------------------------=
// FUNC: main
// DESC: Accepts a record kind and file path or stdin marker, returning success only when the actual native decoder accepts those exact bytes.
// ------------------=
fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let (Some(kind), Some(path), None) = (arguments.next(), arguments.next(), arguments.next()) else {
        eprintln!("usage: ui-payload-validator <boot|template|settings|tokens|styles> <file>");
        return ExitCode::from(2);
    };
    let Some(kind) = kind.to_str().filter(|kind| matches!(*kind, "desktop" | "login" | "boot" | "template" | "settings" | "tokens" | "styles")) else {
        eprintln!("Unsupported UI payload kind");
        return ExitCode::from(2);
    };
    let input = if path == "-" {
        let mut bytes = Vec::new();
        std::io::stdin().read_to_end(&mut bytes).map(|_| bytes)
    } else {
        std::fs::read(&path)
    };
    let bytes = match input {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("Cannot read UI payload: {error}");
            return ExitCode::FAILURE;
        }
    };
    if valid_payload(kind, &bytes) {
        ExitCode::SUCCESS
    } else {
        eprintln!("Native decoder rejected {kind} payload");
        ExitCode::FAILURE
    }
}
