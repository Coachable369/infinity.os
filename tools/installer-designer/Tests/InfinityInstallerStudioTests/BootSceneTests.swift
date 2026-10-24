import XCTest
import AppKit
@testable import InfinityInstallerStudio

final class BootSceneTests: XCTestCase {
    // ------------------------=
    // FUNC: testBootEditingRoundTripAndUndo
    // DESC: Exercises independent loading scenes, editable decoration, undo and the runtime export contract.
    // ------------------=
    @MainActor func testBootEditingRoundTripAndUndo() throws {
        let store = TemplateStore()
        store.bootDocument = .factoryBoot()
        store.selectScreenCollection(.boot)
        let original = store.bootDocument
        let emblem = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .masthead })
        store.selectElement(emblem.id)
        XCTAssertFalse(store.selectedElementIsRequired)
        store.updateSelected { $0.frame.x = 100; $0.opacity = 60 }
        XCTAssertEqual(store.bootDocument.screens[1], original.screens[1])
        store.undo()
        XCTAssertEqual(store.bootDocument, original)
        store.selectElement(emblem.id)
        store.deleteSelected()
        XCTAssertFalse(store.bootDocument.screens[0].elements.contains { $0.role == .masthead })
        let bytes = try RuntimeTemplateCodec.encode(store.bootDocument, assetRoot: store.projectRoot, collection: .boot)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(bytes, collection: .boot), store.bootDocument)
    }

    // ------------------------=
    // FUNC: testBootProductionPixelsAndAnimation
    // DESC: Renders both boot paths with the production compositor, tests reveal/progress and proves hidden layers leave no implicit artwork.
    // ------------------=
    @MainActor func testBootProductionPixelsAndAnimation() async throws {
        let store = TemplateStore()
        store.bootDocument = .factoryBoot()
        store.selectScreenCollection(.boot)
        let runner = NativePreviewRunner(timeout: .seconds(60))
        for screen in 1...2 {
            store.selectScreen(screen)
            store.nativePreviewSimulation.progress = 42
            store.nativePreviewSimulation.bootPhase = 2400
            let complete = try await runner.render(NativePreviewSnapshot.capture(store, width: 640, height: 480))
            store.nativePreviewSimulation.bootPhase = 0
            let start = try await runner.render(NativePreviewSnapshot.capture(store, width: 640, height: 480))
            XCTAssertNotEqual(start.pixels, complete.pixels)
            store.nativePreviewSimulation.bootPhase = 2400
            store.nativePreviewSimulation.progress = 90
            let progressed = try await runner.render(NativePreviewSnapshot.capture(store, width: 640, height: 480))
            XCTAssertNotEqual(complete.pixels, progressed.pixels)
            let image = try XCTUnwrap(progressed.image())
            let png = try XCTUnwrap(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
            try png.write(to: try XCTUnwrap(store.projectRoot).appending(path: "builds/boot-scene-\(screen).png"))
            for index in store.bootDocument.screens[screen-1].elements.indices {
                store.bootDocument.screens[screen-1].elements[index].opacity = 0
            }
            let blank = try await runner.render(NativePreviewSnapshot.capture(store, width: 640, height: 480))
            let pixels = [UInt8](blank.pixels)
            for offset in stride(from: 0, to: pixels.count, by: 4) {
                XCTAssertEqual(Array(pixels[offset..<(offset+3)]), [0,0,0])
            }
        }
    }
}
