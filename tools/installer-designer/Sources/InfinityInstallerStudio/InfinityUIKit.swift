import SwiftUI

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
        static let canvas = StudioColor(red: 2, green: 7, blue: 15, alpha: 255)
        static let panel = StudioColor(red: 10, green: 18, blue: 29, alpha: 230)
        static let panelRaised = StudioColor(red: 18, green: 29, blue: 42, alpha: 240)
        static let border = StudioColor(red: 51, green: 71, blue: 91, alpha: 255)
        static let textPrimary = StudioColor(red: 241, green: 245, blue: 250, alpha: 255)
        static let textSecondary = StudioColor(red: 174, green: 184, blue: 198, alpha: 255)
        static let accent = StudioColor(red: 32, green: 191, blue: 255, alpha: 255)
        static let accentBright = StudioColor(red: 156, green: 232, blue: 255, alpha: 255)
        static let focus = StudioColor(red: 255, green: 255, blue: 255, alpha: 255)
        static let primaryAction = StudioColor(red: 8, green: 54, blue: 84, alpha: 255)
        static let primaryActionBorder = StudioColor(red: 40, green: 181, blue: 231, alpha: 255)
        static let secondaryAction = StudioColor(red: 5, green: 15, blue: 27, alpha: 255)
        static let secondaryActionBorder = StudioColor(red: 42, green: 69, blue: 91, alpha: 255)
        static let field = StudioColor(red: 2, green: 10, blue: 20, alpha: 255)
        static let fieldBorder = StudioColor(red: 40, green: 72, blue: 95, alpha: 255)
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
        authoredBorder: StudioColor
    ) -> InfinityUIKitVisualRecipe {
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
            borderWidth: state == .focused ? 1.5 : 1
        )
    }
}

extension StudioColor {
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
