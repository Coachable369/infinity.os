import AppKit
import Foundation
import SwiftUI
import UniformTypeIdentifiers

enum ISOArchitecture: String, CaseIterable, Identifiable, Codable, Sendable {
    case aarch64, x86_64
    var id: String { rawValue }
    var title: String { self == .aarch64 ? "ARM64 / Apple Silicon" : "x86-64" }
}

struct ISOUpdateRequest: Equatable, Sendable {
    let executableURL: URL
    let workingDirectoryURL: URL
    let arguments: [String]
    let isoURL: URL
    let target: ISOArchitecture
    let cancellationURL: URL

    // ------------------------=
    // FUNC: make
    // DESC: Constructs an injection-safe build-kit invocation constrained to a selected existing repository-local ISO.
    // ------------------=
    static func make(projectRoot: URL, target: ISOArchitecture, isoURL: URL) throws -> Self {
        let root = projectRoot.resolvingSymlinksInPath().standardizedFileURL
        let iso = isoURL.resolvingSymlinksInPath().standardizedFileURL
        guard iso.path.hasPrefix(root.path + "/"), iso.pathExtension.lowercased() == "iso",
              try iso.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile == true else {
            throw ISOUpdateIssue.invalid("Select an existing ISO inside this InfinityOS project")
        }
        let cancellation = root.appending(path: "build/tmp/studio-iso-cancel-\(UUID().uuidString)").resolvingSymlinksInPath()
        guard cancellation.path.hasPrefix(root.path + "/") else {
            throw ISOUpdateIssue.invalid("The build scratch directory must remain inside this InfinityOS project")
        }
        return Self(executableURL: root.appending(path: "build-kit"), workingDirectoryURL: root,
            arguments: ["update-iso", "--target", target.rawValue, "--iso", iso.path,
                        "--cancel-file", cancellation.path], isoURL: iso, target: target, cancellationURL: cancellation)
    }
}

struct ISOProcessResult: Sendable {
    let exitStatus: Int32
}

struct ISOUpdateResult: Equatable, Sendable {
    let isoURL: URL
    let target: ISOArchitecture
    let sha256: String
    let manifestURL: URL?
}

enum ISOUpdateIssue: Error, LocalizedError {
    case invalid(String)
    var errorDescription: String? { switch self { case let .invalid(message): message } }
}

protocol ISOUpdateRunning: Sendable {
    // ------------------------=
    // FUNC: run
    // DESC: Runs a validated build authority request while streaming bounded data chunks independently of the UI thread.
    // ------------------=
    func run(_ request: ISOUpdateRequest, output: @escaping @Sendable (Data) async -> Void) async throws -> ISOProcessResult
}

struct NativeISOUpdateRunner: ISOUpdateRunning {
    // ------------------------=
    // FUNC: run
    // DESC: Launches build-kit directly without a shell and drains merged stdout/stderr off the main actor until the process exits.
    // ------------------=
    func run(_ request: ISOUpdateRequest, output: @escaping @Sendable (Data) async -> Void) async throws -> ISOProcessResult {
        try Task.checkCancellation()
        try FileManager.default.createDirectory(at: request.cancellationURL.deletingLastPathComponent(), withIntermediateDirectories: true)
        return try await withTaskCancellationHandler {
            try await Task.detached(priority: .userInitiated) {
            defer { try? FileManager.default.removeItem(at: request.cancellationURL) }
            let process = Process()
            let pipe = Pipe()
            process.executableURL = request.executableURL
            process.arguments = request.arguments
            process.currentDirectoryURL = request.workingDirectoryURL
            process.standardOutput = pipe
            process.standardError = pipe
            var environment = ProcessInfo.processInfo.environment
            // Finder-launched applications do not inherit a developer shell's PATH.
            // Build authority still owns isolation, temporary paths and all subordinate tools.
            let additions = ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin", "/usr/sbin", "/sbin"]
            environment["PATH"] = ([environment["PATH"] ?? ""] + additions).joined(separator: ":")
            environment["PYTHONUNBUFFERED"] = "1"
            process.environment = environment
            try process.run()
            try pipe.fileHandleForWriting.close()
            while let chunk = try pipe.fileHandleForReading.read(upToCount: 16_384), !chunk.isEmpty {
                await output(chunk)
            }
            process.waitUntilExit()
            return ISOProcessResult(exitStatus: process.terminationStatus)
            }.value
        } onCancel: {
            // The updater owns and terminates its child process groups before
            // unwinding its staged transaction; build-kit can then release its lock.
            try? Data([1]).write(to: request.cancellationURL, options: .atomic)
        }
    }
}

private struct ISOUpdateEvent: Decodable {
    var event: String
    var phase: String?
    var progress: Double?
    var status: String?
    var iso: String?
    var target: ISOArchitecture?
    var sha256: String?
    var manifest: String?
    var error: String?
}

@MainActor
final class ISOUpdateService: ObservableObject {
    @Published var target: ISOArchitecture = .aarch64 {
        didSet { selectCanonicalISO(after: oldValue) }
    }
    @Published var selectedISO: URL?
    @Published private(set) var isRunning = false
    @Published private(set) var isCancelling = false
    @Published private(set) var progress: Double?
    @Published private(set) var status = "Save your sections, then select the ISO to update"
    @Published private(set) var errorMessage: String?
    @Published private(set) var output = ""
    @Published private(set) var lastResult: ISOUpdateResult?
    let projectRoot: URL?
    private let runner: any ISOUpdateRunning
    private var lineBuffer = Data()
    private var resultEvent: ISOUpdateEvent?
    private var runningTask: Task<ISOProcessResult, Error>?

    // ------------------------=
    // FUNC: init
    // DESC: Creates an idle async packaging service with an injectable process boundary for behavioral tests.
    // ------------------=
    init(projectRoot: URL?, runner: any ISOUpdateRunning = NativeISOUpdateRunner()) {
        self.projectRoot = projectRoot
        self.runner = runner
        if let candidate = projectRoot?.appending(path: "builds/InfinityOS-aarch64.iso"),
           FileManager.default.fileExists(atPath: candidate.path) { selectedISO = candidate }
    }

    // ------------------------=
    // FUNC: selectCanonicalISO
    // DESC: Follows an architecture change only for the default ISO selection, preserving deliberately chosen custom media paths.
    // ------------------=
    private func selectCanonicalISO(after previous: ISOArchitecture) {
        guard !isRunning, target != previous, let root = projectRoot else { return }
        let oldCanonical = root.appending(path: "builds/InfinityOS-\(previous.rawValue).iso").standardizedFileURL
        guard selectedISO == nil || selectedISO?.standardizedFileURL == oldCanonical else { return }
        let candidate = root.appending(path: "builds/InfinityOS-\(target.rawValue).iso")
        selectedISO = FileManager.default.fileExists(atPath: candidate.path) ? candidate : nil
    }

    // ------------------------=
    // FUNC: chooseISO
    // DESC: Selects an existing installer image through the native file picker without starting an update implicitly.
    // ------------------=
    func chooseISO() {
        guard !isRunning else { return }
        let panel = NSOpenPanel()
        if let type = UTType(filenameExtension: "iso") { panel.allowedContentTypes = [type] }
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.directoryURL = projectRoot?.appending(path: "builds")
        panel.message = "Select the repository-local installer ISO to update after saving your UI changes."
        guard panel.runModal() == .OK, let url = panel.url else { return }
        selectedISO = url
    }

    // ------------------------=
    // FUNC: start
    // DESC: Updates the selected ISO asynchronously and reports success only after a successful process exit and matching structured artifact result.
    // ------------------=
    func start() async {
        guard !isRunning else { return }
        lastResult = nil; errorMessage = nil; output = ""; lineBuffer = Data(); resultEvent = nil
        progress = nil
        do {
            guard let root = projectRoot, let iso = selectedISO else {
                throw ISOUpdateIssue.invalid("Select an InfinityOS project and installer ISO first")
            }
            let request = try ISOUpdateRequest.make(projectRoot: root, target: target, isoURL: iso)
            isRunning = true; status = "Updating \(target.title) installer ISO…"
            isCancelling = false
            defer { isRunning = false; isCancelling = false; runningTask = nil }
            let runner = self.runner
            let task = Task {
                try await runner.run(request) { [weak self] bytes in
                    await self?.receive(bytes)
                }
            }
            runningTask = task
            let process = try await task.value
            if !lineBuffer.isEmpty { parseEvent(lineBuffer); lineBuffer.removeAll() }
            guard process.exitStatus == 0 else {
                throw ISOUpdateIssue.invalid(resultEvent?.error ?? "ISO update failed (exit \(process.exitStatus)). Review the build output; the previous ISO was not accepted as an update.")
            }
            lastResult = try verifiedResult(for: request)
            progress = 1
            status = "ISO updated and verified: \(request.isoURL.lastPathComponent)"
        } catch {
            errorMessage = error.localizedDescription; status = "ISO update did not complete"
            progress = nil
        }
    }

    // ------------------------=
    // FUNC: cancel
    // DESC: Requests staged cancellation without terminating build authority or leaving native compiler children running.
    // ------------------=
    func cancel() {
        guard isRunning, !isCancelling else { return }
        isCancelling = true
        status = "Cancelling safely; the current build step is stopping…"
        runningTask?.cancel()
    }

    // ------------------------=
    // FUNC: receive
    // DESC: Streams bounded console output and decodes complete JSON progress records across arbitrary pipe chunk boundaries.
    // ------------------=
    private func receive(_ data: Data) {
        output.append(String(decoding: data, as: UTF8.self))
        if output.utf8.count > 131_072 { output = String(output.suffix(65_536)) }
        lineBuffer.append(data)
        while let newline = lineBuffer.firstIndex(of: 10) {
            parseEvent(lineBuffer.prefix(upTo: newline))
            lineBuffer.removeSubrange(...newline)
        }
        if lineBuffer.count > 1_048_576 { lineBuffer.removeAll() }
    }

    // ------------------------=
    // FUNC: parseEvent
    // DESC: Uses structured updater events for progress and artifact evidence while ordinary compiler messages remain diagnostics only.
    // ------------------=
    private func parseEvent(_ line: Data) {
        guard let event = try? JSONDecoder().decode(ISOUpdateEvent.self, from: line) else { return }
        if event.event == "progress" {
            if let phase = event.phase, !isCancelling { status = phase }
            if let value = event.progress, value.isFinite { progress = min(0.99, max(0, value)) }
        } else if event.event == "result" { resultEvent = event }
    }

    // ------------------------=
    // FUNC: verifiedResult
    // DESC: Rejects false success, missing image outputs, wrong architectures and results pointing at another file or repository.
    // ------------------=
    private func verifiedResult(for request: ISOUpdateRequest) throws -> ISOUpdateResult {
        guard let event = resultEvent, event.status == "passed", event.target == request.target,
              let path = event.iso, let hash = event.sha256, hash.count == 64,
              hash.utf8.allSatisfy({ (48...57).contains($0) || (97...102).contains($0) }),
              URL(fileURLWithPath: path).resolvingSymlinksInPath().standardizedFileURL == request.isoURL,
              let attributes = try? FileManager.default.attributesOfItem(atPath: request.isoURL.path),
              let size = attributes[.size] as? NSNumber, size.int64Value > 0 else {
            throw ISOUpdateIssue.invalid("Build exited without a verified result for the selected ISO")
        }
        guard let manifestPath = event.manifest else {
            throw ISOUpdateIssue.invalid("The ISO update did not publish its verification manifest")
        }
        let manifest = URL(fileURLWithPath: manifestPath).resolvingSymlinksInPath().standardizedFileURL
        guard manifest.path.hasPrefix(request.workingDirectoryURL.path + "/builds/manifests/"),
              let manifestSize = try? manifest.resourceValues(forKeys: [.fileSizeKey]).fileSize,
              manifestSize <= 1_048_576,
              let bytes = try? Data(contentsOf: manifest),
              let record = try? JSONSerialization.jsonObject(with: bytes) as? [String: Any],
              record["status"] as? String == "passed", record["sha256"] as? String == hash,
              record["iso"] as? String == request.isoURL.path,
              record["target"] as? String == request.target.rawValue,
              let checksum = try? String(contentsOf: request.isoURL.appendingPathExtension("sha256"), encoding: .utf8),
              checksum == "\(hash)  \(request.isoURL.lastPathComponent)\n" else {
            throw ISOUpdateIssue.invalid("The published checksum or verification manifest does not match this ISO update")
        }
        return ISOUpdateResult(isoURL: request.isoURL, target: request.target, sha256: hash, manifestURL: manifest)
    }
}
