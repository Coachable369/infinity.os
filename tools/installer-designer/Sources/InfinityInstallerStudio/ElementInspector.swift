import AppKit
import SwiftUI

struct ElementInspector: View {
    @ObservedObject var store: TemplateStore

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("LAYERS").font(.caption.weight(.semibold)).foregroundStyle(.secondary)
                Spacer()
                Button(action: { store.moveLayer(-1) }) { Image(systemName: "square.2.layers.3d.bottom.filled") }
                    .help("Send Backward")
                Button(action: { store.moveLayer(1) }) { Image(systemName: "square.2.layers.3d.top.filled") }
                    .help("Bring Forward")
            }
            .buttonStyle(.plain)
            .padding(12)
            List((store.selectedScreen?.elements ?? []).sorted { $0.zIndex > $1.zIndex }) { element in
                Button(action: { store.selectElement(element.id) }) {
                    HStack(spacing: 8) {
                        Image(systemName: icon(for: element.kind))
                            .foregroundStyle(element.locked ? .orange : .cyan)
                            .frame(width: 18)
                        VStack(alignment: .leading, spacing: 1) {
                            Text(element.name).lineLimit(1)
                            Text(element.role.title).font(.caption2).foregroundStyle(.secondary)
                        }
                        Spacer()
                        if element.hidden { Image(systemName: "eye.slash").foregroundStyle(.secondary) }
                        if element.locked { Image(systemName: "lock.fill").foregroundStyle(.orange) }
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .listRowBackground(element.id == store.selectedElementID ? Color.cyan.opacity(0.12) : Color.clear)
            }
            .listStyle(.inset)
            .frame(minHeight: 190, idealHeight: 240)
            Divider()
            if let element = store.selectedElement {
                ScrollView {
                    VStack(alignment: .leading, spacing: 14) {
                        inspectorHeader(element)
                        Divider()
                        geometrySection(element)
                        if element.kind == .text || element.kind == .button {
                            textSection(element)
                        }
                        if element.kind == .image {
                            imageSection(element)
                        }
                        appearanceSection(element)
                        actionSection(element)
                    }
                    .padding(14)
                }
            } else {
                ContentUnavailableView(
                    "No Selection",
                    systemImage: "square.dashed",
                    description: Text("Select a layer or an object on the canvas.")
                )
            }
        }
        .background(Color(nsColor: .controlBackgroundColor).opacity(0.72))
    }

    // ------------------------=
    // FUNC: inspectorHeader
    // DESC: Builds the selected element identity and lock-state header.
    // ------------------=
    private func inspectorHeader(_ element: StudioElement) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Label(element.kind.title, systemImage: icon(for: element.kind))
                    .font(.headline)
                Spacer()
                if element.locked {
                    Label("Protected", systemImage: "lock.fill")
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(.orange)
                }
            }
            TextField("Layer name", text: stringBinding(\.name))
                .disabled(element.locked)
            Picker("Role", selection: enumBinding(\.role, fallback: element.role)) {
                ForEach(StudioElementRole.allCases) { role in Text(role.title).tag(role) }
            }
            .disabled(element.locked)
        }
    }

    // ------------------------=
    // FUNC: geometrySection
    // DESC: Builds precise normalized position and size controls for an editable element.
    // ------------------=
    private func geometrySection(_ element: StudioElement) -> some View {
        GroupBox("Geometry — normalized") {
            Grid(alignment: .leading, horizontalSpacing: 8, verticalSpacing: 8) {
                GridRow {
                    Text("X")
                    TextField("X", value: intBinding(\.frame.x, fallback: element.frame.x), format: .number)
                    Text("Y")
                    TextField("Y", value: intBinding(\.frame.y, fallback: element.frame.y), format: .number)
                }
                GridRow {
                    Text("W")
                    TextField("Width", value: intBinding(\.frame.width, fallback: element.frame.width), format: .number)
                    Text("H")
                    TextField("Height", value: intBinding(\.frame.height, fallback: element.frame.height), format: .number)
                }
            }
            .textFieldStyle(.roundedBorder)
            .disabled(element.locked)
        }
    }

    // ------------------------=
    // FUNC: textSection
    // DESC: Builds multiline content and typography controls for text-capable elements.
    // ------------------=
    private func textSection(_ element: StudioElement) -> some View {
        GroupBox("Text") {
            VStack(alignment: .leading, spacing: 8) {
                TextEditor(text: stringBinding(\.text))
                    .font(.body)
                    .scrollContentBackground(.hidden)
                    .frame(minHeight: 92)
                    .padding(5)
                    .background(Color.black.opacity(0.28), in: RoundedRectangle(cornerRadius: 7))
                HStack {
                    Text("Size")
                    Slider(value: doubleBinding(\.fontSize, fallback: element.fontSize), in: 8...72, step: 1)
                    Text("\(element.fontSize)").monospacedDigit().frame(width: 28)
                }
            }
            .disabled(element.locked)
        }
    }

    // ------------------------=
    // FUNC: imageSection
    // DESC: Builds image-source inspection and repository import controls.
    // ------------------=
    private func imageSection(_ element: StudioElement) -> some View {
        GroupBox("Image") {
            VStack(alignment: .leading, spacing: 8) {
                Text(element.imageAsset.isEmpty ? "No image selected" : element.imageAsset)
                    .font(.caption.monospaced())
                    .lineLimit(2)
                Button("Choose Image…", action: chooseImage)
                    .disabled(element.locked)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    // ------------------------=
    // FUNC: appearanceSection
    // DESC: Builds native appearance controls for editable installer elements.
    // ------------------=
    private func appearanceSection(_ element: StudioElement) -> some View {
        GroupBox("Appearance") {
            VStack(spacing: 9) {
                ColorPicker("Fill / Text", selection: colorBinding(\.fill, fallback: element.fill), supportsOpacity: true)
                ColorPicker("Border", selection: colorBinding(\.border, fallback: element.border), supportsOpacity: true)
                HStack {
                    Text("Opacity")
                    Slider(value: doubleBinding(\.opacity, fallback: element.opacity), in: 10...100, step: 1)
                    Text("\(element.opacity)%").monospacedDigit().frame(width: 38)
                }
                HStack {
                    Text("Radius")
                    Slider(value: doubleBinding(\.cornerRadius, fallback: element.cornerRadius), in: 0...40, step: 1)
                    Text("\(element.cornerRadius)").monospacedDigit().frame(width: 28)
                }
                Toggle("Hidden", isOn: boolBinding(\.hidden, fallback: element.hidden))
            }
            .disabled(element.locked)
        }
    }

    // ------------------------=
    // FUNC: actionSection
    // DESC: Builds duplication and deletion actions while preserving protected controls.
    // ------------------=
    private func actionSection(_ element: StudioElement) -> some View {
        HStack {
            Button("Duplicate", action: store.duplicateSelected)
            Spacer()
            Button("Delete", role: .destructive, action: store.deleteSelected)
        }
        .disabled(element.locked)
    }

    // ------------------------=
    // FUNC: icon
    // DESC: Returns the semantic SF Symbol for one designer element kind.
    // ------------------=
    private func icon(for kind: StudioElementKind) -> String {
        switch kind {
        case .panel: "rectangle.inset.filled"
        case .image: "photo"
        case .text: "textformat"
        case .console: "terminal"
        case .button: "button.programmable"
        }
    }

    // ------------------------=
    // FUNC: stringBinding
    // DESC: Creates an undoable string binding into the selected element.
    // ------------------=
    private func stringBinding(_ keyPath: WritableKeyPath<StudioElement, String>) -> Binding<String> {
        Binding(
            get: { store.selectedElement?[keyPath: keyPath] ?? "" },
            set: { value in store.updateSelected { $0[keyPath: keyPath] = value } }
        )
    }

    // ------------------------=
    // FUNC: intBinding
    // DESC: Creates an undoable integer binding into the selected element.
    // ------------------=
    private func intBinding(
        _ keyPath: WritableKeyPath<StudioElement, Int>,
        fallback: Int
    ) -> Binding<Int> {
        Binding(
            get: { store.selectedElement?[keyPath: keyPath] ?? fallback },
            set: { value in store.updateSelected { $0[keyPath: keyPath] = value } }
        )
    }

    // ------------------------=
    // FUNC: doubleBinding
    // DESC: Creates an undoable slider binding from an integer element property.
    // ------------------=
    private func doubleBinding(
        _ keyPath: WritableKeyPath<StudioElement, Int>,
        fallback: Int
    ) -> Binding<Double> {
        Binding(
            get: { Double(store.selectedElement?[keyPath: keyPath] ?? fallback) },
            set: { value in store.updateSelected { $0[keyPath: keyPath] = Int(value.rounded()) } }
        )
    }

    // ------------------------=
    // FUNC: boolBinding
    // DESC: Creates an undoable Boolean binding into the selected element.
    // ------------------=
    private func boolBinding(
        _ keyPath: WritableKeyPath<StudioElement, Bool>,
        fallback: Bool
    ) -> Binding<Bool> {
        Binding(
            get: { store.selectedElement?[keyPath: keyPath] ?? fallback },
            set: { value in store.updateSelected { $0[keyPath: keyPath] = value } }
        )
    }

    // ------------------------=
    // FUNC: enumBinding
    // DESC: Creates an undoable role binding into the selected element.
    // ------------------=
    private func enumBinding(
        _ keyPath: WritableKeyPath<StudioElement, StudioElementRole>,
        fallback: StudioElementRole
    ) -> Binding<StudioElementRole> {
        Binding(
            get: { store.selectedElement?[keyPath: keyPath] ?? fallback },
            set: { value in store.updateSelected { $0[keyPath: keyPath] = value } }
        )
    }

    // ------------------------=
    // FUNC: colorBinding
    // DESC: Creates an undoable native ColorPicker binding into a runtime color token.
    // ------------------=
    private func colorBinding(
        _ keyPath: WritableKeyPath<StudioElement, StudioColor>,
        fallback: StudioColor
    ) -> Binding<Color> {
        Binding(
            get: { (store.selectedElement?[keyPath: keyPath] ?? fallback).color },
            set: { value in store.updateSelected { $0[keyPath: keyPath] = StudioColor(value) } }
        )
    }

    // ------------------------=
    // FUNC: chooseImage
    // DESC: Imports a selected image into the InfinityOS boot assets and assigns it to the active layer.
    // ------------------=
    private func chooseImage() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.png, .jpeg, .bmp, .tiff]
        guard panel.runModal() == .OK, let source = panel.url else { return }
        guard let root = store.projectRoot else {
            store.updateSelected("Image selected") { $0.imageAsset = source.path }
            return
        }
        let directory = root.appending(path: "assets/boot", directoryHint: .isDirectory)
        var destination = directory.appending(path: source.lastPathComponent)
        if FileManager.default.fileExists(atPath: destination.path) {
            destination = directory.appending(path: "\(UUID().uuidString.prefix(8))-\(source.lastPathComponent)")
        }
        do {
            try FileManager.default.copyItem(at: source, to: destination)
            store.updateSelected("Image imported") { $0.imageAsset = destination.lastPathComponent }
        } catch {
            store.validationIssues = [error.localizedDescription]
        }
    }
}
