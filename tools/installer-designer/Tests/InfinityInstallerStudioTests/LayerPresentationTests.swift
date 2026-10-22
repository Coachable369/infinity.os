import XCTest
@testable import InfinityInstallerStudio

final class LayerPresentationTests: XCTestCase {
    // ------------------------=
    // FUNC: testSceneLayersRemainAccessibleWhenLockedOrHidden
    // DESC: Verifies scene grouping preserves every layer and permits selecting locked artwork.
    // ------------------=
    @MainActor func testSceneLayersRemainAccessibleWhenLockedOrHidden() throws {
        let store = TemplateStore()
        store.configurationDocument = .factoryConfiguration()
        store.selectScreenCollection(.configuration, screen: store.configurationDocument.screens[0].id)
        let elements = try XCTUnwrap(store.selectedScreen).elements
        let scene = ElementInspector.listedElements(elements, scene: true, search: "")
        let content = ElementInspector.listedElements(elements, scene: false, search: "")
        XCTAssertEqual(Set((scene + content).map(\.id)), Set(elements.map(\.id)))
        XCTAssertEqual(scene.count + content.count, elements.count)
        let artwork = try XCTUnwrap(scene.first { $0.kind == .image })
        store.selectElement(artwork.id)
        XCTAssertEqual(store.selectedElement?.id, artwork.id)
        let locked = try XCTUnwrap(store.selectedElement).locked
        store.toggleElementLock(artwork.id)
        XCTAssertEqual(store.selectedElement?.locked, !locked)
        var hidden = artwork
        hidden.hidden = true
        hidden.locked = true
        XCTAssertEqual(ElementInspector.listedElements([hidden], scene: true, search: "").map(\.id), [hidden.id])
        XCTAssertEqual(ElementInspector.listedElements([hidden], scene: true, search: hidden.imageAsset).map(\.id), [hidden.id])
    }
}
