import AppKit
import Darwin
import Foundation
import SwiftUI

enum NativePreviewViewport: String, CaseIterable, Identifiable {
    case compact = "1280 × 800", studio = "1600 × 1000", fullHD = "1920 × 1080", quadHD = "2560 × 1440"
    var id: String { rawValue }
    var width: Int { switch self { case .compact: 1280; case .studio: 1600; case .fullHD: 1920; case .quadHD: 2560 } }
    var height: Int { switch self { case .compact: 800; case .studio: 1000; case .fullHD: 1080; case .quadHD: 1440 } }
}

enum NativePreviewControlState: String, CaseIterable, Identifiable {
    case normal = "Normal", focused = "Focused", pressed = "Pressed", transition = "Transition"
    var id: String { rawValue }
    var focus: Int { self == .normal ? 2 : 1 }
    var isPressed: Bool { self == .pressed }
    var milliseconds: Int { self == .transition ? 70 : 1000 }
}

struct NativePreviewAsset: Equatable, Sendable {
    let names: [String]
    let bytes: Data
}

struct NativePreviewSimulation: Equatable, Sendable {
    var enabled = true
    var confirmation = false
    var progress = 42
    var timeZone = 3
    var dateFocus = 2
    var datePart = 0
    var expandedRow = -1
    var scroll = 0
    var controlFocus = 0

    // ------------------------=
    // FUNC: settingsRange
    // DESC: Restricts dashboard pages and expandable rows to the actual native content count.
    // ------------------=
    static func settingsRange(screen: Int, rows: Int) -> ClosedRange<Int> {
        if screen == 7 { return 0...6 }
        if screen == 8 { return 0...4 }
        return -1...max(-1, rows - 1)
    }

    // ------------------------=
    // FUNC: resolved
    // DESC: Normalizes reused preview state when switching between fields or native Settings sections.
    // ------------------=
    func resolved(collection: ScreenCollection, screen: Int, rows: Int) -> Self {
        var value = self
        if collection == .systemSettings {
            let range = Self.settingsRange(screen: screen, rows: rows)
            value.expandedRow = min(range.upperBound, max(range.lowerBound, expandedRow))
        }
        if collection == .installation && screen == 5 {
            value.datePart = dateFocus == 2 ? min(2, datePart) : dateFocus == 3 ? max(3, datePart) : 0
        }
        return value
    }

    // ------------------------=
    // FUNC: arguments
    // DESC: Encodes bounded simulated service values without giving the renderer any authority to run an installation or change the clock.
    // ------------------=
    func arguments() throws -> [String] {
        guard (0...100).contains(progress), (0...11).contains(timeZone), (2...4).contains(dateFocus),
              (0...4).contains(datePart), (-1...127).contains(expandedRow), (0...10000).contains(scroll),
              (0...127).contains(controlFocus) else { throw NativePreviewIssue.invalid("Preview state exceeds native bounds") }
        guard enabled else { return [] }
        var result = ["--simulate-services", "--progress", String(progress), "--time-zone", String(timeZone),
            "--date-focus", String(dateFocus), "--date-part", String(datePart), "--scroll", String(scroll),
            "--control-focus", String(controlFocus)]
        if confirmation { result.append("--confirmation") }
        if expandedRow >= 0 { result += ["--expanded-row", String(expandedRow)] }
        return result
    }
}

struct NativePreviewSnapshot: Equatable, Sendable {
    let projectRoot: URL
    let template: Data
    let kind: String
    let screen: Int
    let width: Int
    let height: Int
    let atlas: Data
    let glyphs: Data
    let styles: Data
    let background: Data
    let assets: [NativePreviewAsset]
    var tokens = Data()
    var state = NativePreviewControlState.normal
    var simulation = NativePreviewSimulation()

    // ------------------------=
    // FUNC: capture
    // DESC: Captures unsaved editor state into the exact runtime contracts without saving or modifying any source document.
    // ------------------=
    @MainActor static func capture(_ store: TemplateStore, width requestedWidth: Int? = nil, height requestedHeight: Int? = nil) throws -> Self {
        let width = requestedWidth ?? store.canvasViewport.width, height = requestedHeight ?? store.canvasViewport.height
        guard let root = store.projectRoot, (320...3840).contains(width), (240...2160).contains(height) else {
            throw NativePreviewIssue.invalid("A project and bounded native viewport are required")
        }
        let document: InstallerStudioDocument
        let kind: String
        switch store.selectedCollection {
        case .installation: document = store.document; kind = "installer"
        case .configuration: document = store.configurationDocument; kind = "configuration"
        case .systemSettings: document = store.settingsDocument; kind = "settings"
        }
        guard let atlas = store.uiKit.previewAtlas(target: "assets/ui-design-kit/default/eclipse-precision-v1.rgba"),
              let glyphs = store.uiKit.previewAtlas(target: "assets/ui-design-kit/default/eclipse-access-glyphs-v1.rgba") else {
            throw NativePreviewIssue.invalid("The native UI artwork cannot be read")
        }
        let backgroundPath = CanvasRuntimeBackdrop.assetPath(for: store.selectedCollection)
        let masterPath = (backgroundPath as NSString).deletingPathExtension + ".png"
        let backgroundOverride = store.uiKit.document.overrides.last {
            $0.target == backgroundPath || $0.target == masterPath
        }
        let backgroundURL = try UIKitAssetCatalog.resolvedURL(backgroundOverride?.source ?? backgroundPath, root: root)
        var assets: [NativePreviewAsset] = []
        var replacementSources: [String: String] = [:]
        for replacement in store.uiKit.document.overrides where replacement.format != "rgba" {
            replacementSources[replacement.target] = replacement.source
        }
        for target in replacementSources.keys.sorted() {
            assets.append(NativePreviewAsset(names: [target],
                bytes: try NativePreviewBitmapCache.read(UIKitAssetCatalog.resolvedURL(replacementSources[target]!, root: root))))
        }
        return Self(projectRoot: root, template: try RuntimeTemplateCodec.encode(document, assetRoot: root, collection: store.selectedCollection, theme: store.uiKit.document),
            kind: kind, screen: store.selectedScreenID, width: width, height: height, atlas: atlas, glyphs: glyphs,
            styles: try store.uiKit.document.runtimeStyles(), background: try NativePreviewBitmapCache.read(backgroundURL), assets: assets,
            tokens: try store.uiKit.document.runtimeTokens(), state: store.nativePreviewState,
            simulation: store.nativePreviewSimulation.resolved(collection: store.selectedCollection,
                screen: store.selectedScreenID, rows: store.selectedScreen?.elements.filter { $0.role == .settingsRowLabel }.count ?? 0))
    }

    // ------------------------=
    // FUNC: bitmap
    // DESC: Preserves uploaded sRGB and straight alpha in the V4 BGRA bitmap format understood by the production native painter.
    // ------------------=
    @MainActor static func bitmap(_ image: NSImage) throws -> Data {
        guard let source = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
            throw NativePreviewIssue.invalid("The preview image cannot be decoded")
        }
        return try NativeBitmapEncoder.encode(source)
    }
}

@MainActor private enum NativePreviewBitmapCache {
    private static var images: [String: Data] = [:]

    // ------------------------=
    // FUNC: read
    // DESC: Keeps expensive image conversion out of repeated typing/drag updates while invalidating changed source files.
    // ------------------=
    static func read(_ url: URL) throws -> Data {
        let attributes = try url.resourceValues(forKeys: [.contentModificationDateKey, .fileSizeKey])
        let key = url.path + ":\(attributes.contentModificationDate?.timeIntervalSince1970 ?? 0):\(attributes.fileSize ?? 0)"
        if let cached = images[key] { return cached }
        let bytes = try NativeBitmapEncoder.read(url)
        if images.count >= 24 { images.removeAll(keepingCapacity: true) }
        images[key] = bytes
        return bytes
    }
}

struct NativePreviewBounds: Decodable, Equatable, Sendable, Identifiable {
    let id: UUID
    let x: Int
    let y: Int
    let width: Int
    let height: Int
}

enum NativeCanvasMapping {
    // ------------------------=
    // FUNC: editingScale
    // DESC: Inverts the production Settings console transform for authoring gestures, keeping outer-window movement in viewport coordinates.
    // ------------------=
    static func editingScale(element: StudioElement, console: CanvasRect?, window: NativePreviewBounds?,
        viewport: NativePreviewViewport, scale: CGFloat) -> CGSize {
        guard element.role != .console, let console, let window, console.width > 0, console.height > 0 else {
            return CGSize(width: CGFloat(viewport.width) * scale / 1000, height: CGFloat(viewport.height) * scale / 1000)
        }
        return CGSize(width: CGFloat(window.width) * scale / CGFloat(console.width),
            height: CGFloat(window.height) * scale / CGFloat(console.height))
    }
}

struct NativePreviewFrame: Equatable, Sendable {
    let pixels: Data
    let width: Int
    let height: Int
    var bounds: [NativePreviewBounds] = []

    // ------------------------=
    // FUNC: validated
    // DESC: Accepts only a complete native RGBA framebuffer matching the requested viewport.
    // ------------------=
    static func validated(_ pixels: Data, width: Int, height: Int) throws -> Self {
        guard (320...3840).contains(width), (240...2160).contains(height), pixels.count == width * height * 4 else {
            throw NativePreviewIssue.invalid("The native renderer returned an incomplete framebuffer")
        }
        return Self(pixels: pixels, width: width, height: height)
    }

    // ------------------------=
    // FUNC: image
    // DESC: Displays native framebuffer pixels without re-layout or host-font substitution.
    // ------------------=
    func image() -> CGImage? {
        guard let provider = CGDataProvider(data: pixels as CFData), let colorSpace = CGColorSpace(name: CGColorSpace.sRGB) else { return nil }
        return CGImage(width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: width * 4,
            space: colorSpace, bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue),
            provider: provider, decode: nil, shouldInterpolate: true, intent: .defaultIntent)
    }
}

enum NativePreviewIssue: Error, LocalizedError {
    case invalid(String)
    var errorDescription: String? { switch self { case let .invalid(reason): reason } }
}

protocol NativePreviewRunning: Sendable {
    // ------------------------=
    // FUNC: render
    // DESC: Renders a typed immutable draft snapshot independently of the main UI actor.
    // ------------------=
    func render(_ snapshot: NativePreviewSnapshot) async throws -> NativePreviewFrame
}

private final class NativePreviewProcess: @unchecked Sendable {
    private let lock = NSLock()
    private var process: Process?
    private var cancelled = false
    private var timedOut = false
    var expired: Bool { lock.lock(); defer { lock.unlock() }; return timedOut }

    // ------------------------=
    // FUNC: launch
    // DESC: Serializes process startup against cancellation so a superseded draft cannot escape termination.
    // ------------------=
    func launch(_ candidate: Process) throws {
        lock.lock(); defer { lock.unlock() }
        guard !cancelled else { throw CancellationError() }
        process = candidate
        try candidate.run()
    }

    // ------------------------=
    // FUNC: cancel
    // DESC: Terminates only this preview helper, never the editor, build kit or another user's process.
    // ------------------=
    func cancel() {
        lock.lock(); defer { lock.unlock() }
        cancelled = true
        if let process, process.isRunning { process.terminate() }
        DispatchQueue.global().asyncAfter(deadline: .now() + 1) { [self] in
            lock.lock(); defer { lock.unlock() }
            if let process, process.isRunning { Darwin.kill(process.processIdentifier, SIGKILL) }
        }
    }

    // ------------------------=
    // FUNC: timeout
    // DESC: Bounds a hung renderer's lifetime while preserving a distinct user-visible timeout error.
    // ------------------=
    func timeout() {
        lock.lock(); timedOut = true; lock.unlock()
        cancel()
    }
}

struct NativePreviewRunner: NativePreviewRunning {
    var executableOverride: URL? = nil
    var timeout: Duration = .seconds(15)
    // ------------------------=
    // FUNC: render
    // DESC: Runs the prebuilt production renderer directly with repository-local scratch files, cancellable execution and bounded diagnostics.
    // ------------------=
    func render(_ snapshot: NativePreviewSnapshot) async throws -> NativePreviewFrame {
        let processBox = NativePreviewProcess()
        let deadline = Task {
            do { try await Task.sleep(for: timeout); processBox.timeout() } catch {}
        }
        defer { deadline.cancel() }
        return try await withTaskCancellationHandler {
            try Task.checkCancellation()
            return try await Task.detached(priority: .userInitiated) {
                let root = snapshot.projectRoot.standardizedFileURL.resolvingSymlinksInPath()
                let scratch = root.appending(path: "build/tmp/studio-native-preview-\(UUID().uuidString)")
                    .standardizedFileURL.resolvingSymlinksInPath()
                guard scratch.path.hasPrefix(root.path + "/build/tmp/") else {
                    throw NativePreviewIssue.invalid("Native preview scratch files must remain inside the project")
                }
                try FileManager.default.createDirectory(at: scratch, withIntermediateDirectories: true)
                defer { try? FileManager.default.removeItem(at: scratch) }
                let template = scratch.appending(path: "draft.iuit"), output = scratch.appending(path: "frame.rgba")
                let atlas = scratch.appending(path: "precision.rgba"), glyphs = scratch.appending(path: "glyphs.rgba")
                let styles = scratch.appending(path: "styles.bin"), background = scratch.appending(path: "background.bmp")
                let tokens = scratch.appending(path: "tokens.bin")
                for (url, data) in [(template, snapshot.template), (atlas, snapshot.atlas), (glyphs, snapshot.glyphs),
                                    (styles, snapshot.styles), (background, snapshot.background), (tokens, snapshot.tokens)] { try data.write(to: url, options: .atomic) }
                var arguments = ["--studio-native-render", "--template", template.path, "--kind", snapshot.kind,
                    "--screen", String(snapshot.screen), "--width", String(snapshot.width), "--height", String(snapshot.height),
                    "--output", output.path, "--background", background.path, "--atlas", atlas.path,
                    "--glyphs", glyphs.path, "--styles", styles.path, "--tokens", tokens.path,
                    "--focus", String(snapshot.state.focus), "--phase-ms", String(snapshot.state.milliseconds)]
                if snapshot.state.isPressed { arguments.append("--pressed") }
                arguments += try snapshot.simulation.arguments()
                for (index, asset) in snapshot.assets.enumerated() {
                    let url = scratch.appending(path: "asset-\(index).bmp")
                    try asset.bytes.write(to: url, options: .atomic)
                    for name in asset.names { arguments += ["--asset", name + "=" + url.path] }
                }
                let candidates = [executableOverride, Bundle.main.executableURL?.deletingLastPathComponent().appending(path: "InfinityStudioNativeRenderer"),
                    root.appending(path: "build/behavior-tests/infinity-studio-native-renderer"),
                    root.appending(path: "builds/InfinityOS Installer Studio.app/Contents/MacOS/InfinityStudioNativeRenderer")].compactMap { $0 }
                guard let executable = candidates.first(where: { FileManager.default.isExecutableFile(atPath: $0.path) }) else {
                    throw NativePreviewIssue.invalid("Native renderer is not installed. Build Infinity Studio through build-kit studio.")
                }
                let process = Process(), errors = Pipe()
                process.executableURL = executable; process.arguments = arguments; process.currentDirectoryURL = root
                process.standardOutput = FileHandle.nullDevice; process.standardError = errors
                var environment = ProcessInfo.processInfo.environment
                environment["TMPDIR"] = scratch.path; environment["TMP"] = scratch.path; environment["TEMP"] = scratch.path
                process.environment = environment
                try processBox.launch(process)
                try errors.fileHandleForWriting.close()
                var diagnostic = Data()
                while let chunk = try errors.fileHandleForReading.read(upToCount: 4096), !chunk.isEmpty {
                    if diagnostic.count < 8192 { diagnostic.append(chunk.prefix(8192 - diagnostic.count)) }
                }
                process.waitUntilExit()
                if processBox.expired { throw NativePreviewIssue.invalid("The native preview exceeded its rendering time limit. Editing remains available; retry the preview.") }
                guard process.terminationStatus == 0 else {
                    let detail = String(decoding: diagnostic, as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
                    throw NativePreviewIssue.invalid(detail.isEmpty ? "Native renderer exited with status \(process.terminationStatus)" : detail)
                }
                var frame = try NativePreviewFrame.validated(Data(contentsOf: output), width: snapshot.width, height: snapshot.height)
                let geometryURL = output.deletingPathExtension().appendingPathExtension("geometry.json")
                if FileManager.default.fileExists(atPath: geometryURL.path) {
                    let geometry = try Data(contentsOf: geometryURL)
                    guard geometry.count <= 512 * 1024 else { throw NativePreviewIssue.invalid("Native layer geometry exceeds bounds") }
                    frame.bounds = try JSONDecoder().decode([NativePreviewBounds].self, from: geometry)
                    guard frame.bounds.count <= 512, Set(frame.bounds.map(\.id)).count == frame.bounds.count,
                        frame.bounds.allSatisfy({ abs($0.x) <= 16384 && abs($0.y) <= 16384 && (0...16384).contains($0.width) && (0...16384).contains($0.height) }) else {
                        throw NativePreviewIssue.invalid("Native layer geometry is invalid")
                    }
                }
                return frame
            }.value
        } onCancel: { processBox.cancel() }
    }
}

enum NativePreviewPhase: Equatable { case idle, rendering, ready, failed }

@MainActor final class NativePreviewService: ObservableObject {
    @Published private(set) var phase = NativePreviewPhase.idle
    @Published private(set) var frame: NativePreviewFrame?
    @Published private(set) var image: CGImage?
    @Published private(set) var errorMessage: String?
    private let runner: any NativePreviewRunning
    private var task: Task<Void, Never>?
    private var generation = 0
    private var completedSnapshot: NativePreviewSnapshot?

    // ------------------------=
    // FUNC: init
    // DESC: Keeps the rendering boundary injectable for cancellation and stale-result behavioral tests.
    // ------------------=
    init(runner: any NativePreviewRunning = NativePreviewRunner()) { self.runner = runner }

    // ------------------------=
    // FUNC: schedule
    // DESC: Coalesces edits without blocking typing, dragging or selection and snapshots the latest unsaved state only after the debounce interval.
    // ------------------=
    func schedule(_ store: TemplateStore) {
        task?.cancel()
        generation += 1
        phase = .rendering
        let requestGeneration = generation
        task = Task { [weak self, weak store] in
            do {
                try await Task.sleep(for: .milliseconds(140))
                guard let self, let store else { return }
                let snapshot = try NativePreviewSnapshot.capture(store)
                if snapshot == self.completedSnapshot { self.phase = .ready; return }
                await self.render(snapshot, generation: requestGeneration)
            } catch is CancellationError {} catch {
                guard let self, requestGeneration == self.generation else { return }
                self.errorMessage = error.localizedDescription; self.phase = .failed
            }
        }
    }

    // ------------------------=
    // FUNC: update
    // DESC: Renders an immutable snapshot directly for previews and tests while retaining latest-request ownership of published pixels.
    // ------------------=
    func update(_ snapshot: NativePreviewSnapshot) async {
        task?.cancel(); generation += 1
        await render(snapshot, generation: generation)
    }

    // ------------------------=
    // FUNC: render
    // DESC: Publishes only verified complete native frames from the newest request and retains old pixels with an explicit stale/error state otherwise.
    // ------------------=
    private func render(_ snapshot: NativePreviewSnapshot, generation requested: Int) async {
        phase = .rendering; errorMessage = nil
        do {
            let result = try await runner.render(snapshot)
            try Task.checkCancellation()
            guard requested == generation else { return }
            guard result.width == snapshot.width, result.height == snapshot.height else {
                throw NativePreviewIssue.invalid("The native renderer returned the wrong viewport")
            }
            var validated = try NativePreviewFrame.validated(result.pixels, width: snapshot.width, height: snapshot.height)
            validated.bounds = result.bounds
            guard let image = validated.image() else { throw NativePreviewIssue.invalid("Native framebuffer cannot be displayed") }
            frame = validated; self.image = image; completedSnapshot = snapshot; phase = .ready
        } catch is CancellationError {
            if requested == generation { phase = .idle }
        } catch {
            guard requested == generation else { return }
            errorMessage = error.localizedDescription; phase = .failed
        }
    }

    // ------------------------=
    // FUNC: cancel
    // DESC: Cancels an off-screen preview and invalidates its pending result while preserving the last verified image for return navigation.
    // ------------------=
    func cancel() {
        generation += 1; task?.cancel(); task = nil; phase = .idle
    }
}
