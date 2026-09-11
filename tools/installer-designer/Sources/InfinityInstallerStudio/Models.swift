import Foundation
import SwiftUI

enum ConfigurationNetworkLayout {
    // ------------------------=
    // FUNC: row
    // DESC: Matches the runtime network row geometry relative to the authored content panel.
    // ------------------=
    static func row(in content: CanvasRect, index: Int) -> CanvasRect {
        let gutter = content.width * 4 / 100
        return CanvasRect(
            x: content.x + gutter,
            y: content.y + content.height * 42 / 100 + index * (content.height * 12 / 100),
            width: content.width - 2 * gutter,
            height: max(1, content.height * 10 / 100)
        )
    }
}

enum StudioElementKind: UInt8, Codable, CaseIterable, Identifiable {
    case panel = 1
    case image = 2
    case text = 3
    case console = 4
    case button = 5
    case progressBar = 6

    var id: UInt8 { rawValue }
    var title: String {
        switch self {
        case .panel: "Panel"
        case .image: "Image"
        case .text: "Text"
        case .console: "Console"
        case .button: "Button"
        case .progressBar: "Progress Bar"
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
    case input = 10
    case sectionLabel = 11
    case dateField = 12
    case timeField = 13
    case timeZoneSelector = 14
    case offsetBadge = 15
    case timeZoneMap = 16
    case metadata = 17
    case progressSegment = 18
    case progressBar = 19
    case progressHero = 20
    case liveDetails = 21

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
        case .input: "Input Field"
        case .sectionLabel: "Section Label"
        case .dateField: "Date Field"
        case .timeField: "Time Field"
        case .timeZoneSelector: "Time Zone Selector"
        case .offsetBadge: "UTC Offset Badge"
        case .timeZoneMap: "Time Zone Map"
        case .metadata: "Metadata"
        case .progressSegment: "Progress Segment"
        case .progressBar: "Installation Progress"
        case .progressHero: "Progress Hero"
        case .liveDetails: "Live Installer Details"
        }
    }

    // ------------------------=
    // FUNC: imageUsesAspectFill
    // DESC: Matches runtime image fitting so wallpaper and maps fill while ordinary artwork stays wholly visible.
    // ------------------=
    var imageUsesAspectFill: Bool {
        self == .masthead || self == .timeZoneMap
    }
}

enum StudioInputVariable: String, Codable, CaseIterable, Identifiable {
    case none = ""
    case machineNodeName = "machine.node_name"
    case profileName = "user.profile_name"
    case displayName = "user.display_name"
    case password = "credential.password"

    var id: String { rawValue }
    var title: String {
        switch self {
        case .none: "Unbound"
        case .machineNodeName: "Machine Node Name"
        case .profileName: "Profile Name"
        case .displayName: "Display Name"
        case .password: "Password"
        }
    }
    var isSecure: Bool { self == .password }
    var runtimeCode: UInt8 {
        switch self {
        case .none: 0
        case .machineNodeName: 1
        case .profileName: 2
        case .displayName: 3
        case .password: 4
        }
    }

    // ------------------------=
    // FUNC: init_runtimeCode
    // DESC: Restores a typed editor variable from its stable runtime artifact identifier.
    // ------------------=
    init?(runtimeCode: UInt8) {
        switch runtimeCode {
        case 0: self = .none
        case 1: self = .machineNodeName
        case 2: self = .profileName
        case 3: self = .displayName
        case 4: self = .password
        default: return nil
        }
    }
}

enum ScreenCollection: String, CaseIterable, Identifiable {
    case installation
    case configuration
    case systemSettings

    var id: String { rawValue }
    var title: String {
        switch self {
        case .installation: "Installation Screens"
        case .configuration: "OS Configuration Screens"
        case .systemSettings: "System Settings"
        }
    }

    // ------------------------=
    // FUNC: requiredScreenCount
    // DESC: Returns the canonical screen count used to identify a built-in runtime workflow.
    // ------------------=
    var requiredScreenCount: Int {
        switch self {
        case .installation: 11
        case .configuration: 8
        case .systemSettings: 11
        }
    }

    // ------------------------=
    // FUNC: runtimeCollection
    // DESC: Resolves the fixed InfinityOS runtime workflow represented by one template document.
    // ------------------=
    static func runtimeCollection(screenCount: Int) -> ScreenCollection? {
        if screenCount == ScreenCollection.configuration.requiredScreenCount { return .configuration }
        if screenCount == ScreenCollection.installation.requiredScreenCount { return .installation }
        return nil
    }

    // ------------------------=
    // FUNC: requiredRoleCount
    // DESC: Returns the minimum structural role count required by this runtime workflow and screen.
    // ------------------=
    func requiredRoleCount(_ role: StudioElementRole, screenID: Int) -> Int {
        if self == .systemSettings {
            return [.masthead, .console, .content, .title, .body, .sectionLabel, .metadata]
                .contains(role) ? 1 : 0
        }
        if [.masthead, .console, .content, .title, .body, .backButton, .primaryButton, .footer]
            .contains(role)
        {
            return 1
        }
        switch self {
        case .installation where [3, 4, 6, 10].contains(screenID):
            return role == .liveDetails ? 1 : 0
        case .installation where screenID == 5:
            return [.dateField, .timeField, .timeZoneSelector, .offsetBadge, .timeZoneMap]
                .contains(role) ? 1 : 0
        case .installation where screenID == 8:
            return [.progressBar, .progressHero].contains(role) ? 1 : 0
        case .configuration:
            if role == .input { return 1 }
            if role == .progressSegment { return 8 }
            return 0
        case .systemSettings: return 0
        default:
            return 0
        }
    }

    // ------------------------=
    // FUNC: maximumRoleCount
    // DESC: Caps singular runtime roles while allowing additional authored body-copy layers.
    // ------------------=
    func maximumRoleCount(_ role: StudioElementRole, screenID: Int) -> Int? {
        if role == .liveDetails {
            if self == .systemSettings { return 1 }
            return self == .installation && [3, 4, 6, 10].contains(screenID) ? 1 : 0
        }
        let required = requiredRoleCount(role, screenID: screenID)
        if self == .systemSettings && role == .metadata { return nil }
        guard required > 0, role != .body else { return nil }
        return required
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

    static let panel = StudioColor(red: 10, green: 18, blue: 29, alpha: 230)
    static let cyan = StudioColor(red: 32, green: 191, blue: 255, alpha: 255)
    static let text = StudioColor(red: 241, green: 245, blue: 250, alpha: 255)
    static let button = StudioColor(red: 5, green: 15, blue: 27, alpha: 255)

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
    var inputVariable: StudioInputVariable = .none
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
            inputVariable: .none,
            frame: frame,
            text: text,
            imageAsset: imageAsset,
            fill: kind == .text ? .text : .panel,
            border: .cyan,
            fontSize: kind == .text ? 24 : 19,
            opacity: 100,
            cornerRadius: kind == .console ? 16 : 10,
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
        case id, name, kind, role, inputVariable, frame, text, imageAsset, crop, fill, border
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
        inputVariable = try values.decodeIfPresent(StudioInputVariable.self, forKey: .inputVariable) ?? .none
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
            var elements = if index == 4 {
                dateTimeElements(title: headings[index], body: body[index])
            } else if index == 7 {
                installingElements(title: headings[index], body: body[index])
            } else {
                defaultElements(title: headings[index], body: body[index])
            }
            if let details = liveDetailsElement(screenID: index + 1) {
                elements.append(details)
            }
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
    // FUNC: factoryConfiguration
    // DESC: Builds the eight-screen post-install configuration document used by the real first-boot flow.
    // ------------------=
    static func factoryConfiguration() -> InstallerStudioDocument {
        let names = [
            "Welcome", "Node Name", "Profile Name", "Display Name", "Password",
            "Privacy & Appearance", "Network", "Ready",
        ]
        let headings = [
            "Welcome to InfinityOS", "Name your Infinity Node", "Choose your profile name",
            "How should we address you?", "Secure your account", "Private by default",
            "Connect this Infinity Node", "Your Infinity begins here",
        ]
        let body = [
            "A private system shaped around you.",
            "Choose a friendly name for this device. You can change it later.",
            "Your profile name identifies your Personal Space without exposing your full name.",
            "Use the display name you want InfinityOS to show across your local experience.",
            "Use at least eight characters. Your password remains local to this system.",
            "Local AI is ready. Remote processing and microphone access begin disabled.",
            "Choose wired, Wi-Fi, or continue offline. You can change this later.",
            "Your identity, Personal Space, privacy policy, and appearance are ready.",
        ]
        let placeholders = [
            "", "InfinityNode", "your-profile", "Display name", "Create a password", "", "", "",
        ]
        let screens = names.indices.map { index in
            var elements = configurationElements(
                step: index,
                title: headings[index],
                body: body[index],
                placeholder: placeholders[index],
                primary: index == names.count - 1 ? "ENTER INFINITYOS" : "CONTINUE"
            )
            for elementIndex in elements.indices {
                elements[elementIndex].id = configurationElementID(
                    screen: index + 1,
                    element: elementIndex + 1
                )
            }
            return InstallerScreenTemplate(id: index + 1, title: names[index], elements: elements)
        }
        return InstallerStudioDocument(
            version: currentVersion,
            canvasWidth: 1000,
            canvasHeight: 1000,
            screens: screens
        )
    }

    // ------------------------=
    // FUNC: factorySystemSettings
    // DESC: Builds the eleven editable section canvases consumed by the logged-in System Settings app.
    // ------------------=
    static func factorySystemSettings() -> InstallerStudioDocument {
        let sections = [
            "General", "Themes & Skins", "Users & Accounts", "AI & Voice",
            "Privacy & Security", "Devices", "Network", "Nodes & Mesh",
            "Storage", "About", "Input",
        ]
        let descriptions = [
            "Manage this machine, language, region, generation, and updates.",
            "Choose the visual system used across InfinityOS.",
            "Manage identities, credentials, sessions, and Personal Space.",
            "Configure local intelligence, chat, models, and voice access.",
            "Review authority, permissions, session security, and trusted UI.",
            "Inspect displays, keyboards, pointers, and audio devices.",
            "Configure connectivity, interfaces, DNS, routes, and policy.",
            "Discover, pair, and govern trusted Infinity nodes.",
            "Inspect Infinity Pool capacity, placement, and protection.",
            "Review the active InfinityOS System Generation.",
            "Tune pointer, keyboard, scrolling, and interaction behavior.",
        ]
        let rowLabels = [
            ["Machine Name", "Language", "Region", "System Generation", "Updates"],
            ["Skin", "Icon Set", "Primary", "Secondary", "Opacity", "Blur", "UI Scale", "Wallpaper"],
            ["Current User", "Credential", "Session", "Personal Space", "Profile"],
            ["AI Provider", "Desktop AI Chat", "Chat Model", "Remote Processing", "Voice", "Activation", "Model Access"],
            ["Ambient Authority", "Microphone", "Remote AI", "No Activity Timeout", "Trusted UI"],
            ["Display", "Keyboard", "Pointer", "Audio Input", "Audio Output"],
            ["Connectivity", "Profiles", "Interfaces & Topology", "Application & Service Access", "DNS / Resolution", "Routes", "Connections", "Diagnostics"],
            ["Trusted Nodes", "Pair Node", "Mesh Health", "Access Policy", "Security Audit"],
            ["Infinity Pool", "Capacity", "Nodes", "Selected Object", "Replica Location", "Temporary", "Protected", "Critical"],
            ["InfinityOS", "Architecture", "Boot", "Identity Format", "Icon Families"],
            ["Pointer Speed", "Scroll Speed", "Double Click", "Drag Threshold", "Natural Scroll", "Primary Button", "Keyboard Repeat", "Cursor Size"],
        ]
        let screens = sections.indices.map { index in
            var elements = systemSettingsElements(
                section: sections[index], description: descriptions[index], rows: rowLabels[index]
            )
            for elementIndex in elements.indices {
                elements[elementIndex].id = settingsElementID(
                    screen: index + 1, element: elementIndex + 1
                )
            }
            return InstallerScreenTemplate(id: index + 1, title: sections[index], elements: elements)
        }
        return InstallerStudioDocument(
            version: currentVersion, canvasWidth: 1000, canvasHeight: 1000, screens: screens
        )
    }

    // ------------------------=
    // FUNC: settingsElementID
    // DESC: Creates stable identifiers for System Settings WYSIWYG layers.
    // ------------------=
    private static func settingsElementID(screen: Int, element: Int) -> UUID {
        UUID(uuidString: String(format: "53595354-%04X-%04X-8000-000000000001", screen, element))!
    }

    // ------------------------=
    // FUNC: systemSettingsElements
    // DESC: Creates one complete Settings section with editable chrome, navigation, viewport, rows, and detail well.
    // ------------------=
    private static func systemSettingsElements(
        section: String, description: String, rows: [String]
    ) -> [StudioElement] {
        var elements: [StudioElement] = [
            .make(name: "Settings Window", kind: .console, role: .console,
                  frame: CanvasRect(x: 55, y: 80, width: 890, height: 820), zIndex: 0),
            .make(name: "Title Bar", kind: .panel, role: .masthead,
                  frame: CanvasRect(x: 55, y: 80, width: 890, height: 76), zIndex: 1),
            .make(name: "Section Navigation", kind: .panel, role: .sectionLabel,
                  frame: CanvasRect(x: 55, y: 156, width: 250, height: 744), zIndex: 1),
            .make(name: "Content Viewport", kind: .panel, role: .content,
                  frame: CanvasRect(x: 335, y: 176, width: 580, height: 690), zIndex: 1),
            .make(name: "Section Title", kind: .text, role: .title,
                  frame: CanvasRect(x: 370, y: 205, width: 500, height: 54), text: section, zIndex: 3),
            .make(name: "Section Description", kind: .text, role: .body,
                  frame: CanvasRect(x: 370, y: 262, width: 500, height: 48), text: description, zIndex: 3),
        ]
        elements[0].fill = StudioColor(red: 5, green: 18, blue: 31, alpha: 230)
        elements[0].border = StudioColor(red: 118, green: 210, blue: 255, alpha: 210)
        elements[0].cornerRadius = 18
        elements[1].fill = StudioColor(red: 5, green: 38, blue: 61, alpha: 224)
        elements[2].fill = StudioColor(red: 3, green: 16, blue: 28, alpha: 220)
        elements[3].fill = StudioColor(red: 4, green: 21, blue: 36, alpha: 168)
        elements[3].border = StudioColor(red: 65, green: 132, blue: 170, alpha: 80)
        elements[4].fontSize = 32
        elements[5].fontSize = 17
        for (row, label) in rows.enumerated() {
            var element = StudioElement.make(
                name: "Setting Row \(row + 1)", kind: .panel, role: .metadata,
                frame: CanvasRect(x: 370, y: 330 + row * 64, width: 500, height: 50),
                text: label, zIndex: 4
            )
            element.fill = StudioColor(red: 5, green: 20, blue: 34, alpha: 222)
            element.border = StudioColor(red: 76, green: 151, blue: 190, alpha: 170)
            element.fontSize = 17
            elements.append(element)
        }
        var detail = StudioElement.make(
            name: "Expanded Detail Content", kind: .text, role: .liveDetails,
            frame: CanvasRect(x: 370, y: 390, width: 500, height: 150),
            text: "Live controls and details appear here.", zIndex: 5
        )
        detail.fill = StudioColor(red: 5, green: 24, blue: 40, alpha: 236)
        detail.border = StudioColor(red: 109, green: 220, blue: 255, alpha: 180)
        detail.hidden = true
        elements.append(detail)
        return elements
    }

    // ------------------------=
    // FUNC: factoryElementID
    // DESC: Creates a stable identifier so factory template exports are reproducible.
    // ------------------=
    private static func factoryElementID(screen: Int, element: Int) -> UUID {
        UUID(uuidString: String(format: "49554954-%04X-%04X-8000-000000000001", screen, element))!
    }

    // ------------------------=
    // FUNC: configurationElementID
    // DESC: Creates stable non-colliding identifiers for factory OS configuration layers.
    // ------------------=
    private static func configurationElementID(screen: Int, element: Int) -> UUID {
        UUID(uuidString: String(format: "4F534346-%04X-%04X-8000-000000000001", screen, element))!
    }

    // ------------------------=
    // FUNC: configurationElements
    // DESC: Creates the editable first-boot card, semantic copy, input, and protected actions.
    // ------------------=
    private static func configurationElements(
        step: Int,
        title: String,
        body: String,
        placeholder: String,
        primary: String
    ) -> [StudioElement] {
        let eyebrow = [
            "WELCOME", "MACHINE", "PROFILE", "PROFILE", "SECURITY",
            "AI, VOICE & APPEARANCE", "NETWORK", "READY",
        ][step]
        var elements: [StudioElement] = [
            .make(
                name: "First-Boot Background", kind: .image, role: .masthead,
                frame: CanvasRect(x: 0, y: 0, width: 1000, height: 1000),
                imageAsset: "assets/desktop/infinity-onboarding-wallpaper-v1.png", zIndex: 0
            ),
            .make(
                name: "Configuration Card", kind: .console, role: .console,
                frame: InfinityUIKit.Metrics.configurationCard, zIndex: 1
            ),
            .make(
                name: "Configuration Content", kind: .panel, role: .content,
                frame: InfinityUIKit.Metrics.configurationContent, zIndex: 2
            ),
            .make(
                name: "Screen Title", kind: .text, role: .title,
                frame: InfinityUIKit.Metrics.configurationTitle,
                text: title, zIndex: 4
            ),
            .make(
                name: "Body Copy", kind: .text, role: .body,
                frame: InfinityUIKit.Metrics.configurationBody,
                text: body, zIndex: 4
            ),
            .make(
                name: "Configuration Input", kind: .panel, role: .input,
                frame: InfinityUIKit.Metrics.configurationInput,
                text: placeholder, zIndex: 5
            ),
            lockedButton(
                name: "Back", role: .backButton,
                frame: InfinityUIKit.Metrics.configurationBack,
                text: "BACK", zIndex: 8
            ),
            lockedButton(
                name: "Primary", role: .primaryButton,
                frame: InfinityUIKit.Metrics.configurationPrimary,
                text: primary, zIndex: 8
            ),
            .make(
                name: "Privacy Mark", kind: .text, role: .footer,
                frame: CanvasRect(x: 242, y: 151, width: 101, height: 24),
                text: "LOCAL | PRIVATE", zIndex: 6
            ),
            .make(
                name: "InfinityOS Wordmark", kind: .text,
                frame: CanvasRect(x: 72, y: 151, width: 125, height: 24),
                text: "INFINITYOS", zIndex: 6
            ),
            .make(
                name: "Section Eyebrow", kind: .text,
                frame: CanvasRect(x: 72, y: 230, width: 276, height: 24),
                text: eyebrow, zIndex: 6
            ),
        ]
        elements[1].fill = InfinityUIKit.Palette.panel
        elements[1].border = InfinityUIKit.Palette.border
        elements[1].cornerRadius = 16
        elements[2].fill = InfinityUIKit.Palette.panelRaised
        elements[2].border = InfinityUIKit.Palette.border
        elements[2].cornerRadius = 16
        elements[3].fill = InfinityUIKit.Palette.textPrimary
        elements[3].fontSize = 30
        elements[4].fill = InfinityUIKit.Palette.textSecondary
        elements[4].fontSize = 17
        elements[5].fill = InfinityUIKit.Palette.field
        elements[5].border = InfinityUIKit.Palette.fieldBorder
        elements[5].cornerRadius = 10
        elements[5].inputVariable = switch step {
        case 1: .machineNodeName
        case 2: .profileName
        case 3: .displayName
        case 4: .password
        default: .none
        }
        elements[8].fill = InfinityUIKit.Palette.textSecondary
        elements[8].fontSize = 13
        elements[9].fill = InfinityUIKit.Palette.textPrimary
        elements[9].fontSize = 17
        elements[10].fill = InfinityUIKit.Palette.accent
        elements[10].fontSize = 14

        for progressIndex in 0..<8 {
            var segment = StudioElement.make(
                name: "Progress Segment \(progressIndex + 1)", kind: .panel,
                role: .progressSegment,
                frame: CanvasRect(x: 72 + progressIndex * 35, y: 197, width: 31, height: 20),
                text: "", zIndex: 6
            )
            segment.fill = progressIndex <= step
                ? InfinityUIKit.Palette.accent
                : InfinityUIKit.Palette.border
            segment.border = segment.fill
            segment.cornerRadius = 2
            elements.append(segment)
        }
        if placeholder.isEmpty {
            elements[5].hidden = true
        }
        if step == 0 {
            elements[6].frame = CanvasRect(x: 61, y: 789, width: 298, height: 47)
            elements[6].opacity = 0
            elements[7].frame = CanvasRect(x: 61, y: 789, width: 298, height: 47)
            let benefits = [
                ("Yours from the start", "Identity and Personal Space are built in."),
                ("Private by design", "Explicit capability controls stay local."),
                ("Ready to grow", "Objects, apps, and AI share one system."),
            ]
            for (index, benefit) in benefits.enumerated() {
                appendConfigurationRow(
                    to: &elements,
                    index: index,
                    top: 434 + index * 66,
                    heading: benefit.0,
                    detail: benefit.1,
                    prefix: "Welcome Benefit"
                )
            }
        } else if step == 5 {
            for (index, setting) in [
                ("Local AI", "ON"),
                ("Remote processing", "OFF"),
                ("Voice and microphone", "OFF"),
                ("Appearance", "DEFAULT DARK"),
            ].enumerated() {
                appendConfigurationSetting(
                    to: &elements,
                    index: index,
                    top: 434 + index * 56,
                    label: setting.0,
                    value: setting.1
                )
            }
        } else if step == 7 {
            let ready = ["IDENTITY", "PERSONAL SPACE", "PRIVACY DEFAULTS", "APPEARANCE"]
            for (index, label) in ready.enumerated() {
                appendConfigurationSetting(
                    to: &elements,
                    index: index,
                    top: 434 + index * 50,
                    label: label,
                    value: "READY"
                )
            }
        }
        return elements
    }

    // ------------------------=
    // FUNC: appendConfigurationRow
    // DESC: Adds a consistently guttered raised card with primary and secondary copy.
    // ------------------=
    private static func appendConfigurationRow(
        to elements: inout [StudioElement],
        index: Int,
        top: Int,
        heading: String,
        detail: String,
        prefix: String
    ) {
        var panel = StudioElement.make(
            name: "\(prefix) Card \(index + 1)", kind: .panel,
            frame: CanvasRect(x: 72, y: top, width: 276, height: 54), zIndex: 6
        )
        panel.fill = InfinityUIKit.Palette.panelRaised
        panel.border = InfinityUIKit.Palette.border
        panel.cornerRadius = 10
        var headingElement = StudioElement.make(
            name: "\(prefix) Heading \(index + 1)", kind: .text,
            frame: CanvasRect(x: 82, y: top + 8, width: 256, height: 20),
            text: heading, zIndex: 7
        )
        headingElement.fill = InfinityUIKit.Palette.textPrimary
        headingElement.fontSize = 16
        var detailElement = StudioElement.make(
            name: "\(prefix) Detail \(index + 1)", kind: .text,
            frame: CanvasRect(x: 82, y: top + 29, width: 256, height: 20),
            text: detail, zIndex: 7
        )
        detailElement.fill = InfinityUIKit.Palette.textSecondary
        detailElement.fontSize = 13
        elements.append(contentsOf: [panel, headingElement, detailElement])
    }

    // ------------------------=
    // FUNC: appendConfigurationSetting
    // DESC: Adds one standardized configuration value row with aligned label and state.
    // ------------------=
    private static func appendConfigurationSetting(
        to elements: inout [StudioElement],
        index: Int,
        top: Int,
        label: String,
        value: String
    ) {
        var panel = StudioElement.make(
            name: "Setting Card \(index + 1)", kind: .panel,
            frame: CanvasRect(x: 72, y: top, width: 276, height: 44), zIndex: 6
        )
        panel.fill = InfinityUIKit.Palette.panelRaised
        panel.border = InfinityUIKit.Palette.border
        panel.cornerRadius = 10
        var labelElement = StudioElement.make(
            name: "Setting Label \(index + 1)", kind: .text,
            frame: CanvasRect(x: 82, y: top + 12, width: 153, height: 20),
            text: label, zIndex: 7
        )
        labelElement.fill = InfinityUIKit.Palette.textPrimary
        labelElement.fontSize = 15
        var valueElement = StudioElement.make(
            name: "Setting Value \(index + 1)", kind: .text,
            frame: CanvasRect(x: 235, y: top + 12, width: 103, height: 20),
            text: value, zIndex: 7
        )
        valueElement.fill = InfinityUIKit.Palette.accentBright
        valueElement.fontSize = 13
        elements.append(contentsOf: [panel, labelElement, valueElement])
    }

    // ------------------------=
    // FUNC: defaultElements
    // DESC: Creates the shared gold-standard composition for one installer screen.
    // ------------------=
    private static func defaultElements(title: String, body: String) -> [StudioElement] {
        var elements: [StudioElement] = [
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
        elements[1].fill = InfinityUIKit.Palette.panel
        elements[1].border = InfinityUIKit.Palette.border
        elements[1].cornerRadius = 16
        elements[2].fill = InfinityUIKit.Palette.textPrimary
        elements[2].fontSize = 24
        elements[3].fill = InfinityUIKit.Palette.panelRaised
        elements[3].border = InfinityUIKit.Palette.border
        elements[3].cornerRadius = 16
        elements[4].fill = InfinityUIKit.Palette.textSecondary
        elements[4].fontSize = 18
        elements[7].fill = InfinityUIKit.Palette.textSecondary
        elements[7].fontSize = 16
        return elements
    }

    // ------------------------=
    // FUNC: installingElements
    // DESC: Builds the runtime-faithful installation hero, live progress control, and non-interactive footer scene.
    // ------------------=
    private static func installingElements(title: String, body: String) -> [StudioElement] {
        var elements = defaultElements(title: title, body: body)
        elements[3].hidden = true
        elements[4].hidden = true
        elements[5].opacity = 0
        elements[6].opacity = 0
        elements[7].text = "INSTALLATION IN PROGRESS    PLEASE KEEP THIS DEVICE POWERED"

        var hero = StudioElement.make(
            name: "Installation Progress Hero",
            kind: .image,
            role: .progressHero,
            frame: CanvasRect(x: 170, y: 380, width: 660, height: 260),
            imageAsset: "infinity-installer-progress-hero-v1.png",
            zIndex: 4
        )
        hero.fill = InfinityUIKit.Palette.textPrimary

        var progress = StudioElement.make(
            name: "Installation Progress",
            kind: .progressBar,
            role: .progressBar,
            frame: CanvasRect(x: 110, y: 670, width: 780, height: 190),
            text: "PREPARING INSTALLATION",
            zIndex: 5
        )
        progress.fill = StudioColor(red: 174, green: 219, blue: 247, alpha: 255)
        progress.border = StudioColor(red: 53, green: 165, blue: 220, alpha: 255)
        progress.cornerRadius = 15
        elements.append(contentsOf: [hero, progress])
        return elements
    }

    // ------------------------=
    // FUNC: liveDetailsElement
    // DESC: Exposes runtime-owned installer values as one movable, resizable text layer with representative preview data.
    // ------------------=
    static func liveDetailsElement(screenID: Int) -> StudioElement? {
        let preview: String
        switch screenID {
        case 3:
            preview = "Disk: Example SSD\nSize: 262144 MiB\nConnection: SATA\nContents: Empty disk\nENTER: Use this disk"
        case 4:
            preview = "Name: Example SSD\nSize in MiB: 262144\nConnection: SATA\nCurrent contents: Empty\nENTER: Continue to date and time"
        case 6:
            preview = "Disk: Example SSD\nCreates: EFI boot area + Infinity Container\nPool areas: System | Personal | Applications | Recovery\nLocal time: 2026-09-07 18:28\nTime zone: UTC+00:00 Universal"
        case 10:
            preview = "The installation could not be completed.\nReview the detected issue before retrying."
        default:
            return nil
        }
        var element = StudioElement.make(
            name: "Live Installer Details", kind: .text, role: .liveDetails,
            frame: CanvasRect(x: 70, y: 562, width: 500, height: 220),
            text: preview, zIndex: 5
        )
        element.id = factoryElementID(screen: screenID, element: 100)
        element.fontSize = 16
        element.fill = InfinityUIKit.Palette.textSecondary
        return element
    }

    // ------------------------=
    // FUNC: migratedForInstallerRuntimeParity
    // DESC: Adds missing runtime details and progress layers without replacing existing authored geometry or hidden states.
    // ------------------=
    func migratedForInstallerRuntimeParity() -> InstallerStudioDocument {
        var result = self
        for index in result.screens.indices {
            if !result.screens[index].elements.contains(where: { $0.role == .liveDetails }),
               let details = Self.liveDetailsElement(screenID: result.screens[index].id) {
                result.screens[index].elements.append(details)
            }
        }
        guard let screenIndex = result.screens.firstIndex(where: { $0.id == 8 }),
              let factoryScreen = Self.factoryDefault().screens.first(where: { $0.id == 8 })
        else { return result }
        let missingProgress = !result.screens[screenIndex].elements.contains { $0.role == .progressBar }
        let missingHero = !result.screens[screenIndex].elements.contains { $0.role == .progressHero }
        guard missingProgress || missingHero else { return result }
        for index in result.screens[screenIndex].elements.indices {
            switch result.screens[screenIndex].elements[index].role {
            case .content, .body:
                result.screens[screenIndex].elements[index].hidden = true
            case .backButton, .primaryButton:
                result.screens[screenIndex].elements[index].opacity = 0
            case .footer:
                result.screens[screenIndex].elements[index].text =
                    "INSTALLATION IN PROGRESS    PLEASE KEEP THIS DEVICE POWERED"
            default:
                break
            }
        }
        for role in [StudioElementRole.progressHero, .progressBar]
            where !result.screens[screenIndex].elements.contains(where: { $0.role == role }) {
            if let required = factoryScreen.elements.first(where: { $0.role == role }) {
                result.screens[screenIndex].elements.append(required)
            }
        }
        return result
    }

    // ------------------------=
    // FUNC: dateTimeElements
    // DESC: Builds the editable Date and Time kit with semantic glass fields, badge, and map layers.
    // ------------------=
    private static func dateTimeElements(title: String, body: String) -> [StudioElement] {
        var elements = defaultElements(title: title, body: body)
        if let bodyIndex = elements.firstIndex(where: { $0.role == .body }) {
            elements[bodyIndex].frame = CanvasRect(x: 70, y: 400, width: 520, height: 28)
            elements[bodyIndex].fontSize = 14
        }

        let controls: [(String, StudioElementKind, StudioElementRole, CanvasRect, String, String)] = [
            ("Date Label", .text, .sectionLabel, CanvasRect(x: 70, y: 438, width: 250, height: 20), "DATE", ""),
            ("Date Field", .panel, .dateField, CanvasRect(x: 70, y: 462, width: 250, height: 58), "MAY 24, 2024", ""),
            ("Time Label", .text, .sectionLabel, CanvasRect(x: 340, y: 438, width: 250, height: 20), "TIME", ""),
            ("Time Field", .panel, .timeField, CanvasRect(x: 340, y: 462, width: 250, height: 58), "10:30 AM", ""),
            ("Time Zone Label", .text, .sectionLabel, CanvasRect(x: 70, y: 548, width: 520, height: 20), "TIME ZONE", ""),
            ("Time Zone Selector", .panel, .timeZoneSelector, CanvasRect(x: 70, y: 572, width: 520, height: 58), "CENTRAL TIME", ""),
            ("Offset Label", .text, .sectionLabel, CanvasRect(x: 70, y: 658, width: 180, height: 20), "UTC OFFSET", ""),
            ("UTC Offset Badge", .panel, .offsetBadge, CanvasRect(x: 70, y: 682, width: 155, height: 40), "UTC-06:00", ""),
            ("Map Label", .text, .sectionLabel, CanvasRect(x: 620, y: 410, width: 310, height: 20), "TIME ZONE SELECTION", ""),
            ("Time Zone Map", .image, .timeZoneMap, CanvasRect(x: 620, y: 438, width: 310, height: 250), "", "infinity-time-zone-map-v1.png"),
            ("Time Zone Metadata", .text, .metadata, CanvasRect(x: 620, y: 704, width: 310, height: 44), "CENTRAL TIME\nUnited States, Canada (Central)", ""),
        ]
        for (index, control) in controls.enumerated() {
            var element = StudioElement.make(
                name: control.0,
                kind: control.1,
                role: control.2,
                frame: control.3,
                text: control.4,
                imageAsset: control.5,
                zIndex: 5 + index
            )
            switch control.2 {
            case .sectionLabel:
                element.fill = InfinityUIKit.Palette.accent
                element.fontSize = 14
            case .dateField, .timeField, .timeZoneSelector:
                element.fill = InfinityUIKit.Palette.field
                element.border = InfinityUIKit.Palette.fieldBorder
                element.fontSize = 18
                element.cornerRadius = 10
            case .offsetBadge:
                element.fill = InfinityUIKit.Palette.secondaryAction
                element.border = InfinityUIKit.Palette.primaryActionBorder
                element.fontSize = 15
                element.cornerRadius = 12
            case .timeZoneMap:
                element.border = InfinityUIKit.Palette.fieldBorder
                element.cornerRadius = 12
            case .metadata:
                element.fill = InfinityUIKit.Palette.textSecondary
                element.fontSize = 14
            default:
                break
            }
            elements.append(element)
        }
        return elements
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
        element.fill = role == .primaryButton
            ? InfinityUIKit.Palette.primaryAction
            : InfinityUIKit.Palette.secondaryAction
        element.border = role == .primaryButton
            ? InfinityUIKit.Palette.primaryActionBorder
            : InfinityUIKit.Palette.secondaryActionBorder
        element.fontSize = 20
        element.cornerRadius = 10
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
