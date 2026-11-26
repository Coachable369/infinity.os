import AppKit
import SwiftUI
import XCTest
@testable import InfinityInstallerStudio

final class AppDesignTests: XCTestCase {
    // ------------------------=
    // FUNC: testWorkspaceCapture
    // DESC: Captures the real Studio App Design workspace for visual review without changing saved designs.
    // ------------------=
    @MainActor func testWorkspaceCapture() throws {
        let store = TemplateStore()
        store.editingApps = true
        _ = NSApplication.shared
        let bounds = NSRect(x: 0, y: 0, width: 1400, height: 900)
        let host = NSHostingView(rootView: StudioRootView(store: store).frame(width: 1400, height: 900).environment(\.colorScheme, .dark))
        let window = NSWindow(contentRect: bounds, styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = host
        window.orderFrontRegardless()
        defer { window.orderOut(nil); window.close() }
        host.frame = bounds
        host.layoutSubtreeIfNeeded()
        RunLoop.main.run(until: Date().addingTimeInterval(0.3))
        host.layoutSubtreeIfNeeded()
        let bitmap = try XCTUnwrap(host.bitmapImageRepForCachingDisplay(in: bounds))
        host.cacheDisplay(in: bounds, to: bitmap)
        let root = try XCTUnwrap(store.projectRoot)
        let directory = root.appending(path: "builds/validation/app-design")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try XCTUnwrap(bitmap.representation(using: .png, properties: [:])).write(to: directory.appending(path: "studio.png"))
    }
    // ------------------------=
    // FUNC: testAtomicDesignRoundTrip
    // DESC: Verifies undo and source/binary agreement including multi-byte titles without mutating repository designs.
    // ------------------=
    @MainActor func testAtomicDesignRoundTrip() throws {
        let repository = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let root = repository.appending(path: "build/tmp/app-design-test-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appending(path: "runtime/install/System"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let store = AppDesignStore(root: root)
        store.selectedID = "code"
        let original = store.selected
        store.update { $0.title = "Éditeur"; $0.contentInset = 24; $0.inspectorSide = "left" }
        let edited = store.selected
        store.undo(); XCTAssertEqual(store.selected, original)
        store.redo(); XCTAssertEqual(store.selected, edited)
        let writes = try store.prepareSave(root: root)
        for write in writes {
            try FileManager.default.createDirectory(at: write.url.deletingLastPathComponent(), withIntermediateDirectories: true)
            try write.data.write(to: write.url)
        }
        let restored = AppDesignStore(root: root)
        restored.selectedID = "code"
        XCTAssertEqual(restored.selected, edited)
        let bytes = [UInt8](writes[1].data)
        XCTAssertEqual(bytes.count, 1536)
        XCTAssertEqual(Array(bytes[(6*128+16)..<(6*128+16+Int(bytes[6*128]))]), Array(edited.title.utf8))
        XCTAssertEqual(bytes[6*128+1], 24)
        XCTAssertEqual(bytes[6*128+3], 1)
    }
    // ------------------------=
    // FUNC: testMalformedCatalogPreservesSafeSelection
    // DESC: Keeps the editor usable and prevents overwriting an invalid source document.
    // ------------------=
    @MainActor func testMalformedCatalogPreservesSafeSelection() throws {
        let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath).appending(path: "build/tmp/app-design-invalid-\(UUID().uuidString)")
        let directory = root.appending(path: "runtime/install/System")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let data = try JSONEncoder().encode(AppDesignStore.Document(apps: []))
        try data.write(to: directory.appending(path: "app-design.json"))
        let store = AppDesignStore(root: root)
        XCTAssertEqual(store.selected.id, "code")
        XCTAssertNotNil(store.loadError)
        XCTAssertThrowsError(try store.prepareSave(root: root))
    }
    // ------------------------=
    // FUNC: testInvalidDesignIsNotPublishable
    // DESC: Rejects out-of-bounds geometry and invalid titles before saving any artifact.
    // ------------------=
    @MainActor func testInvalidDesignIsNotPublishable() throws {
        let store = AppDesignStore(root: nil)
        store.update { $0.contentInset = 400 }
        XCTAssertThrowsError(try store.validate())
        store.undo()
        store.update { $0.title = "\n" }
        XCTAssertThrowsError(try store.validate())
    }
}
