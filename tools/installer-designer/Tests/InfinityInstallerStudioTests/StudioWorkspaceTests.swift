import XCTest
@testable import InfinityInstallerStudio

final class StudioWorkspaceTests: XCTestCase {
    // ------------------------=
    // FUNC: testSwitchingWorkspacesPreservesIndependentPendingEdits
    // DESC: Exercises real section selection and scoped undo without losing wizard or UIKit changes.
    // ------------------=
    @MainActor func testSwitchingWorkspacesPreservesIndependentPendingEdits() throws {
        let store = TemplateStore()
        store.configurationDocument = .factoryConfiguration()
        let original = store.configurationDocument
        store.selectScreenCollection(.configuration, screen: 2)
        let element = try XCTUnwrap(store.selectedScreen?.elements.first { !$0.locked && $0.kind == .text })
        store.selectElement(element.id)
        store.updateSelected("Edited configuration title") { $0.text = "My configuration" }
        let edited = store.configurationDocument
        let originalColor = store.uiKit.document.colors[UIKitColorRole.accent.rawValue]
        store.selectUIKit(category: "Colors & Theme")
        store.uiKit.setColor(.accent, argb: originalColor ^ 0x0000ff00)
        XCTAssertTrue(store.editingUIKit)
        XCTAssertTrue(store.canUndo)
        store.undo()
        XCTAssertEqual(store.uiKit.document.colors[UIKitColorRole.accent.rawValue], originalColor)
        XCTAssertEqual(store.configurationDocument, edited)
        store.redo()
        XCTAssertNotEqual(store.uiKit.document.colors[UIKitColorRole.accent.rawValue], originalColor)
        store.selectScreenCollection(.configuration, screen: 2)
        XCTAssertFalse(store.editingUIKit)
        XCTAssertEqual(store.configurationDocument, edited)
        store.undo()
        XCTAssertEqual(store.configurationDocument, original)
        XCTAssertNotEqual(store.uiKit.document.colors[UIKitColorRole.accent.rawValue], originalColor)
    }

    // ------------------------=
    // FUNC: testAllRequestedWizardCollectionsRemainAvailable
    // DESC: Verifies UIKit navigation leaves each existing runtime screen collection and selection functional.
    // ------------------=
    @MainActor func testAllRequestedWizardCollectionsRemainAvailable() {
        let store = TemplateStore()
        for collection in ScreenCollection.allCases {
            store.selectUIKit()
            store.selectScreenCollection(collection, screen: 1)
            XCTAssertFalse(store.editingUIKit)
            XCTAssertEqual(store.selectedCollection, collection)
            XCTAssertNotNil(store.selectedScreen)
            XCTAssertGreaterThanOrEqual(store.activeScreens.count, collection.requiredScreenCount)
        }
    }
}
