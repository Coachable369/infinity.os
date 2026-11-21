import AppKit
import SwiftUI

enum GlassButtonPreset: String, CaseIterable, Identifiable {
    case optical = "Optical Glass"
    case liquid = "Liquid Pill"
    case frosted = "Frosted Slate"
    case solid = "Solid Sapphire"
    case porcelain = "Porcelain"
    case carbon = "Carbon"
    case aurora = "Aurora Silk"
    case outline = "Clear Outline"
    case floating = "Floating Surface"
    case tinted = "Tinted Glass"
    case textAction = "Text Action"
    case sapphire = "Classic Sapphire Glass", smoked = "Smoked Glass", ice = "Frosted Ice"
    case satin = "Satin Blue", minimal = "Minimal Blue"
    case graphite = "Graphite Matte"
    var id: String { rawValue }
    var style: UIKitSurfaceStyle {
        if let index = Self.modern.firstIndex(of: self) {
            return UIKitSurfaceStyle(material: UIKitSurfaceMaterial(rawValue: index + 4)!, corners: .rounded,
                radius: [10, 32, 12, 8, 10, 5, 14, 8, 12, 8, 8][index], fill: nil, border: nil)
        }
        let fill: UInt32 = switch self {
        case .sapphire: 0xE8204DAD
        case .smoked: 0xD8101B3A
        case .ice: 0xC8447394
        case .satin: 0xFF3159A6
        case .minimal: 0xFF245CD8
        case .graphite: 0xFF293747
        default: 0
        }
        let material: UIKitSurfaceMaterial = (self == .satin || self == .graphite) ? .matte : self == .minimal ? .flat : .glass
        let radius = self == .satin ? 8 : self == .minimal ? 6 : 10
        return UIKitSurfaceStyle(material: material, corners: .rounded, radius: radius,
            fill: fill, border: 0xFF86C7FF)
    }
    static let modern: [Self] = [.optical, .liquid, .frosted, .solid, .porcelain, .carbon, .aurora, .outline, .floating, .tinted, .textAction]
    var detail: String {
        switch self {
        case .optical: "Optical Glass · shared artwork and states"
        case .liquid: "Liquid Pill · shared artwork and states"
        case .frosted: "Frosted Slate · shared artwork and states"
        case .solid: "Solid Sapphire · shared artwork and states"
        case .porcelain: "Porcelain · shared artwork and states"
        case .carbon: "Carbon · shared artwork and states"
        case .aurora: "Aurora Silk · shared artwork and states"
        case .outline: "Clear Outline · shared artwork and states"
        case .floating: "Floating Surface · shared artwork and states"
        case .tinted: "Tinted Glass · shared artwork and states"
        case .textAction: "Text Action · shared artwork and states"
        case .sapphire: "Traditional reflective blue glass"
        case .smoked: "Deep translucent charcoal glass"
        case .ice: "Light frosted glass"
        case .satin: "Opaque satin, subtle reflections"
        case .minimal: "Flat blue, compact rounded corners"
        case .graphite: "Opaque graphite, softly reflected steel edges"
        }
    }

    // ------------------------=
    // FUNC: apply
    // DESC: Stores a per-button preset through existing native material and color fields without changing actions or geometry.
    // ------------------=
    func apply(to element: inout StudioElement) {
        let recipe = style
        element.surfaceMaterial = recipe.material
        element.surfaceCorners = recipe.corners
        element.cornerRadius = recipe.radius
        element.fill = studioColor(recipe.fill ?? 0xFFFFFFFF)
        element.border = studioColor(recipe.border ?? 0xFFFFFFFF)
        element.fillToken = nil
        element.borderToken = nil
    }

    // ------------------------=
    // FUNC: matching
    // DESC: Identifies an exact preset while leaving custom local appearances explicitly labeled as custom.
    // ------------------=
    static func matching(_ element: StudioElement) -> Self? {
        allCases.first { preset in
            let style = preset.style
            return element.surfaceMaterial == style.material && element.surfaceCorners == style.corners
                && element.cornerRadius == style.radius && element.fill == studioColor(style.fill ?? 0xFFFFFFFF)
                && element.border == studioColor(style.border ?? 0xFFFFFFFF) && element.fillToken == nil && element.borderToken == nil
        }
    }
}

enum UIKitSurfaceMaterial: Int, Codable, CaseIterable, Identifiable, Hashable {
    case authored = 0, glass = 1, matte = 2, flat = 3
    case optical = 4
    case liquid = 5
    case frosted = 6
    case solid = 7
    case porcelain = 8
    case carbon = 9
    case aurora = 10
    case outline = 11
    case floating = 12
    case tinted = 13
    case textAction = 14
    case frame0 = 15
    case frame1 = 16
    case frame2 = 17
    case frame3 = 18
    case frame4 = 19
    case frame5 = 20
    case frame6 = 21
    case frame7 = 22
    case frame8 = 23
    case frame9 = 24
    var id: Int { rawValue }
    var title: String {
        switch self { case .authored: "Kit default"; case .glass: "Glass"; case .matte: "Matte"; case .flat: "Flat"
        case .optical: "Optical Glass"
        case .liquid: "Liquid Pill"
        case .frosted: "Frosted Slate"
        case .solid: "Solid Sapphire"
        case .porcelain: "Porcelain"
        case .carbon: "Carbon"
        case .aurora: "Aurora Silk"
        case .outline: "Clear Outline"
        case .floating: "Floating Surface"
        case .tinted: "Tinted Glass"
        case .textAction: "Text Action"
        case .frame0: "Sapphire Hairline"
        case .frame1: "Soft Shadow"
        case .frame2: "Frosted Rim"
        case .frame3: "Graphite Edge"
        case .frame4: "Porcelain Edge"
        case .frame5: "Satin Silver"
        case .frame6: "Aurora Trace"
        case .frame7: "Horizon Light"
        case .frame8: "Inset Shadow"
        case .frame9: "Cobalt Accent"
        }
    }
}

enum UIKitCornerStyle: Int, Codable, CaseIterable, Identifiable, Hashable {
    case authored = 0, square = 1, rounded = 2
    var id: Int { rawValue }
    var title: String {
        switch self { case .authored: "Kit default"; case .square: "Square"; case .rounded: "Rounded" }
    }
}

/// Ordered global native records: frame, window fill, panel, primary, secondary, field.
struct UIKitSurfaceStyle: Codable, Equatable, Hashable {
    var material: UIKitSurfaceMaterial = .authored
    var corners: UIKitCornerStyle = .authored
    var radius: Int = 10
    var fill: UInt32? = nil
    var border: UInt32? = nil

    static let authored = UIKitSurfaceStyle()
    static let defaults = Array(repeating: UIKitSurfaceStyle(), count: 6)

    // ------------------------=
    // FUNC: index
    // DESC: Shares the native surface-style record order while leaving circular progress indicators independent.
    // ------------------=
    static func index(for kind: PrecisionSurfaceKind) -> Int? {
        switch kind {
        case .frame: 0
        case .windowFill: 1
        case .panel: 2
        case .primary: 3
        case .secondary: 4
        case .field: 5
        case .current, .inactive: nil
        }
    }

    // ------------------------=
    // FUNC: resolved
    // DESC: Resolves one native global style without applying panel options to glyph or progress artwork.
    // ------------------=
    static func resolved(for kind: PrecisionSurfaceKind, styles: [UIKitSurfaceStyle]) -> UIKitSurfaceStyle {
        guard let index = index(for: kind), styles.indices.contains(index) else { return .authored }
        var result = styles[index]
        if kind == .windowFill, let frame = styles.first, frame.corners != .authored {
            result.corners = frame.corners; result.radius = frame.radius
        }
        return result
    }

    // ------------------------=
    // FUNC: template
    // DESC: Inherits shared recipes, opting buttons into shared colors unless their layer selects an explicit material.
    // ------------------=
    func template(material: UIKitSurfaceMaterial, corners: UIKitCornerStyle, radius: Int, inheritColors: Bool = false) -> UIKitSurfaceStyle {
        var result = self
        if material != .authored || !inheritColors { result.fill = nil; result.border = nil }
        if material != .authored { result.material = material }
        if corners != .authored { result.corners = corners; result.radius = radius.clamped(to: 0...40) }
        return result
    }

    // ------------------------=
    // FUNC: template
    // DESC: Resolves a saved layer's explicit appearance over its shared native component recipe.
    // ------------------=
    static func template(for element: StudioElement, configuration: Bool, styles: [UIKitSurfaceStyle]) -> UIKitSurfaceStyle {
        let kind = InfinityUIKit.precisionKind(role: element.role, kind: element.kind, configuration: configuration)
        var result = resolved(for: kind, styles: styles)
            .template(material: element.surfaceMaterial, corners: element.surfaceCorners, radius: element.cornerRadius,
                inheritColors: kind == .primary || kind == .secondary)
        if kind != .primary && kind != .secondary { result.fill = nil; result.border = nil }
        return result
    }
}

/// Saved compositor families use the same shared atlas as the OS.
enum WindowFramePreset: Int, CaseIterable, Identifiable {
    case family0 = 15
    case family1 = 16
    case family2 = 17
    case family3 = 18
    case family4 = 19
    case family5 = 20
    case family6 = 21
    case family7 = 22
    case family8 = 23
    case family9 = 24
    var id: Int { rawValue }
    var title: String { UIKitSurfaceMaterial(rawValue: rawValue)!.title }
    var style: UIKitSurfaceStyle { UIKitSurfaceStyle(material: UIKitSurfaceMaterial(rawValue: rawValue)!, corners: .square) }
}
