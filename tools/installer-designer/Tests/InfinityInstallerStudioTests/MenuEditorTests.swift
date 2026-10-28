import AppKit
import XCTest
import SwiftUI
@testable import InfinityInstallerStudio

final class MenuEditorTests: XCTestCase {
    // ------------------------=
    // FUNC: testMenuDeletionPreservesDesktopLayers
    // DESC: Verifies keyboard-command deletion affects only the selected menu subtree and remains undoable.
    // ------------------=
    @MainActor func testMenuDeletionPreservesDesktopLayers() throws {
        let store = TemplateStore()
        store.desktopDocument = .factoryDesktop()
        store.editInfinityMenu()
        let layers = store.desktopDocument.screens
        let before = store.menu
        let selected = try XCTUnwrap(store.selectedMenuItemID)
        store.deleteSelectedMenuItem()
        XCTAssertFalse(store.menu.rows.contains { $0.id == selected })
        XCTAssertEqual(store.desktopDocument.screens, layers)
        store.undo()
        XCTAssertEqual(store.menu, before)
    }
    // ------------------------=
    // FUNC: testWorkspaceRendering
    // DESC: Captures the real editor at compact and wide sizes without changing saved documents.
    // ------------------=
    @MainActor func testWorkspaceRendering() throws {
        let store = TemplateStore()
        store.desktopDocument = .factoryDesktop()
        store.editInfinityMenu()
        let before = store.desktopDocument
        let directory = try XCTUnwrap(store.projectRoot).appending(path: "builds/validation/studio-menu")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        _ = NSApplication.shared
        for width in [1040, 1440] {
            let height = 820
            let bounds = NSRect(x: 0, y: 0, width: width, height: height)
            let view = StudioRootView(store: store).frame(width: CGFloat(width), height: CGFloat(height))
                .environment(\.colorScheme, .dark)
            let host = NSHostingView(rootView: view)
            let window = NSWindow(contentRect: NSRect(x: -12000, y: -12000, width: width, height: height),
                styleMask: .borderless, backing: .buffered, defer: false)
            window.isReleasedWhenClosed = false
            window.contentView = host
            window.orderFrontRegardless()
            defer { window.orderOut(nil); window.close() }
            host.frame = bounds
            host.layoutSubtreeIfNeeded()
            RunLoop.main.run(until: Date().addingTimeInterval(0.2))
            host.layoutSubtreeIfNeeded()
            window.displayIfNeeded()
            let bitmap = try XCTUnwrap(host.bitmapImageRepForCachingDisplay(in: bounds))
            host.cacheDisplay(in: bounds, to: bitmap)
            let png = try XCTUnwrap(bitmap.representation(using: .png, properties: [:]))
            try png.write(to: directory.appending(path: "menu-editor-\(width).png"))
            XCTAssertGreaterThan(png.count, 15000)
            XCTAssertEqual(Double(bitmap.pixelsWide) / Double(bitmap.pixelsHigh), Double(width) / Double(height), accuracy: 0.001)
        }
        XCTAssertEqual(store.desktopDocument, before)
    }
    // ------------------------=
    // FUNC: testAuthoredDestinationPixels
    // DESC: Verifies native rendering consumes a newly authored destination and responds to actual style edits.
    // ------------------=
    @MainActor func testAuthoredDestinationPixels() async throws {
        let store = TemplateStore()
        store.desktopDocument = .factoryDesktop()
        let item = try XCTUnwrap(store.addMenuItem(action: .system, after: nil))
        let screen = try XCTUnwrap(store.createMenuDestination(for: item, title: "Project Workspace", window: true))
        store.selectScreenCollection(.desktop, screen: screen)
        let runner = NativePreviewRunner(timeout: .seconds(60))
        let frame = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        let image = try XCTUnwrap(frame.image())
        let png = try XCTUnwrap(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
        let directory = try XCTUnwrap(store.projectRoot).appending(path: "builds/validation/studio-menu")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try png.write(to: directory.appending(path: "destination.png"))
        store.desktopDocument.screens[screen - 1].elements[0].opacity = 20
        let changed = try await runner.render(NativePreviewSnapshot.capture(store, width: 1280, height: 800))
        XCTAssertNotEqual(frame.pixels, changed.pixels)
    }
    // ------------------------=
    // FUNC: testSaveAndReopenDesktopMenu
    // DESC: Exercises the real Save Section path and reopens its editable source and installed-runtime payload.
    // ------------------=
    @MainActor func testSaveAndReopenDesktopMenu() throws {
        let store = TemplateStore()
        let repository = try XCTUnwrap(store.projectRoot)
        let root = repository.appending(path: "builds/validation/studio-menu/save-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: root) }
        store.desktopDocument = .factoryDesktop()
        for path in Set(store.desktopDocument.screens.flatMap(\.elements).filter { $0.kind == .image }.map(\.imageAsset)) {
            let destination = root.appending(path: path)
            try FileManager.default.createDirectory(at: destination.deletingLastPathComponent(), withIntermediateDirectories: true)
            try FileManager.default.copyItem(at: repository.appending(path: path), to: destination)
        }
        store.projectRoot = root
        store.editInfinityMenu()
        let id = try XCTUnwrap(store.addMenuItem(action: .system, after: nil))
        XCTAssertEqual(store.createMenuDestination(for: id, title: "Saved Workspace", window: false), 4)
        store.save()
        XCTAssertTrue(store.validationIssues.isEmpty)
        let source = try Data(contentsOf: root.appending(path: "assets/boot/desktop-screens.infinityui"))
        XCTAssertEqual(try JSONDecoder().decode(InstallerStudioDocument.self, from: source), store.desktopDocument)
        let runtime = try Data(contentsOf: root.appending(path: "assets/boot/desktop-screens.iuit"))
        XCTAssertEqual(try RuntimeTemplateCodec.decode(runtime, collection: .desktop).infinityMenu, store.menu)
        XCTAssertEqual(TemplateStore(projectRoot: root).desktopDocument, store.desktopDocument)
    }
    // ------------------------=
    // FUNC: testHierarchyOperations
    // DESC: Exercises insertion, subtree duplication, sibling moves, indent/outdent and deletion by stable identity.
    // ------------------=
    func testHierarchyOperations() throws {
        let parent = StudioMenuItem(label: "Workspace", action: .submenu)
        let child = StudioMenuItem(label: "Editor", action: .application, target: 1)
        var menu = StudioMenu(items: [parent, child])
        menu.indent(child.id)
        XCTAssertEqual(menu.items.count, 1)
        XCTAssertEqual(menu.items[0].children[0].id, child.id)
        menu.outdent(child.id)
        XCTAssertEqual(menu.items.map(\.id), [parent.id, child.id])
        menu.move(child.id, delta: -1)
        XCTAssertEqual(menu.items.map(\.id), [child.id, parent.id])
        XCTAssertTrue(menu.insert(StudioMenu.duplicate(parent), after: parent.id))
        XCTAssertEqual(Set(menu.rows.map(\.id)).count, 3)
        menu.remove(child.id)
        XCTAssertEqual(menu.rows.count, 2)
        try menu.validate(screens: InstallerStudioDocument.factoryDesktop().screens)
    }
    // ------------------------=
    // FUNC: testCreateLinkUndoAndPersistence
    // DESC: Verifies atomic screen/window creation, stable links, document undo/redo and source/runtime round trips.
    // ------------------=
    @MainActor func testCreateLinkUndoAndPersistence() throws {
        let store = TemplateStore()
        store.desktopDocument = .factoryDesktop()
        store.editInfinityMenu()
        XCTAssertTrue(store.editingMenu)
        let id = try XCTUnwrap(store.addMenuItem(action: .system, after: nil))
        let before = store.desktopDocument
        let screen = try XCTUnwrap(store.createMenuDestination(for: id, title: "Project Workspace", window: true))
        XCTAssertEqual(screen, 4)
        XCTAssertEqual(store.menu.rows.last?.item.action, .window)
        XCTAssertEqual(store.menu.rows.last?.item.target, screen)
        let after = store.desktopDocument
        store.undo(); XCTAssertEqual(store.desktopDocument, before)
        store.redo(); XCTAssertEqual(store.desktopDocument, after)
        let source = try JSONEncoder().encode(after)
        XCTAssertEqual(try JSONDecoder().decode(InstallerStudioDocument.self, from: source), after)
        let binary = try RuntimeTemplateCodec.encode(after, collection: .desktop)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(binary, collection: .desktop), after)
        XCTAssertEqual(Array(binary[4...5]), [13, 0])
        let root = try XCTUnwrap(store.projectRoot)
        let directory = root.appending(path: "builds/validation/studio-menu")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try binary.write(to: directory.appending(path: "navigation-fixture.iuit"))
        store.openMenuDestination(try XCTUnwrap(store.menu.rows.last?.item))
        XCTAssertFalse(store.editingMenu)
        XCTAssertEqual(store.selectedScreenID, screen)
        store.undo()
        XCTAssertEqual(store.selectedScreenID, 1)
        XCTAssertNotNil(store.selectedScreenIndex)
        store.redo()
        store.selectScreenCollection(.desktop, screen: screen)
        store.removeScreen()
        XCTAssertEqual(store.desktopDocument.screens.count, 4)
        store.mutateMenu("Remove link") { $0.remove(id) }
        store.removeScreen()
        XCTAssertEqual(store.desktopDocument.screens.count, 3)
    }
    // ------------------------=
    // FUNC: testValidationAndLocking
    // DESC: Rejects broken links, forbidden actions, excessive nesting, duplicate IDs and menu edits under a locked layer.
    // ------------------=
    @MainActor func testValidationAndLocking() throws {
        let store = TemplateStore()
        store.desktopDocument = .factoryDesktop()
        var menu = StudioMenu(items: [StudioMenuItem(label: "Bad link", action: .screen, target: 31)])
        XCTAssertThrowsError(try menu.validate(screens: store.desktopDocument.screens))
        menu.items[0].action = .system; menu.items[0].target = 255
        XCTAssertThrowsError(try menu.validate(screens: store.desktopDocument.screens))
        menu = .factory; menu.items.append(menu.items[0])
        XCTAssertThrowsError(try menu.validate(screens: store.desktopDocument.screens))
        var deep = StudioMenuItem(label: "Nested", action: .submenu)
        for _ in 0..<4 { deep = StudioMenuItem(label: "Nested", action: .submenu, children: [deep]) }
        XCTAssertThrowsError(try StudioMenu(items: [deep]).validate(screens: store.desktopDocument.screens))
        let index = try XCTUnwrap(store.desktopDocument.screens[0].elements.firstIndex { $0.role == .desktopBrand })
        store.desktopDocument.screens[0].elements[index].locked = true
        let before = store.desktopDocument
        XCTAssertNil(store.addMenuItem(action: .system, after: nil))
        XCTAssertEqual(store.desktopDocument, before)
    }
    // ------------------------=
    // FUNC: testNestedBinaryAndTruncation
    // DESC: Verifies hierarchical binary decoding, disabled/check states and fail-closed truncated extensions.
    // ------------------=
    func testNestedBinaryAndTruncation() throws {
        var document = InstallerStudioDocument.factoryDesktop()
        document.infinityMenu = StudioMenu(items: [StudioMenuItem(label: "Tools", action: .submenu, children: [
            StudioMenuItem(label: "Editor", action: .application, target: 1, enabled: false, checked: true),
            StudioMenuItem(label: "", action: .separator), StudioMenuItem(label: "Network", action: .settings, target: 7)
        ])])
        let binary = try RuntimeTemplateCodec.encode(document, collection: .desktop)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(binary, collection: .desktop), document)
        for drop in 1...64 { XCTAssertThrowsError(try RuntimeTemplateCodec.decode(binary.dropLast(drop), collection: .desktop)) }
        var invalid = binary; invalid.append(0)
        XCTAssertThrowsError(try RuntimeTemplateCodec.decode(invalid, collection: .desktop))
    }
    // ------------------------=
    // FUNC: testRemovingEarlierDestinationRemapsLinks
    // DESC: Preserves menu links when an unreferenced preceding custom screen is deleted.
    // ------------------=
    @MainActor func testRemovingEarlierDestinationRemapsLinks() throws {
        let store = TemplateStore(); store.desktopDocument = .factoryDesktop()
        let first = try XCTUnwrap(store.addMenuItem(action: .system, after: nil))
        XCTAssertEqual(store.createMenuDestination(for: first, title: "First", window: false), 4)
        let second = try XCTUnwrap(store.addMenuItem(action: .system, after: first))
        XCTAssertEqual(store.createMenuDestination(for: second, title: "Second", window: true), 5)
        store.mutateMenu("Remove first") { $0.remove(first) }
        store.selectScreenCollection(.desktop, screen: 4); store.removeScreen()
        XCTAssertEqual(store.menu.rows.first { $0.id == second }?.item.target, 4)
        XCTAssertEqual(store.desktopDocument.screens[3].title, "Second")
        try TemplateValidator.validate(store.desktopDocument, for: .desktop)
    }
}
