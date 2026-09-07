import AppKit
import SwiftUI

struct InstallerCanvas: View {
    @ObservedObject var store: TemplateStore
    @State private var marqueeDisplayRect: CGRect?
    @State private var canvasDragMode: CanvasDragMode?

    var body: some View {
        GeometryReader { proxy in
            ZStack {
                ScrollView([.horizontal, .vertical]) {
                    let fit = max(0.24, min(
                        (proxy.size.width - 80) / 1600,
                        (proxy.size.height - 80) / 1000
                    ))
                    let unitScale = fit * store.zoom
                    let canvasScale = CGSize(width: unitScale * 1.6, height: unitScale)
                    ZStack(alignment: .topLeading) {
                        artboardBackground(scale: canvasScale)
                            .contentShape(Rectangle())
                            .onTapGesture {
                                if !store.marqueeSelectionEnabled { store.selectElement(nil) }
                            }
                        if store.showGrid {
                            SnapGrid(gridSize: store.gridSize, scale: canvasScale)
                        }
                        ForEach((store.selectedScreen?.elements ?? []).sorted(by: layerOrder)) { element in
                            if !element.hidden {
                                CanvasElementView(element: element, store: store, canvasScale: canvasScale)
                                    .zIndex(Double(element.zIndex))
                            }
                        }
                        if store.selectedCollection == .configuration,
                           store.selectedScreenID == 7,
                           let content = store.selectedScreen?.elements.first(where: { $0.role == .content && !$0.hidden })
                        {
                            ConfigurationNetworkPreview(canvasScale: canvasScale, content: content.frame, projectRoot: store.projectRoot)
                                .zIndex(7_500)
                        }
                        if store.marqueeSelectionEnabled {
                            Rectangle()
                                .fill(Color.clear)
                                .contentShape(Rectangle())
                                .gesture(marqueeInteractionGesture(canvasScale: canvasScale))
                                .zIndex(39_000)
                        }
                        if let marqueeDisplayRect {
                            RoundedRectangle(cornerRadius: 4)
                                .fill(InfinityUIKit.Palette.nativeAccent.opacity(0.13))
                                .overlay {
                                    RoundedRectangle(cornerRadius: 4)
                                        .stroke(
                                            InfinityUIKit.Palette.nativeAccentBright,
                                            style: StrokeStyle(lineWidth: 1.5, dash: [7, 4])
                                        )
                                }
                                .frame(width: marqueeDisplayRect.width, height: marqueeDisplayRect.height)
                                .position(x: marqueeDisplayRect.midX, y: marqueeDisplayRect.midY)
                                .allowsHitTesting(false)
                                .zIndex(40_000)
                        }
                        selectionHandleOverlay(canvasScale: canvasScale)
                            .zIndex(41_000)
                    }
                    .frame(width: 1000 * canvasScale.width, height: 1000 * canvasScale.height)
                    .coordinateSpace(name: CanvasInteractionMetrics.coordinateSpaceName)
                    .overlay {
                        RoundedRectangle(cornerRadius: 2)
                            .stroke(Color.white.opacity(0.24), lineWidth: 1)
                    }
                    .shadow(color: .black.opacity(0.65), radius: 28, y: 12)
                    .padding(32)
                }
                .background(
                    LinearGradient(
                        colors: [InfinityUIKit.Palette.nativePanelRaised, InfinityUIKit.Palette.nativeCanvas],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
            }
        }
    }

    // ------------------------=
    // FUNC: artboardBackground
    // DESC: Builds the InfinityOS installation-preview background at the current canvas scale.
    // ------------------=
    @ViewBuilder
    private func artboardBackground(scale: CGSize) -> some View {
        ZStack {
            InfinityUIKit.Palette.nativeCanvas
            RadialGradient(
                colors: [InfinityUIKit.Palette.nativeAccent.opacity(0.18), Color.clear],
                center: .top,
                startRadius: 10,
                endRadius: 520 * scale.height
            )
            LinearGradient(
                colors: [Color.clear, Color.blue.opacity(0.08), Color.black.opacity(0.5)],
                startPoint: .top,
                endPoint: .bottom
            )
        }
    }

    // ------------------------=
    // FUNC: layerOrder
    // DESC: Orders canvas elements deterministically by z-index and identifier.
    // ------------------=
    private func layerOrder(_ lhs: StudioElement, _ rhs: StudioElement) -> Bool {
        if lhs.zIndex != rhs.zIndex { return lhs.zIndex < rhs.zIndex }
        return lhs.id.uuidString < rhs.id.uuidString
    }

    // ------------------------=
    // FUNC: marqueeInteractionGesture
    // DESC: Provides point selection, whole-artboard rectangle selection, and selected-group dragging above canvas layers.
    // ------------------=
    private func marqueeInteractionGesture(canvasScale: CGSize) -> some Gesture {
        DragGesture(minimumDistance: 0, coordinateSpace: .named(CanvasInteractionMetrics.coordinateSpaceName))
            .onChanged { value in
                guard store.marqueeSelectionEnabled else { return }
                if canvasDragMode == nil {
                    let start = normalizedPoint(from: value.startLocation, canvasScale: canvasScale)
                    canvasDragMode = store.canMoveSelection(at: start) ? .moveSelection : .marquee
                    if canvasDragMode == .moveSelection {
                        store.beginGesture()
                    }
                }
                if canvasDragMode == .moveSelection {
                    store.moveSelected(translation: value.translation, canvasScale: canvasScale)
                } else {
                    marqueeDisplayRect = clippedDisplayRect(
                        from: value.startLocation,
                        to: value.location,
                        canvasScale: canvasScale
                    )
                }
            }
            .onEnded { value in
                defer {
                    marqueeDisplayRect = nil
                    canvasDragMode = nil
                }
                let distance = max(abs(value.translation.width), abs(value.translation.height))
                guard store.marqueeSelectionEnabled else { return }
                if distance < CanvasInteractionMetrics.dragThreshold {
                    if canvasDragMode == .moveSelection { store.endGesture() }
                    store.activateCanvas(
                        at: normalizedPoint(from: value.location, canvasScale: canvasScale),
                        additive: NSEvent.modifierFlags.contains(.shift)
                    )
                    return
                }
                if canvasDragMode == .moveSelection {
                    store.endGesture()
                    return
                }
                guard let displayRect = clippedDisplayRect(
                    from: value.startLocation,
                    to: value.location,
                    canvasScale: canvasScale
                ),
                      displayRect.width > 0,
                      displayRect.height > 0
                else { return }
                store.selectElements(
                    in: normalizedRect(from: displayRect, canvasScale: canvasScale),
                    additive: NSEvent.modifierFlags.contains(.shift)
                )
            }
    }

    // ------------------------=
    // FUNC: selectionHandleOverlay
    // DESC: Places resize handles above the whole-artboard Marquee interaction surface for one unlocked selection.
    // ------------------=
    @ViewBuilder
    private func selectionHandleOverlay(canvasScale: CGSize) -> some View {
        if store.resizeHandlesVisible, let element = store.selectedElement {
            ZStack {
                ForEach(ResizeHandle.allCases) { handle in
                    ResizeHandleView(
                        handle: handle,
                        elementID: element.id,
                        elementFrame: element.frame,
                        store: store,
                        canvasScale: canvasScale
                    )
                }
            }
            .frame(
                width: CGFloat(element.frame.width) * canvasScale.width,
                height: CGFloat(element.frame.height) * canvasScale.height
            )
            .position(
                x: CGFloat(element.frame.x) * canvasScale.width
                    + CGFloat(element.frame.width) * canvasScale.width / 2,
                y: CGFloat(element.frame.y) * canvasScale.height
                    + CGFloat(element.frame.height) * canvasScale.height / 2
            )
        }
    }

    // ------------------------=
    // FUNC: clippedDisplayRect
    // DESC: Standardizes a pointer rectangle and clips it to the currently scaled artboard.
    // ------------------=
    private func clippedDisplayRect(from start: CGPoint, to end: CGPoint, canvasScale: CGSize) -> CGRect? {
        let pointerRect = CGRect(
            x: min(start.x, end.x),
            y: min(start.y, end.y),
            width: abs(end.x - start.x),
            height: abs(end.y - start.y)
        )
        let artboard = CGRect(x: 0, y: 0, width: 1000 * canvasScale.width, height: 1000 * canvasScale.height)
        let clipped = pointerRect.intersection(artboard)
        return clipped.isNull ? nil : clipped
    }

    // ------------------------=
    // FUNC: normalizedRect
    // DESC: Converts a displayed marquee into the integer coordinate system persisted by Studio templates.
    // ------------------=
    private func normalizedRect(from displayRect: CGRect, canvasScale: CGSize) -> CanvasRect {
        let left = Int(floor(displayRect.minX / canvasScale.width))
        let top = Int(floor(displayRect.minY / canvasScale.height))
        let right = Int(ceil(displayRect.maxX / canvasScale.width))
        let bottom = Int(ceil(displayRect.maxY / canvasScale.height))
        return CanvasRect(x: left, y: top, width: max(1, right - left), height: max(1, bottom - top))
    }

    // ------------------------=
    // FUNC: normalizedPoint
    // DESC: Converts a displayed pointer location into a clamped normalized artboard coordinate.
    // ------------------=
    private func normalizedPoint(from point: CGPoint, canvasScale: CGSize) -> CGPoint {
        CGPoint(
            x: (point.x / canvasScale.width).clamped(to: 0...1000),
            y: (point.y / canvasScale.height).clamped(to: 0...1000)
        )
    }
}

private enum CanvasDragMode: Equatable {
    case marquee
    case moveSelection
}

private struct SnapGrid: View {
    let gridSize: Int
    let scale: CGSize

    var body: some View {
        Canvas { context, size in
            let stepX = max(CGFloat(gridSize) * scale.width, 4)
            let stepY = max(CGFloat(gridSize) * scale.height, 4)
            var minorPath = Path()
            var majorPath = Path()
            var x: CGFloat = 0
            while x <= size.width {
                if Int((x / stepX).rounded()) % 5 == 0 {
                    majorPath.move(to: CGPoint(x: x, y: 0))
                    majorPath.addLine(to: CGPoint(x: x, y: size.height))
                } else {
                    minorPath.move(to: CGPoint(x: x, y: 0))
                    minorPath.addLine(to: CGPoint(x: x, y: size.height))
                }
                x += stepX
            }
            var y: CGFloat = 0
            while y <= size.height {
                if Int((y / stepY).rounded()) % 5 == 0 {
                    majorPath.move(to: CGPoint(x: 0, y: y))
                    majorPath.addLine(to: CGPoint(x: size.width, y: y))
                } else {
                    minorPath.move(to: CGPoint(x: 0, y: y))
                    minorPath.addLine(to: CGPoint(x: size.width, y: y))
                }
                y += stepY
            }
            context.stroke(minorPath, with: .color(InfinityUIKit.Palette.nativeAccent.opacity(0.06)), lineWidth: 0.5)
            context.stroke(majorPath, with: .color(InfinityUIKit.Palette.nativeAccent.opacity(0.15)), lineWidth: 0.8)
            var axes = Path()
            axes.move(to: CGPoint(x: size.width / 2, y: 0))
            axes.addLine(to: CGPoint(x: size.width / 2, y: size.height))
            axes.move(to: CGPoint(x: 0, y: size.height / 2))
            axes.addLine(to: CGPoint(x: size.width, y: size.height / 2))
            context.stroke(axes, with: .color(InfinityUIKit.Palette.nativeAccent.opacity(0.28)), lineWidth: 1)
        }
        .allowsHitTesting(false)
    }
}

struct CanvasElementView: View {
    let element: StudioElement
    @ObservedObject var store: TemplateStore
    let canvasScale: CGSize

    var isSelected: Bool { store.selectedElementIDs.contains(element.id) }

    var body: some View {
        ZStack {
            elementBody
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .opacity(Double(element.opacity) / 100)
                .contentShape(Rectangle())
                .highPriorityGesture(elementInteractionGesture)
            selectionOverlay
        }
        .frame(
            width: CGFloat(element.frame.width) * canvasScale.width,
            height: CGFloat(element.frame.height) * canvasScale.height
        )
            .position(
                x: CGFloat(element.frame.x) * canvasScale.width + CGFloat(element.frame.width) * canvasScale.width / 2,
                y: CGFloat(element.frame.y) * canvasScale.height + CGFloat(element.frame.height) * canvasScale.height / 2
            )
    }

    @ViewBuilder
    private var elementBody: some View {
        switch element.kind {
        case .panel:
            CanvasPanelPreview(
                element: element,
                canvasScale: canvasScale,
                focused: isSelected
            )
        case .console:
            CanvasPanelPreview(
                element: element,
                canvasScale: canvasScale,
                focused: isSelected
            )
        case .text:
            Text(element.text)
                .font(.system(
                    size: max(8, CGFloat(element.fontSize) * canvasScale.height),
                    weight: element.role == .title ? .medium : .regular,
                    design: .rounded
                ))
                .tracking(element.role == .title || element.role == .sectionLabel
                    ? max(0.8, CGFloat(element.fontSize) * canvasScale.height * 0.12)
                    : 0)
                .foregroundStyle(element.fill.color)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                .multilineTextAlignment(.leading)
                .lineSpacing(max(1, 4 * canvasScale.height))
                .clipped()
        case .image:
            if element.role == .timeZoneMap {
                imagePreview
                    .overlay { TimeZoneMapSelectionPreview(canvasScale: canvasScale) }
                    .clipShape(RoundedRectangle(cornerRadius: CGFloat(element.cornerRadius) * canvasScale.height))
                    .overlay {
                        RoundedRectangle(cornerRadius: CGFloat(element.cornerRadius) * canvasScale.height)
                            .stroke(element.border.color, lineWidth: max(1, canvasScale.height))
                    }
                    .shadow(color: InfinityUIKit.Palette.nativeAccent.opacity(0.16), radius: 8 * canvasScale.height)
            } else {
                imagePreview
            }
        case .button:
            CanvasButtonPreview(
                element: element,
                canvasScale: canvasScale,
                focused: isSelected
            )
        case .progressBar:
            CanvasProgressBarPreview(element: element, canvasScale: canvasScale)
        }
    }

    @ViewBuilder
    private var imagePreview: some View {
        if let image = resolveImage() {
            Image(nsImage: croppedImage(image) ?? image)
                .resizable()
                .aspectRatio(contentMode: element.role.imageUsesAspectFill ? .fill : .fit)
                .clipped()
        } else {
            RoundedRectangle(cornerRadius: 8 * canvasScale.height)
                .fill(InfinityUIKit.Palette.nativeAccent.opacity(0.08))
                .overlay {
                    VStack(spacing: 5) {
                        Image(systemName: element.role == .masthead ? "infinity" : "photo")
                        Text(element.imageAsset.isEmpty ? "Choose Image" : element.imageAsset)
                            .lineLimit(1)
                    }
                    .font(.system(size: max(8, 16 * canvasScale.height)))
                    .foregroundStyle(InfinityUIKit.Palette.nativeAccent.opacity(0.8))
                }
        }
    }

    @ViewBuilder
    private var selectionOverlay: some View {
        if isSelected {
            ZStack {
                Rectangle()
                    .stroke(element.locked ? Color.orange : InfinityUIKit.Palette.nativeAccent, style: StrokeStyle(lineWidth: 2, dash: element.locked ? [5, 4] : []))
                    .allowsHitTesting(false)
                if element.locked {
                    VStack {
                        HStack {
                            Label("Locked", systemImage: "lock.fill")
                                .font(.caption2.weight(.semibold))
                                .padding(.horizontal, 6)
                                .padding(.vertical, 3)
                                .background(.orange, in: Capsule())
                                .foregroundStyle(.black)
                            Spacer()
                        }
                        Spacer()
                    }
                    .padding(4)
                    .allowsHitTesting(false)
                }
            }
        }
    }

    // ------------------------=
    // FUNC: elementInteractionGesture
    // DESC: Distinguishes a direct selection click from a canvas move without competing recognizers.
    // ------------------=
    private var elementInteractionGesture: some Gesture {
        DragGesture(minimumDistance: 0, coordinateSpace: .named(CanvasInteractionMetrics.coordinateSpaceName))
            .onChanged { value in
                guard !element.locked else { return }
                let distance = max(abs(value.translation.width), abs(value.translation.height))
                guard distance >= CanvasInteractionMetrics.dragThreshold else { return }
                store.beginGesture(elementID: element.id)
                store.moveSelected(translation: value.translation, canvasScale: canvasScale)
            }
            .onEnded { value in
                let distance = max(abs(value.translation.width), abs(value.translation.height))
                if distance < CanvasInteractionMetrics.dragThreshold {
                    store.activateCanvasElement(element.id)
                } else {
                    store.endGesture()
                }
            }
    }

    // ------------------------=
    // FUNC: resolveImage
    // DESC: Resolves a template image from the repository, absolute path, or app resource bundle.
    // ------------------=
    private func resolveImage() -> NSImage? {
        guard !element.imageAsset.isEmpty else { return nil }
        if element.imageAsset.hasPrefix("/") {
            return NSImage(contentsOfFile: element.imageAsset)
        }
        if let root = store.projectRoot {
            let paths = [
                root.appending(path: "assets/boot/\(element.imageAsset)"),
                root.appending(path: element.imageAsset),
            ]
            for path in paths where FileManager.default.fileExists(atPath: path.path) {
                return NSImage(contentsOf: path)
            }
        }
        return Bundle.module.url(forResource: element.imageAsset, withExtension: nil).flatMap(NSImage.init(contentsOf:))
    }

    // ------------------------=
    // FUNC: croppedImage
    // DESC: Produces a preview bitmap from the element's persisted normalized crop rectangle.
    // ------------------=
    private func croppedImage(_ image: NSImage) -> NSImage? {
        let crop = element.crop.clamped()
        guard crop != .none,
              let source = image.cgImage(forProposedRect: nil, context: nil, hints: nil)
        else { return image }
        let width = CGFloat(source.width)
        let height = CGFloat(source.height)
        let rect = CGRect(
            x: width * CGFloat(crop.left) / 100,
            y: height * CGFloat(crop.bottom) / 100,
            width: width * CGFloat(100 - crop.left - crop.right) / 100,
            height: height * CGFloat(100 - crop.top - crop.bottom) / 100
        ).integral
        guard rect.width >= 1, rect.height >= 1, let result = source.cropping(to: rect) else { return image }
        return NSImage(cgImage: result, size: NSSize(width: rect.width, height: rect.height))
    }
}

private struct CanvasProgressBarPreview: View {
    let element: StudioElement
    let canvasScale: CGSize

    var body: some View {
        VStack(spacing: max(5, 14 * canvasScale.height)) {
            HStack {
                Text(element.text)
                    .font(.system(size: max(8, 18 * canvasScale.height), weight: .semibold, design: .rounded))
                Spacer()
                Text("42%")
                    .font(.system(size: max(8, 18 * canvasScale.height), weight: .semibold, design: .rounded))
                    .foregroundStyle(element.fill.color)
            }
            GeometryReader { geometry in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: max(2, 5 * canvasScale.height))
                        .fill(InfinityUIKit.Palette.nativeCanvas.opacity(0.78))
                    RoundedRectangle(cornerRadius: max(2, 5 * canvasScale.height))
                        .fill(element.fill.color)
                        .frame(width: geometry.size.width * 0.42)
                }
                .overlay {
                    RoundedRectangle(cornerRadius: max(2, 5 * canvasScale.height))
                        .stroke(element.border.color, lineWidth: max(1, canvasScale.height))
                }
            }
            .frame(height: max(8, 16 * canvasScale.height))
        }
        .foregroundStyle(InfinityUIKit.Palette.nativeTextPrimary)
        .padding(max(8, 18 * canvasScale.height))
        .background(
            InfinityUIKit.Palette.nativePanelRaised,
            in: RoundedRectangle(cornerRadius: CGFloat(element.cornerRadius) * canvasScale.height)
        )
        .overlay {
            RoundedRectangle(cornerRadius: CGFloat(element.cornerRadius) * canvasScale.height)
                .stroke(element.border.color, lineWidth: max(1, canvasScale.height))
        }
    }
}

private struct CanvasPanelPreview: View {
    let element: StudioElement
    let canvasScale: CGSize
    let focused: Bool
    @State private var hovered = false

    private var interactionState: InfinityUIKitInteractionState {
        focused ? .focused : (hovered ? .hover : .idle)
    }

    private var recipe: InfinityUIKitVisualRecipe {
        InfinityUIKit.visualRecipe(
            role: element.role,
            state: interactionState,
            authoredFill: element.fill,
            authoredBorder: element.border
        )
    }

    private var isField: Bool {
        [.input, .dateField, .timeField, .timeZoneSelector, .offsetBadge].contains(element.role)
    }

    private var trailingIcon: String? {
        switch element.role {
        case .dateField: "calendar"
        case .timeField: "clock"
        case .timeZoneSelector: "chevron.down"
        default: nil
        }
    }

    var body: some View {
        let radius = CGFloat(element.cornerRadius) * canvasScale.height
        if element.role == .progressSegment {
            RoundedRectangle(cornerRadius: radius)
                .fill(element.fill.color)
                .frame(height: max(2, 4 * canvasScale.height))
        } else {
            RoundedRectangle(cornerRadius: radius)
                .fill(LinearGradient(
                    colors: [recipe.fillTop.color, recipe.fillBottom.color],
                    startPoint: .top,
                    endPoint: .bottom
                ))
                .overlay(alignment: .top) {
                    RoundedRectangle(cornerRadius: radius)
                        .stroke(Color.white.opacity(recipe.highlightOpacity * 0.28), lineWidth: max(0.6, canvasScale.height))
                        .padding(max(1, canvasScale.height))
                        .mask(LinearGradient(colors: [.white, .clear], startPoint: .top, endPoint: .center))
                }
                .overlay {
                    RoundedRectangle(cornerRadius: radius)
                        .stroke(recipe.border.color, lineWidth: max(recipe.borderWidth, canvasScale.height))
                }
                .overlay(alignment: .leading) {
                    if isField, !element.text.isEmpty {
                        HStack(spacing: 8 * canvasScale.height) {
                            Text(element.text)
                                .font(.system(
                                    size: max(8, CGFloat(element.fontSize) * canvasScale.height),
                                    weight: .regular,
                                    design: .rounded
                                ))
                                .tracking(element.role == .offsetBadge ? max(0.5, canvasScale.height) : 0)
                                .foregroundStyle(element.role == .input
                                    ? InfinityUIKit.Palette.placeholder.color
                                    : recipe.text.color)
                                .lineLimit(1)
                            Spacer(minLength: 4)
                            if let trailingIcon {
                                Image(systemName: trailingIcon)
                                    .font(.system(size: max(8, 15 * canvasScale.height), weight: .light))
                                    .foregroundStyle(InfinityUIKit.Palette.nativeAccentBright)
                            }
                        }
                        .padding(.horizontal, max(7, 16 * canvasScale.height))
                    }
                }
                .shadow(
                    color: InfinityUIKit.Palette.nativeAccent.opacity(recipe.glowOpacity),
                    radius: 10 * canvasScale.height
                )
                .onHover { hovered = $0 }
        }
    }
}

private struct CanvasButtonPreview: View {
    let element: StudioElement
    let canvasScale: CGSize
    let focused: Bool
    @State private var hovered = false

    private var interactionState: InfinityUIKitInteractionState {
        focused ? .focused : (hovered ? .hover : .idle)
    }

    private var recipe: InfinityUIKitVisualRecipe {
        InfinityUIKit.visualRecipe(
            role: element.role,
            state: interactionState,
            authoredFill: element.fill,
            authoredBorder: element.border
        )
    }

    var body: some View {
        let radius = CGFloat(element.cornerRadius) * canvasScale.height
        RoundedRectangle(cornerRadius: radius)
            .fill(LinearGradient(
                colors: [recipe.fillTop.color, recipe.fillBottom.color],
                startPoint: .top,
                endPoint: .bottom
            ))
            .overlay(alignment: .top) {
                RoundedRectangle(cornerRadius: radius)
                    .fill(LinearGradient(
                        colors: [Color.white.opacity(recipe.highlightOpacity * 0.20), .clear],
                        startPoint: .top,
                        endPoint: .bottom
                    ))
                    .frame(maxHeight: .infinity)
                    .padding(max(1, canvasScale.height))
                    .mask(Rectangle().frame(maxHeight: .infinity, alignment: .top))
            }
            .overlay {
                RoundedRectangle(cornerRadius: radius)
                    .stroke(recipe.border.color, lineWidth: max(recipe.borderWidth, canvasScale.height))
            }
            .overlay {
                Text(element.text)
                    .font(.system(
                        size: max(8, CGFloat(element.fontSize) * canvasScale.height),
                        weight: .medium,
                        design: .rounded
                    ))
                    .tracking(max(0.7, CGFloat(element.fontSize) * canvasScale.height * 0.10))
                    .foregroundStyle(recipe.text.color)
                    .lineLimit(1)
                    .padding(.horizontal, max(8, 14 * canvasScale.height))
            }
            .shadow(
                color: InfinityUIKit.Palette.nativeAccent.opacity(recipe.glowOpacity),
                radius: 11 * canvasScale.height
            )
            .onHover { hovered = $0 }
    }
}

private struct TimeZoneMapSelectionPreview: View {
    let canvasScale: CGSize

    var body: some View {
        GeometryReader { proxy in
            let bandWidth = max(4, proxy.size.width / 24)
            let marker = CGPoint(x: proxy.size.width * 0.235, y: proxy.size.height * 0.29)
            ZStack(alignment: .topLeading) {
                Rectangle()
                    .fill(InfinityUIKit.Palette.nativeAccent.opacity(0.20))
                    .frame(width: bandWidth)
                    .overlay {
                        Rectangle().stroke(InfinityUIKit.Palette.nativeAccent.opacity(0.72), lineWidth: max(1, canvasScale.height))
                    }
                    .offset(x: marker.x - bandWidth / 2)
                Circle()
                    .fill(InfinityUIKit.Palette.nativeAccentBright)
                    .frame(width: max(8, 13 * canvasScale.height), height: max(8, 13 * canvasScale.height))
                    .overlay(Circle().stroke(.white, lineWidth: max(1, canvasScale.height)))
                    .shadow(color: InfinityUIKit.Palette.nativeAccent, radius: 8 * canvasScale.height)
                    .position(marker)
            }
        }
        .allowsHitTesting(false)
    }
}

struct ConfigurationNetworkPreview: View {
    let canvasScale: CGSize
    let content: CanvasRect
    let projectRoot: URL?
    private let labels = ["Wired network", "Wi-Fi", "Continue offline"]
    private let details = ["Wired adapter detected", "No wireless adapter detected", "Set up networking later in Settings"]
    private let icons = ["actions/01-forward.png", "actions/02-up.png", "base/26-settings.png"]

    var body: some View {
        ZStack(alignment: .topLeading) {
            ForEach(Array(labels.enumerated()), id: \.offset) { index, label in
                let frame = ConfigurationNetworkLayout.row(in: content, index: index)
                RoundedRectangle(cornerRadius: CGFloat(10) * canvasScale.height)
                    .fill(Color(red: 5 / 255, green: 20 / 255, blue: 34 / 255))
                    .overlay {
                        RoundedRectangle(cornerRadius: CGFloat(10) * canvasScale.height)
                            .stroke(
                                index == 0
                                    ? Color(red: 55 / 255, green: 194 / 255, blue: 238 / 255)
                                    : Color(red: 31 / 255, green: 74 / 255, blue: 98 / 255),
                                lineWidth: max(1, canvasScale.height)
                            )
                    }
                    .overlay(alignment: .leading) {
                        HStack(spacing: CGFloat(10) * canvasScale.height) {
                            if let root = projectRoot,
                               let icon = NSImage(contentsOf: root.appending(path: "assets/icons/crystal-blue-glass/32/\(icons[index])")) {
                                Image(nsImage: icon).resizable().scaledToFit()
                                    .frame(width: 20 * canvasScale.height, height: 20 * canvasScale.height)
                            }
                            VStack(alignment: .leading, spacing: 5 * canvasScale.height) {
                                Text(label).font(.system(size: max(6, 14 * canvasScale.height), weight: .semibold))
                                Text(details[index]).font(.system(size: max(6, 14 * canvasScale.height)))
                                    .foregroundStyle(Color(red: 133 / 255, green: 157 / 255, blue: 177 / 255))
                            }
                            .foregroundStyle(InfinityUIKit.Palette.nativeTextPrimary)
                            Spacer(minLength: 0)
                            if index == 0 {
                                Text("SELECTED")
                                    .font(.system(size: max(6, 14 * canvasScale.height)))
                                    .foregroundStyle(InfinityUIKit.Palette.nativeAccent)
                            }
                        }
                        .padding(.horizontal, CGFloat(12) * canvasScale.height)
                    }
                    .frame(
                        width: CGFloat(frame.width) * canvasScale.width,
                        height: CGFloat(frame.height) * canvasScale.height
                    )
                    .position(
                        x: CGFloat(frame.x) * canvasScale.width
                            + CGFloat(frame.width) * canvasScale.width / 2,
                        y: CGFloat(frame.y) * canvasScale.height
                            + CGFloat(frame.height) * canvasScale.height / 2
                    )
            }
        }
        .allowsHitTesting(false)
    }
}

private struct ResizeHandleView: View {
    let handle: ResizeHandle
    let elementID: UUID
    let elementFrame: CanvasRect
    @ObservedObject var store: TemplateStore
    let canvasScale: CGSize

    var body: some View {
        ZStack {
            Rectangle()
                .fill(Color.clear)
            Circle()
                .fill(Color.white)
                .overlay(Circle().stroke(InfinityUIKit.Palette.nativeAccent, lineWidth: 2))
                .frame(
                    width: CanvasInteractionMetrics.resizeHandleVisualSize,
                    height: CanvasInteractionMetrics.resizeHandleVisualSize
                )
                .allowsHitTesting(false)
        }
        .frame(
            width: CanvasInteractionMetrics.resizeHandleHitSize,
            height: CanvasInteractionMetrics.resizeHandleHitSize
        )
        .contentShape(Rectangle())
        .position(handlePosition)
        .highPriorityGesture(
            DragGesture(minimumDistance: 0, coordinateSpace: .named(CanvasInteractionMetrics.coordinateSpaceName))
                .onChanged { value in
                    store.beginGesture(elementID: elementID)
                    store.resizeSelected(handle: handle, translation: value.translation, canvasScale: canvasScale)
                }
                .onEnded { _ in store.endGesture() }
        )
        .accessibilityLabel("Resize \(handle.accessibilityName)")
        .accessibilityHint("Drag to resize the selected element")
    }

    private var handlePosition: CGPoint {
        let width = CGFloat(elementFrame.width) * canvasScale.width
        let height = CGFloat(elementFrame.height) * canvasScale.height
        let insetX = min(CanvasInteractionMetrics.resizeHandleVisualSize / 2, width / 2)
        let insetY = min(CanvasInteractionMetrics.resizeHandleVisualSize / 2, height / 2)
        return switch handle {
        case .topLeft: CGPoint(x: insetX, y: insetY)
        case .top: CGPoint(x: width / 2, y: insetY)
        case .topRight: CGPoint(x: width - insetX, y: insetY)
        case .right: CGPoint(x: width - insetX, y: height / 2)
        case .bottomRight: CGPoint(x: width - insetX, y: height - insetY)
        case .bottom: CGPoint(x: width / 2, y: height - insetY)
        case .bottomLeft: CGPoint(x: insetX, y: height - insetY)
        case .left: CGPoint(x: insetX, y: height / 2)
        }
    }
}

enum CanvasInteractionMetrics {
    static let coordinateSpaceName = "installer-canvas"
    static let resizeHandleVisualSize: CGFloat = 12
    static let resizeHandleHitSize: CGFloat = 30
    static let dragThreshold: CGFloat = 2
}
