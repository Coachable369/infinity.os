import AppKit
import XCTest
@testable import InfinityInstallerStudio

final class ContextMenuTests: XCTestCase {
    // ------------------------=
    // FUNC: testContextCopyPreservesSelectionAndPasteEligibility
    // DESC: Verifies member targeting preserves a multiple selection and copies both layers to the native clipboard.
    // ------------------=
    @MainActor func testContextCopyPreservesSelectionAndPasteEligibility() throws {
        let store = TemplateStore()
        store.configurationDocument = .factoryConfiguration()
        store.selectScreenCollection(.configuration, screen: store.configurationDocument.screens[0].id)
        let layers = (0..<2).map { index in
            StudioElement.make(name: "Panel \(index)", kind: .panel,
                frame: CanvasRect(x: 100 + index * 200, y: 100, width: 100, height: 100))
        }
        store.configurationDocument.screens[0].elements = layers
        store.selectElements(in: CanvasRect(x: 0, y: 0, width: 500, height: 500))
        store.prepareContextSelection(layers[0].id)
        XCTAssertEqual(store.selectedElementIDs, Set(layers.map(\.id)))
        let clipboard = NSPasteboard.withUniqueName()
        defer { clipboard.releaseGlobally() }
        XCTAssertFalse(store.canPasteElements(from: clipboard))
        store.copySelectedElements(to: clipboard)
        XCTAssertTrue(store.canPasteElements(from: clipboard))
        store.pasteElementsInPlace(from: clipboard)
        XCTAssertEqual(store.selectedScreen?.elements.count, 4)
        store.undo()
        XCTAssertEqual(store.selectedScreen?.elements, layers)
        store.selectElement(layers[1].id)
        store.prepareContextSelection(layers[0].id)
        XCTAssertEqual(store.selectedElementIDs, [layers[0].id])
    }
}
