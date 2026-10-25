import XCTest
@testable import InfinityInstallerStudio

final class LayerDragTests: XCTestCase {
    // ------------------------=
    // FUNC: testLayerDragReordersPersistsAndUndoes
    // DESC: Exercises list drag ordering in both directions, filtered rows, undo, export and locked-layer refusal.
    // ------------------=
    @MainActor func testLayerDragReordersPersistsAndUndoes() throws {
        let store = TemplateStore()
        store.document = .factoryDefault()
        store.selectScreen(1)
        let original = try XCTUnwrap(store.selectedScreen)
        let rows = original.elements.sorted {
            $0.zIndex == $1.zIndex ? $0.id.uuidString > $1.id.uuidString : $0.zIndex > $1.zIndex
        }.filter { !$0.locked }.map(\.id)
        XCTAssertGreaterThan(rows.count, 2)
        store.moveVisibleLayers(IndexSet(integer: 0), to: rows.count, visible: rows)
        let changed = try XCTUnwrap(store.selectedScreen)
        XCTAssertEqual(changed.elements.sorted { $0.zIndex > $1.zIndex }.filter { rows.contains($0.id) }.map(\.id), Array(rows.dropFirst()) + [rows[0]])
        XCTAssertEqual(try RuntimeTemplateCodec.decode(RuntimeTemplateCodec.encode(store.document)), store.document)
        store.undo()
        XCTAssertEqual(store.selectedScreen, original)
        store.redo()
        XCTAssertEqual(store.selectedScreen, changed)
        let reordered = Array(rows.dropFirst()) + [rows[0]]
        store.moveVisibleLayers(IndexSet(integer: reordered.count - 1), to: 0, visible: reordered)
        XCTAssertEqual(store.selectedScreen?.elements.sorted { $0.zIndex > $1.zIndex }.filter { rows.contains($0.id) }.map(\.id), rows)
        store.toggleElementLock(rows[0])
        let locked = store.selectedScreen
        store.moveVisibleLayers(IndexSet(integer: 0), to: rows.count, visible: rows)
        XCTAssertEqual(store.selectedScreen, locked)
    }
}
