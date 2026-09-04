//! Host-side compiler for bounded InfinityOS native skin packages.

use std::{env, fs, path::Path};

const MAX_MANIFEST_BYTES: usize = 32 * 1024;
const MAX_WALLPAPER_BYTES: usize = 16 * 1024 * 1024;
const MAX_ICON_SOURCE_BYTES: usize = 256 * 1024;

// ------------------------=
// FUNC: main
// DESC: Validates one source skin tree and emits its deterministic native package.
// ------------------=
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: skin-compiler <skin-source-directory> <output.iskin>");
        std::process::exit(2);
    }
    if let Err(error) = compile(Path::new(&args[1]), Path::new(&args[2])) {
        eprintln!("skin-compiler: {error}");
        std::process::exit(1);
    }
}

// ------------------------=
// FUNC: compile
// DESC: Converts trusted human projections and SVG source into an explicit binary package.
// ------------------=
fn compile(source: &Path, output: &Path) -> Result<(), String> {
    let manifest = fs::read(source.join("manifest.infskin")).map_err(|error| error.to_string())?;
    if manifest.len() > MAX_MANIFEST_BYTES {
        return Err("manifest exceeds bounded size".into());
    }
    let manifest_text = std::str::from_utf8(&manifest).map_err(|_| "manifest is not UTF-8")?;
    let id = required_value(manifest_text, "skin_id")?;
    let version = required_value(manifest_text, "format_version")?.parse::<u16>().map_err(|_| "invalid format_version")?;
    let abi = required_value(manifest_text, "minimum_ui_abi")?.parse::<u16>().map_err(|_| "invalid minimum_ui_abi")?;
    if version != 1 || abi > 1 || id.len() > 32 || id.is_empty() {
        return Err("unsupported skin identity or schema".into());
    }
    let wallpaper_path = optional_value(manifest_text, "wallpaper");
    let wallpaper = if let Some(relative) = wallpaper_path {
        let bytes = fs::read(source.join(relative)).map_err(|error| error.to_string())?;
        if bytes.len() > MAX_WALLPAPER_BYTES || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("wallpaper is not a bounded PNG".into());
        }
        bytes
    } else {
        Vec::new()
    };
    let icon_source_path = optional_value(manifest_text, "icon_source");
    let icon_pack = if let Some(relative) = icon_source_path {
        let bytes = fs::read(source.join(relative)).map_err(|error| error.to_string())?;
        compile_icons(&bytes)?
    } else {
        Vec::new()
    };
    let content_hash = checksum64(&manifest) ^ checksum64(&wallpaper).rotate_left(11) ^ checksum64(&icon_pack).rotate_left(23);
    let mut package = vec![0u8; 64];
    package[..8].copy_from_slice(b"INFSKIN1");
    package[8..10].copy_from_slice(&version.to_le_bytes());
    package[10..12].copy_from_slice(&abi.to_le_bytes());
    package[12..20].copy_from_slice(&content_hash.to_le_bytes());
    package[20] = id.len() as u8;
    package[21..21 + id.len()].copy_from_slice(id.as_bytes());
    package[54] = 1;
    let header_hash = checksum32(&package[..60]);
    package[60..64].copy_from_slice(&header_hash.to_le_bytes());
    append_entry(&mut package, 1, b"manifest", &manifest);
    if !wallpaper.is_empty() {
        append_entry(&mut package, 2, b"wallpaper", &wallpaper);
    }
    if !icon_pack.is_empty() {
        append_entry(&mut package, 3, b"semantic-icons", &icon_pack);
    }
    fs::write(output, package).map_err(|error| error.to_string())
}

// ------------------------=
// FUNC: required_value
// DESC: Resolves a required typed manifest key without accepting duplicates.
// ------------------=
fn required_value<'a>(manifest: &'a str, key: &str) -> Result<&'a str, String> {
    optional_value(manifest, key).ok_or_else(|| format!("missing {key}"))
}

// ------------------------=
// FUNC: optional_value
// DESC: Resolves one exact key from the bounded human-readable manifest projection.
// ------------------=
fn optional_value<'a>(manifest: &'a str, key: &str) -> Option<&'a str> {
    manifest.lines().find_map(|line| line.split_once('=').filter(|(name, _)| *name == key).map(|(_, value)| value.trim()))
}

// ------------------------=
// FUNC: compile_icons
// DESC: Rejects unsafe SVG features and compiles semantic symbols into XML-free native vector records.
// ------------------=
fn compile_icons(source: &[u8]) -> Result<Vec<u8>, String> {
    if source.len() > MAX_ICON_SOURCE_BYTES {
        return Err("icon source exceeds bounded size".into());
    }
    let text = std::str::from_utf8(source).map_err(|_| "icon source is not UTF-8")?;
    for denied in ["<script", "foreignObject", "href=", "url(", "filter=", "<!ENTITY", "<?xml"] {
        if text.contains(denied) {
            return Err(format!("unsafe SVG feature: {denied}"));
        }
    }
    let mut output = b"IVEC1".to_vec();
    let mut count = 0u16;
    output.extend_from_slice(&count.to_le_bytes());
    for line in text.lines().filter(|line| line.contains("<symbol id=")) {
        let id = attribute(line, "id").ok_or("symbol missing id")?;
        if id.is_empty() || id.len() > 32 {
            return Err("invalid semantic icon id".into());
        }
        let geometry = line.split_once('>').and_then(|(_, rest)| rest.rsplit_once("</symbol>").map(|(value, _)| value)).ok_or("malformed symbol")?;
        if !(geometry.contains("<path") || geometry.contains("<circle") || geometry.contains("<rect") || geometry.contains("<ellipse")) {
            return Err("symbol has no supported geometry".into());
        }
        let native = geometry.replace('<', "[").replace('>', "]");
        output.push(id.len() as u8);
        output.extend_from_slice(id.as_bytes());
        output.extend_from_slice(&(native.len() as u16).to_le_bytes());
        output.extend_from_slice(native.as_bytes());
        count = count.saturating_add(1);
    }
    if count < 40 {
        return Err("default skin requires at least 40 semantic icons".into());
    }
    output[5..7].copy_from_slice(&count.to_le_bytes());
    Ok(output)
}

// ------------------------=
// FUNC: attribute
// DESC: Extracts a quoted attribute from one already bounded SVG source line.
// ------------------=
fn attribute<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("{name}=\"");
    let tail = line.split_once(&marker)?.1;
    Some(tail.split_once('"')?.0)
}

// ------------------------=
// FUNC: append_entry
// DESC: Appends one typed length-delimited package entry and its integrity hash.
// ------------------=
fn append_entry(package: &mut Vec<u8>, kind: u16, name: &[u8], data: &[u8]) {
    package.extend_from_slice(&kind.to_le_bytes());
    package.extend_from_slice(&(name.len() as u16).to_le_bytes());
    package.extend_from_slice(&(data.len() as u32).to_le_bytes());
    package.extend_from_slice(&checksum64(data).to_le_bytes());
    package.extend_from_slice(name);
    package.extend_from_slice(data);
}

// ------------------------=
// FUNC: checksum32
// DESC: Calculates the package-header FNV-1a integrity value used by the runtime parser.
// ------------------=
fn checksum32(bytes: &[u8]) -> u32 {
    bytes.iter().fold(2_166_136_261u32, |hash, byte| (hash ^ *byte as u32).wrapping_mul(16_777_619))
}

// ------------------------=
// FUNC: checksum64
// DESC: Calculates a deterministic content hash for native skin resources.
// ------------------=
fn checksum64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(14_695_981_039_346_656_037u64, |hash, byte| (hash ^ *byte as u64).wrapping_mul(1_099_511_628_211))
}
