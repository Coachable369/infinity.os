import AppKit
import XCTest
@testable import InfinityInstallerStudio

final class FacialScreenTests: XCTestCase {
    // ------------------------=
    // FUNC: testMigrationPreservesExistingScreensAndEdits
    // DESC: Verifies migration appends only the facial scene and leaves authored layouts and later edits intact.
    // ------------------=
    func testMigrationPreservesExistingScreensAndEdits() throws {
        var old = InstallerStudioDocument.factoryConfiguration()
        old.screens.removeLast()
        old.screens[0].title = "My welcome screen"
        var migrated = old.migratedForFacialEnrollment()
        XCTAssertEqual(Array(migrated.screens.prefix(8)), old.screens)
        migrated.screens[8].elements[0].frame.x = 43
        XCTAssertEqual(migrated.migratedForFacialEnrollment(), migrated)
        try TemplateValidator.validate(migrated, for: .configuration)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(RuntimeTemplateCodec.encode(migrated, collection: .configuration),
            collection: .configuration), migrated)
    }

    // ------------------------=
    // FUNC: testNativeFacialPixelsFollowAuthoredEdits
    // DESC: Exercises production mesh visibility and control geometry through Studio, saving a reviewable screenshot.
    // ------------------=
    @MainActor func testNativeFacialPixelsFollowAuthoredEdits() async throws {
        let store = TemplateStore()
        store.configurationDocument = .factoryConfiguration()
        store.selectScreenCollection(.configuration, screen: 9)
        let runner = NativePreviewRunner(timeout: .seconds(60))
        let original = try await runner.render(NativePreviewSnapshot.capture(store, width: 1600, height: 900))
        let image = try XCTUnwrap(original.image())
        let png = try XCTUnwrap(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
        try png.write(to: try XCTUnwrap(store.projectRoot).appending(path: "builds/studio-facial-preview.png"))
        let mesh = try XCTUnwrap(store.selectedScreen?.elements.first { $0.codeIdentifier == "faceMesh" })
        store.selectElement(mesh.id)
        store.updateSelected { $0.hidden = true }
        let hidden = try await runner.render(NativePreviewSnapshot.capture(store, width: 1600, height: 900))
        XCTAssertNotEqual(original.pixels, hidden.pixels)
        store.undo()
        let button = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .primaryButton })
        store.selectElement(button.id)
        store.toggleElementLock(button.id)
        store.updateSelected { $0.frame.x = 600; $0.frame.width = 270 }
        let moved = try await runner.render(NativePreviewSnapshot.capture(store, width: 1600, height: 900))
        XCTAssertNotEqual(original.pixels, moved.pixels)
        store.undo()
        let restored = try await runner.render(NativePreviewSnapshot.capture(store, width: 1600, height: 900))
        XCTAssertEqual(restored.pixels, original.pixels)
    }
}
