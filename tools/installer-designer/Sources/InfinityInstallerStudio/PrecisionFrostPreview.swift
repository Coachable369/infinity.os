import AppKit
import SwiftUI

/// These native backgrounds are painted before the saved layer stack by bootstrap/installer.rs and bootstrap/desktop.rs.
struct CanvasRuntimeBackdrop: View {
    @ObservedObject var store: TemplateStore

    // ------------------------=
    // FUNC: assetPath
    // DESC: Selects the exact native pre-template bitmap rather than inventing editor-only gradients beneath translucent surfaces.
    // ------------------=
    static func assetPath(for collection: ScreenCollection) -> String {
        switch collection {
        case .boot: "assets/boot/infinity-eclipse-header-v1.bmp"
        case .installation: "assets/boot/infinity-installer-background-v2.bmp"
        case .configuration: "assets/ui-design-kit/default/eclipse-access-wallpaper-v1.bmp"
        case .systemSettings: "assets/desktop/infinity-shell-wallpaper-v3.bmp"
        }
    }

    // ------------------------=
    // FUNC: image
    // DESC: Includes unsaved UIKit replacements for the runtime background while leaving authored template layers unchanged.
    // ------------------=
    static func image(store: TemplateStore) -> NSImage? {
        if store.selectedCollection == .installation && store.document.explicitInstallerBackground == true {
            return nil
        }
        let path = assetPath(for: store.selectedCollection)
        if let asset = store.uiKit.assets.first(where: { $0.target == path }),
           let image = store.uiKit.previewImage(for: asset) { return image }
        return store.projectRoot.flatMap { NSImage(contentsOf: $0.appending(path: path)) }
    }

    var body: some View {
        GeometryReader { geometry in
            ZStack {
                Color.black
                if let image = Self.image(store: store) {
                    Image(nsImage: image).resizable().interpolation(.none)
                        .aspectRatio(contentMode: .fill)
                        .frame(width: geometry.size.width, height: geometry.size.height)
                        .clipped()
                }
            }
        }
    }
}

/// Full-paint equivalent of kernel/ui/dock_material.rs; the editor always captures a clean parent scene.
enum PrecisionFrostRenderer {
    private static let columns = 256

    // ------------------------=
    // FUNC: mix
    // DESC: Matches the native packed RGB integer interpolation without crossing channel boundaries.
    // ------------------=
    private static func mix(_ a: UInt32, _ b: UInt32, _ amount: Int) -> UInt32 {
        let t = min(255, max(0, amount))
        var result: UInt32 = 0
        for c in 0..<3 {
            let shift = c * 8
            result |= UInt32((Int((a >> shift) & 255) * (255 - t) + Int((b >> shift) & 255) * t) / 255) << shift
        }
        return result
    }

    // ------------------------=
    // FUNC: average
    // DESC: Performs the same bounded separable blur pass over averaged parent pixels as the native dock material.
    // ------------------=
    private static func average(_ values: [UInt32], width: Int, height: Int, x: Int, y: Int, radius: Int, vertical: Bool) -> UInt32 {
        let center = vertical ? y : x, extent = vertical ? height : width
        let begin = max(0, center - radius), end = min(extent, center + radius + 1)
        var channels = [Int](repeating: 0, count: 3)
        for index in begin..<end {
            let pixel = values[vertical ? index * columns + x : y * columns + index]
            for c in 0..<3 { channels[c] += Int((pixel >> (8 * c)) & 255) }
        }
        let count = end - begin
        return UInt32(channels[0] / count) | UInt32(channels[1] / count) << 8 | UInt32(channels[2] / count) << 16
    }

    // ------------------------=
    // FUNC: sample
    // DESC: Reconstructs the blurred parent grid bilinearly with the native fixed-point blend convention.
    // ------------------=
    private static func sample(_ values: [UInt32], width: Int, height: Int, x: Int, y: Int) -> UInt32 {
        let gx = min(x / 256, width - 1), gy = min(y / 256, height - 1)
        let nx = min(gx + 1, width - 1), ny = min(gy + 1, height - 1)
        return mix(mix(values[gy * columns + gx], values[gy * columns + nx], x % 256),
            mix(values[ny * columns + gx], values[ny * columns + nx], x % 256), y % 256)
    }

    // ------------------------=
    // FUNC: render
    // DESC: Composites exact native blur, quartic navy tint, reflective sheen and independent Precision edge coverage over clean RGBA backdrop pixels.
    // ------------------=
    static func render(background: Data, width: Int, height: Int, material originalMaterial: PrecisionMaterial,
                       atlas: PrecisionAtlas, blur: Int = 12) -> Data? {
        guard width > 0, height > 0, width <= 4096, height <= 4096, background.count == width * height * 4 else { return nil }
        var material = originalMaterial
        var opacity = (Int(material.opacity) * 100 + 127) / 255
        if let fill = material.style.fill {
            material.fill = studioColor(fill)
            opacity = opacity * Int(material.fill.alpha) / 255
        }
        if let border = material.style.border {
            material.edge = studioColor(border)
            material.edgeOpacity = UInt8(Int(material.edgeOpacity) * Int(material.edge.alpha) / 255)
        }
        material.style.fill = nil; material.style.border = nil
        var pixels = [UInt32](repeating: 0, count: width * height)
        for index in pixels.indices {
            let p = index * 4
            pixels[index] = UInt32(background[p]) << 16 | UInt32(background[p + 1]) << 8 | UInt32(background[p + 2])
        }
        if material.style.material == .matte || material.style.material == .flat {
            material.opacity = UInt8(opacity * 255 / 100)
            material.focus = 0; material.pressed = false
            var result = background
            for y in 0..<height { for x in 0..<width {
                let pixel = atlas.pixel(.panel, width: width, height: height, x: x, y: y, material: material)
                let color = mix(pixels[y * width + x], UInt32(pixel.red) << 16 | UInt32(pixel.green) << 8 | UInt32(pixel.blue), pixel.alpha)
                let at = (y * width + x) * 4
                result[at] = UInt8((color >> 16) & 255)
                result[at + 1] = UInt8((color >> 8) & 255)
                result[at + 2] = UInt8(color & 255)
            } }
            return result
        }
        let step = max(4, (width + 255) / 256, (height + 63) / 64)
        let gridWidth = (width + step - 1) / step, gridHeight = (height + step - 1) / step
        let blurred = blur > 0 && opacity > 0 && opacity < 100
        var sampled = [UInt32](repeating: 0, count: 256 * 64)
        var scratch = sampled
        if blurred {
            for gy in 0..<gridHeight { for gx in 0..<gridWidth {
                var channels = [Int](repeating: 0, count: 3), count = 0
                for y in gy * step..<min((gy + 1) * step, height) {
                    for x in gx * step..<min((gx + 1) * step, width) {
                        let pixel = pixels[y * width + x]
                        for c in 0..<3 { channels[c] += Int((pixel >> (c * 8)) & 255) }
                        count += 1
                    }
                }
                sampled[gy * columns + gx] = UInt32(channels[0] / count) | UInt32(channels[1] / count) << 8 | UInt32(channels[2] / count) << 16
            } }
            let radius = min(6, max(1, (blur * max(1, material.scale) + step - 1) / step))
            for y in 0..<gridHeight { for x in 0..<gridWidth {
                scratch[y * columns + x] = average(sampled, width: gridWidth, height: gridHeight, x: x, y: y, radius: radius, vertical: false)
            } }
            for y in 0..<gridHeight { for x in 0..<gridWidth {
                sampled[y * columns + x] = average(scratch, width: gridWidth, height: gridHeight, x: x, y: y, radius: radius, vertical: true)
            } }
        }
        let tint = opacity * opacity * opacity * opacity * 255 / 100_000_000
        let primary = UInt32(material.fill.red) << 16 | UInt32(material.fill.green) << 8 | UInt32(material.fill.blue)
        let panel = PrecisionMaterial(edge: material.edge, opacity: 255, edgeOpacity: 0, scale: material.scale, style: material.style)
        var rimMaterial = panel
        rimMaterial.opacity = 0; rimMaterial.edgeOpacity = material.edgeOpacity
        let rim = max(PrecisionAtlas.rimWidth(.panel, scale: material.scale),
            material.style.corners == .rounded ? material.style.radius * max(1, material.scale) : 0)
        var output = Data(count: background.count)
        output.withUnsafeMutableBytes { (bytes: UnsafeMutableRawBufferPointer) in
            for y in 0..<height { for x in 0..<width {
                let index = y * width + x, original = pixels[index]
                let onRim = min(x, y, width - 1 - x, height - 1 - y) < rim
                let coverage = onRim ? atlas.pixel(.panel, width: width, height: height, x: x, y: y, material: panel).alpha : 255
                let edge = onRim ? atlas.pixel(.panel, width: width, height: height, x: x, y: y, material: rimMaterial) : .clear
                var color = original
                if coverage != 0 || edge.alpha != 0 {
                    let backdrop = blurred ? sample(sampled, width: gridWidth, height: gridHeight, x: x * 256 / step, y: y * 256 / step) : original
                    color = mix(backdrop, primary, tint)
                    let sheen = opacity == 0 || opacity == 100 ? 0 : max(0, 20 - y * 16 / height)
                    color = mix(color, 0xffffff, sheen)
                    color = mix(original, color, coverage)
                    color = mix(color, UInt32(edge.red) << 16 | UInt32(edge.green) << 8 | UInt32(edge.blue), edge.alpha)
                }
                bytes[index * 4] = UInt8((color >> 16) & 255)
                bytes[index * 4 + 1] = UInt8((color >> 8) & 255)
                bytes[index * 4 + 2] = UInt8(color & 255)
                bytes[index * 4 + 3] = background[index * 4 + 3]
            } }
        }
        return output
    }
}

@MainActor enum CanvasFrostPreview {
    private static let images = NSCache<NSString, CGImage>()

    // ------------------------=
    // FUNC: image
    // DESC: Captures clean lower authored layers without editor grid or selection overlays, then composites native frosting at preview resolution.
    // ------------------=
    static func image(for element: StudioElement, store: TemplateStore) -> CGImage? {
        guard let screen = store.selectedScreen, let root = store.projectRoot,
              let data = store.uiKit.previewAtlas(target: "assets/ui-design-kit/default/eclipse-precision-v1.rgba"),
              let atlas = PrecisionAtlas(data: data) else { return nil }
        let document = store.uiKit.document
        let overrideKey = document.overrides.map { "\($0.id)|\($0.source)" }.joined(separator: ";")
        let key = "\(root.path)|\(store.selectedCollection)|\(screen.hashValue)|\(element.id)|\(overrideKey)|\(document.colors)|\(document.styles)|\(data.hashValue)" as NSString
        if let cached = images.object(forKey: key) { return cached }
        let layers = screen.elements.sorted {
            $0.zIndex == $1.zIndex ? $0.id.uuidString < $1.id.uuidString : $0.zIndex < $1.zIndex
        }.prefix { $0.id != element.id }.filter { !$0.hidden }
        let scene = ZStack(alignment: .topLeading) {
            CanvasRuntimeBackdrop(store: store)
            ForEach(Array(layers)) { layer in
                CanvasElementView(element: layer, store: store, canvasScale: CGSize(width: 1.6, height: 1), renderingBackdrop: true)
            }
        }.frame(width: 1600, height: 1000)
        guard let source = ImageRenderer(content: scene).cgImage else { return nil }
        let rect = CGRect(x: Double(element.frame.x) * 1.6, y: Double(element.frame.y),
            width: Double(element.frame.width) * 1.6, height: Double(element.frame.height)).integral
        guard let crop = source.cropping(to: rect), crop.width > 0, crop.height > 0,
              let colorSpace = CGColorSpace(name: CGColorSpace.sRGB) else { return nil }
        let width = crop.width, height = crop.height
        var raw = Data(count: width * height * 4)
        let painted = raw.withUnsafeMutableBytes { (bytes: UnsafeMutableRawBufferPointer) -> Bool in
            guard let context = CGContext(data: bytes.baseAddress, width: width, height: height, bitsPerComponent: 8,
                bytesPerRow: width * 4, space: colorSpace,
                bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return false }
            context.draw(crop, in: CGRect(x: 0, y: 0, width: width, height: height))
            return true
        }
        guard painted, let result = PrecisionFrostRenderer.render(background: raw, width: width, height: height,
            material: PrecisionMaterial(fill: element.fill, edge: element.border,
                opacity: UInt8(Int(element.fill.alpha) * element.opacity / 100),
                edgeOpacity: UInt8(Int(element.border.alpha) * element.opacity / 100),
                style: .template(for: element, configuration: store.selectedCollection == .configuration, styles: document.styles)), atlas: atlas),
              let provider = CGDataProvider(data: result as CFData),
              let image = CGImage(width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: width * 4,
                space: colorSpace, bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue),
                provider: provider, decode: nil, shouldInterpolate: true, intent: .defaultIntent) else { return nil }
        images.totalCostLimit = 32 * 1024 * 1024
        images.setObject(image, forKey: key, cost: result.count)
        return image
    }
}
