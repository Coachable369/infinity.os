import SwiftUI

struct InlineElementEditor: View {
    @ObservedObject var store: TemplateStore

    private var element: StudioElement? { store.selectedElement }

    var body: some View {
        VStack(alignment: .leading, spacing: InfinityUIKit.Metrics.controlGap) {
            if let element {
                HStack(spacing: 8) {
                    Image(systemName: icon(for: element.kind))
                        .foregroundStyle(InfinityUIKit.Palette.nativeAccent)
                    VStack(alignment: .leading, spacing: 1) {
                        Text(element.kind.title).font(.headline)
                        Text("Inline editor").font(.caption).foregroundStyle(.secondary)
                    }
                    Spacer()
                    Button(action: { store.toggleElementLock(element.id) }) {
                        Label("Lock", systemImage: "lock.fill")
                    }
                    .buttonStyle(InfinityStudioButtonStyle(emphasis: .secondary))
                    .controlSize(.regular)
                }

                TextField("Layer name", text: stringBinding(\.name))

                if element.kind == .text || element.kind == .button {
                    TextEditor(text: stringBinding(\.text))
                        .font(.body)
                        .scrollContentBackground(.hidden)
                        .frame(height: 76)
                        .padding(5)
                        .background(
                            InfinityUIKit.Palette.nativePanelRaised,
                            in: RoundedRectangle(cornerRadius: InfinityUIKit.Metrics.controlRadius)
                        )
                } else if element.kind == .image {
                    HStack {
                        Text(element.imageAsset).font(.caption.monospaced()).lineLimit(1)
                        Spacer()
                        Button("Replace…", action: store.chooseReplacementImage)
                    }
                    Grid(alignment: .leading, horizontalSpacing: 7, verticalSpacing: 6) {
                        GridRow {
                            Text("Crop L")
                            TextField("Left", value: cropBinding(\.left), format: .number)
                            Text("T")
                            TextField("Top", value: cropBinding(\.top), format: .number)
                        }
                        GridRow {
                            Text("Crop R")
                            TextField("Right", value: cropBinding(\.right), format: .number)
                            Text("B")
                            TextField("Bottom", value: cropBinding(\.bottom), format: .number)
                        }
                    }
                    .textFieldStyle(.roundedBorder)
                }

                Grid(alignment: .leading, horizontalSpacing: 8, verticalSpacing: 7) {
                    GridRow {
                        Text("X")
                        TextField("X", value: intBinding(\.frame.x, fallback: element.frame.x), format: .number)
                        Text("Y")
                        TextField("Y", value: intBinding(\.frame.y, fallback: element.frame.y), format: .number)
                    }
                    GridRow {
                        Text("W")
                        TextField("W", value: intBinding(\.frame.width, fallback: element.frame.width), format: .number)
                        Text("H")
                        TextField("H", value: intBinding(\.frame.height, fallback: element.frame.height), format: .number)
                    }
                }
                .textFieldStyle(.roundedBorder)

                HStack {
                    ColorPicker("Fill", selection: colorBinding(\.fill, fallback: element.fill), supportsOpacity: true)
                    ColorPicker("Border", selection: colorBinding(\.border, fallback: element.border), supportsOpacity: true)
                }
                HStack {
                    Text("Opacity")
                    Slider(value: doubleBinding(\.opacity, fallback: element.opacity), in: 10...100, step: 1)
                    Text(String(element.opacity) + "%")
                        .monospacedDigit()
                        .frame(width: 38)
                }
                if element.kind == .text || element.kind == .button {
                    HStack {
                        Text("Text size")
                        Slider(value: doubleBinding(\.fontSize, fallback: element.fontSize), in: 8...72, step: 1)
                        Text(String(element.fontSize))
                            .monospacedDigit()
                            .frame(width: 28)
                    }
                }

                HStack {
                    Text("Drag the canvas object or use exact values here.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Spacer()
                    Button("Done", action: store.dismissInlineEditor)
                        .buttonStyle(InfinityStudioButtonStyle(emphasis: .primary))
                        .keyboardShortcut(.defaultAction)
                }
            }
        }
        .padding(InfinityUIKit.Metrics.gutter)
        .frame(width: 340)
        .background(
            InfinityUIKit.Palette.nativePanel,
            in: RoundedRectangle(cornerRadius: InfinityUIKit.Metrics.panelRadius)
        )
        .overlay {
            RoundedRectangle(cornerRadius: InfinityUIKit.Metrics.panelRadius)
                .stroke(InfinityUIKit.Palette.nativeAccent.opacity(0.42), lineWidth: 1)
        }
        .shadow(color: .black.opacity(0.5), radius: 22, y: 8)
    }

    // ------------------------=
    // FUNC: icon
    // DESC: Returns the semantic symbol for the inline editor's selected element.
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
    // DESC: Creates a live string binding into the selected unlocked element.
    // ------------------=
    private func stringBinding(_ keyPath: WritableKeyPath<StudioElement, String>) -> Binding<String> {
        Binding(
            get: { store.selectedElement?[keyPath: keyPath] ?? "" },
            set: { value in store.updateSelected("Inline edit") { $0[keyPath: keyPath] = value } }
        )
    }

    // ------------------------=
    // FUNC: intBinding
    // DESC: Creates a live normalized geometry binding into the selected unlocked element.
    // ------------------=
    private func intBinding(
        _ keyPath: WritableKeyPath<StudioElement, Int>,
        fallback: Int
    ) -> Binding<Int> {
        Binding(
            get: { store.selectedElement?[keyPath: keyPath] ?? fallback },
            set: { value in store.updateSelected("Inline geometry") { $0[keyPath: keyPath] = value } }
        )
    }

    // ------------------------=
    // FUNC: doubleBinding
    // DESC: Creates a live slider binding for an integer element property.
    // ------------------=
    private func doubleBinding(
        _ keyPath: WritableKeyPath<StudioElement, Int>,
        fallback: Int
    ) -> Binding<Double> {
        Binding(
            get: { Double(store.selectedElement?[keyPath: keyPath] ?? fallback) },
            set: { value in
                store.updateSelected("Inline appearance") { $0[keyPath: keyPath] = Int(value.rounded()) }
            }
        )
    }

    // ------------------------=
    // FUNC: colorBinding
    // DESC: Creates a live color binding into the selected unlocked element.
    // ------------------=
    private func colorBinding(
        _ keyPath: WritableKeyPath<StudioElement, StudioColor>,
        fallback: StudioColor
    ) -> Binding<Color> {
        Binding(
            get: { (store.selectedElement?[keyPath: keyPath] ?? fallback).color },
            set: { value in
                store.updateSelected("Inline appearance") { $0[keyPath: keyPath] = StudioColor(value) }
            }
        )
    }

    // ------------------------=
    // FUNC: cropBinding
    // DESC: Creates a live percentage binding to one normalized image crop edge.
    // ------------------=
    private func cropBinding(_ keyPath: WritableKeyPath<ImageCrop, Int>) -> Binding<Int> {
        Binding(
            get: { store.selectedElement?.crop[keyPath: keyPath] ?? 0 },
            set: { value in
                store.updateSelected("Inline crop") { $0.crop[keyPath: keyPath] = value }
            }
        )
    }
}
