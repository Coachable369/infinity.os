import AppKit
import Combine
import CryptoKit
import Foundation
import SwiftUI
import UniformTypeIdentifiers

enum LayerOrderAction: String, CaseIterable {
    case front = "Bring to Front"
    case forward = "Bring Forward"
    case back = "Send to Back"
    case backward = "Send Backward"
}

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

private struct ProjectSnapshot: Equatable {
    var desktop: InstallerStudioDocument = .factoryDesktop()
    var login: InstallerStudioDocument = .factoryLogin()
    var boot: InstallerStudioDocument
    var installation: InstallerStudioDocument
    var configuration: InstallerStudioDocument
    var systemSettings: InstallerStudioDocument
}

@MainActor
final class TemplateStore: ObservableObject {
    static let minimumZoom = 0.25
    static let maximumZoom = 3.0
    let uiKit: UIKitStore
    let isoUpdater: ISOUpdateService
    @Published var showISOUpdate = false
    @Published var editingUIKit = false
    @Published var editingMenu = false
    @Published var selectedMenuItemID: UUID?
    @Published var document: InstallerStudioDocument
    @Published var bootDocument: InstallerStudioDocument
    @Published var configurationDocument: InstallerStudioDocument
    @Published var settingsDocument: InstallerStudioDocument
    @Published var loginDocument: InstallerStudioDocument = .factoryLogin()
    @Published var desktopDocument: InstallerStudioDocument = .factoryDesktop()
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
    private var sourceLoadIssues: [ScreenCollection: String] = [:]
    @Published var projectRoot: URL?
    @Published var canvasViewport: NativePreviewViewport = .studio
    @Published var nativePreviewState: NativePreviewControlState = .normal
    @Published var nativePreviewSimulation = NativePreviewSimulation()
    private var pendingNativeSelectionReveal: UUID?

    private var undoStack: [ProjectSnapshot] = []
    private var redoStack: [ProjectSnapshot] = []
    private var gestureBaseline: ProjectSnapshot?
    private var gestureFrames: [UUID: CanvasRect] = [:]
    private var kitObservation: AnyCancellable?
    private var savedProject: ProjectSnapshot?

    var hasUnsavedChanges: Bool {
        uiKit.isDirty || savedProject != ProjectSnapshot(desktop: desktopDocument, login: loginDocument, boot: bootDocument, installation: document,
            configuration: configurationDocument, systemSettings: settingsDocument)
    }

    var activeScreens: [InstallerScreenTemplate] {
        switch selectedCollection {
        case .login: loginDocument.screens
        case .desktop: desktopDocument.screens
        case .boot: bootDocument.screens
        case .installation: document.screens
        case .configuration: configurationDocument.screens
        case .systemSettings: settingsDocument.screens
        }
    }

    private var activeDocument: InstallerStudioDocument {
        get {
            switch selectedCollection {
            case .login: loginDocument
            case .desktop: desktopDocument
            case .boot: bootDocument
            case .installation: document
            case .configuration: configurationDocument
            case .systemSettings: settingsDocument
            }
        }
        set {
            switch selectedCollection {
            case .login: loginDocument = newValue
            case .desktop: desktopDocument = newValue
            case .boot: bootDocument = newValue
            case .installation: document = newValue
            case .configuration: configurationDocument = newValue
            case .systemSettings: settingsDocument = newValue
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

    var selectedCodeIdentifierIsDuplicate: Bool {
        guard let selectedElement, let selectedScreen else { return false }
        return selectedScreen.elements.filter {
            $0.codeIdentifier == selectedElement.codeIdentifier
        }.count > 1
    }

    var canUndo: Bool { editingUIKit ? uiKit.canUndo : !undoStack.isEmpty }
    var canRedo: Bool { editingUIKit ? uiKit.canRedo : !redoStack.isEmpty }
    var canAddScreen: Bool { ![.boot, .login, .desktop].contains(selectedCollection) && activeScreens.count < InstallerStudioDocument.maximumScreenCount }
    var canRemoveScreen: Bool { selectedCollection == .desktop ? selectedScreenID >= 4 : ![.boot, .login].contains(selectedCollection) && activeScreens.count > InstallerStudioDocument.minimumScreenCount }
    var resizeHandlesVisible: Bool {
        marqueeSelectionEnabled && selectedElementIDs.count == 1 && selectedElement?.locked == false
    }

    // ------------------------=
    // FUNC: prepareContextSelection
    // DESC: Keeps a multi-selection when its member is right-clicked; otherwise targets the clicked element.
    // ------------------=
    func prepareContextSelection(_ id: UUID) {
        guard selectedScreen?.elements.contains(where: { $0.id == id }) == true else { return }
        if !selectedElementIDs.contains(id) { selectElement(id) }
    }

    // ------------------------=
    // FUNC: beginContextResize
    // DESC: Exposes existing snapping resize grips for the clicked unlocked, user-positioned element.
    // ------------------=
    func beginContextResize(_ id: UUID) {
        guard let element = selectedScreen?.elements.first(where: { $0.id == id }),
              !element.locked, geometryIsEditable(element) else { return }
        selectElement(id)
        marqueeSelectionEnabled = true
        status = "Drag a grip to resize — uses the current snapping setting"
    }

    // ------------------------=
    // FUNC: canPasteElements
    // DESC: Enables paste only for a bounded, decodable Studio clipboard payload on an editable screen.
    // ------------------=
    func canPasteElements(from pasteboard: NSPasteboard = .general) -> Bool {
        guard !editingUIKit, selectedScreen != nil,
              let data = pasteboard.data(forType: Self.elementPasteboardType), data.count <= 4_194_304,
              let elements = try? JSONDecoder().decode([StudioElement].self, from: data) else { return false }
        return !elements.isEmpty && elements.count <= 256
    }
    // ------------------------=
    // FUNC: selectedElementIsRequired
    // DESC: Reports whether the current element participates in the active runtime workflow contract.
    // ------------------=
    var selectedElementIsRequired: Bool {
        guard let element = selectedElement, let screen = selectedScreen else { return false }
        let required = selectedCollection.requiredRoleCount(element.role, screenID: selectedScreenID)
        return required > 0 && screen.elements.filter { $0.role == element.role }.count <= required
    }

    // ------------------------=
    // FUNC: init
    // DESC: Loads the repository template when available or creates the complete factory document.
    // ------------------=
    init(projectRoot suppliedRoot: URL? = nil) {
        let root = suppliedRoot ?? Self.findProjectRoot()
        uiKit = UIKitStore(projectRoot: root)
        isoUpdater = ISOUpdateService(projectRoot: root)
        projectRoot = root
        desktopDocument = root.flatMap { try? Data(contentsOf: $0.appending(path: "assets/boot/desktop-screens.infinityui")) }
            .flatMap { try? JSONDecoder().decode(InstallerStudioDocument.self, from: $0) } ?? .factoryDesktop()
        loginDocument = root.flatMap { try? Data(contentsOf: $0.appending(path: "assets/boot/login-screens.infinityui")) }
            .flatMap { try? JSONDecoder().decode(InstallerStudioDocument.self, from: $0) } ?? .factoryLogin()
        bootDocument = root.flatMap { try? Data(contentsOf: $0.appending(path: "assets/boot/boot-screens.infinityui")) }
            .flatMap { try? JSONDecoder().decode(InstallerStudioDocument.self, from: $0) } ?? .factoryBoot()
        let configuration = root.flatMap { root in
            try? Data(contentsOf: root.appending(path: "assets/boot/configuration-screens.infinityui"))
        }.flatMap { data in
            try? JSONDecoder().decode(InstallerStudioDocument.self, from: data)
        }
        configurationDocument = if let configuration,
                                   (try? TemplateValidator.validate(configuration, for: .configuration)) != nil {
            configuration
        } else {
            .factoryConfiguration()
        }
        let settings = root.flatMap { root in
            try? Data(contentsOf: root.appending(path: "assets/boot/settings-screens.infinityui"))
        }.flatMap { data in
            try? JSONDecoder().decode(InstallerStudioDocument.self, from: data)
        }.map { $0.migratedForSettingsCardReadability() }
        settingsDocument = if let settings,
                              (try? TemplateValidator.validate(settings, for: .systemSettings)) != nil {
            settings.migratedForSettingsCardReadability()
        } else {
            .factorySystemSettings()
        }
        if let root,
           let data = try? Data(contentsOf: root.appending(path: "assets/boot/installer-screens.infinityui")),
           let decoded = try? JSONDecoder().decode(InstallerStudioDocument.self, from: data)
        {
            let migrated = decoded.migratedForInstallerRuntimeParity()
            if (try? TemplateValidator.validate(migrated)) != nil {
                document = migrated
                status = migrated == decoded
                    ? "Loaded repository templates"
                    : "Loaded and upgraded installer runtime layers"
            } else {
                document = .factoryDefault()
            }
        } else {
            document = .factoryDefault()
        }
        kitObservation = uiKit.objectWillChange.sink { [weak self] in
            self?.objectWillChange.send()
        }
        savedProject = ProjectSnapshot(desktop: desktopDocument, login: loginDocument, boot: bootDocument, installation: document, configuration: configurationDocument,
            systemSettings: settingsDocument)
        if let root {
            for collection in ScreenCollection.allCases {
                let name = switch collection {
                case .login: "login-screens"
                case .desktop: "desktop-screens"
                case .boot: "boot-screens"
                case .installation: "installer-screens"
                case .configuration: "configuration-screens"
                case .systemSettings: "settings-screens"
                }
                let url = root.appending(path: "assets/boot/\(name).infinityui")
                guard FileManager.default.fileExists(atPath: url.path) else { continue }
                do {
                    var loaded = try JSONDecoder().decode(InstallerStudioDocument.self, from: Data(contentsOf: url))
                    if collection == .installation { loaded = loaded.migratedForInstallerRuntimeParity() }
                    if collection == .systemSettings { loaded = loaded.migratedForSettingsCardReadability() }
                    try TemplateValidator.validate(loaded, for: collection)
                } catch {
                    sourceLoadIssues[collection] = "\(name).infinityui could not be loaded. Import a valid replacement or repair the source and reopen Studio. The original file is protected: \(error.localizedDescription)"
                }
            }
            if !sourceLoadIssues.isEmpty {
                validationIssues = sourceLoadIssues.values.sorted()
                status = "Source loading failed; saving is protected"
            }
        }
    }

    // ------------------------=
    // FUNC: selectScreenCollection
    // DESC: Activates one sidebar screen collection and selects its requested screen.
    // ------------------=
    func selectScreenCollection(_ collection: ScreenCollection, screen id: Int = 1) {
        editingMenu = false
        editingUIKit = false
        selectedCollection = collection
        if !nativePreviewState.isSupported(in: collection) { nativePreviewState = .normal }
        selectScreen(id)
    }

    // ------------------------=
    // FUNC: selectUIKit
    // DESC: Opens the shared OS kit without discarding pending template edits or their selections.
    // ------------------=
    func selectUIKit(category: String = "All Elements") {
        editingMenu = false
        pendingNativeSelectionReveal = nil
        editingUIKit = true
        inlineEditorElementID = nil
        uiKit.selectedCategory = category
        status = "UIKit · \(category)"
    }

    // ------------------------=
    // FUNC: selectScreen
    // DESC: Selects one installer screen and clears an element selection that does not belong to it.
    // ------------------=
    func selectScreen(_ id: Int) {
        editingMenu = false
        pendingNativeSelectionReveal = nil
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
        pendingNativeSelectionReveal = nil
        if selectedElementID != id {
            inlineEditorElementID = nil
        }
        selectedElementIDs = id.map { [$0] } ?? []
        selectedElementID = id
        if selectedCollection == .systemSettings, let element = selectedElement,
           element.role.isSettingsChild, let parent = element.parentRowID,
           let row = selectedScreen?.elements.filter({ $0.role == .metadata }).firstIndex(where: { $0.id == parent }) {
            nativePreviewSimulation.enabled = true
            nativePreviewSimulation.expandedRow = row
            pendingNativeSelectionReveal = element.id
        }
    }

    // ------------------------=
    // FUNC: applyNativeSelectionReveal
    // DESC: Reveals a newly selected Settings action once using the production layout's bounded scroll target, never estimating detail placement in Studio.
    // ------------------=
    @discardableResult func applyNativeSelectionReveal(_ bounds: [NativePreviewBounds], screen: Int,
                                                       expandedRow: Int) -> Bool {
        guard !editingUIKit, selectedCollection == .systemSettings, selectedScreenID == screen,
              nativePreviewSimulation.enabled, nativePreviewSimulation.expandedRow == expandedRow,
              let pending = pendingNativeSelectionReveal, pending == selectedElementID,
              let element = selectedElement, element.role.isSettingsChild,
              let parent = element.parentRowID,
              selectedScreen?.elements.filter({ $0.role == .metadata }).firstIndex(where: { $0.id == parent }) == expandedRow,
              let geometry = bounds.first(where: { $0.id == pending }), geometry.isBounded,
              let parentBounds = geometry.parentBounds, parentBounds.width > 0, parentBounds.height > 0,
              let viewport = geometry.viewportBounds, viewport.width > 0, viewport.height > 0,
              let target = geometry.revealScroll, geometry.maximumScroll != nil else { return false }
        pendingNativeSelectionReveal = nil
        guard nativePreviewSimulation.scroll != target else { return false }
        nativePreviewSimulation.scroll = target
        return true
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
        let factory = switch selectedCollection {
        case .installation: InstallerStudioDocument.factoryDefault()
        case .configuration: InstallerStudioDocument.factoryConfiguration()
        case .systemSettings: InstallerStudioDocument.factorySystemSettings()
        case .boot: InstallerStudioDocument.factoryBoot()
        case .login: InstallerStudioDocument.factoryLogin()
        case .desktop: InstallerStudioDocument.factoryDesktop()
        }
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
        if selectedCollection == .desktop { removeMenuScreen(); return }
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
        guard selectedCollection != .desktop, !fromOffsets.isEmpty else { return }
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
        guard selectedCollection != .desktop else { return }
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
        var candidate = location.element
        mutation(&candidate)
        let screenID = activeDocument.screens[location.screen].id
        let originalRequired = selectedCollection.requiredRoleCount(
            location.element.role,
            screenID: screenID
        )
        let originalCount = activeDocument.screens[location.screen].elements.filter {
            $0.role == location.element.role
        }.count
        let candidateCount = activeDocument.screens[location.screen].elements.filter {
            $0.role == candidate.role
        }.count
        let candidateMaximum = selectedCollection.maximumRoleCount(candidate.role, screenID: screenID)
        let removesRequiredRole = originalRequired > 0 && originalCount <= originalRequired
            && (candidate.role != location.element.role
                || (!location.element.hidden && candidate.hidden
                    && candidate.role != .liveDetails && !candidate.role.isSettingsChild))
        let exceedsCandidateMaximum = candidate.role != location.element.role
            && candidateMaximum.map { candidateCount >= $0 } == true
        if removesRequiredRole {
            refuseStructuralChange(
                "\(location.element.role.title) is required by InfinityOS and cannot be removed, hidden, or reassigned."
            )
            return
        }
        if exceedsCandidateMaximum {
            refuseStructuralChange(
                "\(candidate.role.title) is singular runtime structure and already exists on this screen."
            )
            return
        }
        var changedScreen = activeDocument.screens[location.screen]
        changedScreen.elements[location.elementIndex] = candidate
        do { try TemplateValidator.validateSettingsActions(changedScreen, collection: selectedCollection) }
        catch { refuseStructuralChange(String(describing: error)); return }
        recordUndo()
        activeDocument.screens[location.screen].elements[location.elementIndex] = candidate
        activeDocument.screens[location.screen].elements[location.elementIndex].frame =
            activeDocument.screens[location.screen].elements[location.elementIndex].frame.clamped()
        activeDocument.screens[location.screen].elements[location.elementIndex].crop =
            activeDocument.screens[location.screen].elements[location.elementIndex].crop.clamped()
        status = label
    }

    // ------------------------=
    // FUNC: reparentSelectedAction
    // DESC: Exchanges existing action parents atomically so every required native action remains represented exactly once.
    // ------------------=
    func reparentSelectedAction(_ parent: UUID) {
        reparentSelectedSettingsChild(parent)
    }

    // ------------------------=
    // FUNC: reparentSelectedSettingsChild
    // DESC: Exchanges same-role parent bindings atomically, retaining every required native child and validating its destination.
    // ------------------=
    func reparentSelectedSettingsChild(_ parent: UUID) {
        guard let location = selectedLocation(), !location.element.locked,
              location.element.role.isSettingsChild, let oldParent = location.element.parentRowID,
              parent != oldParent else { return }
        var screen = activeDocument.screens[location.screen]
        if let other = screen.elements.firstIndex(where: { $0.role == location.element.role && $0.parentRowID == parent }) {
            guard !screen.elements[other].locked else { status = "Unlock the other child before exchanging parents"; return }
            screen.elements[other].parentRowID = oldParent
        }
        screen.elements[location.elementIndex].parentRowID = parent
        do { try TemplateValidator.validateSettingsActions(screen, collection: selectedCollection) }
        catch { refuseStructuralChange(String(describing: error)); return }
        recordUndo()
        activeDocument.screens[location.screen] = screen
        selectElement(location.element.id)
        status = "Settings parent bindings exchanged"
    }

    // ------------------------=
    // FUNC: updateSelectedCodeIdentifier
    // DESC: Updates the code-facing element identity while preserving the immutable internal UUID.
    // ------------------=
    func updateSelectedCodeIdentifier(_ identifier: String) {
        guard let location = selectedLocation() else { return }
        let bounded = String(identifier.prefix(127))
        guard activeDocument.screens[location.screen].elements[location.elementIndex].codeIdentifier != bounded else {
            return
        }
        recordUndo()
        activeDocument.screens[location.screen].elements[location.elementIndex].codeIdentifier = bounded
        status = "Element ID updated"
    }

    // ------------------------=
    // FUNC: selectLiveDetails
    // DESC: Opens the screen's runtime text region in the sidebar without creating duplicate live-data layers.
    // ------------------=
    func selectLiveDetails() {
        guard let element = selectedScreen?.elements.first(where: { $0.role == .liveDetails }) else { return }
        selectElement(element.id)
        status = "Live Installer Details selected — move, resize, and style in the sidebar"
    }

    // ------------------------=
    // FUNC: addElement
    // DESC: Adds a real panel, image, text, console, or progress element to the active screen.
    // ------------------=
    func addElement(kind: StudioElementKind) {
        guard kind != .button, let screen = selectedScreenIndex else { return }
        recordUndo()
        let offset = activeDocument.screens[screen].elements.count % 6 * 12
        let role: StudioElementRole = switch kind {
        case .image: .image
        case .text: .body
        case .progressBar: .progressBar
        default: .decoration
        }
        var element = StudioElement.make(
            name: "New \(kind.title)",
            kind: kind,
            role: role,
            frame: CanvasRect(
                x: 300 + offset,
                y: 470 + offset,
                width: kind == .progressBar ? 520 : kind == .text ? 360 : 280,
                height: kind == .progressBar ? 120 : kind == .text ? 70 : 180
            ),
            text: kind == .progressBar
                ? "PREPARING INSTALLATION"
                : kind == .text ? "Editable text" : "",
            zIndex: nextZIndex(in: screen)
        )
        element.codeIdentifier = uniqueCodeIdentifier(element.codeIdentifier, in: screen)
        if kind == .progressBar {
            element.fill = StudioColor(red: 174, green: 219, blue: 247, alpha: 255)
            element.border = StudioColor(red: 53, green: 165, blue: 220, alpha: 255)
            element.cornerRadius = 15
        }
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
    // FUNC: selectScreenBackground
    // DESC: Exposes the screen-owned background in the same inspector used for ordinary images without changing its lock state.
    // ------------------=
    func selectScreenBackground() {
        guard let elements = selectedScreen?.elements,
              let background = elements.first(where: { $0.kind == .image && $0.role == .sceneBackground })
                ?? elements.first(where: { $0.kind == .image && $0.role == .masthead }) else {
            status = "This screen has no background image layer"; return
        }
        selectElement(background.id)
        status = background.locked ? "Unlock the background in Image controls to replace it" : "Choose Replace Image in the inspector"
    }

    // ------------------------=
    // FUNC: importImageAsset
    // DESC: Copies a validated PNG or bitmap into boot assets and creates or updates its canvas layer.
    // ------------------=
    @discardableResult
    func importImageAsset(from source: URL, replacingSelected: Bool = false) throws -> UUID {
        if replacingSelected {
            guard let element = selectedElement, element.kind == .image, !element.locked else {
                throw TemplateValidationIssue.invalidDocument("Select and unlock an image before replacing it")
            }
        }
        guard ["png", "bmp", "jpg", "jpeg", "tif", "tiff"].contains(source.pathExtension.lowercased()),
              let dimensions = UIKitAssetCatalog.dimensions(source),
              dimensions.0 > 0, dimensions.1 > 0,
              dimensions.0 <= 8192, dimensions.1 <= 8192,
              dimensions.0 * dimensions.1 <= 16_777_216,
              let size = try source.resourceValues(forKeys: [.fileSizeKey]).fileSize,
              size <= 64 * 1024 * 1024
        else {
            throw TemplateValidationIssue.invalidDocument("Choose a PNG, BMP, JPEG or TIFF image up to 16 megapixels and 64 MiB")
        }
        _ = try NativeBitmapEncoder.read(source)
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
            // Keep the authored background fit and geometry; replacing artwork must not change its layout.
            activeDocument.screens[location.screen].elements[location.elementIndex].imageFit = location.element.imageFit
            status = "Image replaced"
            projectRoot = root
            return location.element.id
        }
        let area = editableContentFrame(in: screen)
        // X and Y are normalized separately; compensate for the actual widescreen viewport.
        let aspect = CGFloat(dimensions.0) / CGFloat(dimensions.1)
            * CGFloat(canvasViewport.height) / CGFloat(canvasViewport.width)
        var width = 460
        var height = Int((CGFloat(width) / aspect).rounded())
        if height > 600 {
            height = 600
            width = Int((CGFloat(height) * aspect).rounded())
        }
        var element = StudioElement.make(
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
        element.codeIdentifier = uniqueCodeIdentifier(element.codeIdentifier, in: screen)
        element.imageFit = .fit
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
        guard let location = selectedLocation() else { return }
        let required = selectedCollection.requiredRoleCount(
            location.element.role,
            screenID: activeDocument.screens[location.screen].id
        )
        let roleCount = activeDocument.screens[location.screen].elements.filter {
            $0.role == location.element.role
        }.count
        if required > 0 && roleCount <= required {
            refuseStructuralChange(
                "\(location.element.role.title) is required by InfinityOS and cannot be deleted."
            )
            return
        }
        guard !location.element.locked else {
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
        guard let location = selectedLocation() else { return }
        let maximum = selectedCollection.maximumRoleCount(
            location.element.role,
            screenID: activeDocument.screens[location.screen].id
        )
        let roleCount = activeDocument.screens[location.screen].elements.filter {
            $0.role == location.element.role
        }.count
        if maximum.map({ roleCount >= $0 }) == true {
            refuseStructuralChange(
                "\(location.element.role.title) is unique runtime structure and cannot be duplicated."
            )
            return
        }
        guard !location.element.locked else {
            status = "Locked elements cannot be duplicated"
            return
        }
        recordUndo()
        var copy = location.element
        copy.id = UUID()
        copy.name += " Copy"
        copy.codeIdentifier = uniqueCodeIdentifier(copy.codeIdentifier, in: location.screen)
        copy.frame.x += gridSize
        copy.frame.y += gridSize
        copy.frame = copy.frame.clamped()
        copy.zIndex = nextZIndex(in: location.screen)
        activeDocument.screens[location.screen].elements.append(copy)
        selectedElementID = copy.id
        status = "Element duplicated"
    }

    static let elementPasteboardType = NSPasteboard.PasteboardType("org.infinityos.studio.elements.v1")

    // ------------------------=
    // FUNC: copySelectedElements
    // DESC: Copies the complete marquee selection in authored order without changing geometry or the document.
    // ------------------=
    func copySelectedElements(to pasteboard: NSPasteboard = .general) {
        guard !editingUIKit, let screen = selectedScreen else { return }
        let elements = screen.elements.filter { selectedElementIDs.contains($0.id) }
        guard !elements.isEmpty, let data = try? JSONEncoder().encode(elements) else { return }
        pasteboard.clearContents()
        pasteboard.setData(data, forType: Self.elementPasteboardType)
        status = "Copied \(elements.count) elements — paste preserves X/Y"
    }

    // ------------------------=
    // FUNC: pasteElementsInPlace
    // DESC: Atomically pastes fresh layer identities at exact authored coordinates; rejects runtime-role conflicts before mutation.
    // ------------------=
    func pasteElementsInPlace(from pasteboard: NSPasteboard = .general) {
        guard !editingUIKit, let index = selectedScreenIndex,
              let data = pasteboard.data(forType: Self.elementPasteboardType), data.count <= 4_194_304,
              let elements = try? JSONDecoder().decode([StudioElement].self, from: data),
              !elements.isEmpty, elements.count <= 256 else {
            status = "Clipboard does not contain Studio elements"
            return
        }
        let screen = activeDocument.screens[index]
        var additions: [StudioElement] = []
        var identifiers = Set(screen.elements.map(\.codeIdentifier))
        var names = Set(screen.elements.map(\.name))
        for var element in SettingsActionContract.clonedElements(elements) {
            let count = (screen.elements + additions).filter { $0.role == element.role }.count
            if let maximum = selectedCollection.maximumRoleCount(element.role, screenID: screen.id), count >= maximum {
                if element.kind == .image && [.masthead, .sceneBackground, .progressHero].contains(element.role) {
                    // A copied illustration is not a second runtime-owned header/background.
                    // Keep its authored pixels and geometry as an ordinary image layer.
                    if element.imageFit == .legacy {
                        element.imageFit = element.imageUsesAspectFill ? .fill : .fit
                    }
                    element.role = .image
                } else {
                    status = "Cannot paste: \(element.role.title) conflicts with this screen's runtime structure"
                    return
                }
            }
            guard element.frame == element.frame.clamped() else {
                status = "Cannot paste out-of-canvas coordinates without moving the selection"
                return
            }
            var copy = element
            var nameSuffix = 2
            while names.contains(copy.name) {
                copy.name = "\(element.name) \(nameSuffix)"
                nameSuffix += 1
            }
            names.insert(copy.name)
            var suffix = 1
            while identifiers.contains(copy.codeIdentifier) {
                copy.codeIdentifier = "\(element.codeIdentifier).copy\(suffix)"
                suffix += 1
            }
            guard copy.hasValidCodeIdentifier else {
                status = "Cannot paste: shorten the element's code identifier first"
                return
            }
            identifiers.insert(copy.codeIdentifier)
            additions.append(copy)
        }
        var candidate = screen
        candidate.elements.append(contentsOf: additions)
        do { try TemplateValidator.validateSettingsActions(candidate, collection: selectedCollection) }
        catch { status = "Cannot paste: \(error)"; return }
        recordUndo()
        activeDocument.screens[index].elements.append(contentsOf: additions)
        selectedElementIDs = Set(additions.map(\.id))
        selectedElementID = additions.last?.id
        inlineEditorElementID = nil
        status = "Pasted \(additions.count) elements at original X/Y"
    }

    // ------------------------=
    // FUNC: geometryIsEditable
    // DESC: Rejects geometry controls for native responsive dashboard regions whose bounds are owned by the OS layout contract.
    // ------------------=
    func geometryIsEditable(_ element: StudioElement) -> Bool {
        !(selectedCollection == .systemSettings && [7,8].contains(selectedScreenID)
            && [.metadata,.settingsTab,.settingsSummaryCard,.settingsMainCard,.settingsSidebarCard,.settingsArtwork].contains(element.role))
    }

    var canReorderSelectedLayer: Bool {
        selectedCollection != .systemSettings && selectedElement?.locked == false
    }

    // ------------------------=
    // FUNC: moveLayer
    // DESC: Moves an editable element forward or backward in deterministic z-order.
    // ------------------=
    func moveLayer(_ delta: Int) {
        guard let id = selectedElementID, delta != 0 else { return }
        reorderLayer(id, action: delta > 0 ? .forward : .backward)
    }

    // ------------------------=
    // FUNC: moveVisibleLayers
    // DESC: Applies a Layers-list drag in front-to-back order, preserving filtered-out layers and one undo transaction.
    // ------------------=
    func moveVisibleLayers(_ source: IndexSet, to destination: Int, visible: [UUID]) {
        guard selectedCollection != .systemSettings, let screen = selectedScreenIndex,
              !source.isEmpty, source.allSatisfy({ visible.indices.contains($0) }),
              (0...visible.count).contains(destination), Set(visible).count == visible.count else { return }
        let current = activeDocument.screens[screen].elements.sorted {
            $0.zIndex == $1.zIndex ? $0.id.uuidString > $1.id.uuidString : $0.zIndex > $1.zIndex
        }
        guard visible == current.filter({ visible.contains($0.id) }).map(\.id) else { return }
        let moved = source.map { visible[$0] }
        guard current.filter({ moved.contains($0.id) }).allSatisfy({ !$0.locked }) else { return }
        var order = visible
        order.move(fromOffsets: source, toOffset: destination)
        guard order != visible, let first = order.firstIndex(where: { moved.contains($0) }) else { return }
        let next = order.dropFirst(first + moved.count).first
        let previous = order.prefix(first).last
        var layers = current.filter { !moved.contains($0.id) }
        let insertion: Int
        if let next, let index = layers.firstIndex(where: { $0.id == next }) { insertion = index }
        else if let previous, let index = layers.firstIndex(where: { $0.id == previous }) { insertion = index + 1 }
        else { insertion = layers.count }
        layers.insert(contentsOf: current.filter { moved.contains($0.id) }, at: insertion)
        recordUndo()
        let ranks = Dictionary(uniqueKeysWithValues: layers.reversed().enumerated().map { ($0.element.id, $0.offset) })
        for index in activeDocument.screens[screen].elements.indices {
            activeDocument.screens[screen].elements[index].zIndex = ranks[activeDocument.screens[screen].elements[index].id]!
        }
        selectedElementIDs = Set(moved)
        selectedElementID = moved.first
        status = "Layer order updated"
    }

    // ------------------------=
    // FUNC: reorderedLayers
    // DESC: Resolves positional stack moves, including tied and sparse z indices.
    // ------------------=
    private func reorderedLayers(_ id: UUID, action: LayerOrderAction) -> [StudioElement]? {
        guard selectedCollection != .systemSettings, let screen = selectedScreen,
              screen.elements.first(where: { $0.id == id })?.locked == false else { return nil }
        var layers = screen.elements.sorted {
            $0.zIndex == $1.zIndex ? $0.id.uuidString < $1.id.uuidString : $0.zIndex < $1.zIndex
        }
        guard let source = layers.firstIndex(where: { $0.id == id }) else { return nil }
        let destination: Int
        switch action {
        case .front: destination = layers.count - 1
        case .back: destination = 0
        case .forward: destination = min(source + 1, layers.count - 1)
        case .backward: destination = max(source - 1, 0)
        }
        guard source != destination else { return nil }
        layers.insert(layers.remove(at: source), at: destination)
        return layers
    }

    // ------------------------=
    // FUNC: canReorderLayer
    // DESC: Disables locked, OS-owned and already-at-boundary actions.
    // ------------------=
    func canReorderLayer(_ id: UUID, action: LayerOrderAction) -> Bool {
        reorderedLayers(id, action: action) != nil
    }

    // ------------------------=
    // FUNC: reorderLayer
    // DESC: Records one undoable stack move for the clicked element, preserving other relative order.
    // ------------------=
    func reorderLayer(_ id: UUID, action: LayerOrderAction) {
        guard let layers = reorderedLayers(id, action: action), let screen = selectedScreenIndex else { return }
        recordUndo()
        let ranks = Dictionary(uniqueKeysWithValues: layers.enumerated().map { ($0.element.id, $0.offset) })
        for index in activeDocument.screens[screen].elements.indices {
            let elementID = activeDocument.screens[screen].elements[index].id
            activeDocument.screens[screen].elements[index].zIndex = ranks[elementID]!
        }
        selectElement(id)
        status = action.rawValue
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
        let movable = screen.elements.filter { selectedElementIDs.contains($0.id) && !$0.locked && geometryIsEditable($0) }
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
              gestureFrames.count == 1, let origin = gestureFrames[location.element.id],
              canvasScale.width > 0, canvasScale.height > 0
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
            CanvasRect(x: location.element.role == .settingsSliderThumb ? 0 : left,
                y: top, width: right - left, height: bottom - top).clamped()
    }

    // ------------------------=
    // FUNC: endGesture
    // DESC: Commits a completed canvas gesture as one undo operation.
    // ------------------=
    func endGesture() {
        if let baseline = gestureBaseline,
           baseline.installation != document
            || baseline.boot != bootDocument
            || baseline.configuration != configurationDocument
            || baseline.systemSettings != settingsDocument
            || baseline.login != loginDocument
            || baseline.desktop != desktopDocument
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
            selectedElementIDs.contains($0.id) && !$0.locked && geometryIsEditable($0) ? ($0.id, $0.frame) : nil
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
        if editingUIKit { uiKit.undo(); return }
        guard let previous = undoStack.popLast() else { return }
        redoStack.append(projectSnapshot())
        document = previous.installation
        configurationDocument = previous.configuration
        bootDocument = previous.boot
        settingsDocument = previous.systemSettings
        loginDocument = previous.login
        desktopDocument = previous.desktop
        if !activeScreens.contains(where: { $0.id == selectedScreenID }) {
            selectedScreenID = activeScreens.first?.id ?? 1
        }
        selectedElementID = nil
        inlineEditorElementID = nil
        status = "Undo"
    }

    // ------------------------=
    // FUNC: redo
    // DESC: Restores the next complete template document.
    // ------------------=
    func redo() {
        if editingUIKit { uiKit.redo(); return }
        guard let next = redoStack.popLast() else { return }
        undoStack.append(projectSnapshot())
        document = next.installation
        configurationDocument = next.configuration
        bootDocument = next.boot
        settingsDocument = next.systemSettings
        loginDocument = next.login
        desktopDocument = next.desktop
        if !activeScreens.contains(where: { $0.id == selectedScreenID }) {
            selectedScreenID = activeScreens.first?.id ?? 1
        }
        selectedElementID = nil
        inlineEditorElementID = nil
        status = "Redo"
    }

    // ------------------------=
    // FUNC: refuseStructuralChange
    // DESC: Publishes an immediate native alert when an edit would invalidate required runtime structure.
    // ------------------=
    private func refuseStructuralChange(_ message: String) {
        validationIssues = [message]
        status = "Required structure protected"
    }

    // ------------------------=
    // FUNC: validate
    // DESC: Runs authoritative template validation and publishes actionable issues.
    // ------------------=
    @discardableResult
    func validate() -> Bool {
        guard sourceLoadIssues.isEmpty else {
            validationIssues = sourceLoadIssues.values.sorted()
            status = "Resolve source loading errors before saving all sections"
            return false
        }
        do {
            try uiKit.validate()
            try TemplateValidator.validate(bootDocument, for: .boot)
            try TemplateValidator.validate(document, for: .installation)
            try TemplateValidator.validate(configurationDocument, for: .configuration)
            try TemplateValidator.validate(settingsDocument, for: .systemSettings)
            try TemplateValidator.validate(loginDocument, for: .login)
            try TemplateValidator.validate(desktopDocument, for: .desktop)
            validationIssues = []
            status = "Validated UIKit and all wizard/settings sections"
            return true
        } catch {
            validationIssues = [String(describing: error)]
            status = "Validation failed"
            return false
        }
    }

    // ------------------------=
    // FUNC: save
    // DESC: Saves the active source/runtime pair with rollback if either file cannot be published.
    // ------------------=
    func save() {
        if editingUIKit {
            saveUIKit()
            return
        }
        if let issue = sourceLoadIssues[selectedCollection] {
            validationIssues = [issue]; status = "Original source protected"; return
        }
        do {
            try uiKit.validate()
            try TemplateValidator.validate(activeDocument, for: selectedCollection)
        } catch {
            validationIssues = [String(describing: error)]
            status = "Validation failed"
            return
        }
        guard let root = projectRoot ?? chooseProjectRoot() else {
            status = "Save cancelled"
            return
        }
        do {
            try StudioFileTransaction.commit(prepareSave(activeDocument, collection: selectedCollection, root: root), root: root)
            projectRoot = root
            switch selectedCollection {
            case .boot: savedProject?.boot = bootDocument
            case .login: savedProject?.login = loginDocument
            case .desktop: savedProject?.desktop = desktopDocument
            case .installation: savedProject?.installation = document
            case .configuration: savedProject?.configuration = configurationDocument
            case .systemSettings: savedProject?.systemSettings = settingsDocument
            }
            validationIssues = []
            status = "Saved \(selectedCollection.title) source and OS runtime"
        } catch {
            validationIssues = [error.localizedDescription]
            status = "Save failed"
        }
    }

    // ------------------------=
    // FUNC: prepareSave
    // DESC: Compiles and round-trips a collection without publishing any files or changing canvas selection.
    // ------------------=
    private func prepareSave(_ source: InstallerStudioDocument, collection: ScreenCollection, root: URL) throws -> [StudioFileWrite] {
        try TemplateValidator.validate(source, for: collection)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        let runtime = try RuntimeTemplateCodec.encode(source, assetRoot: root, collection: collection, theme: uiKit.document)
        guard try RuntimeTemplateCodec.decode(runtime, collection: collection) == source.runtimeProjection(theme: uiKit.document) else {
            throw TemplateValidationIssue.invalidDocument("Generated \(collection.title) runtime artifact failed round-trip verification")
        }
        let baseName = switch collection {
        case .boot: "boot-screens"
        case .login: "login-screens"
        case .desktop: "desktop-screens"
        case .installation: "installer-screens"
        case .configuration: "configuration-screens"
        case .systemSettings: "settings-screens"
        }
        let directory = root.appending(path: "assets/boot")
        return [StudioFileWrite(url: directory.appending(path: "\(baseName).infinityui"), data: try encoder.encode(source)),
                StudioFileWrite(url: directory.appending(path: "\(baseName).iuit"), data: runtime)]
    }

    // ------------------------=
    // FUNC: saveUIKit
    // DESC: Persists the shared UIKit source and reports validation failures through the common Studio alert.
    // ------------------=
    private func saveUIKit() {
        do {
            try uiKit.save()
            validationIssues = []
            status = uiKit.status
        } catch {
            validationIssues = [error.localizedDescription]
            status = "UIKit save failed"
        }
    }

    // ------------------------=
    // FUNC: saveAll
    // DESC: Prepares every collection and UIKit before a rollback-capable save without changing the active editing context.
    // ------------------=
    @discardableResult func saveAll() -> Bool {
        guard validate() else { return false }
        guard let root = projectRoot else {
            validationIssues = ["Choose an InfinityOS project before saving all sections"]
            return false
        }
        do {
            var writes: [StudioFileWrite] = []
            for (section, source) in [(ScreenCollection.desktop, desktopDocument), (.login, loginDocument), (.boot, bootDocument), (.installation, document), (.configuration, configurationDocument), (.systemSettings, settingsDocument)] {
                writes += try prepareSave(source, collection: section, root: root)
            }
            writes += try uiKit.prepareSave()
            try StudioFileTransaction.commit(writes, root: root)
            uiKit.didSave()
            savedProject = ProjectSnapshot(desktop: desktopDocument, login: loginDocument, boot: bootDocument, installation: document, configuration: configurationDocument,
                systemSettings: settingsDocument)
            validationIssues = []
            status = "Saved UIKit and all wizard/settings sections"
            return true
        } catch {
            validationIssues = [error.localizedDescription]
            status = "Save failed; review recovery details"
            return false
        }
    }

    // ------------------------=
    // FUNC: saveAndUpdateISO
    // DESC: Saves validated edits before asynchronously updating an explicitly selected installer ISO.
    // ------------------=
    func saveAndUpdateISO() async {
        guard !isoUpdater.isRunning, isoUpdater.selectedISO != nil, projectRoot != nil else { return }
        guard saveAll() else { return }
        await isoUpdater.start()
    }

    // ------------------------=
    // FUNC: resolveClose
    // DESC: Keeps drafts open on cancellation or failed saving and never abandons an active ISO publication.
    // ------------------=
    func resolveClose(_ choice: StudioCloseChoice) -> Bool {
        guard !isoUpdater.isRunning else { return false }
        switch choice {
        case .cancel: return false
        case .discard: return true
        case .save: return saveAll()
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
            try importDocument(from: url)
        } catch {
            validationIssues = [String(describing: error)]
            status = "Import failed"
        }
    }

    // ------------------------=
    // FUNC: importDocument
    // DESC: Imports a validated source document or restores embedded runtime artwork before changing the active draft.
    // ------------------=
    func importDocument(from url: URL) throws {
        guard let size = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize,
              size <= 512 * 1024 * 1024 else {
            throw TemplateValidationIssue.invalidDocument("Template exceeds the import byte limit")
        }
        let data = try Data(contentsOf: url)
        var imported: InstallerStudioDocument
        var writes: [StudioFileWrite] = []
        if url.pathExtension.lowercased() == "iuit" {
            let package = try RuntimeTemplateCodec.decodePackage(data, collection: selectedCollection)
            imported = package.document
            if !package.assets.isEmpty {
                guard let root = projectRoot else { throw UIKitIssue.invalid("Open a project before importing embedded artwork") }
                var paths: [String: String] = [:]
                var staged = Set<String>()
                for (name, bytes) in package.assets {
                    let hash = SHA256.hash(data: bytes).map { String(format: "%02x", $0) }.joined()
                    let relative = "installer-assets/imported-\(hash).bmp"
                    let destination = try UIKitAssetCatalog.resolvedURL("assets/boot/" + relative, root: root)
                    paths[name] = relative
                    if FileManager.default.fileExists(atPath: destination.path) {
                        guard try Data(contentsOf: destination) == bytes else { throw UIKitIssue.invalid("Imported artwork identity conflicts with an existing file") }
                    } else if staged.insert(relative).inserted {
                        writes.append(StudioFileWrite(url: destination, data: bytes))
                    }
                }
                for screen in imported.screens.indices {
                    for element in imported.screens[screen].elements.indices {
                        if let path = paths[imported.screens[screen].elements[element].imageAsset] {
                            imported.screens[screen].elements[element].imageAsset = path
                        }
                    }
                }
            }
        } else {
            imported = try JSONDecoder().decode(InstallerStudioDocument.self, from: data)
        }
        if selectedCollection == .installation { imported = imported.migratedForInstallerRuntimeParity() }
        if selectedCollection == .systemSettings { imported = imported.migratedForSettingsCardReadability() }
        try TemplateValidator.validate(imported, for: selectedCollection)
        if let root = projectRoot { try StudioFileTransaction.commit(writes, root: root) }
        recordUndo()
        activeDocument = imported
        sourceLoadIssues.removeValue(forKey: selectedCollection)
        selectedScreenID = 1; selectedElementID = nil; inlineEditorElementID = nil
        validationIssues = []
        status = "Imported \(url.lastPathComponent)"
    }

    // ------------------------=
    // FUNC: exportDocument
    // DESC: Exports an editable source and exact native template into a new repository-local folder without saving or mutating the draft.
    // ------------------=
    @discardableResult func exportDocument(to parent: URL) throws -> URL {
        guard let root = projectRoot else { throw UIKitIssue.invalid("Open a project before exporting") }
        let folder = parent.appending(path: "\(selectedCollection.rawValue)-\(UUID().uuidString)")
        let writes = try prepareSave(activeDocument, collection: selectedCollection, root: root).map {
            StudioFileWrite(url: folder.appending(path: $0.url.lastPathComponent), data: $0.data)
        }
        try StudioFileTransaction.commit(writes, root: root)
        return folder
    }

    // ------------------------=
    // FUNC: chooseTemplateExport
    // DESC: Selects a managed export folder while keeping runtime images embedded and original source files unchanged.
    // ------------------=
    func chooseTemplateExport() {
        guard let root = projectRoot else { return }
        let panel = NSOpenPanel()
        panel.canChooseFiles = false; panel.canChooseDirectories = true
        panel.canCreateDirectories = true; panel.allowsMultipleSelection = false
        panel.directoryURL = root.appending(path: "builds")
        panel.message = "Choose a folder inside the project. A new folder will contain the editable source and native template with embedded custom artwork."
        panel.prompt = "Export"
        guard panel.runModal() == .OK, let folder = panel.url else { return }
        do {
            let exported = try exportDocument(to: folder)
            validationIssues = []; status = "Exported templates to \(exported.path)"
        } catch { validationIssues = [error.localizedDescription]; status = "Export failed" }
    }

    // ------------------------=
    // FUNC: resetScreen
    // DESC: Restores only the active screen from the factory template as an undoable action.
    // ------------------=
    func resetScreen() {
        let factoryDocument = switch selectedCollection {
        case .desktop: InstallerStudioDocument.factoryDesktop()
        case .installation: InstallerStudioDocument.factoryDefault()
        case .configuration: InstallerStudioDocument.factoryConfiguration()
        case .systemSettings: InstallerStudioDocument.factorySystemSettings()
        case .boot: InstallerStudioDocument.factoryBoot()
        case .login: InstallerStudioDocument.factoryLogin()
        }
        guard let index = selectedScreenIndex,
              let factory = factoryDocument.screens.first(where: { $0.id == selectedScreenID })
        else { return }
        recordUndo()
        activeDocument.screens[index] = factory
        selectedElementID = nil
        inlineEditorElementID = nil
        status = "Screen reset"
    }

    // ------------------------=
    // FUNC: chooseImage
    // DESC: Presents the native raster picker and routes validated artwork into the repository-managed asset directory.
    // ------------------=
    private func chooseImage(replacingSelected: Bool) {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = ["png", "bmp", "jpeg", "tiff"].compactMap {
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
            if activeDocument.screens[screen].elements[index].role != .settingsSliderThumb { frame.x += dx }
            frame.y += dy
            activeDocument.screens[screen].elements[index].frame = frame
        }
    }

    // ------------------------=
    // FUNC: clonedElements
    // DESC: Copies a screen's element collection with fresh stable identities.
    // ------------------=
    private func clonedElements(_ elements: [StudioElement]) -> [StudioElement] {
        SettingsActionContract.clonedElements(elements)
    }

    // ------------------------=
    // FUNC: uniqueCodeIdentifier
    // DESC: Produces the next available dot-qualified identifier for a newly created layer.
    // ------------------=
    private func uniqueCodeIdentifier(_ preferred: String, in screen: Int) -> String {
        let existing = Set(activeDocument.screens[screen].elements.map(\.codeIdentifier))
        guard existing.contains(preferred) else { return preferred }
        var ordinal = 2
        while existing.contains("\(preferred).\(ordinal)") {
            ordinal += 1
        }
        return "\(preferred).\(ordinal)"
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
    func recordUndo() {
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
        ProjectSnapshot(
            desktop: desktopDocument,
            login: loginDocument,
            boot: bootDocument,
            installation: document,
            configuration: configurationDocument,
            systemSettings: settingsDocument
        )
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
