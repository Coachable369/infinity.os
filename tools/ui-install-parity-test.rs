use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const LIVE_BOOT_IMAGE_BYTES: u64 = 1024 * 1024 * 1024;

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
    icon_root: Option<&'a str>,
    wallpaper_root: &'a str,
    installer_root: &'a str,
    crash_root: &'a str,
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
            .args([
                "-osirrox",
                "on",
                "-indev",
                container.image,
                "-extract",
                packaged_path,
            ])
            .arg(destination)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
    }
    .expect("container extraction tool must run");
    assert!(
        status.success(),
        "packaged asset must be extractable: {packaged_path}"
    );
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
    assert_eq!(
        actual,
        expected,
        "packaged bytes must match source: {}",
        source.display()
    );
}

// ------------------------=
// FUNC: assert_packaged_path_absent
// DESC: Verifies an undeclared asset cannot be extracted from a live or installed system container.
// ------------------=
fn assert_packaged_path_absent(
    container: &Container<'_>,
    packaged_path: &str,
    scratch: &Path,
    sequence: usize,
) {
    let destination = scratch.join(format!("unexpected-asset-{sequence}"));
    let status = match container.kind {
        ContainerKind::Fat => Command::new("mcopy")
            .args(["-i", container.image, &format!("::{packaged_path}")])
            .arg(&destination)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
        ContainerKind::Iso => Command::new("xorriso")
            .args([
                "-osirrox",
                "on",
                "-indev",
                container.image,
                "-extract",
                packaged_path,
            ])
            .arg(&destination)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
    }
    .expect("container extraction tool must run");
    assert!(
        !status.success(),
        "undeclared asset must not be extractable: {packaged_path}"
    );
}

// ------------------------=
// FUNC: assert_packaged_tree
// DESC: Extracts and byte-compares a complete packaged asset hierarchy in one container operation.
// ------------------=
fn assert_packaged_tree(
    container: &Container<'_>,
    source_root: &Path,
    packaged_root: &str,
    scratch: &Path,
) {
    let tree_name = packaged_root
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("asset-tree");
    let destination = scratch.join(format!("asset-tree-{tree_name}"));
    fs::create_dir_all(&destination).expect("tree destination must be creatable");
    let status = match container.kind {
        ContainerKind::Fat => Command::new("mcopy")
            .args(["-i", container.image, "-s", &format!("::{packaged_root}/*")])
            .arg(&destination)
            .status(),
        ContainerKind::Iso => Command::new("xorriso")
            .args([
                "-osirrox",
                "on",
                "-indev",
                container.image,
                "-extract",
                packaged_root,
            ])
            .arg(&destination)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
    }
    .expect("container tree extraction tool must run");
    assert!(
        status.success(),
        "packaged asset hierarchy must be extractable"
    );

    let expected_files = collect_files(source_root);
    for source in &expected_files {
        let relative = source
            .strip_prefix(source_root)
            .expect("asset relative path");
        let actual = destination.join(relative);
        assert!(
            actual.is_file(),
            "packaged asset must exist: {}",
            relative.display()
        );
        assert_eq!(
            fs::read(&actual).expect("packaged asset must be readable"),
            fs::read(source).expect("source asset must be readable"),
            "packaged asset bytes must match: {}",
            relative.display()
        );
    }
    assert_eq!(
        collect_files(&destination).len(),
        expected_files.len(),
        "packaged asset hierarchy must contain every and only declared asset"
    );
}

// ------------------------=
// FUNC: assert_tree_absent
// DESC: Verifies a desktop-only asset hierarchy cannot be extracted from a live boot container.
// ------------------=
fn assert_tree_absent(container: &Container<'_>, packaged_root: &str, scratch: &Path) {
    let destination = scratch.join("unexpected-icon-tree");
    fs::create_dir_all(&destination).expect("absence destination must be creatable");
    let status = match container.kind {
        ContainerKind::Fat => Command::new("mcopy")
            .args(["-i", container.image, "-s", &format!("::{packaged_root}/*")])
            .arg(&destination)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
        ContainerKind::Iso => Command::new("xorriso")
            .args([
                "-osirrox",
                "on",
                "-indev",
                container.image,
                "-extract",
                packaged_root,
            ])
            .arg(&destination)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
    }
    .expect("container extraction tool must run");
    assert!(
        !status.success(),
        "desktop icon hierarchy must not be present in a live boot container"
    );
}

// ------------------------=
// FUNC: verify_container
// DESC: Verifies fonts, licenses, skins, and wallpapers in one live or installed system container.
// ------------------=
fn verify_container(container: &Container<'_>, scratch: &Path) {
    let mut sequence = 0usize;
    for source in collect_files(Path::new("assets/fonts")) {
        let extension = source.extension().and_then(OsStr::to_str);
        let name = source
            .file_name()
            .and_then(OsStr::to_str)
            .expect("font name");
        let root = if extension == Some("ttf") {
            container.font_root
        } else if extension == Some("txt") && name.starts_with("OFL-") {
            container.license_root
        } else {
            continue;
        };
        assert_packaged_bytes(
            container,
            &source,
            &format!("{root}/{name}"),
            scratch,
            sequence,
        );
        sequence += 1;
    }

    for source in collect_files(Path::new("assets/skins")) {
        let relative = source
            .strip_prefix("assets/skins")
            .expect("skin relative path");
        assert_packaged_bytes(
            container,
            &source,
            &format!("{}/{}", container.skin_root, relative.display()),
            scratch,
            sequence,
        );
        sequence += 1;
    }

    if let Some(icon_root) = container.icon_root {
        assert_packaged_tree(container, Path::new("assets/icons"), icon_root, scratch);
    } else {
        let icon_root = match container.kind {
            ContainerKind::Fat => "/EFI/INFINITY/INFINITYUI/Icons",
            ContainerKind::Iso => "/System/InfinityUI/Icons",
        };
        assert_tree_absent(container, icon_root, scratch);
    }

    for source in [
        Path::new("assets/desktop/infinity-default-dark-wallpaper-v2.png"),
        Path::new("assets/desktop/infinity-shell-wallpaper-v3.png"),
        Path::new("assets/desktop/infinity-onboarding-wallpaper-v1.png"),
    ] {
        let name = source
            .file_name()
            .and_then(OsStr::to_str)
            .expect("wallpaper name");
        assert_packaged_bytes(
            container,
            &source,
            &format!("{}/{name}", container.wallpaper_root),
            scratch,
            sequence,
        );
        sequence += 1;
    }

    for name in [
        "infinity-default-dark-wallpaper-v2-source.png",
        "infinity-shell-wallpaper-v3-source.png",
        "infinity-topbar-icon-v2.png",
        "infinity-desktop-wallpaper-v1.png",
        "infinity-topbar-icon-v1.png",
    ] {
        assert_packaged_path_absent(
            container,
            &format!("{}/{name}", container.wallpaper_root),
            scratch,
            sequence,
        );
        sequence += 1;
    }

    for source in [
        Path::new("assets/boot/infinity-installer-mesh-diagram-v1.png"),
        Path::new("assets/boot/installer-screens.infinityui"),
        Path::new("assets/boot/installer-screens.iuit"),
        Path::new("assets/boot/configuration-screens.infinityui"),
        Path::new("assets/boot/configuration-screens.iuit"),
        Path::new("assets/boot/settings-screens.infinityui"),
        Path::new("assets/boot/settings-screens.iuit"),
    ] {
        let name = source
            .file_name()
            .and_then(OsStr::to_str)
            .expect("installer asset name");
        assert_packaged_bytes(
            container,
            source,
            &format!("{}/{name}", container.installer_root),
            scratch,
            sequence,
        );
        sequence += 1;
    }
    assert_packaged_tree(
        container,
        Path::new("assets/boot/installer-assets"),
        &format!("{}/installer-assets", container.installer_root),
        scratch,
    );

    for source in collect_files(Path::new("assets/crash")) {
        let name = source
            .file_name()
            .and_then(OsStr::to_str)
            .expect("crash asset name");
        assert_packaged_bytes(
            container,
            &source,
            &format!("{}/{name}", container.crash_root),
            scratch,
            sequence,
        );
        sequence += 1;
    }
    assert!(
        sequence > 0,
        "each system container must expose packaged UI assets"
    );
}

// ------------------------=
// FUNC: assert_live_boot_image_capacity
// DESC: Verifies every generated live boot FAT container retains the required one-gigabyte capacity.
// ------------------=
fn assert_live_boot_image_capacity(container: &Container<'_>) {
    if !matches!(container.kind, ContainerKind::Fat)
        || !container.image.starts_with("build/infinity-")
    {
        return;
    }
    assert_eq!(
        fs::metadata(container.image)
            .expect("live boot image metadata must be readable")
            .len(),
        LIVE_BOOT_IMAGE_BYTES,
        "live boot images must provide one gigabyte of FAT capacity"
    );
}

// ------------------------=
// FUNC: main
// DESC: Exercises every live and fresh-install UI container and compares its extracted asset bytes.
// ------------------=
fn main() {
    let requested: Vec<String> = env::args().skip(1).collect();
    let scratch = env::temp_dir().join(format!("infinity-ui-parity-{}", std::process::id()));
    if scratch.exists() {
        fs::remove_dir_all(&scratch).expect("stale scratch directory must be removable");
    }
    fs::create_dir_all(&scratch).expect("scratch directory must be creatable");

    let containers = [
        Container {
            kind: ContainerKind::Fat,
            image: "build/infinity-x86_64.img",
            font_root: "/EFI/INFINITY/FONTS",
            license_root: "/EFI/INFINITY/FONT-LICENSES",
            skin_root: "/EFI/INFINITY/INFINITYUI",
            icon_root: None,
            wallpaper_root: "/EFI/INFINITY/INFINITYUI/Wallpapers",
            installer_root: "/EFI/INFINITY/INFINITYUI/Installer",
            crash_root: "/EFI/INFINITY/INFINITYUI/Crash",
        },
        Container {
            kind: ContainerKind::Fat,
            image: "build/infinity-aarch64.img",
            font_root: "/EFI/INFINITY/FONTS",
            license_root: "/EFI/INFINITY/FONT-LICENSES",
            skin_root: "/EFI/INFINITY/INFINITYUI",
            icon_root: None,
            wallpaper_root: "/EFI/INFINITY/INFINITYUI/Wallpapers",
            installer_root: "/EFI/INFINITY/INFINITYUI/Installer",
            crash_root: "/EFI/INFINITY/INFINITYUI/Crash",
        },
        Container {
            kind: ContainerKind::Fat,
            image: "build/infinity-aarch64-qemu.img",
            font_root: "/EFI/INFINITY/FONTS",
            license_root: "/EFI/INFINITY/FONT-LICENSES",
            skin_root: "/EFI/INFINITY/INFINITYUI",
            icon_root: None,
            wallpaper_root: "/EFI/INFINITY/INFINITYUI/Wallpapers",
            installer_root: "/EFI/INFINITY/INFINITYUI/Installer",
            crash_root: "/EFI/INFINITY/INFINITYUI/Crash",
        },
        Container {
            kind: ContainerKind::Fat,
            image: "build/x86_64/installed-esp.img",
            font_root: "/EFI/InfinityOS/Fonts",
            license_root: "/EFI/InfinityOS/FontLicenses",
            skin_root: "/EFI/InfinityOS/InfinityUI",
            icon_root: Some("/EFI/InfinityOS/InfinityUI/Icons"),
            wallpaper_root: "/EFI/InfinityOS/InfinityUI/Wallpapers",
            installer_root: "/EFI/InfinityOS/InfinityUI/Installer",
            crash_root: "/EFI/InfinityOS/InfinityUI/Crash",
        },
        Container {
            kind: ContainerKind::Fat,
            image: "build/aarch64/installed-esp.img",
            font_root: "/EFI/InfinityOS/Fonts",
            license_root: "/EFI/InfinityOS/FontLicenses",
            skin_root: "/EFI/InfinityOS/InfinityUI",
            icon_root: Some("/EFI/InfinityOS/InfinityUI/Icons"),
            wallpaper_root: "/EFI/InfinityOS/InfinityUI/Wallpapers",
            installer_root: "/EFI/InfinityOS/InfinityUI/Installer",
            crash_root: "/EFI/InfinityOS/InfinityUI/Crash",
        },
        Container {
            kind: ContainerKind::Iso,
            image: "builds/InfinityOS-x86.iso",
            font_root: "/System/Fonts",
            license_root: "/System/FontLicenses",
            skin_root: "/System/InfinityUI",
            icon_root: None,
            wallpaper_root: "/System/InfinityUI/Wallpapers",
            installer_root: "/System/InfinityUI/Installer",
            crash_root: "/System/InfinityUI/Crash",
        },
    ];

    let mut verified = 0usize;
    for (index, container) in containers.iter().enumerate() {
        if !requested.is_empty() && !requested.iter().any(|image| image == container.image) {
            continue;
        }
        assert!(
            Path::new(container.image).is_file(),
            "system container must exist"
        );
        assert_live_boot_image_capacity(container);
        let container_scratch = scratch.join(format!("container-{index}"));
        fs::create_dir_all(&container_scratch)
            .expect("container scratch directory must be creatable");
        verify_container(container, &container_scratch);
        verified += 1;
    }

    assert!(
        verified > 0,
        "at least one requested system container must be verified"
    );

    fs::remove_dir_all(&scratch).expect("scratch directory must be removable");
}
