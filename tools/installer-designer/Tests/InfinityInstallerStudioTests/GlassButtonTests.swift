import XCTest
import SwiftUI
@testable import InfinityInstallerStudio

final class GlassButtonTests: XCTestCase {
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
        store.applyButtonPreset(.smoked)
        XCTAssertEqual(store.document.styles[3], GlassButtonPreset.smoked.style)
        XCTAssertEqual(store.document.styles[4], GlassButtonPreset.smoked.style)
        XCTAssertEqual(Array(store.document.styles.prefix(3)), Array(original.styles.prefix(3)))
        XCTAssertEqual(try JSONDecoder().decode(UIKitDocument.self, from: JSONEncoder().encode(store.document)), store.document)
        store.undo()
        XCTAssertEqual(store.document, original)
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
            XCTAssertLessThan(center.alpha, 255)
            XCTAssertEqual(atlas.pixel(.primary, width: 250, height: 52, x: 0, y: 0, material: material).alpha, 0)
        }
        XCTAssertEqual(images.count, 3)
    }
}
