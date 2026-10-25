import AppKit
import XCTest
@testable import InfinityInstallerStudio

final class ImageFitTests: XCTestCase {
    // ------------------------=
    // FUNC: testNativePixelsUseExplicitFitIndependentlyOfRole
    // DESC: Compares production-rendered image pixels across masthead and ordinary-image roles and verifies fill is distinct.
    // ------------------=
    @MainActor func testNativePixelsUseExplicitFitIndependentlyOfRole() async throws {
        let store = TemplateStore()
        store.projectRoot = URL(fileURLWithPath: "/opt/codebase/infinity.os")
        store.document = .factoryDefault()
        store.selectScreen(1)
        store.nativePreviewSimulation.enabled = false
        let index = try XCTUnwrap(store.document.screens[0].elements.firstIndex { $0.role == .masthead })
        store.document.screens[0].elements[index].imageFit = .fit
        store.document.screens[0].elements[index].frame = CanvasRect(x: 200, y: 30, width: 600, height: 250)
        let runner = NativePreviewRunner()
        let header = try await runner.render(NativePreviewSnapshot.capture(store, width: 800, height: 500))
        store.document.screens[0].elements[index].role = .image
        let image = try await runner.render(NativePreviewSnapshot.capture(store, width: 800, height: 500))
        XCTAssertEqual(header.pixels, image.pixels)
        store.document.screens[0].elements[index].imageFit = .fill
        let fill = try await runner.render(NativePreviewSnapshot.capture(store, width: 800, height: 500))
        XCTAssertNotEqual(fill.pixels, image.pixels)
        let cgImage = try XCTUnwrap(image.image())
        let png = try XCTUnwrap(NSBitmapImageRep(cgImage: cgImage).representation(using: .png, properties: [:]))
        try png.write(to: URL(fileURLWithPath: "/opt/codebase/infinity.os/builds/studio-image-fit-proof.png"))
    }

    // ------------------------=
    // FUNC: testImportSizingIsScreenAndFilenameIndependent
    // DESC: Imports identical image bytes onto different screens and verifies physical aspect, explicit fit and round-trip persistence.
    // ------------------=
    @MainActor func testImportSizingIsScreenAndFilenameIndependent() throws {
        let root = FileManager.default.temporaryDirectory.appending(path: UUID().uuidString)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let bitmap = try XCTUnwrap(NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 80, pixelsHigh: 40,
            bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
            colorSpaceName: .deviceRGB, bytesPerRow: 320, bitsPerPixel: 32))
        let bytes = try XCTUnwrap(bitmap.representation(using: .png, properties: [:]))
        let source = root.appending(path: "logo.png")
        let renamed = root.appending(path: "renamed.png")
        try bytes.write(to: source); try bytes.write(to: renamed)
        let store = TemplateStore()
        store.projectRoot = root; store.document = .factoryDefault()
        store.selectScreen(1)
        try store.importImageAsset(from: source)
        let first = try XCTUnwrap(store.selectedElement)
        store.selectScreen(2)
        try store.importImageAsset(from: renamed)
        let second = try XCTUnwrap(store.selectedElement)
        XCTAssertEqual(first.frame.width, second.frame.width)
        XCTAssertEqual(first.frame.height, second.frame.height)
        let physicalAspect = Double(second.frame.width * store.canvasViewport.width) / Double(second.frame.height * store.canvasViewport.height)
        XCTAssertEqual(physicalAspect, 2, accuracy: 0.01)
        XCTAssertEqual(second.imageFit, .fit)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(RuntimeTemplateCodec.encode(store.document)), store.document)
        var masthead = first
        masthead.role = .masthead
        XCTAssertFalse(masthead.imageUsesAspectFill)
        masthead.imageFit = .fill
        XCTAssertTrue(masthead.imageUsesAspectFill)
        masthead.imageFit = .legacy
        XCTAssertTrue(masthead.imageUsesAspectFill)
    }
}
