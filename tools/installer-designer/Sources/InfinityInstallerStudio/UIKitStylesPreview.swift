import AppKit
import SwiftUI

/// The editor previews the native compositor recipe, including unsaved artwork and style changes.
struct UIKitStylesPreview: View {
    @ObservedObject var store: UIKitStore
    private let kinds: [PrecisionSurfaceKind] = [.frame, .windowFill, .panel, .primary, .secondary, .field]
    private let columns = [GridItem(.adaptive(minimum: 240, maximum: 420), spacing: 20)]

    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
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
                                    focus: UInt8 = 0, pressed: Bool = false) -> some View {
        let material = material(kind, focus: focus, pressed: pressed)
        if let image = UIKitStylePreviewImage.image(store: store, kind: kind, width: width, height: height, material: material) {
            Image(decorative: image, scale: 1).resizable().interpolation(.high).aspectRatio(contentMode: .fit)
                .overlay {
                    if kind == .primary || kind == .secondary {
                        Text(kind == .primary ? "Continue" : "Cancel")
                            .font(InfinityUIKit.EclipseAccess.font(size: 16, semibold: true, projectRoot: store.projectRoot))
                    } else if kind == .field {
                        Text("Editable input").font(InfinityUIKit.EclipseAccess.font(size: 15, projectRoot: store.projectRoot))
                            .foregroundStyle(studioColor(store.document.colors[5]).color)
                    }
                }
        } else {
            Text("Native preview artwork unavailable").font(.caption).foregroundStyle(.orange)
        }
    }
}

@MainActor private enum UIKitStylePreviewImage {
    private static let cache = NSCache<NSString, CGImage>()

    // ------------------------=
    // FUNC: image
    // DESC: Composes draft native surface pixels over an aspect-filled OS bitmap and caches exact style/artwork identities.
    // ------------------=
    static func image(store: UIKitStore, kind: PrecisionSurfaceKind, width: Int, height: Int, material: PrecisionMaterial) -> CGImage? {
        let wallpaperPath = "assets/ui-design-kit/default/eclipse-access-wallpaper-v1.bmp"
        guard let data = store.previewAtlas(target: "assets/ui-design-kit/default/eclipse-precision-v1.rgba"),
              let atlas = PrecisionAtlas(data: data), let root = store.projectRoot else { return nil }
        let wallpaper = store.assets.first(where: { $0.target == wallpaperPath }).flatMap { store.previewImage(for: $0) }
            ?? NSImage(contentsOf: root.appending(path: wallpaperPath))
        guard let source = wallpaper?.cgImage(forProposedRect: nil, context: nil, hints: nil),
              let colorSpace = CGColorSpace(name: CGColorSpace.sRGB) else { return nil }
        let key = "\(data.hashValue)|\(store.document.overrides)|\(kind)|\(width)|\(height)|\(material)" as NSString
        if let image = cache.object(forKey: key) { return image }
        var raw = Data(count: width * height * 4)
        let painted = raw.withUnsafeMutableBytes { (bytes: UnsafeMutableRawBufferPointer) -> Bool in
            guard let context = CGContext(data: bytes.baseAddress, width: width, height: height, bitsPerComponent: 8,
                bytesPerRow: width * 4, space: colorSpace, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return false }
            let scale = max(CGFloat(width) / CGFloat(source.width), CGFloat(height) / CGFloat(source.height))
            let size = CGSize(width: CGFloat(source.width) * scale, height: CGFloat(source.height) * scale)
            context.draw(source, in: CGRect(x: (CGFloat(width) - size.width) / 2, y: (CGFloat(height) - size.height) / 2,
                width: size.width, height: size.height))
            return true
        }
        guard painted else { return nil }
        if kind == .panel {
            guard let frost = PrecisionFrostRenderer.render(background: raw, width: width, height: height,
                material: material, atlas: atlas) else { return nil }
            raw = frost
        } else {
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
