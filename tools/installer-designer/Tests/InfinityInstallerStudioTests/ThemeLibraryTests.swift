import XCTest
import AppKit
import ImageIO
import UniformTypeIdentifiers
import SwiftUI
@testable import InfinityInstallerStudio

final class ThemeLibraryTests: XCTestCase {
    private let atlas = "assets/ui-design-kit/default/test.rgba"

    // ------------------------=
    // FUNC: testCaptureThemeLibraryWorkspace
    // DESC: Captures the actual named-theme workspace for optional visual review without applying themes or saving user documents.
    // ------------------=
    @MainActor func testCaptureThemeLibraryWorkspace() throws {
        guard let directory = ProcessInfo.processInfo.environment["INFINITY_STUDIO_PRECISION_PROOFS"] else { return }
        var repository = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { repository.deleteLastPathComponent() }
        let destination = URL(fileURLWithPath: directory).resolvingSymlinksInPath().standardizedFileURL
        guard destination.path.hasPrefix(repository.resolvingSymlinksInPath().path + "/builds/") else {
            throw CocoaError(.fileWriteInvalidFileName)
        }
        let store = UIKitStore(projectRoot: repository)
        let before = store.document
        let view = ThemeLibraryWorkspace(store: store).frame(width: 1120, height: 820)
            .background(InfinityUIKit.Palette.nativeCanvas).environment(\.colorScheme, .dark)
        _ = NSApplication.shared
        let host = NSHostingView(rootView: view)
        let window = NSWindow(contentRect: NSRect(x: -12000, y: -12000, width: 1120, height: 820),
            styleMask: .borderless, backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = host
        window.orderFrontRegardless()
        defer { window.orderOut(nil); window.close() }
        host.frame = NSRect(x: 0, y: 0, width: 1120, height: 820)
        host.layoutSubtreeIfNeeded()
        RunLoop.main.run(until: Date().addingTimeInterval(0.15))
        host.layoutSubtreeIfNeeded()
        window.displayIfNeeded()
        let bitmap = try XCTUnwrap(host.bitmapImageRepForCachingDisplay(in: host.bounds))
        host.cacheDisplay(in: host.bounds, to: bitmap)
        let image = try XCTUnwrap(bitmap.cgImage)
        let png = try XCTUnwrap(bitmap.representation(using: .png, properties: [:]))
        try FileManager.default.createDirectory(at: destination, withIntermediateDirectories: true)
        try png.write(to: destination.appending(path: "theme-library-workspace.png"), options: .atomic)
        XCTAssertEqual(Double(image.width) / Double(image.height), 1120.0 / 820.0, accuracy: 0.001)
        try assertVisibleThemeCards(image)
        XCTAssertEqual(store.document, before)
        XCTAssertFalse(store.isDirty)
    }

    // ------------------------=
    // FUNC: assertVisibleThemeCards
    // DESC: Rejects blank proofs by checking contrasting text and artwork pixels in each of the three theme-card regions.
    // ------------------=
    private func assertVisibleThemeCards(_ image: CGImage) throws {
        var rgba = [UInt8](repeating: 0, count: 1120 * 820 * 4)
        try rgba.withUnsafeMutableBytes { buffer in
            let context = try XCTUnwrap(CGContext(data: buffer.baseAddress, width: 1120, height: 820,
                bitsPerComponent: 8, bytesPerRow: 1120 * 4, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue))
            context.draw(image, in: CGRect(x: 0, y: 0, width: 1120, height: 820))
        }
        // The production adaptive grid fits four 256-point slots at this viewport;
        // the three built-in presets occupy its first three slots.
        let slotWidth = (1120 - 48 - 3 * 16) / 4
        for column in 0..<3 {
            var contrastPixels = 0
            for y in 210..<640 {
                let left = 24 + column * (slotWidth + 16)
                for x in left..<(left + slotWidth) {
                    let pixel = (y * 1120 + x) * 4
                    if rgba[pixel] > 110 && rgba[pixel + 1] > 130 && rgba[pixel + 2] > 140 {
                        contrastPixels += 1
                    }
                }
            }
            XCTAssertGreaterThan(contrastPixels, 150, "Theme card \(column) must contain visible text and native artwork")
        }
    }

    // ------------------------=
    // FUNC: testBuiltInsAreImmutableAndApplicationIsUndoableUnsaved
    // DESC: Exercises preset selection through the store while proving immutable identities, isolated styles and draft-only application.
    // ------------------=
    @MainActor func testBuiltInsAreImmutableAndApplicationIsUndoableUnsaved() throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let store = UIKitStore(projectRoot: root)
        let library = ThemeLibrary(projectRoot: root)
        XCTAssertEqual(library.themes.count, 3)
        XCTAssertTrue(library.themes.allSatisfy(\.isBuiltIn))
        XCTAssertEqual(Set(library.themes.map(\.id)).count, 3)
        try library.apply(id: ThemeLibrary.glassID, store: store)
        XCTAssertTrue(store.isDirty)
        XCTAssertEqual(store.document.styles[2].material, .glass)
        XCTAssertEqual(store.document.styles[2].corners, .rounded)
        XCTAssertEqual(store.document.styles[0], .authored)
        let glass = store.document
        store.undo()
        XCTAssertEqual(store.document, UIKitDocument())
        store.redo()
        XCTAssertEqual(store.document, glass)
        XCTAssertThrowsError(try library.rename(id: ThemeLibrary.glassID, name: "Changed"))
        XCTAssertThrowsError(try library.delete(id: ThemeLibrary.glassID))
        XCTAssertThrowsError(try library.saveCurrent(name: "Changed", store: store, replacingID: ThemeLibrary.glassID))
        XCTAssertEqual(store.document, glass)
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appending(path: "assets/ui-design-kit/uikit.infinityui").path))
        try library.apply(id: ThemeLibrary.matteID, store: store)
        XCTAssertEqual(store.document.styles[2].material, .matte)
        XCTAssertEqual(store.document.styles[2].corners, .square)
        try library.apply(id: ThemeLibrary.precisionID, store: store)
        XCTAssertEqual(store.document, UIKitDocument())
    }

    // ------------------------=
    // FUNC: testCustomThemeLifecyclePreservesIdentityAndRecoversTrash
    // DESC: Saves, replaces, renames, duplicates, reloads, exports and recoverably deletes actual custom theme bundles.
    // ------------------=
    @MainActor func testCustomThemeLifecyclePreservesIdentityAndRecoversTrash() throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let store = UIKitStore(projectRoot: root)
        let library = ThemeLibrary(projectRoot: root)
        store.setColor(.accent, argb: 0x80123456)
        let first = try library.saveCurrent(name: "  Night / Personal  ", store: store)
        XCTAssertFalse(first.isBuiltIn)
        XCTAssertEqual(first.name, "Night / Personal")
        XCTAssertTrue(store.isDirty)
        store.setColor(.accent, argb: 0x80765432)
        let updated = try library.saveCurrent(name: first.name, store: store, replacingID: first.id)
        XCTAssertEqual(updated.id, first.id)
        XCTAssertEqual(try library.bundle(id: first.id).document, store.document)
        let renamed = try library.rename(id: first.id, name: "Updated")
        XCTAssertEqual(renamed.id, first.id)
        let second = try library.duplicate(id: first.id, name: "Second")
        XCTAssertNotEqual(second.id, first.id)
        XCTAssertEqual(second.previewDocument, renamed.previewDocument)
        let reopened = ThemeLibrary(projectRoot: root)
        XCTAssertEqual(Set(reopened.themes.map(\.id)), Set(library.themes.map(\.id)))
        let exported = root.appending(path: "builds/exports/themes/copy.infinitytheme")
        try library.export(id: first.id, to: exported)
        XCTAssertEqual(try ThemeBundle.read(from: exported, catalog: store.assets), try library.bundle(id: first.id))
        let trash = try library.delete(id: first.id)
        XCTAssertTrue(FileManager.default.fileExists(atPath: trash.path))
        XCTAssertEqual(library.lastTrashURL, trash)
        XCTAssertFalse(library.themes.contains { $0.id == first.id })
        let restored = try library.importTheme(from: trash)
        XCTAssertEqual(restored.id, first.id)
        XCTAssertEqual(restored.previewDocument, store.document)
    }

    // ------------------------=
    // FUNC: testPortableArtworkPreviewApplySaveAndUndoAcrossProjects
    // DESC: Transfers a complete theme between independent roots, observing exact native pixels before and after applying and saving.
    // ------------------=
    @MainActor func testPortableArtworkPreviewApplySaveAndUndoAcrossProjects() throws {
        let source = try fixtureRoot()
        let target = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: source); try? FileManager.default.removeItem(at: target) }
        let author = UIKitStore(projectRoot: source)
        let library = ThemeLibrary(projectRoot: source)
        let upload = source.appending(path: "uploaded.png")
        try png(red: 220, green: 40, blue: 80).write(to: upload)
        try author.importAsset(from: upload, for: XCTUnwrap(author.assets.first))
        author.setColor(.accent, argb: 0xff345678)
        let saved = try library.saveCurrent(name: "Portable", store: author)
        let file = source.appending(path: "builds/exports/themes/portable.infinitytheme")
        try library.export(id: saved.id, to: file)
        let editor = UIKitStore(projectRoot: target)
        let receiver = ThemeLibrary(projectRoot: target)
        let original = try Data(contentsOf: target.appending(path: atlas))
        let imported = try receiver.importTheme(from: file)
        XCTAssertEqual(imported.previewDocument, author.document)
        let imagePath = try XCTUnwrap(imported.previewDocument.overrides.first?.source)
        XCTAssertFalse(FileManager.default.fileExists(atPath: target.appending(path: imagePath).path))
        let preview = try XCTUnwrap(receiver.previewAtlas(id: imported.id, target: atlas, store: editor))
        XCTAssertEqual(Array(preview.prefix(4)), [220, 40, 80, 255])
        XCTAssertEqual(preview.suffix(16), original.suffix(16))
        XCTAssertEqual(editor.document, UIKitDocument())
        XCTAssertFalse(FileManager.default.fileExists(atPath: target.appending(path: imagePath).path))
        try receiver.apply(id: imported.id, store: editor)
        XCTAssertEqual(editor.document, author.document)
        XCTAssertTrue(editor.isDirty)
        XCTAssertTrue(FileManager.default.fileExists(atPath: target.appending(path: imagePath).path))
        XCTAssertEqual(try Data(contentsOf: target.appending(path: atlas)), original)
        editor.undo(); XCTAssertEqual(editor.document, UIKitDocument())
        editor.redo(); XCTAssertEqual(editor.document, author.document)
        try editor.save()
        XCTAssertEqual(UIKitStore(projectRoot: target).document, author.document)
        XCTAssertEqual(try Data(contentsOf: target.appending(path: "assets/ui-design-kit/default/uikit-tokens.bin")), try author.document.runtimeTokens())
        // Simulate a build emitting the custom atlas; built-in cards must retain the approved original pixels.
        try preview.write(to: target.appending(path: atlas))
        XCTAssertEqual(receiver.previewAtlas(id: ThemeLibrary.precisionID, target: atlas, store: editor), original)
        XCTAssertEqual(receiver.previewAtlas(id: imported.id, target: atlas, store: editor), preview)
        XCTAssertEqual(editor.document, author.document)
    }

    // ------------------------=
    // FUNC: testBundleRejectsTamperingTraversalAndIncompatibleCatalog
    // DESC: Validates SHA, schema, exact catalog contracts, PNG dimensions, native ranges and source containment before import.
    // ------------------=
    @MainActor func testBundleRejectsTamperingTraversalAndIncompatibleCatalog() throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let store = UIKitStore(projectRoot: root)
        let upload = root.appending(path: "upload.png")
        try png(red: 240, green: 12, blue: 30).write(to: upload)
        try store.importAsset(from: upload, for: XCTUnwrap(store.assets.first))
        let valid = try ThemeBundle.capture(document: store.document, catalog: store.assets, root: root, name: "Valid")
        try valid.validate(catalog: store.assets)
        var changed = valid; changed.version = 9
        XCTAssertThrowsError(try changed.validate(catalog: store.assets))
        changed = valid; changed.artwork[0].png[0] ^= 1
        XCTAssertThrowsError(try changed.validate(catalog: store.assets))
        changed = valid; changed.artwork.append(changed.artwork[0])
        XCTAssertThrowsError(try changed.validate(catalog: store.assets))
        changed = valid; changed.document.overrides[0].source = "assets/ui-design-kit/overrides/../../escape.png"
        XCTAssertThrowsError(try changed.validate(catalog: store.assets))
        changed = valid; changed.document.overrides[0].format = "bmp"
        XCTAssertThrowsError(try changed.validate(catalog: store.assets))
        changed = valid; changed.document.overrides[0].width = 3
        XCTAssertThrowsError(try changed.validate(catalog: store.assets))
        var alteredCatalog = store.assets
        alteredCatalog[0].offset = Int.max
        changed = valid; changed.document.overrides[0].offset = Int.max
        XCTAssertThrowsError(try changed.validate(catalog: alteredCatalog))
        let second = try XCTUnwrap(store.assets.last)
        changed = valid
        var overlapping = changed.document.overrides[0]
        overlapping.id = second.id; overlapping.offset = 8
        alteredCatalog = store.assets; alteredCatalog[alteredCatalog.count - 1].offset = 8
        changed.document.overrides.append(overlapping)
        XCTAssertThrowsError(try changed.validate(catalog: alteredCatalog))
        let badFile = root.appending(path: "bad.infinitytheme")
        try JSONEncoder().encode(changed).write(to: badFile)
        let library = ThemeLibrary(projectRoot: root)
        let draft = store.document
        XCTAssertThrowsError(try library.importTheme(from: badFile, applyTo: store))
        XCTAssertEqual(store.document, draft)
        XCTAssertEqual(library.themes.count, 3)
    }

    // ------------------------=
    // FUNC: testConflictingImportAndFailedSavePreserveDraftAndSavedTheme
    // DESC: Rejects implicit identity replacement and invalid saves without discarding unsaved UIKit edits.
    // ------------------=
    @MainActor func testConflictingImportAndFailedSavePreserveDraftAndSavedTheme() throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let store = UIKitStore(projectRoot: root)
        let library = ThemeLibrary(projectRoot: root)
        let saved = try library.saveCurrent(name: "Original", store: store)
        var conflict = try library.bundle(id: saved.id)
        conflict.document.colors[0] ^= 255
        let file = root.appending(path: "conflict.infinitytheme")
        try conflict.encoded(catalog: store.assets).write(to: file)
        store.setColor(.accent, argb: 0x88336699)
        let draft = store.document
        XCTAssertThrowsError(try library.importTheme(from: file, applyTo: store))
        XCTAssertThrowsError(try library.saveCurrent(name: "\n", store: store, replacingID: saved.id))
        XCTAssertThrowsError(try library.saveCurrent(name: "Unknown", store: store, replacingID: UUID()))
        XCTAssertEqual(store.document, draft)
        XCTAssertEqual(try library.bundle(id: saved.id).document, UIKitDocument())
        XCTAssertTrue(store.isDirty)
    }

    // ------------------------=
    // FUNC: testExportsAreContainedAndBuiltInsArePortable
    // DESC: Prevents project escape and managed-library overwrite while exporting self-contained built-ins and unsaved drafts.
    // ------------------=
    @MainActor func testExportsAreContainedAndBuiltInsArePortable() throws {
        let root = try fixtureRoot()
        let other = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root); try? FileManager.default.removeItem(at: other) }
        let library = ThemeLibrary(projectRoot: root)
        let store = UIKitStore(projectRoot: root)
        XCTAssertThrowsError(try library.export(id: ThemeLibrary.precisionID, to: other.appending(path: "outside.infinitytheme")))
        try FileManager.default.createSymbolicLink(at: root.appending(path: "shortcut"), withDestinationURL: other)
        XCTAssertThrowsError(try library.export(id: ThemeLibrary.precisionID, to: root.appending(path: "shortcut/out.infinitytheme")))
        XCTAssertThrowsError(try library.export(id: ThemeLibrary.precisionID, to: root.appending(path: "assets/ui-design-kit/themes/test.infinitytheme")))
        let target = root.appending(path: "builds/exports/themes/precision.infinitytheme")
        try library.export(id: ThemeLibrary.precisionID, to: target)
        XCTAssertEqual(try ThemeBundle.read(from: target, catalog: store.assets), try library.bundle(id: ThemeLibrary.precisionID))
        XCTAssertEqual(try library.importTheme(from: target).id, ThemeLibrary.precisionID)
        store.setColor(.accent, argb: 0xffabcdef)
        try library.exportCurrent(name: "Draft", store: store, to: target)
        XCTAssertEqual(try ThemeBundle.read(from: target, catalog: store.assets).document, store.document)
        XCTAssertTrue(store.isDirty)
        XCTAssertEqual(library.themes.count, 3)
    }

    // ------------------------=
    // FUNC: testOversizedFileAndMissingArtworkNeverApply
    // DESC: Checks bounded input parsing and failed artwork staging without huge allocations or mutation of the editor.
    // ------------------=
    @MainActor func testOversizedFileAndMissingArtworkNeverApply() throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let store = UIKitStore(projectRoot: root)
        let library = ThemeLibrary(projectRoot: root)
        let huge = root.appending(path: "oversized.infinitytheme")
        XCTAssertTrue(FileManager.default.createFile(atPath: huge.path, contents: Data()))
        let handle = try FileHandle(forWritingTo: huge)
        try handle.truncate(atOffset: UInt64(ThemeBundle.maximumFileBytes + 1)); try handle.close()
        XCTAssertThrowsError(try library.importTheme(from: huge, applyTo: store))
        XCTAssertEqual(store.document, UIKitDocument())
        let upload = root.appending(path: "upload.png")
        try png(red: 220, green: 30, blue: 60).write(to: upload)
        try store.importAsset(from: upload, for: XCTUnwrap(store.assets.first))
        let saved = try library.saveCurrent(name: "Artwork", store: store)
        let originalDocument = store.document
        let source = try XCTUnwrap(store.document.overrides.first?.source)
        try Data([0, 1, 2]).write(to: root.appending(path: source))
        XCTAssertThrowsError(try library.apply(id: saved.id, store: store))
        XCTAssertEqual(store.document, originalDocument)
    }

    // ------------------------=
    // FUNC: testFailedApplyRollsBackStagedUploadsAndKeepsEditorDraft
    // DESC: Injects a missing runtime target after import and proves application failure cannot leave new uploads or replace user edits.
    // ------------------=
    @MainActor func testFailedApplyRollsBackStagedUploadsAndKeepsEditorDraft() throws {
        let source = try fixtureRoot()
        let target = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: source); try? FileManager.default.removeItem(at: target) }
        let author = UIKitStore(projectRoot: source)
        let upload = source.appending(path: "upload.png")
        try png(red: 230, green: 120, blue: 80).write(to: upload)
        try author.importAsset(from: upload, for: XCTUnwrap(author.assets.first))
        let exported = source.appending(path: "export.infinitytheme")
        try ThemeLibrary(projectRoot: source).exportCurrent(name: "Broken target", store: author, to: exported)
        let editor = UIKitStore(projectRoot: target)
        editor.setColor(.accent, argb: 0xff987654)
        let draft = editor.document
        let library = ThemeLibrary(projectRoot: target)
        let imported = try library.importTheme(from: exported)
        let pngPath = try XCTUnwrap(imported.previewDocument.overrides.first?.source)
        try FileManager.default.removeItem(at: target.appending(path: atlas))
        XCTAssertThrowsError(try library.apply(id: imported.id, store: editor))
        XCTAssertEqual(editor.document, draft)
        XCTAssertTrue(editor.isDirty)
        XCTAssertFalse(FileManager.default.fileExists(atPath: target.appending(path: pngPath).path))
        let directory = target.appending(path: "assets/ui-design-kit/overrides")
        XCTAssertTrue(try FileManager.default.contentsOfDirectory(atPath: directory.path).isEmpty)
    }

    // ------------------------=
    // FUNC: testMalformedCustomFileDoesNotHideValidPresetsOrThemes
    // DESC: Isolates broken custom files and rejects theme-folder symlinks without losing valid library entries.
    // ------------------=
    @MainActor func testMalformedCustomFileDoesNotHideValidPresetsOrThemes() throws {
        let root = try fixtureRoot()
        let other = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root); try? FileManager.default.removeItem(at: other) }
        let library = ThemeLibrary(projectRoot: root)
        let store = UIKitStore(projectRoot: root)
        let good = try library.saveCurrent(name: "Good", store: store)
        let invalid = root.appending(path: "assets/ui-design-kit/themes/\(UUID().uuidString.lowercased()).infinitytheme")
        try Data([0, 1, 2]).write(to: invalid)
        try library.reload()
        XCTAssertEqual(library.issues.count, 1)
        XCTAssertEqual(library.themes.count, 4)
        XCTAssertTrue(library.themes.contains { $0.id == good.id })
        let second = ThemeLibrary(projectRoot: other)
        try FileManager.default.createSymbolicLink(at: other.appending(path: "assets/ui-design-kit/themes"), withDestinationURL: root)
        XCTAssertThrowsError(try second.saveCurrent(name: "Escape", store: UIKitStore(projectRoot: other)))
    }

    // ------------------------=
    // FUNC: fixtureRoot
    // DESC: Creates tiny native atlas fixtures beneath repository validation storage with no host temporary files.
    // ------------------=
    private func fixtureRoot() throws -> URL {
        var repository = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { repository.deleteLastPathComponent() }
        let root = repository.appending(path: "builds/validation/studio-themes/\(UUID().uuidString)", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: root.appending(path: "assets/ui-design-kit/default"), withIntermediateDirectories: true)
        let cells = [UIKitAsset(id: "test.normal", label: "A Normal", group: "Panels", target: atlas, width: 2, height: 2, format: "rgba"),
            UIKitAsset(id: "test.focus", label: "B Focus", group: "Panels", target: atlas, offset: 16, width: 2, height: 2, format: "rgba")]
        try JSONEncoder().encode(cells).write(to: root.appending(path: "assets/ui-design-kit/uikit-catalog.json"))
        try Data(Array(repeating: [UInt8(20), 40, 80, 255], count: 8).flatMap { $0 }).write(to: root.appending(path: atlas))
        return root
    }

    // ------------------------=
    // FUNC: png
    // DESC: Produces exact two-pixel RGBA test artwork for portable bundle and native atlas assertions.
    // ------------------=
    private func png(red: UInt8, green: UInt8, blue: UInt8) throws -> Data {
        let bytes = Data(Array(repeating: [red, green, blue, 255], count: 4).flatMap { $0 })
        let provider = try XCTUnwrap(CGDataProvider(data: bytes as CFData))
        let image = try XCTUnwrap(CGImage(width: 2, height: 2, bitsPerComponent: 8, bitsPerPixel: 32,
            bytesPerRow: 8, space: CGColorSpace(name: CGColorSpace.sRGB)!,
            bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue), provider: provider,
            decode: nil, shouldInterpolate: false, intent: .defaultIntent))
        let result = NSMutableData()
        let destination = try XCTUnwrap(CGImageDestinationCreateWithData(result, UTType.png.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(destination, image, nil)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return result as Data
    }
}
