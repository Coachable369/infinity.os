import Foundation

extension InstallerStudioDocument {
    // ------------------------=
    // FUNC: factoryLogin
    // DESC: Exposes authentication artwork and controls as authored layers while retaining native password and sign-in actions.
    // ------------------=
    static func factoryLogin() -> InstallerStudioDocument {
        var elements: [StudioElement] = []
        // ------------------------=
        // FUNC: add
        // DESC: Converts the existing 1536 by 1024 authentication layout to the shared normalized canvas.
        // ------------------=
        func add(_ name: String, _ kind: StudioElementKind, _ role: StudioElementRole,
                 _ x: Int, _ y: Int, _ w: Int, _ h: Int, _ text: String = "", _ image: String = "") {
            var element = StudioElement.make(name: name, kind: kind, role: role,
                frame: CanvasRect(x: x * 1000 / 1536, y: y * 1000 / 1024,
                    width: max(1, w * 1000 / 1536), height: max(1, h * 1000 / 1024)),
                text: text, imageAsset: image, zIndex: elements.count)
            element.fontSize = 22
            element.imageFit = .fit
            if role == .input { element.inputVariable = .password }
            elements.append(element)
        }
        add("Login Background", .image, .sceneBackground, 0, 0, 1536, 1024, "", "assets/desktop/infinity-shell-wallpaper-v3.bmp")
        elements[0].imageFit = .fill
        add("Login Panel", .panel, .console, 54, 123, 521, 754)
        add("Infinity Symbol", .image, .image, 206, 166, 218, 100, "", "infinity-emblem-v2.bmp")
        add("Welcome", .text, .title, 82, 271, 465, 49, "Welcome Back")
        elements[elements.count - 1].fontSize = 36
        add("Tagline", .text, .body, 82, 325, 465, 31, "Secure. Limitless.")
        add("Login Account", .button, .loginAccount, 103, 376, 423, 86, "Select an account")
        add("Password", .panel, .input, 103, 479, 423, 57)
        add("Sign In", .button, .primaryButton, 103, 562, 423, 56, "Sign In")
        add("Security Key", .button, .loginSecurityKey, 103, 680, 423, 56, "Sign In with Security Key")
        for (i, title) in ["Add User", "Forgot Password", "Options"].enumerated() {
            add(title, .button, [.loginAddUser, .loginRecovery, .loginOptions][i], 103 + i * 141, 751, 137, 75, title)
            elements[elements.count - 1].fontSize = 14
        }
        add("Utility Tray", .panel, .content, 529, 914, 478, 90)
        for (i, title) in ["Shut Down", "Restart", "Network", "Accessibility"].enumerated() {
            add(title, .button, [.loginShutdown, .loginRestart, .loginNetwork, .loginAccessibility][i], 534 + i * 119, 924, 109, 70, title)
            elements[elements.count - 1].fontSize = 14
        }
        add("Login Wordmark", .image, .image, 900, 92, 570, 92, "", "infinity-eclipse-header-v1.bmp")
        elements[elements.count - 1].crop = ImageCrop(left: 14, top: 1, right: 12, bottom: 80)
        return InstallerStudioDocument(version: currentVersion, canvasWidth: 1000, canvasHeight: 1000,
            screens: [InstallerScreenTemplate(id: 1, title: "Login", elements: elements)])
    }
}
