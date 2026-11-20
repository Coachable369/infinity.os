import AppKit
import SwiftUI

/// The editor previews the native compositor recipe, including unsaved artwork and style changes.
struct UIKitStylesPreview: View {
    @ObservedObject var store: UIKitStore
    private let kinds: [PrecisionSurfaceKind] = [.frame, .windowFill, .panel, .primary, .secondary, .field]
    private let columns = [GridItem(.adaptive(minimum: 240, maximum: 420), spacing: 20)]

    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            VStack(alignment: .leading, spacing: 12) {
                Text("Shared button styles").font(.title2.weight(.semibold))
                Text("Choose a theme for primary and secondary buttons together. Individual screen appearance overrides are preserved.")
                    .foregroundStyle(.secondary)
                LazyVGrid(columns: columns, spacing: 20) {
                    ForEach(GlassButtonPreset.allCases) { preset in
                        Button { store.applyButtonPreset(preset) } label: {
                            VStack(alignment: .leading, spacing: 12) {
                                sample(.primary, width: 250, height: 52, style: preset.style)
                                    .frame(height: 52)
                                Text(preset.rawValue).font(.headline)
                                Text(preset.detail).font(.caption).foregroundStyle(.secondary)
                                Text("Apply to shared buttons").font(.caption).foregroundStyle(.secondary)
                            }.padding(16).background(InfinityUIKit.Palette.nativePanel)
                                .clipShape(RoundedRectangle(cornerRadius: 12))
                        }.buttonStyle(.plain)
                    }
                }
            }
            VStack(alignment: .leading, spacing: 8) {
                Text("A material for every surface")
                    .font(.title.weight(.semibold))
                Text("Select a component to edit its material, corner geometry and independent fill or rim colors. These previews use the same pixels and state transitions as InfinityOS.")
                    .font(.body).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            }
            LazyVGrid(columns: columns, spacing: 20) {
                ForEach(Array(kinds.enumerated()), id: \.offset) { index, kind in
                    Button { store.selectedSurface = index } label: {
                        VStack(alignment: .leading, spacing: 12) {
                            sample(kind, width: 250, height: 124)
                                .frame(height: 124).clipped()
                            HStack {
                                Text(UIKitStyleInspector.titles[index]).font(.headline)
                                Spacer()
                                if store.selectedSurface == index {
                                    Image(systemName: "checkmark.circle.fill")
                                        .foregroundStyle(InfinityUIKit.Palette.nativeAccent)
                                }
                            }
                            Text(summary(index)).font(.caption).foregroundStyle(.secondary)
                        }
                        .padding(16)
                        .background(InfinityUIKit.Palette.nativePanel)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                        .overlay {
                            RoundedRectangle(cornerRadius: 12).strokeBorder(
                                store.selectedSurface == index ? InfinityUIKit.Palette.nativeAccent : InfinityUIKit.Palette.nativeBorder.opacity(0.25),
                                lineWidth: store.selectedSurface == index ? 2 : 1)
                        }
                    }.buttonStyle(.plain)
                        .accessibilityIdentifier("studio.uikit.surface.\(index)")
                }
            }
            VStack(alignment: .leading, spacing: 16) {
                Text("\(UIKitStyleInspector.titles[store.selectedSurface]) · interaction states")
                    .font(.headline)
                HStack(spacing: 16) {
                    state("Normal", focus: 0, pressed: false)
                    state("Focused", focus: 255, pressed: false)
                    state("Pressed", focus: 255, pressed: true)
                }
                Text("Hover transitions retain the approved 140 ms easing. Reduced Motion keeps immediate, stable states.")
                    .font(.caption).foregroundStyle(.secondary)
            }.padding(20).background(InfinityUIKit.Palette.nativePanel)
                .clipShape(RoundedRectangle(cornerRadius: 12))
        }.foregroundStyle(.white)
    }

    // ------------------------=
    // FUNC: material
    // DESC: Resolves each native component's own saved material and semantic tint without changing another family's appearance.
    // ------------------=
    private func material(_ kind: PrecisionSurfaceKind, focus: UInt8 = 0, pressed: Bool = false) -> PrecisionMaterial {
        PrecisionMaterial(fill: studioColor(store.document.colors[kind == .primary ? 6 : 1]),
            edge: studioColor(store.document.colors[3]), focus: focus, pressed: pressed,
            style: .resolved(for: kind, styles: store.document.styles))
    }

    // ------------------------=
    // FUNC: summary
    // DESC: Describes actual persisted choices instead of presenting a cosmetic preset that is disconnected from runtime data.
    // ------------------=
    private func summary(_ index: Int) -> String {
        let style = store.document.styles[index]
        let corners = style.corners == .rounded ? "Rounded · \(style.radius) px" : style.corners.title
        return "\(style.material.title) · \(corners)"
    }

    // ------------------------=
    // FUNC: state
    // DESC: Shows the three native interaction samples for the selected family with exact independent state artwork.
    // ------------------=
    private func state(_ title: String, focus: UInt8, pressed: Bool) -> some View {
        VStack(spacing: 10) {
            sample(kinds[store.selectedSurface], width: 180, height: 72, focus: focus, pressed: pressed)
                .frame(height: 72).clipped()
            Text(title).font(.caption).foregroundStyle(.secondary)
        }.frame(maxWidth: .infinity)
    }

    // ------------------------=
    // FUNC: sample
    // DESC: Renders the current draft atlas over real OS wallpaper, using native frost for panel glass and image-backed surface sampling otherwise.
    // ------------------=
    @ViewBuilder private func sample(_ kind: PrecisionSurfaceKind, width: Int, height: Int,
                                    focus: UInt8 = 0, pressed: Bool = false, style: UIKitSurfaceStyle? = nil) -> some View {
        let material = previewMaterial(kind, focus: focus, pressed: pressed, style: style)
        if let image = UIKitStylePreviewImage.image(store: store, kind: kind, width: width, height: height, material: material) {
            Image(decorative: image, scale: 1).resizable().interpolation(.high).aspectRatio(contentMode: .fit)
                .overlay {
                    if kind == .primary || kind == .secondary {
                        Text(kind == .primary ? "Continue" : "Cancel")
                            .font(InfinityUIKit.EclipseAccess.font(size: 16, semibold: true, projectRoot: store.projectRoot))
                            .foregroundStyle(material.style.material == .porcelain ? Color(red: 0.094, green: 0.133, blue: 0.196) : .white)
                    } else if kind == .field {
                        Text("Editable input").font(InfinityUIKit.EclipseAccess.font(size: 15, projectRoot: store.projectRoot))
                            .foregroundStyle(studioColor(store.document.colors[5]).color)
                    }
                }
        } else {
            Text("Native preview artwork unavailable").font(.caption).foregroundStyle(.orange)
        }
    }

    // ------------------------=
    // FUNC: previewMaterial
    // DESC: Renders prospective button presets with the same pixel recipe as saved OS surfaces.
    // ------------------=
    private func previewMaterial(_ kind: PrecisionSurfaceKind, focus: UInt8, pressed: Bool,
                                 style: UIKitSurfaceStyle?) -> PrecisionMaterial {
        var result = material(kind, focus: focus, pressed: pressed)
        if let style { result.style = style }
        return result
    }
}

@MainActor enum UIKitStylePreviewImage {
    private static let cache = NSCache<NSString, CGImage>()

    // ------------------------=
    // FUNC: image
    // DESC: Composes native buttons over neutral navy so previews show only saved texture; other surfaces retain the wallpaper scene.
    // ------------------=
    static func image(store: UIKitStore, kind: PrecisionSurfaceKind, width: Int, height: Int, material: PrecisionMaterial) -> CGImage? {
        let wallpaperPath = "assets/ui-design-kit/default/eclipse-access-wallpaper-v1.bmp"
        guard let data = store.previewAtlas(target: "assets/ui-design-kit/default/eclipse-precision-v1.rgba"),
              let atlas = PrecisionAtlas(data: data), let root = store.projectRoot else { return nil }
        let wallpaper = store.assets.first(where: { $0.target == wallpaperPath }).flatMap { store.previewImage(for: $0) }
            ?? NSImage(contentsOf: root.appending(path: wallpaperPath))
        guard let source = wallpaper?.cgImage(forProposedRect: nil, context: nil, hints: nil),
              let colorSpace = CGColorSpace(name: CGColorSpace.sRGB) else { return nil }
        let key = "\(data.hashValue)|\(store.document.overrides)|\(store.document.styles)|\(kind)|\(width)|\(height)|\(material)" as NSString
        if let image = cache.object(forKey: key) { return image }
        var raw = Data(count: width * height * 4)
        let painted = raw.withUnsafeMutableBytes { (bytes: UnsafeMutableRawBufferPointer) -> Bool in
            guard let context = CGContext(data: bytes.baseAddress, width: width, height: height, bitsPerComponent: 8,
                bytesPerRow: width * 4, space: colorSpace, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return false }
            if kind == .primary || kind == .secondary {
                context.setFillColor(CGColor(red: 9.0 / 255, green: 15.0 / 255, blue: 38.0 / 255, alpha: 1))
                context.fill(CGRect(x: 0, y: 0, width: width, height: height))
            } else {
                let scale = max(CGFloat(width) / CGFloat(source.width), CGFloat(height) / CGFloat(source.height))
                let size = CGSize(width: CGFloat(source.width) * scale, height: CGFloat(source.height) * scale)
                context.draw(source, in: CGRect(x: (CGFloat(width) - size.width) / 2, y: (CGFloat(height) - size.height) / 2,
                    width: size.width, height: size.height))
            }
            return true
        }
        guard painted else { return nil }
        if kind == .panel {
            guard let frost = PrecisionFrostRenderer.render(background: raw, width: width, height: height,
                material: material, atlas: atlas) else { return nil }
            raw = frost
        } else {
            if kind == .frame {
                let background = PrecisionMaterial(fill: studioColor(store.document.colors[1]), edgeOpacity: 0,
                    style: .resolved(for: .windowFill, styles: store.document.styles))
                guard let fill = atlas.render(.windowFill, width: width, height: height, material: background) else { return nil }
                for p in stride(from: 0, to: raw.count, by: 4) {
                    let alpha = Int(fill[p + 3])
                    for c in 0..<3 { raw[p + c] = UInt8((Int(fill[p + c]) * alpha + Int(raw[p + c]) * (255 - alpha)) / 255) }
                }
            }
            guard let pixels = atlas.render(kind, width: width, height: height, material: material) else { return nil }
            for p in stride(from: 0, to: raw.count, by: 4) {
                let alpha = Int(pixels[p + 3])
                for c in 0..<3 {
                    raw[p + c] = UInt8((Int(pixels[p + c]) * alpha + Int(raw[p + c]) * (255 - alpha)) / 255)
                }
            }
        }
        guard let provider = CGDataProvider(data: raw as CFData),
              let image = CGImage(width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 32,
                bytesPerRow: width * 4, space: colorSpace, bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue),
                provider: provider, decode: nil, shouldInterpolate: true, intent: .defaultIntent) else { return nil }
        cache.totalCostLimit = 16 * 1024 * 1024
        cache.setObject(image, forKey: key, cost: raw.count)
        return image
    }
}

/// Per-element dropdown reuses the shared native preview and style payload.
struct ElementButtonStylePicker: View {
    @ObservedObject var store: TemplateStore
    let element: StudioElement
    @State private var expanded = false

    private var title: String {
        element.surfaceMaterial == .authored ? "Use shared UIKit style"
            : GlassButtonPreset.matching(element)?.rawValue ?? "Custom appearance"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Button style").font(.caption.weight(.semibold)).foregroundStyle(.secondary)
            Button { expanded.toggle() } label: {
                HStack {
                    Text(title).lineLimit(1)
                    Spacer()
                    Image(systemName: "chevron.down").font(.caption.weight(.semibold))
                }.frame(maxWidth: .infinity, minHeight: 28)
            }
            .buttonStyle(InfinityStudioButtonStyle(emphasis: .secondary))
            .accessibilityIdentifier("studio.element.button-style")
            .popover(isPresented: $expanded, arrowEdge: .bottom) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Button style").font(.headline)
                    Button {
                        store.updateSelected("Shared button style restored") {
                            $0.surfaceMaterial = .authored
                            $0.surfaceCorners = .authored
                        }
                        expanded = false
                    } label: {
                        row("Use shared UIKit style", style: .resolved(for: .primary, styles: store.uiKit.document.styles),
                            selected: element.surfaceMaterial == .authored)
                    }.buttonStyle(.plain)
                    Divider()
                    ScrollView { VStack(spacing: 8) {
                    ForEach(GlassButtonPreset.allCases) { preset in
                        Button {
                            store.updateSelected("Button style: \(preset.rawValue)") { preset.apply(to: &$0) }
                            expanded = false
                        } label: {
                            row(preset.rawValue, style: preset.style, selected: GlassButtonPreset.matching(element) == preset)
                        }.buttonStyle(.plain)
                    }
                    } }.frame(maxHeight: 440)
                }.padding(16).frame(width: 320)
                    .background(InfinityUIKit.Palette.nativePanel)
            }
        }
    }

    // ------------------------=
    // FUNC: row
    // DESC: Shows each dropdown choice using the actual native button sampler and a visible selection checkmark.
    // ------------------=
    private func row(_ name: String, style: UIKitSurfaceStyle, selected: Bool) -> some View {
        HStack(spacing: 12) {
            if let image = UIKitStylePreviewImage.image(store: store.uiKit, kind: .primary,
                width: 112, height: 32, material: PrecisionMaterial(style: style)) {
                Image(decorative: image, scale: 1).overlay {
                    Text("Button").font(.system(size: 12, weight: .semibold)).foregroundStyle(style.material == .porcelain ? Color.black : .white)
                }.frame(width: 112, height: 32)
            }
            Text(name).font(.callout).frame(maxWidth: .infinity, alignment: .leading)
            Image(systemName: selected ? "checkmark.circle.fill" : "circle")
                .foregroundStyle(selected ? InfinityUIKit.Palette.nativeAccent : .secondary)
        }.padding(6).contentShape(Rectangle())
    }
}

/// Global recipe chooser reuses the element inspector's native previews and popover treatment.
struct GlobalButtonStylePicker: View {
    @ObservedObject var store: UIKitStore
    @State private var expanded = false
    private var selected: GlassButtonPreset? {
        GlassButtonPreset.allCases.first { store.document.styles[3] == $0.style && store.document.styles[4] == $0.style }
    }
    var body: some View {
        Button { expanded.toggle() } label: {
            Label("Global Buttons", systemImage: "rectangle.roundedtop")
        }
        .buttonStyle(InfinityStudioButtonStyle(emphasis: .secondary))
        .help("Set the shared primary and secondary button style across the OS")
        .accessibilityIdentifier("studio.global.button-style")
        .popover(isPresented: $expanded, arrowEdge: .bottom) {
            VStack(alignment: .leading, spacing: 10) {
                Text("Global button style").font(.headline)
                Text("Current: \(selected?.rawValue ?? "Custom / UIKit default")")
                    .font(.caption).foregroundStyle(.secondary)
                ScrollView { VStack(spacing: 8) {
                ForEach(GlassButtonPreset.allCases) { preset in
                    Button {
                        store.applyButtonPreset(preset)
                    } label: {
                        HStack(spacing: 12) {
                            if let image = UIKitStylePreviewImage.image(store: store, kind: .primary,
                                width: 112, height: 32, material: PrecisionMaterial(style: preset.style)) {
                                Image(decorative: image, scale: 1).overlay {
                                    Text("Button").font(.system(size: 12, weight: .semibold)).foregroundStyle(preset.style.material == .porcelain ? Color.black : .white)
                                }.frame(width: 112, height: 32)
                            }
                            VStack(alignment: .leading, spacing: 2) {
                                Text(preset.rawValue).font(.callout)
                                Text(preset.detail).font(.caption2).foregroundStyle(.secondary)
                            }.frame(maxWidth: .infinity, alignment: .leading)
                            Image(systemName: selected == preset ? "checkmark.circle.fill" : "circle")
                                .foregroundStyle(selected == preset ? InfinityUIKit.Palette.nativeAccent : .secondary)
                        }.padding(6).contentShape(Rectangle())
                    }.buttonStyle(.plain)
                }
                } }.frame(maxHeight: 470)
                Divider()
                Text("Applies to primary and secondary buttons. Individual button overrides are preserved. Use Save All Sections or Save & Update ISO to publish.")
                    .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                Button("Done") { expanded = false }
                    .buttonStyle(InfinityStudioButtonStyle(emphasis: .primary))
            }.padding(16).frame(width: 380).background(InfinityUIKit.Palette.nativePanel)
        }
    }
}

/// One shared editor for compositor-owned window chrome across all applications.
struct GlobalWindowFrameEditor: View {
    @ObservedObject var store: UIKitStore
    @State private var expanded = false

    var body: some View {
        Button {
            store.selectedSurface = 0
            expanded.toggle()
        } label: {
            Label("Global Window Frame", systemImage: "macwindow")
        }
        .buttonStyle(InfinityStudioButtonStyle(emphasis: .secondary))
        .accessibilityIdentifier("studio.global.window-frame")
        .popover(isPresented: $expanded, arrowEdge: .bottom) {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Text("Global window frame").font(.title2.weight(.semibold))
                    Text("Shared by every application. Edit Window Border or Window Background below.")
                        .font(.callout).foregroundStyle(.secondary)
                    HStack(spacing: 16) {
                        preview("Active", focus: 255)
                        preview("Inactive", focus: 0)
                    }
                    Divider()
                    Text("Frame collection").font(.headline)
                    ForEach(WindowFramePreset.allCases) { preset in
                        Button { store.setSurfaceStyle(0, style: preset.style) } label: {
                            HStack(spacing: 16) {
                                if let image = UIKitStylePreviewImage.image(store: store, kind: .frame,
                                    width: 112, height: 52, material: PrecisionMaterial(focus: 255, style: preset.style)) {
                                    Image(decorative: image, scale: 1)
                                }
                                Text(preset.title).frame(maxWidth: .infinity, alignment: .leading)
                                if store.document.styles[0].material == preset.style.material { Image(systemName: "checkmark.circle.fill") }
                            }.padding(4).contentShape(Rectangle())
                        }.buttonStyle(.plain)
                    }
                    UIKitStyleInspector(store: store)
                    Divider()
                    Text("Save All Sections or Save & Update ISO to publish. Changes remain undoable in UIKit.")
                        .font(.caption).foregroundStyle(.secondary)
                    Button("Done") { expanded = false }
                        .buttonStyle(InfinityStudioButtonStyle(emphasis: .primary))
                }.padding(20)
            }.frame(width: 520, height: 660).background(InfinityUIKit.Palette.nativePanel)
        }
    }

    // ------------------------=
    // FUNC: preview
    // DESC: Composes active and inactive frames from the same editable atlas and saved styles used by the native compositor.
    // ------------------=
    private func preview(_ title: String, focus: UInt8) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            if let image = UIKitStylePreviewImage.image(store: store, kind: .frame,
                width: 224, height: 112, material: PrecisionMaterial(focus: focus,
                    style: .resolved(for: .frame, styles: store.document.styles))) {
                Image(decorative: image, scale: 1).overlay(alignment: .topLeading) {
                    Text("InfinityOS").font(.system(size: 13, weight: .semibold)).padding(12)
                }
            }
            Text(title).font(.caption).foregroundStyle(.secondary)
        }
    }
}
