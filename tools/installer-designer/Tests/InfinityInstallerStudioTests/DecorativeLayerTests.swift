import XCTest
import AppKit
@testable import InfinityInstallerStudio

final class DecorativeLayerTests: XCTestCase {
    // ------------------------=
    // FUNC: testNativeSceneHasNoUneditableArtwork
    // DESC: Checks production pixels after hiding authored layers and restoring only the editable background.
    // ------------------=
    @MainActor func testNativeSceneHasNoUneditableArtwork() async throws {
        let store = TemplateStore()
        store.document = .factoryDefault()
        store.selectScreen(1)
        for index in store.document.screens[0].elements.indices {
            store.document.screens[0].elements[index].opacity = 0
        }
        let runner = NativePreviewRunner()
        let blank = try await runner.render(NativePreviewSnapshot.capture(store, width: 640, height: 480))
        let pixels = [UInt8](blank.pixels)
        for offset in stride(from: 0, to: pixels.count, by: 4) {
            XCTAssertEqual(Array(pixels[offset..<(offset + 3)]), [0, 0, 0])
        }
        let background = try XCTUnwrap(store.document.screens[0].elements.firstIndex { $0.role == .sceneBackground })
        store.document.screens[0].elements[background].opacity = 100
        let visible = try await runner.render(NativePreviewSnapshot.capture(store, width: 640, height: 480))
        XCTAssertNotEqual(blank.pixels, visible.pixels)
        if let path = ProcessInfo.processInfo.environment["INFINITY_DECORATIVE_PREVIEW"] {
            store.document = .factoryDefault()
            let frame = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
            let image = try XCTUnwrap(frame.image())
            let png = try XCTUnwrap(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
            try png.write(to: URL(fileURLWithPath: path))
        }
        store.selectScreenCollection(.systemSettings)
        store.settingsDocument = .factorySystemSettings()
        store.selectScreen(1)
        store.settingsDocument.screens[0].elements.removeAll { $0.role == .settingsNavigationIcon }
        let settingsFrame = try await runner.render(NativePreviewSnapshot.capture(store, width: 640, height: 480))
        XCTAssertEqual(settingsFrame.pixels.count, 640 * 480 * 4)
    }

    // ------------------------=
    // FUNC: testDecorativeImagesCanBeDeletedAndStayDeleted
    // DESC: Exercises editor deletion and saved runtime migration while protecting functional navigation.
    // ------------------=
    @MainActor func testDecorativeImagesCanBeDeletedAndStayDeleted() throws {
        let store = TemplateStore()
        store.document = .factoryDefault()
        for (screen, role) in [(1, StudioElementRole.masthead), (1, .sceneBackground), (5, .timeZoneMap), (8, .progressHero)] {
            store.selectScreen(screen)
            let element = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == role })
            if element.locked { store.toggleElementLock(element.id) }
            store.selectElement(element.id)
            XCTAssertFalse(store.selectedElementIsRequired)
            store.deleteSelected()
            XCTAssertFalse(store.selectedScreen!.elements.contains { $0.id == element.id })
        }
        let decoded = try RuntimeTemplateCodec.decode(RuntimeTemplateCodec.encode(store.document))
        XCTAssertEqual(decoded, store.document)
        XCTAssertEqual(decoded.migratedForInstallerRuntimeParity(), decoded)
        var noBackgrounds = decoded
        noBackgrounds.screens.removeLast()
        for index in noBackgrounds.screens.indices { noBackgrounds.screens[index].elements.removeAll { $0.role == .sceneBackground } }
        let reopened = try RuntimeTemplateCodec.decode(RuntimeTemplateCodec.encode(noBackgrounds))
        XCTAssertEqual(reopened, noBackgrounds)
        XCTAssertEqual(reopened.migratedForInstallerRuntimeParity(), reopened)
        let json = try JSONEncoder().encode(store.document)
        XCTAssertEqual(try JSONDecoder().decode(InstallerStudioDocument.self, from: json).migratedForInstallerRuntimeParity(), store.document)
        store.selectScreen(1)
        let button = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .primaryButton })
        store.selectElement(button.id)
        XCTAssertTrue(store.selectedElementIsRequired)
        store.deleteSelected()
        XCTAssertTrue(store.selectedScreen!.elements.contains { $0.id == button.id })
        store.selectScreenCollection(.systemSettings)
        store.settingsDocument = .factorySystemSettings()
        store.selectScreen(1)
        let icon = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .settingsNavigationIcon })
        if icon.locked { store.toggleElementLock(icon.id) }
        store.selectElement(icon.id)
        XCTAssertFalse(store.selectedElementIsRequired)
        store.deleteSelected()
        XCTAssertFalse(store.selectedScreen!.elements.contains { $0.id == icon.id })
        let settings = try RuntimeTemplateCodec.encode(store.settingsDocument, collection: .systemSettings)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(settings, collection: .systemSettings), store.settingsDocument)
    }
}
