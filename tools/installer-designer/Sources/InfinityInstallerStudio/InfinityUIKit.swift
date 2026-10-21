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
        // FUNC: progressDiameter
        // DESC: Uses the runtime's circular progress footprint without stretching a wide authored layer.
        // ------------------=
        static func progressDiameter(_ size: CGSize) -> CGFloat {
            max(0, min(size.width, size.height))
        }

        // ------------------------=
        // FUNC: glyph
        // DESC: Loads the exact native 64px RGBA cell so configuration previews reuse the installed glyph coverage and colors.
        // ------------------=
        static func glyph(_ index: Int, projectRoot: URL?, atlasData: Data? = nil) -> NSImage? {
            let source = projectRoot?.appending(path: "assets/ui-design-kit/default/eclipse-access-glyphs-v1.rgba")
            guard (0..<16).contains(index),
                  let atlas = atlasData ?? source.flatMap({ try? Data(contentsOf: $0) }),
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

extension InfinityUIKit {
    // ------------------------=
    // FUNC: precisionKind
    // DESC: Resolves saved template roles to the same semantic atlas surfaces used by the native compositor.
    // ------------------=
    static func precisionKind(role: StudioElementRole, kind: StudioElementKind = .panel,
                              configuration: Bool = false, current: Bool = true) -> PrecisionSurfaceKind {
        if configuration && role == .progressSegment { return current ? .current : .inactive }
        if kind == .button { return role == .primaryButton ? .primary : .secondary }
        if role == .input || (!configuration && [.dateField, .timeField, .timeZoneSelector, .offsetBadge].contains(role)) {
            return .field
        }
        return .panel
    }

    // ------------------------=
    // FUNC: precisionMaterial
    // DESC: Preserves authored color and alpha while selecting native focus and pressure states, without procedural recoloring.
    // ------------------=
    static func precisionMaterial(state: InfinityUIKitInteractionState, fill: StudioColor,
                                  border: StudioColor, scale: Int = 1) -> PrecisionMaterial {
        PrecisionMaterial(fill: fill, edge: border, focus: state == .idle ? 0 : 255,
                          pressed: state == .pressed, scale: scale)
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
