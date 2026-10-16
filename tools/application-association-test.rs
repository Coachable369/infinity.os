#![allow(dead_code)]

#[path = "../kernel/runtime/application_association.rs"]
mod application_association;

use application_association::{ApplicationHandler, AssociationError, ContentKind, LaunchAdapter, Registry};

// ------------------------=
// FUNC: main
// DESC: Behaviorally verifies compatible discovery, custom registration, persisted defaults, startup delivery, and rejection paths.
// ------------------=
fn main() {
    let mut registry = Registry::new();
    assert_eq!(registry.compatible_count(ContentKind::Utf8Text, b"txt"), 1);
    assert_eq!(registry.compatible_count(ContentKind::Namespace, b""), 1);
    assert!(registry.resolve(ContentKind::Binary, b"bin").is_err());

    let mut custom = ApplicationHandler::new(
        b"app.example.markdown-studio", b"Markdown Studio", LaunchAdapter::InstalledApplication(0xb001),
    ).expect("custom handler");
    custom.add_extension(b"md").expect("md capability");
    custom.add_extension(b"markdown").expect("markdown capability");
    registry.install(custom).expect("custom install");
    assert_eq!(registry.compatible_count(ContentKind::Utf8Text, b"md"), 2);
    assert_eq!(registry.compatible_nth(ContentKind::Utf8Text, b"md", 1), Some(custom));
    let manifest = b"identity=app.example.image-viewer\ndisplay-name=Image Viewer\nimage-id=0xb002\nopen-content-types=binary\nopen-extensions=png,jpg\n";
    let image = ApplicationHandler::from_manifest(manifest).expect("manifest handler");
    assert_eq!(image.launch, LaunchAdapter::InstalledApplication(0xb002));
    assert!(image.supports(ContentKind::Binary, b"png"));
    assert_eq!(ApplicationHandler::from_manifest(b"identity=broken\nimage-id=7\n"), Err(AssociationError::InvalidManifest));
    let spoof = ApplicationHandler::new(
        b"app.infinity.text-editor", b"Spoof", LaunchAdapter::InstalledApplication(0xb003),
    ).unwrap().with_content_type(ContentKind::Utf8Text);
    assert_eq!(registry.install(spoof), Err(AssociationError::InvalidManifest));

    registry.set_default(ContentKind::Utf8Text, b"md", custom.identity.bytes()).expect("default");
    assert_eq!(registry.resolve(ContentKind::Utf8Text, b"md"), Ok(custom));
    assert_eq!(registry.resolve(ContentKind::Utf8Text, b"txt").unwrap().launch, LaunchAdapter::TextEditor);
    assert_eq!(
        registry.set_default(ContentKind::Binary, b"bin", custom.identity.bytes()),
        Err(AssociationError::IncompatibleApplication),
    );

    let encoded = registry.encode_defaults();
    let mut restored = Registry::new();
    restored.install(custom).expect("restored custom install");
    restored.restore_defaults(&encoded).expect("restored defaults");
    assert_eq!(restored.resolve(ContentKind::Utf8Text, b"md"), Ok(custom));
    let mut corrupt = encoded;
    corrupt[24] ^= 1;
    assert_eq!(restored.restore_defaults(&corrupt), Err(AssociationError::CorruptState));

    let request = restored.stage_open(custom, b"/home/default/readme.md", ContentKind::Utf8Text, b"md")
        .expect("typed startup request");
    assert_eq!(request.path.bytes(), b"/home/default/readme.md");
    assert_eq!(restored.take_open_request(custom.identity.bytes()), Some(request));
    assert_eq!(restored.take_open_request(custom.identity.bytes()), None);
    assert_eq!(
        restored.stage_open(custom, b"/home/default/raw.bin", ContentKind::Binary, b"bin"),
        Err(AssociationError::IncompatibleApplication),
    );
}
