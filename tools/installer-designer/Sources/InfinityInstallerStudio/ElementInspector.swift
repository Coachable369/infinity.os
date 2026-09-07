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
            .padding(InfinityUIKit.Metrics.controlGap)
            List((store.selectedScreen?.elements ?? []).sorted { $0.zIndex > $1.zIndex }) { element in
                HStack(spacing: 8) {
                    Button(action: { store.selectElement(element.id) }) {
                        HStack(spacing: 8) {
                            Image(systemName: icon(for: element.kind))
                                .foregroundStyle(element.locked ? .orange : InfinityUIKit.Palette.nativeAccent)
                                .frame(width: 18)
                            VStack(alignment: .leading, spacing: 1) {
                                Text(element.name).lineLimit(1)
                                Text(element.role.title).font(.caption2).foregroundStyle(.secondary)
                            }
                            Spacer()
                            if element.hidden { Image(systemName: "eye.slash").foregroundStyle(.secondary) }
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    Button(action: { store.toggleElementLock(element.id) }) {
                        Image(systemName: element.locked ? "lock.fill" : "lock.open")
                            .foregroundStyle(element.locked ? .orange : .secondary)
                            .frame(width: 18)
                    }
                    .buttonStyle(.borderless)
                    .help(element.locked ? "Unlock Element" : "Lock Element")
                }
                .listRowBackground(element.id == store.selectedElementID
                    ? InfinityUIKit.Palette.nativeAccent.opacity(0.12)
                    : Color.clear)
            }
            .listStyle(.inset)
            .frame(minHeight: 190, idealHeight: 240)
            Divider()
            if let element = store.selectedElement {
                ScrollView {
                    VStack(alignment: .leading, spacing: InfinityUIKit.Metrics.controlGap) {
                        inspectorHeader(element)
                        Divider()
                        geometrySection(element)
                        if element.role == .input {
                            variableSection(element)
                        }
                        if element.kind == .text || element.kind == .button {
                            textSection(element)
                        }
                        if element.kind == .image {
                            imageSection(element)
                        }
                        appearanceSection(element)
                        actionSection(element)
                    }
                    .padding(InfinityUIKit.Metrics.gutter)
                }
            } else {
                ContentUnavailableView(
                    "No Selection",
                    systemImage: "square.dashed",
                    description: Text("Select a layer or an object on the canvas.")
                )
            }
        }
        .background(InfinityUIKit.Palette.nativePanel)
        .tint(InfinityUIKit.Palette.nativeAccent)
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
                Button(action: { store.toggleElementLock(element.id) }) {
                    Label(
                        element.locked ? "Unlock" : "Lock",
                        systemImage: element.locked ? "lock.fill" : "lock.open"
                    )
                }
                .buttonStyle(InfinityStudioButtonStyle(emphasis: .secondary))
                .tint(element.locked ? .orange : InfinityUIKit.Palette.nativeAccent)
                .controlSize(.regular)
            }
            TextField("Layer name", text: stringBinding(\.name))
                .disabled(element.locked)
            Picker("Role", selection: enumBinding(\.role, fallback: element.role)) {
                ForEach(StudioElementRole.allCases) { role in Text(role.title).tag(role) }
            }
            .disabled(element.locked || element.kind == .button)
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
    // FUNC: variableSection
    // DESC: Offers the runtime variable registry applicable to the selected screen collection.
    // ------------------=
    private func variableSection(_ element: StudioElement) -> some View {
        GroupBox("Data Binding") {
            VStack(alignment: .leading, spacing: 8) {
                Picker("Variable", selection: inputVariableBinding(fallback: element.inputVariable)) {
                    ForEach(availableInputVariables) { variable in
                        Text(variable.title).tag(variable)
                    }
                }
                Text(element.inputVariable.rawValue.isEmpty
                    ? "Choose the value this field edits at runtime."
                    : element.inputVariable.rawValue)
                    .font(.caption.monospaced())
                    .foregroundStyle(.secondary)
            }
            .disabled(element.locked)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private var availableInputVariables: [StudioInputVariable] {
        store.selectedCollection == .configuration
            ? [.machineNodeName, .profileName, .displayName, .password]
            : [.none]
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
                    .background(
                        InfinityUIKit.Palette.nativePanelRaised,
                        in: RoundedRectangle(cornerRadius: InfinityUIKit.Metrics.controlRadius)
                    )
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
                HStack {
                    Button("Replace Image…", action: store.chooseReplacementImage)
                    Button("Reset Crop") {
                        store.updateSelected("Crop reset") { $0.crop = .none }
                    }
                    .disabled(element.crop == .none)
                }
                Divider()
                cropSlider("Left", keyPath: \.left, value: element.crop.left)
                cropSlider("Top", keyPath: \.top, value: element.crop.top)
                cropSlider("Right", keyPath: \.right, value: element.crop.right)
                cropSlider("Bottom", keyPath: \.bottom, value: element.crop.bottom)
            }
            .disabled(element.locked)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    // ------------------------=
    // FUNC: cropSlider
    // DESC: Builds one precise normalized image crop-edge control.
    // ------------------=
    private func cropSlider(
        _ label: String,
        keyPath: WritableKeyPath<ImageCrop, Int>,
        value: Int
    ) -> some View {
        HStack {
            Text(label).frame(width: 48, alignment: .leading)
            Slider(value: cropBinding(keyPath), in: 0...90, step: 1)
            Text("\(value)%").monospacedDigit().frame(width: 34)
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
        .buttonStyle(InfinityStudioButtonStyle(emphasis: .secondary))
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
            set: { value in
                store.updateSelected {
                    $0[keyPath: keyPath] = value
                    if value != .input { $0.inputVariable = .none }
                }
            }
        )
    }

    // ------------------------=
    // FUNC: inputVariableBinding
    // DESC: Creates an undoable binding between an input layer and its runtime variable identifier.
    // ------------------=
    private func inputVariableBinding(fallback: StudioInputVariable) -> Binding<StudioInputVariable> {
        Binding(
            get: { store.selectedElement?.inputVariable ?? fallback },
            set: { value in store.updateSelected("Variable binding changed") { $0.inputVariable = value } }
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
    // FUNC: cropBinding
    // DESC: Creates an undoable binding to one normalized image crop edge.
    // ------------------=
    private func cropBinding(_ keyPath: WritableKeyPath<ImageCrop, Int>) -> Binding<Double> {
        Binding(
            get: { Double(store.selectedElement?.crop[keyPath: keyPath] ?? 0) },
            set: { value in
                store.updateSelected("Image cropped") {
                    $0.crop[keyPath: keyPath] = Int(value.rounded())
                }
            }
        )
    }
}
