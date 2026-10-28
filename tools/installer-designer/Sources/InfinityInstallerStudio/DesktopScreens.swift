import Foundation

extension InstallerStudioDocument {
    // ------------------------=
    // FUNC: factoryDesktop
    // DESC: Exposes desktop chrome as native-bound layers using the existing Eclipse surfaces, fonts and icon family.
    // ------------------=
    static func factoryDesktop() -> InstallerStudioDocument {
        var screens: [InstallerScreenTemplate] = []
        for screen in 1...3 {
            var layers: [StudioElement] = []
            // ------------------------=
            // FUNC: add
            // DESC: Creates an editable layer in the same normalized canvas used by native desktop geometry.
            // ------------------=
            func add(_ name: String, _ kind: StudioElementKind, _ role: StudioElementRole,
                     _ x: Int, _ y: Int, _ w: Int, _ h: Int, _ text: String = "", _ image: String = "") {
                var e = StudioElement.make(name: name, kind: kind, role: role,
                    frame: CanvasRect(x: x, y: y, width: w, height: h), text: text,
                    imageAsset: image, zIndex: layers.count)
                e.fontSize = 16; e.imageFit = .fit; e.locked = false
                layers.append(e)
            }
            add("Desktop Wallpaper", .image, .sceneBackground, 0, 0, 1000, 1000, "", "assets/desktop/infinity-shell-wallpaper-v3.bmp")
            layers[0].imageFit = .fill
            if screen == 1 {
                add("Navigation Glass", .panel, .console, 0, 0, 1000, 40)
                add("Infinity Menu", .panel, .desktopBrand, 8, 4, 170, 32)
                add("Infinity Emblem", .image, .image, 10, 5, 43, 30, "", "assets/desktop/infinity-topbar-icon-v2.bmp")
                add("Infinity Wordmark", .text, .body, 58, 7, 115, 26, "I N F I N I T Y O S")
                add("Help Menu", .text, .desktopHelp, 380, 7, 60, 26, "Help")
                add("Privacy Status", .text, .desktopPrivacy, 580, 7, 130, 26, "LOCAL | PRIVATE")
                let roles: [StudioElementRole] = [.desktopVolume,.desktopNetwork,.desktopBluetooth,.desktopBattery,.desktopDevices,.desktopSearch,.desktopSettings]
                let names = ["Volume","Network","Bluetooth","Battery","Devices","Search","Settings"]
                let icons = ["40-volume","37-wifi","38-bluetooth","39-battery","19-computer","27-search","26-settings"]
                for i in 0..<roles.count {
                    add(names[i], .image, roles[i], 714+i*28, 5, 24, 30, "",
                        "assets/icons/crystal-blue-glass/64/base/\(icons[i]).png")
                }
                add("System Clock", .text, .desktopClock, 915, 7, 80, 26, "12:34:56")
            } else if screen == 2 {
                add("Overview Glass", .panel, .console, 680, 70, 280, 300)
                add("Overview Title", .text, .title, 697, 86, 240, 30, "SYSTEM OVERVIEW")
                let roles: [StudioElementRole] = [.desktopMachine,.desktopGeneration,.desktopRuntime,.desktopStorage,.desktopInput]
                let labels = ["Machine Name","System Generation","Runtime","Storage Service","Input"]
                let values = ["Infinity","Active","Online","Ready","Ready"]
                for i in 0..<roles.count {
                    add(labels[i], .text, .body, 697, 130+i*45, 155, 28, labels[i])
                    add("\(labels[i]) Value", .text, roles[i], 857, 130+i*45, 85, 28, values[i])
                }
            } else {
                add("AI Widget Glass", .panel, .console, 680, 390, 280, 508)
                add("AI Title", .text, .title, 697, 403, 155, 27, "INFINITY AI")
                add("Minimize", .button, .desktopMinimize, 909, 400, 20, 28, "-")
                add("Close", .button, .desktopClose, 934, 400, 20, 28, "x")
                add("Voice Waveform", .panel, .desktopVoice, 692, 438, 256, 42)
                add("Selected Model", .panel, .desktopModel, 692, 494, 256, 64, "Hermes 3 Llama 3.2 3B")
                add("Conversation", .panel, .desktopTimeline, 692, 573, 256, 227)
                add("Prompt Input", .panel, .desktopComposer, 692, 817, 210, 64)
                add("Send", .button, .desktopSend, 912, 817, 36, 64, "↑")
            }
            screens.append(InstallerScreenTemplate(id: screen,
                title: ["Desktop Navigation", "System Overview Widget", "AI Voice Widget"][screen-1], elements: layers))
        }
        return InstallerStudioDocument(version: currentVersion, canvasWidth: 1000, canvasHeight: 1000, screens: screens)
    }
}
