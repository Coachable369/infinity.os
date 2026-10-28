import AppKit
import XCTest
@testable import InfinityInstallerStudio

final class LoginSceneTests: XCTestCase {
    // ------------------------=
    // FUNC: testMotionMigrationPreservesIntentionalEdits
    // DESC: Restores missing legacy motion once and retains geometry, deletion and native binding across saves.
    // ------------------=
    func testMotionMigrationPreservesIntentionalEdits() throws {
        var old = InstallerStudioDocument.factoryLogin()
        old.explicitLoginMotion = nil
        old.screens[0].elements.removeAll { $0.role == .progressHero }
        var migrated = old.migratedForLoginMotion()
        let i = try XCTUnwrap(migrated.screens[0].elements.firstIndex { $0.role == .progressHero })
        migrated.screens[0].elements[i].frame.x = 550
        migrated.screens[0].elements[i].opacity = 65
        XCTAssertEqual(migrated.migratedForLoginMotion(), migrated)
        let saved = try JSONDecoder().decode(InstallerStudioDocument.self, from: JSONEncoder().encode(migrated))
        XCTAssertEqual(saved, migrated)
        migrated.screens[0].elements.remove(at: i)
        XCTAssertFalse(migrated.migratedForLoginMotion().screens[0].elements.contains { $0.role == .progressHero })
    }
    // ------------------------=
    // FUNC: testEditableLoginRoundTripAndUndo
    // DESC: Exercises authored authentication layers, undo, binary persistence and protected credential controls.
    // ------------------=
    @MainActor func testEditableLoginRoundTripAndUndo() throws {
        let store = TemplateStore()
        store.loginDocument = .factoryLogin()
        store.selectScreenCollection(.login)
        let original = store.loginDocument
        let image = try XCTUnwrap(store.selectedScreen?.elements.first { $0.codeIdentifier == "loginWordmark" })
        store.selectElement(image.id)
        XCTAssertFalse(store.selectedElementIsRequired)
        store.updateSelected { $0.frame.x = 500; $0.opacity = 60 }
        XCTAssertNotEqual(store.loginDocument, original)
        store.undo()
        XCTAssertEqual(store.loginDocument, original)
        let bytes = try RuntimeTemplateCodec.encode(store.loginDocument, assetRoot: store.projectRoot, collection: .login)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(bytes, collection: .login), original)
        var invalid = original
        let password = try XCTUnwrap(invalid.screens[0].elements.firstIndex { $0.role == .input })
        invalid.screens[0].elements[password].inputVariable = .none
        XCTAssertThrowsError(try TemplateValidator.validate(invalid, for: .login))
    }

    // ------------------------=
    // FUNC: testProductionLoginPixelsFollowEdits
    // DESC: Renders the native login compositor and verifies that hiding decorative artwork changes its actual pixels.
    // ------------------=
    @MainActor func testProductionLoginPixelsFollowEdits() async throws {
        let store = TemplateStore()
        store.loginDocument = .factoryLogin()
        store.selectScreenCollection(.login)
        let runner = NativePreviewRunner(timeout: .seconds(60))
        let frame = try await runner.render(NativePreviewSnapshot.capture(store, width: 1536, height: 1024))
        store.nativePreviewSimulation.loginPhase = 900
        let impact = try await runner.render(NativePreviewSnapshot.capture(store, width: 1536, height: 1024))
        XCTAssertNotEqual(frame.pixels, impact.pixels)
        store.nativePreviewSimulation.loginPhase = 0
        let image = try XCTUnwrap(frame.image())
        let png = try XCTUnwrap(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
        try png.write(to: try XCTUnwrap(store.projectRoot).appending(path: "builds/studio-login-preview.png"))
        let index = try XCTUnwrap(store.loginDocument.screens[0].elements.firstIndex { $0.codeIdentifier == "loginWordmark" })
        store.loginDocument.screens[0].elements[index].hidden = true
        let hidden = try await runner.render(NativePreviewSnapshot.capture(store, width: 1536, height: 1024))
        XCTAssertNotEqual(frame.pixels, hidden.pixels)
    }
}
