import XCTest
import SwiftUI
@testable import InfinityInstallerStudio

final class GlassButtonTests: XCTestCase {
    // ------------------------=
    // FUNC: testStateStripAndStableCoverage
    // DESC: Renders every selectable style in all three states and exports the actual new dropdown rows for visual review.
    // ------------------=
    @MainActor func testStateStripAndStableCoverage() throws {
        let root = URL(fileURLWithPath: "/opt/codebase/infinity.os")
        let atlas = try XCTUnwrap(PrecisionAtlas(data: Data(contentsOf: root.appending(path: "assets/ui-design-kit/default/eclipse-precision-v1.rgba"))))
        for preset in GlassButtonPreset.allCases {
            let normal = try XCTUnwrap(atlas.render(.primary, width: 250, height: 64, material: PrecisionMaterial(style: preset.style)))
            let hover = try XCTUnwrap(atlas.render(.primary, width: 250, height: 64, material: PrecisionMaterial(focus: 255, style: preset.style)))
            let pressed = try XCTUnwrap(atlas.render(.primary, width: 250, height: 64, material: PrecisionMaterial(pressed: true, style: preset.style)))
            XCTAssertNotEqual(normal, hover); XCTAssertNotEqual(normal, pressed); XCTAssertNotEqual(hover, pressed)
            if preset.style.material.rawValue >= 25 {
                for offset in stride(from: 3, to: normal.count, by: 4) {
                    XCTAssertEqual(normal[offset] == 0, hover[offset] == 0)
                    XCTAssertEqual(normal[offset] == 0, pressed[offset] == 0)
                }
            }
        }
        let store = UIKitStore(projectRoot: root)
        let view = VStack(spacing: 12) {
            ForEach([GlassButtonPreset.luminous, .capsule, .faceted, .prism]) { preset in
                ButtonStyleStateRow(store: store, name: preset.rawValue, style: preset.style, selected: false)
            }
        }.padding(20).frame(width: 500).background(InfinityUIKit.Palette.nativePanel)
        let renderer = ImageRenderer(content: view)
        renderer.scale = 2
        let image = try XCTUnwrap(renderer.cgImage)
        let png = try XCTUnwrap(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
        try png.write(to: root.appending(path: "builds/futuristic-glass-dropdown.png"))
    }
    // ------------------------=
    // FUNC: testPersistentCloudTexture
    // DESC: Checks native and Studio texture bytes agree and glass has visible horizontal detail without a wallpaper behind it.
    // ------------------=
    func testPersistentCloudTexture() throws {
        let root = URL(fileURLWithPath: "/opt/codebase/infinity.os")
        let texture = try Data(contentsOf: root.appending(path: "assets/ui-design-kit/default/glass-button-clouds-v1.rgb"))
        XCTAssertEqual(texture.count, 256 * 72 * 3)
        XCTAssertEqual(texture, try Data(contentsOf: root.appending(path: "tools/installer-designer/Sources/InfinityInstallerStudio/Resources/glass-button-clouds-v1.rgb")))
        let atlas = try XCTUnwrap(PrecisionAtlas(data: Data(contentsOf: root.appending(path: "assets/ui-design-kit/default/eclipse-precision-v1.rgba"))))
        for pressed in [false, true] {
            let material = PrecisionMaterial(pressed: pressed, style: GlassButtonPreset.smoked.style)
            let row = (30..<220).map { atlas.pixel(.primary, width: 250, height: 72, x: $0, y: 36, material: material).blue }
            XCTAssertGreaterThan(try XCTUnwrap(row.max()) - XCTUnwrap(row.min()), 20)
            XCTAssertEqual(atlas.pixel(.primary, width: 250, height: 72, x: 0, y: 0, material: material).alpha, 0)
        }
        var hidden = GlassButtonPreset.smoked.style
        hidden.fill = 0x00101B3A
        hidden.border = 0x0086C7FF
        XCTAssertEqual(atlas.pixel(.primary, width: 250, height: 72, x: 125, y: 36,
            material: PrecisionMaterial(style: hidden)).alpha, 0)
    }
    // ------------------------=
    // FUNC: testElementPresetsPersistWithoutChangingBehavior
    // DESC: Verifies individual style selection survives saving while preserving button identity, label, placement and role.
    // ------------------=
    func testElementPresetsPersistWithoutChangingBehavior() throws {
        let original = StudioElement.make(name: "Record", kind: .button, role: .primaryButton,
            frame: CanvasRect(x: 100, y: 200, width: 250, height: 52), text: "Record sample")
        for preset in GlassButtonPreset.allCases {
            var button = original
            preset.apply(to: &button)
            let decoded = try JSONDecoder().decode(StudioElement.self, from: JSONEncoder().encode(button))
            XCTAssertEqual(GlassButtonPreset.matching(decoded), preset)
            XCTAssertEqual(decoded.id, original.id)
            XCTAssertEqual(decoded.role, original.role)
            XCTAssertEqual(decoded.frame, original.frame)
            XCTAssertEqual(decoded.text, original.text)
            XCTAssertEqual(decoded.opacity, original.opacity)
            button.surfaceMaterial = .authored
            button.surfaceCorners = .authored
            XCTAssertNil(GlassButtonPreset.matching(button))
        }
    }
    // ------------------------=
    // FUNC: testPreviewBoard
    // DESC: Captures the actual Studio surface selector for visual review without changing the saved UIKit document.
    // ------------------=
    @MainActor func testPreviewBoard() throws {
        let root = URL(fileURLWithPath: "/opt/codebase/infinity.os")
        let store = UIKitStore(projectRoot: root)
        let renderer = ImageRenderer(content: UIKitStylesPreview(store: store)
            .padding(24).frame(width: 1000).background(Color(red: 0.035, green: 0.059, blue: 0.149)))
        let image = try XCTUnwrap(renderer.cgImage)
        let data = try XCTUnwrap(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
        try data.write(to: root.appending(path: "builds/studio-glass-preview.png"))
    }

    // ------------------------=
    // FUNC: testButtonThemeInheritanceAndOverride
    // DESC: Ensures a shared recipe survives inheritance while an explicit per-layer material retains local colors.
    // ------------------=
    func testButtonThemeInheritanceAndOverride() {
        let style = GlassButtonPreset.smoked.style
        XCTAssertEqual(style.template(material: .authored, corners: .authored, radius: 0, inheritColors: true), style)
        let local = style.template(material: .matte, corners: .square, radius: 0)
        XCTAssertEqual(local.material, .matte)
        XCTAssertEqual(local.corners, .square)
        XCTAssertNil(local.fill)
        XCTAssertNil(local.border)
    }
    // ------------------------=
    // FUNC: testPresetsApplyAtomicallyAndUndo
    // DESC: Verifies both shared button families change together and unrelated surfaces survive undo and serialization.
    // ------------------=
    @MainActor func testPresetsApplyAtomicallyAndUndo() throws {
        let store = UIKitStore(projectRoot: nil)
        let original = store.document
        for preset in GlassButtonPreset.allCases {
            store.applyButtonPreset(preset)
            XCTAssertEqual(store.document.styles[3], preset.style)
            XCTAssertEqual(store.document.styles[4], preset.style)
            XCTAssertEqual(Array(store.document.styles.prefix(3)), Array(original.styles.prefix(3)))
            XCTAssertEqual(try JSONDecoder().decode(UIKitDocument.self, from: JSONEncoder().encode(store.document)), store.document)
            store.undo()
            XCTAssertEqual(store.document, original)
        }
    }

    // ------------------------=
    // FUNC: testGlassHasTransmissionAndDistinctPresetPixels
    // DESC: Exercises actual surface pixels, preserving antialiasing and independently identifiable translucent glass presets.
    // ------------------=
    func testGlassHasTransmissionAndDistinctPresetPixels() throws {
        let data = try Data(contentsOf: URL(fileURLWithPath: "/opt/codebase/infinity.os/assets/ui-design-kit/default/eclipse-precision-v1.rgba"))
        let atlas = try XCTUnwrap(PrecisionAtlas(data: data))
        var images = Set<Data>()
        for preset in GlassButtonPreset.allCases {
            let material = PrecisionMaterial(style: preset.style)
            let pixels = try XCTUnwrap(atlas.render(.primary, width: 250, height: 52, material: material))
            images.insert(pixels)
            let center = atlas.pixel(.primary, width: 250, height: 52, x: 125, y: 26, material: material)
            if [.outline, .textAction].contains(preset) { XCTAssertEqual(center.alpha, 0) }
            else if [.optical, .liquid, .frosted, .tinted, .luminous, .capsule, .faceted, .prism].contains(preset) || preset.style.material == .glass {
                XCTAssertGreaterThan(center.alpha, 0); XCTAssertLessThan(center.alpha, 255)
            } else { XCTAssertEqual(center.alpha, 255) }
            XCTAssertEqual(atlas.pixel(.primary, width: 250, height: 52, x: 0, y: 0, material: material).alpha, 0)
        }
        XCTAssertGreaterThanOrEqual(images.count, 6)
        XCTAssertEqual(images.count, GlassButtonPreset.allCases.count)
    }
    // ------------------------=
    // FUNC: testModernTemplateAndFrameRoundTrip
    // DESC: Exercises every collection selection through binary save and reload without losing actions or legacy compatibility.
    // ------------------=
    @MainActor func testModernTemplateAndFrameRoundTrip() throws {
        var document = InstallerStudioDocument.factoryDefault()
        let index = try XCTUnwrap(document.screens[0].elements.firstIndex { $0.kind == .button })
        for preset in GlassButtonPreset.modern {
            preset.apply(to: &document.screens[0].elements[index])
            let encoded = try RuntimeTemplateCodec.encode(document)
            let decoded = try RuntimeTemplateCodec.decode(encoded)
            XCTAssertEqual(decoded, document)
            try encoded.write(to: URL(fileURLWithPath: "/opt/codebase/infinity.os/builds/modern-button-\(preset.style.material.rawValue).iuit"))
        }
        var desktop = InstallerStudioDocument.factoryDesktop()
        desktop.infinityMenu = StudioMenu(items: [StudioMenuItem(label: "Tools", action: .submenu, children: [StudioMenuItem(label: "Editor", action: .application, target: 1)])])
        let button = StudioElement.make(name: "Test", kind: .button, frame: CanvasRect(x: 100, y: 200, width: 250, height: 52), text: "Continue")
        desktop.screens[0].elements.append(button)
        GlassButtonPreset.optical.apply(to: &desktop.screens[0].elements[desktop.screens[0].elements.count-1])
        let menuBytes = try RuntimeTemplateCodec.encode(desktop, collection: .desktop)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(menuBytes, collection: .desktop), desktop)
        try menuBytes.write(to: URL(fileURLWithPath: "/opt/codebase/infinity.os/builds/modern-menu.iuit"))
        let store = UIKitStore(projectRoot: nil)
        let atlas = try XCTUnwrap(PrecisionAtlas(data: Data(contentsOf: URL(fileURLWithPath: "/opt/codebase/infinity.os/assets/ui-design-kit/default/eclipse-precision-v1.rgba"))))
        var frames = Set<Data>()
        for preset in WindowFramePreset.allCases {
            store.setSurfaceStyle(0, style: preset.style)
            let saved = try JSONDecoder().decode(UIKitDocument.self, from: JSONEncoder().encode(store.document))
            XCTAssertEqual(saved.styles[0], preset.style)
            XCTAssertEqual(try saved.runtimeStyles()[12], UInt8(preset.rawValue))
            let normal = try XCTUnwrap(atlas.render(.frame, width: 224, height: 112, material: PrecisionMaterial(style: preset.style)))
            let active = try XCTUnwrap(atlas.render(.frame, width: 224, height: 112, material: PrecisionMaterial(focus: 255, style: preset.style)))
            XCTAssertNotEqual(normal, active); frames.insert(active)
            XCTAssertEqual(atlas.pixel(.frame, width: 224, height: 112, x: 100, y: 50, material: PrecisionMaterial(style: preset.style)).alpha, 0)
        }
        XCTAssertEqual(frames.count, 10)
    }

}
