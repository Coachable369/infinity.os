import Foundation
import SwiftUI

struct ThemeSummary: Identifiable, Equatable {
    var id: UUID
    var name: String
    var isBuiltIn: Bool
    var previewDocument: UIKitDocument
    var colors: [UInt32] { previewDocument.colors }
    var styles: [UIKitSurfaceStyle] { previewDocument.styles }
}

@MainActor final class ThemeLibrary: ObservableObject {
    @Published private(set) var themes: [ThemeSummary] = []
    @Published private(set) var issues: [String] = []
    @Published private(set) var lastTrashURL: URL?
    let projectRoot: URL?
    private let atlasCache = NSCache<NSString, NSData>()

    static let precisionID = UUID(uuidString: "29906AD5-94BC-4CEE-9364-B8DCEDE00001")!
    static let glassID = UUID(uuidString: "29906AD5-94BC-4CEE-9364-B8DCEDE00002")!
    static let matteID = UUID(uuidString: "29906AD5-94BC-4CEE-9364-B8DCEDE00003")!

    // ------------------------=
    // FUNC: builtIns
    // DESC: Derives immutable presets from the approved Eclipse artwork and semantic palette without replacing any original images.
    // ------------------=
    static func builtIns() -> [ThemeBundle] {
        let precision = ThemeBundle(id: precisionID, name: "Eclipse Precision", document: UIKitDocument())
        var glass = UIKitDocument()
        var matte = UIKitDocument()
        for index in 1..<6 {
            glass.styles[index].material = .glass
            matte.styles[index].material = .matte
        }
        for index in 2..<6 {
            glass.styles[index].corners = .rounded
            glass.styles[index].radius = index == 2 ? 16 : 10
            matte.styles[index].corners = .square
            matte.styles[index].radius = 0
        }
        glass.colors[UIKitColorRole.panel.rawValue] = 0xc4101b3a
        matte.colors[UIKitColorRole.panel.rawValue] = 0xff101b3a
        return [precision, ThemeBundle(id: glassID, name: "Frosted Glass", document: glass),
                ThemeBundle(id: matteID, name: "Matte", document: matte)]
    }

    // ------------------------=
    // FUNC: init
    // DESC: Opens a repository-contained theme library while making immutable approved presets available even without a project.
    // ------------------=
    init(projectRoot: URL?) {
        self.projectRoot = projectRoot?.resolvingSymlinksInPath().standardizedFileURL
        atlasCache.totalCostLimit = 64 * 1024 * 1024
        atlasCache.countLimit = 12
        themes = Self.builtIns().map { Self.summary($0, builtIn: true) }
        do { try reload() } catch { issues = [error.localizedDescription] }
    }

    // ------------------------=
    // FUNC: summary
    // DESC: Keeps small preview documents in the library list without retaining embedded image data for every theme card.
    // ------------------=
    private static func summary(_ bundle: ThemeBundle, builtIn: Bool) -> ThemeSummary {
        ThemeSummary(id: bundle.id, name: bundle.name, isBuiltIn: builtIn, previewDocument: bundle.document)
    }

    // ------------------------=
    // FUNC: requireRoot
    // DESC: Rejects modifying an unbound library or another project's active editor document.
    // ------------------=
    private func requireRoot(store: UIKitStore? = nil) throws -> URL {
        guard let root = projectRoot else { throw UIKitIssue.invalid("Open an InfinityOS project before using the theme library") }
        if let store, store.projectRoot?.resolvingSymlinksInPath().standardizedFileURL != root {
            throw UIKitIssue.invalid("The theme and editor must belong to the same project")
        }
        return root
    }

    // ------------------------=
    // FUNC: directory
    // DESC: Resolves the theme library through the same no-symlink, repository-contained policy used by native UIKit assets.
    // ------------------=
    private func directory() throws -> URL {
        try UIKitAssetCatalog.resolvedURL("assets/ui-design-kit/themes", root: requireRoot())
    }

    // ------------------------=
    // FUNC: file
    // DESC: Uses only a canonical UUID as a filename, never user-visible names or imported paths.
    // ------------------=
    private func file(_ id: UUID) throws -> URL {
        try UIKitAssetCatalog.resolvedURL("assets/ui-design-kit/themes/\(id.uuidString.lowercased()).infinitytheme", root: requireRoot())
    }

    // ------------------------=
    // FUNC: reload
    // DESC: Discovers valid custom themes while isolating malformed files and retaining immutable approved presets.
    // ------------------=
    func reload() throws {
        atlasCache.removeAllObjects()
        var result = Self.builtIns().map { Self.summary($0, builtIn: true) }
        var failures: [String] = []
        guard let root = projectRoot else { themes = result; issues = []; return }
        let folder = try directory()
        if FileManager.default.fileExists(atPath: folder.path) {
            let catalog = try UIKitAssetCatalog.load(root: root)
            for url in try FileManager.default.contentsOfDirectory(at: folder, includingPropertiesForKeys: [.isRegularFileKey], options: [.skipsHiddenFiles])
                .filter({ $0.pathExtension == "infinitytheme" }).sorted(by: { $0.lastPathComponent < $1.lastPathComponent }) {
                do {
                    guard let id = UUID(uuidString: url.deletingPathExtension().lastPathComponent),
                          !Self.builtIns().contains(where: { $0.id == id }), url.standardizedFileURL == (try file(id)).standardizedFileURL else {
                        throw UIKitIssue.invalid("The theme filename must match its custom UUID")
                    }
                    let bundle = try ThemeBundle.read(from: url, catalog: catalog)
                    guard bundle.id == id else { throw UIKitIssue.invalid("Theme identity differs from its filename") }
                    result.append(Self.summary(bundle, builtIn: false))
                } catch { failures.append("\(url.lastPathComponent): \(error.localizedDescription)") }
            }
        }
        let builtIn = result.filter(\.isBuiltIn)
        themes = builtIn + result.filter { !$0.isBuiltIn }.sorted { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
        issues = failures
    }

    // ------------------------=
    // FUNC: bundle
    // DESC: Loads one validated theme on demand so large artwork payloads are not retained by the sidebar or preview list.
    // ------------------=
    func bundle(id: UUID) throws -> ThemeBundle {
        if let preset = Self.builtIns().first(where: { $0.id == id }) { return preset }
        let root = try requireRoot()
        let result = try ThemeBundle.read(from: file(id), catalog: UIKitAssetCatalog.load(root: root))
        guard result.id == id else { throw UIKitIssue.invalid("Theme identity differs from its filename") }
        return result
    }

    // ------------------------=
    // FUNC: previewAtlas
    // DESC: Caches actual portable theme pixels for native preview cards while preserving both the active editor and original artwork.
    // ------------------=
    func previewAtlas(id: UUID, target: String, store: UIKitStore) -> Data? {
        do {
            _ = try requireRoot(store: store)
            let key = "\(id.uuidString):\(target)" as NSString
            if let cached = atlasCache.object(forKey: key) { return cached as Data }
            guard let pixels = store.previewThemeAtlas(target: target, theme: try bundle(id: id)) else { return nil }
            atlasCache.setObject(pixels as NSData, forKey: key, cost: pixels.count)
            return pixels
        } catch { return nil }
    }

    // ------------------------=
    // FUNC: persist
    // DESC: Atomically publishes a complete self-contained custom theme after validation, leaving editor state unchanged on failure.
    // ------------------=
    private func persist(_ bundle: ThemeBundle) throws -> ThemeSummary {
        guard !Self.builtIns().contains(where: { $0.id == bundle.id }) else {
            throw UIKitIssue.invalid("Built-in themes are read-only; save a new custom theme instead")
        }
        let root = try requireRoot()
        let bytes = try bundle.encoded(catalog: UIKitAssetCatalog.load(root: root))
        let target = try file(bundle.id)
        try FileManager.default.createDirectory(at: directory(), withIntermediateDirectories: true)
        try bytes.write(to: target, options: .atomic)
        try reload()
        return Self.summary(bundle, builtIn: false)
    }

    // ------------------------=
    // FUNC: saveCurrent
    // DESC: Saves the current unsaved editor recipe under a new UUID or an explicitly chosen existing custom identity.
    // ------------------=
    @discardableResult func saveCurrent(name: String, store: UIKitStore, replacingID: UUID? = nil) throws -> ThemeSummary {
        let root = try requireRoot(store: store)
        if let id = replacingID {
            guard themes.contains(where: { $0.id == id && !$0.isBuiltIn }) else {
                throw UIKitIssue.invalid("Only an explicitly selected custom theme can be replaced")
            }
        }
        let bundle = try ThemeBundle.capture(document: store.document, catalog: store.assets, root: root,
            name: name, id: replacingID ?? UUID())
        return try persist(bundle)
    }

    // ------------------------=
    // FUNC: duplicate
    // DESC: Creates an independent custom theme identity without changing the original theme or applying it to the editor.
    // ------------------=
    @discardableResult func duplicate(id: UUID, name: String) throws -> ThemeSummary {
        var copy = try bundle(id: id)
        copy.id = UUID(); copy.name = try ThemeBundle.checkedName(name)
        return try persist(copy)
    }

    // ------------------------=
    // FUNC: rename
    // DESC: Changes only a custom theme's visible name while preserving its stable identity and all saved appearance data.
    // ------------------=
    @discardableResult func rename(id: UUID, name: String) throws -> ThemeSummary {
        var value = try bundle(id: id)
        value.name = try ThemeBundle.checkedName(name)
        return try persist(value)
    }

    // ------------------------=
    // FUNC: delete
    // DESC: Moves one explicitly selected custom bundle to repository-local recoverable trash without deleting shared artwork or built-ins.
    // ------------------=
    @discardableResult func delete(id: UUID) throws -> URL {
        guard themes.contains(where: { $0.id == id && !$0.isBuiltIn }) else {
            throw UIKitIssue.invalid("Built-in themes cannot be removed")
        }
        let root = try requireRoot()
        let trash = try UIKitAssetCatalog.resolvedURL("assets/ui-design-kit/themes/.trash", root: root)
        try FileManager.default.createDirectory(at: trash, withIntermediateDirectories: true)
        let target = trash.appending(path: "\(id.uuidString.lowercased())-\(UUID().uuidString.lowercased()).infinitytheme")
        try FileManager.default.moveItem(at: file(id), to: target)
        lastTrashURL = target
        try reload()
        return target
    }

    // ------------------------=
    // FUNC: apply
    // DESC: Stages verified immutable uploads, then changes the editor document as one undoable operation without saving it implicitly.
    // ------------------=
    func apply(id: UUID, store: UIKitStore) throws {
        let root = try requireRoot(store: store)
        let selected = try bundle(id: id)
        try selected.validate(catalog: store.assets)
        let staging = try UIKitAssetCatalog.resolvedURL("assets/ui-design-kit/overrides/.theme-stage-\(UUID().uuidString)", root: root)
        try FileManager.default.createDirectory(at: staging, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: staging) }
        var staged: [(URL, URL)] = []
        var published: [URL] = []
        do {
            for item in selected.artwork {
                let target = try UIKitAssetCatalog.resolvedURL("assets/ui-design-kit/overrides/\(item.sha256).png", root: root)
                if FileManager.default.fileExists(atPath: target.path) {
                    guard ThemeBundle.digest(try Data(contentsOf: target)) == item.sha256 else {
                        throw UIKitIssue.invalid("Existing content-addressed artwork has been modified")
                    }
                } else {
                    let pending = staging.appending(path: item.sha256 + ".png")
                    try item.png.write(to: pending, options: .atomic)
                    staged.append((pending, target))
                }
            }
            for (pending, target) in staged {
                try FileManager.default.moveItem(at: pending, to: target)
                published.append(target)
            }
            try store.applyThemeDocument(selected.document, name: selected.name)
        } catch {
            for path in published { try? FileManager.default.removeItem(at: path) }
            throw error
        }
    }

    // ------------------------=
    // FUNC: importTheme
    // DESC: Imports a fully validated portable theme without overwriting a conflicting stable identity or changing the editor unless requested.
    // ------------------=
    @discardableResult func importTheme(from source: URL, applyTo store: UIKitStore? = nil) throws -> ThemeSummary {
        let root = try requireRoot(store: store)
        let incoming = try ThemeBundle.read(from: source, catalog: UIKitAssetCatalog.load(root: root))
        if themes.contains(where: { $0.id == incoming.id }) {
            guard try bundle(id: incoming.id) == incoming else {
                throw UIKitIssue.invalid("A different theme already uses this identity; duplicate it before importing a replacement")
            }
        } else {
            _ = try persist(incoming)
        }
        if let store { try apply(id: incoming.id, store: store) }
        return Self.summary(incoming, builtIn: Self.builtIns().contains { $0.id == incoming.id })
    }

    // ------------------------=
    // FUNC: exportDestination
    // DESC: Limits export writes to explicit project-local .infinitytheme destinations and rejects symlink traversal.
    // ------------------=
    private func exportDestination(_ destination: URL) throws -> URL {
        let root = try requireRoot()
        let target = destination.standardizedFileURL
        guard target.pathExtension.lowercased() == "infinitytheme", target.path.hasPrefix(root.path + "/"),
              target.resolvingSymlinksInPath().path.hasPrefix(root.path + "/") else {
            throw UIKitIssue.invalid("Export themes inside this InfinityOS project using the .infinitytheme extension")
        }
        var parent = root
        for component in target.path.dropFirst(root.path.count + 1).split(separator: "/") {
            parent.append(path: String(component))
            if let values = try? parent.resourceValues(forKeys: [.isSymbolicLinkKey]), values.isSymbolicLink == true {
                throw UIKitIssue.invalid("Theme exports cannot traverse symbolic links")
            }
        }
        guard !target.path.hasPrefix((try directory()).path + "/") else {
            throw UIKitIssue.invalid("Use Save Theme for the managed library; export a portable copy under builds/exports/themes")
        }
        return target
    }

    // ------------------------=
    // FUNC: export
    // DESC: Exports a selected saved or built-in theme as one atomic portable JSON bundle including all referenced artwork.
    // ------------------=
    func export(id: UUID, to destination: URL) throws {
        let root = try requireRoot()
        let target = try exportDestination(destination)
        let data = try bundle(id: id).encoded(catalog: UIKitAssetCatalog.load(root: root))
        try FileManager.default.createDirectory(at: target.deletingLastPathComponent(), withIntermediateDirectories: true)
        try data.write(to: target, options: .atomic)
    }

    // ------------------------=
    // FUNC: exportCurrent
    // DESC: Exports current draft edits without saving, applying, or otherwise changing the editor or named theme library.
    // ------------------=
    func exportCurrent(name: String, store: UIKitStore, to destination: URL) throws {
        let root = try requireRoot(store: store)
        let target = try exportDestination(destination)
        let selected = try ThemeBundle.capture(document: store.document, catalog: store.assets, root: root, name: name)
        let bytes = try selected.encoded(catalog: store.assets)
        try FileManager.default.createDirectory(at: target.deletingLastPathComponent(), withIntermediateDirectories: true)
        try bytes.write(to: target, options: .atomic)
    }
}
