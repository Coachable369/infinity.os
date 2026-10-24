import XCTest
@testable import InfinityInstallerStudio

final class LayerOrderTests: XCTestCase {
    // ------------------------=
    // FUNC: testStackActionsUndoAndPersistence
    // DESC: Exercises all four commands on tied layers, including nonselected targets and saved ordering.
    // ------------------=
    @MainActor func testStackActionsUndoAndPersistence() throws {
        let store = TemplateStore()
        store.configurationDocument = .factoryConfiguration()
        store.selectScreenCollection(.configuration, screen: store.configurationDocument.screens[0].id)
        let layers = (0..<4).map { index in
            StudioElement.make(name: "Layer \(index)", kind: .panel,
                frame: CanvasRect(x: 0, y: 0, width: 100, height: 100))
        }.sorted { $0.id.uuidString < $1.id.uuidString }
        store.configurationDocument.screens[0].elements = layers
        let expectations: [(LayerOrderAction, [Int])] = [
            (.front, [0, 2, 3, 1]), (.back, [1, 0, 2, 3]),
            (.forward, [0, 2, 1, 3]), (.backward, [1, 0, 2, 3])
        ]
        for (action, indices) in expectations {
            store.selectElement(layers[3].id)
            store.reorderLayer(layers[1].id, action: action)
            let result = try XCTUnwrap(store.selectedScreen).elements.sorted { $0.zIndex < $1.zIndex }
            XCTAssertEqual(result.map(\.id), indices.map { layers[$0].id })
            XCTAssertEqual(store.selectedElementID, layers[1].id)
            let encoded = try JSONEncoder().encode(store.configurationDocument)
            let decoded = try JSONDecoder().decode(InstallerStudioDocument.self, from: encoded)
            XCTAssertEqual(decoded, store.configurationDocument)
            store.undo()
            XCTAssertEqual(store.selectedScreen?.elements, layers)
            store.redo()
            XCTAssertEqual(store.selectedScreen?.elements.sorted { $0.zIndex < $1.zIndex }, result)
            store.undo()
        }
        XCTAssertFalse(store.canReorderLayer(layers[3].id, action: .front))
        XCTAssertFalse(store.canReorderLayer(layers[0].id, action: .backward))
        for index in layers.indices {
            store.configurationDocument.screens[0].elements[index].zIndex = [-32_768, 0, 500, 32_767][index]
        }
        store.reorderLayer(layers[1].id, action: .forward)
        XCTAssertEqual(store.selectedScreen?.elements.sorted { $0.zIndex < $1.zIndex }.map(\.id),
            [layers[0].id, layers[2].id, layers[1].id, layers[3].id])
        store.undo()
        store.configurationDocument.screens[0].elements[1].locked = true
        let before = store.configurationDocument
        store.reorderLayer(layers[1].id, action: .front)
        XCTAssertEqual(store.configurationDocument, before)
        store.selectScreenCollection(.systemSettings, screen: 1)
        for element in try XCTUnwrap(store.selectedScreen).elements {
            XCTAssertFalse(store.canReorderLayer(element.id, action: .front))
        }
    }
}
