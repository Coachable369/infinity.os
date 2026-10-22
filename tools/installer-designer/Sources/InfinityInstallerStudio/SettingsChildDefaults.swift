import Foundation

enum SettingsChildDefaults {
    // ------------------------=
    // FUNC: element
    // DESC: Seeds editable native Settings children using the approved typed parent-local geometry and production copy.
    // ------------------=
    static func element(role: StudioElementRole, screenID: Int, row: Int, parent: UUID) -> StudioElement {
        let frame: CanvasRect
        let fill: StudioColor
        let border: StudioColor
        switch role {
        case .settingsSummaryDescription:
            frame = CanvasRect(x: 92, y: 600, width: 450, height: 270)
            fill = StudioColor(red: 141, green: 174, blue: 198, alpha: 255); border = .white
        case .settingsDetailDescription:
            frame = CanvasRect(x: 32, y: 120, width: 936, height: 260)
            fill = StudioColor(red: 167, green: 188, blue: 203, alpha: 255); border = .white
        case .settingsDetailSurface:
            frame = CanvasRect(x: 0, y: 0, width: 1000, height: 1000)
            fill = StudioColor(red: 8, green: 20, blue: 34, alpha: 232)
            border = StudioColor(red: 55, green: 110, blue: 128, alpha: 255)
        case .settingsSliderTrack:
            frame = CanvasRect(x: 32, y: 500, width: 936, height: 80)
            fill = StudioColor(red: 214, green: 228, blue: 241, alpha: 94)
            border = StudioColor(red: 109, green: 220, blue: 255, alpha: 255)
        default:
            frame = CanvasRect(x: 0, y: 440, width: 40, height: 200)
            fill = StudioColor(red: 239, green: 247, blue: 255, alpha: 255)
            border = StudioColor(red: 109, green: 220, blue: 255, alpha: 255)
        }
        let text = role == .settingsSummaryDescription ? summary(screenID: screenID, row: row)
            : role == .settingsDetailDescription ? detail(screenID: screenID, row: row) : ""
        var child = StudioElement.make(name: "Row \(row + 1) \(role.title)", kind: SettingsChildContract.kind(role),
            role: role, codeIdentifier: "settingsRow.\(row + 1).child\(role.rawValue)", frame: frame,
            text: text, zIndex: role == .settingsDetailSurface ? 5 : 6)
        child.id = SettingsChildContract.childID(parent: parent, role: role)
        child.parentRowID = parent; child.fill = fill; child.border = border
        child.fontSize = role == .settingsSummaryDescription ? 16 : 18
        child.actionCaptionMode = role.hasNativeCaption ? .literal : .native
        child.hidden = role == .settingsDetailDescription && screenID == 6 && row == 2
        return child
    }

    // ------------------------=
    // FUNC: summary
    // DESC: Preserves the existing native card descriptions as editable literal defaults during migration.
    // ------------------=
    static func summary(screenID: Int, row: Int) -> String {
        let values: [String]
        switch screenID {
        case 1: values = ["Your device's network identity", "Current system language", "Current regional configuration", "Installed system generation", "System update availability"]
        case 3: values = ["Signed-in local identity", "Authentication information", "Current session details", "Your private storage space", "Local profile information"]
        case 4: values = ["Local and remote routing policy", "Show or hide desktop chat", "Choose an installed local model", "Remote processing restrictions", "Microphone authorization", "Voice activation availability", "Installed model access"]
        case 5: values = ["Authority is granted explicitly", "Microphone capability status", "Remote processing permission", "Lock after a period of inactivity", "Protected interaction boundary"]
        case 6: values = ["Observed display availability", "Keyboard input availability", "Choose your pointer appearance", "Observed recording availability", "Observed playback availability"]
        case 10: values = ["Operating system information", "Native execution platform", "Current boot information", "Identity schema information", "Packaged appearance families"]
        case 11: values = ["Adjust pointer movement", "Adjust wheel scrolling", "Choose wheel direction", "Choose the primary mouse button", "Adjust pointer acceleration", "Delay before a key repeats", "Speed of repeated keystrokes", "Restore default input preferences"]
        default: values = []
        }
        return values.indices.contains(row) ? values[row] : "Inspect current configuration"
    }

    // ------------------------=
    // FUNC: detail
    // DESC: Preserves installed Settings explanations while making their text and typography explicit document properties.
    // ------------------=
    static func detail(screenID: Int, row: Int) -> String {
        let values: [String]
        switch screenID {
        case 1: values = ["Rename this machine through the durable identity service.", "The installed interface language is English (US).", "Regional formatting follows the installed US locale.", "This is the active installed System Generation.", "Updates are delivered as a complete System Generation."]
        case 3: values = ["The authenticated user owns this desktop session.", "Password credentials are managed by the trusted identity flow.", "This session is authenticated; locking preserves your workspace.", "Personal Space isolates this user's objects and preferences.", "The user profile persists with the installed system."]
        case 4: values = ["Choose whether the local provider is strictly required or preferred.", "Show or hide your local desktop AI chat widget.", "Choose the next installed local model without restarting.", "Remote processing is disabled; local inference stays on this device.", "Microphone access follows the current user permission.", "Choose the phrase that opens an eight-second command window.", "Model operations remain subject to capability enforcement."]
        case 5: values = ["Applications receive no implicit authority over your data.", "Microphone access is explicitly granted or restricted here.", "Remote AI access requires explicit capability authorization.", "Lock this user's session after the selected period without input.", "Sensitive decisions use the trusted system interface."]
        case 6: values = ["The active display is used by the native desktop compositor.", "Open Input to adjust keyboard repeat delay and rate.", "This value is read from the active System Generation.", "This audio path is unavailable; no device control is exposed.", "This audio path is unavailable; no device control is exposed."]
        case 10: values = ["InfinityOS development System Generation.", "The desktop and services execute in the native OS runtime.", "Startup uses the installed boot contract and generation checks.", "Identity records use the version 1 durable format.", "Select an installed icon family in Themes & Skins."]
        default: values = []
        }
        return values.indices.contains(row) ? values[row] : "This value is read from the active System Generation."
    }
}
