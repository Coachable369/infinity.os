import Foundation
import XCTest
@testable import InfinityInstallerStudio

final class StudioFileTransactionTests: XCTestCase {
    // ------------------------=
    // FUNC: workspace
    // DESC: Creates test scratch inside the repository rather than the host temporary directory.
    // ------------------=
    private func workspace() throws -> URL {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .appending(path: "build/tmp/studio-transaction-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: root) }
        return root
    }

    // ------------------------=
    // FUNC: testPublicationFailureRestoresExistingAndRemovesNewFiles
    // DESC: Injects an I/O failure after publishing two entries and verifies exact originals and absence of partial artifacts.
    // ------------------=
    func testPublicationFailureRestoresExistingAndRemovesNewFiles() throws {
        let root = try workspace()
        let source = root.appending(path: "source.infinityui"), runtime = root.appending(path: "screen.iuit")
        let tokens = root.appending(path: "tokens.bin")
        let original = Data([0, 1, 2, 3])
        try original.write(to: source)
        let writes = [StudioFileWrite(url: source, data: Data([4])), StudioFileWrite(url: runtime, data: Data([5])),
                      StudioFileWrite(url: tokens, data: Data([6]))]
        XCTAssertThrowsError(try StudioFileTransaction.commit(writes, root: root) { index, _ in
            if index == 2 { throw NSError(domain: NSPOSIXErrorDomain, code: 28) }
        })
        XCTAssertEqual(try Data(contentsOf: source), original)
        XCTAssertFalse(FileManager.default.fileExists(atPath: runtime.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: tokens.path))
        try StudioFileTransaction.commit(writes, root: root)
        for write in writes { XCTAssertEqual(try Data(contentsOf: write.url), write.data) }
        XCTAssertTrue(try FileManager.default.contentsOfDirectory(atPath: root.appending(path: "build/tmp").path).isEmpty)
    }

    // ------------------------=
    // FUNC: testInvalidDestinationsRejectBeforePublication
    // DESC: Rejects a directory, repeated target and symlink escape before any preceding valid file changes.
    // ------------------=
    func testInvalidDestinationsRejectBeforePublication() throws {
        let root = try workspace(), outside = try workspace()
        let source = root.appending(path: "source.infinityui"), directory = root.appending(path: "directory")
        let link = root.appending(path: "link")
        try Data([9]).write(to: source)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false)
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: outside)
        for invalid in [directory, source, link.appending(path: "file")] {
            XCTAssertThrowsError(try StudioFileTransaction.commit([
                StudioFileWrite(url: source, data: Data([1])), StudioFileWrite(url: invalid, data: Data([2]))], root: root))
            XCTAssertEqual(try Data(contentsOf: source), Data([9]))
        }
        XCTAssertTrue(try FileManager.default.contentsOfDirectory(atPath: outside.path).isEmpty)
    }

    // ------------------------=
    // FUNC: testNormalizedDestinationIsTheOnlyPathPublished
    // DESC: Publishes through the exact normalized path validated by the transaction, never through a discarded path component.
    // ------------------=
    func testNormalizedDestinationIsTheOnlyPathPublished() throws {
        let root = try workspace()
        let raw = URL(fileURLWithPath: root.path + "/missing/../document.infinityui")
        let normalized = raw.standardizedFileURL
        var destinations: [URL] = []
        try StudioFileTransaction.commit([StudioFileWrite(url: raw, data: Data([7, 8]))], root: root) { _, target in
            destinations.append(target)
        }
        XCTAssertEqual(destinations, [normalized])
        XCTAssertEqual(try Data(contentsOf: normalized), Data([7, 8]))
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appending(path: "missing").path))
    }

    // ------------------------=
    // FUNC: testParentSymlinkIntroducedDuringPublicationCannotEscapeProject
    // DESC: Rechecks confinement after staging and rolls back a preceding file when another destination parent changes into a symlink.
    // ------------------=
    func testParentSymlinkIntroducedDuringPublicationCannotEscapeProject() throws {
        let root = try workspace(), outside = try workspace()
        let original = root.appending(path: "first.infinityui")
        let directory = root.appending(path: "destination")
        let moved = root.appending(path: "old-destination")
        try Data([9]).write(to: original)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false)
        XCTAssertThrowsError(try StudioFileTransaction.commit([
            StudioFileWrite(url: original, data: Data([1])),
            StudioFileWrite(url: directory.appending(path: "second.iuit"), data: Data([2]))], root: root) { index, _ in
                if index == 1 {
                    try FileManager.default.moveItem(at: directory, to: moved)
                    try FileManager.default.createSymbolicLink(at: directory, withDestinationURL: outside)
                }
            })
        XCTAssertEqual(try Data(contentsOf: original), Data([9]))
        XCTAssertTrue(try FileManager.default.contentsOfDirectory(atPath: outside.path).isEmpty)
    }
}
