import AppKit
import Foundation

enum TemplateValidationIssue: Error, Equatable, CustomStringConvertible {
    case invalidDocument(String)
    case invalidScreen(Int, String)
    case invalidElement(Int, UUID, String)

    var description: String {
        switch self {
        case let .invalidDocument(message): message
        case let .invalidScreen(screen, message): "Screen \(screen): \(message)"
        case let .invalidElement(screen, _, message): "Screen \(screen): \(message)"
        }
    }
}

enum TemplateValidator {
    // ------------------------=
    // FUNC: validate
    // DESC: Validates screen coverage, bounded geometry, roles, and immutable navigation controls.
    // ------------------=
    static func validate(_ document: InstallerStudioDocument) throws {
        guard document.version == InstallerStudioDocument.currentVersion else {
            throw TemplateValidationIssue.invalidDocument("Unsupported document version")
        }
        guard document.canvasWidth == 1000, document.canvasHeight == 1000 else {
            throw TemplateValidationIssue.invalidDocument("Canvas must use normalized 1000 × 1000 coordinates")
        }
        guard (InstallerStudioDocument.minimumScreenCount...InstallerStudioDocument.maximumScreenCount).contains(document.screens.count),
              document.screens.map(\.id) == Array(1...document.screens.count)
        else {
            throw TemplateValidationIssue.invalidDocument("Installer screens must be ordered and numbered contiguously")
        }
        for screen in document.screens {
            guard !screen.title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
                  screen.title.utf8.count <= 63
            else {
                throw TemplateValidationIssue.invalidScreen(screen.id, "Screen name must contain 1 to 63 UTF-8 bytes")
            }
            let back = screen.elements.filter { $0.role == .backButton }
            let primary = screen.elements.filter { $0.role == .primaryButton }
            let liveInputs = screen.elements.filter { $0.role == .input && !$0.hidden }
            guard back.count == 1, primary.count == 1 else {
                throw TemplateValidationIssue.invalidScreen(screen.id, "Canonical Back and Primary buttons are required")
            }
            guard back[0].kind == .button, primary[0].kind == .button,
                  !back[0].hidden, !primary[0].hidden
            else {
                throw TemplateValidationIssue.invalidScreen(screen.id, "Visible Back and Primary button actions are required")
            }
            guard screen.elements.contains(where: { $0.role == .console }) else {
                throw TemplateValidationIssue.invalidScreen(screen.id, "A console frame is required")
            }
            guard liveInputs.count <= 1 else {
                throw TemplateValidationIssue.invalidScreen(screen.id, "Only one live input field is supported per screen")
            }
            for element in screen.elements {
                guard element.kind != .progressBar || element.role == .progressBar,
                      element.role != .progressBar || element.kind == .progressBar,
                      element.role != .progressHero || element.kind == .image
                else {
                    throw TemplateValidationIssue.invalidElement(
                        screen.id, element.id, "Progress controls require their semantic element types"
                    )
                }
                guard element.role == .input || element.inputVariable == .none else {
                    throw TemplateValidationIssue.invalidElement(
                        screen.id, element.id, "Only input fields can bind runtime variables"
                    )
                }
                guard element.role != .input || element.hidden || element.inputVariable != .none else {
                    throw TemplateValidationIssue.invalidElement(
                        screen.id, element.id, "Visible input fields require a runtime variable"
                    )
                }
                guard element.frame == element.frame.clamped() else {
                    throw TemplateValidationIssue.invalidElement(screen.id, element.id, "Element lies outside the artboard")
                }
                guard element.name.utf8.count <= 63,
                      element.text.utf8.count <= 512,
                      element.imageAsset.utf8.count <= 127
                else {
                    throw TemplateValidationIssue.invalidElement(screen.id, element.id, "Element strings exceed runtime limits")
                }
                guard element.crop == element.crop.clamped() else {
                    throw TemplateValidationIssue.invalidElement(screen.id, element.id, "Image crop leaves no visible area")
                }
            }
        }
    }
}

enum RuntimeTemplateCodec {
    static let magic = Data([0x49, 0x55, 0x49, 0x54])
    static let version: UInt16 = 5

    // ------------------------=
    // FUNC: encode
    // DESC: Encodes a validated editor document into the deterministic InfinityOS runtime format.
    // ------------------=
    static func encode(_ document: InstallerStudioDocument, assetRoot: URL? = nil) throws -> Data {
        try TemplateValidator.validate(document)
        var output = Data()
        output.append(magic)
        output.appendLittleEndian(version)
        output.appendLittleEndian(UInt16(document.screens.count))
        for screen in document.screens {
            output.append(UInt8(screen.id))
            output.appendLengthPrefixed(screen.title, length: .u8, limit: 63)
            output.appendLittleEndian(UInt16(screen.elements.count))
            for element in screen.elements {
                output.append(contentsOf: element.id.bytes)
                output.append(element.kind.rawValue)
                output.append(element.role.rawValue)
                output.append(element.inputVariable.runtimeCode)
                var flags: UInt8 = 0
                if element.locked { flags |= 1 }
                if element.hidden { flags |= 2 }
                output.append(flags)
                output.appendLittleEndian(Int16(element.zIndex.clamped(to: -32_768...32_767)))
                output.appendLittleEndian(UInt16(element.frame.x))
                output.appendLittleEndian(UInt16(element.frame.y))
                output.appendLittleEndian(UInt16(element.frame.width))
                output.appendLittleEndian(UInt16(element.frame.height))
                output.append(contentsOf: [
                    element.fill.red, element.fill.green, element.fill.blue, element.fill.alpha,
                    element.border.red, element.border.green, element.border.blue, element.border.alpha,
                    UInt8(element.opacity.clamped(to: 0...100)),
                    UInt8(element.cornerRadius.clamped(to: 0...255)),
                ])
                output.appendLittleEndian(UInt16(element.fontSize.clamped(to: 6...256)))
                output.appendLengthPrefixed(element.name, length: .u8, limit: 63)
                output.appendLengthPrefixed(element.text, length: .u16, limit: 512)
                output.appendLengthPrefixed(element.imageAsset, length: .u8, limit: 127)
                output.append(contentsOf: [
                    UInt8(element.crop.left), UInt8(element.crop.top),
                    UInt8(element.crop.right), UInt8(element.crop.bottom),
                ])
            }
        }
        let assetNames = Array(Set(document.screens.flatMap { screen in
            screen.elements.compactMap { element in
                element.kind == .image && !element.imageAsset.isEmpty ? element.imageAsset : nil
            }
        })).sorted()
        var packagedAssets: [(String, Data)] = []
        if let assetRoot {
            for name in assetNames where !isBuiltInImage(name) {
                if let bitmap = try runtimeBitmap(named: name, root: assetRoot) {
                    packagedAssets.append((name, bitmap))
                }
            }
        }
        output.appendLittleEndian(UInt16(packagedAssets.count))
        for (name, bytes) in packagedAssets {
            output.appendLengthPrefixed(name, length: .u8, limit: 127)
            output.appendLittleEndian(UInt32(bytes.count))
            output.append(bytes)
        }
        return output
    }

    // ------------------------=
    // FUNC: runtimeBitmap
    // DESC: Resolves an editor image and converts it to the kernel's deterministic 24-bit BMP payload.
    // ------------------=
    private static func runtimeBitmap(named name: String, root: URL) throws -> Data? {
        let candidates = [root.appending(path: name), root.appending(path: "assets/boot/\(name)")]
        guard let source = candidates.first(where: { FileManager.default.fileExists(atPath: $0.path) }) else {
            if isBuiltInImage(name) {
                return nil
            }
            throw TemplateValidationIssue.invalidDocument("Image asset cannot be packaged: \(name)")
        }
        guard let image = NSImage(contentsOf: source),
              let tiff = image.tiffRepresentation,
              let bitmap = NSBitmapImageRep(data: tiff),
              let data = bitmap.representation(using: .bmp, properties: [:])
        else {
            throw TemplateValidationIssue.invalidDocument("Image asset cannot be packaged: \(name)")
        }
        return data
    }

    // ------------------------=
    // FUNC: isBuiltInImage
    // DESC: Identifies artwork already compiled once into both installer and installed kernels.
    // ------------------=
    private static func isBuiltInImage(_ name: String) -> Bool {
        name.hasSuffix("infinity-installer-masthead-v2.png")
            || name.hasSuffix("infinity-installer-masthead-v1.png")
            || name.hasSuffix("infinity-onboarding-wallpaper-v1.png")
            || name.hasSuffix("infinity-time-zone-map-v1.png")
            || name.hasSuffix("infinity-installer-progress-hero-v1.png")
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Decodes runtime data back into an editable document and rejects malformed records.
    // ------------------=
    static func decode(_ data: Data) throws -> InstallerStudioDocument {
        var reader = DataReader(data: data)
        guard try reader.readData(count: 4) == magic else {
            throw TemplateValidationIssue.invalidDocument("Invalid runtime-template signature")
        }
        guard try reader.readUInt16() == version else {
            throw TemplateValidationIssue.invalidDocument("Unsupported runtime-template version")
        }
        let screenCount = Int(try reader.readUInt16())
        var screens: [InstallerScreenTemplate] = []
        for _ in 0..<screenCount {
            let screenID = Int(try reader.readUInt8())
            let screenTitle = try reader.readString(length: .u8, limit: 63)
            let elementCount = Int(try reader.readUInt16())
            var elements: [StudioElement] = []
            for _ in 0..<elementCount {
                let id = try UUID(bytes: reader.readBytes(count: 16))
                guard let kind = StudioElementKind(rawValue: try reader.readUInt8()),
                      let role = StudioElementRole(rawValue: try reader.readUInt8()),
                      let inputVariable = StudioInputVariable(runtimeCode: try reader.readUInt8())
                else {
                    throw TemplateValidationIssue.invalidScreen(screenID, "Unknown element kind or role")
                }
                let flags = try reader.readUInt8()
                let zIndex = Int(try reader.readInt16())
                let frame = CanvasRect(
                    x: Int(try reader.readUInt16()),
                    y: Int(try reader.readUInt16()),
                    width: Int(try reader.readUInt16()),
                    height: Int(try reader.readUInt16())
                )
                let fill = try reader.readColor()
                let border = try reader.readColor()
                let opacity = Int(try reader.readUInt8())
                let cornerRadius = Int(try reader.readUInt8())
                let fontSize = Int(try reader.readUInt16())
                let name = try reader.readString(length: .u8, limit: 63)
                let text = try reader.readString(length: .u16, limit: 512)
                let imageAsset = try reader.readString(length: .u8, limit: 127)
                let crop = ImageCrop(
                    left: Int(try reader.readUInt8()), top: Int(try reader.readUInt8()),
                    right: Int(try reader.readUInt8()), bottom: Int(try reader.readUInt8())
                )
                elements.append(StudioElement(
                    id: id,
                    name: name,
                    kind: kind,
                    role: role,
                    inputVariable: inputVariable,
                    frame: frame,
                    text: text,
                    imageAsset: imageAsset,
                    crop: crop,
                    fill: fill,
                    border: border,
                    fontSize: fontSize,
                    opacity: opacity,
                    cornerRadius: cornerRadius,
                    zIndex: zIndex,
                    locked: flags & 1 != 0,
                    hidden: flags & 2 != 0
                ))
            }
            screens.append(InstallerScreenTemplate(
                id: screenID,
                title: screenTitle,
                elements: elements
            ))
        }
        let assetCount = Int(try reader.readUInt16())
        for _ in 0..<assetCount {
            _ = try reader.readString(length: .u8, limit: 127)
            let byteCount = Int(try reader.readUInt32())
            _ = try reader.readData(count: byteCount)
        }
        guard reader.isAtEnd else {
            throw TemplateValidationIssue.invalidDocument("Trailing runtime-template bytes")
        }
        let document = InstallerStudioDocument(
            version: InstallerStudioDocument.currentVersion,
            canvasWidth: 1000,
            canvasHeight: 1000,
            screens: screens
        )
        try TemplateValidator.validate(document)
        return document
    }
}

enum LengthField {
    case u8
    case u16
}

private struct DataReader {
    let data: Data
    var offset = 0
    var isAtEnd: Bool { offset == data.count }

    // ------------------------=
    // FUNC: readData
    // DESC: Reads a bounded data slice from the current runtime-template cursor.
    // ------------------=
    mutating func readData(count: Int) throws -> Data {
        guard count >= 0, offset + count <= data.count else {
            throw TemplateValidationIssue.invalidDocument("Truncated runtime template")
        }
        defer { offset += count }
        return data.subdata(in: offset..<(offset + count))
    }

    // ------------------------=
    // FUNC: readBytes
    // DESC: Reads a bounded byte array from the runtime template.
    // ------------------=
    mutating func readBytes(count: Int) throws -> [UInt8] {
        Array(try readData(count: count))
    }

    // ------------------------=
    // FUNC: readUInt8
    // DESC: Reads one unsigned byte from the runtime template.
    // ------------------=
    mutating func readUInt8() throws -> UInt8 {
        try readBytes(count: 1)[0]
    }

    // ------------------------=
    // FUNC: readUInt16
    // DESC: Reads one little-endian unsigned 16-bit value.
    // ------------------=
    mutating func readUInt16() throws -> UInt16 {
        let bytes = try readBytes(count: 2)
        return UInt16(bytes[0]) | UInt16(bytes[1]) << 8
    }

    // ------------------------=
    // FUNC: readUInt32
    // DESC: Reads one little-endian unsigned 32-bit value.
    // ------------------=
    mutating func readUInt32() throws -> UInt32 {
        let bytes = try readBytes(count: 4)
        return UInt32(bytes[0]) | UInt32(bytes[1]) << 8 | UInt32(bytes[2]) << 16 | UInt32(bytes[3]) << 24
    }

    // ------------------------=
    // FUNC: readInt16
    // DESC: Reads one little-endian signed 16-bit value.
    // ------------------=
    mutating func readInt16() throws -> Int16 {
        Int16(bitPattern: try readUInt16())
    }

    // ------------------------=
    // FUNC: readColor
    // DESC: Reads one RGBA appearance token.
    // ------------------=
    mutating func readColor() throws -> StudioColor {
        StudioColor(
            red: try readUInt8(),
            green: try readUInt8(),
            blue: try readUInt8(),
            alpha: try readUInt8()
        )
    }

    // ------------------------=
    // FUNC: readString
    // DESC: Reads and UTF-8 validates one bounded length-prefixed string.
    // ------------------=
    mutating func readString(length: LengthField, limit: Int) throws -> String {
        let count = switch length {
        case .u8: Int(try readUInt8())
        case .u16: Int(try readUInt16())
        }
        guard count <= limit,
              let value = String(data: try readData(count: count), encoding: .utf8)
        else {
            throw TemplateValidationIssue.invalidDocument("Invalid runtime-template string")
        }
        return value
    }
}

private extension Data {
    // ------------------------=
    // FUNC: appendLittleEndian
    // DESC: Appends one fixed-width little-endian integer to template data.
    // ------------------=
    mutating func appendLittleEndian<T: FixedWidthInteger>(_ value: T) {
        var encoded = value.littleEndian
        Swift.withUnsafeBytes(of: &encoded) { append(contentsOf: $0) }
    }

    // ------------------------=
    // FUNC: appendLengthPrefixed
    // DESC: Appends one UTF-8 string with its bounded runtime length field.
    // ------------------=
    mutating func appendLengthPrefixed(_ value: String, length: LengthField, limit: Int) {
        let bytes = Array(value.utf8.prefix(limit))
        switch length {
        case .u8: append(UInt8(bytes.count))
        case .u16: appendLittleEndian(UInt16(bytes.count))
        }
        append(contentsOf: bytes)
    }
}

private extension UUID {
    var bytes: [UInt8] {
        withUnsafeBytes(of: uuid) { Array($0) }
    }

    // ------------------------=
    // FUNC: init_bytes
    // DESC: Reconstructs a stable UUID from its 16 runtime bytes.
    // ------------------=
    init(bytes: [UInt8]) throws {
        guard bytes.count == 16 else {
            throw TemplateValidationIssue.invalidDocument("Invalid element identifier")
        }
        self = bytes.withUnsafeBytes { raw in
            UUID(uuid: raw.load(as: uuid_t.self))
        }
    }
}
