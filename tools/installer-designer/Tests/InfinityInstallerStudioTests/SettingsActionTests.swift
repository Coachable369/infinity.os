import AppKit
import XCTest
@testable import InfinityInstallerStudio

enum SettingsActionFixtures {
    // ------------------------=
    // FUNC: legacyBytes
    // DESC: Constructs actual older-format fixtures by removing v8 extension records and unsupported action children, preserving all older field bytes.
    // ------------------=
    static func legacyBytes(_ document: InstallerStudioDocument, version: UInt8,
                            collection: ScreenCollection? = nil) throws -> Data {
        let bytes = [UInt8](try RuntimeTemplateCodec.encode(document, collection: collection))
        var cursor = 8
        var output = Array(bytes.prefix(8)); output[4] = version; output[5] = 0
        for _ in document.screens {
            let start = cursor
            cursor += 1
            let titleCount = Int(bytes[cursor]); cursor += 1 + titleCount
            output += bytes[start..<cursor]
            let count = Int(bytes[cursor]) | Int(bytes[cursor + 1]) << 8; cursor += 2
            var records: [[UInt8]] = []
            for _ in 0..<count {
                let recordStart = cursor
                cursor += 16
                let codeCount = Int(bytes[cursor]); cursor += 1 + codeCount
                let role = bytes[cursor + 1]
                let flagsOffset = cursor + 3 - recordStart
                cursor += 26
                let nameCount = Int(bytes[cursor]); cursor += 1 + nameCount
                let textCount = Int(bytes[cursor]) | Int(bytes[cursor + 1]) << 8; cursor += 2 + textCount
                let imageCount = Int(bytes[cursor]); cursor += 1 + imageCount
                cursor += 4
                var record = Array(bytes[recordStart..<cursor])
                if version == 6 { record[flagsOffset] &= 3 }
                if version >= 8 { record += bytes[cursor..<(cursor + 21)] }
                if role < (version >= 8 ? 34 : 33) { records.append(record) }
                cursor += 21
            }
            output += [UInt8(truncatingIfNeeded: records.count), UInt8(truncatingIfNeeded: records.count >> 8)]
            for record in records { output += record }
        }
        output += bytes[cursor...]
        return Data(output)
    }
}

final class SettingsActionTests: XCTestCase {
    // ------------------------=
    // FUNC: testDeterministicMigrationPreservesExistingLayersAndCustomActions
    // DESC: Adds exactly nine stable action children without changing legacy user-authored layers or subsequent customized action records.
    // ------------------=
    func testDeterministicMigrationPreservesExistingLayersAndCustomActions() throws {
        var legacy = InstallerStudioDocument.factorySystemSettings()
        for screen in legacy.screens.indices { legacy.screens[screen].elements.removeAll { $0.role == .settingsRowAction } }
        legacy.screens[0].elements[0].opacity = 63
        let migrated = legacy.migratedForSettingsActions()
        XCTAssertEqual(migrated.screens.flatMap(\.elements).filter { $0.role == .settingsRowAction }.count, 9)
        XCTAssertEqual(migrated, legacy.migratedForSettingsActions())
        XCTAssertEqual(migrated, migrated.migratedForSettingsActions())
        for (before, after) in zip(legacy.screens, migrated.screens) {
            XCTAssertEqual(before.elements, after.elements.filter { $0.role != .settingsRowAction })
            let rows = after.elements.filter { $0.role == .metadata }
            for action in after.elements.filter({ $0.role == .settingsRowAction }) {
                let parent = try XCTUnwrap(action.parentRowID)
                XCTAssertEqual(action.id, SettingsActionContract.actionID(parent: parent))
                XCTAssertTrue(SettingsActionContract.rows(screenID: after.id).contains(try XCTUnwrap(rows.firstIndex { $0.id == parent })))
                XCTAssertEqual(action.frame, CanvasRect(x: 32, y: 520, width: 420, height: 360))
            }
        }
        var customized = migrated
        let index = try XCTUnwrap(customized.screens[0].elements.firstIndex { $0.role == .settingsRowAction })
        customized.screens[0].elements[index].frame.x = 300
        customized.screens[0].elements[index].surfaceMaterial = .flat
        XCTAssertEqual(customized.migratedForSettingsActions(), customized)
        try TemplateValidator.validate(customized, for: .systemSettings)
    }

    // ------------------------=
    // FUNC: testV9RoundTripAndRealV6V7V8Migration
    // DESC: Retains parent identities, captions and styles in JSON and native bytes while reading valid older templates without v8 record extensions.
    // ------------------=
    func testV9RoundTripAndRealV6V7V8Migration() throws {
        var document = InstallerStudioDocument.factorySystemSettings()
        let action = try XCTUnwrap(document.screens[3].elements.firstIndex { $0.role == .settingsRowAction })
        document.screens[3].elements[action].actionCaptionMode = .literal
        document.screens[3].elements[action].text = "Local intelligence policy"
        document.screens[3].elements[action].surfaceMaterial = .flat
        document.screens[3].elements[action].surfaceCorners = .rounded
        document.screens[3].elements[action].captionColor = StudioColor(red: 231, green: 173, blue: 120, alpha: 142)
        document.screens[3].elements[action].frame = CanvasRect(x: 400, y: 500, width: 480, height: 350)
        let bytes = try RuntimeTemplateCodec.encode(document, collection: .systemSettings)
        XCTAssertEqual(Array(bytes[4...5]), [10, 0])
        XCTAssertEqual(try RuntimeTemplateCodec.decode(bytes, collection: .systemSettings), document)
        XCTAssertEqual(try JSONDecoder().decode(InstallerStudioDocument.self, from: JSONEncoder().encode(document)), document)
        let defaults = InstallerStudioDocument.factorySystemSettings()
        for version in [UInt8(6), UInt8(7), UInt8(8)] {
            let legacy = try SettingsActionFixtures.legacyBytes(defaults, version: version, collection: .systemSettings)
            XCTAssertEqual(try RuntimeTemplateCodec.decode(legacy, collection: .systemSettings), defaults)
        }
    }

    // ------------------------=
    // FUNC: testInvalidParentContractsFailClosed
    // DESC: Rejects dangling, cross-screen, duplicate, wrong-role and disallowed-row references as well as non-action binding fields.
    // ------------------=
    func testInvalidParentContractsFailClosed() throws {
        let source = InstallerStudioDocument.factorySystemSettings()
        let index = try XCTUnwrap(source.screens[3].elements.firstIndex { $0.role == .settingsRowAction })
        let rows = source.screens[3].elements.filter { $0.role == .metadata }
        for invalidParent in [UUID(), source.screens[0].elements.first { $0.role == .metadata }!.id,
                              source.screens[3].elements[0].id, rows[3].id, rows[1].id] {
            var invalid = source; invalid.screens[3].elements[index].parentRowID = invalidParent
            XCTAssertThrowsError(try RuntimeTemplateCodec.encode(invalid, collection: .systemSettings))
        }
        var invalid = source
        invalid.screens[3].elements[index].parentRowID = nil
        XCTAssertThrowsError(try RuntimeTemplateCodec.encode(invalid, collection: .systemSettings))
        invalid = source; invalid.screens[3].elements[index].kind = .panel
        XCTAssertThrowsError(try RuntimeTemplateCodec.encode(invalid, collection: .systemSettings))
        invalid = source; invalid.screens[3].elements[0].parentRowID = rows[0].id
        XCTAssertThrowsError(try RuntimeTemplateCodec.encode(invalid, collection: .systemSettings))
        invalid = source; invalid.screens[3].elements[0].actionCaptionMode = .literal
        XCTAssertThrowsError(try RuntimeTemplateCodec.encode(invalid, collection: .systemSettings))
        invalid = source; invalid.screens[3].elements[0].captionColor = .cyan
        XCTAssertThrowsError(try RuntimeTemplateCodec.encode(invalid, collection: .systemSettings))
        invalid = source; invalid.screens[3].elements.remove(at: index)
        XCTAssertThrowsError(try RuntimeTemplateCodec.encode(invalid, collection: .systemSettings))
    }

    // ------------------------=
    // FUNC: testLegacyFileImportMigratesWithoutLosingExistingParentIdentities
    // DESC: Exercises the actual document import entrypoint with saved legacy JSON and verifies atomic preservation of authoring and action-parent ownership.
    // ------------------=
    @MainActor func testLegacyFileImportMigratesWithoutLosingExistingParentIdentities() throws {
        var legacy = InstallerStudioDocument.factorySystemSettings()
        for screen in legacy.screens.indices { legacy.screens[screen].elements.removeAll { $0.role == .settingsRowAction } }
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let directory = root.appending(path: "builds/.studio-work/tmp/settings-actions-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let path = directory.appending(path: "legacy.infinityui")
        try JSONEncoder().encode(legacy).write(to: path, options: .atomic)
        let store = TemplateStore(); store.selectScreenCollection(.systemSettings, screen: 1)
        try store.importDocument(from: path)
        XCTAssertEqual(store.settingsDocument, legacy.migratedForSettingsCardReadability())
        let parents = legacy.screens[0].elements.filter { $0.role == .metadata }.map(\.id)
        XCTAssertEqual(store.settingsDocument.screens[0].elements.first { $0.role == .settingsRowAction }?.parentRowID, parents[0])
        let stable = store.settingsDocument
        try store.importDocument(from: path)
        XCTAssertEqual(store.settingsDocument, stable)
    }

    // ------------------------=
    // FUNC: testGraphCloneAndImportRetainParentOwnership
    // DESC: Remaps copied parent-child graphs together and preserves those identities through native encoding and document import.
    // ------------------=
    @MainActor func testGraphCloneAndImportRetainParentOwnership() throws {
        let store = TemplateStore(); store.settingsDocument = .factorySystemSettings()
        store.selectScreenCollection(.systemSettings, screen: 1)
        let screen = try XCTUnwrap(store.selectedScreen)
        let action = try XCTUnwrap(screen.elements.first { $0.role == .settingsRowAction })
        let parent = try XCTUnwrap(screen.elements.first { $0.id == action.parentRowID })
        let copied = SettingsActionContract.clonedElements([parent, action])
        XCTAssertNotEqual(copied[0].id, parent.id)
        XCTAssertNotEqual(copied[1].id, action.id)
        XCTAssertEqual(copied[1].parentRowID, copied[0].id)
        XCTAssertEqual(copied.map(\.frame), [parent.frame, action.frame])
        var clonedDocument = store.settingsDocument
        clonedDocument.screens[0].elements = SettingsActionContract.clonedElements(screen.elements)
        let native = try RuntimeTemplateCodec.encode(clonedDocument, collection: .systemSettings)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(native, collection: .systemSettings), clonedDocument)
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let directory = root.appending(path: "builds/.studio-work/tmp/settings-action-graph-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let path = directory.appending(path: "graph.infinityui")
        try JSONEncoder().encode(clonedDocument).write(to: path, options: .atomic)
        try store.importDocument(from: path)
        let restoredScreen = try XCTUnwrap(store.selectedScreen)
        let restoredParent = try XCTUnwrap(restoredScreen.elements.first { $0.role == .metadata })
        let restoredAction = try XCTUnwrap(restoredScreen.elements.first { $0.role == .settingsRowAction })
        XCTAssertEqual(restoredAction.parentRowID, restoredParent.id)
        XCTAssertNotEqual(restoredParent.id, parent.id)
        XCTAssertEqual(restoredScreen.elements.map(\.role), screen.elements.map(\.role))
        try TemplateValidator.validate(store.settingsDocument, for: .systemSettings)

    }

    // ------------------------=
    // FUNC: testActionSelectionProtectionAndAtomicParentExchange
    // DESC: Expands selected actions, preserves all required native bindings and provides undo for explicit parent exchanges and appearance edits.
    // ------------------=
    @MainActor func testActionSelectionProtectionAndAtomicParentExchange() throws {
        let store = TemplateStore(); store.settingsDocument = .factorySystemSettings()
        store.selectScreenCollection(.systemSettings, screen: 4)
        let screen = try XCTUnwrap(store.selectedScreen)
        let actions = screen.elements.filter { $0.role == .settingsRowAction }
        store.selectElement(actions[0].id)
        XCTAssertTrue(store.nativePreviewSimulation.enabled)
        XCTAssertEqual(store.nativePreviewSimulation.expandedRow, 0)
        store.deleteSelected(); store.duplicateSelected()
        XCTAssertEqual(store.selectedScreen, screen)
        store.reparentSelectedAction(try XCTUnwrap(actions[1].parentRowID))
        XCTAssertEqual(store.selectedElement?.parentRowID, actions[1].parentRowID)
        XCTAssertEqual(store.selectedScreen?.elements.first { $0.id == actions[1].id }?.parentRowID, actions[0].parentRowID)
        XCTAssertEqual(store.nativePreviewSimulation.expandedRow, 1)
        try TemplateValidator.validate(store.settingsDocument, for: .systemSettings)
        store.undo(); XCTAssertEqual(store.selectedScreen, screen)
        store.selectElement(actions[0].id)
        store.updateSelected { $0.hidden = true; $0.opacity = 50; $0.surfaceMaterial = .matte }
        XCTAssertEqual(store.selectedElement?.hidden, true)
        XCTAssertEqual(store.selectedElement?.opacity, 50)
    }

    // ------------------------=
    // FUNC: testSelectionRevealUsesBoundedNativeMetadataOnce
    // DESC: Applies only the selected action's current native reveal target and preserves later manual scrolling, rejecting stale or malformed geometry.
    // ------------------=
    @MainActor func testSelectionRevealUsesBoundedNativeMetadataOnce() throws {
        let store = TemplateStore(); store.settingsDocument = .factorySystemSettings()
        store.selectScreenCollection(.systemSettings, screen: 4)
        let action = try XCTUnwrap(store.selectedScreen?.elements.last { $0.role == .settingsRowAction })
        store.nativePreviewSimulation.scroll = 999
        store.selectElement(action.id)
        XCTAssertEqual(store.nativePreviewSimulation.expandedRow, 7)
        let geometry = NativePreviewBounds(id: action.id, x: 120, y: -30, width: 180, height: 46,
            parentBounds: NativePreviewRect(x: 100, y: -96, width: 500, height: 128),
            viewportBounds: NativePreviewRect(x: 100, y: 200, width: 600, height: 400),
            revealScroll: 317, maximumScroll: 420)
        XCTAssertFalse(store.applyNativeSelectionReveal([geometry], screen: 1, expandedRow: 7))
        XCTAssertFalse(store.applyNativeSelectionReveal([geometry], screen: 4, expandedRow: 0))
        XCTAssertFalse(store.applyNativeSelectionReveal([], screen: 4, expandedRow: 7))
        var malformed = geometry; malformed.revealScroll = 421
        XCTAssertFalse(store.applyNativeSelectionReveal([malformed], screen: 4, expandedRow: 7))
        malformed = geometry; malformed.maximumScroll = nil
        XCTAssertFalse(store.applyNativeSelectionReveal([malformed], screen: 4, expandedRow: 7))
        XCTAssertEqual(store.nativePreviewSimulation.scroll, 999)
        XCTAssertTrue(store.applyNativeSelectionReveal([geometry], screen: 4, expandedRow: 7))
        XCTAssertEqual(store.nativePreviewSimulation.scroll, 317)
        store.nativePreviewSimulation.scroll = 280
        XCTAssertFalse(store.applyNativeSelectionReveal([geometry], screen: 4, expandedRow: 7))
        XCTAssertEqual(store.nativePreviewSimulation.scroll, 280)
        store.selectElement(action.id)
        XCTAssertTrue(store.applyNativeSelectionReveal([geometry], screen: 4, expandedRow: 7))
        store.selectElement(action.id); store.selectElement(nil)
        store.nativePreviewSimulation.scroll = 44
        XCTAssertFalse(store.applyNativeSelectionReveal([geometry], screen: 4, expandedRow: 7))
        XCTAssertEqual(store.nativePreviewSimulation.scroll, 44)
    }

    // ------------------------=
    // FUNC: testSettingsRejectsUnsupportedTransitionState
    // DESC: Keeps the preview state selector aligned with real native scene capabilities when changing collections.
    // ------------------=
    @MainActor func testSettingsRejectsUnsupportedTransitionState() {
        let store = TemplateStore()
        XCTAssertTrue(NativePreviewControlState.transition.isSupported(in: .configuration))
        XCTAssertFalse(NativePreviewControlState.transition.isSupported(in: .systemSettings))
        for state in [NativePreviewControlState.normal, .focused, .pressed] {
            XCTAssertTrue(state.isSupported(in: .systemSettings))
        }
        store.nativePreviewState = .transition
        store.selectScreenCollection(.systemSettings)
        XCTAssertEqual(store.nativePreviewState, .normal)
        store.nativePreviewState = .pressed
        store.selectScreenCollection(.systemSettings, screen: 4)
        XCTAssertEqual(store.nativePreviewState, .pressed)
    }

    // ------------------------=
    // FUNC: testActionInteractionClipPreservesFullEditingGeometry
    // DESC: Clips selection and resize hit areas to both production parent and viewport without translating the original native bounds or detail-local edit scale.
    // ------------------=
    func testActionInteractionClipPreservesFullEditingGeometry() throws {
        let action = try XCTUnwrap(InstallerStudioDocument.factorySystemSettings().screens[0].elements.first { $0.role == .settingsRowAction })
        let bounds = NativePreviewBounds(id: action.id, x: 50, y: 50, width: 100, height: 80,
            parentBounds: NativePreviewRect(x: 75, y: 40, width: 100, height: 100),
            viewportBounds: NativePreviewRect(x: 60, y: 70, width: 120, height: 50))
        let clip = NativeCanvasMapping.actionInteractionClip(bounds: bounds, scale: 0.5)
        XCTAssertEqual(clip, CGRect(x: 12.5, y: 10, width: 37.5, height: 25))
        XCTAssertFalse(clip.contains(CGPoint(x: 10, y: 20)))
        XCTAssertFalse(clip.contains(CGPoint(x: 20, y: 5)))
        XCTAssertTrue(clip.contains(CGPoint(x: 20, y: 20)))
        XCTAssertEqual(NativeCanvasMapping.editingScale(element: action, console: nil, window: nil,
            viewport: .compact, scale: 0.5, bounds: bounds), CGSize(width: 0.05, height: 0.05))
        var unavailable = bounds; unavailable.viewportBounds = nil
        XCTAssertEqual(NativeCanvasMapping.actionInteractionClip(bounds: unavailable, scale: 0.5), .zero)
        unavailable = bounds; unavailable.parentBounds = NativePreviewRect(x: 400, y: 500, width: 20, height: 20)
        XCTAssertEqual(NativeCanvasMapping.actionInteractionClip(bounds: unavailable, scale: 0.5), .zero)
        XCTAssertEqual(NativeCanvasMapping.actionInteractionClip(bounds: bounds, scale: 0), .zero)
        XCTAssertEqual(bounds.x, 50); XCTAssertEqual(bounds.y, 50)
        XCTAssertEqual(bounds.width, 100); XCTAssertEqual(bounds.height, 80)
    }

    // ------------------------=
    // FUNC: testDetailLocalInverseRequiresActualNativeParentBounds
    // DESC: Uses only production-exported parent geometry for authoring distances, rejecting unavailable bounds instead of substituting a viewport scale.
    // ------------------=
    @MainActor func testDetailLocalInverseRequiresActualNativeParentBounds() throws {
        let store = TemplateStore(); store.settingsDocument = .factorySystemSettings(); store.snapEnabled = false
        store.selectScreenCollection(.systemSettings, screen: 1)
        let action = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .settingsRowAction })
        let bounds = NativePreviewBounds(id: action.id, x: 45, y: 72, width: 336, height: 72,
            parentBounds: NativePreviewRect(x: 17, y: 24, width: 800, height: 240))
        XCTAssertEqual(NativeCanvasMapping.editingScale(element: action, console: nil, window: nil,
            viewport: .studio, scale: 1), .zero)
        let scale = NativeCanvasMapping.editingScale(element: action, console: nil, window: nil,
            viewport: .studio, scale: 0.5, bounds: bounds)
        XCTAssertEqual(scale.width, 0.4, accuracy: 0.0001)
        XCTAssertEqual(scale.height, 0.12, accuracy: 0.0001)
        store.selectElement(action.id); store.beginGesture(elementID: action.id)
        store.moveSelected(translation: CGSize(width: 40, height: 12), canvasScale: scale); store.endGesture()
        XCTAssertEqual(store.selectedElement?.frame.x, action.frame.x + 100)
        XCTAssertEqual(store.selectedElement?.frame.y, action.frame.y + 100)
        let decoded = try JSONDecoder().decode(NativePreviewBounds.self, from: JSONSerialization.data(withJSONObject: [
            "id": action.id.uuidString, "x": 45, "y": 72, "width": 336, "height": 72,
            "parentBounds": ["x": 17, "y": 24, "width": 800, "height": 240]]))
        XCTAssertEqual(decoded, bounds)
    }
}
