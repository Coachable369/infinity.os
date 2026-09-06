import SwiftUI

struct StudioRootView: View {
    @ObservedObject var store: TemplateStore

    var body: some View {
        VStack(spacing: 0) {
            StudioToolbar(store: store)
            Divider()
            HSplitView {
                ScreenSidebar(store: store)
                    .frame(minWidth: 220, idealWidth: 248, maxWidth: 290)
                InstallerCanvas(store: store)
                    .frame(minWidth: 560)
                ElementInspector(store: store)
                    .frame(minWidth: 280, idealWidth: 310, maxWidth: 360)
            }
            Divider()
            StudioStatusBar(store: store)
        }
        .background(Color(nsColor: .windowBackgroundColor))
        .alert("Template Validation", isPresented: .constant(!store.validationIssues.isEmpty)) {
            Button("OK") { store.validationIssues = [] }
        } message: {
            Text(store.validationIssues.joined(separator: "\n"))
        }
    }
}

private struct StudioToolbar: View {
    @ObservedObject var store: TemplateStore

    var body: some View {
        HStack(spacing: 10) {
            if let image = studioIcon() {
                Image(nsImage: image)
                    .resizable()
                    .frame(width: 30, height: 30)
                    .clipShape(RoundedRectangle(cornerRadius: 7))
            } else {
                Image(systemName: "infinity")
                    .font(.title2.weight(.semibold))
                    .foregroundStyle(.cyan)
            }
            VStack(alignment: .leading, spacing: 0) {
                Text("Installer Studio").font(.headline)
                Text("InfinityOS UI Templates").font(.caption).foregroundStyle(.secondary)
            }
            Divider().frame(height: 28).padding(.horizontal, 4)
            Button(action: store.undo) { Label("Undo", systemImage: "arrow.uturn.backward") }
                .disabled(!store.canUndo)
            Button(action: store.redo) { Label("Redo", systemImage: "arrow.uturn.forward") }
                .disabled(!store.canRedo)
            Divider().frame(height: 28)
            Menu {
                Button("Panel") { store.addElement(kind: .panel) }
                Button("Text") { store.addElement(kind: .text) }
                Button("Image") { store.addElement(kind: .image) }
                Button("Console") { store.addElement(kind: .console) }
            } label: {
                Label("Add", systemImage: "plus")
            }
            Toggle(isOn: $store.showGrid) { Label("Grid", systemImage: "grid") }
                .toggleStyle(.button)
            Toggle(isOn: $store.snapEnabled) { Label("Snap", systemImage: "magnet") }
                .toggleStyle(.button)
            Picker("Grid", selection: $store.gridSize) {
                Text("5").tag(5)
                Text("10").tag(10)
                Text("20").tag(20)
                Text("50").tag(50)
            }
            .frame(width: 86)
            Spacer()
            Button(action: { _ = store.validate() }) {
                Label("Validate", systemImage: "checkmark.shield")
            }
            Button(action: store.save) {
                Label("Save", systemImage: "square.and.arrow.down")
            }
            .buttonStyle(.borderedProminent)
            .tint(.cyan)
        }
        .controlSize(.small)
        .padding(.horizontal, 14)
        .frame(height: 58)
        .background(.ultraThinMaterial)
    }

    // ------------------------=
    // FUNC: studioIcon
    // DESC: Loads the generated Installer Studio identity from the packaged resource bundle.
    // ------------------=
    private func studioIcon() -> NSImage? {
        Bundle.module.url(forResource: "InstallerStudioIcon-v1", withExtension: "png")
            .flatMap(NSImage.init(contentsOf:))
    }
}

private struct ScreenSidebar: View {
    @ObservedObject var store: TemplateStore

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("INSTALLATION SCREENS")
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
                .padding(.horizontal, 14)
                .padding(.vertical, 12)
            List(store.document.screens) { screen in
                Button(action: { store.selectScreen(screen.id) }) {
                    HStack(spacing: 10) {
                        Text(String(format: "%02d", screen.id))
                            .font(.system(.caption, design: .monospaced).weight(.bold))
                            .foregroundStyle(screen.id == store.selectedScreenID ? .black : .cyan)
                            .frame(width: 28, height: 28)
                            .background(screen.id == store.selectedScreenID ? Color.cyan : Color.cyan.opacity(0.12))
                            .clipShape(RoundedRectangle(cornerRadius: 7))
                        VStack(alignment: .leading, spacing: 2) {
                            Text(screen.title).font(.callout.weight(.medium))
                            Text("\(screen.elements.count) layers")
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                        }
                        Spacer()
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .listRowBackground(
                    screen.id == store.selectedScreenID ? Color.cyan.opacity(0.13) : Color.clear
                )
            }
            .listStyle(.sidebar)
            Divider()
            Button(role: .destructive, action: store.resetScreen) {
                Label("Reset Current Screen", systemImage: "arrow.counterclockwise")
            }
            .buttonStyle(.plain)
            .padding(14)
        }
        .background(Color(nsColor: .controlBackgroundColor).opacity(0.7))
    }
}

private struct StudioStatusBar: View {
    @ObservedObject var store: TemplateStore

    var body: some View {
        HStack(spacing: 14) {
            Circle()
                .fill(store.validationIssues.isEmpty ? Color.green : Color.orange)
                .frame(width: 7, height: 7)
            Text(store.status).lineLimit(1)
            if let element = store.selectedElement {
                Divider().frame(height: 14)
                Text("\(element.frame.x), \(element.frame.y)   \(element.frame.width) × \(element.frame.height)")
                    .font(.system(.caption, design: .monospaced))
            }
            Spacer()
            Text(store.snapEnabled ? "Snap \(store.gridSize)" : "Free positioning")
            Slider(value: $store.zoom, in: 0.35...1.25, step: 0.05)
                .frame(width: 130)
            Text("\(Int(store.zoom * 100))%").monospacedDigit().frame(width: 40)
        }
        .font(.caption)
        .foregroundStyle(.secondary)
        .padding(.horizontal, 14)
        .frame(height: 30)
        .background(.ultraThinMaterial)
    }
}
