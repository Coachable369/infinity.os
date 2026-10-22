import AppKit
import XCTest
@testable import InfinityInstallerStudio

final class SettingsChildTests: XCTestCase {
    // ------------------------=
    // FUNC: testMigrationAddsStableCompleteChildrenWithoutRewritingCustomizations
    // DESC: Verifies forty independently bound description/surface triplets and exactly one timeout track/thumb pair with stable identities.
    // ------------------=
    func testMigrationAddsStableCompleteChildrenWithoutRewritingCustomizations() throws {
        var legacy = InstallerStudioDocument.factorySystemSettings()
        for screen in legacy.screens.indices {
            legacy.screens[screen].elements.removeAll { SettingsChildContract.roles.contains($0.role) }
        }
        let migrated = legacy.migratedForSettingsChildren()
        XCTAssertEqual(migrated, legacy.migratedForSettingsChildren())
        XCTAssertEqual(migrated, migrated.migratedForSettingsChildren())
        for (role, count) in zip(SettingsChildContract.roles, [40, 40, 40, 1, 1]) {
            XCTAssertEqual(migrated.screens.flatMap(\.elements).filter { $0.role == role }.count, count)
        }
        for (before, after) in zip(legacy.screens, migrated.screens) {
            XCTAssertEqual(before.elements, after.elements.filter { !SettingsChildContract.roles.contains($0.role) })
            let rows = after.elements.filter { $0.role == .metadata }
            for child in after.elements.filter({ SettingsChildContract.roles.contains($0.role) }) {
                let parent = try XCTUnwrap(child.parentRowID)
                let row = try XCTUnwrap(rows.firstIndex { $0.id == parent })
                XCTAssertTrue(SettingsChildContract.rows(role: child.role, screenID: after.id).contains(row))
                XCTAssertEqual(child.id, SettingsChildContract.childID(parent: parent, role: child.role))
            }
        }
        var customized = migrated
        let index = try XCTUnwrap(customized.screens[0].elements.firstIndex { $0.role == .settingsSummaryDescription })
        customized.screens[0].elements[index].text = "My custom device description"
        customized.screens[0].elements[index].fontSize = 21
        customized.screens[0].elements[index].fill.alpha = 173
        XCTAssertEqual(customized.migratedForSettingsChildren(), customized)
        try TemplateValidator.validate(customized, for: .systemSettings)
        let encoded = try RuntimeTemplateCodec.encode(customized, collection: .systemSettings)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(encoded, collection: .systemSettings), customized)
    }

    // ------------------------=
    // FUNC: testChildContractsRejectInvalidParentsKindsDuplicatesAndThumbPosition
    // DESC: Rejects cross-screen and forbidden row ownership, duplicate children, mismatched kinds and value-owned thumb X before export.
    // ------------------=
    func testChildContractsRejectInvalidParentsKindsDuplicatesAndThumbPosition() throws {
        let source = InstallerStudioDocument.factorySystemSettings()
        let index = try XCTUnwrap(source.screens[4].elements.firstIndex { $0.role == .settingsSliderThumb })
        let rows = source.screens[4].elements.filter { $0.role == .metadata }
        let mutations: [(inout StudioElement) -> Void] = [
            { $0.frame.x = 1 }, { $0.parentRowID = rows[0].id }, { $0.parentRowID = nil },
            { $0.parentRowID = source.screens[0].elements.first { $0.role == .metadata }!.id },
            { $0.kind = .text }, { $0.actionCaptionMode = .literal }, { $0.captionColor = .cyan }
        ]
        for mutation in mutations {
            var invalid = source; mutation(&invalid.screens[4].elements[index])
            XCTAssertThrowsError(try RuntimeTemplateCodec.encode(invalid, collection: .systemSettings))
        }
        var invalid = source
        var duplicate = invalid.screens[4].elements[index]; duplicate.id = UUID(); duplicate.codeIdentifier += "Copy"
        invalid.screens[4].elements.append(duplicate)
        XCTAssertThrowsError(try RuntimeTemplateCodec.encode(invalid, collection: .systemSettings))
        invalid = source; invalid.screens[4].elements.remove(at: index)
        XCTAssertThrowsError(try RuntimeTemplateCodec.encode(invalid, collection: .systemSettings))
        invalid = source
        let description = try XCTUnwrap(invalid.screens[4].elements.firstIndex { $0.role == .settingsDetailDescription })
        invalid.screens[4].elements[description].kind = .button
        XCTAssertThrowsError(try RuntimeTemplateCodec.encode(invalid, collection: .systemSettings))
    }

    // ------------------------=
    // FUNC: testThumbGesturesRetainNativeHorizontalOwnership
    // DESC: Exercises actual authoring gestures, nudges and inverse native scale while keeping thumb X at zero and allowing vertical/size edits.
    // ------------------=
    @MainActor func testThumbGesturesRetainNativeHorizontalOwnership() throws {
        let store = TemplateStore(); store.settingsDocument = .factorySystemSettings(); store.snapEnabled = false
        store.selectScreenCollection(.systemSettings, screen: 5)
        let thumb = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .settingsSliderThumb })
        store.selectElement(thumb.id)
        XCTAssertEqual(store.nativePreviewSimulation.expandedRow, 3)
        let bounds = NativePreviewBounds(id: thumb.id, x: 135, y: 420, width: 32, height: 48,
            parentBounds: NativePreviewRect(x: 135, y: 314, width: 800, height: 240))
        XCTAssertEqual(NativeCanvasMapping.editingScale(element: thumb, console: nil, window: nil,
            viewport: .compact, scale: 1), .zero)
        let scale = NativeCanvasMapping.editingScale(element: thumb, console: nil, window: nil,
            viewport: .compact, scale: 0.5, bounds: bounds)
        XCTAssertEqual(scale, CGSize(width: 0.4, height: 0.12))
        store.beginGesture(elementID: thumb.id)
        store.moveSelected(translation: CGSize(width: 40, height: 12), canvasScale: scale); store.endGesture()
        XCTAssertEqual(store.selectedElement?.frame.x, 0)
        XCTAssertEqual(store.selectedElement?.frame.y, 540)
        store.nudgeSelected(dx: 10, dy: 1, byGrid: false)
        XCTAssertEqual(store.selectedElement?.frame.x, 0)
        XCTAssertEqual(store.selectedElement?.frame.y, 541)
        store.beginGesture(elementID: thumb.id)
        store.resizeSelected(handle: .right, translation: CGSize(width: 8, height: 0), canvasScale: scale); store.endGesture()
        XCTAssertEqual(store.selectedElement?.frame.width, 60)
        XCTAssertEqual(store.selectedElement?.frame.x, 0)
        let unchanged = store.settingsDocument
        store.updateSelected { $0.frame.x = 99 }
        XCTAssertEqual(store.settingsDocument, unchanged)
        try TemplateValidator.validate(store.settingsDocument, for: .systemSettings)
    }

    // ------------------------=
    // FUNC: testDescriptionParentExchangeAndHiddenPointerLayerRemainEditable
    // DESC: Preserves all required bindings during parent swaps and exposes the intentionally hidden pointer explanation as an editable native layer.
    // ------------------=
    @MainActor func testDescriptionParentExchangeAndHiddenPointerLayerRemainEditable() throws {
        let store = TemplateStore(); store.settingsDocument = .factorySystemSettings()
        store.selectScreenCollection(.systemSettings, screen: 1)
        let descriptions = try XCTUnwrap(store.selectedScreen).elements.filter { $0.role == .settingsDetailDescription }
        store.selectElement(descriptions[0].id)
        store.reparentSelectedSettingsChild(try XCTUnwrap(descriptions[1].parentRowID))
        XCTAssertEqual(store.selectedElement?.parentRowID, descriptions[1].parentRowID)
        XCTAssertEqual(store.selectedScreen?.elements.first { $0.id == descriptions[1].id }?.parentRowID, descriptions[0].parentRowID)
        store.deleteSelected(); store.duplicateSelected()
        XCTAssertEqual(store.selectedScreen?.elements.filter { $0.role == .settingsDetailDescription }.count, 5)
        store.selectScreenCollection(.systemSettings, screen: 6)
        let rows = try XCTUnwrap(store.selectedScreen).elements.filter { $0.role == .metadata }
        let pointer = try XCTUnwrap(store.selectedScreen?.elements.first { $0.role == .settingsDetailDescription && $0.parentRowID == rows[2].id })
        XCTAssertTrue(pointer.hidden)
        store.selectElement(pointer.id)
        XCTAssertEqual(store.selectedElementID, pointer.id)
        XCTAssertEqual(store.nativePreviewSimulation.expandedRow, 2)
        store.updateSelected { $0.text = "Pointer appearance"; $0.frame.y = 965; $0.frame.height = 30; $0.hidden = false }
        XCTAssertEqual(store.selectedElement?.hidden, false)
        XCTAssertEqual(store.selectedElement?.frame.y, 965)
        try TemplateValidator.validate(store.settingsDocument, for: .systemSettings)
    }

    // ------------------------=
    // FUNC: testParentGraphRoundTripRetainsEveryChildAndAuthoredDescription
    // DESC: Remaps copied native parent graphs and verifies actual JSON import plus native-byte round trips preserve all child links and authoring state.
    // ------------------=
    @MainActor func testParentGraphRoundTripRetainsEveryChildAndAuthoredDescription() throws {
        var document = InstallerStudioDocument.factorySystemSettings()
        document.screens[4].elements = SettingsActionContract.clonedElements(document.screens[4].elements)
        let index = try XCTUnwrap(document.screens[4].elements.firstIndex { $0.role == .settingsSliderTrack })
        document.screens[4].elements[index].surfaceMaterial = .glass
        document.screens[4].elements[index].surfaceCorners = .rounded
        document.screens[4].elements[index].cornerRadius = 12
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let directory = root.appending(path: "build/tmp/settings-child-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let path = directory.appending(path: "children.infinityui")
        try JSONEncoder().encode(document).write(to: path, options: .atomic)
        let store = TemplateStore(); store.selectScreenCollection(.systemSettings, screen: 5)
        try store.importDocument(from: path)
        XCTAssertEqual(store.settingsDocument, document)
        let native = try RuntimeTemplateCodec.encode(store.settingsDocument, collection: .systemSettings)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(native, collection: .systemSettings), document)
        let rows = document.screens[4].elements.filter { $0.role == .metadata }
        XCTAssertEqual(document.screens[4].elements[index].parentRowID, rows[3].id)
    }
}
