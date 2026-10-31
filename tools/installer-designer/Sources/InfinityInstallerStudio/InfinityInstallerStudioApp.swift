import SwiftUI
import Darwin

@main
struct InfinityInstallerStudioApp: App {
    @StateObject private var store = TemplateStore()
    @NSApplicationDelegateAdaptor(StudioApplicationDelegate.self) private var applicationDelegate

    // ------------------------=
    // FUNC: init
    // DESC: Supports deterministic factory-template export before launching the native editor UI.
    // ------------------=
    init() {
        let arguments = CommandLine.arguments
        if arguments.count == 3, arguments[1] == "--export-desktop-default" {
            do {
                let root = URL(fileURLWithPath: arguments[2], isDirectory: true)
                let document = InstallerStudioDocument.factoryDesktop()
                let encoder = JSONEncoder()
                encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
                try ContentAwareAtomicWriter.write(encoder.encode(document), to: root.appending(path: "assets/boot/desktop-screens.infinityui"))
                try ContentAwareAtomicWriter.write(RuntimeTemplateCodec.encode(document, assetRoot: root, collection: .desktop),
                    to: root.appending(path: "assets/boot/desktop-screens.iuit"))
                Darwin.exit(0)
            } catch {
                fputs("Desktop template export failed: \(error)\n", stderr)
                Darwin.exit(1)
            }
        }
        if arguments.count == 3, arguments[1] == "--export-settings-default" {
            do {
                let source = URL(fileURLWithPath: arguments[2])
                let document = InstallerStudioDocument.factorySystemSettings()
                let encoder = JSONEncoder()
                encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
                try encoder.encode(document).write(to: source, options: .atomic)
                print("Exported System Settings source template to \(source.path)")
                Darwin.exit(0)
            } catch {
                fputs("Settings template export failed: \(error)\n", stderr)
                Darwin.exit(1)
            }
        }
        if arguments.count == 4, arguments[1] == "--compile-template" {
            do {
                let source = URL(fileURLWithPath: arguments[2])
                let destination = URL(fileURLWithPath: arguments[3])
                var document = try JSONDecoder().decode(
                    InstallerStudioDocument.self,
                    from: Data(contentsOf: source)
                )
                if source.lastPathComponent == "installer-screens.infinityui" {
                    document = document.migratedForInstallerRuntimeParity()
                }
                if source.lastPathComponent == "settings-screens.infinityui" {
                    document = document.migratedForSettingsCardReadability()
                }
                if source.lastPathComponent == "login-screens.infinityui" {
                    document = document.migratedForLoginMotion()
                }
                if source.lastPathComponent == "configuration-screens.infinityui" {
                    document = document.migratedForFacialEnrollment()
                }
                let collection: ScreenCollection = if source.lastPathComponent == "boot-screens.infinityui" {
                    .boot
                } else if source.lastPathComponent == "desktop-screens.infinityui" {
                    .desktop
                } else if source.lastPathComponent == "configuration-screens.infinityui" {
                    .configuration
                } else if source.lastPathComponent == "login-screens.infinityui" {
                    .login
                } else if source.lastPathComponent == "settings-screens.infinityui" {
                    .systemSettings
                } else {
                    .installation
                }
                try TemplateValidator.validate(document, for: collection)
                try FileManager.default.createDirectory(
                    at: destination.deletingLastPathComponent(),
                    withIntermediateDirectories: true
                )
                let compiled = try RuntimeTemplateCodec.encode(
                    document,
                    assetRoot: source.deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent(),
                    collection: collection
                )
                try ContentAwareAtomicWriter.write(compiled, to: destination)
                print("Compiled \(source.path) to \(destination.path)")
                Darwin.exit(0)
            } catch {
                fputs("Template compilation failed: \(error)\n", stderr)
                Darwin.exit(1)
            }
        }
        if arguments.count == 3, arguments[1] == "--export-login-default" {
            do {
                let root = URL(fileURLWithPath: arguments[2], isDirectory: true)
                let source = InstallerStudioDocument.factoryLogin()
                let encoder = JSONEncoder()
                encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
                try ContentAwareAtomicWriter.write(encoder.encode(source), to: root.appending(path: "assets/boot/login-screens.infinityui"))
                try ContentAwareAtomicWriter.write(RuntimeTemplateCodec.encode(source, assetRoot: root, collection: .login),
                    to: root.appending(path: "assets/boot/login-screens.iuit"))
                Darwin.exit(0)
            } catch {
                fputs("Login template export failed: \(error)\n", stderr)
                Darwin.exit(1)
            }
        }
        if arguments.count == 3, arguments[1] == "--export-boot-default" {
            do {
                let root = URL(fileURLWithPath: arguments[2], isDirectory: true)
                let source = InstallerStudioDocument.factoryBoot()
                let encoder = JSONEncoder()
                encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
                try ContentAwareAtomicWriter.write(encoder.encode(source), to: root.appending(path: "assets/boot/boot-screens.infinityui"))
                try ContentAwareAtomicWriter.write(RuntimeTemplateCodec.encode(source, assetRoot: root, collection: .boot),
                    to: root.appending(path: "assets/boot/boot-screens.iuit"))
                Darwin.exit(0)
            } catch {
                fputs("Boot template export failed: \(error)\n", stderr)
                Darwin.exit(1)
            }
        }
        if arguments.count == 3, arguments[1] == "--export-default" {
            do {
                let directory = URL(fileURLWithPath: arguments[2], isDirectory: true)
                try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
                let document = InstallerStudioDocument.factoryDefault()
                let configuration = InstallerStudioDocument.factoryConfiguration()
                let settings = InstallerStudioDocument.factorySystemSettings()
                let encoder = JSONEncoder()
                encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
                try encoder.encode(document).write(
                    to: directory.appending(path: "installer-screens.infinityui"),
                    options: .atomic
                )
                try RuntimeTemplateCodec.encode(document, assetRoot: directory.deletingLastPathComponent().deletingLastPathComponent(), collection: .installation).write(
                    to: directory.appending(path: "installer-screens.iuit"),
                    options: .atomic
                )
                try encoder.encode(configuration).write(
                    to: directory.appending(path: "configuration-screens.infinityui"),
                    options: .atomic
                )
                try RuntimeTemplateCodec.encode(configuration, assetRoot: directory.deletingLastPathComponent().deletingLastPathComponent(), collection: .configuration).write(
                    to: directory.appending(path: "configuration-screens.iuit"),
                    options: .atomic
                )
                try encoder.encode(settings).write(
                    to: directory.appending(path: "settings-screens.infinityui"), options: .atomic
                )
                try RuntimeTemplateCodec.encode(settings, assetRoot: directory.deletingLastPathComponent().deletingLastPathComponent(), collection: .systemSettings).write(
                    to: directory.appending(path: "settings-screens.iuit"), options: .atomic
                )
                print("Exported InfinityOS installer, first-boot, and System Settings templates to \(directory.path)")
                Darwin.exit(0)
            } catch {
                fputs("Template export failed: \(error)\n", stderr)
                Darwin.exit(1)
            }
        }
    }

    var body: some Scene {
        WindowGroup("Infinity Studio") {
            StudioRootView(store: store)
                .onAppear { applicationDelegate.store = store }
                .frame(minWidth: 1040, minHeight: 640)
                .preferredColorScheme(.dark)
        }
        .windowStyle(.hiddenTitleBar)
        .windowResizability(.contentMinSize)
        .defaultSize(width: 1480, height: 900)
        .commands {
            CommandGroup(replacing: .saveItem) {
                Button("Save Section") { store.save() }
                    .keyboardShortcut("s", modifiers: .command)
                Button("Save All Sections") { store.saveAll() }
                    .keyboardShortcut("s", modifiers: [.command, .shift])
                Button("Save & Update ISO…") { store.showISOUpdate = true }
                Button("Export Section Templates…") { store.chooseTemplateExport() }
                    .disabled(store.editingUIKit)
                Button(store.editingUIKit ? "Import Artwork…" : "Import Templates…") {
                    if store.editingUIKit { store.uiKit.chooseReplacementImage() }
                    else { store.importDocument() }
                }
                    .keyboardShortcut("o", modifiers: .command)
            }
            CommandGroup(replacing: .undoRedo) {
                Button("Undo") { store.undo() }
                    .keyboardShortcut("z", modifiers: .command)
                    .disabled(!store.canUndo)
                Button("Redo") { store.redo() }
                    .keyboardShortcut("z", modifiers: [.command, .shift])
                    .disabled(!store.canRedo)
            }
            CommandGroup(replacing: .pasteboard) {
                Button("Cut") { NSApp.sendAction(#selector(NSText.cut(_:)), to: nil, from: nil) }
                    .keyboardShortcut("x", modifiers: .command)
                Button("Copy") {
                    if NSApp.keyWindow?.firstResponder is NSTextView {
                        NSApp.sendAction(#selector(NSText.copy(_:)), to: nil, from: nil)
                    } else if !store.editingMenu { store.copySelectedElements() }
                }.keyboardShortcut("c", modifiers: .command)
                Button("Paste") {
                    if NSApp.keyWindow?.firstResponder is NSTextView {
                        NSApp.sendAction(#selector(NSText.paste(_:)), to: nil, from: nil)
                    } else if !store.editingMenu { store.pasteElementsInPlace() }
                }.keyboardShortcut("v", modifiers: .command)
                Button("Copy Selected Elements") { store.copySelectedElements() }
                    .keyboardShortcut("c", modifiers: [.command, .shift])
                    .disabled(store.editingUIKit || store.editingMenu || store.selectedElementIDs.isEmpty)
                Button("Paste Elements in Place") { store.pasteElementsInPlace() }
                    .keyboardShortcut("v", modifiers: [.command, .shift])
                    .disabled(store.editingUIKit || store.editingMenu)
                Divider()
                Button(store.editingMenu ? "Duplicate Menu Item" : "Duplicate Element") {
                    if store.editingMenu {
                        if let id = store.selectedMenuItemID { store.selectedMenuItemID = store.duplicateMenuItem(id) }
                    } else { store.duplicateSelected() }
                }
                    .keyboardShortcut("d", modifiers: .command)
                    .disabled(store.editingUIKit)
                Button(store.editingMenu ? "Delete Menu Item" : "Delete Element") {
                    if NSApp.keyWindow?.firstResponder is NSTextView {
                        NSApp.sendAction(#selector(NSText.delete(_:)), to: nil, from: nil)
                    } else if store.editingMenu { store.deleteSelectedMenuItem() }
                    else { store.deleteSelected() }
                }
                    .keyboardShortcut(.delete, modifiers: [])
                    .disabled(store.editingUIKit)
            }
            CommandMenu("Canvas") {
                Button(store.marqueeSelectionEnabled ? "Disable Marquee Select" : "Enable Marquee Select") {
                    store.toggleMarqueeSelection()
                }
                .keyboardShortcut("m", modifiers: [.command, .shift])
                .disabled(store.editingUIKit || store.editingMenu)
                Divider()
                Button("Zoom In") { store.zoomIn() }
                    .keyboardShortcut("+", modifiers: .command)
                Button("Zoom Out") { store.zoomOut() }
                    .keyboardShortcut("-", modifiers: .command)
                Button("Reset Zoom") { store.resetZoom() }
                    .keyboardShortcut("0", modifiers: .command)
                Divider()
                Button("Add Screen") { store.addScreen() }
                    .keyboardShortcut("n", modifiers: [.command, .shift])
                    .disabled(store.editingUIKit || !store.canAddScreen)
                Button("Duplicate Screen") { store.duplicateScreen() }
                    .keyboardShortcut("d", modifiers: [.command, .shift])
                    .disabled(store.editingUIKit || !store.canAddScreen)
            }
        }
    }
}
