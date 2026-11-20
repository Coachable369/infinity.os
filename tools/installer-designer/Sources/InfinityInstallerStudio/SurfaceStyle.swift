import AppKit
import SwiftUI

enum GlassButtonPreset: String, CaseIterable, Identifiable {
    case sapphire = "Classic Sapphire Glass", smoked = "Smoked Glass", ice = "Frosted Ice"
    case satin = "Satin Blue", minimal = "Minimal Blue"
    case graphite = "Graphite Matte"
    var id: String { rawValue }
    var style: UIKitSurfaceStyle {
        let fill: UInt32 = switch self {
        case .sapphire: 0xE8204DAD
        case .smoked: 0xD8101B3A
        case .ice: 0xC8447394
        case .satin: 0xFF3159A6
        case .minimal: 0xFF245CD8
        case .graphite: 0xFF293747
        }
        let material: UIKitSurfaceMaterial = (self == .satin || self == .graphite) ? .matte : self == .minimal ? .flat : .glass
        let radius = self == .satin ? 8 : self == .minimal ? 6 : 10
        return UIKitSurfaceStyle(material: material, corners: .rounded, radius: radius,
            fill: fill, border: 0xFF86C7FF)
    }
    var detail: String {
        switch self {
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
        element.fill = studioColor(recipe.fill!)
        element.border = studioColor(recipe.border!)
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
                && element.cornerRadius == style.radius && element.fill == studioColor(style.fill!)
                && element.border == studioColor(style.border!) && element.fillToken == nil && element.borderToken == nil
        }
    }
}

enum UIKitSurfaceMaterial: Int, Codable, CaseIterable, Identifiable, Hashable {
    case authored = 0, glass = 1, matte = 2, flat = 3
    var id: Int { rawValue }
    var title: String {
        switch self { case .authored: "Kit default"; case .glass: "Glass"; case .matte: "Matte"; case .flat: "Flat" }
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
