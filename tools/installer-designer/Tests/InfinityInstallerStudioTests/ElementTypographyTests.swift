import AppKit
import XCTest
@testable import InfinityInstallerStudio

final class ElementTypographyTests: XCTestCase {
    // ------------------------=
    // FUNC: testTypographyPersistsAndRejectsInvalidValues
    // DESC: Round-trips authored family and effects through both project JSON and native payload and rejects out-of-range data.
    // ------------------=
    func testTypographyPersistsAndRejectsInvalidValues() throws {
        var document = InstallerStudioDocument.factoryDefault()
        let index = try XCTUnwrap(document.screens[0].elements.firstIndex { $0.kind == .button })
        document.screens[0].elements[index].fontFamily = 8
        document.screens[0].elements[index].textSharpness = 65
        document.screens[0].elements[index].textGlow = 40
        let json = try JSONDecoder().decode(InstallerStudioDocument.self, from: JSONEncoder().encode(document))
        XCTAssertEqual(json, document)
        let native = try RuntimeTemplateCodec.decode(RuntimeTemplateCodec.encode(document))
        XCTAssertEqual(native, document)
        document.screens[0].elements[index].fontFamily = 47
        XCTAssertThrowsError(try RuntimeTemplateCodec.encode(document))
    }

    // ------------------------=
    // FUNC: testNativeFontEffectsChangePixelsWithoutLeakingToOtherLayers
    // DESC: Compares the actual compositor output for an authored button and verifies restoring defaults restores the original framebuffer.
    // ------------------=
    @MainActor func testNativeFontEffectsChangePixelsWithoutLeakingToOtherLayers() async throws {
        let store = TemplateStore()
        store.projectRoot = URL(fileURLWithPath: "/opt/codebase/infinity.os")
        store.document = .factoryDefault()
        store.selectScreen(1)
        let index = try XCTUnwrap(store.document.screens[0].elements.firstIndex { $0.kind == .button })
        let original = store.document.screens[0].elements[index]
        let runner = NativePreviewRunner()
        let before = try await runner.render(NativePreviewSnapshot.capture(store, width: 800, height: 500))
        store.document.screens[0].elements[index].fontFamily = 8
        let font = try await runner.render(NativePreviewSnapshot.capture(store, width: 800, height: 500))
        XCTAssertNotEqual(before.pixels, font.pixels)
        store.document.screens[0].elements[index].textSharpness = 100
        store.document.screens[0].elements[index].textGlow = 100
        let effects = try await runner.render(NativePreviewSnapshot.capture(store, width: 800, height: 500))
        XCTAssertNotEqual(font.pixels, effects.pixels)
        store.document.screens[0].elements[index] = original
        let restored = try await runner.render(NativePreviewSnapshot.capture(store, width: 800, height: 500))
        XCTAssertEqual(before.pixels, restored.pixels)
    }

    // ------------------------=
    // FUNC: testLockedReplacementDoesNotCreateAnotherImage
    // DESC: Rejects replacement of a locked background before importing or mutating any layer.
    // ------------------=
    @MainActor func testLockedReplacementDoesNotCreateAnotherImage() throws {
        let store = TemplateStore()
        store.document = .factoryDefault()
        store.selectScreen(1)
        store.selectScreenBackground()
        let element = try XCTUnwrap(store.selectedElement)
        if !element.locked { store.toggleElementLock(element.id) }
        let before = store.document
        XCTAssertThrowsError(try store.importImageAsset(from: URL(fileURLWithPath: "/missing.png"), replacingSelected: true))
        XCTAssertEqual(before, store.document)
    }
}
