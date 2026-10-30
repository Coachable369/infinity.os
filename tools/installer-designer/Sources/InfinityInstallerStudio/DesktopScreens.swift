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
                add("Infinity Emblem", .image, .desktopBrand, 10, 5, 43, 30, "", "assets/desktop/infinity-topbar-icon-v2.bmp")
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
                add("AI Widget Glass", .panel, .console, 680, 390, 280, 471)
                add("AI Title", .text, .title, 697, 403, 155, 27, "INFINITY AI")
                add("Minimize", .button, .desktopMinimize, 909, 400, 20, 28, "-")
                add("Close", .button, .desktopClose, 934, 400, 20, 28, "x")
                add("Voice Waveform", .panel, .desktopVoice, 692, 438, 256, 42)
                add("Selected Model", .panel, .desktopModel, 692, 494, 256, 64, "Hermes 3 Llama 3.2 3B")
                add("Conversation", .panel, .desktopTimeline, 692, 573, 256, 190)
                add("Prompt Input", .panel, .desktopComposer, 692, 780, 210, 64)
                add("Send", .button, .desktopSend, 912, 780, 36, 64, "↑")
            }
            screens.append(InstallerScreenTemplate(id: screen,
                title: ["Desktop Navigation", "System Overview Widget", "AI Voice Widget"][screen-1], elements: layers))
        }
        return InstallerStudioDocument(version: currentVersion, canvasWidth: 1000, canvasHeight: 1000, screens: screens).migratedForDesktopDock()
    }

    // ------------------------=
    // FUNC: migratedForDesktopDock
    // DESC: Adds the native dock editor without changing authored navigation, widgets, menus or existing custom scenes.
    // ------------------=
    func migratedForDesktopDock() -> InstallerStudioDocument {
        guard !screens.contains(where: { $0.elements.contains(where: { $0.role == .desktopDock }) }),
              screens.count < Self.maximumScreenCount else { return self }
        var result = self
        var layers = [StudioElement.make(name: "Dock Glass", kind: .panel, role: .desktopDock,
            frame: CanvasRect(x: 230, y: 914, width: 540, height: 72), zIndex: 0)]
        let roles: [StudioElementRole] = [.dockLauncher,.dockFiles,.dockSettings,.dockAbout,.dockVoice,.dockSearch,.dockWidgets,.dockTrash]
        let names = ["Applications","Files","Settings","About InfinityOS","AI Voice","Search","Widgets","Trash"]
        let icons = ["assets/desktop/infinity-topbar-icon-v2.png", "assets/icons/crystal-blue-glass/64/base/02-folder.png",
            "assets/icons/crystal-blue-glass/64/base/26-settings.png", "assets/icons/crystal-blue-glass/64/base/28-information.png",
            "assets/icons/crystal-blue-glass/64/base/23-microphone.png", "assets/icons/crystal-blue-glass/64/base/27-search.png",
            "assets/icons/crystal-blue-glass/64/base/19-computer.png", "assets/icons/crystal-blue-glass/64/base/10-trash-empty.png"]
        for index in roles.indices {
            var layer = StudioElement.make(name: names[index], kind: .image, role: roles[index],
                frame: CanvasRect(x: 242 + index * 67, y: 924, width: 46, height: 46),
                imageAsset: icons[index], zIndex: index + 1)
            layer.imageFit = .fit; layer.locked = false
            layers.append(layer)
        }
        result.screens.append(InstallerScreenTemplate(id: (screens.map(\.id).max() ?? 3) + 1,
            title: "Desktop App Dock", elements: layers))
        return result
    }
}
