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
        if arguments.count == 3, arguments[1] == "--export-default" {
            do {
                let directory = URL(fileURLWithPath: arguments[2], isDirectory: true)
                try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
                let document = InstallerStudioDocument.factoryDefault()
                let encoder = JSONEncoder()
                encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
                try encoder.encode(document).write(
                    to: directory.appending(path: "installer-screens.infinityui"),
                    options: .atomic
                )
                try RuntimeTemplateCodec.encode(document).write(
                    to: directory.appending(path: "installer-screens.iuit"),
                    options: .atomic
                )
                print("Exported InfinityOS installer templates to \(directory.path)")
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
                .frame(minWidth: 1320, minHeight: 760)
                .preferredColorScheme(.dark)
        }
        .windowStyle(.hiddenTitleBar)
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
        }
    }
}
