import XCTest
import SwiftUI
@testable import InfinityInstallerStudio

final class InstallerEndScreenTests: XCTestCase {
    // ------------------------=
    // FUNC: testProgressSurfaceFillsAuthoredBounds
    // DESC: Renders the real canvas control at multiple sizes and verifies its panel does not shrink to intrinsic content height.
    // ------------------=
    @MainActor func testProgressSurfaceFillsAuthoredBounds() throws {
        let element = try XCTUnwrap(InstallerStudioDocument.factoryDefault().screens[7].elements.first { $0.kind == .progressBar })
        for size in [CGSize(width: 780, height: 190), CGSize(width: 500, height: 100)] {
            let view = CanvasProgressBarPreview(element: element, canvasScale: CGSize(width: 1, height: 1))
                .frame(width: size.width, height: size.height)
            let image = try XCTUnwrap(ImageRenderer(content: view).cgImage)
            let bitmap = NSBitmapImageRep(cgImage: image)
            XCTAssertEqual(image.width, Int(size.width))
            XCTAssertEqual(image.height, Int(size.height))
            for y in [6, Int(size.height) - 7] {
                XCTAssertGreaterThan(try XCTUnwrap(bitmap.colorAt(x: Int(size.width) / 2, y: y)).alphaComponent, 0.9)
            }
        }
    }

    // ------------------------=
    // FUNC: testEndScreenEditsSurviveBuildAndMigration
    // DESC: Checks saved custom layers, hidden hero state, progress geometry, and Complete content through binary packaging.
    // ------------------=
    func testEndScreenEditsSurviveBuildAndMigration() throws {
        var document = InstallerStudioDocument.factoryDefault()
        let index = try XCTUnwrap(document.screens[7].elements.firstIndex { $0.role == .progressBar })
        document.screens[7].elements[index].frame = CanvasRect(x: 160, y: 700, width: 680, height: 100)
        let hero = try XCTUnwrap(document.screens[7].elements.firstIndex { $0.role == .progressHero })
        document.screens[7].elements[hero].hidden = true
        for index in [7, 8] {
            document.screens[index].elements.append(.make(name: "Authored panel", kind: .panel,
                frame: CanvasRect(x: 140, y: 420, width: 640, height: 160), zIndex: 12))
        }
        let restored = try RuntimeTemplateCodec.decode(RuntimeTemplateCodec.encode(document))
        XCTAssertEqual(restored, document)
        XCTAssertEqual(restored.migratedForInstallerRuntimeParity(), document)
    }

    // ------------------------=
    // FUNC: testCaptureSavedEndScreenLayers
    // DESC: Captures the actual canvas components for Installing and Complete without modifying the user's project.
    // ------------------=
    @MainActor func testCaptureSavedEndScreenLayers() throws {
        guard let directory = ProcessInfo.processInfo.environment["INFINITY_END_SCREEN_PROOFS"] else { return }
        let store = TemplateStore()
        for screen in [8, 9] {
            store.selectScreen(screen)
            store.selectElement(nil)
            let layers = try XCTUnwrap(store.selectedScreen?.elements)
            let view = ZStack(alignment: .topLeading) {
                InfinityUIKit.Palette.nativeCanvas
                ForEach(layers.filter { !$0.hidden }.sorted { $0.zIndex < $1.zIndex }) { element in
                    CanvasElementView(element: element, store: store, canvasScale: CGSize(width: 1.6, height: 1))
                }
            }.frame(width: 1600, height: 1000)
            let image = try XCTUnwrap(ImageRenderer(content: view).cgImage)
            let png = try XCTUnwrap(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
            try png.write(to: URL(fileURLWithPath: directory).appending(path: "studio-\(screen).png"))
        }
    }
}
