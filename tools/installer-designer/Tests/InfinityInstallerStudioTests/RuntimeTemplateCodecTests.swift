import XCTest
@testable import InfinityInstallerStudio

final class RuntimeTemplateCodecTests: XCTestCase {
    // ------------------------=
    // FUNC: testFactoryDocumentCoversEveryInstallerScreen
    // DESC: Verifies all runtime screens and their protected navigation controls exist.
    // ------------------=
    func testFactoryDocumentCoversEveryInstallerScreen() throws {
        let document = InstallerStudioDocument.factoryDefault()

        XCTAssertEqual(document.screens.map(\.id), Array(1...11))
        XCTAssertNoThrow(try TemplateValidator.validate(document))
        for screen in document.screens {
            XCTAssertEqual(screen.elements.filter { $0.role == .backButton && $0.locked }.count, 1)
            XCTAssertEqual(screen.elements.filter { $0.role == .primaryButton && $0.locked }.count, 1)
        }
    }

    // ------------------------=
    // FUNC: testRuntimeCodecRoundTripsEditableState
    // DESC: Exercises binary persistence and proves geometry, text, styling, and locks survive decoding.
    // ------------------=
    func testRuntimeCodecRoundTripsEditableState() throws {
        var document = InstallerStudioDocument.factoryDefault()
        document.screens[2].elements[4].frame = CanvasRect(x: 115, y: 450, width: 470, height: 96)
        document.screens[2].elements[4].text = "A disk joins your protected Infinity Pool."
        document.screens[2].elements[4].opacity = 84

        let decoded = try RuntimeTemplateCodec.decode(RuntimeTemplateCodec.encode(document))

        XCTAssertEqual(decoded, document)
    }

    // ------------------------=
    // FUNC: testNavigationButtonsCannotBeChangedInSavedTemplates
    // DESC: Proves validation rejects edits to protected navigation geometry and state.
    // ------------------=
    func testNavigationButtonsCannotBeChangedInSavedTemplates() {
        var document = InstallerStudioDocument.factoryDefault()
        let button = document.screens[0].elements.firstIndex { $0.role == .primaryButton }!
        document.screens[0].elements[button].frame.x += 10
        document.screens[0].elements[button].locked = false

        XCTAssertThrowsError(try RuntimeTemplateCodec.encode(document))
    }

    // ------------------------=
    // FUNC: testMalformedRuntimeDataIsRejected
    // DESC: Proves truncated persisted data cannot become an active installer layout.
    // ------------------=
    func testMalformedRuntimeDataIsRejected() throws {
        let encoded = try RuntimeTemplateCodec.encode(.factoryDefault())

        XCTAssertThrowsError(try RuntimeTemplateCodec.decode(encoded.dropLast(9)))
    }

    // ------------------------=
    // FUNC: testCanvasMoveAndResizeSnapToConfiguredGrid
    // DESC: Exercises observable editor movement and eight-handle resizing against the snapping contract.
    // ------------------=
    @MainActor
    func testCanvasMoveAndResizeSnapToConfiguredGrid() {
        let store = TemplateStore()
        let body = store.selectedScreen!.elements.first { $0.role == .body }!
        store.selectElement(body.id)
        store.beginGesture()
        store.moveSelected(translation: CGSize(width: 19, height: 16), canvasScale: CGSize(width: 1, height: 1))
        store.endGesture()
        XCTAssertEqual(store.selectedElement!.frame.x, 90)
        XCTAssertEqual(store.selectedElement!.frame.y, 450)

        let moved = store.selectedElement!.frame
        store.beginGesture()
        store.resizeSelected(
            handle: .bottomRight,
            translation: CGSize(width: 27, height: 33),
            canvasScale: CGSize(width: 1, height: 1)
        )
        store.endGesture()
        XCTAssertEqual(store.selectedElement!.frame.width, moved.width + 30)
        XCTAssertEqual(store.selectedElement!.frame.height, moved.height + 30)
    }

    // ------------------------=
    // FUNC: testLockedButtonIgnoresEditorMutations
    // DESC: Proves button selection remains inspectable while every editor mutation is refused.
    // ------------------=
    @MainActor
    func testLockedButtonIgnoresEditorMutations() {
        let store = TemplateStore()
        let primary = store.selectedScreen!.elements.first { $0.role == .primaryButton }!
        store.selectElement(primary.id)

        store.updateSelected { $0.text = "CHANGED" }
        store.beginGesture()
        store.moveSelected(translation: CGSize(width: 100, height: 100), canvasScale: CGSize(width: 1, height: 1))
        store.endGesture()

        XCTAssertEqual(store.selectedElement, primary)
    }

    // ------------------------=
    // FUNC: testSaveWritesEditableAndRuntimeTemplatesAtomically
    // DESC: Exercises the editor save path and decodes the resulting runtime artifact.
    // ------------------=
    @MainActor
    func testSaveWritesEditableAndRuntimeTemplatesAtomically() throws {
        let root = FileManager.default.temporaryDirectory
            .appending(path: "infinity-installer-studio-\(UUID().uuidString)", directoryHint: .isDirectory)
        defer { try? FileManager.default.removeItem(at: root) }
        let store = TemplateStore()
        store.projectRoot = root

        store.save()

        let editableURL = root.appending(path: "assets/boot/installer-screens.infinityui")
        let runtimeURL = root.appending(path: "assets/boot/installer-screens.iuit")
        XCTAssertTrue(FileManager.default.fileExists(atPath: editableURL.path))
        let runtime = try Data(contentsOf: runtimeURL)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(runtime), store.document)
        XCTAssertEqual(store.status, "Saved editable and runtime templates")
    }
}
