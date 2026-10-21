import AppKit
import SwiftUI

/// The surface order and integer sampling contract mirror kernel/ui/panel_recipe.rs.
enum PrecisionSurfaceKind: String, CaseIterable, Identifiable {
    case frame, windowFill, panel, primary, secondary, field, current, inactive
    var id: String { rawValue }
}

struct PrecisionMaterial: Hashable {
    var fill: StudioColor
    var edge: StudioColor
    var opacity: UInt8
    var edgeOpacity: UInt8
    var focus: UInt8
    var pressed: Bool
    var scale: Int
    var style: UIKitSurfaceStyle

    // ------------------------=
    // FUNC: init
    // DESC: Keeps authored fill and edge alpha independent, matching the native surface material.
    // ------------------=
    init(fill: StudioColor = InfinityUIKit.EclipseAccess.surface,
         edge: StudioColor = InfinityUIKit.EclipseAccess.border,
         opacity: UInt8? = nil, edgeOpacity: UInt8? = nil, focus: UInt8 = 0,
         pressed: Bool = false, scale: Int = 1, style: UIKitSurfaceStyle = .authored) {
        self.fill = fill
        self.edge = edge
        self.opacity = opacity ?? fill.alpha
        self.edgeOpacity = edgeOpacity ?? edge.alpha
        self.focus = focus
        self.pressed = pressed
        self.scale = max(1, scale)
        self.style = style
    }
}

struct PrecisionPixel: Equatable {
    var red: Int
    var green: Int
    var blue: Int
    var alpha: Int
    static let clear = PrecisionPixel(red: 0, green: 0, blue: 0, alpha: 0)
}

struct PrecisionAtlas {
    static let cellSize = 256
    static let cellCount = 14
    static let byteCount = 2 * cellCount * cellSize * cellSize * 4
    let bytes: Data

    // ------------------------=
    // FUNC: init
    // DESC: Rejects incomplete or unrelated uploads rather than presenting a synthetic replacement surface.
    // ------------------=
    init?(data: Data) {
        guard data.count == Self.byteCount else { return nil }
        bytes = data
    }

    // ------------------------=
    // FUNC: rimWidth
    // DESC: Exposes the same native nine-slice corner extents for preview layout and behavioral validation.
    // ------------------=
    static func rimWidth(_ kind: PrecisionSurfaceKind, scale: Int) -> Int {
        let width: Int = switch kind {
        case .frame: 18
        case .panel: 12
        case .primary, .secondary, .field: 8
        default: 0
        }
        return width * max(1, scale)
    }

    // ------------------------=
    // FUNC: axis
    // DESC: Preserves native corner pixels and stretches only interior spans using the kernel fixed-point coordinates.
    // ------------------=
    private static func axis(_ at: Int, length: Int, cut: Int, scale: Int) -> Int {
        let cap = max(1, min(cut * scale, length / 2))
        if at < cap { return at * cut * 256 / cap }
        if at >= max(0, length - cap) {
            return (cellSize - cut) * 256 + (at - (length - cap)) * cut * 256 / cap
        }
        return cut * 256 + (at - cap) * (cellSize - cut * 2) * 256 / max(1, length - cap * 2)
    }

    // ------------------------=
    // FUNC: texture
    // DESC: Bilinearly filters authored pixels in premultiplied space without a dark transparency fringe.
    // ------------------=
    private func texture(_ index: Int, x: Int, y: Int) -> PrecisionPixel {
        let sx = x / 256, sy = y / 256, wx = x % 256, wy = y % 256
        let positions = [(sx, sy), (sx + 1, sy), (sx, sy + 1), (sx + 1, sy + 1)]
        let offsets = positions.map { (index * Self.cellSize * Self.cellSize + min(255, $0.1) * 256 + min(255, $0.0)) * 4 }
        if wx == 0 && wy == 0 {
            let p = offsets[0]
            return PrecisionPixel(red: Int(bytes[p]), green: Int(bytes[p + 1]), blue: Int(bytes[p + 2]), alpha: Int(bytes[p + 3]))
        }
        let weights = [(256 - wx) * (256 - wy), wx * (256 - wy), (256 - wx) * wy, wx * wy]
        var channels = [Int](repeating: 0, count: 4)
        if offsets.allSatisfy({ bytes[$0 + 3] == 255 }) {
            for i in 0..<4 { for c in 0..<3 { channels[c] += Int(bytes[offsets[i] + c]) * weights[i] } }
            return PrecisionPixel(red: channels[0] >> 16, green: channels[1] >> 16, blue: channels[2] >> 16, alpha: 255)
        }
        for i in 0..<4 {
            let alpha = Int(bytes[offsets[i] + 3])
            for c in 0..<3 { channels[c] += Int(bytes[offsets[i] + c]) * alpha * weights[i] }
            channels[3] += alpha * weights[i]
        }
        guard channels[3] != 0 else { return .clear }
        return PrecisionPixel(red: channels[0] / channels[3], green: channels[1] / channels[3],
            blue: channels[2] / channels[3], alpha: (channels[3] + 32768) / 65536)
    }

    // ------------------------=
    // FUNC: transition
    // DESC: Interpolates authored state coverage with the same integer premultiplied arithmetic as the OS.
    // ------------------=
    private static func transition(_ a: PrecisionPixel, _ b: PrecisionPixel, amount: UInt8) -> PrecisionPixel {
        let t = Int(amount)
        if t == 0 { return a }
        if t == 255 { return b }
        let alpha = a.alpha * (255 - t) + b.alpha * t
        guard alpha != 0 else { return .clear }
        let aChannels = [a.red, a.green, a.blue], bChannels = [b.red, b.green, b.blue]
        let channels = (0..<3).map { (aChannels[$0] * a.alpha * (255 - t) + bChannels[$0] * b.alpha * t) / alpha }
        return PrecisionPixel(red: channels[0], green: channels[1], blue: channels[2], alpha: (alpha + 127) / 255)
    }

    // ------------------------=
    // FUNC: themed
    // DESC: Applies only the deviation from approved tokens, retaining the atlas contrast and independent opacity.
    // ------------------=
    private static func themed(_ pixel: PrecisionPixel, base: (Int, Int, Int), target: StudioColor, opacity: UInt8) -> PrecisionPixel {
        guard pixel.alpha != 0 && opacity != 0 else { return .clear }
        return PrecisionPixel(red: min(255, max(0, pixel.red + Int(target.red) - base.0)),
            green: min(255, max(0, pixel.green + Int(target.green) - base.1)),
            blue: min(255, max(0, pixel.blue + Int(target.blue) - base.2)), alpha: pixel.alpha * Int(opacity) / 255)
    }

    // ------------------------=
    // FUNC: over
    // DESC: Composes edge light over the fill without making the edge depend on background alpha.
    // ------------------=
    private static func over(_ background: PrecisionPixel, edge: PrecisionPixel) -> PrecisionPixel {
        if edge.alpha == 0 { return background }
        if edge.alpha == 255 || background.alpha == 0 { return edge }
        let retained = background.alpha * (255 - edge.alpha)
        let alpha = edge.alpha * 255 + retained
        guard alpha != 0 else { return .clear }
        let b = [background.red, background.green, background.blue], e = [edge.red, edge.green, edge.blue]
        let channels = (0..<3).map { (e[$0] * edge.alpha * 255 + b[$0] * retained) / alpha }
        return PrecisionPixel(red: channels[0], green: channels[1], blue: channels[2], alpha: (alpha + 127) / 255)
    }

    // ------------------------=
    // FUNC: authoredPixel
    // DESC: Samples actual Precision state pixels, native nine-slice geometry, and independent authored tint and alpha.
    // ------------------=
    private func authoredPixel(_ kind: PrecisionSurfaceKind, width: Int, height: Int, x: Int, y: Int, material m: PrecisionMaterial, matte: Bool = false) -> PrecisionPixel {
        guard width > 0, height > 0, x >= 0, y >= 0, x < width, y < height else { return .clear }
        let scale = max(1, m.scale), circular = kind == .current || kind == .inactive
        let rim = Self.rimWidth(kind, scale: scale)
        let distance = min(x, y, width - 1 - x, height - 1 - y)
        if kind == .frame && distance >= rim { return .clear }
        let pair: (Int, Int) = switch kind {
        case .frame: (0, 1)
        case .panel, .windowFill: (2, 3)
        case .primary: m.pressed ? (6, 6) : (4, 5)
        case .secondary: m.pressed ? (13, 13) : (11, 12)
        case .field: (7, 8)
        case .current: (9, 9)
        case .inactive: (10, 10)
        }
        let cut = kind == .windowFill ? 12 : rim / scale
        let fx = circular ? x * 255 * 256 / width : Self.axis(x, length: width, cut: cut, scale: scale)
        let fy = circular ? y * 255 * 256 / height : Self.axis(y, length: height, cut: cut, scale: scale)
        let state: (Int) -> PrecisionPixel = { layer in
            let normal = pair.0 + layer * Self.cellCount, focused = pair.1 + layer * Self.cellCount
            if normal == focused || m.focus == 0 { return texture(normal, x: fx, y: fy) }
            if m.focus == 255 { return texture(focused, x: fx, y: fy) }
            return Self.transition(texture(normal, x: fx, y: fy), texture(focused, x: fx, y: fy), amount: m.focus)
        }
        let base = kind == .primary || kind == .current ? (62, 111, 255) : (16, 27, 58)
        var background = PrecisionPixel.clear
        if m.opacity != 0 && kind != .frame && (kind == .windowFill || circular || distance >= 2 * scale) {
            var sample = kind == .windowFill ? texture(2, x: min(243 * 256, max(12 * 256, fx)),
                y: min(243 * 256, max(12 * 256, fy))) : state(0)
            if matte {
                sample.red = (base.0 + (sample.red - base.0) / 5).clamped(to: 0...255)
                sample.green = (base.1 + (sample.green - base.1) / 5).clamped(to: 0...255)
                sample.blue = (base.2 + (sample.blue - base.2) / 5).clamped(to: 0...255)
            }
            background = Self.themed(sample, base: base, target: m.fill, opacity: m.opacity)
        }
        let edge = m.edgeOpacity != 0 && !circular && distance < rim
            ? Self.themed(state(1), base: (134, 199, 255), target: m.edge, opacity: m.edgeOpacity) : .clear
        return Self.over(background, edge: edge)
    }

    // ------------------------=
    // FUNC: styledAxis
    // DESC: Resizes only authored antialiased corner coverage using the native fixed-point radius remapping.
    // ------------------=
    private static func styledAxis(_ at: Int, length: Int, cut: Int, cap: Int) -> Int {
        let cap = max(1, min(cap, length / 2))
        if at < cap { return at * cut * 256 / cap }
        if at >= length - cap { return (cellSize - cut) * 256 + (at - (length - cap)) * cut * 256 / cap }
        return cut * 256 + (at - cap) * (cellSize - cut * 2) * 256 / max(1, length - cap * 2)
    }

    // ------------------------=
    // FUNC: integerSquareRoot
    // DESC: Uses the native integer-only edge-depth calculation so ARM, x86 and Studio agree at every radius pixel.
    // ------------------=
    private static func integerSquareRoot(_ value: Int) -> Int {
        if value < 2 { return value }
        var low = 1, high = min(value, Int(UInt32.max))
        while low < high {
            let middle = low + (high - low + 1) / 2
            if middle <= value / middle { low = middle } else { high = middle - 1 }
        }
        return low
    }

    // ------------------------=
    // FUNC: roundedShape
    // DESC: Matches the native four-by-four antialiased silhouette and fixed-point inward depth without flattening uploaded rim artwork.
    // ------------------=
    private static func roundedShape(x: Int, y: Int, width: Int, height: Int, radius: Int) -> (Int, Int) {
        let r = min(radius, width / 2, height / 2)
        let ix = min(x * 2 + 1, (width - x) * 2 - 1) * 128
        let iy = min(y * 2 + 1, (height - y) * 2 - 1) * 128
        if r == 0 || ix >= r * 256 || iy >= r * 256 { return (255, min(ix, iy)) }
        let rx = r * 256 - ix, ry = r * 256 - iy
        let depth = max(0, r * 256 - integerSquareRoot(rx * rx + ry * ry))
        let cx = x < r ? r * 8 : (width - r) * 8
        let cy = y < r ? r * 8 : (height - r) * 8
        var count = 0
        for sy in [1, 3, 5, 7] { for sx in [1, 3, 5, 7] {
            let dx = x * 8 + sx - cx, dy = y * 8 + sy - cy
            if dx * dx + dy * dy <= r * r * 64 { count += 1 }
        } }
        return ((count * 255 + 8) / 16, depth)
    }

    // ------------------------=
    // FUNC: pixel
    // DESC: Mirrors native material, tint and image-backed corner variants while leaving approved authored pixels unchanged by default.
    // ------------------=
    func pixel(_ originalKind: PrecisionSurfaceKind, width: Int, height: Int, x: Int, y: Int, material original: PrecisionMaterial) -> PrecisionPixel {
        guard width > 0, height > 0, x >= 0, y >= 0, x < width, y < height else { return .clear }
        var kind = originalKind
        var style = original.style
        if (kind == .frame || kind == .windowFill) && style.corners == .square { style.corners = .authored }
        var m = original
        if let fill = style.fill {
            m.fill = studioColor(fill)
            m.opacity = UInt8(Int(m.opacity) * Int(m.fill.alpha) / 255)
        }
        if let border = style.border {
            m.edge = studioColor(border)
            m.edgeOpacity = UInt8(Int(m.edgeOpacity) * Int(m.edge.alpha) / 255)
        }
        if style.material != .matte && style.corners == .authored {
            return authoredPixel(kind, width: width, height: height, x: x, y: y, material: m)
        }
        if kind == .current || kind == .inactive {
            if style.corners == .authored {
                return authoredPixel(kind, width: width, height: height, x: x, y: y, material: m, matte: true)
            }
            // Explicit silhouettes reuse the approved filled/quiet surface edge family;
            // authored circular markers retain their original image coverage above.
            kind = kind == .current ? .primary : .secondary
        }
        let scale = max(1, m.scale)
        let distance = min(x, y, width - 1 - x, height - 1 - y)
        let rounded = style.corners == .rounded && style.radius != 0
        let square = style.corners == .square || (style.corners == .rounded && style.radius == 0)
        let pair: (Int, Int) = switch kind {
        case .frame: rounded ? (2, 3) : (0, 1)
        case .windowFill, .panel: (2, 3)
        case .primary: m.pressed ? (6, 6) : (4, 5)
        case .secondary: m.pressed ? (13, 13) : (11, 12)
        case .field: (7, 8)
        default: (2, 3)
        }
        let cut = kind == .frame && !rounded ? 18 : ([.frame, .windowFill, .panel].contains(kind) ? 12 : 8)
        let cap = rounded ? style.radius * scale : cut * scale
        let fx = rounded ? Self.styledAxis(x, length: width, cut: cut, cap: cap) : Self.axis(x, length: width, cut: cut, scale: scale)
        let fy = rounded ? Self.styledAxis(y, length: height, cut: cut, cap: cap) : Self.axis(y, length: height, cut: cut, scale: scale)
        let state: (Int, Int, Int) -> PrecisionPixel = { layer, sx, sy in
            let normal = pair.0 + layer * Self.cellCount, focused = pair.1 + layer * Self.cellCount
            if normal == focused || m.focus == 0 { return texture(normal, x: sx, y: sy) }
            if m.focus == 255 { return texture(focused, x: sx, y: sy) }
            return Self.transition(texture(normal, x: sx, y: sy), texture(focused, x: sx, y: sy), amount: m.focus)
        }
        let base = kind == .primary ? (62, 111, 255) : (16, 27, 58)
        let (coverage, depth) = rounded ? Self.roundedShape(x: x, y: y, width: width, height: height, radius: cap) : (255, 0)
        if coverage == 0 { return .clear }
        var fill = square || rounded || kind == .windowFill
            ? state(0, fx.clamped(to: cut * 256...(255 - cut) * 256), fy.clamped(to: cut * 256...(255 - cut) * 256))
            : state(0, fx, fy)
        if style.material == .matte {
            fill.red = (base.0 + (fill.red - base.0) / 5).clamped(to: 0...255)
            fill.green = (base.1 + (fill.green - base.1) / 5).clamped(to: 0...255)
            fill.blue = (base.2 + (fill.blue - base.2) / 5).clamped(to: 0...255)
        }
        let background = kind == .frame ? .clear : Self.themed(fill, base: base, target: m.fill, opacity: m.opacity)
        var edge = PrecisionPixel.clear
        if kind != .windowFill && m.edgeOpacity != 0 {
            var raw = PrecisionPixel.clear
            if rounded {
                if depth < cut * scale * 256 {
                    let sample = min(depth / scale, cut * 256)
                    if min(x, width - 1 - x) < min(y, height - 1 - y) {
                        raw = state(1, x < width / 2 ? sample : 255 * 256 - sample, 128 * 256)
                    } else {
                        raw = state(1, 128 * 256, y < height / 2 ? sample : 255 * 256 - sample)
                    }
                }
            } else if square {
                let horizontal = min(y, height - 1 - y) < cut * scale ? state(1, 128 * 256, fy) : .clear
                let vertical = min(x, width - 1 - x) < cut * scale ? state(1, fx, 128 * 256) : .clear
                raw = Self.over(horizontal, edge: vertical)
            } else if distance < max(cap, cut * scale) { raw = state(1, fx, fy) }
            edge = Self.themed(raw, base: (134, 199, 255), target: m.edge, opacity: m.edgeOpacity)
        }
        var result = Self.over(background, edge: edge)
        result.alpha = result.alpha * coverage / 255
        return result
    }

    // ------------------------=
    // FUNC: render
    // DESC: Renders bounded straight-alpha RGBA bytes suitable for both visual previews and pixel-parity tests.
    // ------------------=
    func render(_ kind: PrecisionSurfaceKind, width: Int, height: Int, material: PrecisionMaterial) -> Data? {
        guard width > 0, height > 0, width <= 4096, height <= 4096 else { return nil }
        var output = Data(count: width * height * 4)
        output.withUnsafeMutableBytes { (bytes: UnsafeMutableRawBufferPointer) in
            for y in 0..<height { for x in 0..<width {
                let pixel = pixel(kind, width: width, height: height, x: x, y: y, material: material)
                let offset = (y * width + x) * 4
                bytes[offset] = UInt8(pixel.red); bytes[offset + 1] = UInt8(pixel.green)
                bytes[offset + 2] = UInt8(pixel.blue); bytes[offset + 3] = UInt8(pixel.alpha)
            } }
        }
        return output
    }
}

@MainActor private enum PrecisionPreviewCache {
    private static var atlases: [URL: (Date, UInt64, PrecisionAtlas)] = [:]
    private static var drafts: [URL: (Data, PrecisionAtlas)] = [:]
    private static let images = NSCache<NSString, CGImage>()

    // ------------------------=
    // FUNC: image
    // DESC: Reuses rendered states and refreshes uploaded atlas bytes when the on-disk resource changes.
    // ------------------=
    static func image(at url: URL, kind: PrecisionSurfaceKind, width: Int, height: Int, material: PrecisionMaterial, atlasData: Data? = nil) -> CGImage? {
        let atlas: PrecisionAtlas
        if let atlasData {
            if drafts[url]?.0 != atlasData {
                guard let loaded = PrecisionAtlas(data: atlasData) else { return nil }
                drafts[url] = (atlasData, loaded)
                images.removeAllObjects()
            }
            guard let loaded = drafts[url]?.1 else { return nil }
            atlas = loaded
        } else {
        guard let attributes = try? FileManager.default.attributesOfItem(atPath: url.path),
              let modified = attributes[.modificationDate] as? Date,
              let length = attributes[.size] as? UInt64 else { return nil }
        if atlases[url]?.0 != modified || atlases[url]?.1 != length {
            guard let data = try? Data(contentsOf: url), let atlas = PrecisionAtlas(data: data) else { return nil }
            atlases[url] = (modified, length, atlas)
            images.removeAllObjects()
        }
            guard let loaded = atlases[url]?.2 else { return nil }
            atlas = loaded
        }
        let key = "\(url.path)|\(atlasData == nil ? 0 : 1)|\(kind.rawValue)|\(width)|\(height)|\(material)" as NSString
        if let image = images.object(forKey: key) { return image }
        guard let data = atlas.render(kind, width: width, height: height, material: material),
              let provider = CGDataProvider(data: data as CFData),
              let colorSpace = CGColorSpace(name: CGColorSpace.sRGB),
              let image = CGImage(width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 32,
                bytesPerRow: width * 4, space: colorSpace,
                bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue), provider: provider,
                decode: nil, shouldInterpolate: true, intent: .defaultIntent) else { return nil }
        images.totalCostLimit = 64 * 1024 * 1024
        images.setObject(image, forKey: key, cost: width * height * 4)
        return image
    }
}

struct PrecisionSurface: View, Animatable {
    let kind: PrecisionSurfaceKind
    let material: PrecisionMaterial
    let projectRoot: URL?
    var logicalScale: CGFloat = 1
    var atlasURL: URL? = nil
    var atlasData: Data? = nil
    nonisolated var focusAmount: Double
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    // ------------------------=
    // FUNC: init
    // DESC: Accepts a semantic native surface and optional uploaded atlas without an approximate artwork fallback.
    // ------------------=
    init(kind: PrecisionSurfaceKind, material: PrecisionMaterial, projectRoot: URL?, logicalScale: CGFloat = 1, atlasURL: URL? = nil, atlasData: Data? = nil) {
        self.kind = kind
        self.material = material
        self.projectRoot = projectRoot
        self.logicalScale = logicalScale
        self.atlasURL = atlasURL
        self.atlasData = atlasData
        focusAmount = Double(material.focus)
    }

    nonisolated var animatableData: Double {
        get { focusAmount }
        set { focusAmount = newValue }
    }

    var body: some View {
        GeometryReader { geometry in
            let width = Int((geometry.size.width / max(0.01, logicalScale)).rounded())
            let height = Int((geometry.size.height / max(0.01, logicalScale)).rounded())
            let url = atlasURL ?? projectRoot?.appending(path: "assets/ui-design-kit/default/eclipse-precision-v1.rgba")
            var animated = material
            let _ = animated.focus = reduceMotion ? material.focus : UInt8(min(255, max(0, focusAmount.rounded())))
            if let url, let image = PrecisionPreviewCache.image(at: url, kind: kind, width: width, height: height, material: animated, atlasData: atlasData) {
                Image(decorative: image, scale: 1).resizable().interpolation(.high)
            } else {
                // Fail visibly; do not misrepresent a procedural approximation as WYSIWYG.
                Text("Precision artwork unavailable").font(.caption).foregroundStyle(.orange)
            }
        }
        .allowsHitTesting(false)
    }
}
