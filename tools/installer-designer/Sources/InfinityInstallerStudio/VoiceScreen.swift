import Foundation

extension InstallerStudioDocument {
    // ------------------------=
    // FUNC: voiceEnrollmentScreen
    // DESC: Defines editable voice enrollment bindings with shared UIKit controls and generated microphone artwork.
    // ------------------=
    static func voiceEnrollmentScreen() -> InstallerScreenTemplate {
        var elements: [StudioElement] = []
        // ------------------------=
        // FUNC: add
        // DESC: Creates a stable typed screen binding using the shared Precision component palette.
        // ------------------=
        func add(_ name: String, _ identifier: String, _ kind: StudioElementKind,
                 _ role: StudioElementRole, _ x: Int, _ y: Int, _ w: Int, _ h: Int,
                 _ text: String = "", _ size: Int = 20, _ image: String = "") {
            var item = StudioElement.make(name: name, kind: kind, role: role,
                codeIdentifier: identifier, frame: CanvasRect(x: x, y: y, width: w, height: h),
                text: text, locked: role == .backButton || role == .primaryButton,
                zIndex: elements.count)
            item.id = UUID(uuidString: String(format: "B01C0000-0000-4000-8000-%012X", elements.count + 1))!
            item.fontSize = size
            item.imageAsset = image
            if !image.isEmpty { item.imageFit = (identifier == "voiceSprite" || identifier.hasPrefix("voicePoseGuide")) ? .fit : .legacy }
            if kind == .panel || kind == .console || kind == .button {
                item.surfaceMaterial = .glass
                item.surfaceCorners = .rounded
                item.cornerRadius = 14
            }
            if identifier == "voicePanel" {
                item.fill = StudioColor(red: 7, green: 23, blue: 48, alpha: 190)
                item.cornerRadius = 20
            } else if identifier.hasPrefix("voicePhotoCard") || identifier == "voicePreviewCard" {
                item.fill = StudioColor(red: 10, green: 32, blue: 60, alpha: 135)
            } else if identifier == "voicePrimary" {
                item.fill = StudioColor(red: 35, green: 107, blue: 255, alpha: 255)
            }
            if ["voiceWave", "voiceSampleWave0", "voiceSampleWave1", "voiceSampleWave2"].contains(identifier) {
                item.fill.alpha = 0
                item.border.alpha = 0
            }
            elements.append(item)
        }
        add("Background", "firstBootBackground", .image, .masthead, 0, 0, 1000, 1000, "", 19, "installer-assets/Glossy-Blue-Infinity-Wave-Wallpaper-2.png")
        add("Panel", "voicePanel", .console, .console, 45, 65, 910, 870, "", 20, "")
        add("Eyebrow", "voiceEyebrow", .text, .decoration, 85, 92, 480, 30, "SECURE SIGN-IN", 22, "")
        add("Title", "voiceTitle", .text, .title, 85, 130, 720, 80, "Create your voice identity", 62, "")
        add("Subtitle", "voiceSubtitle", .text, .decoration, 85, 200, 680, 40, "Add a voice check after facial sign-in.", 28, "")
        add("Read aloud card", "voicePreviewCard", .console, .decoration, 85, 265, 305, 310, "", 20, "")
        add("Read aloud", "voiceReadLabel", .text, .decoration, 105, 282, 260, 35, "READ ALOUD", 22, "")
        add("Phrase", "voicePhrase", .text, .body, 105, 322, 260, 100, "The morning sky is clear and bright.", 36, "")
        add("Waveform", "voiceWave", .panel, .decoration, 110, 440, 250, 65, "", 20, "")
        add("Recording status", "voiceRecordStatus", .text, .decoration, 112, 521, 250, 30, "Ready to record", 22, "")
        add("Step title", "voiceStepTitle", .text, .decoration, 405, 350, 210, 45, "Read the phrase", 34, "")
        add("Instructions", "voiceGuidance", .text, .body, 405, 402, 210, 110, "Speak clearly and naturally in a quiet place.", 27, "")
        add("Microphone", "voiceSprite", .image, .decoration, 620, 140, 300, 640, "", 20, "installer-assets/Voice-Enrollment-Microphone-v1.png")
        add("Notice", "voiceStatus", .text, .decoration, 85, 845, 420, 65, "Your voice stays on this device. Password sign-in remains available.", 19, "")
        add("Back", "voiceBack", .button, .backButton, 515, 835, 100, 58, "Back", 21, "")
        add("Skip", "voiceSkip", .button, .decoration, 625, 835, 125, 58, "Skip for now", 21, "")
        add("Record", "voicePrimary", .button, .primaryButton, 760, 835, 170, 58, "Record sample", 21, "")
        add("Sample card", "voicePhotoCard0", .console, .decoration, 85, 600, 158, 200, "", 20, "")
        add("Sample heading", "voiceSampleLabel0", .text, .decoration, 97, 612, 134, 35, "1   First sample", 22, "")
        add("Sample wave", "voiceSampleWave0", .panel, .decoration, 110, 660, 108, 70, "", 20, "")
        add("Sample status", "voiceSampleStatus0", .text, .decoration, 97, 765, 134, 26, "Pending", 21, "")
        add("Sample card", "voicePhotoCard1", .console, .decoration, 255, 600, 158, 200, "", 20, "")
        add("Sample heading", "voiceSampleLabel1", .text, .decoration, 267, 612, 134, 35, "2   Second sample", 22, "")
        add("Sample wave", "voiceSampleWave1", .panel, .decoration, 280, 660, 108, 70, "", 20, "")
        add("Sample status", "voiceSampleStatus1", .text, .decoration, 267, 765, 134, 26, "Pending", 21, "")
        add("Sample card", "voicePhotoCard2", .console, .decoration, 425, 600, 158, 200, "", 20, "")
        add("Sample heading", "voiceSampleLabel2", .text, .decoration, 437, 612, 134, 35, "3   Verify", 22, "")
        add("Sample wave", "voiceSampleWave2", .panel, .decoration, 450, 660, 108, 70, "", 20, "")
        add("Sample status", "voiceSampleStatus2", .text, .decoration, 437, 765, 134, 26, "Pending", 21, "")
        return InstallerScreenTemplate(id: 10, title: "Voice Identity", elements: elements)
    }
}
