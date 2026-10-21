import AppKit
import Foundation
import ImageIO

enum UIKitColorRole: Int, CaseIterable, Identifiable, Codable {
    case canvas, panel, panelRaised, border, textPrimary, textSecondary
    case accent, accentBright, focus, success, warning, danger, scrim

    var id: Int { rawValue }
    var title: String {
        ["Canvas", "Panel", "Elevated Panel", "Border", "Primary Text", "Secondary Text",
         "Accent / Primary", "Bright Accent", "Focus", "Success", "Warning", "Danger", "Scrim"][rawValue]
    }
}

struct UIKitOverride: Codable, Equatable, Identifiable {
    var id: String
    var target: String
    var offset: Int
    var width: Int
    var height: Int
    var source: String
    var format: String = "rgba"
}

struct UIKitBaseline: Codable, Equatable {
    var target: String
    var source: String
    var sha256: String
    var appliedSHA256: String?
}

struct UIKitBaselines: Codable {
    var version = 1
    var files: [UIKitBaseline] = []
}

struct UIKitDocument: Codable, Equatable {
    var version = 1
    var colors: [UInt32] = Self.defaultColors
    var overrides: [UIKitOverride] = []
    var styles: [UIKitSurfaceStyle] = Array(repeating: UIKitSurfaceStyle(), count: 6)

    private enum CodingKeys: String, CodingKey { case version, colors, overrides, styles }

    // ------------------------=
    // FUNC: init
    // DESC: Creates the approved color and authored surface defaults without rewriting artwork.
    // ------------------=
    init() {}

    // ------------------------=
    // FUNC: init
    // DESC: Reads existing UIKit projects while preserving authored surfaces when no style overrides were saved.
    // ------------------=
    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        version = try values.decode(Int.self, forKey: .version)
        colors = try values.decode([UInt32].self, forKey: .colors)
        overrides = try values.decodeIfPresent([UIKitOverride].self, forKey: .overrides) ?? []
        styles = try values.decodeIfPresent([UIKitSurfaceStyle].self, forKey: .styles)
            ?? Array(repeating: UIKitSurfaceStyle(), count: 6)
    }

    static let defaultColors: [UInt32] = [
        0xff090f26, 0xe6101b3a, 0xf01a2e63, 0xff86c7ff, 0xffffffff, 0xffbebebe,
        0xff3e6fff, 0xff86c7ff, 0xffffffff, 0xff50d890, 0xffffc857, 0xffff6170, 0x99000000,
    ]

    // ------------------------=
    // FUNC: validate
    // DESC: Rejects unknown documents, duplicate targets, unsafe paths and invalid native image bounds before saving.
    // ------------------=
    func validate() throws {
        guard version == 1, colors.count == UIKitColorRole.allCases.count else {
            throw UIKitIssue.invalid("Unsupported UIKit version or color-role count")
        }
        guard styles.count == 6, styles.allSatisfy({ (0...40).contains($0.radius) }) else {
            throw UIKitIssue.invalid("UIKit requires six surface styles with corner radii between 0 and 40")
        }
        guard styles[0].corners != .rounded, styles[1].corners != .rounded else {
            throw UIKitIssue.invalid("Outer application windows use the shared rectangular OS frame; rounded corners are available for dialogs, panels, buttons and fields")
        }
        guard Set(overrides.map(\.id)).count == overrides.count else {
            throw UIKitIssue.invalid("A UIKit element has multiple replacement images")
        }
        var targets = Set<String>()
        for item in overrides {
            guard UIKitAssetCatalog.isSafeAssetPath(item.target),
                  UIKitAssetCatalog.isSafeAssetPath(item.source),
                  item.source.hasPrefix("assets/ui-design-kit/overrides/"),
                  item.offset >= 0, item.width > 0, item.height > 0,
                  ["rgba", "png", "bmp"].contains(item.format),
                  item.width <= 8192, item.height <= 8192,
                  item.width * item.height <= 16_777_216,
                  targets.insert("\(item.target):\(item.offset)").inserted
            else { throw UIKitIssue.invalid("Invalid replacement for \(item.id)") }
        }
    }

    // ------------------------=
    // FUNC: runtimeTokens
    // DESC: Encodes the exact versioned little-endian semantic color contract consumed by both OS targets.
    // ------------------=
    func runtimeTokens() throws -> Data {
        try validate()
        var result = Data("IUIKIT01".utf8)
        result.append(contentsOf: [1, 0, UInt8(colors.count), 0])
        for color in colors {
            result.append(contentsOf: (0..<4).map { UInt8(truncatingIfNeeded: color >> ($0 * 8)) })
        }
        return result
    }

    // ------------------------=
    // FUNC: runtimeStyles
    // DESC: Encodes optional surface materials, geometry and independent tint overrides for the shared native compositor.
    // ------------------=
    func runtimeStyles() throws -> Data {
        try validate()
        var result = Data("IUISTY01".utf8)
        result.append(contentsOf: [1, 0, 6, 0])
        for style in styles {
            let flags: UInt8 = (style.fill == nil ? 0 : 1) | (style.border == nil ? 0 : 2)
            result.append(contentsOf: [UInt8(style.material.rawValue), UInt8(style.corners.rawValue), UInt8(style.radius), flags])
            for color in [style.fill ?? 0, style.border ?? 0] {
                result.append(contentsOf: (0..<4).map { UInt8(truncatingIfNeeded: color >> ($0 * 8)) })
            }
        }
        return result
    }
}

enum UIKitIssue: Error, LocalizedError {
    case invalid(String)
    var errorDescription: String? {
        switch self { case let .invalid(message): message }
    }
}

struct UIKitAsset: Codable, Hashable, Identifiable {
    var id: String
    var label: String
    var group: String
    var target: String
    var offset: Int
    var width: Int
    var height: Int
    var format: String

    var title: String { label }
    var category: String { group }
    var isRawRGBA: Bool { format == "rgba" || target.hasSuffix(".rgba") }
    var runtimeDescription: String {
        "\(width) × \(height) · \(isRawRGBA ? "RGBA runtime cell" : format.uppercased()) · \(target)"
    }

    enum CodingKeys: String, CodingKey { case id, label, group, target, offset, width, height, format }

    // ------------------------=
    // FUNC: init
    // DESC: Creates a repository-backed artwork record with a fixed runtime footprint.
    // ------------------=
    init(id: String, label: String, group: String, target: String, offset: Int = 0,
         width: Int, height: Int, format: String) {
        self.id = id; self.label = label; self.group = group; self.target = target
        self.offset = offset; self.width = width; self.height = height; self.format = format
    }

    // ------------------------=
    // FUNC: init
    // DESC: Reads the shared build catalog while inferring the format for older explicit native-cell records.
    // ------------------=
    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        id = try values.decode(String.self, forKey: .id)
        label = try values.decode(String.self, forKey: .label)
        group = try values.decode(String.self, forKey: .group)
        target = try values.decode(String.self, forKey: .target)
        offset = try values.decodeIfPresent(Int.self, forKey: .offset) ?? 0
        width = try values.decode(Int.self, forKey: .width)
        height = try values.decode(Int.self, forKey: .height)
        format = try values.decodeIfPresent(String.self, forKey: .format)
            ?? URL(fileURLWithPath: target).pathExtension.lowercased()
    }
}

enum UIKitAssetCatalog {
    private struct Catalog: Decodable { var version: Int; var elements: [UIKitAsset] }
    @MainActor private static var cache: [URL: (Date?, [UIKitAsset])] = [:]

    // ------------------------=
    // FUNC: load
    // DESC: Loads build-owned runtime cells and discovers repository raster artwork without copying or modifying originals.
    // ------------------=
    @MainActor static func load(root: URL, refresh: Bool = false) throws -> [UIKitAsset] {
        let catalogURL = root.appending(path: "assets/ui-design-kit/uikit-catalog.json")
        let stamp = try? catalogURL.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate
        if !refresh, let cached = cache[root], cached.0 == stamp { return cached.1 }
        var result: [UIKitAsset] = []
        if FileManager.default.fileExists(atPath: catalogURL.path) {
            let data = try Data(contentsOf: catalogURL)
            if let array = try? JSONDecoder().decode([UIKitAsset].self, from: data) {
                result = array
            } else {
                let catalog = try JSONDecoder().decode(Catalog.self, from: data)
                guard catalog.version == 1 else { throw UIKitIssue.invalid("Unsupported UIKit catalog version") }
                result = catalog.elements
            }
        }
        var targets = Set(result.map(\.target))
        let assetsURL = root.appending(path: "assets", directoryHint: .isDirectory)
        let enumerator = FileManager.default.enumerator(at: assetsURL,
            includingPropertiesForKeys: [.isRegularFileKey, .isSymbolicLinkKey], options: [.skipsHiddenFiles])
        while let url = enumerator?.nextObject() as? URL {
            let relative = String(url.path.dropFirst(root.path.count + 1))
            if relative.hasPrefix("assets/ui-design-kit/overrides/") { continue }
            let ext = url.pathExtension.lowercased()
            guard ["png", "bmp"].contains(ext),
                  !url.lastPathComponent.contains("-reference"), !url.lastPathComponent.contains("-preview"),
                  !targets.contains(relative),
                  try url.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey]).isRegularFile == true,
                  try url.resourceValues(forKeys: [.isSymbolicLinkKey]).isSymbolicLink != true,
                  let size = dimensions(url), size.0 > 0, size.1 > 0
            else { continue }
            targets.insert(relative)
            result.append(UIKitAsset(id: "asset." + relative, label: url.deletingPathExtension().lastPathComponent,
                group: category(relative), target: relative, width: size.0, height: size.1, format: ext))
        }
        result.sort { ($0.group, $0.label, $0.id) < ($1.group, $1.label, $1.id) }
        if cache.count >= 16 { cache.removeAll() }
        cache[root] = (stamp, result)
        return result
    }

    // ------------------------=
    // FUNC: isSafeAssetPath
    // DESC: Allows normalized repository asset paths only, never absolute paths or traversal components.
    // ------------------=
    static func isSafeAssetPath(_ path: String) -> Bool {
        path.hasPrefix("assets/") && !path.contains("\\") && !path.contains("\0")
            && path.split(separator: "/", omittingEmptySubsequences: false).allSatisfy { !$0.isEmpty && $0 != "." && $0 != ".." }
    }

    // ------------------------=
    // FUNC: resolvedURL
    // DESC: Resolves a normalized path while preventing asset or parent symlinks from escaping the selected repository.
    // ------------------=
    static func resolvedURL(_ path: String, root: URL) throws -> URL {
        guard isSafeAssetPath(path) else { throw UIKitIssue.invalid("Invalid asset path") }
        let base = root.resolvingSymlinksInPath().standardizedFileURL
        var parent = base
        for component in path.split(separator: "/") {
            parent.append(path: String(component))
            if let attributes = try? FileManager.default.attributesOfItem(atPath: parent.path),
               attributes[.type] as? FileAttributeType == .typeSymbolicLink {
                throw UIKitIssue.invalid("UIKit asset paths cannot traverse symbolic links")
            }
        }
        let url = base.appending(path: path).resolvingSymlinksInPath().standardizedFileURL
        guard url.path.hasPrefix(base.path + "/assets/") else {
            throw UIKitIssue.invalid("UIKit assets must remain inside this repository")
        }
        return url
    }

    // ------------------------=
    // FUNC: dimensions
    // DESC: Reads pixel dimensions without decompressing every image in the complete OS artwork catalog.
    // ------------------=
    static func dimensions(_ url: URL) -> (Int, Int)? {
        if let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
              let width = properties[kCGImagePropertyPixelWidth] as? Int,
              let height = properties[kCGImagePropertyPixelHeight] as? Int { return (width, height) }
        guard url.pathExtension.lowercased() == "bmp",
              let size = try? url.resourceValues(forKeys: [.fileSizeKey]).fileSize, size <= 128 * 1024 * 1024,
              let bytes = try? Data(contentsOf: url, options: .mappedIfSafe) else { return nil }
        return NativeBitmapEncoder.dimensions(bytes)
    }

    // ------------------------=
    // FUNC: category
    // DESC: Groups discovered OS artwork by its existing repository ownership rather than an artificial duplicate hierarchy.
    // ------------------=
    private static func category(_ path: String) -> String {
        let components = path.split(separator: "/")
        guard components.count > 1 else { return "Other Artwork" }
        switch components[1] {
        case "icons": return "Application & System Icons"
        case "boot": return "Installer & Configuration Artwork"
        case "desktop": return "Desktop & Wallpapers"
        case "apps": return "Application Artwork"
        case "cursors": return "Pointer Artwork"
        case "fonts": return "Font Atlases"
        case "ui-design-kit": return "Design Kit Artwork"
        default: return String(components[1]).capitalized + " Artwork"
        }
    }
}
