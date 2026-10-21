import AppKit
import SwiftUI

struct UIKitStyleInspector: View {
    @ObservedObject var store: UIKitStore
    static let titles = ["Window Border", "Window Background", "Panels & Dialogs", "Primary Buttons", "Secondary Buttons", "Input Fields"]
    private var style: UIKitSurfaceStyle { store.document.styles[store.selectedSurface] }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("SURFACE RECIPE").font(.caption.weight(.semibold)).foregroundStyle(.secondary)
            Picker("Component", selection: $store.selectedSurface) {
                ForEach(0..<Self.titles.count, id: \.self) { index in
                    Text(Self.titles[index]).tag(index)
                }
            }
            Picker("Material", selection: binding(\.material)) {
                Text("Approved artwork").tag(UIKitSurfaceMaterial.authored)
                Text("Glass").tag(UIKitSurfaceMaterial.glass)
                Text("Matte").tag(UIKitSurfaceMaterial.matte)
            }.disabled(store.selectedSurface == 0)
            Picker("Corners", selection: binding(\.corners)) {
                Text("Approved artwork").tag(UIKitCornerStyle.authored)
                Text("Square").tag(UIKitCornerStyle.square)
                if store.selectedSurface >= 2 { Text("Rounded").tag(UIKitCornerStyle.rounded) }
            }
            if store.selectedSurface < 2 {
                Text("Outer application frames remain rectangular and OS-owned. Dialogs, panels and controls support rounded corners independently.")
                    .font(.caption).foregroundStyle(.secondary)
            }
            if style.corners == .rounded {
                HStack {
                    Text("Radius")
                    Slider(value: Binding(get: { Double(style.radius) }, set: { value in
                        var next = style; next.radius = Int(value.rounded())
                        store.setSurfaceStyle(store.selectedSurface, style: next)
                    }), in: 0...40, step: 1)
                    Text("\(style.radius) px").font(.caption.monospacedDigit()).frame(width: 44)
                }
            }
            Divider()
            Toggle("Custom fill", isOn: overrideBinding(\.fill, fallback: defaultFill))
                .disabled(store.selectedSurface == 0)
            if style.fill != nil {
                ColorPicker("Fill color", selection: colorBinding(\.fill, fallback: defaultFill), supportsOpacity: true)
                    .disabled(store.selectedSurface == 0)
            }
            Toggle("Custom border", isOn: overrideBinding(\.border, fallback: store.document.colors[3]))
            if style.border != nil {
                ColorPicker("Border / rim color", selection: colorBinding(\.border, fallback: store.document.colors[3]), supportsOpacity: true)
            }
            Text("Each component has its own style. Glass retains the approved reflections; matte removes reflective shading. Rounded and square edges remain antialiased.")
                .font(.caption).foregroundStyle(.secondary)
            Text("Screen elements can override material and corners in Appearance. Their explicit fill and border colors remain independent of this shared recipe.")
                .font(.caption).foregroundStyle(.secondary)
            Button("Reset This Style") { store.resetSurfaceStyle(store.selectedSurface) }
                .buttonStyle(InfinityStudioButtonStyle(emphasis: .secondary))
        }
    }

    private var defaultFill: UInt32 { store.document.colors[store.selectedSurface == 3 ? 6 : 1] }

    // ------------------------=
    // FUNC: binding
    // DESC: Applies a single surface property as an undoable UIKit document edit.
    // ------------------=
    private func binding<T>(_ keyPath: WritableKeyPath<UIKitSurfaceStyle, T>) -> Binding<T> {
        Binding(get: { style[keyPath: keyPath] }, set: { value in
            var next = style; next[keyPath: keyPath] = value
            store.setSurfaceStyle(store.selectedSurface, style: next)
        })
    }

    // ------------------------=
    // FUNC: overrideBinding
    // DESC: Distinguishes inherited colors from explicitly selected per-component colors, including transparent overrides.
    // ------------------=
    private func overrideBinding(_ keyPath: WritableKeyPath<UIKitSurfaceStyle, UInt32?>, fallback: UInt32) -> Binding<Bool> {
        Binding(get: { style[keyPath: keyPath] != nil }, set: { enabled in
            var next = style; next[keyPath: keyPath] = enabled ? (next[keyPath: keyPath] ?? fallback) : nil
            store.setSurfaceStyle(store.selectedSurface, style: next)
        })
    }

    // ------------------------=
    // FUNC: colorBinding
    // DESC: Preserves sRGB and alpha when editing an optional native ARGB surface tint.
    // ------------------=
    private func colorBinding(_ keyPath: WritableKeyPath<UIKitSurfaceStyle, UInt32?>, fallback: UInt32) -> Binding<Color> {
        Binding(get: { studioColor(style[keyPath: keyPath] ?? fallback).color }, set: { value in
            guard let color = NSColor(value).usingColorSpace(.sRGB) else { return }
            let byte: (CGFloat) -> UInt32 = { UInt32(($0 * 255).rounded().clamped(to: 0...255)) }
            var next = style
            next[keyPath: keyPath] = byte(color.alphaComponent) << 24 | byte(color.redComponent) << 16 |
                byte(color.greenComponent) << 8 | byte(color.blueComponent)
            store.setSurfaceStyle(store.selectedSurface, style: next)
        })
    }
}
