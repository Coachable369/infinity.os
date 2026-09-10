import XCTest
import SwiftUI
@testable import InfinityInstallerStudio

final class LiveDetailsTests: XCTestCase {
    // ------------------------=
    // FUNC: testLiveDetailsMoveResizeHideAndPersist
    // DESC: Exercises the real editor interaction and binary save path for runtime-owned text geometry and appearance.
    // ------------------=
    @MainActor func testLiveDetailsMoveResizeHideAndPersist() throws {
        let store = TemplateStore()
        store.document = .factoryDefault()
        store.selectScreen(6)
        store.selectLiveDetails()
        let original = try XCTUnwrap(store.selectedElement)
        XCTAssertFalse(original.locked)
        store.beginGesture(elementID: original.id)
        store.moveSelected(translation: CGSize(width: 30, height: -40), canvasScale: CGSize(width: 1, height: 1))
        store.endGesture()
        store.beginGesture()
        store.resizeSelected(handle: .bottomRight, translation: CGSize(width: 60, height: 20), canvasScale: CGSize(width: 1, height: 1))
        store.endGesture()
        XCTAssertEqual(store.selectedElement!.frame.x, original.frame.x + 30)
        XCTAssertEqual(store.selectedElement!.frame.width, original.frame.width + 60)
        store.updateSelected { $0.fontSize = 22; $0.opacity = 80; $0.hidden = true }
        XCTAssertTrue(store.selectedElement!.hidden)
        store.toggleElementLock(original.id)
        let locked = store.selectedElement!
        store.updateSelected { $0.frame.x = 800 }
        XCTAssertEqual(store.selectedElement, locked)
        let restored = try RuntimeTemplateCodec.decode(RuntimeTemplateCodec.encode(store.document))
        XCTAssertEqual(restored, store.document)
        XCTAssertEqual(restored.migratedForInstallerRuntimeParity(), restored)
    }

    // ------------------------=
    // FUNC: testMigrationAddsOnlyOneRuntimeLayerAndPreservesExistingArtwork
    // DESC: Checks idempotent legacy upgrade, singular roles, and preservation of authored layers.
    // ------------------=
    func testMigrationAddsOnlyOneRuntimeLayerAndPreservesExistingArtwork() throws {
        var legacy = InstallerStudioDocument.factoryDefault()
        for index in legacy.screens.indices { legacy.screens[index].elements.removeAll { $0.role == .liveDetails } }
        let migrated = legacy.migratedForInstallerRuntimeParity()
        for index in legacy.screens.indices {
            XCTAssertEqual(migrated.screens[index].elements.filter { $0.role != .liveDetails }, legacy.screens[index].elements)
            XCTAssertEqual(migrated.screens[index].elements.filter { $0.role == .liveDetails }.count,
                           [3, 4, 6, 10].contains(index + 1) ? 1 : 0)
        }
        XCTAssertEqual(migrated.migratedForInstallerRuntimeParity(), migrated)
        var invalid = migrated
        invalid.screens[5].elements.append(try XCTUnwrap(InstallerStudioDocument.liveDetailsElement(screenID: 6)))
        XCTAssertThrowsError(try TemplateValidator.validate(invalid))
        invalid = migrated
        let index = try XCTUnwrap(invalid.screens[5].elements.firstIndex { $0.role == .liveDetails })
        invalid.screens[5].elements[index].kind = .panel
        XCTAssertThrowsError(try TemplateValidator.validate(invalid))
    }

    // ------------------------=
    // FUNC: testDefaultPlanSeparatesRuntimeDetailsFromAuthoredCopy
    // DESC: Checks factory Plan Review geometry independently of the user's editable project and optionally captures the fixture.
    // ------------------=
    @MainActor func testDefaultPlanSeparatesRuntimeDetailsFromAuthoredCopy() throws {
        let store = TemplateStore()
        // Saved projects may intentionally overlap layers; they must not gate building the editor.
        store.document = .factoryDefault()
        store.selectScreen(6)
        store.showGrid = false
        store.selectElement(nil)
        let elements = try XCTUnwrap(store.selectedScreen?.elements)
        let details = try XCTUnwrap(elements.first { $0.role == .liveDetails })
        for copy in elements where copy.role == .body && !copy.hidden {
            let a = copy.frame, b = details.frame
            XCTAssertTrue(a.x + a.width <= b.x || b.x + b.width <= a.x || a.y + a.height <= b.y || b.y + b.height <= a.y)
        }
        if let destination = ProcessInfo.processInfo.environment["INFINITY_PLAN_PREVIEW_PATH"] {
            let view = ZStack(alignment: .topLeading) {
                Color(red: 0.01, green: 0.03, blue: 0.05)
                ForEach(elements.filter { !$0.hidden }.sorted { $0.zIndex < $1.zIndex }) { element in
                    CanvasElementView(element: element, store: store, canvasScale: CGSize(width: 1.6, height: 1))
                }
            }.frame(width: 1600, height: 1000)
            let image = try XCTUnwrap(ImageRenderer(content: view).cgImage)
            let png = try XCTUnwrap(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
            try png.write(to: URL(fileURLWithPath: destination))
        }
    }
}
