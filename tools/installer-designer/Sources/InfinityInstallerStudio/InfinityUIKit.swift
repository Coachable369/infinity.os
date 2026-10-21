import SwiftUI
import CoreText

enum InfinityUIKit {
    enum Metrics {
        static let detailRadius: CGFloat = 4
        static let controlRadius: CGFloat = 10
        static let panelRadius: CGFloat = 16
        static let heroRadius: CGFloat = 24
        static let compactSpacing: CGFloat = 8
        static let controlGap: CGFloat = 12
        static let gutter: CGFloat = 16
        static let panelInset: CGFloat = 32
        static let toolbarHeight: CGFloat = 64
        static let studioControlHeight: CGFloat = 32
        static let statusBarHeight: CGFloat = 34

        static let configurationCard = CanvasRect(x: 40, y: 128, width: 340, height: 736)
        static let configurationContent = CanvasRect(x: 61, y: 222, width: 298, height: 504)
        static let configurationTitle = CanvasRect(x: 72, y: 270, width: 276, height: 44)
        static let configurationBody = CanvasRect(x: 72, y: 326, width: 276, height: 82)
        static let configurationInput = CanvasRect(x: 72, y: 442, width: 276, height: 47)
        static let configurationBack = CanvasRect(x: 61, y: 789, width: 87, height: 47)
        static let configurationPrimary = CanvasRect(x: 156, y: 789, width: 203, height: 47)
    }

    enum Palette {
        static let canvas = StudioColor(red: 9, green: 15, blue: 38, alpha: 255)
        static let panel = StudioColor(red: 16, green: 27, blue: 58, alpha: 230)
        static let panelRaised = StudioColor(red: 16, green: 27, blue: 58, alpha: 240)
        static let border = StudioColor(red: 134, green: 199, blue: 255, alpha: 255)
        static let textPrimary = StudioColor(red: 241, green: 245, blue: 250, alpha: 255)
        static let textSecondary = StudioColor(red: 174, green: 184, blue: 198, alpha: 255)
        static let accent = StudioColor(red: 32, green: 191, blue: 255, alpha: 255)
        static let accentBright = StudioColor(red: 156, green: 232, blue: 255, alpha: 255)
        static let focus = StudioColor(red: 255, green: 255, blue: 255, alpha: 255)
        static let primaryAction = StudioColor(red: 62, green: 111, blue: 255, alpha: 255)
        static let primaryActionBorder = StudioColor(red: 134, green: 199, blue: 255, alpha: 255)
        static let secondaryAction = StudioColor(red: 9, green: 15, blue: 38, alpha: 255)
        static let secondaryActionBorder = StudioColor(red: 134, green: 199, blue: 255, alpha: 255)
        static let field = StudioColor(red: 9, green: 15, blue: 38, alpha: 255)
        static let fieldBorder = StudioColor(red: 134, green: 199, blue: 255, alpha: 255)
        static let placeholder = StudioColor(red: 119, green: 133, blue: 149, alpha: 255)

        static let nativeCanvas = canvas.color
        static let nativePanel = panel.color
        static let nativePanelRaised = panelRaised.color
        static let nativeBorder = border.color
        static let nativeTextPrimary = textPrimary.color
        static let nativeTextSecondary = textSecondary.color
        static let nativeAccent = accent.color
        static let nativeAccentBright = accentBright.color
    }

    enum EclipseAccess {
        static let background = StudioColor(red: 9, green: 15, blue: 38, alpha: 255)
        static let surface = StudioColor(red: 16, green: 27, blue: 58, alpha: 255)
        static let elevated = StudioColor(red: 26, green: 46, blue: 99, alpha: 255)
        static let primary = StudioColor(red: 62, green: 111, blue: 255, alpha: 255)
        static let border = StudioColor(red: 134, green: 199, blue: 255, alpha: 255)
        static let mutedBorder = StudioColor(red: 67, green: 88, blue: 140, alpha: 255)
        static let focus = StudioColor(red: 255, green: 255, blue: 255, alpha: 255)
        static let text = StudioColor(red: 255, green: 255, blue: 255, alpha: 255)
        static let secondary = StudioColor(red: 190, green: 190, blue: 190, alpha: 255)
        static let focusAnimation = Animation.timingCurve(1 / 3, 2 / 3, 2 / 3, 1, duration: 0.14)
        @MainActor private static var fontDescriptors: [URL: CTFontDescriptor] = [:]

        // ------------------------=
        // FUNC: nativeFont
        // DESC: Loads the same variable Inter asset as native FontKit and binds the regular or semibold weight without changing other Studio typography.
        // ------------------=
        @MainActor static func nativeFont(size: CGFloat, semibold: Bool, projectRoot: URL?) -> CTFont? {
            guard let projectRoot else { return nil }
            let url = projectRoot.appending(path: "assets/fonts/Inter-Variable.ttf").standardizedFileURL
            let descriptor: CTFontDescriptor
            if let cached = fontDescriptors[url] {
                descriptor = cached
            } else {
                guard let descriptors = CTFontManagerCreateFontDescriptorsFromURL(url as CFURL) as? [CTFontDescriptor],
                      let loaded = descriptors.first else { return nil }
                CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
                fontDescriptors[url] = loaded
                descriptor = loaded
            }
            // FontKit rasterizes the named Regular/SemiBold instances at their
            // default optical size. Do not let host size-dependent optical
            // shaping change preview geometry as the user zooms the canvas.
            let variation = [NSNumber(value: 0x77676874): NSNumber(value: semibold ? 600 : 400),
                             NSNumber(value: 0x6f70737a): NSNumber(value: 14)]
            let weighted = CTFontDescriptorCreateCopyWithAttributes(descriptor,
                [kCTFontVariationAttribute: variation] as CFDictionary)
            return CTFontCreateWithFontDescriptor(weighted, max(1, size), nil)
        }

        // ------------------------=
        // FUNC: font
        // DESC: Provides Eclipse-only SwiftUI typography from the registered project asset rather than the host system face.
        // ------------------=
        @MainActor static func font(size: CGFloat, semibold: Bool = false, projectRoot: URL?) -> Font {
            if let loaded = nativeFont(size: size, semibold: semibold, projectRoot: projectRoot) {
                return Font(loaded)
            }
            return .custom("Inter", size: max(1, size))
        }

        // ------------------------=
        // FUNC: matches
        // DESC: Selects only authored Eclipse surface colors, irrespective of their independently editable opacity.
        // ------------------=
        static func matches(_ fill: StudioColor) -> Bool {
            [background, surface, elevated, primary].contains { $0.sameRGB(as: fill) }
        }

        // ------------------------=
        // FUNC: progressDiameter
        // DESC: Uses the runtime's left-aligned circular progress footprint without stretching a wide authored layer.
        // ------------------=
        static func progressDiameter(_ size: CGSize) -> CGFloat {
            max(0, min(size.width, size.height))
        }

        // ------------------------=
        // FUNC: glyph
        // DESC: Loads the exact native 64px RGBA cell so configuration previews reuse the installed glyph coverage and colors.
        // ------------------=
        static func glyph(_ index: Int, projectRoot: URL?) -> NSImage? {
            guard (0..<16).contains(index), let projectRoot,
                  let atlas = try? Data(contentsOf: projectRoot.appending(path: "assets/ui-design-kit/default/eclipse-access-glyphs-v1.rgba")),
                  atlas.count == 16 * 64 * 64 * 4
            else { return nil }
            let cellSize = 64 * 64 * 4
            let bytes = atlas.subdata(in: index * cellSize..<(index + 1) * cellSize)
            guard let provider = CGDataProvider(data: bytes as CFData),
                  let image = CGImage(width: 64, height: 64, bitsPerComponent: 8, bitsPerPixel: 32,
                    bytesPerRow: 256, space: CGColorSpaceCreateDeviceRGB(),
                    bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue),
                    provider: provider, decode: nil, shouldInterpolate: true, intent: .defaultIntent)
            else { return nil }
            return NSImage(cgImage: image, size: NSSize(width: 64, height: 64))
        }
    }
}

enum InfinityStudioButtonEmphasis {
    case secondary
    case primary
    case quiet
}

enum InfinityUIKitInteractionState: Equatable {
    case idle
    case hover
    case focused
    case pressed
}

struct InfinityUIKitVisualRecipe: Equatable {
    let fillTop: StudioColor
    let fillBottom: StudioColor
    let border: StudioColor
    let text: StudioColor
    let highlightOpacity: Double
    let glowOpacity: Double
    let borderWidth: CGFloat
    let eclipseAccess: Bool
}

extension InfinityUIKit {
    // ------------------------=
    // FUNC: visualRecipe
    // DESC: Resolves the shared UIKit glass, border, text, and glow recipe for a semantic canvas control state.
    // ------------------=
    static func visualRecipe(
        role: StudioElementRole,
        state: InfinityUIKitInteractionState,
        authoredFill: StudioColor,
        authoredBorder: StudioColor,
        configuration: Bool = false
    ) -> InfinityUIKitVisualRecipe {
        if configuration && EclipseAccess.matches(authoredFill) {
            return eclipseRecipe(state: state, fill: authoredFill, border: authoredBorder)
        }
        let active = state == .hover || state == .focused
        let pressed = state == .pressed
        let isPrimary = role == .primaryButton
        let isAction = isPrimary || role == .backButton
        let isField = [.input, .dateField, .timeField, .timeZoneSelector].contains(role)
        let isBadge = role == .offsetBadge

        let lift: (Int, Int, Int) = if isPrimary {
            active ? (30, 82, 104) : (17, 57, 75)
        } else if isAction {
            active ? (24, 55, 75) : (12, 25, 36)
        } else if isField || isBadge {
            active ? (14, 38, 52) : (7, 21, 28)
        } else {
            active ? (12, 27, 38) : (6, 14, 21)
        }
        let bottom = pressed ? authoredFill.adjusting(red: -3, green: -8, blue: -11) : authoredFill
        let border = if state == .focused {
            Palette.accentBright
        } else if state == .hover {
            Palette.accent
        } else {
            authoredBorder
        }
        return InfinityUIKitVisualRecipe(
            fillTop: bottom.adjusting(red: lift.0, green: lift.1, blue: lift.2),
            fillBottom: bottom,
            border: border,
            text: isBadge ? Palette.accentBright : Palette.textPrimary,
            highlightOpacity: pressed ? 0.22 : (active ? 0.58 : 0.34),
            glowOpacity: state == .focused ? 0.72 : (state == .hover ? 0.38 : (isPrimary ? 0.16 : 0)),
            borderWidth: state == .focused ? 1.5 : 1,
            eclipseAccess: false
        )
    }

    // ------------------------=
    // FUNC: eclipseRecipe
    // DESC: Mirrors native access_surface endpoints, pressed mixing and inward-only focus treatment without recoloring other collections.
    // ------------------=
    private static func eclipseRecipe(state: InfinityUIKitInteractionState, fill: StudioColor,
                                      border: StudioColor) -> InfinityUIKitVisualRecipe {
        let primary = fill.sameRGB(as: EclipseAccess.primary)
        let active = state != .idle && (fill.alpha != 0 || border.alpha != 0)
        var top = primary ? StudioColor(red: 49, green: 174, blue: 255, alpha: fill.alpha)
            : fill.mixed(with: EclipseAccess.border, amount: 20)
        var bottom = primary ? StudioColor(red: 22, green: 62, blue: 220, alpha: fill.alpha) : fill
        if primary && state == .pressed {
            top = top.mixed(with: EclipseAccess.elevated, amount: 56)
            bottom = bottom.mixed(with: EclipseAccess.elevated, amount: 56)
        }
        return InfinityUIKitVisualRecipe(fillTop: top, fillBottom: bottom,
            border: active ? EclipseAccess.focus : border, text: EclipseAccess.text,
            highlightOpacity: 0, glowOpacity: active ? 1 : 0, borderWidth: 1, eclipseAccess: true)
    }
}

extension StudioColor {
    // ------------------------=
    // FUNC: withAlpha
    // DESC: Keeps an authored palette color while changing only its independent opacity byte.
    // ------------------=
    func withAlpha(_ alpha: UInt8) -> StudioColor {
        StudioColor(red: red, green: green, blue: blue, alpha: alpha)
    }

    // ------------------------=
    // FUNC: sameRGB
    // DESC: Compares authored palette identity without conflating it with surface alpha.
    // ------------------=
    func sameRGB(as other: StudioColor) -> Bool {
        red == other.red && green == other.green && blue == other.blue
    }

    // ------------------------=
    // FUNC: mixed
    // DESC: Matches the native integer palette interpolation while retaining the authored surface opacity.
    // ------------------=
    func mixed(with other: StudioColor, amount: UInt8) -> StudioColor {
        let t = Int(amount)
        return StudioColor(red: UInt8((Int(red) * (255 - t) + Int(other.red) * t) / 255),
            green: UInt8((Int(green) * (255 - t) + Int(other.green) * t) / 255),
            blue: UInt8((Int(blue) * (255 - t) + Int(other.blue) * t) / 255), alpha: alpha)
    }

    // ------------------------=
    // FUNC: adjusting
    // DESC: Applies clamped byte-channel offsets used by both default and interactive glass control states.
    // ------------------=
    func adjusting(red: Int, green: Int, blue: Int) -> StudioColor {
        StudioColor(
            red: UInt8((Int(self.red) + red).clamped(to: 0...255)),
            green: UInt8((Int(self.green) + green).clamped(to: 0...255)),
            blue: UInt8((Int(self.blue) + blue).clamped(to: 0...255)),
            alpha: alpha
        )
    }
}

struct EclipseAccessSurface: View {
    let recipe: InfinityUIKitVisualRecipe
    let radius: CGFloat

    var body: some View {
        RoundedRectangle(cornerRadius: radius)
            .fill(LinearGradient(colors: [recipe.fillTop.color, recipe.fillBottom.color],
                                 startPoint: .top, endPoint: .bottom))
            .overlay {
                EclipseAccessFocusGlow(amount: recipe.glowOpacity * Double(recipe.fillTop.alpha) / 255)
                    .clipShape(RoundedRectangle(cornerRadius: radius))
            }
            .overlay {
                RoundedRectangle(cornerRadius: radius).strokeBorder(recipe.border.color, lineWidth: 1)
            }
    }
}

private struct EclipseAccessFocusGlow: View, Animatable {
    nonisolated var amount: Double

    nonisolated var animatableData: Double {
        get { amount }
        set { amount = newValue }
    }

    var body: some View {
        Canvas { context, size in
            // Native focus light is confined to the first five inward pixels.
            for distance in 1..<5 where size.width > CGFloat(distance * 2) && size.height > CGFloat(distance * 2) {
                let inset = CGFloat(distance) + 0.5
                let rect = CGRect(origin: .zero, size: size).insetBy(dx: inset, dy: inset)
                context.stroke(Path(rect), with: .color(InfinityUIKit.EclipseAccess.border.color
                    .opacity(Double(5 - distance) / 20 * amount)), lineWidth: 1)
            }
        }
    }
}

struct InfinityStudioButtonStyle: ButtonStyle {
    @Environment(\.isEnabled) private var isEnabled
    let emphasis: InfinityStudioButtonEmphasis

    // ------------------------=
    // FUNC: makeBody
    // DESC: Applies the shared InfinityOS control height, radius, palette, and pressed state.
    // ------------------=
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.callout.weight(.semibold))
            .foregroundStyle(foreground)
            .padding(.horizontal, emphasis == .quiet ? 7 : 11)
            .frame(minHeight: InfinityUIKit.Metrics.studioControlHeight)
            .background(background(configuration.isPressed), in: RoundedRectangle(
                cornerRadius: InfinityUIKit.Metrics.controlRadius
            ))
            .overlay {
                RoundedRectangle(cornerRadius: InfinityUIKit.Metrics.controlRadius)
                    .stroke(border, lineWidth: emphasis == .quiet ? 0 : 1)
            }
            .contentShape(RoundedRectangle(cornerRadius: InfinityUIKit.Metrics.controlRadius))
            .opacity(isEnabled ? 1 : 0.42)
    }

    private var foreground: Color {
        emphasis == .primary ? .white : InfinityUIKit.Palette.nativeTextPrimary
    }

    private var border: Color {
        emphasis == .primary
            ? InfinityUIKit.Palette.primaryActionBorder.color
            : InfinityUIKit.Palette.nativeBorder
    }

    // ------------------------=
    // FUNC: background
    // DESC: Resolves a semantic button surface without changing its hit geometry.
    // ------------------=
    private func background(_ pressed: Bool) -> Color {
        let base: Color
        switch emphasis {
        case .primary:
            base = InfinityUIKit.Palette.primaryAction.color
        case .secondary:
            base = InfinityUIKit.Palette.secondaryAction.color
        case .quiet:
            base = InfinityUIKit.Palette.nativePanelRaised.opacity(0.52)
        }
        return pressed ? base.opacity(0.72) : base
    }
}
