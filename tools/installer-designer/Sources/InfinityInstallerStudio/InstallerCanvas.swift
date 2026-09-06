import AppKit
import SwiftUI

struct InstallerCanvas: View {
    @ObservedObject var store: TemplateStore

    var body: some View {
        GeometryReader { proxy in
            ScrollView([.horizontal, .vertical]) {
                let fit = max(0.24, min(
                    (proxy.size.width - 80) / 1600,
                    (proxy.size.height - 80) / 1000
                ))
                let unitScale = fit * store.zoom
                let canvasScale = CGSize(width: unitScale * 1.6, height: unitScale)
                ZStack(alignment: .topLeading) {
                    artboardBackground(scale: canvasScale)
                    if store.showGrid {
                        SnapGrid(gridSize: store.gridSize, scale: canvasScale)
                    }
                    ForEach((store.selectedScreen?.elements ?? []).sorted(by: layerOrder)) { element in
                        if !element.hidden {
                            CanvasElementView(element: element, store: store, canvasScale: canvasScale)
                                .zIndex(Double(element.zIndex))
                        }
                    }
                }
                .frame(width: 1000 * canvasScale.width, height: 1000 * canvasScale.height)
                .overlay {
                    RoundedRectangle(cornerRadius: 2)
                        .stroke(Color.white.opacity(0.24), lineWidth: 1)
                }
                .shadow(color: .black.opacity(0.65), radius: 28, y: 12)
                .padding(32)
                .contentShape(Rectangle())
                .onTapGesture { store.selectElement(nil) }
            }
            .background(
                LinearGradient(
                    colors: [Color(nsColor: .windowBackgroundColor), Color.black.opacity(0.88)],
                    startPoint: .top,
                    endPoint: .bottom
                )
            )
        }
    }

    // ------------------------=
    // FUNC: artboardBackground
    // DESC: Builds the InfinityOS installation-preview background at the current canvas scale.
    // ------------------=
    @ViewBuilder
    private func artboardBackground(scale: CGSize) -> some View {
        ZStack {
            Color(red: 0.008, green: 0.018, blue: 0.035)
            RadialGradient(
                colors: [Color.cyan.opacity(0.18), Color.clear],
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
            context.stroke(minorPath, with: .color(.cyan.opacity(0.06)), lineWidth: 0.5)
            context.stroke(majorPath, with: .color(.cyan.opacity(0.15)), lineWidth: 0.8)
            var axes = Path()
            axes.move(to: CGPoint(x: size.width / 2, y: 0))
            axes.addLine(to: CGPoint(x: size.width / 2, y: size.height))
            axes.move(to: CGPoint(x: 0, y: size.height / 2))
            axes.addLine(to: CGPoint(x: size.width, y: size.height / 2))
            context.stroke(axes, with: .color(.cyan.opacity(0.28)), lineWidth: 1)
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
        elementBody
            .frame(
                width: CGFloat(element.frame.width) * canvasScale.width,
                height: CGFloat(element.frame.height) * canvasScale.height
            )
            .opacity(Double(element.opacity) / 100)
            .overlay { selectionOverlay }
            .position(
                x: CGFloat(element.frame.x) * canvasScale.width + CGFloat(element.frame.width) * canvasScale.width / 2,
                y: CGFloat(element.frame.y) * canvasScale.height + CGFloat(element.frame.height) * canvasScale.height / 2
            )
            .contentShape(Rectangle())
            .onTapGesture { store.selectElement(element.id) }
            .gesture(moveGesture)
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
        case .console:
            RoundedRectangle(cornerRadius: CGFloat(element.cornerRadius) * canvasScale.height)
                .fill(
                    LinearGradient(
                        colors: [element.fill.color.opacity(0.96), Color.black.opacity(0.92)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .overlay {
                    RoundedRectangle(cornerRadius: CGFloat(element.cornerRadius) * canvasScale.height)
                        .stroke(element.border.color.opacity(0.85), lineWidth: max(1, canvasScale.height))
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
                .fill(
                    LinearGradient(
                        colors: [Color(red: 0.08, green: 0.18, blue: 0.28), element.fill.color],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .overlay {
                    RoundedRectangle(cornerRadius: CGFloat(element.cornerRadius) * canvasScale.height)
                        .stroke(Color.cyan.opacity(0.7), lineWidth: max(1, canvasScale.height))
                }
                .overlay {
                    HStack(spacing: 5) {
                        Image(systemName: "lock.fill")
                        Text(element.text).fontWeight(.semibold)
                    }
                    .font(.system(size: max(8, CGFloat(element.fontSize) * canvasScale.height)))
                    .foregroundStyle(.white)
                }
        }
    }

    @ViewBuilder
    private var imagePreview: some View {
        if let image = resolveImage() {
            Image(nsImage: image)
                .resizable()
                .aspectRatio(contentMode: .fit)
        } else {
            RoundedRectangle(cornerRadius: 8 * canvasScale.height)
                .fill(Color.cyan.opacity(0.08))
                .overlay {
                    VStack(spacing: 5) {
                        Image(systemName: element.role == .masthead ? "infinity" : "photo")
                        Text(element.imageAsset.isEmpty ? "Choose Image" : element.imageAsset)
                            .lineLimit(1)
                    }
                    .font(.system(size: max(8, 16 * canvasScale.height)))
                    .foregroundStyle(Color.cyan.opacity(0.8))
                }
        }
    }

    @ViewBuilder
    private var selectionOverlay: some View {
        if isSelected {
            ZStack {
                Rectangle()
                    .stroke(element.locked ? Color.orange : Color.cyan, style: StrokeStyle(lineWidth: 2, dash: element.locked ? [5, 4] : []))
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
                } else {
                    ForEach(ResizeHandle.allCases) { handle in
                        ResizeHandleView(handle: handle, store: store, canvasScale: canvasScale)
                    }
                }
            }
        }
    }

    private var moveGesture: some Gesture {
        DragGesture(minimumDistance: 2)
            .onChanged { value in
                guard isSelected, !element.locked else { return }
                store.beginGesture()
                store.moveSelected(translation: value.translation, canvasScale: canvasScale)
            }
            .onEnded { _ in store.endGesture() }
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
}

private struct ResizeHandleView: View {
    let handle: ResizeHandle
    @ObservedObject var store: TemplateStore
    let canvasScale: CGSize

    var body: some View {
        Circle()
            .fill(Color.white)
            .overlay(Circle().stroke(Color.cyan, lineWidth: 2))
            .frame(width: 11, height: 11)
            .position(handlePosition)
            .gesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { value in
                        store.beginGesture()
                        store.resizeSelected(handle: handle, translation: value.translation, canvasScale: canvasScale)
                    }
                    .onEnded { _ in store.endGesture() }
            )
    }

    private var handlePosition: CGPoint {
        guard let element = store.selectedElement else { return .zero }
        let width = CGFloat(element.frame.width) * canvasScale.width
        let height = CGFloat(element.frame.height) * canvasScale.height
        return switch handle {
        case .topLeft: CGPoint(x: 0, y: 0)
        case .top: CGPoint(x: width / 2, y: 0)
        case .topRight: CGPoint(x: width, y: 0)
        case .right: CGPoint(x: width, y: height / 2)
        case .bottomRight: CGPoint(x: width, y: height)
        case .bottom: CGPoint(x: width / 2, y: height)
        case .bottomLeft: CGPoint(x: 0, y: height)
        case .left: CGPoint(x: 0, y: height / 2)
        }
    }
}
