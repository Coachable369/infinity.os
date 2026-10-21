import XCTest
@testable import InfinityInstallerStudio

final class SettingsMigrationTests: XCTestCase {
    // ------------------------=
    // FUNC: legacyDocument
    // DESC: Recreates the prior row text contract independently of the updated factory defaults.
    // ------------------=
    private func legacyDocument() -> InstallerStudioDocument {
        var document = InstallerStudioDocument.factorySystemSettings()
        for screen in document.screens.indices where ![7, 8].contains(document.screens[screen].id) {
            let rows = document.screens[screen].elements.filter { $0.role == .metadata }
            for role in [StudioElementRole.settingsRowLabel, .settingsRowValue, .settingsDisclosure] {
                let indices = document.screens[screen].elements.indices.filter { document.screens[screen].elements[$0].role == role }
                for (row, index) in zip(rows, indices) {
                    let x = role == .settingsRowLabel ? 16 : role == .settingsRowValue ? 250 : 470
                    let width = role == .settingsRowLabel ? 220 : role == .settingsRowValue ? 210 : 20
                    document.screens[screen].elements[index].frame = CanvasRect(
                        x: row.frame.x + x, y: row.frame.y + 13, width: width, height: 24)
                    document.screens[screen].elements[index].fontSize = role == .settingsDisclosure ? 17 : 15
                }
            }
        }
        return document
    }

    // ------------------------=
    // FUNC: testLegacyRowsMigrateToNativeSlotsAndRemainIdempotent
    // DESC: Verifies precise native geometry and readable type across every ordinary section without replacing stable element identities.
    // ------------------=
    func testLegacyRowsMigrateToNativeSlotsAndRemainIdempotent() throws {
        let legacy = legacyDocument()
        let migrated = legacy.migratedForSettingsCardReadability()
        XCTAssertNotEqual(migrated, legacy)
        XCTAssertEqual(migrated.migratedForSettingsCardReadability(), migrated)
        for (before, after) in zip(legacy.screens, migrated.screens) {
            XCTAssertEqual(before.elements.map(\.id), after.elements.map(\.id))
            XCTAssertEqual(before.elements.map(\.codeIdentifier), after.elements.map(\.codeIdentifier))
            XCTAssertEqual(before.elements.map(\.text), after.elements.map(\.text))
            XCTAssertEqual(before.elements.map(\.fill), after.elements.map(\.fill))
            if [7, 8].contains(before.id) { XCTAssertEqual(before, after); continue }
            let rows = after.elements.filter { $0.role == .metadata }
            for role in [StudioElementRole.settingsRowLabel, .settingsRowValue, .settingsDisclosure] {
                for (row, child) in zip(rows, after.elements.filter({ $0.role == role })) {
                    let expected: CanvasRect
                    switch role {
                    case .settingsRowLabel:
                        expected = CanvasRect(x: row.frame.x + 68, y: row.frame.y + 8, width: 234, height: 28)
                    case .settingsRowValue:
                        expected = CanvasRect(x: row.frame.x + 314, y: row.frame.y + 22, width: 150, height: 28)
                    default:
                        expected = CanvasRect(x: row.frame.x + 470, y: row.frame.y + 20, width: 20, height: 32)
                    }
                    XCTAssertEqual(child.frame, expected)
                    XCTAssertEqual(child.fontSize, role == .settingsDisclosure ? 17 : 24)
                }
            }
        }
        try TemplateValidator.validate(migrated, for: .systemSettings)
        XCTAssertEqual(try RuntimeTemplateCodec.decode(RuntimeTemplateCodec.encode(migrated, collection: .systemSettings),
            collection: .systemSettings), migrated)
    }

    // ------------------------=
    // FUNC: testMigrationPreservesCustomizedGeometryAndAppearance
    // DESC: Keeps deliberate user offsets and font choices while carrying custom colors, text and materials through safe legacy upgrades.
    // ------------------=
    func testMigrationPreservesCustomizedGeometryAndAppearance() throws {
        var document = legacyDocument()
        let label = try XCTUnwrap(document.screens[0].elements.firstIndex { $0.role == .settingsRowLabel })
        let value = try XCTUnwrap(document.screens[0].elements.firstIndex { $0.role == .settingsRowValue })
        let disclosure = try XCTUnwrap(document.screens[0].elements.firstIndex { $0.role == .settingsDisclosure })
        document.screens[0].elements[label].frame.x += 3
        document.screens[0].elements[value].fontSize = 19
        document.screens[0].elements[disclosure].fill = StudioColor(red: 83, green: 47, blue: 198, alpha: 121)
        document.screens[0].elements[disclosure].text = "Custom"
        document.screens[0].elements[disclosure].surfaceMaterial = .matte
        let migrated = document.migratedForSettingsCardReadability()
        XCTAssertEqual(migrated.screens[0].elements[label], document.screens[0].elements[label])
        XCTAssertEqual(migrated.screens[0].elements[value], document.screens[0].elements[value])
        var expected = document.screens[0].elements[disclosure]
        expected.frame.y += 7; expected.frame.height = 32
        XCTAssertEqual(migrated.screens[0].elements[disclosure], expected)
    }

    // ------------------------=
    // FUNC: testResizedRowsUseNativeColumnProportions
    // DESC: Adapts untouched legacy child fields to an authored wider row without reverting the row's own size or style.
    // ------------------=
    func testResizedRowsUseNativeColumnProportions() throws {
        var document = legacyDocument()
        let row = try XCTUnwrap(document.screens[0].elements.firstIndex { $0.role == .metadata })
        let label = try XCTUnwrap(document.screens[0].elements.firstIndex { $0.role == .settingsRowLabel })
        let value = try XCTUnwrap(document.screens[0].elements.firstIndex { $0.role == .settingsRowValue })
        document.screens[0].elements[row].frame.width = 600
        let migrated = document.migratedForSettingsCardReadability()
        XCTAssertEqual(migrated.screens[0].elements[row], document.screens[0].elements[row])
        XCTAssertEqual(migrated.screens[0].elements[label].frame.width, 296)
        XCTAssertEqual(migrated.screens[0].elements[value].frame.width, 188)
        XCTAssertEqual(migrated.screens[0].elements[value].frame.x, document.screens[0].elements[row].frame.x + 376)
        let factory = InstallerStudioDocument.factorySystemSettings()
        XCTAssertEqual(factory.migratedForSettingsCardReadability(), factory)
    }
}
