import XCTest
import CryptoKit
@testable import InfinityInstallerStudio

private actor ISOFixtureRunner: ISOUpdateRunning {
    let chunks: [Data]
    let exitStatus: Int32
    private(set) var requests: [ISOUpdateRequest] = []

    // ------------------------=
    // FUNC: init
    // DESC: Configures an isolated process-boundary fixture without invoking any compiler or modifying an ISO.
    // ------------------=
    init(chunks: [Data], exitStatus: Int32) { self.chunks = chunks; self.exitStatus = exitStatus }

    // ------------------------=
    // FUNC: run
    // DESC: Captures the typed invocation and streams deterministic protocol fragments to exercise the real service state machine.
    // ------------------=
    func run(_ request: ISOUpdateRequest, output: @escaping @Sendable (Data) async -> Void) async throws -> ISOProcessResult {
        requests.append(request)
        for chunk in chunks { await output(chunk) }
        return ISOProcessResult(exitStatus: exitStatus)
    }
}

private actor ISOCancellationRunner: ISOUpdateRunning {
    private(set) var started = false
    private(set) var cancelled = false

    // ------------------------=
    // FUNC: run
    // DESC: Keeps a typed operation pending until cancellation reaches the execution boundary.
    // ------------------=
    func run(_ request: ISOUpdateRequest, output: @escaping @Sendable (Data) async -> Void) async throws -> ISOProcessResult {
        started = true
        do {
            try await Task.sleep(for: .seconds(30))
            return ISOProcessResult(exitStatus: 0)
        } catch {
            cancelled = true
            throw error
        }
    }
}

final class ISOUpdateServiceTests: XCTestCase {
    // ------------------------=
    // FUNC: testArchitectureChangeFollowsCanonicalISOButPreservesCustomSelection
    // DESC: Keeps the common target switch ergonomic without replacing a deliberately selected custom ISO.
    // ------------------=
    @MainActor func testArchitectureChangeFollowsCanonicalISOButPreservesCustomSelection() throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let arm = root.appending(path: "builds/InfinityOS-aarch64.iso")
        let intel = root.appending(path: "builds/InfinityOS-x86_64.iso")
        let custom = root.appending(path: "builds/My custom image.iso")
        for path in [arm, intel, custom] { try Data([73, 83, 79]).write(to: path) }
        let service = ISOUpdateService(projectRoot: root)
        XCTAssertEqual(service.selectedISO, arm)
        service.target = .x86_64
        XCTAssertEqual(service.selectedISO, intel)
        service.target = .aarch64
        XCTAssertEqual(service.selectedISO, arm)
        service.selectedISO = custom
        service.target = .x86_64
        XCTAssertEqual(service.selectedISO, custom)
    }

    // ------------------------=
    // FUNC: testRequestUsesBuildAuthorityWithoutShellInterpolation
    // DESC: Verifies spaces and shell metacharacters remain one inert file argument in the authorized build-kit command.
    // ------------------=
    func testRequestUsesBuildAuthorityWithoutShellInterpolation() throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let iso = root.appending(path: "builds/Test image;not-a-command.iso")
        try Data([1, 2, 3]).write(to: iso)
        let request = try ISOUpdateRequest.make(projectRoot: root, target: .aarch64, isoURL: iso)
        XCTAssertEqual(request.executableURL, root.appending(path: "build-kit"))
        XCTAssertEqual(request.arguments, ["update-iso", "--target", "aarch64", "--iso", iso.path,
                                           "--cancel-file", request.cancellationURL.path])
        XCTAssertEqual(request.cancellationURL.deletingLastPathComponent().path, root.appending(path: "build/tmp").path)
        XCTAssertEqual(request.workingDirectoryURL, root)
        XCTAssertThrowsError(try ISOUpdateRequest.make(projectRoot: root, target: .aarch64,
            isoURL: root.appending(path: "builds/missing.iso")))
        let escaped = root.appending(path: "builds/escaped.iso")
        try FileManager.default.createSymbolicLink(at: escaped, withDestinationURL: root.deletingLastPathComponent())
        XCTAssertThrowsError(try ISOUpdateRequest.make(projectRoot: root, target: .aarch64, isoURL: escaped))
    }

    // ------------------------=
    // FUNC: testStructuredSuccessAcrossSplitChunks
    // DESC: Accepts only the matching completed artifact after all progress and result bytes cross the asynchronous stream boundary.
    // ------------------=
    @MainActor func testStructuredSuccessAcrossSplitChunks() async throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let iso = root.appending(path: "builds/InfinityOS-aarch64.iso")
        let bytes = Data([73, 83, 79])
        try bytes.write(to: iso)
        let hash = SHA256.hash(data: bytes).map { String(format: "%02x", $0) }.joined()
        let manifest = root.appending(path: "builds/manifests/ui-update-test.json")
        let event = try JSONSerialization.data(withJSONObject: [
            "event": "result", "status": "passed", "target": "aarch64", "iso": iso.path,
            "sha256": hash, "manifest": manifest.path,
        ])
        try FileManager.default.createDirectory(at: manifest.deletingLastPathComponent(), withIntermediateDirectories: true)
        try event.write(to: manifest)
        try "\(hash)  \(iso.lastPathComponent)\n".write(to: iso.appendingPathExtension("sha256"), atomically: true, encoding: .utf8)
        let progress = try JSONSerialization.data(withJSONObject: ["event": "progress", "phase": "Packaging", "progress": 0.5])
        let runner = ISOFixtureRunner(chunks: [progress + Data([10]), event.prefix(7), event.dropFirst(7), Data([10])], exitStatus: 0)
        let service = ISOUpdateService(projectRoot: root, runner: runner)
        await service.start()
        XCTAssertFalse(service.isRunning)
        XCTAssertNil(service.errorMessage)
        XCTAssertEqual(service.progress, 1)
        XCTAssertEqual(service.lastResult?.isoURL, iso)
        XCTAssertEqual(service.lastResult?.sha256, hash)
        XCTAssertEqual(service.lastResult?.target, .aarch64)
        let requests = await runner.requests
        XCTAssertEqual(requests.count, 1)
        XCTAssertEqual(requests[0].isoURL, iso)
    }

    // ------------------------=
    // FUNC: testMissingVerificationMetadataIsNotAccepted
    // DESC: Rejects a typed success event when the promised manifest and checksum artifacts were never published.
    // ------------------=
    @MainActor func testMissingVerificationMetadataIsNotAccepted() async throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let iso = root.appending(path: "builds/InfinityOS-aarch64.iso")
        try Data([73, 83, 79]).write(to: iso)
        let event = try JSONSerialization.data(withJSONObject: ["event": "result", "status": "passed", "target": "aarch64",
            "iso": iso.path, "sha256": String(repeating: "a", count: 64),
            "manifest": root.appending(path: "builds/manifests/missing.json").path])
        let service = ISOUpdateService(projectRoot: root, runner: ISOFixtureRunner(chunks: [event], exitStatus: 0))
        await service.start()
        XCTAssertNil(service.lastResult)
        XCTAssertNotNil(service.errorMessage)
    }

    // ------------------------=
    // FUNC: testCancellationReachesRunnerAndAllowsRetry
    // DESC: Proves cancellation releases the service's busy state without reporting a stale or fabricated ISO result.
    // ------------------=
    @MainActor func testCancellationReachesRunnerAndAllowsRetry() async throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        try Data([73, 83, 79]).write(to: root.appending(path: "builds/InfinityOS-aarch64.iso"))
        let runner = ISOCancellationRunner()
        let service = ISOUpdateService(projectRoot: root, runner: runner)
        let task = Task { await service.start() }
        for _ in 0..<100 {
            if await runner.started { break }
            try await Task.sleep(for: .milliseconds(10))
        }
        XCTAssertTrue(service.isRunning)
        service.cancel()
        XCTAssertTrue(service.isCancelling)
        await task.value
        let cancelled = await runner.cancelled
        XCTAssertTrue(cancelled)
        XCTAssertFalse(service.isRunning)
        XCTAssertFalse(service.isCancelling)
        XCTAssertNil(service.lastResult)
        XCTAssertNotNil(service.errorMessage)
    }

    // ------------------------=
    // FUNC: testCompatibilityFailureAndNonzeroExitNeverReportSuccess
    // DESC: Keeps an old unsupported ISO unsuccessful even if a subprocess also emitted a misleading successful artifact record.
    // ------------------=
    @MainActor func testCompatibilityFailureAndNonzeroExitNeverReportSuccess() async throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let iso = root.appending(path: "builds/InfinityOS-aarch64.iso")
        let original = Data([73, 83, 79])
        try original.write(to: iso)
        let event = try JSONSerialization.data(withJSONObject: ["event": "result", "status": "failed",
            "error": "This ISO requires one compatibility rebuild before in-place UIKit updates."])
        let runner = ISOFixtureRunner(chunks: [event], exitStatus: 2)
        let service = ISOUpdateService(projectRoot: root, runner: runner)
        await service.start()
        XCTAssertFalse(service.isRunning)
        XCTAssertNil(service.lastResult)
        XCTAssertNotNil(service.errorMessage)
        XCTAssertNil(service.progress)
        XCTAssertEqual(try Data(contentsOf: iso), original)
    }

    // ------------------------=
    // FUNC: testExitZeroWithoutTypedArtifactIsNotSuccess
    // DESC: Rejects prose-only process output so changing a log line can never make artifact verification pass.
    // ------------------=
    @MainActor func testExitZeroWithoutTypedArtifactIsNotSuccess() async throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let iso = root.appending(path: "builds/InfinityOS-aarch64.iso")
        try Data([73, 83, 79]).write(to: iso)
        let service = ISOUpdateService(projectRoot: root,
            runner: ISOFixtureRunner(chunks: [Data("Build finished\n".utf8)], exitStatus: 0))
        await service.start()
        XCTAssertNil(service.lastResult)
        XCTAssertNotNil(service.errorMessage)
        XCTAssertFalse(service.isRunning)
    }

    // ------------------------=
    // FUNC: testWrongArchitectureIsNotAccepted
    // DESC: Rejects a successful artifact for a different target despite a zero process exit.
    // ------------------=
    @MainActor func testWrongArchitectureIsNotAccepted() async throws {
        let root = try fixtureRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let iso = root.appending(path: "builds/InfinityOS-aarch64.iso")
        try Data([73, 83, 79]).write(to: iso)
        let event = try JSONSerialization.data(withJSONObject: ["event": "result", "status": "passed", "target": "x86_64",
            "iso": iso.path, "sha256": String(repeating: "a", count: 64)])
        let service = ISOUpdateService(projectRoot: root, runner: ISOFixtureRunner(chunks: [event], exitStatus: 0))
        await service.start()
        XCTAssertNil(service.lastResult)
        XCTAssertNotNil(service.errorMessage)
    }

    // ------------------------=
    // FUNC: fixtureRoot
    // DESC: Contains every synthetic ISO and test manifest within repository-owned validation storage.
    // ------------------=
    private func fixtureRoot() throws -> URL {
        var repository = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { repository.deleteLastPathComponent() }
        let root = repository.appending(path: "builds/validation/studio-iso-update/\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appending(path: "builds"), withIntermediateDirectories: true)
        return root
    }
}
