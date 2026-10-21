import AppKit
import CryptoKit
import Foundation
import ImageIO
import UniformTypeIdentifiers

struct ThemeArtwork: Codable, Equatable {
    var sha256: String
    var png: Data
}

struct ThemeBundle: Codable, Equatable, Identifiable {
    static let maximumFileBytes = 192 * 1024 * 1024
    static let maximumArtworkBytes = 128 * 1024 * 1024
    static let maximumImageBytes = 32 * 1024 * 1024
    var version = 1
    var id: UUID
    var name: String
    var document: UIKitDocument
    var artwork: [ThemeArtwork] = []

    // ------------------------=
    // FUNC: checkedName
    // DESC: Validates a human theme name independently from its immutable UUID-based file identity.
    // ------------------=
    static func checkedName(_ name: String) throws -> String {
        let result = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !result.isEmpty, result.count <= 80,
              result.unicodeScalars.allSatisfy({ !CharacterSet.controlCharacters.contains($0) }) else {
            throw UIKitIssue.invalid("Theme names must contain 1–80 visible characters")
        }
        return result
    }

    // ------------------------=
    // FUNC: digest
    // DESC: Gives each immutable embedded PNG a content-addressed identity shared with UIKit's upload store.
    // ------------------=
    static func digest(_ data: Data) -> String {
        SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
    }

    // ------------------------=
    // FUNC: validate
    // DESC: Rejects oversized, forged, traversing or incompatible theme records before they can modify repository assets or editor state.
    // ------------------=
    func validate(catalog: [UIKitAsset]) throws {
        guard version == 1, name == (try Self.checkedName(name)),
              artwork.count <= 2048, document.overrides.count <= 2048 else {
            throw UIKitIssue.invalid("Unsupported or oversized theme bundle")
        }
        try document.validate()
        var images: [String: Data] = [:]
        var total = 0
        for image in artwork {
            total += image.png.count
            guard image.png.count <= Self.maximumImageBytes, total <= Self.maximumArtworkBytes,
                  image.sha256.count == 64, Self.digest(image.png) == image.sha256,
                  images.updateValue(image.png, forKey: image.sha256) == nil else {
                throw UIKitIssue.invalid("Theme artwork is oversized, duplicated or has an invalid SHA-256")
            }
        }
        var used = Set<String>()
        var spans: [String: [Range<Int>]] = [:]
        for item in document.overrides {
            guard let asset = catalog.first(where: { $0.id == item.id }),
                  asset.target == item.target, asset.offset == item.offset, asset.format == item.format,
                  asset.width == item.width, asset.height == item.height else {
                throw UIKitIssue.invalid("Theme element \(item.id) is incompatible with this OS catalog")
            }
            let hash = URL(fileURLWithPath: item.source).deletingPathExtension().lastPathComponent
            guard item.source == "assets/ui-design-kit/overrides/\(hash).png", let png = images[hash] else {
                throw UIKitIssue.invalid("Theme artwork must use its embedded content-addressed PNG")
            }
            try Self.validatePNG(png, width: item.width, height: item.height)
            let length = item.width * item.height * 4
            guard item.offset <= Int.max - length else {
                throw UIKitIssue.invalid("Theme artwork range exceeds native address bounds")
            }
            let range = item.offset..<(item.offset + length)
            guard !(spans[item.target] ?? []).contains(where: { $0.overlaps(range) }) else {
                throw UIKitIssue.invalid("Theme contains overlapping native artwork cells")
            }
            spans[item.target, default: []].append(range)
            used.insert(hash)
        }
        guard used == Set(images.keys) else {
            throw UIKitIssue.invalid("Theme contains unused embedded artwork")
        }
    }

    // ------------------------=
    // FUNC: validatePNG
    // DESC: Bounds and decodes actual PNG pixels, rejecting misleading extensions, dimensions, animations and unsupported raster precision.
    // ------------------=
    static func validatePNG(_ data: Data, width: Int, height: Int) throws {
        guard data.starts(with: [137, 80, 78, 71, 13, 10, 26, 10]),
              let source = CGImageSourceCreateWithData(data as CFData, nil),
              CGImageSourceGetType(source) as String? == UTType.png.identifier,
              CGImageSourceGetCount(source) == 1,
              let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
              properties[kCGImagePropertyPixelWidth] as? Int == width,
              properties[kCGImagePropertyPixelHeight] as? Int == height,
              let image = CGImageSourceCreateImageAtIndex(source, 0, [kCGImageSourceShouldCache: false] as CFDictionary),
              image.width == width, image.height == height, image.bitsPerComponent == 8,
              image.colorSpace?.model == .rgb else {
            throw UIKitIssue.invalid("Theme artwork must be a normalized 8-bit RGB PNG with the exact native dimensions")
        }
    }

    // ------------------------=
    // FUNC: capture
    // DESC: Packages the current document and only its referenced immutable uploads without altering unsaved editor changes.
    // ------------------=
    static func capture(document: UIKitDocument, catalog: [UIKitAsset], root: URL,
                        name: String, id: UUID = UUID()) throws -> ThemeBundle {
        try document.validate()
        var images: [String: Data] = [:]
        for item in document.overrides {
            let source = try UIKitAssetCatalog.resolvedURL(item.source, root: root)
            guard let size = try source.resourceValues(forKeys: [.fileSizeKey]).fileSize,
                  size <= maximumImageBytes else { throw UIKitIssue.invalid("Theme artwork is too large") }
            let data = try Data(contentsOf: source)
            images[Self.digest(data)] = data
        }
        let result = ThemeBundle(id: id, name: try checkedName(name), document: document,
            artwork: images.keys.sorted().map { ThemeArtwork(sha256: $0, png: images[$0]!) })
        try result.validate(catalog: catalog)
        return result
    }

    // ------------------------=
    // FUNC: encoded
    // DESC: Emits one portable versioned JSON document with embedded PNGs and a bounded total encoded size.
    // ------------------=
    func encoded(catalog: [UIKitAsset]) throws -> Data {
        try validate(catalog: catalog)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        let result = try encoder.encode(self)
        guard result.count <= Self.maximumFileBytes else { throw UIKitIssue.invalid("Theme bundle exceeds 192 MiB") }
        return result
    }

    // ------------------------=
    // FUNC: read
    // DESC: Reads a user-selected bundle with a pre-decode size limit, then validates its schema and actual embedded artwork.
    // ------------------=
    static func read(from url: URL, catalog: [UIKitAsset]) throws -> ThemeBundle {
        let values = try url.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
        guard values.isRegularFile == true, let size = values.fileSize, size > 0, size <= maximumFileBytes else {
            throw UIKitIssue.invalid("Select a regular theme bundle of at most 192 MiB")
        }
        let result = try JSONDecoder().decode(Self.self, from: Data(contentsOf: url))
        try result.validate(catalog: catalog)
        return result
    }
}
