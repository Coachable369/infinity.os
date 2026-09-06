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
    // FUNC: testNavigationActionsSurviveUnlockedAuthoredAppearance
    // DESC: Proves buttons retain required action roles while lock state and geometry persist.
    // ------------------=
    func testNavigationActionsSurviveUnlockedAuthoredAppearance() throws {
        var document = InstallerStudioDocument.factoryDefault()
        let button = document.screens[0].elements.firstIndex { $0.role == .primaryButton }!
        document.screens[0].elements[button].frame.x += 10
        document.screens[0].elements[button].locked = false

        let decoded = try RuntimeTemplateCodec.decode(RuntimeTemplateCodec.encode(document))

        XCTAssertEqual(decoded.screens[0].elements[button].role, .primaryButton)
        XCTAssertFalse(decoded.screens[0].elements[button].locked)
        XCTAssertEqual(decoded.screens[0].elements[button].frame.x, 520)

        document.screens[0].elements[button].role = .decoration
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
        store.beginGesture(elementID: body.id)
        store.moveSelected(translation: CGSize(width: 19, height: 16), canvasScale: CGSize(width: 1, height: 1))
        store.endGesture()
        XCTAssertEqual(store.selectedElementID, body.id)
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
    // FUNC: testEveryResizeHandleMutatesItsOwnedEdges
    // DESC: Exercises all eight canvas handles and proves each changes only its corresponding edges.
    // ------------------=
    @MainActor
    func testEveryResizeHandleMutatesItsOwnedEdges() {
        let translation = CGSize(width: 20, height: 20)
        for handle in ResizeHandle.allCases {
            let store = TemplateStore()
            store.snapEnabled = false
            let body = store.selectedScreen!.elements.first { $0.role == .body }!
            store.beginGesture(elementID: body.id)
            let origin = store.selectedElement!.frame
            store.resizeSelected(handle: handle, translation: translation, canvasScale: CGSize(width: 1, height: 1))
            store.endGesture()
            let resized = store.selectedElement!.frame

            let ownsLeft = [ResizeHandle.topLeft, .left, .bottomLeft].contains(handle)
            let ownsRight = [ResizeHandle.topRight, .right, .bottomRight].contains(handle)
            let ownsTop = [ResizeHandle.topLeft, .top, .topRight].contains(handle)
            let ownsBottom = [ResizeHandle.bottomLeft, .bottom, .bottomRight].contains(handle)
            XCTAssertEqual(resized.x, origin.x + (ownsLeft ? 20 : 0), "\(handle) left edge")
            XCTAssertEqual(resized.y, origin.y + (ownsTop ? 20 : 0), "\(handle) top edge")
            XCTAssertEqual(resized.width, origin.width + (ownsRight ? 20 : 0) - (ownsLeft ? 20 : 0), "\(handle) width")
            XCTAssertEqual(resized.height, origin.height + (ownsBottom ? 20 : 0) - (ownsTop ? 20 : 0), "\(handle) height")
        }
    }

    // ------------------------=
    // FUNC: testResizeHandleTargetsRemainEasyToGrab
    // DESC: Verifies resize handles expose a larger interactive target than their visual marker.
    // ------------------=
    func testResizeHandleTargetsRemainEasyToGrab() {
        XCTAssertGreaterThanOrEqual(CanvasInteractionMetrics.resizeHandleHitSize, 28)
        XCTAssertGreaterThan(CanvasInteractionMetrics.resizeHandleHitSize, CanvasInteractionMetrics.resizeHandleVisualSize)
    }

    // ------------------------=
    // FUNC: testCanvasActivationSelectsObjectsAndHonorsLocks
    // DESC: Exercises direct canvas selection with inline editing for unlocked objects and inspection-only locks.
    // ------------------=
    @MainActor
    func testCanvasActivationSelectsObjectsAndHonorsLocks() {
        let store = TemplateStore()
        let body = store.selectedScreen!.elements.first { $0.role == .body }!
        store.activateCanvasElement(body.id)
        XCTAssertEqual(store.selectedElementID, body.id)
        XCTAssertEqual(store.inlineEditorElementID, body.id)

        let primary = store.selectedScreen!.elements.first { $0.role == .primaryButton }!
        store.activateCanvasElement(primary.id)
        XCTAssertEqual(store.selectedElementID, primary.id)
        XCTAssertNil(store.inlineEditorElementID)
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
    // FUNC: testEveryElementCanToggleLockAndInlineEditingRequiresUnlock
    // DESC: Exercises persisted lock control and the canvas inline-editor gate for every element kind.
    // ------------------=
    @MainActor
    func testEveryElementCanToggleLockAndInlineEditingRequiresUnlock() {
        let store = TemplateStore()
        let elements = store.selectedScreen!.elements

        for element in elements {
            store.selectElement(element.id)
            if !store.selectedElement!.locked {
                store.toggleElementLock(element.id)
            }
            store.presentInlineEditor(for: element.id)
            XCTAssertNil(store.inlineEditorElementID)

            store.toggleElementLock(element.id)
            store.presentInlineEditor(for: element.id)
            XCTAssertEqual(store.inlineEditorElementID, element.id)
            store.updateSelected("Inline edit") { $0.name += " Edited" }
            XCTAssertTrue(store.selectedElement!.name.hasSuffix(" Edited"))
            store.dismissInlineEditor()
        }
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
        let masthead = store.selectedScreen!.elements.first { $0.role == .masthead }!
        store.toggleElementLock(masthead.id)
        store.presentInlineEditor(for: masthead.id)
        store.updateSelected("Inline edit") { $0.name = "Editable Masthead" }

        store.save()

        let editableURL = root.appending(path: "assets/boot/installer-screens.infinityui")
        let runtimeURL = root.appending(path: "assets/boot/installer-screens.iuit")
        XCTAssertTrue(FileManager.default.fileExists(atPath: editableURL.path))
        let editable = try JSONDecoder().decode(
            InstallerStudioDocument.self,
            from: Data(contentsOf: editableURL)
        )
        let runtime = try Data(contentsOf: runtimeURL)
        XCTAssertEqual(editable, store.document)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(runtime), store.document)
        XCTAssertEqual(store.status, "Saved editable and runtime templates")
    }

    // ------------------------=
    // FUNC: testScreenLifecyclePreservesOrderAndProtectedControls
    // DESC: Exercises add, duplicate, reorder, rename, and remove behavior through editor state.
    // ------------------=
    @MainActor
    func testScreenLifecyclePreservesOrderAndProtectedControls() throws {
        let store = TemplateStore()
        store.selectScreen(3)
        store.addScreen()
        XCTAssertEqual(store.document.screens.count, 12)
        XCTAssertEqual(store.selectedScreenID, 4)
        store.renameSelectedScreen("Storage Choices")
        store.duplicateScreen()
        XCTAssertEqual(store.document.screens.count, 13)
        XCTAssertEqual(store.selectedScreen?.title, "Storage Choices Copy")
        store.moveScreen(-1)
        XCTAssertEqual(store.selectedScreenID, 4)
        store.removeScreen()

        XCTAssertEqual(store.document.screens.map(\.id), Array(1...12))
        for screen in store.document.screens {
            XCTAssertEqual(screen.elements.filter { $0.role == .backButton && $0.locked }.count, 1)
            XCTAssertEqual(screen.elements.filter { $0.role == .primaryButton && $0.locked }.count, 1)
        }
        XCTAssertNoThrow(try TemplateValidator.validate(store.document))
    }

    // ------------------------=
    // FUNC: testVariableScreenCountSurvivesRuntimePersistence
    // DESC: Proves an authored screen addition is retained by the exact binary format consumed by the installer.
    // ------------------=
    @MainActor
    func testVariableScreenCountSurvivesRuntimePersistence() throws {
        let store = TemplateStore()
        store.addScreen()
        store.renameSelectedScreen("Custom Diagnostics")

        let decoded = try RuntimeTemplateCodec.decode(RuntimeTemplateCodec.encode(store.document))

        XCTAssertEqual(decoded, store.document)
        XCTAssertEqual(decoded.screens.count, 12)
        XCTAssertEqual(decoded.screens[1].title, "Custom Diagnostics")
    }

    // ------------------------=
    // FUNC: testZoomControlsClampAndResetCanvasScale
    // DESC: Exercises user zoom controls across their supported bounds and fit reset.
    // ------------------=
    @MainActor
    func testZoomControlsClampAndResetCanvasScale() {
        let store = TemplateStore()
        for _ in 0..<40 { store.zoomIn() }
        XCTAssertEqual(store.zoom, TemplateStore.maximumZoom, accuracy: 0.001)
        for _ in 0..<40 { store.zoomOut() }
        XCTAssertEqual(store.zoom, TemplateStore.minimumZoom, accuracy: 0.001)
        store.resetZoom()
        XCTAssertEqual(store.zoom, 1.0, accuracy: 0.001)
    }
}
