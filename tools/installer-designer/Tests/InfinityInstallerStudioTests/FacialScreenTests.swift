import AppKit
import XCTest
@testable import InfinityInstallerStudio

final class FacialScreenTests: XCTestCase {
    // ------------------------=
    // FUNC: testMigrationPreservesExistingScreensAndEdits
    // DESC: Verifies both biometric scenes are appended without replacing authored layouts or later edits.
    // ------------------=
    func testMigrationPreservesExistingScreensAndEdits() throws {
        var old = InstallerStudioDocument.factoryConfiguration()
        old.screens.removeLast(2)
        old.screens[0].title = "My welcome screen"
        var migrated = old.migratedForFacialEnrollment()
        XCTAssertEqual(Array(migrated.screens.prefix(8)), old.screens)
        let title = try XCTUnwrap(migrated.screens[8].elements.firstIndex { $0.role == .title })
        migrated.screens[8].elements[title].frame.x = 43
        let voiceTitle = try XCTUnwrap(migrated.screens[9].elements.firstIndex { $0.role == .title })
        migrated.screens[9].elements[voiceTitle].frame.x = 47
        XCTAssertEqual(migrated.migratedForFacialEnrollment(), migrated)
        try TemplateValidator.validate(migrated, for: .configuration)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(RuntimeTemplateCodec.encode(migrated, collection: .configuration),
            collection: .configuration), migrated)
    }

    // ------------------------=
    // FUNC: testNativeFacialPixelsFollowAuthoredEdits
    // DESC: Exercises production illustration visibility and control geometry through Studio, saving a reviewable screenshot.
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
        let mesh = try XCTUnwrap(store.selectedScreen?.elements.first { $0.codeIdentifier == "faceSprite" })
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

    // ------------------------=
    // FUNC: testNativeVoicePixelsFollowAuthoredEdits
    // DESC: Verifies voice enrollment selection, WYSIWYG geometry, undo and runtime serialization preserve editable controls.
    // ------------------=
    @MainActor func testNativeVoicePixelsFollowAuthoredEdits() async throws {
        let store = TemplateStore()
        store.configurationDocument = .factoryConfiguration()
        store.selectScreenCollection(.configuration, screen: 10)
        let runner = NativePreviewRunner(timeout: .seconds(60))
        let original = try await runner.render(NativePreviewSnapshot.capture(store, width: 1600, height: 900))
        let image = try XCTUnwrap(original.image())
        let png = try XCTUnwrap(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
        try png.write(to: try XCTUnwrap(store.projectRoot).appending(path: "builds/studio-voice-preview.png"))
        let sprite = try XCTUnwrap(store.selectedScreen?.elements.first { $0.codeIdentifier == "voiceSprite" })
        store.selectElement(sprite.id)
        store.updateSelected { $0.frame.x -= 30 }
        let moved = try await runner.render(NativePreviewSnapshot.capture(store, width: 1600, height: 900))
        XCTAssertNotEqual(original.pixels, moved.pixels)
        let encoded = try RuntimeTemplateCodec.encode(store.configurationDocument, collection: .configuration)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(encoded, collection: .configuration), store.configurationDocument)
        store.undo()
        let restored = try await runner.render(NativePreviewSnapshot.capture(store, width: 1600, height: 900))
        XCTAssertEqual(restored.pixels, original.pixels)
    }
}
