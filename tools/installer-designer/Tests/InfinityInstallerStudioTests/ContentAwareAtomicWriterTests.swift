import Darwin
import Foundation
import XCTest
@testable import InfinityInstallerStudio

final class ContentAwareAtomicWriterTests: XCTestCase {
    // ------------------------=
    // FUNC: workspace
    // DESC: Creates a bounded repository-local directory for real filesystem output tests.
    // ------------------=
    private func workspace() throws -> URL {
        let directory = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .appending(path: "builds/.studio-work/tmp/content-aware-writer-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: directory) }
        return directory
    }

    // ------------------------=
    // FUNC: metadata
    // DESC: Reads observable file identity and modification time rather than inferring changes from compiler messages.
    // ------------------=
    private func metadata(_ url: URL) throws -> (date: Date, inode: UInt64) {
        let attributes = try FileManager.default.attributesOfItem(atPath: url.path)
        return (try XCTUnwrap(attributes[.modificationDate] as? Date),
                try XCTUnwrap(attributes[.systemFileNumber] as? NSNumber).uint64Value)
    }

    // ------------------------=
    // FUNC: testIdenticalBytesPreserveModificationDateAndInode
    // DESC: Verifies both empty and nonempty unchanged compiled files are never replaced or touched.
    // ------------------=
    func testIdenticalBytesPreserveModificationDateAndInode() throws {
        let directory = try workspace()
        for (index, data) in [Data(), Data([0, 1, 127, 255])].enumerated() {
            let target = directory.appending(path: "\(index).iuit")
            try data.write(to: target)
            try FileManager.default.setAttributes([.modificationDate: Date(timeIntervalSince1970: 1_000)], ofItemAtPath: target.path)
            let before = try metadata(target)
            XCTAssertFalse(try ContentAwareAtomicWriter.write(data, to: target))
            let after = try metadata(target)
            XCTAssertEqual(after.date, before.date)
            XCTAssertEqual(after.inode, before.inode)
            XCTAssertEqual(try Data(contentsOf: target), data)
        }
    }

    // ------------------------=
    // FUNC: testChangedBytesReplaceOutputAtomically
    // DESC: Checks changed payloads replace the complete output and advance its modification date and inode.
    // ------------------=
    func testChangedBytesReplaceOutputAtomically() throws {
        let target = try workspace().appending(path: "changed.iuit")
        try Data([1, 2, 3]).write(to: target)
        try FileManager.default.setAttributes([.modificationDate: Date(timeIntervalSince1970: 1_000)], ofItemAtPath: target.path)
        let before = try metadata(target)
        let replacement = Data([4, 5, 6])
        XCTAssertTrue(try ContentAwareAtomicWriter.write(replacement, to: target))
        let after = try metadata(target)
        XCTAssertNotEqual(after.date, before.date)
        XCTAssertNotEqual(after.inode, before.inode)
        XCTAssertEqual(try Data(contentsOf: target), replacement)
    }

    // ------------------------=
    // FUNC: testMissingOutputIsCreated
    // DESC: Verifies a compiler's first invocation creates complete output and a repeat becomes a no-op.
    // ------------------=
    func testMissingOutputIsCreated() throws {
        let target = try workspace().appending(path: "new.iuit")
        let data = Data([19, 37, 128, 255])
        XCTAssertFalse(FileManager.default.fileExists(atPath: target.path))
        XCTAssertTrue(try ContentAwareAtomicWriter.write(data, to: target))
        XCTAssertEqual(try Data(contentsOf: target), data)
        XCTAssertFalse(try ContentAwareAtomicWriter.write(data, to: target))
    }

    // ------------------------=
    // FUNC: testFailedAtomicWritePreservesExistingOutput
    // DESC: Forces a real directory-permission publication failure and checks original bytes and file identity survive.
    // ------------------=
    func testFailedAtomicWritePreservesExistingOutput() throws {
        guard geteuid() != 0 else { throw XCTSkip("Root bypasses directory write permissions") }
        let directory = try workspace(), target = directory.appending(path: "protected.iuit")
        let original = Data([21, 42, 84])
        try original.write(to: target)
        let before = try metadata(target)
        try FileManager.default.setAttributes([.posixPermissions: 0o500], ofItemAtPath: directory.path)
        defer { try? FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: directory.path) }
        XCTAssertThrowsError(try ContentAwareAtomicWriter.write(Data([85, 43, 22]), to: target))
        XCTAssertEqual(try Data(contentsOf: target), original)
        let after = try metadata(target)
        XCTAssertEqual(after.date, before.date)
        XCTAssertEqual(after.inode, before.inode)
        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: directory.path), ["protected.iuit"])
    }
}
