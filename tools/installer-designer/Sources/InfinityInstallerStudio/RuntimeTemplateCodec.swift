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
    static func validate(
        _ document: InstallerStudioDocument,
        for explicitCollection: ScreenCollection? = nil
    ) throws {
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
        let collection = explicitCollection
            ?? (document.screens.contains { $0.elements.contains { $0.role.rawValue >= 49 } } ? .desktop
                : ScreenCollection.runtimeCollection(screenCount: document.screens.count))
        if collection == .login && document.screens.count != 1 {
            throw TemplateValidationIssue.invalidDocument("Login requires exactly one authentication screen")
        }
        if collection == .desktop && document.screens.count < 3 {
            throw TemplateValidationIssue.invalidDocument("Desktop requires its navigation, overview and AI widget scenes")
        }
        if collection == .boot && document.screens.count != 2 {
            throw TemplateValidationIssue.invalidDocument("Boot loading requires the ISO and installed scenes")
        }
        if collection == .systemSettings && document.screens.count != ScreenCollection.systemSettings.requiredScreenCount {
            throw TemplateValidationIssue.invalidDocument("System Settings requires its eleven native sections")
        }
        if let menu = document.infinityMenu {
            guard collection == .desktop else { throw TemplateValidationIssue.invalidDocument("Infinity Menu belongs to the desktop collection") }
            try menu.validate(screens: document.screens)
        }
        for screen in document.screens {
            guard screen.elements.count <= Int(UInt16.max) else {
                throw TemplateValidationIssue.invalidScreen(screen.id, "Element count exceeds the native format limit")
            }
            guard !screen.title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
                  screen.title.utf8.count <= 63
            else {
                throw TemplateValidationIssue.invalidScreen(screen.id, "Screen name must contain 1 to 63 UTF-8 bytes")
            }
            let back = screen.elements.filter { $0.role == .backButton }
            let primary = screen.elements.filter { $0.role == .primaryButton }
            let liveInputs = screen.elements.filter { $0.role == .input && !$0.hidden }
            if collection == .login {
                guard liveInputs.count == 1, liveInputs[0].inputVariable == .password,
                      primary.count == 1, primary[0].kind == .button, !primary[0].hidden,
                      primary[0].opacity > 0, liveInputs[0].opacity > 0 else {
                    throw TemplateValidationIssue.invalidScreen(screen.id, "Visible password and sign-in controls are required")
                }
            }
            let identifiers = screen.elements.map(\.codeIdentifier)
            guard Set(identifiers).count == identifiers.count,
                  Set(screen.elements.map(\.id)).count == screen.elements.count else {
                throw TemplateValidationIssue.invalidScreen(screen.id, "Element IDs must be unique within a screen")
            }
            for role in StudioElementRole.allCases {
                let required = collection?.requiredRoleCount(role, screenID: screen.id)
                    ?? ([StudioElementRole.console, .content, .title, .body,
                         .backButton, .primaryButton, .footer].contains(role) ? 1 : 0)
                let count = screen.elements.filter { $0.role == role }.count
                guard count >= required else {
                    throw TemplateValidationIssue.invalidScreen(
                        screen.id,
                        "Required \(role.title) structure is missing"
                    )
                }
                let maximum: Int?
                if let collection {
                    maximum = collection.maximumRoleCount(role, screenID: screen.id)
                } else {
                    maximum = required > 0 && role != .body ? required : nil
                }
                guard maximum.map({ count <= $0 }) ?? true else {
                    throw TemplateValidationIssue.invalidScreen(
                        screen.id,
                        "Required \(role.title) structure is duplicated"
                    )
                }
            }
            if screen.elements.contains(where: { $0.role == .sceneBackground && $0.kind != .image }) {
                throw TemplateValidationIssue.invalidScreen(screen.id, "Scene background must be an image")
            }
            if collection != .systemSettings && collection != .boot && collection != .login && collection != .desktop {
                guard back.count == 1, primary.count == 1 else {
                    throw TemplateValidationIssue.invalidScreen(screen.id, "Canonical Back and Primary buttons are required")
                }
                guard back[0].kind == .button, primary[0].kind == .button,
                      !back[0].hidden, !primary[0].hidden
                else {
                    throw TemplateValidationIssue.invalidScreen(screen.id, "Visible Back and Primary button actions are required")
                }
            }
            if collection == .systemSettings && ![7, 8].contains(screen.id) {
                let rowCount = screen.elements.filter { $0.role == .metadata }.count
                for role in [StudioElementRole.settingsRowLabel, .settingsRowValue, .settingsDisclosure] {
                    guard screen.elements.filter({ $0.role == role }).count == rowCount else {
                        throw TemplateValidationIssue.invalidScreen(
                            screen.id, "Every Settings row requires one editable \(role.title) layer"
                        )
                    }
                }
            }
            try validateSettingsActions(screen, collection: collection)
            guard collection == .boot || collection == .login || screen.elements.contains(where: { $0.role == .console }) else {
                throw TemplateValidationIssue.invalidScreen(screen.id, "A console frame is required")
            }
            guard liveInputs.count <= 1 else {
                throw TemplateValidationIssue.invalidScreen(screen.id, "Only one live input field is supported per screen")
            }
            for element in screen.elements {
                if (40...48).contains(element.role.rawValue) && (collection != .login || element.kind != .button) {
                    throw TemplateValidationIssue.invalidElement(screen.id, element.id, "Authentication actions require login buttons")
                }
                if element.role.rawValue >= 49 {
                    let allowed = screen.id == 1 ? 49...59 : screen.id == 2 ? 60...64 : 65...71
                    guard collection == .desktop, screen.id <= 3, allowed.contains(Int(element.role.rawValue)) else {
                        throw TemplateValidationIssue.invalidElement(screen.id, element.id, "This binding belongs to a different desktop scene")
                    }
                }
                guard element.kind == .button || element.captionColor == .white else {
                    throw TemplateValidationIssue.invalidElement(screen.id, element.id, "Only buttons may override caption color")
                }
                guard element.hasValidCodeIdentifier else {
                    throw TemplateValidationIssue.invalidElement(
                        screen.id, element.id,
                        "Element ID must use dot-camelcase, for example settingsRow.1"
                    )
                }
                guard element.kind != .progressBar || element.role == .progressBar,
                      element.role != .progressBar || element.kind == .progressBar,
                      element.role != .progressHero || element.kind == .image,
                      element.role != .liveDetails
                        || element.kind == .text
                        || (collection == .systemSettings && element.kind == .panel),
                      ![StudioElementRole.settingsRowLabel, .settingsRowValue, .settingsDisclosure]
                        .contains(element.role) || element.kind == .text
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
                guard (0...46).contains(element.fontFamily), (0...100).contains(element.textSharpness),
                      (0...100).contains(element.textGlow),
                      (0...100).contains(element.opacity), (6...256).contains(element.fontSize),
                      (0...255).contains(element.cornerRadius),
                      (Int(Int16.min)...Int(Int16.max)).contains(element.zIndex) else {
                    throw TemplateValidationIssue.invalidElement(screen.id, element.id, "Element appearance exceeds native limits")
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

    // ------------------------=
    // FUNC: validateSettingsActions
    // DESC: Rejects dangling, duplicate, foreign or unauthorized action parents before any native payload or document mutation is accepted.
    // ------------------=
    static func validateSettingsActions(_ screen: InstallerScreenTemplate, collection: ScreenCollection?) throws {
        let rows = screen.elements.filter { $0.role == .metadata }
        var boundParents: [StudioElementRole: Set<UUID>] = [:]
        for element in screen.elements {
            guard element.role.isSettingsChild else {
                guard element.parentRowID == nil, element.actionCaptionMode == .native else {
                    throw TemplateValidationIssue.invalidElement(screen.id, element.id, "Only Settings children may bind a parent or caption mode")
                }
                continue
            }
            guard collection == .systemSettings, element.kind == SettingsChildContract.kind(element.role), let parent = element.parentRowID,
                  parent != UUID(uuid: (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)),
                  let row = rows.firstIndex(where: { $0.id == parent }),
                  screen.elements.filter({ $0.id == parent }).count == 1,
                  SettingsChildContract.rows(role: element.role, screenID: screen.id).contains(row),
                  boundParents[element.role, default: []].insert(parent).inserted,
                  element.role.hasNativeCaption || element.actionCaptionMode == .native,
                  element.role != .settingsSliderThumb || element.frame.x == 0 else {
                throw TemplateValidationIssue.invalidElement(screen.id, element.id, "Child requires one unique permitted Metadata parent, the correct kind and valid native-owned fields")
            }
        }
    }
}

enum RuntimeTemplateCodec {
    struct Package {
        let document: InstallerStudioDocument
        let assets: [String: Data]
    }
    static let magic = Data([0x49, 0x55, 0x49, 0x54])
    static let version: UInt16 = 12
    static let maximumAssetCount = 128
    static let maximumTemplateBytes = 512 * 1024 * 1024

    // ------------------------=
    // FUNC: encode
    // DESC: Encodes a validated editor document into the deterministic InfinityOS runtime format.
    // ------------------=
    static func encode(
        _ document: InstallerStudioDocument,
        assetRoot: URL? = nil,
        collection: ScreenCollection? = nil,
        theme: UIKitDocument? = nil
    ) throws -> Data {
        try TemplateValidator.validate(document, for: collection)
        let palette: UIKitDocument
        if let theme { palette = theme }
        else if let root = assetRoot,
                FileManager.default.fileExists(atPath: root.appending(path: "assets/ui-design-kit/uikit.infinityui").path) {
            palette = try JSONDecoder().decode(UIKitDocument.self,
                from: Data(contentsOf: root.appending(path: "assets/ui-design-kit/uikit.infinityui")))
        } else { palette = UIKitDocument() }
        try palette.validate()
        var output = Data()
        output.append(magic)
        output.appendLittleEndian(document.infinityMenu == nil ? version : UInt16(13))
        output.appendLittleEndian(UInt16(document.screens.count))
        for screen in document.screens {
            output.append(UInt8(screen.id))
            output.appendLengthPrefixed(screen.title, length: .u8, limit: 63)
            output.appendLittleEndian(UInt16(screen.elements.count))
            for authored in screen.elements {
                let element = authored.resolvingThemeColors(palette.colors)
                output.append(contentsOf: element.id.bytes)
                output.appendLengthPrefixed(element.codeIdentifier, length: .u8, limit: 127)
                output.append(element.kind.rawValue)
                output.append(element.role.rawValue)
                output.append(element.inputVariable.runtimeCode)
                var flags: UInt8 = 0
                if element.locked { flags |= 1 }
                if element.hidden { flags |= 2 }
                flags |= UInt8(element.surfaceMaterial.rawValue) << 2
                flags |= UInt8(element.surfaceCorners.rawValue) << 4
                flags |= UInt8(element.imageFit.rawValue) << 6
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
                output.append(contentsOf: element.parentRowID?.bytes ?? [UInt8](repeating: 0, count: 16))
                output.append(element.actionCaptionMode.rawValue)
                output.append(contentsOf: [element.captionColor.red, element.captionColor.green,
                    element.captionColor.blue, element.captionColor.alpha])
                output.append(contentsOf: [UInt8(element.fontFamily), UInt8(element.textSharpness), UInt8(element.textGlow)])
                guard output.count <= maximumTemplateBytes - 2 else {
                    throw TemplateValidationIssue.invalidDocument("Runtime template exceeds the native byte budget")
                }
            }
        }
        let assetNames = Array(Set(document.screens.flatMap { screen in
            screen.elements.compactMap { element in
                element.kind == .image && !element.imageAsset.isEmpty ? element.imageAsset : nil
            }
        })).sorted()
        guard assetNames.filter({ !isBuiltInImage($0) }).count <= maximumAssetCount else {
            throw TemplateValidationIssue.invalidDocument("Template exceeds the native limit of 128 embedded images")
        }
        var packagedAssets: [(String, Data)] = []
        var packagedSize = output.count + 2
        if let assetRoot {
            for name in assetNames where !isBuiltInImage(name) {
                if let bitmap = try runtimeBitmap(named: name, root: assetRoot) {
                    let recordSize = name.utf8.count + 5 + bitmap.count
                    guard recordSize <= maximumTemplateBytes - packagedSize else {
                        throw TemplateValidationIssue.invalidDocument("Runtime template exceeds the native byte budget")
                    }
                    packagedSize += recordSize
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
        if let menu = document.infinityMenu {
            output.append(contentsOf: [73, 78, 65, 86, 1])
            output.appendLittleEndian(UInt16(menu.rows.count))
            output.append(UInt8(menu.items.count))
            for item in menu.items { encodeMenuItem(item, into: &output) }
        }
        return output
    }

    // ------------------------=
    // FUNC: encodeMenuItem
    // DESC: Encodes the validated navigation tree as bounded typed records, never executable command strings.
    // ------------------=
    private static func encodeMenuItem(_ item: StudioMenuItem, into output: inout Data) {
        output.append(contentsOf: item.id.bytes)
        output.append(contentsOf: [item.action.rawValue, UInt8(item.target), (item.enabled ? 1 : 0) | (item.checked ? 2 : 0), UInt8(item.children.count)])
        output.appendLengthPrefixed(item.label, length: .u8, limit: 63)
        for child in item.children { encodeMenuItem(child, into: &output) }
    }

    // ------------------------=
    // FUNC: decodeMenuItem
    // DESC: Decodes a bounded menu subtree, checking depth and total count before recursion or allocation.
    // ------------------=
    private static func decodeMenuItem(_ reader: inout DataReader, depth: Int, remaining: inout Int) throws -> StudioMenuItem {
        guard remaining > 0, depth < StudioMenu.maximumDepth else { throw TemplateValidationIssue.invalidDocument("Invalid menu hierarchy") }
        remaining -= 1
        let id = try UUID(bytes: reader.readBytes(count: 16))
        guard let action = StudioMenuAction(rawValue: try reader.readUInt8()) else { throw TemplateValidationIssue.invalidDocument("Unknown menu action") }
        let target = Int(try reader.readUInt8()); let flags = try reader.readUInt8(); let count = Int(try reader.readUInt8())
        guard flags & ~3 == 0, count <= remaining, action == .submenu || count == 0 else { throw TemplateValidationIssue.invalidDocument("Invalid menu flags or children") }
        let label = try reader.readString(length: .u8, limit: 63)
        var children: [StudioMenuItem] = []
        for _ in 0..<count { children.append(try decodeMenuItem(&reader, depth: depth + 1, remaining: &remaining)) }
        return StudioMenuItem(id: id, label: label, action: action, target: target, enabled: flags & 1 != 0, checked: flags & 2 != 0, children: children)
    }

    // ------------------------=
    // FUNC: runtimeBitmap
    // DESC: Resolves only repository-owned artwork and shares the preview's exact native bitmap and alpha contract.
    // ------------------=
    private static func runtimeBitmap(named name: String, root: URL) throws -> Data? {
        guard !name.hasPrefix("/"), !name.split(separator: "/").contains("..") else {
            throw TemplateValidationIssue.invalidDocument("Image assets must remain inside the project")
        }
        let paths = name.hasPrefix("assets/") ? [name] : ["assets/boot/\(name)"]
        let candidates = try paths.map { try UIKitAssetCatalog.resolvedURL($0, root: root) }
        guard let source = candidates.first(where: { FileManager.default.fileExists(atPath: $0.path) }) else {
            if isBuiltInImage(name) {
                return nil
            }
            throw TemplateValidationIssue.invalidDocument("Image asset cannot be packaged: \(name)")
        }
        return try NativeBitmapEncoder.read(source)
    }

    // ------------------------=
    // FUNC: isBuiltInImage
    // DESC: Identifies artwork already compiled once into both installer and installed kernels.
    // ------------------=
    private static func isBuiltInImage(_ name: String) -> Bool {
        let boot = ["infinity-installer-background-v2.png", "infinity-installer-masthead-v2.png", "infinity-installer-masthead-v1.png",
                    "infinity-eclipse-header-v1.bmp", "infinity-time-zone-map-v1.png",
                    "infinity-installer-progress-hero-v1.png"]
        return boot.contains(name) || boot.contains(where: { name == "assets/boot/" + $0 })
            || name == "infinity-onboarding-wallpaper-v1.png"
            || name == "assets/desktop/infinity-onboarding-wallpaper-v1.png"
            || name == "eclipse-access-wallpaper-v1.png"
            || name == "assets/ui-design-kit/default/eclipse-access-wallpaper-v1.png"
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Decodes runtime data back into an editable document and rejects malformed records.
    // ------------------=
    static func decode(
        _ data: Data,
        collection: ScreenCollection? = nil
    ) throws -> InstallerStudioDocument {
        // A layout-only round trip deliberately has no assetRoot at encoding.
        // Importing a portable runtime file uses strict decodePackage instead.
        try decodePackage(data, collection: collection, requireEmbeddedAssets: false).document
    }

    // ------------------------=
    // FUNC: decodePackage
    // DESC: Retains validated embedded image payloads so importing an exported runtime template never loses its artwork.
    // ------------------=
    static func decodePackage(_ data: Data, collection: ScreenCollection? = nil,
                              requireEmbeddedAssets: Bool = true) throws -> Package {
        guard data.count <= maximumTemplateBytes else {
            throw TemplateValidationIssue.invalidDocument("Runtime template exceeds the import byte limit")
        }
        var reader = DataReader(data: data)
        guard try reader.readData(count: 4) == magic else {
            throw TemplateValidationIssue.invalidDocument("Invalid runtime-template signature")
        }
        let sourceVersion = try reader.readUInt16()
        guard [6, 7, 8, 9, 10, 11, version, 13].contains(sourceVersion) else {
            throw TemplateValidationIssue.invalidDocument("Unsupported runtime-template version")
        }
        let screenCount = Int(try reader.readUInt16())
        guard (InstallerStudioDocument.minimumScreenCount...InstallerStudioDocument.maximumScreenCount).contains(screenCount) else {
            throw TemplateValidationIssue.invalidDocument("Runtime screen count exceeds native limits")
        }
        var screens: [InstallerScreenTemplate] = []
        for _ in 0..<screenCount {
            let screenID = Int(try reader.readUInt8())
            let screenTitle = try reader.readString(length: .u8, limit: 63)
            let elementCount = Int(try reader.readUInt16())
            var elements: [StudioElement] = []
            for _ in 0..<elementCount {
                let id = try UUID(bytes: reader.readBytes(count: 16))
                let codeIdentifier = try reader.readString(length: .u8, limit: 127)
                guard let kind = StudioElementKind(rawValue: try reader.readUInt8()),
                      let role = StudioElementRole(rawValue: try reader.readUInt8()),
                      let inputVariable = StudioInputVariable(runtimeCode: try reader.readUInt8())
                else {
                    throw TemplateValidationIssue.invalidScreen(screenID, "Unknown element kind or role")
                }
                guard sourceVersion >= 9 || !SettingsChildContract.roles.contains(role) else {
                    throw TemplateValidationIssue.invalidScreen(screenID, "Settings descriptions and slider layers require native template version 9")
                }
                guard role != .sceneBackground || (sourceVersion >= 10 && kind == .image) else {
                    throw TemplateValidationIssue.invalidScreen(screenID, "Scene backgrounds require a v10 image layer")
                }
                let flags = try reader.readUInt8()
                guard (sourceVersion >= 11 || flags & 0xc0 == 0),
                      let imageFit = StudioImageFit(rawValue: Int(flags >> 6)),
                      let surfaceMaterial = UIKitSurfaceMaterial(rawValue: Int((flags >> 2) & 3)),
                      let surfaceCorners = UIKitCornerStyle(rawValue: Int((flags >> 4) & 3)),
                      sourceVersion != 6 || flags & 0xfc == 0 else {
                    throw TemplateValidationIssue.invalidScreen(screenID, "Invalid surface style flags")
                }
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
                var parentRowID: UUID?
                var captionMode = SettingsActionCaptionMode.native
                var captionColor = StudioColor.white
                if sourceVersion >= 8 {
                    let parentBytes = try reader.readBytes(count: 16)
                    if parentBytes.contains(where: { $0 != 0 }) { parentRowID = try UUID(bytes: parentBytes) }
                    guard let mode = SettingsActionCaptionMode(rawValue: try reader.readUInt8()) else {
                        throw TemplateValidationIssue.invalidScreen(screenID, "Unknown action caption mode")
                    }
                    captionMode = mode
                    captionColor = try reader.readColor()
                } else if role == .settingsRowAction {
                    throw TemplateValidationIssue.invalidScreen(screenID, "Action children require native template version 8")
                }
                let fontFamily = sourceVersion >= 12 ? Int(try reader.readUInt8()) : 0
                let textSharpness = sourceVersion >= 12 ? Int(try reader.readUInt8()) : 0
                let textGlow = sourceVersion >= 12 ? Int(try reader.readUInt8()) : 0
                elements.append(StudioElement(
                    id: id,
                    codeIdentifier: codeIdentifier,
                    name: name,
                    kind: kind,
                    role: role,
                    inputVariable: inputVariable,
                    frame: frame,
                    text: text,
                    imageAsset: imageAsset,
                    crop: crop,
                    imageFit: imageFit,
                    fill: fill,
                    border: border,
                    fontSize: fontSize,
                    opacity: opacity,
                    cornerRadius: cornerRadius,
                    zIndex: zIndex,
                    locked: flags & 1 != 0,
                    hidden: flags & 2 != 0,
                    surfaceMaterial: surfaceMaterial,
                    surfaceCorners: surfaceCorners,
                    parentRowID: parentRowID,
                    actionCaptionMode: captionMode,
                    captionColor: captionColor,
                    fontFamily: fontFamily, textSharpness: textSharpness, textGlow: textGlow
                ))
            }
            screens.append(InstallerScreenTemplate(
                id: screenID,
                title: screenTitle,
                elements: elements
            ))
        }
        let assetCount = Int(try reader.readUInt16())
        guard assetCount <= maximumAssetCount else {
            throw TemplateValidationIssue.invalidDocument("Template exceeds the native limit of 128 embedded images")
        }
        var assets: [String: Data] = [:]
        for _ in 0..<assetCount {
            let name = try reader.readString(length: .u8, limit: 127)
            let byteCount = Int(try reader.readUInt32())
            guard !name.isEmpty, !name.hasPrefix("/"), !name.contains("\\"), !name.contains("\0"),
                  !name.split(separator: "/", omittingEmptySubsequences: false).contains(where: { $0.isEmpty || $0 == ".." || $0 == "." }),
                  assets[name] == nil, byteCount <= 128 * 1024 * 1024 else {
                throw TemplateValidationIssue.invalidDocument("Invalid or duplicate runtime image payload")
            }
            let bytes = try reader.readData(count: byteCount)
            guard NativeBitmapEncoder.supportedBitmap(bytes) else {
                throw TemplateValidationIssue.invalidDocument("Runtime image is not a supported native bitmap")
            }
            assets[name] = bytes
        }
        var navigation: StudioMenu?
        if sourceVersion == 13 {
            guard try reader.readData(count: 5) == Data([73, 78, 65, 86, 1]) else { throw TemplateValidationIssue.invalidDocument("Invalid menu signature") }
            var remaining = Int(try reader.readUInt16()); let count = Int(try reader.readUInt8())
            guard remaining <= StudioMenu.maximumItems, count <= remaining else { throw TemplateValidationIssue.invalidDocument("Menu exceeds native limits") }
            var items: [StudioMenuItem] = []
            for _ in 0..<count { items.append(try decodeMenuItem(&reader, depth: 0, remaining: &remaining)) }
            guard remaining == 0 else { throw TemplateValidationIssue.invalidDocument("Menu record count mismatch") }
            navigation = StudioMenu(items: items)
        }
        guard reader.isAtEnd else {
            throw TemplateValidationIssue.invalidDocument("Trailing runtime-template bytes")
        }
        var document = InstallerStudioDocument(
            version: InstallerStudioDocument.currentVersion,
            canvasWidth: 1000,
            canvasHeight: 1000,
            screens: screens
        )
        document.infinityMenu = navigation
        let decodedCollection = collection ?? (screens.contains(where: { $0.elements.contains(where: { $0.role.rawValue >= 49 }) }) ? .desktop : screens.contains(where: { $0.elements.contains(where: { $0.role == .settingsNavigationItem }) })
            ? .systemSettings : screens.contains(where: { $0.elements.contains(where: { $0.role == .input }) })
                ? .configuration : .installation)
        if sourceVersion >= 10 && decodedCollection == .installation {
            document.explicitInstallerBackground = true
        }
        if collection == .login { document.explicitLoginMotion = true }
        if sourceVersion < 9 && collection == .systemSettings {
            document = document.migratedForSettingsActions()
        }
        try TemplateValidator.validate(document, for: collection)
        if requireEmbeddedAssets {
            let required = Set(screens.flatMap(\.elements).compactMap { element -> String? in
                guard element.kind == .image, !element.imageAsset.isEmpty,
                      !isBuiltInImage(element.imageAsset) else { return nil }
                return element.imageAsset
            })
            guard required == Set(assets.keys) else {
                throw TemplateValidationIssue.invalidDocument("Runtime artwork must contain exactly the referenced custom images")
            }
        }
        return Package(document: document, assets: assets)
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
