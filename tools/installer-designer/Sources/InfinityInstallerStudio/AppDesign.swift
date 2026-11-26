import Foundation
import SwiftUI

/// Application identity remains stable when its user-visible title changes.
struct AppDesign: Codable, Equatable, Identifiable {
    var id: String
    var title: String
    var enabled = false
    var menuAlignment = "leading"
    var contentInset = 16
    var inspectorSide = "right"
    var inspectorWidth = 260
    static let catalog: [(String, String)] = [
        ("files", "File Navigator"), ("editor", "Text Editor"),
        ("terminal", "Command Window"), ("tasks", "Task Manager"),
        ("settings", "System Settings"), ("browser", "Infinity Browser"),
        ("code", "InfinityCode"), ("memory", "Infinity Memory"),
        ("fabric", "Infinity Fabric"), ("media", "Media Player"),
        ("world", "World Shift"), ("holographic", "Holographic Desktop")]
    var supportsTitle: Bool { !["browser", "world", "holographic"].contains(id) }
    var supportsWorkspace: Bool { ["code", "memory", "fabric", "media"].contains(id) }
    static var defaults: [AppDesign] { catalog.map { AppDesign(id: $0.0, title: $0.1) } }
}

@MainActor final class AppDesignStore: ObservableObject {
    @Published var designs = AppDesign.defaults
    @Published var selectedID = "code"
    @Published var loadError: String?
    private var saved = AppDesign.defaults
    private var history: [[AppDesign]] = []
    private var future: [[AppDesign]] = []
    var isDirty: Bool { designs != saved }
    var canUndo: Bool { !history.isEmpty }
    var canRedo: Bool { !future.isEmpty }
    var selected: AppDesign { designs.first { $0.id == selectedID } ?? designs[0] }
    // ------------------------=
    // FUNC: init
    // DESC: Loads versioned application appearance without discarding malformed source files.
    // ------------------=
    init(root: URL?) {
        guard let root else { return }
        let url = root.appending(path: "runtime/install/System/app-design.json")
        guard FileManager.default.fileExists(atPath: url.path) else { return }
        do {
            let document = try JSONDecoder().decode(Document.self, from: Data(contentsOf: url))
            guard document.version == 1 else { throw UIKitIssue.invalid("Unsupported application design version") }
            designs = document.apps
            try validate()
            saved = designs
        } catch { designs = AppDesign.defaults; loadError = error.localizedDescription }
    }
    struct Document: Codable { var version = 1; var apps: [AppDesign] }
    // ------------------------=
    // FUNC: update
    // DESC: Records one appearance edit without changing application identity or commands.
    // ------------------=
    func update(_ edit: (inout AppDesign) -> Void) {
        guard let index = designs.firstIndex(where: { $0.id == selectedID }) else { return }
        let old = designs
        edit(&designs[index])
        if old != designs { history.append(old); future.removeAll() }
    }
    // ------------------------=
    // FUNC: undo
    // DESC: Restores the prior application appearance edit.
    // ------------------=
    func undo() { guard let prior = history.popLast() else { return }; future.append(designs); designs = prior }
    // ------------------------=
    // FUNC: redo
    // DESC: Reapplies an undone application appearance edit.
    // ------------------=
    func redo() { guard let next = future.popLast() else { return }; history.append(designs); designs = next }
    // ------------------------=
    // FUNC: validate
    // DESC: Bounds layouts and titles and rejects duplicate or missing application identities.
    // ------------------=
    func validate() throws {
        if let loadError { throw UIKitIssue.invalid(loadError) }
        guard Set(designs.map(\.id)) == Set(AppDesign.catalog.map { $0.0 }), designs.count == AppDesign.catalog.count else {
            throw UIKitIssue.invalid("Application design catalog has missing or duplicate identities")
        }
        for app in designs {
            guard !app.title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
                app.title.utf8.count <= 96, !app.title.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) }),
                ["leading", "trailing"].contains(app.menuAlignment),
                ["left", "right"].contains(app.inspectorSide),
                (8...48).contains(app.contentInset), (200...400).contains(app.inspectorWidth) else {
                throw UIKitIssue.invalid("Invalid title or layout for \(app.id)")
            }
        }
    }
    // ------------------------=
    // FUNC: prepareSave
    // DESC: Produces the installed application design source as part of Studio's atomic save.
    // ------------------=
    func prepareSave(root: URL) throws -> [StudioFileWrite] {
        try validate()
        let encoder = JSONEncoder(); encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        var binary = Data()
        for entry in AppDesign.catalog {
            let app = designs.first { $0.id == entry.0 }!
            var record = [UInt8](repeating: 0, count: 128)
            let title = Array(app.title.utf8)
            record[0] = UInt8(title.count)
            record[6] = app.enabled ? 1 : 0
            record[1] = UInt8(app.contentInset)
            record[2] = app.menuAlignment == "trailing" ? 1 : 0
            record[3] = app.inspectorSide == "left" ? 1 : 0
            record[4] = UInt8(app.inspectorWidth & 255)
            record[5] = UInt8(app.inspectorWidth >> 8)
            record.replaceSubrange(16..<(16 + title.count), with: title)
            binary.append(contentsOf: record)
        }
        return [StudioFileWrite(url: root.appending(path: "runtime/install/System/app-design.json"),
            data: try encoder.encode(Document(apps: designs))),
            StudioFileWrite(url: root.appending(path: "assets/ui-design-kit/default/app-design.bin"), data: binary)]
    }
    // ------------------------=
    // FUNC: didSave
    // DESC: Marks only successfully published application designs as saved.
    // ------------------=
    func didSave() { saved = designs }
}

struct AppDesignWorkspace: View {
    @ObservedObject var store: AppDesignStore
    let openUIKit: () -> Void
    let openSettings: () -> Void
    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text(store.selected.title).font(.largeTitle.weight(.semibold))
            Text("Application design").foregroundStyle(.secondary)
            Form {
                Section("Window") {
                    Toggle("Use app-specific design", isOn: binding(\.enabled)).disabled(!store.selected.supportsTitle)
                    TextField("Title", text: binding(\.title)).disabled(!store.selected.supportsTitle)
                    if !store.selected.supportsTitle { Text("This app uses a specialized surface. Its app-specific adapter is not available yet.").font(.caption).foregroundStyle(.secondary) }
                    Button("Edit shared UIKit surfaces, buttons and window frame", action: openUIKit)
                    Text("The system compositor owns the frame and window controls. Title and menu heights stay fixed when resizing.")
                        .font(.caption).foregroundStyle(.secondary)
                }
                Section("Workspace layout") {
                    if store.selected.id == "settings" {
                        Button("Edit Settings pages on the design canvas", action: openSettings)
                    }
                    if store.selected.supportsWorkspace {
                    if store.selected.id != "media" { Picker("Menu placement", selection: binding(\.menuAlignment)) {
                        Text("Left aligned").tag("leading"); Text("Right aligned").tag("trailing")
                    } }
                    Stepper("Content inset: \(store.selected.contentInset) px", value: binding(\.contentInset), in: 8...48, step: 4)
                    if store.selected.id == "code" { Picker("Inspector placement", selection: binding(\.inspectorSide)) {
                        Text("Left").tag("left"); Text("Right").tag("right")
                    }
                    Stepper("Inspector width: \(store.selected.inspectorWidth) px", value: binding(\.inspectorWidth), in: 200...400, step: 10)
                    }
                    } else { Text("This native app currently supports title and shared UIKit styling. Per-app menu and content layout integration is pending.").foregroundStyle(.secondary) }
                }
            }.formStyle(.grouped).scrollContentBackground(.hidden)
        }.padding(24).frame(minWidth: 680, maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
    // ------------------------=
    // FUNC: binding
    // DESC: Routes property edits through the application's undo history.
    // ------------------=
    private func binding<T>(_ path: WritableKeyPath<AppDesign, T>) -> Binding<T> {
        Binding(get: { store.selected[keyPath: path] }, set: { value in store.update { $0[keyPath: path] = value } })
    }
}
