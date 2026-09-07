import AppKit
import Foundation
import SwiftUI
import UniformTypeIdentifiers

enum ResizeHandle: CaseIterable, Identifiable {
    case topLeft, top, topRight, right, bottomRight, bottom, bottomLeft, left
    var id: Self { self }

    var accessibilityName: String {
        switch self {
        case .topLeft: "top left"
        case .top: "top"
        case .topRight: "top right"
        case .right: "right"
        case .bottomRight: "bottom right"
        case .bottom: "bottom"
        case .bottomLeft: "bottom left"
        case .left: "left"
        }
    }
}

enum ConsoleLayoutPreset: String, CaseIterable, Identifiable {
    case diskCards
    case diskSplit
    case storageMap

    var id: String { rawValue }
    var title: String {
        switch self {
        case .diskCards: "Disk Cards"
        case .diskSplit: "Discovery Split"
        case .storageMap: "Storage Map"
        }
    }
}

private struct ProjectSnapshot {
    var installation: InstallerStudioDocument
    var configuration: InstallerStudioDocument
}

@MainActor
final class TemplateStore: ObservableObject {
    static let minimumZoom = 0.25
    static let maximumZoom = 3.0
    @Published var document: InstallerStudioDocument
    @Published var configurationDocument: InstallerStudioDocument
    @Published var selectedCollection = ScreenCollection.installation
    @Published var selectedScreenID = 1
    @Published var selectedElementID: UUID? {
        didSet {
            guard let selectedElementID else {
                selectedElementIDs = []
                return
            }
            if !selectedElementIDs.contains(selectedElementID) {
                selectedElementIDs = [selectedElementID]
            }
        }
    }
    @Published var selectedElementIDs: Set<UUID> = []
    @Published var inlineEditorElementID: UUID?
    @Published var marqueeSelectionEnabled = false
    @Published var gridSize = 10
    @Published var snapEnabled = true
    @Published var showGrid = true
    @Published var zoom = 1.0
    @Published var status = "Ready"
    @Published var validationIssues: [String] = []
    @Published var projectRoot: URL?

    private var undoStack: [ProjectSnapshot] = []
    private var redoStack: [ProjectSnapshot] = []
    private var gestureBaseline: ProjectSnapshot?
    private var gestureFrames: [UUID: CanvasRect] = [:]

    var activeScreens: [InstallerScreenTemplate] {
        selectedCollection == .installation ? document.screens : configurationDocument.screens
    }

    private var activeDocument: InstallerStudioDocument {
        get { selectedCollection == .installation ? document : configurationDocument }
        set {
            if selectedCollection == .installation {
                document = newValue
            } else {
                configurationDocument = newValue
            }
        }
    }

    var selectedScreenIndex: Int? {
        activeScreens.firstIndex(where: { $0.id == selectedScreenID })
    }

    var selectedScreen: InstallerScreenTemplate? {
        guard let index = selectedScreenIndex else { return nil }
        return activeScreens[index]
    }

    var selectedElement: StudioElement? {
        guard let screen = selectedScreen,
              let id = selectedElementID
        else { return nil }
        return screen.elements.first(where: { $0.id == id })
    }

    var canUndo: Bool { !undoStack.isEmpty }
    var canRedo: Bool { !redoStack.isEmpty }
    var canAddScreen: Bool { activeScreens.count < InstallerStudioDocument.maximumScreenCount }
    var canRemoveScreen: Bool { activeScreens.count > InstallerStudioDocument.minimumScreenCount }
    var resizeHandlesVisible: Bool {
        marqueeSelectionEnabled && selectedElementIDs.count == 1 && selectedElement?.locked == false
    }

    // ------------------------=
    // FUNC: init
    // DESC: Loads the repository template when available or creates the complete factory document.
    // ------------------=
    init() {
        let root = Self.findProjectRoot()
        projectRoot = root
        let configuration = root.flatMap { root in
            try? Data(contentsOf: root.appending(path: "assets/boot/configuration-screens.infinityui"))
        }.flatMap { data in
            try? JSONDecoder().decode(InstallerStudioDocument.self, from: data)
        }
        configurationDocument = if let configuration,
                                   (try? TemplateValidator.validate(configuration)) != nil {
            configuration
        } else {
            .factoryConfiguration()
        }
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
    // FUNC: selectScreenCollection
    // DESC: Activates one sidebar screen collection and selects its requested screen.
    // ------------------=
    func selectScreenCollection(_ collection: ScreenCollection, screen id: Int = 1) {
        selectedCollection = collection
        selectScreen(id)
    }

    // ------------------------=
    // FUNC: selectScreen
    // DESC: Selects one installer screen and clears an element selection that does not belong to it.
    // ------------------=
    func selectScreen(_ id: Int) {
        selectedScreenID = id
        selectedElementID = nil
        inlineEditorElementID = nil
        status = "\(selectedCollection.title) · Screen \(id) selected"
    }

    // ------------------------=
    // FUNC: selectElement
    // DESC: Selects an editable or locked element for canvas and inspector feedback.
    // ------------------=
    func selectElement(_ id: UUID?) {
        if selectedElementID != id {
            inlineEditorElementID = nil
        }
        selectedElementIDs = id.map { [$0] } ?? []
        selectedElementID = id
    }

    // ------------------------=
    // FUNC: selectElements
    // DESC: Selects every visible unlocked element fully enclosed by a marquee, optionally extending the current set.
    // ------------------=
    func selectElements(in marquee: CanvasRect, additive: Bool = false) {
        guard let screen = selectedScreen else {
            selectElement(nil)
            return
        }
        let hits = screen.elements.filter { element in
            !element.hidden && !element.locked && marqueeContains(element.frame, marquee: marquee)
        }
        var nextSelection = additive ? selectedElementIDs : []
        nextSelection.formUnion(hits.map(\.id))
        selectedElementIDs = nextSelection
        inlineEditorElementID = nil

        if nextSelection.isEmpty {
            selectedElementID = nil
            status = "No elements selected"
            return
        }
        if !additive || selectedElementID == nil || !nextSelection.contains(selectedElementID!) {
            selectedElementID = hits.max { lhs, rhs in
                if lhs.zIndex != rhs.zIndex { return lhs.zIndex < rhs.zIndex }
                return lhs.id.uuidString < rhs.id.uuidString
            }?.id ?? nextSelection.first
        }
        status = nextSelection.count == 1 ? "1 element selected" : "\(nextSelection.count) elements selected"
    }

    // ------------------------=
    // FUNC: activateCanvas
    // DESC: Point-selects the frontmost visible layer through the Marquee interaction surface.
    // ------------------=
    func activateCanvas(at point: CGPoint, additive: Bool = false) {
        guard let element = topmostVisibleElement(at: point) else {
            if !additive { selectElement(nil) }
            return
        }
        if !additive {
            activateCanvasElement(element.id)
            return
        }
        inlineEditorElementID = nil
        var nextSelection = selectedElementIDs
        if nextSelection.contains(element.id) {
            nextSelection.remove(element.id)
        } else {
            nextSelection.insert(element.id)
        }
        selectedElementIDs = nextSelection
        selectedElementID = nextSelection.contains(element.id) ? element.id : nextSelection.first
        status = nextSelection.isEmpty
            ? "No elements selected"
            : nextSelection.count == 1 ? "1 element selected" : "\(nextSelection.count) elements selected"
    }

    // ------------------------=
    // FUNC: canMoveSelection
    // DESC: Determines whether a Marquee drag began on the frontmost unlocked member of the current selection.
    // ------------------=
    func canMoveSelection(at point: CGPoint) -> Bool {
        guard let element = topmostVisibleElement(at: point) else { return false }
        return !element.locked && selectedElementIDs.contains(element.id)
    }

    // ------------------------=
    // FUNC: toggleMarqueeSelection
    // DESC: Toggles the whole-artboard selection tool and its resize affordances.
    // ------------------=
    func toggleMarqueeSelection() {
        marqueeSelectionEnabled.toggle()
        inlineEditorElementID = nil
        if !marqueeSelectionEnabled {
            selectedElementID = nil
            gestureBaseline = nil
            gestureFrames = [:]
        }
        status = marqueeSelectionEnabled ? "Marquee Select enabled" : "Marquee Select disabled"
    }

    // ------------------------=
    // FUNC: activateCanvasElement
    // DESC: Selects a clicked canvas object for editing in the persistent right-hand inspector.
    // ------------------=
    func activateCanvasElement(_ id: UUID) {
        selectElement(id)
        inlineEditorElementID = nil
        status = selectedElement?.locked == false
            ? "Selected for editing in the inspector"
            : "Locked element selected for inspection"
    }

    // ------------------------=
    // FUNC: toggleElementLock
    // DESC: Toggles the persisted lock state for any element on the selected screen.
    // ------------------=
    func toggleElementLock(_ id: UUID) {
        guard let screen = selectedScreenIndex,
              let element = activeDocument.screens[screen].elements.firstIndex(where: { $0.id == id })
        else { return }
        recordUndo()
        activeDocument.screens[screen].elements[element].locked.toggle()
        selectedElementID = id
        if activeDocument.screens[screen].elements[element].locked {
            inlineEditorElementID = nil
            status = "Element locked"
        } else {
            status = "Element unlocked"
        }
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
        let insertion = min((selectedScreenIndex ?? (activeScreens.count - 1)) + 1, activeScreens.count)
        let factory = selectedCollection == .installation
            ? InstallerStudioDocument.factoryDefault()
            : InstallerStudioDocument.factoryConfiguration()
        var screen = selectedScreen ?? factory.screens[0]
        screen.title = "New Screen"
        screen.elements = clonedElements(screen.elements)
        activeDocument.screens.insert(screen, at: insertion)
        reindexScreens()
        selectedScreenID = insertion + 1
        selectedElementID = nil
        inlineEditorElementID = nil
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
        var screen = activeDocument.screens[index]
        screen.title += " Copy"
        screen.elements = clonedElements(screen.elements)
        activeDocument.screens.insert(screen, at: index + 1)
        reindexScreens()
        selectedScreenID = index + 2
        selectedElementID = nil
        inlineEditorElementID = nil
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
        activeDocument.screens.remove(at: index)
        reindexScreens()
        selectedScreenID = min(index + 1, activeScreens.count)
        selectedElementID = nil
        inlineEditorElementID = nil
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
        activeDocument.screens.move(fromOffsets: fromOffsets, toOffset: toOffset)
        reindexScreens()
        restoreScreenSelection(marker: selectedMarker)
        selectedElementID = nil
        inlineEditorElementID = nil
        status = "Screens reordered"
    }

    // ------------------------=
    // FUNC: moveScreensInCollection
    // DESC: Activates and reorders one explicit sidebar collection from native list drag positions.
    // ------------------=
    func moveScreensInCollection(
        _ collection: ScreenCollection,
        fromOffsets: IndexSet,
        toOffset: Int
    ) {
        if selectedCollection != collection {
            selectedCollection = collection
            selectedScreenID = activeScreens.first?.id ?? 1
            selectedElementID = nil
            inlineEditorElementID = nil
        }
        moveScreens(fromOffsets: fromOffsets, toOffset: toOffset)
    }

    // ------------------------=
    // FUNC: addScreenToCollection
    // DESC: Activates one collection before inserting a new editable screen into it.
    // ------------------=
    func addScreenToCollection(_ collection: ScreenCollection) {
        if selectedCollection != collection {
            selectScreenCollection(collection)
        }
        addScreen()
    }

    // ------------------------=
    // FUNC: moveScreen
    // DESC: Moves the selected screen one position for precise keyboard and button reordering.
    // ------------------=
    func moveScreen(_ delta: Int) {
        guard let index = selectedScreenIndex else { return }
        let destination = (index + delta).clamped(to: 0...(activeScreens.count - 1))
        guard destination != index else { return }
        let selectedMarker = selectedScreen?.elements.first?.id
        recordUndo()
        let screen = activeDocument.screens.remove(at: index)
        activeDocument.screens.insert(screen, at: destination)
        reindexScreens()
        restoreScreenSelection(marker: selectedMarker)
        selectedElementID = nil
        inlineEditorElementID = nil
        status = "Screen moved"
    }

    // ------------------------=
    // FUNC: renameSelectedScreen
    // DESC: Updates the selected screen's editor label as one undoable document change.
    // ------------------=
    func renameSelectedScreen(_ title: String) {
        guard let index = selectedScreenIndex, activeDocument.screens[index].title != title else { return }
        recordUndo()
        activeDocument.screens[index].title = String(title.prefix(63))
        status = "Screen renamed"
    }

    // ------------------------=
    // FUNC: updateSelected
    // DESC: Applies one undoable inspector mutation while enforcing locked-element invariants.
    // ------------------=
    func updateSelected(_ label: String = "Edit Element", mutation: (inout StudioElement) -> Void) {
        guard let location = selectedLocation(), !location.element.locked else {
            status = "Unlock this element to edit it"
            return
        }
        recordUndo()
        mutation(&activeDocument.screens[location.screen].elements[location.elementIndex])
        activeDocument.screens[location.screen].elements[location.elementIndex].frame =
            activeDocument.screens[location.screen].elements[location.elementIndex].frame.clamped()
        activeDocument.screens[location.screen].elements[location.elementIndex].crop =
            activeDocument.screens[location.screen].elements[location.elementIndex].crop.clamped()
        status = label
    }

    // ------------------------=
    // FUNC: addElement
    // DESC: Adds a real panel, image, text, or console element to the active screen.
    // ------------------=
    func addElement(kind: StudioElementKind) {
        guard kind != .button, let screen = selectedScreenIndex else { return }
        recordUndo()
        let offset = activeDocument.screens[screen].elements.count % 6 * 12
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
        activeDocument.screens[screen].elements.append(element)
        selectedElementID = element.id
        status = "Added \(kind.title)"
    }

    // ------------------------=
    // FUNC: chooseAndAddImage
    // DESC: Selects a PNG or bitmap and immediately adds it as an editable canvas layer.
    // ------------------=
    func chooseAndAddImage() {
        chooseImage(replacingSelected: false)
    }

    // ------------------------=
    // FUNC: chooseReplacementImage
    // DESC: Selects a PNG or bitmap to replace the active editable image layer.
    // ------------------=
    func chooseReplacementImage() {
        chooseImage(replacingSelected: true)
    }

    // ------------------------=
    // FUNC: importImageAsset
    // DESC: Copies a validated PNG or bitmap into boot assets and creates or updates its canvas layer.
    // ------------------=
    @discardableResult
    func importImageAsset(from source: URL, replacingSelected: Bool = false) throws -> UUID {
        guard ["png", "bmp"].contains(source.pathExtension.lowercased()),
              let image = NSImage(contentsOf: source), image.size.width > 0, image.size.height > 0
        else {
            throw TemplateValidationIssue.invalidDocument("Choose a valid PNG or BMP image")
        }
        guard let screen = selectedScreenIndex else {
            throw TemplateValidationIssue.invalidDocument("Select an installation screen first")
        }
        let root = projectRoot ?? chooseProjectRoot()
        guard let root else { throw TemplateValidationIssue.invalidDocument("An InfinityOS project is required") }
        let directory = root.appending(path: "assets/boot/installer-assets", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let destination = uniqueAssetDestination(for: source, in: directory)
        if source.standardizedFileURL != destination.standardizedFileURL {
            try FileManager.default.copyItem(at: source, to: destination)
        }
        let relativePath = "installer-assets/" + destination.lastPathComponent
        recordUndo()
        if replacingSelected, let location = selectedLocation(), location.element.kind == .image,
           !location.element.locked
        {
            activeDocument.screens[location.screen].elements[location.elementIndex].imageAsset = relativePath
            activeDocument.screens[location.screen].elements[location.elementIndex].crop = .none
            status = "Image replaced"
            projectRoot = root
            return location.element.id
        }
        let area = editableContentFrame(in: screen)
        let aspect = image.size.width / image.size.height
        var width = min(460, max(160, area.width - 40))
        var height = Int((CGFloat(width) / aspect).rounded())
        if height > area.height - 40 {
            height = max(100, area.height - 40)
            width = Int((CGFloat(height) * aspect).rounded())
        }
        let element = StudioElement.make(
            name: destination.deletingPathExtension().lastPathComponent,
            kind: .image,
            role: .image,
            frame: CanvasRect(
                x: area.x + max(0, (area.width - width) / 2),
                y: area.y + max(0, (area.height - height) / 2),
                width: width,
                height: height
            ).clamped(),
            imageAsset: relativePath,
            zIndex: nextZIndex(in: screen)
        )
        activeDocument.screens[screen].elements.append(element)
        selectedElementID = element.id
        inlineEditorElementID = nil
        projectRoot = root
        status = "Image added to canvas"
        return element.id
    }

    // ------------------------=
    // FUNC: applyConsoleLayout
    // DESC: Inserts a polished disk-oriented arrangement wholly inside the editable console content area.
    // ------------------=
    func applyConsoleLayout(_ preset: ConsoleLayoutPreset) {
        guard let screen = selectedScreenIndex else { return }
        let area = editableContentFrame(in: screen)
        recordUndo()
        activeDocument.screens[screen].elements.removeAll { $0.name.hasPrefix("Preset • ") }
        let base = nextZIndex(in: screen)
        let gap = 14
        let top = area.y + 92
        let height = max(80, area.height - 108)
        var additions: [StudioElement] = []
        switch preset {
        case .diskCards:
            let cardWidth = max(80, (area.width - gap * 4) / 3)
            for index in 0..<3 {
                let x = area.x + gap + index * (cardWidth + gap)
                additions.append(.make(
                    name: "Preset • Disk Card \(index + 1)", kind: .panel,
                    frame: CanvasRect(x: x, y: top, width: cardWidth, height: height), zIndex: base + index
                ))
                additions.append(.make(
                    name: "Preset • Disk Label \(index + 1)", kind: .text, role: .body,
                    frame: CanvasRect(x: x + 18, y: top + 20, width: cardWidth - 36, height: 58),
                    text: index == 0 ? "SYSTEM DISK\nReady to inspect" : "AVAILABLE DEVICE\nSelect to review",
                    zIndex: base + 3 + index
                ))
            }
        case .diskSplit:
            let leftWidth = max(180, (area.width - gap * 3) * 2 / 5)
            additions.append(.make(
                name: "Preset • Device Details", kind: .panel,
                frame: CanvasRect(x: area.x + gap, y: top, width: leftWidth, height: height), zIndex: base
            ))
            additions.append(.make(
                name: "Preset • Device Summary", kind: .text, role: .body,
                frame: CanvasRect(x: area.x + gap * 2, y: top + 22, width: leftWidth - gap * 2, height: height - 44),
                text: "DEVICE\nConnection\nCapacity\nCurrent contents", zIndex: base + 2
            ))
            additions.append(.make(
                name: "Preset • Discovery Visual", kind: .image, role: .image,
                frame: CanvasRect(x: area.x + leftWidth + gap * 2, y: top, width: area.width - leftWidth - gap * 3, height: height),
                imageAsset: "infinity-disk-discovery-vision-v1.png", zIndex: base + 1
            ))
        case .storageMap:
            additions.append(.make(
                name: "Preset • Storage Map", kind: .image, role: .image,
                frame: CanvasRect(x: area.x + gap, y: top, width: area.width - gap * 2, height: height),
                imageAsset: "infinity-installer-mesh-diagram-v1.png", zIndex: base
            ))
        }
        activeDocument.screens[screen].elements.append(contentsOf: additions)
        selectedElementID = additions.last?.id
        inlineEditorElementID = nil
        status = "\(preset.title) layout inserted"
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
        activeDocument.screens[location.screen].elements.remove(at: location.elementIndex)
        selectedElementID = nil
        inlineEditorElementID = nil
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
        activeDocument.screens[location.screen].elements.append(copy)
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
        if let elementID {
            if !selectedElementIDs.contains(elementID) {
                selectElement(elementID)
            } else {
                selectedElementID = elementID
            }
        }
        inlineEditorElementID = nil
        guard gestureBaseline == nil, let screen = selectedScreen else { return }
        let movable = screen.elements.filter { selectedElementIDs.contains($0.id) && !$0.locked }
        guard !movable.isEmpty else { return }
        gestureBaseline = projectSnapshot()
        gestureFrames = Dictionary(uniqueKeysWithValues: movable.map { ($0.id, $0.frame) })
    }

    // ------------------------=
    // FUNC: moveSelected
    // DESC: Moves every selected element uniformly from its gesture origin with optional grid snapping.
    // ------------------=
    func moveSelected(translation: CGSize, canvasScale: CGSize) {
        guard let screen = selectedScreenIndex, !gestureFrames.isEmpty,
              canvasScale.width > 0, canvasScale.height > 0
        else { return }
        let anchor = selectedElementID.flatMap { gestureFrames[$0] } ?? gestureFrames.values.first!
        let rawDX = Int((translation.width / canvasScale.width).rounded())
        let rawDY = Int((translation.height / canvasScale.height).rounded())
        let requestedDX = snap(anchor.x + rawDX) - anchor.x
        let requestedDY = snap(anchor.y + rawDY) - anchor.y
        let translation = boundedGroupTranslation(
            frames: Array(gestureFrames.values),
            requestedDX: requestedDX,
            requestedDY: requestedDY
        )
        applyGroupTranslation(
            origins: gestureFrames,
            dx: translation.dx,
            dy: translation.dy,
            screen: screen
        )
    }

    // ------------------------=
    // FUNC: resizeSelected
    // DESC: Resizes the selected element from any of eight handles with snapping and minimum bounds.
    // ------------------=
    func resizeSelected(handle: ResizeHandle, translation: CGSize, canvasScale: CGSize) {
        guard let location = selectedLocation(), !location.element.locked,
              gestureFrames.count == 1, let origin = gestureFrames[location.element.id]
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
        activeDocument.screens[location.screen].elements[location.elementIndex].frame =
            CanvasRect(x: left, y: top, width: right - left, height: bottom - top).clamped()
    }

    // ------------------------=
    // FUNC: endGesture
    // DESC: Commits a completed canvas gesture as one undo operation.
    // ------------------=
    func endGesture() {
        if let baseline = gestureBaseline,
           baseline.installation != document || baseline.configuration != configurationDocument
        {
            undoStack.append(baseline)
            redoStack.removeAll()
            status = "Layout updated"
        }
        gestureBaseline = nil
        gestureFrames = [:]
    }

    // ------------------------=
    // FUNC: nudgeSelected
    // DESC: Moves all selected unlocked elements uniformly by one unit or one configured grid interval.
    // ------------------=
    func nudgeSelected(dx: Int, dy: Int, byGrid: Bool) {
        guard let screen = selectedScreenIndex else { return }
        let distance = byGrid ? gridSize : 1
        let selectedFrames = Dictionary(uniqueKeysWithValues: activeDocument.screens[screen].elements.compactMap {
            selectedElementIDs.contains($0.id) && !$0.locked ? ($0.id, $0.frame) : nil
        })
        guard !selectedFrames.isEmpty else {
            status = "Unlock an element to move it"
            return
        }
        let translation = boundedGroupTranslation(
            frames: Array(selectedFrames.values),
            requestedDX: dx * distance,
            requestedDY: dy * distance
        )
        guard translation.dx != 0 || translation.dy != 0 else { return }
        recordUndo()
        applyGroupTranslation(
            origins: selectedFrames,
            dx: translation.dx,
            dy: translation.dy,
            screen: screen
        )
        status = selectedFrames.count == 1 ? "Element nudged" : "Selection nudged"
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
        redoStack.append(projectSnapshot())
        document = previous.installation
        configurationDocument = previous.configuration
        selectedElementID = nil
        inlineEditorElementID = nil
        status = "Undo"
    }

    // ------------------------=
    // FUNC: redo
    // DESC: Restores the next complete template document.
    // ------------------=
    func redo() {
        guard let next = redoStack.popLast() else { return }
        undoStack.append(projectSnapshot())
        document = next.installation
        configurationDocument = next.configuration
        selectedElementID = nil
        inlineEditorElementID = nil
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
            try TemplateValidator.validate(configurationDocument)
            validationIssues = []
            status = "Validated \(document.screens.count) installation and \(configurationDocument.screens.count) configuration screens"
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
            let runtime = try RuntimeTemplateCodec.encode(document, assetRoot: root)
            let configurationEditable = try encoder.encode(configurationDocument)
            let configurationRuntime = try RuntimeTemplateCodec.encode(configurationDocument, assetRoot: root)
            guard try RuntimeTemplateCodec.decode(runtime) == document else {
                throw TemplateValidationIssue.invalidDocument("Generated runtime artifact failed round-trip verification")
            }
            guard try RuntimeTemplateCodec.decode(configurationRuntime) == configurationDocument else {
                throw TemplateValidationIssue.invalidDocument("Generated configuration artifact failed round-trip verification")
            }
            try editable.write(to: assetDirectory.appending(path: "installer-screens.infinityui"), options: .atomic)
            try runtime.write(to: assetDirectory.appending(path: "installer-screens.iuit"), options: .atomic)
            try configurationEditable.write(
                to: assetDirectory.appending(path: "configuration-screens.infinityui"), options: .atomic
            )
            try configurationRuntime.write(
                to: assetDirectory.appending(path: "configuration-screens.iuit"), options: .atomic
            )
            projectRoot = root
            status = "Saved installation and OS configuration templates"
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
            activeDocument = imported
            selectedScreenID = 1
            selectedElementID = nil
            inlineEditorElementID = nil
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
              let factory = (selectedCollection == .installation
                  ? InstallerStudioDocument.factoryDefault()
                  : InstallerStudioDocument.factoryConfiguration())
                  .screens.first(where: { $0.id == selectedScreenID })
        else { return }
        recordUndo()
        activeDocument.screens[index] = factory
        selectedElementID = nil
        inlineEditorElementID = nil
        status = "Screen reset"
    }

    // ------------------------=
    // FUNC: chooseImage
    // DESC: Presents the native file picker and routes a selected PNG or bitmap into the project.
    // ------------------=
    private func chooseImage(replacingSelected: Bool) {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = ["png", "bmp"].compactMap {
            UTType(filenameExtension: $0, conformingTo: .image)
        }
        panel.allowsMultipleSelection = false
        panel.prompt = replacingSelected ? "Replace Image" : "Add to Canvas"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            _ = try importImageAsset(from: url, replacingSelected: replacingSelected)
        } catch {
            validationIssues = [String(describing: error)]
            status = "Image import failed"
        }
    }

    // ------------------------=
    // FUNC: editableContentFrame
    // DESC: Resolves a safe preset and image placement area inside the selected console.
    // ------------------=
    private func editableContentFrame(in screen: Int) -> CanvasRect {
        if let content = activeDocument.screens[screen].elements.first(where: { $0.role == .content }) {
            return content.frame
        }
        if let console = activeDocument.screens[screen].elements.first(where: { $0.role == .console }) {
            return CanvasRect(
                x: console.frame.x + 18,
                y: console.frame.y + 70,
                width: max(20, console.frame.width - 36),
                height: max(20, console.frame.height - 190)
            ).clamped()
        }
        return CanvasRect(x: 40, y: 390, width: 920, height: 410)
    }

    // ------------------------=
    // FUNC: uniqueAssetDestination
    // DESC: Creates a safe non-destructive repository destination for one imported image.
    // ------------------=
    private func uniqueAssetDestination(for source: URL, in directory: URL) -> URL {
        let allowed = CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "-_."))
        let cleaned = source.lastPathComponent.unicodeScalars.map { allowed.contains($0) ? Character(String($0)) : "-" }
        let filename = String(cleaned).isEmpty ? "installer-image.\(source.pathExtension.lowercased())" : String(cleaned)
        var destination = directory.appending(path: filename)
        var suffix = 2
        while FileManager.default.fileExists(atPath: destination.path),
              destination.standardizedFileURL != source.standardizedFileURL
        {
            let stem = URL(fileURLWithPath: filename).deletingPathExtension().lastPathComponent
            destination = directory.appending(path: "\(stem)-\(suffix).\(source.pathExtension.lowercased())")
            suffix += 1
        }
        return destination
    }

    // ------------------------=
    // FUNC: selectedLocation
    // DESC: Resolves the selected element and its stable document indexes.
    // ------------------=
    private func selectedLocation() -> (screen: Int, elementIndex: Int, element: StudioElement)? {
        guard let screen = selectedScreenIndex,
              let id = selectedElementID,
              let elementIndex = activeDocument.screens[screen].elements.firstIndex(where: { $0.id == id })
        else { return nil }
        return (screen, elementIndex, activeDocument.screens[screen].elements[elementIndex])
    }

    // ------------------------=
    // FUNC: marqueeContains
    // DESC: Tests whether a normalized marquee completely encloses an element frame.
    // ------------------=
    private func marqueeContains(_ element: CanvasRect, marquee: CanvasRect) -> Bool {
        element.x >= marquee.x
            && element.y >= marquee.y
            && element.x + element.width <= marquee.x + marquee.width
            && element.y + element.height <= marquee.y + marquee.height
    }

    // ------------------------=
    // FUNC: topmostVisibleElement
    // DESC: Resolves the highest visible layer containing one normalized point for click and drag routing.
    // ------------------=
    private func topmostVisibleElement(at point: CGPoint) -> StudioElement? {
        selectedScreen?.elements.filter { element in
            !element.hidden
                && point.x >= CGFloat(element.frame.x)
                && point.x <= CGFloat(element.frame.x + element.frame.width)
                && point.y >= CGFloat(element.frame.y)
                && point.y <= CGFloat(element.frame.y + element.frame.height)
        }.max { lhs, rhs in
            if lhs.zIndex != rhs.zIndex { return lhs.zIndex < rhs.zIndex }
            return lhs.id.uuidString < rhs.id.uuidString
        }
    }

    // ------------------------=
    // FUNC: boundedGroupTranslation
    // DESC: Constrains one shared translation so the complete selected group remains inside the artboard.
    // ------------------=
    private func boundedGroupTranslation(
        frames: [CanvasRect],
        requestedDX: Int,
        requestedDY: Int
    ) -> (dx: Int, dy: Int) {
        guard let minX = frames.map(\.x).min(),
              let minY = frames.map(\.y).min(),
              let maxX = frames.map({ $0.x + $0.width }).max(),
              let maxY = frames.map({ $0.y + $0.height }).max()
        else { return (0, 0) }
        return (
            requestedDX.clamped(to: -minX...(1000 - maxX)),
            requestedDY.clamped(to: -minY...(1000 - maxY))
        )
    }

    // ------------------------=
    // FUNC: applyGroupTranslation
    // DESC: Applies one identical bounded delta to every selected origin without changing relative spacing.
    // ------------------=
    private func applyGroupTranslation(
        origins: [UUID: CanvasRect],
        dx: Int,
        dy: Int,
        screen: Int
    ) {
        for index in activeDocument.screens[screen].elements.indices {
            let id = activeDocument.screens[screen].elements[index].id
            guard var frame = origins[id] else { continue }
            frame.x += dx
            frame.y += dy
            activeDocument.screens[screen].elements[index].frame = frame
        }
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
        for index in activeDocument.screens.indices {
            activeDocument.screens[index].id = index + 1
        }
    }

    // ------------------------=
    // FUNC: restoreScreenSelection
    // DESC: Restores the selected screen after reordering using an element identity marker.
    // ------------------=
    private func restoreScreenSelection(marker: UUID?) {
        guard let marker,
              let screen = activeScreens.first(where: { screen in
                  screen.elements.contains(where: { $0.id == marker })
              })
        else {
            selectedScreenID = activeScreens.first?.id ?? 1
            return
        }
        selectedScreenID = screen.id
    }

    // ------------------------=
    // FUNC: recordUndo
    // DESC: Records one document snapshot and invalidates redo history.
    // ------------------=
    private func recordUndo() {
        undoStack.append(projectSnapshot())
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
        (activeDocument.screens[screen].elements.map(\.zIndex).max() ?? 0) + 1
    }

    // ------------------------=
    // FUNC: projectSnapshot
    // DESC: Captures both editable screen collections as one atomic undo state.
    // ------------------=
    private func projectSnapshot() -> ProjectSnapshot {
        ProjectSnapshot(installation: document, configuration: configurationDocument)
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
