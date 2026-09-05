use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone, Copy)]
enum ContainerKind {
    Fat,
    Iso,
}

struct Container<'a> {
    kind: ContainerKind,
    image: &'a str,
    font_root: &'a str,
    license_root: &'a str,
    skin_root: &'a str,
    wallpaper_root: &'a str,
    installer_root: &'a str,
}

// ------------------------=
// FUNC: collect_files
// DESC: Collects every regular asset below a source directory for byte-for-byte package comparison.
// ------------------=
fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("asset directory must be readable") {
            let path = entry.expect("asset entry must be readable").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

// ------------------------=
// FUNC: extract
// DESC: Extracts one packaged asset through the container format's public command-line interface.
// ------------------=
fn extract(container: &Container<'_>, packaged_path: &str, destination: &Path) {
    let status = match container.kind {
        ContainerKind::Fat => Command::new("mcopy")
            .args(["-i", container.image, &format!("::{packaged_path}")])
            .arg(destination)
            .status(),
        ContainerKind::Iso => Command::new("xorriso")
            .args(["-osirrox", "on", "-indev", container.image, "-extract", packaged_path])
            .arg(destination)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
    }
    .expect("container extraction tool must run");
    assert!(status.success(), "packaged asset must be extractable: {packaged_path}");
}

// ------------------------=
// FUNC: assert_packaged_bytes
// DESC: Verifies one packaged asset is present and byte-identical to its source asset.
// ------------------=
fn assert_packaged_bytes(
    container: &Container<'_>,
    source: &Path,
    packaged_path: &str,
    scratch: &Path,
    sequence: usize,
) {
    let destination = scratch.join(format!("asset-{sequence}"));
    extract(container, packaged_path, &destination);
    let expected = fs::read(source).expect("source asset must be readable");
    let actual = fs::read(&destination).expect("extracted asset must be readable");
    assert_eq!(actual, expected, "packaged bytes must match source: {}", source.display());
}

// ------------------------=
// FUNC: verify_container
// DESC: Verifies fonts, licenses, skins, and wallpapers in one live or installed system container.
// ------------------=
fn verify_container(container: &Container<'_>, scratch: &Path) {
    let mut sequence = 0usize;
    for source in collect_files(Path::new("assets/fonts")) {
        let extension = source.extension().and_then(OsStr::to_str);
        let name = source.file_name().and_then(OsStr::to_str).expect("font name");
        let root = if extension == Some("ttf") {
            container.font_root
        } else if extension == Some("txt") && name.starts_with("OFL-") {
            container.license_root
        } else {
            continue;
        };
        assert_packaged_bytes(container, &source, &format!("{root}/{name}"), scratch, sequence);
        sequence += 1;
    }

    for source in collect_files(Path::new("assets/skins")) {
        let relative = source.strip_prefix("assets/skins").expect("skin relative path");
        assert_packaged_bytes(
            container,
            &source,
            &format!("{}/{}", container.skin_root, relative.display()),
            scratch,
            sequence,
        );
        sequence += 1;
    }

    for source in collect_files(Path::new("assets/desktop")) {
        if source.extension().and_then(OsStr::to_str) != Some("png") {
            continue;
        }
        let name = source.file_name().and_then(OsStr::to_str).expect("wallpaper name");
        assert_packaged_bytes(
            container,
            &source,
            &format!("{}/{name}", container.wallpaper_root),
            scratch,
            sequence,
        );
        sequence += 1;
    }

    for source in [Path::new("assets/boot/infinity-installer-mesh-diagram-v1.png")] {
        let name = source.file_name().and_then(OsStr::to_str).expect("installer asset name");
        assert_packaged_bytes(
            container,
            source,
            &format!("{}/{name}", container.installer_root),
            scratch,
            sequence,
        );
        sequence += 1;
    }
    assert!(sequence > 0, "each system container must expose packaged UI assets");
}

// ------------------------=
// FUNC: main
// DESC: Exercises every live and fresh-install UI container and compares its extracted asset bytes.
// ------------------=
fn main() {
    let scratch = env::temp_dir().join(format!("infinity-ui-parity-{}", std::process::id()));
    if scratch.exists() {
        fs::remove_dir_all(&scratch).expect("stale scratch directory must be removable");
    }
    fs::create_dir_all(&scratch).expect("scratch directory must be creatable");

    let containers = [
        Container { kind: ContainerKind::Fat, image: "build/infinity-x86_64.img", font_root: "/EFI/INFINITY/FONTS", license_root: "/EFI/INFINITY/FONT-LICENSES", skin_root: "/EFI/INFINITY/INFINITYUI", wallpaper_root: "/EFI/INFINITY/INFINITYUI/Wallpapers", installer_root: "/EFI/INFINITY/INFINITYUI/Installer" },
        Container { kind: ContainerKind::Fat, image: "build/infinity-aarch64.img", font_root: "/EFI/INFINITY/FONTS", license_root: "/EFI/INFINITY/FONT-LICENSES", skin_root: "/EFI/INFINITY/INFINITYUI", wallpaper_root: "/EFI/INFINITY/INFINITYUI/Wallpapers", installer_root: "/EFI/INFINITY/INFINITYUI/Installer" },
        Container { kind: ContainerKind::Fat, image: "build/infinity-aarch64-qemu.img", font_root: "/EFI/INFINITY/FONTS", license_root: "/EFI/INFINITY/FONT-LICENSES", skin_root: "/EFI/INFINITY/INFINITYUI", wallpaper_root: "/EFI/INFINITY/INFINITYUI/Wallpapers", installer_root: "/EFI/INFINITY/INFINITYUI/Installer" },
        Container { kind: ContainerKind::Fat, image: "build/x86_64/installed-esp.img", font_root: "/EFI/InfinityOS/Fonts", license_root: "/EFI/InfinityOS/FontLicenses", skin_root: "/EFI/InfinityOS/InfinityUI", wallpaper_root: "/EFI/InfinityOS/InfinityUI/Wallpapers", installer_root: "/EFI/InfinityOS/InfinityUI/Installer" },
        Container { kind: ContainerKind::Fat, image: "build/aarch64/installed-esp.img", font_root: "/EFI/InfinityOS/Fonts", license_root: "/EFI/InfinityOS/FontLicenses", skin_root: "/EFI/InfinityOS/InfinityUI", wallpaper_root: "/EFI/InfinityOS/InfinityUI/Wallpapers", installer_root: "/EFI/InfinityOS/InfinityUI/Installer" },
        Container { kind: ContainerKind::Iso, image: "build/infinity-x86.iso", font_root: "/System/Fonts", license_root: "/System/FontLicenses", skin_root: "/System/InfinityUI", wallpaper_root: "/System/InfinityUI/Wallpapers", installer_root: "/System/InfinityUI/Installer" },
    ];

    for (index, container) in containers.iter().enumerate() {
        assert!(Path::new(container.image).is_file(), "system container must exist");
        let container_scratch = scratch.join(format!("container-{index}"));
        fs::create_dir_all(&container_scratch).expect("container scratch directory must be creatable");
        verify_container(container, &container_scratch);
    }

    fs::remove_dir_all(&scratch).expect("scratch directory must be removable");
}
