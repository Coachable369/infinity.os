import AppKit
import CryptoKit
import Foundation
import ImageIO
import SwiftUI
import UniformTypeIdentifiers

@MainActor
final class UIKitStore: ObservableObject {
    @Published private(set) var document = UIKitDocument()
    @Published private(set) var assets: [UIKitAsset] = []
    @Published var selectedAssetID: String?
    @Published var selectedSurface = 2
    @Published var selectedCategory = "All Elements"
    @Published var searchText = ""
    @Published var status = "Select a UIKit element or semantic color"
    @Published var validationIssues: [String] = []
    let projectRoot: URL?
    private var savedDocument = UIKitDocument()
    private var sourceLoadIssue: String?
    private var undoStack: [UIKitDocument] = []
    private var redoStack: [UIKitDocument] = []
    private var imageCache: [String: NSImage] = [:]
    private var atlasCache: [String: Data] = [:]

    var selectedAsset: UIKitAsset? { assets.first { $0.id == selectedAssetID } }
    var categories: [String] { ["All Elements"] + Set(assets.map(\.category)).sorted() }
    var filteredAssets: [UIKitAsset] {
        assets.filter { asset in
            (selectedCategory == "All Elements" || asset.category == selectedCategory)
                && (searchText.isEmpty || "\(asset.title) \(asset.target) \(asset.category)"
                    .localizedCaseInsensitiveContains(searchText))
        }
    }
    var canUndo: Bool { !undoStack.isEmpty }
    var canRedo: Bool { !redoStack.isEmpty }
    var isDirty: Bool { document != savedDocument }

    // ------------------------=
    // FUNC: init
    // DESC: Opens the project UIKit document without creating files or changing any installed visual assets.
    // ------------------=
    init(projectRoot: URL?) {
        self.projectRoot = projectRoot?.standardizedFileURL
        do { try reload(refreshCatalog: false) }
        catch {
            sourceLoadIssue = "UIKit could not be loaded. Repair its source and reopen Studio before saving: \(error.localizedDescription)"
            validationIssues = [sourceLoadIssue!]; status = "UIKit could not be loaded"
        }
    }

    // ------------------------=
    // FUNC: reload
    // DESC: Loads validated customization state and rebuilds the catalog while preserving original repository artwork.
    // ------------------=
    func reload(refreshCatalog: Bool = true) throws {
        guard let root = projectRoot else { return }
        let url = root.appending(path: "assets/ui-design-kit/uikit.infinityui")
        let loaded = FileManager.default.fileExists(atPath: url.path)
            ? try JSONDecoder().decode(UIKitDocument.self, from: Data(contentsOf: url)) : UIKitDocument()
        try loaded.validate()
        let catalog = try UIKitAssetCatalog.load(root: root, refresh: refreshCatalog)
        try validateOverrides(loaded, catalog: catalog, root: root)
        document = loaded; savedDocument = loaded; assets = catalog
        sourceLoadIssue = nil
        undoStack = []; redoStack = []; validationIssues = []
        imageCache.removeAll(); atlasCache.removeAll()
        status = "Loaded \(catalog.count) OS artwork elements and \(loaded.colors.count) semantic colors"
    }

    // ------------------------=
    // FUNC: selectAsset
    // DESC: Selects a stable native element identity for preview and replacement.
    // ------------------=
    func selectAsset(_ id: String) {
        guard assets.contains(where: { $0.id == id }) else { return }
        selectedAssetID = id
    }

    // ------------------------=
    // FUNC: setColor
    // DESC: Changes one native semantic color without flattening the authored surface artwork.
    // ------------------=
    func setColor(_ role: UIKitColorRole, argb: UInt32) {
        guard document.colors[role.rawValue] != argb else { return }
        recordUndo(); document.colors[role.rawValue] = argb
        status = "Updated \(role.title)"
    }

    // ------------------------=
    // FUNC: resetColors
    // DESC: Restores the approved Eclipse Precision semantic palette as one undoable action.
    // ------------------=
    func resetColors() {
        guard document.colors != UIKitDocument.defaultColors else { return }
        recordUndo(); document.colors = UIKitDocument.defaultColors
        status = "Restored approved color tokens"
    }

    // ------------------------=
    // FUNC: setSurfaceStyle
    // DESC: Records an independent reusable OS surface recipe, retaining ordinary undo and save behavior.
    // ------------------=
    func setSurfaceStyle(_ index: Int, style: UIKitSurfaceStyle) {
        guard document.styles.indices.contains(index), (0...40).contains(style.radius),
              index >= 2 || style.corners != .rounded,
              document.styles[index] != style else { return }
        recordUndo(); document.styles[index] = style
        status = "Updated surface style; save to apply it to the OS"
    }

    // ------------------------=
    // FUNC: resetSurfaceStyle
    // DESC: Restores one surface family to approved authored geometry and lighting without altering other styles.
    // ------------------=
    func resetSurfaceStyle(_ index: Int) { setSurfaceStyle(index, style: UIKitSurfaceStyle()) }

    // ------------------------=
    // FUNC: applyThemeDocument
    // DESC: Applies a fully staged theme as one undoable unsaved edit while preserving original runtime artwork for every replacement.
    // ------------------=
    func applyThemeDocument(_ replacement: UIKitDocument, name: String) throws {
        try replacement.validate()
        guard let root = projectRoot else { throw UIKitIssue.invalid("Open an InfinityOS project before applying a theme") }
        try validateOverrides(replacement, catalog: assets, root: root)
        for item in replacement.overrides {
            guard let asset = assets.first(where: { $0.id == item.id }) else {
                throw UIKitIssue.invalid("The theme contains an unavailable native element")
            }
            try preserveOriginal(asset, root: root)
        }
        guard replacement != document else { status = "\(name) is already applied"; return }
        recordUndo(); document = replacement; validationIssues = []
        status = "Applied \(name); save this section to update the OS"
    }

    // ------------------------=
    // FUNC: importAsset
    // DESC: Validates and stores an alpha-preserving normalized replacement without overwriting its original runtime asset.
    // ------------------=
    func importAsset(from source: URL, for asset: UIKitAsset) throws {
        guard let root = projectRoot, assets.contains(asset),
              asset.width > 0, asset.height > 0,
              asset.width <= 8192, asset.height <= 8192, asset.width * asset.height <= 16_777_216,
              let inputSize = UIKitAssetCatalog.dimensions(source), inputSize.0 > 0, inputSize.1 > 0,
              inputSize.0 <= 8192, inputSize.1 <= 8192, inputSize.0 * inputSize.1 <= 16_777_216
        else { throw UIKitIssue.invalid("Choose a valid image of at most 16 megapixels for an existing OS element") }
        let png = try normalizedPNG(source, width: asset.width, height: asset.height)
        try preserveOriginal(asset, root: root)
        let hash = SHA256.hash(data: png).map { String(format: "%02x", $0) }.joined()
        let relative = "assets/ui-design-kit/overrides/\(hash).png"
        let destination = try UIKitAssetCatalog.resolvedURL(relative, root: root)
        try FileManager.default.createDirectory(at: destination.deletingLastPathComponent(), withIntermediateDirectories: true)
        if !FileManager.default.fileExists(atPath: destination.path) { try png.write(to: destination, options: .atomic) }
        let replacement = UIKitOverride(id: asset.id, target: asset.target, offset: asset.offset,
            width: asset.width, height: asset.height, source: relative, format: asset.format)
        guard document.overrides.first(where: { $0.id == asset.id }) != replacement else { return }
        recordUndo(); document.overrides.removeAll { $0.id == asset.id }
        document.overrides.append(replacement); selectedAssetID = asset.id
        status = "Replaced \(asset.title); save this section to include it in the next OS build"
    }

    // ------------------------=
    // FUNC: chooseReplacementImage
    // DESC: Opens the native image importer for the currently selected runtime element.
    // ------------------=
    func chooseReplacementImage() {
        guard let asset = selectedAsset else { return }
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.png, .bmp, .jpeg, .tiff]
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do { try importAsset(from: url, for: asset); validationIssues = [] }
        catch { validationIssues = [error.localizedDescription]; status = "Image replacement failed" }
    }

    // ------------------------=
    // FUNC: resetSelectedAsset
    // DESC: Removes the selected customization so the build restores the approved original artwork.
    // ------------------=
    func resetSelectedAsset() {
        guard let id = selectedAssetID, document.overrides.contains(where: { $0.id == id }) else { return }
        recordUndo(); document.overrides.removeAll { $0.id == id }
        status = "Restored original artwork; save to apply this change to the next build"
    }

    // ------------------------=
    // FUNC: validate
    // DESC: Validates token and artwork contracts against actual repository catalog entries before publication.
    // ------------------=
    func validate() throws {
        if let sourceLoadIssue { throw UIKitIssue.invalid(sourceLoadIssue) }
        try document.validate()
        guard let root = projectRoot else { throw UIKitIssue.invalid("Open an InfinityOS project before saving UIKit") }
        try validateOverrides(document, catalog: assets, root: root)
    }

    // ------------------------=
    // FUNC: save
    // DESC: Publishes authoritative customizations and native bytes together with rollback on an I/O failure.
    // ------------------=
    func save() throws {
        guard let root = projectRoot else { throw UIKitIssue.invalid("No InfinityOS project is selected") }
        try StudioFileTransaction.commit(prepareSave(), root: root)
        didSave()
    }

    // ------------------------=
    // FUNC: prepareSave
    // DESC: Validates and serializes all UIKit artifacts without mutating source or runtime files.
    // ------------------=
    func prepareSave() throws -> [StudioFileWrite] {
        try validate()
        guard let root = projectRoot else { throw UIKitIssue.invalid("No InfinityOS project is selected") }
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        let source = try encoder.encode(document)
        let binary = try document.runtimeTokens()
        let styles = try document.runtimeStyles()
        let documentURL = try UIKitAssetCatalog.resolvedURL("assets/ui-design-kit/uikit.infinityui", root: root)
        let tokenURL = try UIKitAssetCatalog.resolvedURL("assets/ui-design-kit/default/uikit-tokens.bin", root: root)
        let stylesURL = try UIKitAssetCatalog.resolvedURL("assets/ui-design-kit/default/uikit-styles.bin", root: root)
        return [StudioFileWrite(url: documentURL, data: source), StudioFileWrite(url: tokenURL, data: binary),
                StudioFileWrite(url: stylesURL, data: styles)]
    }

    // ------------------------=
    // FUNC: didSave
    // DESC: Marks the draft persisted only after the complete multi-artifact save succeeds.
    // ------------------=
    func didSave() {
        savedDocument = document; validationIssues = []
        status = "Saved UIKit styles, colors and artwork for live and fresh-install OS builds"
    }

    // ------------------------=
    // FUNC: previewImage
    // DESC: Previews replacement pixels or the exact original native cell while preserving straight-alpha coverage.
    // ------------------=
    func previewImage(for asset: UIKitAsset, maxDimension: Int? = nil) -> NSImage? {
        guard let root = projectRoot else { return nil }
        let key = "\(asset.id):\(maxDimension ?? 0)"
        if let image = imageCache[key] { return image }
        if let replacement = document.overrides.first(where: { $0.id == asset.id }),
           let url = try? UIKitAssetCatalog.resolvedURL(replacement.source, root: root) {
            return cacheImage(at: url, key: key, maxDimension: maxDimension)
        }
        let original = (try? baselineRegistry(root: root))?.files.first { $0.target == asset.target }?.source
        guard let url = try? UIKitAssetCatalog.resolvedURL(original ?? asset.target, root: root) else { return nil }
        if !asset.isRawRGBA { return cacheImage(at: url, key: key, maxDimension: maxDimension) }
        let count = asset.width * asset.height * 4
        guard count > 0, let data = try? Data(contentsOf: url),
              asset.offset >= 0, asset.offset <= data.count - count else { return nil }
        let bytes = data.subdata(in: asset.offset..<(asset.offset + count))
        guard let provider = CGDataProvider(data: bytes as CFData),
              let image = CGImage(width: asset.width, height: asset.height, bitsPerComponent: 8, bitsPerPixel: 32,
                bytesPerRow: asset.width * 4, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue), provider: provider,
                decode: nil, shouldInterpolate: true, intent: .defaultIntent) else { return nil }
        let result = NSImage(cgImage: image, size: NSSize(width: asset.width, height: asset.height))
        if imageCache.count >= 64 { imageCache.removeAll() }
        imageCache[key] = result
        return result
    }

    // ------------------------=
    // FUNC: previewAtlas
    // DESC: Supplies production-equivalent native bytes with unsaved artwork changes applied for the shared WYSIWYG renderer.
    // ------------------=
    func previewAtlas(target: String) -> Data? {
        if let data = atlasCache[target] { return data }
        guard let root = projectRoot, target.hasSuffix(".rgba") else { return nil }
        let original = (try? baselineRegistry(root: root))?.files.first { $0.target == target }?.source
        guard let url = try? UIKitAssetCatalog.resolvedURL(original ?? target, root: root),
              var data = try? Data(contentsOf: url) else { return nil }
        for item in document.overrides where item.target == target {
            guard let source = try? UIKitAssetCatalog.resolvedURL(item.source, root: root),
                  let bytes = try? rawRGBA(source, width: item.width, height: item.height),
                  item.offset >= 0, item.offset <= data.count - bytes.count else { return nil }
            data.replaceSubrange(item.offset..<(item.offset + bytes.count), with: bytes)
        }
        atlasCache[target] = data
        return data
    }

    // ------------------------=
    // FUNC: previewThemeAtlas
    // DESC: Composes portable theme artwork over immutable authored pixels without applying, extracting or saving the selected theme.
    // ------------------=
    func previewThemeAtlas(target: String, theme: ThemeBundle) -> Data? {
        guard let root = projectRoot, target.hasSuffix(".rgba") else { return nil }
        do {
            try theme.validate(catalog: assets)
            let original = try baselineRegistry(root: root).files.first { $0.target == target }?.source
            var data = try Data(contentsOf: UIKitAssetCatalog.resolvedURL(original ?? target, root: root))
            let images = Dictionary(uniqueKeysWithValues: theme.artwork.map { ($0.sha256, $0.png) })
            for item in theme.document.overrides where item.target == target {
                let hash = URL(fileURLWithPath: item.source).deletingPathExtension().lastPathComponent
                guard let png = images[hash] else { return nil }
                let bytes = try rawRGBA(png, width: item.width, height: item.height)
                guard item.offset >= 0, item.offset <= data.count - bytes.count else { return nil }
                data.replaceSubrange(item.offset..<(item.offset + bytes.count), with: bytes)
            }
            return data
        } catch { return nil }
    }

    // ------------------------=
    // FUNC: undo
    // DESC: Restores the preceding complete UIKit document without mutating original assets.
    // ------------------=
    func undo() {
        guard let previous = undoStack.popLast() else { return }
        redoStack.append(document); document = previous; status = "Undid UIKit change"
        imageCache.removeAll(); atlasCache.removeAll()
    }

    // ------------------------=
    // FUNC: redo
    // DESC: Reapplies a previously undone color or artwork customization.
    // ------------------=
    func redo() {
        guard let next = redoStack.popLast() else { return }
        undoStack.append(document); document = next; status = "Redid UIKit change"
        imageCache.removeAll(); atlasCache.removeAll()
    }

    // ------------------------=
    // FUNC: recordUndo
    // DESC: Keeps bounded section-local undo history without retaining decoded image buffers.
    // ------------------=
    private func recordUndo() {
        undoStack.append(document)
        if undoStack.count > 100 { undoStack.removeFirst() }
        redoStack.removeAll()
        imageCache.removeAll(); atlasCache.removeAll()
    }

    // ------------------------=
    // FUNC: cacheImage
    // DESC: Bounds decoded artwork memory and creates high-quality thumbnails without loading full-resolution wallpaper for a grid tile.
    // ------------------=
    private func cacheImage(at url: URL, key: String, maxDimension: Int?) -> NSImage? {
        var image: NSImage?
        if let maxDimension, let source = CGImageSourceCreateWithURL(url as CFURL, nil),
           let thumbnail = CGImageSourceCreateThumbnailAtIndex(source, 0, [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceThumbnailMaxPixelSize: max(1, maxDimension),
            kCGImageSourceCreateThumbnailWithTransform: true,
           ] as CFDictionary) {
            image = NSImage(cgImage: thumbnail, size: NSSize(width: thumbnail.width, height: thumbnail.height))
        } else { image = NSImage(contentsOf: url) }
        if image == nil, let bytes = try? Data(contentsOf: url), let native = NativeBitmapEncoder.image(bytes) {
            let ratio = min(1, CGFloat(maxDimension ?? max(native.width, native.height)) / CGFloat(max(native.width, native.height)))
            let width = max(1, Int(CGFloat(native.width) * ratio)), height = max(1, Int(CGFloat(native.height) * ratio))
            if let context = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8,
                bytesPerRow: width * 4, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) {
                context.interpolationQuality = .high
                context.draw(native, in: CGRect(x: 0, y: 0, width: width, height: height))
                if let thumbnail = context.makeImage() { image = NSImage(cgImage: thumbnail, size: NSSize(width: width, height: height)) }
            }
        }
        if let image {
            if imageCache.count >= 64 { imageCache.removeAll() }
            imageCache[key] = image
        }
        return image
    }

    // ------------------------=
    // FUNC: rawRGBA
    // DESC: Decodes a replacement into exact straight-alpha native cells without imposing host bitmap channel order.
    // ------------------=
    private func rawRGBA(_ url: URL, width: Int, height: Int) throws -> Data {
        try rawRGBA(Data(contentsOf: url), width: width, height: height)
    }

    // ------------------------=
    // FUNC: rawRGBA
    // DESC: Converts embedded PNG bytes to native straight-alpha pixels in memory so theme previews never materialize uploads.
    // ------------------=
    func rawRGBA(_ png: Data, width: Int, height: Int) throws -> Data {
        guard let source = CGImageSourceCreateWithData(png as CFData, nil),
              let image = CGImageSourceCreateImageAtIndex(source, 0, nil), image.width == width, image.height == height else {
            throw UIKitIssue.invalid("Replacement size does not match the native cell")
        }
        let native = try NativeBitmapEncoder.encode(image)
        var result = Data(count: width * height * 4)
        for at in stride(from: 0, to: result.count, by: 4) {
            result[at] = native[122 + at + 2]; result[at + 1] = native[122 + at + 1]
            result[at + 2] = native[122 + at]; result[at + 3] = native[122 + at + 3]
        }
        return result
    }

    // ------------------------=
    // FUNC: baselineRegistry
    // DESC: Reads durable original-image ownership independently from undoable overrides so reset remains reversible after builds.
    // ------------------=
    private func baselineRegistry(root: URL) throws -> UIKitBaselines {
        let url = try UIKitAssetCatalog.resolvedURL("assets/ui-design-kit/uikit-baselines.json", root: root)
        guard FileManager.default.fileExists(atPath: url.path) else { return UIKitBaselines() }
        let result = try JSONDecoder().decode(UIKitBaselines.self, from: Data(contentsOf: url))
        guard result.version == 1, Set(result.files.map(\.target)).count == result.files.count else {
            throw UIKitIssue.invalid("Invalid UIKit original-artwork registry")
        }
        for item in result.files {
            guard item.source.hasPrefix("assets/ui-design-kit/overrides/baselines/") else {
                throw UIKitIssue.invalid("Invalid original-artwork source")
            }
            let original = try Data(contentsOf: UIKitAssetCatalog.resolvedURL(item.source, root: root))
            guard SHA256.hash(data: original).map({ String(format: "%02x", $0) }).joined() == item.sha256 else {
                throw UIKitIssue.invalid("Original artwork has changed; refusing to overwrite user data")
            }
            _ = try UIKitAssetCatalog.resolvedURL(item.target, root: root)
        }
        return result
    }

    // ------------------------=
    // FUNC: preserveOriginal
    // DESC: Preserves immutable original bytes once so uploads and later resets never destroy the approved artwork.
    // ------------------=
    private func preserveOriginal(_ asset: UIKitAsset, root: URL) throws {
        var registry = try baselineRegistry(root: root)
        if registry.files.contains(where: { $0.target == asset.target }) { return }
        let original = try Data(contentsOf: UIKitAssetCatalog.resolvedURL(asset.target, root: root))
        let hash = SHA256.hash(data: original).map { String(format: "%02x", $0) }.joined()
        let relative = "assets/ui-design-kit/overrides/baselines/\(hash).original"
        let destination = try UIKitAssetCatalog.resolvedURL(relative, root: root)
        try FileManager.default.createDirectory(at: destination.deletingLastPathComponent(), withIntermediateDirectories: true)
        if !FileManager.default.fileExists(atPath: destination.path) { try original.write(to: destination, options: .atomic) }
        registry.files.append(UIKitBaseline(target: asset.target, source: relative, sha256: hash))
        let encoder = JSONEncoder(); encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        try encoder.encode(registry).write(to: UIKitAssetCatalog.resolvedURL("assets/ui-design-kit/uikit-baselines.json", root: root), options: .atomic)
    }

    // ------------------------=
    // FUNC: validateOverrides
    // DESC: Rejects forged replacement IDs, changed native geometry, escaped paths and missing uploaded pixels.
    // ------------------=
    private func validateOverrides(_ document: UIKitDocument, catalog: [UIKitAsset], root: URL) throws {
        for item in document.overrides {
            guard let asset = catalog.first(where: { $0.id == item.id }),
                  asset.target == item.target, asset.offset == item.offset, asset.format == item.format,
                  asset.width == item.width, asset.height == item.height,
                  let size = UIKitAssetCatalog.dimensions(try UIKitAssetCatalog.resolvedURL(item.source, root: root)),
                  size.0 == item.width, size.1 == item.height else {
                throw UIKitIssue.invalid("Replacement \(item.id) does not match its native element")
            }
            _ = try UIKitAssetCatalog.resolvedURL(item.target, root: root)
        }
    }

    // ------------------------=
    // FUNC: normalizedPNG
    // DESC: Resamples imported bitmap artwork once at its native footprint using high-quality filtering and retained alpha.
    // ------------------=
    private func normalizedPNG(_ url: URL, width: Int, height: Int) throws -> Data {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let image = CGImageSourceCreateImageAtIndex(source, 0, nil),
              let context = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8,
                bytesPerRow: width * 4, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue) else {
            throw UIKitIssue.invalid("The selected image could not be decoded")
        }
        context.interpolationQuality = .high
        context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
        let png = NSMutableData()
        guard let rendered = context.makeImage(),
              let destination = CGImageDestinationCreateWithData(png, UTType.png.identifier as CFString, 1, nil) else {
            throw UIKitIssue.invalid("The replacement image could not be converted to PNG")
        }
        CGImageDestinationAddImage(destination, rendered, nil)
        guard CGImageDestinationFinalize(destination) else {
            throw UIKitIssue.invalid("The replacement image could not be encoded as PNG")
        }
        return png as Data
    }
}
