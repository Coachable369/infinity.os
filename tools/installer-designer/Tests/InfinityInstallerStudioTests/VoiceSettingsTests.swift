import XCTest
@testable import InfinityInstallerStudio

final class VoiceSettingsTests: XCTestCase {
    // ------------------------=
    // FUNC: testLegacyVoiceRowMigrationPreservesAuthoredLayers
    // DESC: Migrates a seven-row saved screen once and verifies parent-linked action, serialization and existing layer preservation.
    // ------------------=
    func testLegacyVoiceRowMigrationPreservesAuthoredLayers() throws {
        var legacy = InstallerStudioDocument.factorySystemSettings()
        let rows = legacy.screens[3].elements.filter { $0.role == .metadata }
        let last = try XCTUnwrap(rows.last)
        legacy.screens[3].elements.removeAll {
            $0.id == last.id || $0.parentRowID == last.id ||
            $0.codeIdentifier.hasPrefix("settingsRow.8.")
        }
        let original = legacy.screens[3].elements
        let migrated = legacy.migratedForSettingsActions()
        XCTAssertEqual(migrated, migrated.migratedForSettingsActions())
        XCTAssertEqual(migrated.screens[3].elements.filter { $0.role == .metadata }.count, 8)
        XCTAssertEqual(Array(migrated.screens[3].elements.prefix(original.count)), original)
        let added = try XCTUnwrap(migrated.screens[3].elements.last { $0.role == .metadata })
        XCTAssertEqual(migrated.screens[3].elements.filter { $0.role == .settingsRowAction && $0.parentRowID == added.id }.count, 1)
        try TemplateValidator.validate(migrated, for: .systemSettings)
        let saved = try JSONEncoder().encode(migrated)
        XCTAssertEqual(try JSONDecoder().decode(InstallerStudioDocument.self, from: saved), migrated)
    }
}
