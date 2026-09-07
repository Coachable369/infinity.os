import AppKit
import SwiftUI

struct InstallerCanvas: View {
    @ObservedObject var store: TemplateStore

    var body: some View {
        GeometryReader { proxy in
            ZStack(alignment: .topTrailing) {
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
                            .onTapGesture { store.selectElement(nil) }
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
                           store.selectedScreenID == 7
                        {
                            ConfigurationNetworkPreview(canvasScale: canvasScale)
                                .zIndex(7_500)
                        }
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
                if store.inlineEditorElementID != nil {
                    InlineElementEditor(store: store)
                        .padding(16)
                        .transition(.move(edge: .trailing).combined(with: .opacity))
                        .zIndex(50_000)
                }
            }
            .animation(.easeOut(duration: 0.16), value: store.inlineEditorElementID)
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

private struct CanvasElementView: View {
    let element: StudioElement
    @ObservedObject var store: TemplateStore
    let canvasScale: CGSize

    var isSelected: Bool { store.selectedElementID == element.id }

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
            RoundedRectangle(cornerRadius: CGFloat(element.cornerRadius) * canvasScale.height)
                .fill(element.fill.color)
                .overlay {
                    RoundedRectangle(cornerRadius: CGFloat(element.cornerRadius) * canvasScale.height)
                        .stroke(element.border.color, lineWidth: max(1, canvasScale.height))
                }
                .overlay(alignment: .leading) {
                    if element.role == .input, !element.text.isEmpty {
                        Text(element.text)
                            .font(.system(size: max(8, CGFloat(16) * canvasScale.height)))
                            .foregroundStyle(InfinityUIKit.Palette.placeholder.color)
                            .padding(.leading, CGFloat(16) * canvasScale.height)
                            .lineLimit(1)
                    }
                }
        case .console:
            RoundedRectangle(cornerRadius: CGFloat(element.cornerRadius) * canvasScale.height)
                .fill(element.fill.color)
                .overlay {
                    RoundedRectangle(cornerRadius: CGFloat(element.cornerRadius) * canvasScale.height)
                        .stroke(element.border.color, lineWidth: max(1, canvasScale.height))
                }
        case .text:
            Text(element.text)
                .font(.system(size: max(8, CGFloat(element.fontSize) * canvasScale.height), weight: element.role == .title ? .semibold : .regular))
                .foregroundStyle(element.fill.color)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                .multilineTextAlignment(.leading)
        case .image:
            imagePreview
        case .button:
            RoundedRectangle(cornerRadius: CGFloat(element.cornerRadius) * canvasScale.height)
                .fill(element.fill.color)
                .overlay {
                    RoundedRectangle(cornerRadius: CGFloat(element.cornerRadius) * canvasScale.height)
                        .stroke(element.border.color, lineWidth: max(1, canvasScale.height))
                }
                .overlay {
                    Text(element.text).fontWeight(.semibold)
                    .font(.system(size: max(8, CGFloat(element.fontSize) * canvasScale.height)))
                    .foregroundStyle(.white)
                }
        }
    }

    @ViewBuilder
    private var imagePreview: some View {
        if let image = resolveImage() {
            Image(nsImage: croppedImage(image) ?? image)
                .resizable()
                .aspectRatio(contentMode: .fill)
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
                } else {
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

private struct ConfigurationNetworkPreview: View {
    let canvasScale: CGSize
    private let labels = ["WIRED", "WI-FI", "CONTINUE OFFLINE"]

    var body: some View {
        ZStack(alignment: .topLeading) {
            ForEach(Array(labels.enumerated()), id: \.offset) { index, label in
                let frame = CanvasRect(x: 72, y: 434 + index * 58, width: 276, height: 46)
                RoundedRectangle(cornerRadius: CGFloat(10) * canvasScale.height)
                    .fill(InfinityUIKit.Palette.panelRaised.color)
                    .overlay {
                        RoundedRectangle(cornerRadius: CGFloat(10) * canvasScale.height)
                            .stroke(
                                index == 0
                                    ? InfinityUIKit.Palette.accent.color
                                    : InfinityUIKit.Palette.border.color,
                                lineWidth: max(1, canvasScale.height)
                            )
                    }
                    .overlay(alignment: .leading) {
                        HStack(spacing: CGFloat(10) * canvasScale.height) {
                            Circle()
                                .fill(index == 0
                                    ? InfinityUIKit.Palette.accent.color
                                    : InfinityUIKit.Palette.border.color)
                                .frame(
                                    width: CGFloat(8) * canvasScale.height,
                                    height: CGFloat(8) * canvasScale.height
                                )
                            Text(label)
                                .font(.system(
                                    size: max(8, CGFloat(15) * canvasScale.height),
                                    weight: .semibold
                                ))
                                .foregroundStyle(InfinityUIKit.Palette.nativeTextPrimary)
                        }
                        .padding(.leading, CGFloat(16) * canvasScale.height)
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
