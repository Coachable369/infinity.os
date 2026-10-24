import AppKit
import XCTest
@testable import InfinityInstallerStudio

final class TemplateTransferTests: XCTestCase {
    // ------------------------=
    // FUNC: workspace
    // DESC: Creates an isolated project-local fixture for transfer tests without modifying the user's templates.
    // ------------------=
    private func workspace() throws -> URL {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .appending(path: "builds/.studio-work/tmp/studio-transfer-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: root) }
        return root
    }

    // ------------------------=
    // FUNC: testExportImportPreservesCustomPixelsAndDoesNotSaveDraft
    // DESC: Transfers an actual embedded bitmap to a separate project, reopens it, and compares binary pixels and editable layer identity.
    // ------------------=
    @MainActor func testExportImportPreservesCustomPixelsAndDoesNotSaveDraft() throws {
        let root = try workspace(), target = try workspace()
        let pixels = Data([32,64,128,255, 127,63,15,17, 255,20,30,255, 0,0,0,0])
        let image = try XCTUnwrap(CGImage(width: 2, height: 2, bitsPerComponent: 8, bitsPerPixel: 32,
            bytesPerRow: 8, space: CGColorSpace(name: CGColorSpace.sRGB)!,
            bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue),
            provider: try XCTUnwrap(CGDataProvider(data: pixels as CFData)), decode: nil,
            shouldInterpolate: false, intent: .defaultIntent))
        let bitmap = try NativeBitmapEncoder.encode(image)
        let asset = root.appending(path: "test.bmp")
        try bitmap.write(to: asset)
        let source = TemplateStore()
        source.document = .factoryDefault(); source.projectRoot = root
        let identity = try source.importImageAsset(from: asset)
        let before = source.document
        let selection = source.selectedElementID
        let folder = try source.exportDocument(to: root.appending(path: "builds/exports"))
        XCTAssertEqual(source.document, before)
        XCTAssertEqual(source.selectedElementID, selection)
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appending(path: "assets/boot/installer-screens.infinityui").path))
        let exported = try Data(contentsOf: folder.appending(path: "installer-screens.iuit"))
        XCTAssertEqual(try JSONDecoder().decode(InstallerStudioDocument.self,
            from: Data(contentsOf: folder.appending(path: "installer-screens.infinityui"))), before)
        let package = try RuntimeTemplateCodec.decodePackage(exported)
        XCTAssertTrue(package.assets.values.contains(bitmap))
        let destination = TemplateStore()
        destination.document = .factoryDefault(); destination.projectRoot = target
        try destination.importDocument(from: folder.appending(path: "installer-screens.iuit"))
        let imported = try XCTUnwrap(destination.document.screens[0].elements.first { $0.id == identity })
        XCTAssertEqual(imported.frame, before.screens[0].elements.first { $0.id == identity }?.frame)
        let restored = target.appending(path: "assets/boot/" + imported.imageAsset)
        XCTAssertEqual(try Data(contentsOf: restored), bitmap)
        let recompiled = try RuntimeTemplateCodec.encode(destination.document, assetRoot: target)
        XCTAssertTrue(try RuntimeTemplateCodec.decodePackage(recompiled).assets.values.contains(bitmap))
        destination.undo()
        XCTAssertEqual(destination.document, .factoryDefault())
    }

    // ------------------------=
    // FUNC: testMalformedImportAndOutsideExportPreserveDraft
    // DESC: Rejects malformed transfer data and an out-of-project destination before mutating the editor or external files.
    // ------------------=
    @MainActor func testMalformedImportAndOutsideExportPreserveDraft() throws {
        let root = try workspace(), outside = try workspace()
        let store = TemplateStore()
        store.document = .factoryDefault(); store.projectRoot = root
        let before = store.document
        let broken = root.appending(path: "broken.iuit")
        try Data([0,1,2,3]).write(to: broken)
        XCTAssertThrowsError(try store.importDocument(from: broken))
        XCTAssertEqual(store.document, before)
        XCTAssertThrowsError(try store.exportDocument(to: outside))
        XCTAssertTrue(try FileManager.default.contentsOfDirectory(atPath: outside.path).isEmpty)
    }
}
