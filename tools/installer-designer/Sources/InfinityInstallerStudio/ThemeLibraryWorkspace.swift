import AppKit
import SwiftUI
import UniformTypeIdentifiers

/// A reusable theme is a saved authoring document, not a destructive global reset.
struct ThemeLibraryWorkspace: View {
    @ObservedObject var store: UIKitStore
    @StateObject private var library: ThemeLibrary
    @State private var selectedID: UUID?
    @State private var name = "My Eclipse Theme"
    @State private var notice = "Create a reusable theme from your current UIKit draft."
    @State private var error: String?
    private let columns = [GridItem(.adaptive(minimum: 220, maximum: 360), spacing: 16)]
    private var selected: ThemeSummary? { library.themes.first { $0.id == selectedID } }

    // ------------------------=
    // FUNC: init
    // DESC: Keeps library selection separate from the current unsaved UIKit document.
    // ------------------=
    init(store: UIKitStore) {
        self.store = store
        _library = StateObject(wrappedValue: ThemeLibrary(projectRoot: store.projectRoot))
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Make InfinityOS your own").font(.title.weight(.semibold))
                    Text("Save palettes, materials and artwork as portable themes. Apply a preset, customize it, then publish your changes to an ISO.")
                        .font(.callout).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                }
                VStack(alignment: .leading, spacing: 12) {
                    Text("THEME NAME").font(.caption.weight(.semibold)).foregroundStyle(.secondary)
                    TextField("Name your theme", text: $name).textFieldStyle(.roundedBorder)
                        .accessibilityIdentifier("studio.theme.name")
                    ViewThatFits(in: .horizontal) {
                        HStack(spacing: 12) { saveActions; transferActions }
                        VStack(alignment: .leading, spacing: 12) {
                            HStack(spacing: 12) { saveActions }
                            HStack(spacing: 12) { transferActions }
                        }
                    }
                }
                .padding(20).background(InfinityUIKit.Palette.nativePanel)
                .clipShape(RoundedRectangle(cornerRadius: 12))
                if let error {
                    Label(error, systemImage: "exclamationmark.triangle")
                        .font(.callout).foregroundStyle(.orange).textSelection(.enabled)
                        .accessibilityIdentifier("studio.theme.error")
                } else {
                    Label(notice, systemImage: "info.circle")
                        .font(.callout).foregroundStyle(.secondary)
                        .accessibilityIdentifier("studio.theme.status")
                }
                if !library.issues.isEmpty {
                    VStack(alignment: .leading, spacing: 8) {
                        Label("Some saved themes could not be loaded", systemImage: "exclamationmark.triangle")
                        ForEach(library.issues, id: \.self) { issue in
                            Text(issue).font(.caption).textSelection(.enabled)
                        }
                    }.foregroundStyle(.orange)
                }
                LazyVGrid(columns: columns, alignment: .leading, spacing: 16) {
                    ForEach(library.themes) { theme in card(theme) }
                }
                if let selected {
                    VStack(alignment: .leading, spacing: 12) {
                        HStack {
                            Text(selected.name).font(.headline)
                            Spacer()
                            Text(selected.isBuiltIn ? "Built-in preset" : "Custom theme")
                                .font(.caption).foregroundStyle(.secondary)
                        }
                        ViewThatFits(in: .horizontal) {
                            HStack(spacing: 12) { selectedActions(selected) }
                            VStack(alignment: .leading, spacing: 12) { selectedActions(selected) }
                        }
                        Text("Applying is undoable. It does not overwrite your screen layouts or update a running OS.")
                            .font(.caption).foregroundStyle(.secondary)
                    }
                    .padding(20).background(InfinityUIKit.Palette.nativePanel)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
                }
            }.padding(24)
        }
        .buttonStyle(InfinityStudioButtonStyle(emphasis: .secondary))
        .onAppear { perform { try library.reload() } }
    }

    private var saveActions: some View {
        Group {
            Button("Save as New Theme") {
                perform {
                    let saved = try library.saveCurrent(name: name, store: store)
                    selectedID = saved.id
                    notice = "Saved \(saved.name). The current UIKit draft is unchanged."
                }
            }
            .buttonStyle(InfinityStudioButtonStyle(emphasis: .primary))
            .disabled(name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            .accessibilityIdentifier("studio.theme.create")
            Button("Update Selected") {
                guard let selected, !selected.isBuiltIn else { return }
                perform {
                    let saved = try library.saveCurrent(name: name, store: store, replacingID: selected.id)
                    notice = "Updated \(saved.name) from the current UIKit draft."
                }
            }.disabled(selected == nil || selected?.isBuiltIn == true)
        }
    }

    private var transferActions: some View {
        Group {
            Button("Import…", action: importTheme)
                .accessibilityIdentifier("studio.theme.import")
            Button("Export Current…", action: exportCurrent)
                .accessibilityIdentifier("studio.theme.exportCurrent")
        }
    }

    // ------------------------=
    // FUNC: selectedActions
    // DESC: Separates applying, renaming and exporting a preset from replacing its saved contents.
    // ------------------=
    @ViewBuilder private func selectedActions(_ theme: ThemeSummary) -> some View {
        Button("Apply Theme") {
            perform {
                try library.apply(id: theme.id, store: store)
                notice = "Applied \(theme.name) to the draft. Undo is available; save when ready."
            }
        }
        .buttonStyle(InfinityStudioButtonStyle(emphasis: .primary))
        .accessibilityIdentifier("studio.theme.apply")
        Button("Duplicate") {
            perform {
                let copy = try library.duplicate(id: theme.id, name: theme.name + " Copy")
                selectedID = copy.id; name = copy.name
                notice = "Created \(copy.name); the original theme is unchanged."
            }
        }
        Button("Rename") {
            perform {
                try library.rename(id: theme.id, name: name)
                notice = "Renamed the saved theme."
            }
        }.disabled(theme.isBuiltIn)
        Button("Export…") { export(theme) }
        Button("Move to Theme Trash", role: .destructive) {
            perform {
                try library.delete(id: theme.id)
                selectedID = nil
                notice = "Moved \(theme.name) to assets/ui-design-kit/themes/.trash; the active draft is unchanged."
            }
        }.disabled(theme.isBuiltIn)
    }

    // ------------------------=
    // FUNC: card
    // DESC: Presents exact shared surface artwork with the saved theme recipe, not an unrelated illustrative thumbnail.
    // ------------------=
    private func card(_ theme: ThemeSummary) -> some View {
        Button {
            selectedID = theme.id; name = theme.name; error = nil
        } label: {
            VStack(alignment: .leading, spacing: 12) {
                if let atlas = library.previewAtlas(id: theme.id,
                    target: "assets/ui-design-kit/default/eclipse-precision-v1.rgba", store: store) {
                    ThemeSurfaceSample(document: theme.previewDocument, root: store.projectRoot, atlasData: atlas)
                        .frame(height: 132).clipped()
                } else {
                    Label("Theme artwork unavailable", systemImage: "exclamationmark.triangle")
                        .foregroundStyle(.orange).frame(height: 132)
                }
                HStack {
                    Text(theme.name).font(.headline).lineLimit(2)
                    Spacer()
                    if selectedID == theme.id {
                        Image(systemName: "checkmark.circle.fill").foregroundStyle(InfinityUIKit.Palette.nativeAccent)
                    }
                }
                Text(theme.isBuiltIn ? "PRESET" : "CUSTOM")
                    .font(.caption2.weight(.semibold)).foregroundStyle(.secondary)
            }
            .padding(16).frame(maxWidth: .infinity, alignment: .leading)
            .background(InfinityUIKit.Palette.nativePanel)
            .clipShape(RoundedRectangle(cornerRadius: 12))
            .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(
                selectedID == theme.id ? InfinityUIKit.Palette.nativeAccent : InfinityUIKit.Palette.nativeBorder.opacity(0.25),
                lineWidth: selectedID == theme.id ? 2 : 1))
        }.buttonStyle(.plain).accessibilityIdentifier("studio.theme.\(theme.id.uuidString)")
    }

    // ------------------------=
    // FUNC: perform
    // DESC: Keeps errors inline and preserves draft edits when validation or filesystem operations fail.
    // ------------------=
    private func perform(_ operation: () throws -> Void) {
        do { try operation(); error = nil }
        catch { self.error = error.localizedDescription }
    }

    // ------------------------=
    // FUNC: importTheme
    // DESC: Reads a portable validated bundle without applying it implicitly over the active draft.
    // ------------------=
    private func importTheme() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [UTType(filenameExtension: "infinitytheme") ?? .json]
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let source = panel.url else { return }
        perform {
            let theme = try library.importTheme(from: source)
            selectedID = theme.id; name = theme.name
            notice = "Imported \(theme.name). Choose Apply Theme to change the current draft."
        }
    }

    // ------------------------=
    // FUNC: exportDestination
    // DESC: Defaults portable theme exports to the repository-managed export folder.
    // ------------------=
    private func exportDestination(_ title: String) -> URL? {
        guard let root = store.projectRoot else { return nil }
        let directory = root.appending(path: "builds/exports/themes", directoryHint: .isDirectory)
        guard directory.resolvingSymlinksInPath().path.hasPrefix(root.resolvingSymlinksInPath().path + "/") else {
            error = "The theme export folder must remain inside this project."
            return nil
        }
        do { try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true) }
        catch { self.error = error.localizedDescription; return nil }
        let panel = NSSavePanel()
        panel.allowedContentTypes = [UTType(filenameExtension: "infinitytheme") ?? .json]
        panel.nameFieldStringValue = title + ".infinitytheme"
        panel.directoryURL = directory
        panel.canCreateDirectories = true
        return panel.runModal() == .OK ? panel.url : nil
    }

    // ------------------------=
    // FUNC: exportCurrent
    // DESC: Exports unsaved palette, styles and artwork without forcing a project save.
    // ------------------=
    private func exportCurrent() {
        guard let target = exportDestination(name) else { return }
        perform {
            try library.exportCurrent(name: name, store: store, to: target)
            notice = "Exported the current draft to \(target.lastPathComponent)."
        }
    }

    // ------------------------=
    // FUNC: export
    // DESC: Exports a selected saved theme independently of current draft changes.
    // ------------------=
    private func export(_ theme: ThemeSummary) {
        guard let target = exportDestination(theme.name) else { return }
        perform {
            try library.export(id: theme.id, to: target)
            notice = "Exported \(theme.name) to \(target.lastPathComponent)."
        }
    }
}

private struct ThemeSurfaceSample: View {
    let document: UIKitDocument
    let root: URL?
    let atlasData: Data?

    var body: some View {
        ZStack {
            studioColor(document.colors[0]).color
            PrecisionSurface(kind: .panel, material: material(.panel), projectRoot: root, atlasData: atlasData)
            VStack(spacing: 12) {
                HStack(spacing: 6) {
                    ForEach([3, 6, 7, 8, 9], id: \.self) { role in
                        Circle().fill(studioColor(document.colors[role]).color).frame(width: 12, height: 12)
                    }
                    Spacer()
                }
                PrecisionSurface(kind: .field, material: material(.field), projectRoot: root, atlasData: atlasData).frame(height: 26)
                HStack(spacing: 10) {
                    PrecisionSurface(kind: .secondary, material: material(.secondary), projectRoot: root, atlasData: atlasData)
                    PrecisionSurface(kind: .primary, material: material(.primary), projectRoot: root, atlasData: atlasData)
                }.frame(height: 28)
            }.padding(18)
        }
        .accessibilityLabel("Theme palette and surface samples")
    }

    // ------------------------=
    // FUNC: material
    // DESC: Resolves the saved theme's independent native surface recipe and semantic tint.
    // ------------------=
    private func material(_ kind: PrecisionSurfaceKind) -> PrecisionMaterial {
        PrecisionMaterial(fill: studioColor(document.colors[kind == .primary ? 6 : 1]),
            edge: studioColor(document.colors[3]), style: .resolved(for: kind, styles: document.styles))
    }
}
