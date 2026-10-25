import AppKit
import XCTest
@testable import InfinityInstallerStudio

final class DecorativeClipboardTests: XCTestCase {
    // ------------------------=
    // FUNC: testPasteMastheadPreservesArtworkAndRuntimeIdentity
    // DESC: Pastes a masthead over an existing masthead as editable artwork, with lossless undo and export.
    // ------------------=
    @MainActor func testPasteMastheadPreservesArtworkAndRuntimeIdentity() throws {
        let store = TemplateStore()
        store.document = .factoryDefault()
        store.selectScreen(1)
        let original = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .masthead })
        store.selectElement(original.id)
        let board = NSPasteboard.withUniqueName()
        defer { board.releaseGlobally() }
        store.copySelectedElements(to: board)
        store.selectScreen(2)
        let before = try XCTUnwrap(store.selectedScreen)
        store.pasteElementsInPlace(from: board)
        let after = try XCTUnwrap(store.selectedScreen)
        XCTAssertEqual(after.elements.count, before.elements.count + 1)
        XCTAssertEqual(after.elements.filter { $0.role == .masthead }, before.elements.filter { $0.role == .masthead })
        let copy = try XCTUnwrap(after.elements.last)
        XCTAssertEqual(copy.role, .image)
        XCTAssertEqual(copy.imageAsset, original.imageAsset)
        XCTAssertEqual(copy.frame, original.frame)
        XCTAssertEqual(copy.opacity, original.opacity)
        XCTAssertNotEqual(copy.id, original.id)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(RuntimeTemplateCodec.encode(store.document)), store.document)
        store.undo()
        XCTAssertEqual(store.selectedScreen, before)
        store.redo()
        XCTAssertEqual(store.selectedScreen, after)
        store.pasteElementsInPlace(from: board)
        let names = try XCTUnwrap(store.selectedScreen).elements.map(\.name)
        XCTAssertEqual(Set(names).count, names.count)
        XCTAssertEqual(store.selectedScreen?.elements.last?.name, "\(original.name) 3")
    }

    // ------------------------=
    // FUNC: testFunctionalRoleConflictDoesNotPartiallyPaste
    // DESC: Ensures decorative conversion cannot bypass unique functional controls or partially mutate a screen.
    // ------------------=
    @MainActor func testFunctionalRoleConflictDoesNotPartiallyPaste() throws {
        let store = TemplateStore()
        store.document = .factoryDefault()
        store.selectScreen(1)
        let screen = try XCTUnwrap(store.selectedScreen)
        let masthead = try XCTUnwrap(screen.elements.first { $0.role == .masthead })
        let button = try XCTUnwrap(screen.elements.first { $0.role == .primaryButton })
        let board = NSPasteboard.withUniqueName()
        defer { board.releaseGlobally() }
        board.setData(try JSONEncoder().encode([masthead, button]), forType: TemplateStore.elementPasteboardType)
        store.pasteElementsInPlace(from: board)
        XCTAssertEqual(store.selectedScreen, screen)
    }
}
