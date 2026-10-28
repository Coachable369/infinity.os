import AppKit
import XCTest
@testable import InfinityInstallerStudio

final class StudioCloseGuardTests: XCTestCase {
    @MainActor private final class ExistingDelegate: NSObject, NSWindowDelegate {
        var closeCalls = 0
        var resizeCalls = 0

        // ------------------------=
        // FUNC: windowShouldClose
        // DESC: Supplies an observable existing delegate veto for lifecycle forwarding verification.
        // ------------------=
        func windowShouldClose(_ sender: NSWindow) -> Bool { closeCalls += 1; return false }

        // ------------------------=
        // FUNC: windowDidResize
        // DESC: Records an optional callback forwarded through the close guard.
        // ------------------=
        func windowDidResize(_ notification: Notification) { resizeCalls += 1 }
    }

    // ------------------------=
    // FUNC: workspace
    // DESC: Creates a disposable repository-local project so close/save tests never alter the user's drafts.
    // ------------------=
    private func workspace() throws -> URL {
        let repository = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let root = repository.appending(path: "builds/.studio-work/tmp/studio-close-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: root) }
        let documents = [InstallerStudioDocument.factoryBoot(), .factoryDefault(), .factoryConfiguration(), .factorySystemSettings(), .factoryLogin(), .factoryDesktop()]
        let references = Set(documents.flatMap(\.screens).flatMap(\.elements).map(\.imageAsset))
        for path in references where path.hasPrefix("assets/") {
            let source = repository.appending(path: path), destination = root.appending(path: path)
            try FileManager.default.createDirectory(at: destination.deletingLastPathComponent(), withIntermediateDirectories: true)
            try FileManager.default.copyItem(at: source, to: destination)
        }
        return root
    }

    // ------------------------=
    // FUNC: testDirtyStateSpansSectionsAndSaveClearsOnlyPublishedEdits
    // DESC: Saves actual files, verifies per-section dirty state, and confirms a successful close-save preserves all drafts.
    // ------------------=
    @MainActor func testDirtyStateSpansSectionsAndSaveClearsOnlyPublishedEdits() throws {
        let root = try workspace()
        let store = TemplateStore(projectRoot: root)
        XCTAssertFalse(store.hasUnsavedChanges)
        store.document.screens[0].title = "Edited installation"
        store.bootDocument.screens[0].title = "Edited ISO loading"
        store.configurationDocument.screens[0].title = "Edited configuration"
        store.uiKit.setColor(.accent, argb: 0xff325690)
        XCTAssertTrue(store.hasUnsavedChanges)
        XCTAssertFalse(store.resolveClose(.cancel))
        store.selectScreenCollection(.installation)
        store.save()
        XCTAssertTrue(store.validationIssues.isEmpty)
        XCTAssertTrue(store.hasUnsavedChanges)
        guard store.resolveClose(.save) else { XCTFail(store.validationIssues.joined(separator: "; ")); return }
        XCTAssertFalse(store.hasUnsavedChanges)
        let reopened = TemplateStore(projectRoot: root)
        XCTAssertEqual(reopened.bootDocument, store.bootDocument)
        XCTAssertEqual(reopened.document, store.document)
        XCTAssertEqual(reopened.configurationDocument, store.configurationDocument)
        XCTAssertEqual(reopened.settingsDocument, store.settingsDocument)
        XCTAssertEqual(reopened.loginDocument, store.loginDocument)
        XCTAssertEqual(reopened.desktopDocument, store.desktopDocument)
        XCTAssertEqual(reopened.uiKit.document, store.uiKit.document)
        XCTAssertFalse(reopened.hasUnsavedChanges)
    }

    // ------------------------=
    // FUNC: testFailedCloseSaveKeepsDraftAndDiscardDoesNotWrite
    // DESC: Proves validation errors prevent closing and discard permits closing without changing persisted data.
    // ------------------=
    @MainActor func testFailedCloseSaveKeepsDraftAndDiscardDoesNotWrite() throws {
        let root = try workspace()
        let store = TemplateStore(projectRoot: root)
        guard store.saveAll() else { XCTFail(store.validationIssues.joined(separator: "; ")); return }
        let file = root.appending(path: "assets/boot/installer-screens.infinityui")
        let before = try Data(contentsOf: file)
        store.document.screens.removeAll()
        XCTAssertTrue(store.hasUnsavedChanges)
        XCTAssertFalse(store.resolveClose(.save))
        XCTAssertTrue(store.hasUnsavedChanges)
        XCTAssertEqual(try Data(contentsOf: file), before)
        XCTAssertTrue(store.resolveClose(.discard))
        XCTAssertTrue(store.hasUnsavedChanges)
        XCTAssertEqual(try Data(contentsOf: file), before)
    }

    // ------------------------=
    // FUNC: testUnreadableSourcesCannotBeOverwrittenByFallbackDocuments
    // DESC: Corrupts persisted JSON and proves default display documents cannot silently replace the user's original files.
    // ------------------=
    @MainActor func testUnreadableSourcesCannotBeOverwrittenByFallbackDocuments() throws {
        let root = try workspace()
        let initial = TemplateStore(projectRoot: root)
        XCTAssertTrue(initial.saveAll())
        let source = root.appending(path: "assets/boot/configuration-screens.infinityui")
        let good = try Data(contentsOf: source)
        let invalid = Data([0xff, 0x00, 0x7b])
        try invalid.write(to: source)
        let reopened = TemplateStore(projectRoot: root)
        reopened.selectScreenCollection(.configuration)
        reopened.save()
        XCTAssertFalse(reopened.saveAll())
        XCTAssertEqual(try Data(contentsOf: source), invalid)
        let recovery = root.appending(path: "configuration-recovery.infinityui")
        try good.write(to: recovery)
        try reopened.importDocument(from: recovery)
        XCTAssertTrue(reopened.saveAll())
        XCTAssertEqual(try Data(contentsOf: source), good)

        let kit = root.appending(path: "assets/ui-design-kit/uikit.infinityui")
        try invalid.write(to: kit)
        let brokenKit = TemplateStore(projectRoot: root)
        brokenKit.uiKit.setColor(.accent, argb: 0xff123456)
        XCTAssertFalse(brokenKit.saveAll())
        XCTAssertThrowsError(try brokenKit.uiKit.save())
        XCTAssertEqual(try Data(contentsOf: kit), invalid)
    }

    // ------------------------=
    // FUNC: testWindowGuardForwardsAndRestoresTheExistingDelegate
    // DESC: Attaches to a real offscreen AppKit window and verifies veto forwarding and delegate restoration after removal.
    // ------------------=
    @MainActor func testWindowGuardForwardsAndRestoresTheExistingDelegate() throws {
        _ = NSApplication.shared
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 200, height: 150),
            styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.close() }
        let existing = ExistingDelegate()
        window.delegate = existing
        let store = TemplateStore(projectRoot: try workspace())
        let view = StudioWindowCloseGuard.GuardView(store: store)
        try XCTUnwrap(window.contentView).addSubview(view)
        let bridge = try XCTUnwrap(window.delegate as? StudioWindowCloseGuard.WindowDelegate)
        XCTAssertFalse(bridge.windowShouldClose(window))
        XCTAssertEqual(existing.closeCalls, 1)
        let notification = Notification(name: NSWindow.didResizeNotification, object: window)
        window.delegate?.windowDidResize?(notification)
        XCTAssertEqual(existing.resizeCalls, 1)
        view.removeFromSuperview()
        XCTAssertTrue(window.delegate === existing)
        XCTAssertNil(view.window)
    }
}
