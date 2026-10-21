import XCTest
import AppKit
import ImageIO
import UniformTypeIdentifiers
@testable import InfinityInstallerStudio

final class UIKitStoreTests: XCTestCase {
    // ------------------------=
    // FUNC: testIndependentSurfaceStylesPersistAndUndo
    // DESC: Exercises independent button/panel materials, corner geometry and native binary serialization through save and reload.
    // ------------------=
    @MainActor func testIndependentSurfaceStylesPersistAndUndo() throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let store = UIKitStore(projectRoot: root)
        let panel = UIKitSurfaceStyle(material: .matte, corners: .square, radius: 0,
                                      fill: 0x80123456, border: 0xffabcdef)
        let button = UIKitSurfaceStyle(material: .glass, corners: .rounded, radius: 18,
                                       fill: nil, border: 0x00000000)
        store.setSurfaceStyle(2, style: panel)
        store.setSurfaceStyle(3, style: button)
        XCTAssertEqual(store.document.styles[2], panel)
        XCTAssertEqual(store.document.styles[4], .authored)
        store.undo()
        XCTAssertEqual(store.document.styles[3], .authored)
        XCTAssertEqual(store.document.styles[2], panel)
        store.redo()
        try store.save()
        let loaded = UIKitStore(projectRoot: root)
        XCTAssertEqual(loaded.document, store.document)
        let data = try Data(contentsOf: root.appending(path: "assets/ui-design-kit/default/uikit-styles.bin"))
        XCTAssertEqual(data, try loaded.document.runtimeStyles())
        XCTAssertEqual(data.count, 84)
        XCTAssertEqual(Array(data[36..<48]), [2, 1, 0, 3, 0x56, 0x34, 0x12, 0x80, 0xef, 0xcd, 0xab, 0xff])
        XCTAssertEqual(Array(data[48..<52]), [1, 2, 18, 2])
        loaded.resetSurfaceStyle(2)
        XCTAssertEqual(loaded.document.styles[2], .authored)
        XCTAssertEqual(loaded.document.styles[3], button)
        loaded.undo()
        XCTAssertEqual(loaded.document.styles[2], panel)
    }

    // ------------------------=
    // FUNC: testLegacyStyleDefaultsAndInvalidStyleRejection
    // DESC: Keeps old saved projects visually unchanged and rejects malformed surface contracts before publication.
    // ------------------=
    func testLegacyStyleDefaultsAndInvalidStyleRejection() throws {
        let source = try JSONEncoder().encode(UIKitDocument())
        var object = try XCTUnwrap(JSONSerialization.jsonObject(with: source) as? [String: Any])
        object.removeValue(forKey: "styles")
        let legacy = try JSONDecoder().decode(UIKitDocument.self, from: JSONSerialization.data(withJSONObject: object))
        XCTAssertEqual(legacy.styles, UIKitSurfaceStyle.defaults)
        var invalid = legacy
        invalid.styles[2].radius = 41
        XCTAssertThrowsError(try invalid.validate())
        invalid = legacy; invalid.styles.removeLast()
        XCTAssertThrowsError(try invalid.runtimeStyles())
    }

    // ------------------------=
    // FUNC: testSemanticColorBinaryAndValidation
    // DESC: Checks actual runtime token bytes, required role count and rejection of unsafe document overrides.
    // ------------------=
    func testSemanticColorBinaryAndValidation() throws {
        var document = UIKitDocument()
        document.colors[UIKitColorRole.panel.rawValue] = 0x85123456
        let binary = try document.runtimeTokens()
        XCTAssertEqual(binary.count, 64)
        XCTAssertEqual(Array(binary[8..<12]), [1, 0, 13, 0])
        XCTAssertEqual(Array(binary[16..<20]), [0x56, 0x34, 0x12, 0x85])
        document.colors.removeLast()
        XCTAssertThrowsError(try document.validate())
        document = UIKitDocument()
        document.overrides = [UIKitOverride(id: "bad", target: "assets/../../bad.png", offset: 0,
            width: 2, height: 2, source: "assets/ui-design-kit/overrides/upload.png", format: "png")]
        XCTAssertThrowsError(try document.validate())
    }

    // ------------------------=
    // FUNC: testUIKitSaveReopenColorUndoAndReset
    // DESC: Saves and reopens semantic settings through real project files while retaining isolated undo history.
    // ------------------=
    @MainActor func testUIKitSaveReopenColorUndoAndReset() throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let store = UIKitStore(projectRoot: root)
        XCTAssertTrue(store.validationIssues.isEmpty)
        store.setColor(.accent, argb: 0xff1234ab)
        XCTAssertTrue(store.isDirty)
        store.undo()
        XCTAssertEqual(store.document.colors, UIKitDocument.defaultColors)
        store.redo()
        XCTAssertEqual(store.document.colors[UIKitColorRole.accent.rawValue], 0xff1234ab)
        try store.save()
        XCTAssertFalse(store.isDirty)
        let loaded = UIKitStore(projectRoot: root)
        XCTAssertEqual(loaded.document, store.document)
        XCTAssertEqual(try Data(contentsOf: root.appending(path: "assets/ui-design-kit/default/uikit-tokens.bin")),
            try loaded.document.runtimeTokens())
        loaded.resetColors()
        XCTAssertEqual(loaded.document.colors, UIKitDocument.defaultColors)
        loaded.undo()
        XCTAssertEqual(loaded.document, store.document)
    }

    // ------------------------=
    // FUNC: testArtworkImportPreviewPersistenceAndOriginalReset
    // DESC: Proves imported pixels reach the preview and saved state without overwriting original art, including reset after build output changes.
    // ------------------=
    @MainActor func testArtworkImportPreviewPersistenceAndOriginalReset() throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let target = root.appending(path: "assets/icons/test.png")
        let upload = root.appending(path: "upload.png")
        let original = try png(red: 12, green: 30, blue: 60, alpha: 255)
        try original.write(to: target)
        try png(red: 200, green: 40, blue: 100, alpha: 120).write(to: upload)
        let store = UIKitStore(projectRoot: root)
        let asset = try XCTUnwrap(store.assets.first { $0.target == "assets/icons/test.png" })
        try store.importAsset(from: upload, for: asset)
        XCTAssertEqual(try Data(contentsOf: target), original)
        XCTAssertEqual(store.document.overrides.count, 1)
        let importedColor = try previewPixel(store, asset: asset)
        XCTAssertEqual(importedColor.redComponent, 200.0 / 255.0, accuracy: 0.015)
        XCTAssertEqual(importedColor.alphaComponent, 120.0 / 255.0, accuracy: 0.015)
        try store.save()
        let loaded = UIKitStore(projectRoot: root)
        XCTAssertEqual(loaded.document, store.document)
        loaded.selectAsset(asset.id)
        // Simulate the emitted runtime artwork; reset must still preview the immutable original.
        try Data(contentsOf: upload).write(to: target)
        loaded.resetSelectedAsset()
        XCTAssertTrue(loaded.document.overrides.isEmpty)
        let restoredColor = try previewPixel(loaded, asset: asset)
        XCTAssertEqual(restoredColor.redComponent, 12.0 / 255.0, accuracy: 0.015)
        XCTAssertEqual(restoredColor.alphaComponent, 1, accuracy: 0.01)
        loaded.undo()
        XCTAssertEqual(loaded.document.overrides.count, 1)
        XCTAssertTrue(FileManager.default.fileExists(atPath: root.appending(path: "assets/ui-design-kit/uikit-baselines.json").path))
    }

    // ------------------------=
    // FUNC: testRawAtlasCellPreviewAndIndependentReplacement
    // DESC: Reads the correct native RGBA cell and edits only its identified override rather than the whole state atlas.
    // ------------------=
    @MainActor func testRawAtlasCellPreviewAndIndependentReplacement() throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let target = "assets/ui-design-kit/default/test.rgba"
        let first = UIKitAsset(id: "test.normal", label: "Normal", group: "Panels & Controls", target: target,
            width: 2, height: 2, format: "rgba")
        let second = UIKitAsset(id: "test.focus", label: "Focus", group: "Panels & Controls", target: target,
            offset: 16, width: 2, height: 2, format: "rgba")
        struct FixtureCatalog: Encodable { let version = 1; let elements: [UIKitAsset] }
        try JSONEncoder().encode(FixtureCatalog(elements: [first, second]))
            .write(to: root.appending(path: "assets/ui-design-kit/uikit-catalog.json"))
        let source = Data(Array(repeating: [UInt8(20), 40, 80, 255], count: 4).flatMap { $0 }
            + Array(repeating: [UInt8(80), 120, 200, 128], count: 4).flatMap { $0 })
        try source.write(to: root.appending(path: target))
        let store = UIKitStore(projectRoot: root)
        XCTAssertEqual(store.assets.count, 2)
        let firstPixel = try previewPixel(store, asset: first)
        let secondPixel = try previewPixel(store, asset: second)
        XCTAssertEqual(firstPixel.blueComponent, 80.0 / 255.0, accuracy: 0.01)
        XCTAssertEqual(secondPixel.blueComponent, 200.0 / 255.0, accuracy: 0.01)
        XCTAssertEqual(secondPixel.alphaComponent, 128.0 / 255.0, accuracy: 0.01)
        let upload = root.appending(path: "upload.png")
        try png(red: 220, green: 0, blue: 30, alpha: 255).write(to: upload)
        try store.importAsset(from: upload, for: second)
        XCTAssertEqual(store.document.overrides.map(\.offset), [16])
        XCTAssertEqual(try Data(contentsOf: root.appending(path: target)), source)
        XCTAssertEqual(try previewPixel(store, asset: first).blueComponent, 80.0 / 255.0, accuracy: 0.01)
        let candidate = try XCTUnwrap(store.previewAtlas(target: target))
        XCTAssertEqual(candidate.prefix(16), source.prefix(16))
        XCTAssertEqual(Array(candidate[16..<20]), [220, 0, 30, 255])
        try store.save()
        let loaded = UIKitStore(projectRoot: root)
        XCTAssertEqual(loaded.document, store.document)
        loaded.selectAsset(second.id); loaded.resetSelectedAsset()
        XCTAssertEqual(try previewPixel(loaded, asset: second).alphaComponent, 128.0 / 255.0, accuracy: 0.01)
    }

    // ------------------------=
    // FUNC: testInvalidUploadAndEscapedAssetsAreRejected
    // DESC: Ensures malformed input and symlink path escapes cannot write outside the chosen project or mutate its document.
    // ------------------=
    @MainActor func testInvalidUploadAndEscapedAssetsAreRejected() throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let target = root.appending(path: "assets/icons/test.png")
        try png(red: 0, green: 0, blue: 0, alpha: 0).write(to: target)
        let invalid = root.appending(path: "invalid.png")
        try Data([1, 2, 3]).write(to: invalid)
        let store = UIKitStore(projectRoot: root)
        let asset = try XCTUnwrap(store.assets.first)
        XCTAssertThrowsError(try store.importAsset(from: invalid, for: asset))
        XCTAssertFalse(store.isDirty)
        XCTAssertThrowsError(try UIKitAssetCatalog.resolvedURL("assets/../escape.png", root: root))
        let link = root.appending(path: "assets/escape")
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: root.deletingLastPathComponent())
        XCTAssertThrowsError(try UIKitAssetCatalog.resolvedURL("assets/escape/no.png", root: root))
    }

    // ------------------------=
    // FUNC: testImportedArtworkNamedLikeBuiltInIsActuallyPackaged
    // DESC: Verifies a user replacement with a familiar filename still contributes actual bitmap bytes to the runtime document.
    // ------------------=
    func testImportedArtworkNamedLikeBuiltInIsActuallyPackaged() throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let directory = root.appending(path: "assets/boot/installer-assets")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let relative = "installer-assets/infinity-installer-masthead-v2.png"
        let image = try png(red: 77, green: 99, blue: 155, alpha: 255)
        try image.write(to: root.appending(path: "assets/boot/" + relative))
        var document = InstallerStudioDocument.factoryDefault()
        document.screens[0].elements.append(.make(name: "Imported replacement", kind: .image,
            frame: CanvasRect(x: 100, y: 100, width: 100, height: 100), imageAsset: relative, zIndex: 60))
        let withoutPixels = try RuntimeTemplateCodec.encode(document)
        let withPixels = try RuntimeTemplateCodec.encode(document, assetRoot: root)
        XCTAssertGreaterThan(withPixels.count, withoutPixels.count + 32)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(withPixels), document)
    }

    // ------------------------=
    // FUNC: fixtureRoot
    // DESC: Creates isolated disposable test data within the repository-owned build validation tree, never the host temp directory.
    // ------------------=
    private func fixtureRoot() throws -> URL {
        var repository = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { repository.deleteLastPathComponent() }
        let root = repository.appending(path: "builds/validation/studio-uikit/\(UUID().uuidString)", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: root.appending(path: "assets/ui-design-kit/default"), withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: root.appending(path: "assets/icons"), withIntermediateDirectories: true)
        return root
    }

    // ------------------------=
    // FUNC: png
    // DESC: Creates deterministic tiny test pixels with explicit alpha for upload and rendering assertions.
    // ------------------=
    private func png(red: UInt8, green: UInt8, blue: UInt8, alpha: UInt8) throws -> Data {
        let bytes = Data(Array(repeating: [red, green, blue, alpha], count: 4).flatMap { $0 })
        let provider = try XCTUnwrap(CGDataProvider(data: bytes as CFData))
        let image = try XCTUnwrap(CGImage(width: 2, height: 2, bitsPerComponent: 8, bitsPerPixel: 32,
            bytesPerRow: 8, space: CGColorSpace(name: CGColorSpace.sRGB)!,
            bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue), provider: provider,
            decode: nil, shouldInterpolate: false, intent: .defaultIntent))
        let output = NSMutableData()
        let destination = try XCTUnwrap(CGImageDestinationCreateWithData(output, UTType.png.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(destination, image, nil)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return output as Data
    }

    // ------------------------=
    // FUNC: previewPixel
    // DESC: Observes actual preview image pixels rather than source text or descriptive log output.
    // ------------------=
    @MainActor private func previewPixel(_ store: UIKitStore, asset: UIKitAsset) throws -> NSColor {
        let image = try XCTUnwrap(store.previewImage(for: asset))
        let pixels = try XCTUnwrap(image.cgImage(forProposedRect: nil, context: nil, hints: nil))
        var bytes = [UInt8](repeating: 0, count: 4)
        try bytes.withUnsafeMutableBytes { buffer in
            let context = try XCTUnwrap(CGContext(data: buffer.baseAddress, width: 1, height: 1,
                bitsPerComponent: 8, bytesPerRow: 4, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue))
            context.draw(pixels, in: CGRect(x: 0, y: 0, width: 1, height: 1))
        }
        let alpha = CGFloat(bytes[3])
        return NSColor(srgbRed: alpha == 0 ? 0 : CGFloat(bytes[0]) / alpha,
            green: alpha == 0 ? 0 : CGFloat(bytes[1]) / alpha,
            blue: alpha == 0 ? 0 : CGFloat(bytes[2]) / alpha, alpha: alpha / 255)
    }
}
