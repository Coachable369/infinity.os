import Foundation

extension InstallerStudioDocument {
    // ------------------------=
    // FUNC: migratedForFacialEnrollment
    // DESC: Appends the runtime facial screen without renumbering or replacing existing authored screens.
    // ------------------=
    func migratedForFacialEnrollment() -> InstallerStudioDocument {
        guard screens.count == 8 else { return self }
        var result = self
        result.screens.append(Self.facialEnrollmentScreen())
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
                 _ text: String = "") {
            var item = StudioElement.make(name: name, kind: kind, role: role,
                codeIdentifier: identifier, frame: CanvasRect(x: x, y: y, width: w, height: h),
                text: text, locked: role == .backButton || role == .primaryButton,
                zIndex: elements.count)
            item.id = UUID(uuidString: String(format: "FAC10000-0000-4000-8000-%012X", elements.count + 1))!
            item.fontSize = role == .title ? 32 : 20
            if identifier == "faceName" { item.inputVariable = .facialProfileName }
            elements.append(item)
        }
        add("Enrollment Panel", "facePanel", .console, .console, 80, 160, 840, 700)
        add("Title", "faceTitle", .text, .title, 98, 192, 800, 64, "Create your facial identity")
        add("Capture Guidance", "faceGuidance", .text, .body, 98, 270, 400, 140)
        add("Face Name / Retake", "faceName", .panel, .input, 98, 415, 400, 56)
        add("Live Camera", "facePreview", .panel, .decoration, 98, 505, 120, 160)
        add("Camera Label", "facePreviewLabel", .text, .decoration, 98, 477, 120, 24, "Live camera preview")
        for (index, label) in ["Front", "Left", "Right"].enumerated() {
            let x = 232 + index * 90
            add("\(label) Photo", "facePhoto\(index)", .panel, .decoration, x, 505, 76, 101)
            add("\(label) Label", "facePhotoLabel\(index)", .text, .decoration, x, 616, 76, 22, label)
            add("\(label) Status", "facePhotoStatus\(index)", .text, .decoration, x, 646, 76, 22)
        }
        add("Rotating Face · Live Animation", "faceMesh", .panel, .decoration, 530, 260, 360, 480)
        add("Status / Errors", "faceStatus", .text, .decoration, 98, 670, 400, 80)
        add("Back", "faceBack", .button, .backButton, 98, 772, 248, 56, "Back")
        add("Skip", "faceSkip", .button, .decoration, 364, 772, 248, 56, "Skip")
        add("Capture / Verify / Continue", "facePrimary", .button, .primaryButton, 630, 772, 248, 56)
        return InstallerScreenTemplate(id: 9, title: "Facial Recognition", elements: elements)
    }
}
