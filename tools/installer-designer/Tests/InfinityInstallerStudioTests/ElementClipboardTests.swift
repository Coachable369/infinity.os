import AppKit
import XCTest
@testable import InfinityInstallerStudio

final class ElementClipboardTests: XCTestCase {
    // ------------------------=
    // FUNC: testMarqueeClipboardPreservesGeometryAndUndoAcrossScreens
    // DESC: Copies multiple actual selected layers through a pasteboard and verifies exact placement and atomic undo.
    // ------------------=
    @MainActor func testMarqueeClipboardPreservesGeometryAndUndoAcrossScreens() throws {
        let store = TemplateStore()
        store.configurationDocument = .factoryConfiguration()
        store.selectScreenCollection(.configuration, screen: store.configurationDocument.screens[0].id)
        let first = StudioElement.make(name: "Copied panel", kind: .panel,
            frame: CanvasRect(x: 123, y: 234, width: 200, height: 100))
        var second = StudioElement.make(name: "Copied button", kind: .button,
            frame: CanvasRect(x: 337, y: 351, width: 160, height: 48))
        second.surfaceMaterial = .flat
        store.configurationDocument.screens[0].elements = [first, second]
        store.selectElements(in: CanvasRect(x: 100, y: 200, width: 450, height: 250))
        XCTAssertEqual(store.selectedElementIDs, Set([first.id, second.id]))
        let clipboard = NSPasteboard.withUniqueName()
        defer { clipboard.releaseGlobally() }
        store.copySelectedElements(to: clipboard)
        store.selectScreenCollection(.configuration, screen: store.configurationDocument.screens[1].id)
        let before = try XCTUnwrap(store.selectedScreen)
        store.pasteElementsInPlace(from: clipboard)
        let pasted = Array(try XCTUnwrap(store.selectedScreen).elements.suffix(2))
        XCTAssertEqual(pasted.map(\.frame), [first.frame, second.frame])
        XCTAssertEqual(pasted.map(\.surfaceMaterial), [first.surfaceMaterial, .flat])
        XCTAssertNotEqual(pasted[0].id, first.id)
        XCTAssertNotEqual(pasted[1].id, second.id)
        XCTAssertEqual(store.selectedElementIDs, Set(pasted.map(\.id)))
        store.undo()
        XCTAssertEqual(store.selectedScreen, before)
        store.redo()
        XCTAssertEqual(Array(try XCTUnwrap(store.selectedScreen).elements.suffix(2)), pasted)
        store.pasteElementsInPlace(from: clipboard)
        let all = try XCTUnwrap(store.selectedScreen).elements
        XCTAssertEqual(Set(all.map(\.id)).count, all.count)
        XCTAssertEqual(Set(all.map(\.codeIdentifier)).count, all.count)
    }
}
