import Foundation
import SwiftUI

enum StudioElementKind: UInt8, Codable, CaseIterable, Identifiable {
    case panel = 1
    case image = 2
    case text = 3
    case console = 4
    case button = 5

    var id: UInt8 { rawValue }
    var title: String {
        switch self {
        case .panel: "Panel"
        case .image: "Image"
        case .text: "Text"
        case .console: "Console"
        case .button: "Button"
        }
    }
}

enum StudioElementRole: UInt8, Codable, CaseIterable, Identifiable {
    case decoration = 0
    case masthead = 1
    case console = 2
    case content = 3
    case title = 4
    case body = 5
    case image = 6
    case backButton = 7
    case primaryButton = 8
    case footer = 9

    var id: UInt8 { rawValue }
    var title: String {
        switch self {
        case .decoration: "Decoration"
        case .masthead: "Masthead"
        case .console: "Console Frame"
        case .content: "Content"
        case .title: "Title"
        case .body: "Body Copy"
        case .image: "Image"
        case .backButton: "Back Button"
        case .primaryButton: "Primary Button"
        case .footer: "Footer"
        }
    }
}

struct CanvasRect: Codable, Hashable {
    var x: Int
    var y: Int
    var width: Int
    var height: Int

    static let artboard = CanvasRect(x: 0, y: 0, width: 1000, height: 1000)

    // ------------------------=
    // FUNC: clamped
    // DESC: Constrains editable geometry to the normalized installer artboard.
    // ------------------=
    func clamped(minimumWidth: Int = 20, minimumHeight: Int = 20) -> CanvasRect {
        let nextWidth = width.clamped(to: minimumWidth...1000)
        let nextHeight = height.clamped(to: minimumHeight...1000)
        return CanvasRect(
            x: x.clamped(to: 0...(1000 - nextWidth)),
            y: y.clamped(to: 0...(1000 - nextHeight)),
            width: nextWidth,
            height: nextHeight
        )
    }
}

struct StudioColor: Codable, Hashable {
    var red: UInt8
    var green: UInt8
    var blue: UInt8
    var alpha: UInt8

    static let panel = StudioColor(red: 2, green: 10, blue: 18, alpha: 232)
    static let cyan = StudioColor(red: 52, green: 198, blue: 246, alpha: 255)
    static let text = StudioColor(red: 220, green: 230, blue: 241, alpha: 255)
    static let button = StudioColor(red: 14, green: 28, blue: 44, alpha: 255)

    var color: Color {
        Color(
            red: Double(red) / 255,
            green: Double(green) / 255,
            blue: Double(blue) / 255,
            opacity: Double(alpha) / 255
        )
    }

    // ------------------------=
    // FUNC: init_rgba
    // DESC: Creates an editor color from runtime-safe byte components.
    // ------------------=
    init(red: UInt8, green: UInt8, blue: UInt8, alpha: UInt8) {
        self.red = red
        self.green = green
        self.blue = blue
        self.alpha = alpha
    }

    // ------------------------=
    // FUNC: init_color
    // DESC: Converts a SwiftUI color into runtime-safe byte components.
    // ------------------=
    init(_ color: Color) {
        let resolved = NSColor(color).usingColorSpace(.deviceRGB) ?? .white
        red = UInt8((resolved.redComponent * 255).rounded().clamped(to: 0...255))
        green = UInt8((resolved.greenComponent * 255).rounded().clamped(to: 0...255))
        blue = UInt8((resolved.blueComponent * 255).rounded().clamped(to: 0...255))
        alpha = UInt8((resolved.alphaComponent * 255).rounded().clamped(to: 0...255))
    }
}

struct StudioElement: Identifiable, Codable, Hashable {
    var id: UUID
    var name: String
    var kind: StudioElementKind
    var role: StudioElementRole
    var frame: CanvasRect
    var text: String
    var imageAsset: String
    var crop: ImageCrop = .none
    var fill: StudioColor
    var border: StudioColor
    var fontSize: Int
    var opacity: Int
    var cornerRadius: Int
    var zIndex: Int
    var locked: Bool
    var hidden: Bool

    // ------------------------=
    // FUNC: make
    // DESC: Creates a fully styled editable installer element.
    // ------------------=
    static func make(
        name: String,
        kind: StudioElementKind,
        role: StudioElementRole = .decoration,
        frame: CanvasRect,
        text: String = "",
        imageAsset: String = "",
        locked: Bool = false,
        zIndex: Int = 0
    ) -> StudioElement {
        StudioElement(
            id: UUID(),
            name: name,
            kind: kind,
            role: role,
            frame: frame,
            text: text,
            imageAsset: imageAsset,
            fill: kind == .text ? .text : .panel,
            border: .cyan,
            fontSize: kind == .text ? 24 : 19,
            opacity: 100,
            cornerRadius: kind == .console ? 6 : 12,
            zIndex: zIndex,
            locked: locked,
            hidden: false
        )
    }
}

struct ImageCrop: Codable, Hashable {
    var left: Int
    var top: Int
    var right: Int
    var bottom: Int

    static let none = ImageCrop(left: 0, top: 0, right: 0, bottom: 0)

    // ------------------------=
    // FUNC: clamped
    // DESC: Constrains normalized crop edges while retaining a visible image area.
    // ------------------=
    func clamped() -> ImageCrop {
        var result = ImageCrop(
            left: left.clamped(to: 0...90),
            top: top.clamped(to: 0...90),
            right: right.clamped(to: 0...90),
            bottom: bottom.clamped(to: 0...90)
        )
        if result.left + result.right > 95 { result.right = 95 - result.left }
        if result.top + result.bottom > 95 { result.bottom = 95 - result.top }
        return result
    }
}

extension StudioElement {
    private enum CodingKeys: String, CodingKey {
        case id, name, kind, role, frame, text, imageAsset, crop, fill, border
        case fontSize, opacity, cornerRadius, zIndex, locked, hidden
    }

    // ------------------------=
    // FUNC: init_decoder
    // DESC: Decodes current elements while migrating pre-crop saved projects safely.
    // ------------------=
    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        id = try values.decode(UUID.self, forKey: .id)
        name = try values.decode(String.self, forKey: .name)
        kind = try values.decode(StudioElementKind.self, forKey: .kind)
        role = try values.decode(StudioElementRole.self, forKey: .role)
        frame = try values.decode(CanvasRect.self, forKey: .frame)
        text = try values.decode(String.self, forKey: .text)
        imageAsset = try values.decode(String.self, forKey: .imageAsset)
        crop = try values.decodeIfPresent(ImageCrop.self, forKey: .crop) ?? .none
        fill = try values.decode(StudioColor.self, forKey: .fill)
        border = try values.decode(StudioColor.self, forKey: .border)
        fontSize = try values.decode(Int.self, forKey: .fontSize)
        opacity = try values.decode(Int.self, forKey: .opacity)
        cornerRadius = try values.decode(Int.self, forKey: .cornerRadius)
        zIndex = try values.decode(Int.self, forKey: .zIndex)
        locked = try values.decode(Bool.self, forKey: .locked)
        hidden = try values.decode(Bool.self, forKey: .hidden)
    }
}

struct InstallerScreenTemplate: Identifiable, Codable, Hashable {
    var id: Int
    var title: String
    var elements: [StudioElement]
}

struct InstallerStudioDocument: Codable, Hashable {
    static let currentVersion = 1
    static let minimumScreenCount = 1
    static let maximumScreenCount = 32
    var version: Int
    var canvasWidth: Int
    var canvasHeight: Int
    var screens: [InstallerScreenTemplate]

    // ------------------------=
    // FUNC: factoryDefault
    // DESC: Builds the complete eleven-screen first-run design document.
    // ------------------=
    static func factoryDefault() -> InstallerStudioDocument {
        let names = [
            "Welcome", "Infinity Pool", "Disk Discovery", "Disk Review", "Date & Time",
            "Plan Review", "Confirmation", "Installing", "Complete", "Attention", "Help",
        ]
        let headings = [
            "WELCOME TO INFINITYOS", "HOW INFINITY POOL WORKS", "DISKS DISCOVERED",
            "REVIEW SELECTED DISK", "DATE & TIME", "REVIEW INSTALLATION",
            "REVIEW INSTALLATION", "INSTALLING INFINITYOS", "INSTALLATION COMPLETE",
            "INSTALLATION NEEDS ATTENTION", "INFINITYOS SETUP HELP",
        ]
        let body = [
            "InfinityOS is a distributed operating system. One simple home for your system, your apps, and everything you create. We'll guide you through every choice. Nothing changes until you approve it.",
            "The Infinity Pool organizes your disk into four protected areas that keep your data safe, isolated, and easy to recover.",
            "Select a disk to add to the Infinity Pool.",
            "Confirm the selected device, connection, capacity, and current contents.",
            "Set the local date, time, and time zone saved into the installed system.",
            "Review the immutable installation plan before final approval.",
            "The selected disk will be erased only after explicit confirmation.",
            "InfinityOS is creating and verifying the new System Generation.",
            "Installation is verified and ready to boot.",
            "Installation stopped safely before activation.",
            "Use Tab or arrows to move, Enter to select, and Escape to return.",
        ]
        let screens = names.indices.map { index in
            var elements = defaultElements(title: headings[index], body: body[index])
            for elementIndex in elements.indices {
                elements[elementIndex].id = factoryElementID(screen: index + 1, element: elementIndex + 1)
            }
            return InstallerScreenTemplate(
                id: index + 1,
                title: names[index],
                elements: elements
            )
        }
        return InstallerStudioDocument(
            version: currentVersion,
            canvasWidth: 1000,
            canvasHeight: 1000,
            screens: screens
        )
    }

    // ------------------------=
    // FUNC: factoryElementID
    // DESC: Creates a stable identifier so factory template exports are reproducible.
    // ------------------=
    private static func factoryElementID(screen: Int, element: Int) -> UUID {
        UUID(uuidString: String(format: "49554954-%04X-%04X-8000-000000000001", screen, element))!
    }

    // ------------------------=
    // FUNC: defaultElements
    // DESC: Creates the shared gold-standard composition for one installer screen.
    // ------------------=
    private static func defaultElements(title: String, body: String) -> [StudioElement] {
        [
            .make(
                name: "InfinityOS Masthead",
                kind: .image,
                role: .masthead,
                frame: CanvasRect(x: 230, y: 0, width: 540, height: 300),
                imageAsset: "infinity-installer-masthead-v2.png",
                locked: true,
                zIndex: 0
            ),
            .make(
                name: "Installer Console",
                kind: .console,
                role: .console,
                frame: CanvasRect(x: 20, y: 320, width: 960, height: 660),
                locked: false,
                zIndex: 1
            ),
            .make(
                name: "Screen Title",
                kind: .text,
                role: .title,
                frame: CanvasRect(x: 40, y: 342, width: 560, height: 42),
                text: title,
                zIndex: 3
            ),
            .make(
                name: "Content Panel",
                kind: .panel,
                role: .content,
                frame: CanvasRect(x: 38, y: 396, width: 924, height: 404),
                zIndex: 2
            ),
            .make(
                name: "Body Copy",
                kind: .text,
                role: .body,
                frame: CanvasRect(x: 70, y: 430, width: 540, height: 120),
                text: body,
                zIndex: 4
            ),
            lockedButton(
                name: "Back",
                role: .backButton,
                frame: CanvasRect(x: 135, y: 820, width: 350, height: 55),
                text: "BACK",
                zIndex: 8
            ),
            lockedButton(
                name: "Primary",
                role: .primaryButton,
                frame: CanvasRect(x: 510, y: 820, width: 350, height: 55),
                text: "CONTINUE",
                zIndex: 8
            ),
            .make(
                name: "Keyboard Footer",
                kind: .text,
                role: .footer,
                frame: CanvasRect(x: 100, y: 920, width: 800, height: 40),
                text: "TAB / ARROWS: MOVE FOCUS    ENTER: SELECT    ESC: CANCEL    F1: HELP",
                locked: true,
                zIndex: 9
            ),
        ]
    }

    // ------------------------=
    // FUNC: lockedButton
    // DESC: Creates a canonical immutable installer navigation button.
    // ------------------=
    private static func lockedButton(
        name: String,
        role: StudioElementRole,
        frame: CanvasRect,
        text: String,
        zIndex: Int
    ) -> StudioElement {
        var element = StudioElement.make(
            name: name,
            kind: .button,
            role: role,
            frame: frame,
            text: text,
            locked: true,
            zIndex: zIndex
        )
        element.fill = .button
        element.fontSize = 20
        element.cornerRadius = 9
        return element
    }
}

extension Comparable {
    // ------------------------=
    // FUNC: clamped
    // DESC: Constrains a comparable value to a closed range.
    // ------------------=
    func clamped(to limits: ClosedRange<Self>) -> Self {
        min(max(self, limits.lowerBound), limits.upperBound)
    }
}
