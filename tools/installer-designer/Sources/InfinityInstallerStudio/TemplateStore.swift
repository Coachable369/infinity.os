import AppKit
import Foundation
import SwiftUI
import UniformTypeIdentifiers

enum ResizeHandle: CaseIterable, Identifiable {
    case topLeft, top, topRight, right, bottomRight, bottom, bottomLeft, left
    var id: Self { self }
}

@MainActor
final class TemplateStore: ObservableObject {
    static let minimumZoom = 0.25
    static let maximumZoom = 3.0
    @Published var document: InstallerStudioDocument
    @Published var selectedScreenID = 1
    @Published var selectedElementID: UUID?
    @Published var gridSize = 10
    @Published var snapEnabled = true
    @Published var showGrid = true
    @Published var zoom = 1.0
    @Published var status = "Ready"
    @Published var validationIssues: [String] = []
    @Published var projectRoot: URL?

    private var undoStack: [InstallerStudioDocument] = []
    private var redoStack: [InstallerStudioDocument] = []
    private var gestureBaseline: InstallerStudioDocument?
    private var gestureFrame: CanvasRect?

    var selectedScreenIndex: Int? {
        document.screens.firstIndex(where: { $0.id == selectedScreenID })
    }

    var selectedScreen: InstallerScreenTemplate? {
        guard let index = selectedScreenIndex else { return nil }
        return document.screens[index]
    }

    var selectedElement: StudioElement? {
        guard let screen = selectedScreen,
              let id = selectedElementID
        else { return nil }
        return screen.elements.first(where: { $0.id == id })
    }

    var canUndo: Bool { !undoStack.isEmpty }
    var canRedo: Bool { !redoStack.isEmpty }
    var canAddScreen: Bool { document.screens.count < InstallerStudioDocument.maximumScreenCount }
    var canRemoveScreen: Bool { document.screens.count > InstallerStudioDocument.minimumScreenCount }

    // ------------------------=
    // FUNC: init
    // DESC: Loads the repository template when available or creates the complete factory document.
    // ------------------=
    init() {
        let root = Self.findProjectRoot()
        projectRoot = root
        if let root,
           let data = try? Data(contentsOf: root.appending(path: "assets/boot/installer-screens.infinityui")),
           let decoded = try? JSONDecoder().decode(InstallerStudioDocument.self, from: data),
           (try? TemplateValidator.validate(decoded)) != nil
        {
            document = decoded
            status = "Loaded repository templates"
        } else {
            document = .factoryDefault()
        }
    }

    // ------------------------=
    // FUNC: selectScreen
    // DESC: Selects one installer screen and clears an element selection that does not belong to it.
    // ------------------=
    func selectScreen(_ id: Int) {
        selectedScreenID = id
        selectedElementID = nil
        status = "Screen \(id) selected"
    }

    // ------------------------=
    // FUNC: selectElement
    // DESC: Selects an editable or locked element for canvas and inspector feedback.
    // ------------------=
    func selectElement(_ id: UUID?) {
        selectedElementID = id
    }

    // ------------------------=
    // FUNC: addScreen
    // DESC: Inserts a complete editable installer screen after the current screen.
    // ------------------=
    func addScreen() {
        guard canAddScreen else {
            status = "A maximum of 32 screens is supported"
            return
        }
        recordUndo()
        let insertion = min((selectedScreenIndex ?? (document.screens.count - 1)) + 1, document.screens.count)
        var screen = selectedScreen ?? InstallerStudioDocument.factoryDefault().screens[0]
        screen.title = "New Screen"
        screen.elements = clonedElements(screen.elements)
        document.screens.insert(screen, at: insertion)
        reindexScreens()
        selectedScreenID = insertion + 1
        selectedElementID = nil
        status = "Screen added"
    }

    // ------------------------=
    // FUNC: duplicateScreen
    // DESC: Duplicates the selected screen with fresh element identifiers and intact protected controls.
    // ------------------=
    func duplicateScreen() {
        guard canAddScreen, let index = selectedScreenIndex else {
            status = "A maximum of 32 screens is supported"
            return
        }
        recordUndo()
        var screen = document.screens[index]
        screen.title += " Copy"
        screen.elements = clonedElements(screen.elements)
        document.screens.insert(screen, at: index + 1)
        reindexScreens()
        selectedScreenID = index + 2
        selectedElementID = nil
        status = "Screen duplicated"
    }

    // ------------------------=
    // FUNC: removeScreen
    // DESC: Removes the selected screen while retaining at least one valid installer screen.
    // ------------------=
    func removeScreen() {
        guard canRemoveScreen, let index = selectedScreenIndex else {
            status = "At least one installer screen is required"
            return
        }
        recordUndo()
        document.screens.remove(at: index)
        reindexScreens()
        selectedScreenID = min(index + 1, document.screens.count)
        selectedElementID = nil
        status = "Screen removed"
    }

    // ------------------------=
    // FUNC: moveScreens
    // DESC: Reorders screens from native list drag-and-drop positions and preserves the active selection.
    // ------------------=
    func moveScreens(fromOffsets: IndexSet, toOffset: Int) {
        guard !fromOffsets.isEmpty else { return }
        let selectedMarker = selectedScreen?.elements.first?.id
        recordUndo()
        document.screens.move(fromOffsets: fromOffsets, toOffset: toOffset)
        reindexScreens()
        restoreScreenSelection(marker: selectedMarker)
        selectedElementID = nil
        status = "Screens reordered"
    }

    // ------------------------=
    // FUNC: moveScreen
    // DESC: Moves the selected screen one position for precise keyboard and button reordering.
    // ------------------=
    func moveScreen(_ delta: Int) {
        guard let index = selectedScreenIndex else { return }
        let destination = (index + delta).clamped(to: 0...(document.screens.count - 1))
        guard destination != index else { return }
        let selectedMarker = selectedScreen?.elements.first?.id
        recordUndo()
        let screen = document.screens.remove(at: index)
        document.screens.insert(screen, at: destination)
        reindexScreens()
        restoreScreenSelection(marker: selectedMarker)
        selectedElementID = nil
        status = "Screen moved"
    }

    // ------------------------=
    // FUNC: renameSelectedScreen
    // DESC: Updates the selected screen's editor label as one undoable document change.
    // ------------------=
    func renameSelectedScreen(_ title: String) {
        guard let index = selectedScreenIndex, document.screens[index].title != title else { return }
        recordUndo()
        document.screens[index].title = String(title.prefix(63))
        status = "Screen renamed"
    }

    // ------------------------=
    // FUNC: updateSelected
    // DESC: Applies one undoable inspector mutation while enforcing locked-element invariants.
    // ------------------=
    func updateSelected(_ label: String = "Edit Element", mutation: (inout StudioElement) -> Void) {
        guard let location = selectedLocation(), !location.element.locked else {
            status = "Navigation and system elements are locked"
            return
        }
        recordUndo()
        mutation(&document.screens[location.screen].elements[location.elementIndex])
        document.screens[location.screen].elements[location.elementIndex].frame =
            document.screens[location.screen].elements[location.elementIndex].frame.clamped()
        status = label
    }

    // ------------------------=
    // FUNC: addElement
    // DESC: Adds a real panel, image, text, or console element to the active screen.
    // ------------------=
    func addElement(kind: StudioElementKind) {
        guard kind != .button, let screen = selectedScreenIndex else { return }
        recordUndo()
        let offset = document.screens[screen].elements.count % 6 * 12
        let element = StudioElement.make(
            name: "New \(kind.title)",
            kind: kind,
            role: kind == .image ? .image : kind == .text ? .body : .decoration,
            frame: CanvasRect(
                x: 300 + offset,
                y: 470 + offset,
                width: kind == .text ? 360 : 280,
                height: kind == .text ? 70 : 180
            ),
            text: kind == .text ? "Editable text" : "",
            zIndex: nextZIndex(in: screen)
        )
        document.screens[screen].elements.append(element)
        selectedElementID = element.id
        status = "Added \(kind.title)"
    }

    // ------------------------=
    // FUNC: deleteSelected
    // DESC: Deletes the selected editable element and refuses deletion of locked controls.
    // ------------------=
    func deleteSelected() {
        guard let location = selectedLocation(), !location.element.locked else {
            status = "Locked elements cannot be deleted"
            return
        }
        recordUndo()
        document.screens[location.screen].elements.remove(at: location.elementIndex)
        selectedElementID = nil
        status = "Element deleted"
    }

    // ------------------------=
    // FUNC: duplicateSelected
    // DESC: Duplicates the selected editable element with a snapped offset.
    // ------------------=
    func duplicateSelected() {
        guard let location = selectedLocation(), !location.element.locked else {
            status = "Locked elements cannot be duplicated"
            return
        }
        recordUndo()
        var copy = location.element
        copy.id = UUID()
        copy.name += " Copy"
        copy.frame.x += gridSize
        copy.frame.y += gridSize
        copy.frame = copy.frame.clamped()
        copy.zIndex = nextZIndex(in: location.screen)
        document.screens[location.screen].elements.append(copy)
        selectedElementID = copy.id
        status = "Element duplicated"
    }

    // ------------------------=
    // FUNC: moveLayer
    // DESC: Moves an editable element forward or backward in deterministic z-order.
    // ------------------=
    func moveLayer(_ delta: Int) {
        updateSelected(delta > 0 ? "Brought Forward" : "Sent Backward") {
            $0.zIndex = ($0.zIndex + delta).clamped(to: 0...32_767)
        }
    }

    // ------------------------=
    // FUNC: beginGesture
    // DESC: Captures one undo baseline and geometry origin for a canvas drag or resize.
    // ------------------=
    func beginGesture(elementID: UUID? = nil) {
        if let elementID, selectedElementID != elementID {
            selectedElementID = elementID
        }
        guard gestureBaseline == nil, let element = selectedElement, !element.locked else { return }
        gestureBaseline = document
        gestureFrame = element.frame
    }

    // ------------------------=
    // FUNC: moveSelected
    // DESC: Moves the selected element from its gesture origin with optional grid snapping.
    // ------------------=
    func moveSelected(translation: CGSize, canvasScale: CGSize) {
        guard let location = selectedLocation(), !location.element.locked,
              let origin = gestureFrame
        else { return }
        let dx = Int((translation.width / canvasScale.width).rounded())
        let dy = Int((translation.height / canvasScale.height).rounded())
        var frame = origin
        frame.x = snap(origin.x + dx)
        frame.y = snap(origin.y + dy)
        document.screens[location.screen].elements[location.elementIndex].frame = frame.clamped()
    }

    // ------------------------=
    // FUNC: resizeSelected
    // DESC: Resizes the selected element from any of eight handles with snapping and minimum bounds.
    // ------------------=
    func resizeSelected(handle: ResizeHandle, translation: CGSize, canvasScale: CGSize) {
        guard let location = selectedLocation(), !location.element.locked,
              let origin = gestureFrame
        else { return }
        let dx = Int((translation.width / canvasScale.width).rounded())
        let dy = Int((translation.height / canvasScale.height).rounded())
        var left = origin.x
        var top = origin.y
        var right = origin.x + origin.width
        var bottom = origin.y + origin.height
        if [.topLeft, .left, .bottomLeft].contains(handle) { left = snap(left + dx) }
        if [.topRight, .right, .bottomRight].contains(handle) { right = snap(right + dx) }
        if [.topLeft, .top, .topRight].contains(handle) { top = snap(top + dy) }
        if [.bottomLeft, .bottom, .bottomRight].contains(handle) { bottom = snap(bottom + dy) }
        if right - left < 20 {
            if [.topLeft, .left, .bottomLeft].contains(handle) { left = right - 20 } else { right = left + 20 }
        }
        if bottom - top < 20 {
            if [.topLeft, .top, .topRight].contains(handle) { top = bottom - 20 } else { bottom = top + 20 }
        }
        document.screens[location.screen].elements[location.elementIndex].frame =
            CanvasRect(x: left, y: top, width: right - left, height: bottom - top).clamped()
    }

    // ------------------------=
    // FUNC: endGesture
    // DESC: Commits a completed canvas gesture as one undo operation.
    // ------------------=
    func endGesture() {
        if let baseline = gestureBaseline, baseline != document {
            undoStack.append(baseline)
            redoStack.removeAll()
            status = "Layout updated"
        }
        gestureBaseline = nil
        gestureFrame = nil
    }

    // ------------------------=
    // FUNC: nudgeSelected
    // DESC: Moves the selected element by one unit or one configured grid interval.
    // ------------------=
    func nudgeSelected(dx: Int, dy: Int, byGrid: Bool) {
        let distance = byGrid ? gridSize : 1
        updateSelected("Element nudged") {
            $0.frame.x += dx * distance
            $0.frame.y += dy * distance
        }
    }

    // ------------------------=
    // FUNC: zoomIn
    // DESC: Increases canvas magnification by one predictable step within safe bounds.
    // ------------------=
    func zoomIn() {
        zoom = (zoom + 0.1).clamped(to: Self.minimumZoom...Self.maximumZoom)
        status = "Canvas zoom " + String(Int(zoom * 100)) + "%"
    }

    // ------------------------=
    // FUNC: zoomOut
    // DESC: Decreases canvas magnification by one predictable step within safe bounds.
    // ------------------=
    func zoomOut() {
        zoom = (zoom - 0.1).clamped(to: Self.minimumZoom...Self.maximumZoom)
        status = "Canvas zoom " + String(Int(zoom * 100)) + "%"
    }

    // ------------------------=
    // FUNC: resetZoom
    // DESC: Restores the fit-relative canvas magnification to one hundred percent.
    // ------------------=
    func resetZoom() {
        zoom = 1.0
        status = "Canvas fit reset"
    }

    // ------------------------=
    // FUNC: undo
    // DESC: Restores the previous complete template document.
    // ------------------=
    func undo() {
        guard let previous = undoStack.popLast() else { return }
        redoStack.append(document)
        document = previous
        selectedElementID = nil
        status = "Undo"
    }

    // ------------------------=
    // FUNC: redo
    // DESC: Restores the next complete template document.
    // ------------------=
    func redo() {
        guard let next = redoStack.popLast() else { return }
        undoStack.append(document)
        document = next
        selectedElementID = nil
        status = "Redo"
    }

    // ------------------------=
    // FUNC: validate
    // DESC: Runs authoritative template validation and publishes actionable issues.
    // ------------------=
    @discardableResult
    func validate() -> Bool {
        do {
            try TemplateValidator.validate(document)
            validationIssues = []
            status = "All " + String(document.screens.count) + " screens are valid"
            return true
        } catch {
            validationIssues = [String(describing: error)]
            status = "Validation failed"
            return false
        }
    }

    // ------------------------=
    // FUNC: save
    // DESC: Atomically saves editable JSON and runtime binary templates into the selected InfinityOS project.
    // ------------------=
    func save() {
        guard validate() else { return }
        guard let root = projectRoot ?? chooseProjectRoot() else {
            status = "Save cancelled"
            return
        }
        do {
            let assetDirectory = root.appending(path: "assets/boot", directoryHint: .isDirectory)
            try FileManager.default.createDirectory(at: assetDirectory, withIntermediateDirectories: true)
            let encoder = JSONEncoder()
            encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
            let editable = try encoder.encode(document)
            let runtime = try RuntimeTemplateCodec.encode(document)
            try editable.write(to: assetDirectory.appending(path: "installer-screens.infinityui"), options: .atomic)
            try runtime.write(to: assetDirectory.appending(path: "installer-screens.iuit"), options: .atomic)
            projectRoot = root
            status = "Saved editable and runtime templates"
        } catch {
            validationIssues = [error.localizedDescription]
            status = "Save failed"
        }
    }

    // ------------------------=
    // FUNC: importDocument
    // DESC: Imports and validates an editable JSON template selected by the user.
    // ------------------=
    func importDocument() {
        let panel = NSOpenPanel()
        let editableType = UTType(filenameExtension: "infinityui", conformingTo: .json)
        let runtimeType = UTType(filenameExtension: "iuit", conformingTo: .data)
        panel.allowedContentTypes = [.json] + [editableType, runtimeType].compactMap { $0 }
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            let data = try Data(contentsOf: url)
            let imported = if url.pathExtension.lowercased() == "iuit" {
                try RuntimeTemplateCodec.decode(data)
            } else {
                try JSONDecoder().decode(InstallerStudioDocument.self, from: data)
            }
            try TemplateValidator.validate(imported)
            recordUndo()
            document = imported
            selectedScreenID = 1
            selectedElementID = nil
            status = "Imported \(url.lastPathComponent)"
        } catch {
            validationIssues = [String(describing: error)]
            status = "Import failed"
        }
    }

    // ------------------------=
    // FUNC: resetScreen
    // DESC: Restores only the active screen from the factory template as an undoable action.
    // ------------------=
    func resetScreen() {
        guard let index = selectedScreenIndex,
              let factory = InstallerStudioDocument.factoryDefault().screens.first(where: { $0.id == selectedScreenID })
        else { return }
        recordUndo()
        document.screens[index] = factory
        selectedElementID = nil
        status = "Screen reset"
    }

    // ------------------------=
    // FUNC: selectedLocation
    // DESC: Resolves the selected element and its stable document indexes.
    // ------------------=
    private func selectedLocation() -> (screen: Int, elementIndex: Int, element: StudioElement)? {
        guard let screen = selectedScreenIndex,
              let id = selectedElementID,
              let elementIndex = document.screens[screen].elements.firstIndex(where: { $0.id == id })
        else { return nil }
        return (screen, elementIndex, document.screens[screen].elements[elementIndex])
    }

    // ------------------------=
    // FUNC: clonedElements
    // DESC: Copies a screen's element collection with fresh stable identities.
    // ------------------=
    private func clonedElements(_ elements: [StudioElement]) -> [StudioElement] {
        elements.map { element in
            var copy = element
            copy.id = UUID()
            return copy
        }
    }

    // ------------------------=
    // FUNC: reindexScreens
    // DESC: Keeps persisted screen identifiers contiguous and aligned with visible ordering.
    // ------------------=
    private func reindexScreens() {
        for index in document.screens.indices {
            document.screens[index].id = index + 1
        }
    }

    // ------------------------=
    // FUNC: restoreScreenSelection
    // DESC: Restores the selected screen after reordering using an element identity marker.
    // ------------------=
    private func restoreScreenSelection(marker: UUID?) {
        guard let marker,
              let screen = document.screens.first(where: { screen in
                  screen.elements.contains(where: { $0.id == marker })
              })
        else {
            selectedScreenID = document.screens.first?.id ?? 1
            return
        }
        selectedScreenID = screen.id
    }

    // ------------------------=
    // FUNC: recordUndo
    // DESC: Records one document snapshot and invalidates redo history.
    // ------------------=
    private func recordUndo() {
        undoStack.append(document)
        if undoStack.count > 100 { undoStack.removeFirst() }
        redoStack.removeAll()
    }

    // ------------------------=
    // FUNC: snap
    // DESC: Snaps one normalized coordinate to the configured grid when enabled.
    // ------------------=
    private func snap(_ value: Int) -> Int {
        guard snapEnabled, gridSize > 1 else { return value }
        return Int((Double(value) / Double(gridSize)).rounded()) * gridSize
    }

    // ------------------------=
    // FUNC: nextZIndex
    // DESC: Returns the next frontmost editable layer index for one screen.
    // ------------------=
    private func nextZIndex(in screen: Int) -> Int {
        (document.screens[screen].elements.map(\.zIndex).max() ?? 0) + 1
    }

    // ------------------------=
    // FUNC: chooseProjectRoot
    // DESC: Prompts for an InfinityOS repository folder when automatic discovery is unavailable.
    // ------------------=
    private func chooseProjectRoot() -> URL? {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.prompt = "Use InfinityOS Project"
        guard panel.runModal() == .OK else { return nil }
        return panel.url
    }

    // ------------------------=
    // FUNC: findProjectRoot
    // DESC: Finds the nearest ancestor containing the InfinityOS kernel and boot assets.
    // ------------------=
    private static func findProjectRoot() -> URL? {
        let candidates = [
            URL(fileURLWithPath: FileManager.default.currentDirectoryPath),
            Bundle.main.bundleURL,
        ]
        for candidate in candidates {
            var cursor = candidate
            for _ in 0..<10 {
                let kernel = cursor.appending(path: "kernel/core/bootstrap/installer.rs")
                let assets = cursor.appending(path: "assets/boot")
                if FileManager.default.fileExists(atPath: kernel.path),
                   FileManager.default.fileExists(atPath: assets.path)
                {
                    return cursor
                }
                cursor.deleteLastPathComponent()
            }
        }
        return nil
    }
}
