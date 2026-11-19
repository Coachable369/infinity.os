import XCTest
import SwiftUI
@testable import InfinityInstallerStudio

final class GlassButtonTests: XCTestCase {
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
            XCTAssertGreaterThan(center.alpha, 0)
            if preset.style.material == .glass { XCTAssertLessThan(center.alpha, 255) }
            else { XCTAssertEqual(center.alpha, 255) }
            XCTAssertEqual(atlas.pixel(.primary, width: 250, height: 52, x: 0, y: 0, material: material).alpha, 0)
        }
        XCTAssertEqual(images.count, 5)
    }
}
