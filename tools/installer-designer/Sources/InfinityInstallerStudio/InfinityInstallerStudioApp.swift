import SwiftUI
import Darwin

@main
struct InfinityInstallerStudioApp: App {
    @StateObject private var store = TemplateStore()

    // ------------------------=
    // FUNC: init
    // DESC: Supports deterministic factory-template export before launching the native editor UI.
    // ------------------=
    init() {
        let arguments = CommandLine.arguments
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
                let collection: ScreenCollection = if source.lastPathComponent == "configuration-screens.infinityui" {
                    .configuration
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
                try RuntimeTemplateCodec.encode(
                    document,
                    assetRoot: source.deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent(),
                    collection: collection
                ).write(to: destination, options: .atomic)
                print("Compiled \(source.path) to \(destination.path)")
                Darwin.exit(0)
            } catch {
                fputs("Template compilation failed: \(error)\n", stderr)
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
        WindowGroup("InfinityOS Installer Studio") {
            StudioRootView(store: store)
                .frame(minWidth: 1040, minHeight: 640)
                .preferredColorScheme(.dark)
        }
        .windowStyle(.hiddenTitleBar)
        .windowResizability(.contentMinSize)
        .defaultSize(width: 1480, height: 900)
        .commands {
            CommandGroup(replacing: .saveItem) {
                Button("Save Templates") { store.save() }
                    .keyboardShortcut("s", modifiers: .command)
                Button("Import Templates…") { store.importDocument() }
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
            CommandGroup(after: .pasteboard) {
                Button("Duplicate Element") { store.duplicateSelected() }
                    .keyboardShortcut("d", modifiers: .command)
                Button("Delete Element") { store.deleteSelected() }
                    .keyboardShortcut(.delete, modifiers: [])
            }
            CommandMenu("Canvas") {
                Button(store.marqueeSelectionEnabled ? "Disable Marquee Select" : "Enable Marquee Select") {
                    store.toggleMarqueeSelection()
                }
                .keyboardShortcut("m", modifiers: [.command, .shift])
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
                    .disabled(!store.canAddScreen)
                Button("Duplicate Screen") { store.duplicateScreen() }
                    .keyboardShortcut("d", modifiers: [.command, .shift])
                    .disabled(!store.canAddScreen)
            }
        }
    }
}
