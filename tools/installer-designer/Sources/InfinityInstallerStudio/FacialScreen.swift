import Foundation

extension InstallerStudioDocument {
    // ------------------------=
    // FUNC: migratedForFacialEnrollment
    // DESC: Appends the runtime facial screen without renumbering or replacing existing authored screens.
    // ------------------=
    func migratedForFacialEnrollment() -> InstallerStudioDocument {
        var result = self
        if result.screens.count == 8 { result.screens.append(Self.facialEnrollmentScreen()) }
        if result.screens.count == 9 { result.screens.append(Self.voiceEnrollmentScreen()) }
        return result
    }

    // ------------------------=
    // FUNC: facialEnrollmentScreen
    // DESC: Defines editable native enrollment bindings; camera and mesh layers retain their live behavior.
    // ------------------=
    static func facialEnrollmentScreen() -> InstallerScreenTemplate {
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
            item.id = UUID(uuidString: String(format: "FAC10000-0000-4000-8000-%012X", elements.count + 1))!
            item.fontSize = size
            item.imageAsset = image
            if !image.isEmpty { item.imageFit = (identifier == "faceSprite" || identifier.hasPrefix("facePoseGuide")) ? .fit : .legacy }
            if identifier.hasPrefix("facePoseGuide"), let slot = Int(identifier.suffix(1)) {
                item.crop = ImageCrop(left: [0,33,67][slot], top: 0, right: [67,33,0][slot], bottom: 0)
            }
            if identifier == "faceName" { item.inputVariable = .facialProfileName }
            if kind == .panel || kind == .console || kind == .button {
                item.surfaceMaterial = .glass
                item.surfaceCorners = .rounded
                item.cornerRadius = 14
            }
            if identifier == "facePanel" {
                item.fill = StudioColor(red: 7, green: 23, blue: 48, alpha: 190)
                item.cornerRadius = 20
            } else if identifier.hasPrefix("facePhotoCard") || identifier == "facePreviewCard" {
                item.fill = StudioColor(red: 10, green: 32, blue: 60, alpha: 135)
            } else if identifier == "facePrimary" {
                item.fill = StudioColor(red: 35, green: 107, blue: 255, alpha: 255)
            }
            if ["facePreview", "facePhoto0", "facePhoto1", "facePhoto2"].contains(identifier) {
                item.fill.alpha = 0
                item.border.alpha = 0
            }
            elements.append(item)
        }
        add("First-Boot Background", "firstBootBackground", .image, .masthead, 0, 0, 1000, 1000, "", 19, "installer-assets/Glossy-Blue-Infinity-Wave-Wallpaper-2.png")
        add("facePanel", "facePanel", .console, .console, 45, 65, 910, 870, "", 20, "")
        add("faceEyebrow", "faceEyebrow", .text, .decoration, 85, 92, 480, 30, "SECURE SIGN-IN", 22, "")
        add("faceTitle", "faceTitle", .text, .title, 85, 130, 530, 80, "Create your facial identity", 64, "")
        add("faceSubtitle", "faceSubtitle", .text, .decoration, 85, 200, 530, 40, "Capture three views to set up facial sign-in.", 28, "")
        add("facePreview", "facePreviewCard", .panel, .decoration, 85, 270, 300, 310, "", 20, "")
        add("facePreview", "facePreview", .panel, .decoration, 145, 295, 185, 260, "", 20, "")
        add("facePreviewLabel", "facePreviewLabel", .text, .decoration, 100, 280, 260, 32, "Live camera", 18, "")
        add("faceGuidance", "faceGuidance", .text, .body, 405, 400, 190, 90, "", 23, "")
        add("faceNameLabel", "faceNameLabel", .text, .decoration, 405, 490, 180, 26, "Identity name", 16, "")
        add("faceName", "faceName", .panel, .input, 405, 522, 190, 46, "", 19, "")
        add("facePhotoCard0", "facePhotoCard0", .console, .decoration, 85, 600, 158, 200, "", 20, "")
        add("facePhotoLabel0", "facePhotoLabel0", .text, .decoration, 97, 612, 134, 30, "1   Front", 21, "")
        add("Front pose guide", "facePoseGuide0", .image, .decoration, 107, 631, 114, 145, "", 20, "installer-assets/Face-Enrollment-Poses-v2.png")
        add("facePhoto0", "facePhoto0", .panel, .decoration, 123, 650, 82, 109, "", 20, "")
        add("facePhotoStatus0", "facePhotoStatus0", .text, .decoration, 97, 765, 134, 26, "", 21, "")
        add("facePhotoCard1", "facePhotoCard1", .console, .decoration, 255, 600, 158, 200, "", 20, "")
        add("facePhotoLabel1", "facePhotoLabel1", .text, .decoration, 267, 612, 134, 30, "2   Left", 21, "")
        add("Left pose guide", "facePoseGuide1", .image, .decoration, 277, 631, 114, 145, "", 20, "installer-assets/Face-Enrollment-Poses-v2.png")
        add("facePhoto1", "facePhoto1", .panel, .decoration, 293, 650, 82, 109, "", 20, "")
        add("facePhotoStatus1", "facePhotoStatus1", .text, .decoration, 267, 765, 134, 26, "", 21, "")
        add("facePhotoCard2", "facePhotoCard2", .console, .decoration, 425, 600, 158, 200, "", 20, "")
        add("facePhotoLabel2", "facePhotoLabel2", .text, .decoration, 437, 612, 134, 30, "3   Right", 21, "")
        add("Right pose guide", "facePoseGuide2", .image, .decoration, 447, 631, 114, 145, "", 20, "installer-assets/Face-Enrollment-Poses-v2.png")
        add("facePhoto2", "facePhoto2", .panel, .decoration, 463, 650, 82, 109, "", 20, "")
        add("facePhotoStatus2", "facePhotoStatus2", .text, .decoration, 437, 765, 134, 26, "", 21, "")
        add("Scanning face sprite", "faceSprite", .image, .decoration, 600, 85, 340, 700, "", 19, "installer-assets/Face-Enrollment-Scan-v2.png")
        add("faceMesh", "faceMesh", .panel, .decoration, 600, 85, 340, 700, "", 20, "")
        add("faceStatus", "faceStatus", .text, .decoration, 85, 810, 420, 115, "", 15, "")
        add("faceBack", "faceBack", .button, .backButton, 515, 835, 100, 58, "Back", 21, "")
        add("faceSkip", "faceSkip", .button, .decoration, 625, 835, 125, 58, "Skip for now", 21, "")
        add("facePrimary", "facePrimary", .button, .primaryButton, 760, 835, 170, 58, "", 21, "")
        add("Capture step heading", "faceStepTitle", .text, .decoration, 405, 342, 190, 48, "Face the camera", 32, "")
        return InstallerScreenTemplate(id: 9, title: "Facial Recognition", elements: elements)
    }
}
