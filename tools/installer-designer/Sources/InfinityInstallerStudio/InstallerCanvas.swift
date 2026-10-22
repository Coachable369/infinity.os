import AppKit
import SwiftUI

struct InstallerCanvas: View {
    @ObservedObject var store: TemplateStore
    @State private var marqueeDisplayRect: CGRect?
    @State private var canvasDragMode: CanvasDragMode?
    @StateObject private var nativePreview = NativePreviewService()
    private var simulatedSettings: Bool {
        store.selectedCollection == .systemSettings && store.nativePreviewSimulation.enabled
            && [1,3,4,5,6,7,8,10,11].contains(store.selectedScreenID)
    }

    var body: some View {
        GeometryReader { proxy in
            ZStack {
                ScrollView([.horizontal, .vertical]) {
                    let fit = max(0.24, min(
                        (proxy.size.width - 80) / CGFloat(store.canvasViewport.width),
                        (proxy.size.height - 80) / CGFloat(store.canvasViewport.height)
                    ))
                    let unitScale = fit * store.zoom
                    let canvasScale = CGSize(width: unitScale * CGFloat(store.canvasViewport.width) / 1000,
                        height: unitScale * CGFloat(store.canvasViewport.height) / 1000)
                    ZStack(alignment: .topLeading) {
                        artboardBackground(scale: canvasScale)
                            .contentShape(Rectangle())
                            .onTapGesture {
                                if !store.marqueeSelectionEnabled { store.selectElement(nil) }
                            }
                        if store.showGrid {
                            SnapGrid(gridSize: store.gridSize, scale: canvasScale)
                        }
                        if simulatedSettings {
                            nativeSelectionOverlays(scale: unitScale)
                        } else {
                        ForEach((store.selectedScreen?.elements ?? []).sorted(by: layerOrder)) { element in
                            if !element.hidden && !element.role.isSettingsChild {
                                CanvasElementView(element: element, store: store, canvasScale: canvasScale, interactionOnly: true)
                                    .zIndex(Double(element.zIndex))
                            }
                        }
                        }
                        if store.marqueeSelectionEnabled && !simulatedSettings {
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
                        if !simulatedSettings {
                            selectionHandleOverlay(canvasScale: canvasScale).zIndex(41_000)
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
                .overlay(alignment: .topLeading) { nativePreviewStatus.padding(12) }
            }
        }
        .onAppear { nativePreview.schedule(store) }
        .onReceive(store.objectWillChange) { nativePreview.schedule(store) }
        .onDisappear { nativePreview.cancel() }
    }

    // ------------------------=
    // FUNC: nativeSelectionOverlays
    // DESC: Maps native Settings selection, dragging and resizing back to authored coordinates without duplicating OS paint code.
    // ------------------=
    @ViewBuilder private func nativeSelectionOverlays(scale: CGFloat) -> some View {
        if let frame = nativePreview.frame {
            let console = store.selectedScreen?.elements.first { $0.role == .console }
            let window = frame.bounds.first { $0.id == console?.id }
            ForEach(frame.bounds) { bounds in
                if let element = store.selectedScreen?.elements.first(where: { $0.id == bounds.id }), !element.hidden {
                    let editingScale = NativeCanvasMapping.editingScale(element: element, console: console?.frame,
                        window: window, viewport: store.canvasViewport, scale: scale, bounds: bounds)
                    let displayed = CGRect(x: CGFloat(bounds.x) * scale, y: CGFloat(bounds.y) * scale,
                        width: CGFloat(bounds.width) * scale, height: CGFloat(bounds.height) * scale)
                    let fixedGeometry = !store.geometryIsEditable(element)
                        || editingScale.width <= 0 || editingScale.height <= 0
                    let interactionClip = element.role.isSettingsChild
                        ? NativeCanvasMapping.actionInteractionClip(bounds: bounds, scale: scale) : nil
                    CanvasElementView(element: element, store: store, canvasScale: editingScale, interactionOnly: true,
                        displayFrame: displayed, fixedNativeGeometry: fixedGeometry, interactionClip: interactionClip)
                        .zIndex(Double(element.zIndex))
                    if store.resizeHandlesVisible && store.selectedElementID == element.id && !element.locked && !fixedGeometry {
                        ZStack {
                            ForEach(ResizeHandle.allCases) { handle in
                                ResizeHandleView(handle: handle, elementID: element.id, elementFrame: element.frame,
                                    store: store, canvasScale: editingScale, displaySize: displayed.size,
                                    interactionClip: interactionClip, displayOrigin: displayed.origin)
                            }
                        }.frame(width: displayed.width, height: displayed.height)
                            .position(x: displayed.midX, y: displayed.midY).zIndex(41_000)
                    }
                }
            }
        }
    }

    // ------------------------=
    // FUNC: artboardBackground
    // DESC: Displays one complete production-rendered framebuffer, without host-font or approximate scene overlays.
    // ------------------=
    @ViewBuilder
    private func artboardBackground(scale: CGSize) -> some View {
        ZStack {
            Color.black
            if let image = nativePreview.image {
                Image(decorative: image, scale: 1)
                    .resizable().interpolation(.high)
            }
        }.frame(width: 1000 * scale.width, height: 1000 * scale.height)
    }

    @ViewBuilder private var nativePreviewStatus: some View {
        VStack(alignment: .leading, spacing: 6) {
        HStack(spacing: 8) {
            switch nativePreview.phase {
            case .idle, .rendering:
                ProgressView().controlSize(.small)
                Text(nativePreview.image == nil ? "Rendering native OS preview…" : "Updating native OS preview…")
            case .ready:
                Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
                Text(simulatedSettings ? "Native Settings preview" : store.selectedCollection == .systemSettings ? "Native template preview" : "Native OS preview")
            case .failed:
                Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(.orange)
                VStack(alignment: .leading, spacing: 3) {
                    Text("Preview is not current").fontWeight(.semibold)
                    Text(nativePreview.errorMessage ?? "Rendering failed").lineLimit(3)
                }
                Button("Retry") { nativePreview.schedule(store) }
            }
            if nativePreview.phase != .failed {
                Picker("Viewport", selection: $store.canvasViewport) {
                    ForEach(NativePreviewViewport.allCases) { viewport in Text(viewport.rawValue).tag(viewport) }
                }.labelsHidden().frame(width: 142)
                Picker("Control state", selection: $store.nativePreviewState) {
                    ForEach(NativePreviewControlState.allCases) { state in
                        Text(state.rawValue).tag(state).disabled(!state.isSupported(in: store.selectedCollection))
                    }
                }.labelsHidden().frame(width: 100)
            }
        }
        if store.selectedCollection == .systemSettings {
            Text("Transition preview unavailable: Settings has no native entrance animation.")
                .foregroundStyle(.secondary)
        }
        if store.selectedCollection != .configuration {
            HStack(spacing: 8) {
                Toggle("Simulated service values", isOn: $store.nativePreviewSimulation.enabled)
                    .toggleStyle(.checkbox)
                if store.nativePreviewSimulation.enabled {
                    Text("Preview only — no system changes").foregroundStyle(.secondary)
                }
            }
            if store.nativePreviewSimulation.enabled {
                simulationControls
            }
        }
        }
        .font(.system(size: 11))
        .padding(.horizontal, 12).padding(.vertical, 8)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 8))
        .frame(maxWidth: 560, alignment: .leading)
    }

    @ViewBuilder private var simulationControls: some View {
        if store.selectedCollection == .installation {
            switch store.selectedScreenID {
            case 5:
                HStack(spacing: 8) {
                    Picker("Time zone", selection: $store.nativePreviewSimulation.timeZone) {
                        ForEach(Array(["UTC", "Pacific", "Mountain", "Central", "Eastern", "Atlantic", "GMT", "Europe", "India", "Singapore", "Japan", "Australia"].enumerated()), id: \.offset) { index, name in
                            Text(name).tag(index)
                        }
                    }.frame(width: 170)
                    Picker("Field", selection: $store.nativePreviewSimulation.dateFocus) {
                        Text("Date").tag(2); Text("Time").tag(3); Text("Zone").tag(4)
                    }.frame(width: 110)
                    if store.nativePreviewSimulation.dateFocus != 4 {
                        Picker("Part", selection: Binding(get: {
                            store.nativePreviewSimulation.resolved(collection: .installation, screen: 5, rows: 0).datePart
                        }, set: { store.nativePreviewSimulation.datePart = $0 })) {
                            if store.nativePreviewSimulation.dateFocus == 2 {
                                Text("Year").tag(0); Text("Month").tag(1); Text("Day").tag(2)
                            } else { Text("Hour").tag(3); Text("Minute").tag(4) }
                        }.frame(width: 110)
                    }
                }
            case 7:
                Toggle("Show erase confirmation", isOn: $store.nativePreviewSimulation.confirmation)
            case 8:
                Stepper("Installation progress: \(store.nativePreviewSimulation.progress)%", value: $store.nativePreviewSimulation.progress, in: 0...100, step: 5)
            default:
                Text("Authored layers; this screen has no simulated service controls.").foregroundStyle(.secondary)
            }
        } else if store.selectedCollection == .systemSettings {
            VStack(alignment: .leading, spacing: 5) {
                if [1,3,4,5,6,7,8,10,11].contains(store.selectedScreenID) {
                  HStack(spacing: 10) {
                    let rows = store.selectedScreen?.elements.filter { $0.role == .settingsRowLabel }.count ?? 0
                    let resolved = store.nativePreviewSimulation.resolved(collection: .systemSettings, screen: store.selectedScreenID, rows: rows)
                    Stepper([7,8].contains(store.selectedScreenID) ? "Page \(resolved.expandedRow + 1)" : resolved.expandedRow < 0 ? "Rows collapsed" : "Expanded row \(resolved.expandedRow + 1)",
                        value: Binding(get: { resolved.expandedRow }, set: { store.nativePreviewSimulation.expandedRow = $0 }),
                        in: NativePreviewSimulation.settingsRange(screen: store.selectedScreenID, rows: rows))
                    Stepper("Scroll \(store.nativePreviewSimulation.scroll) px", value: $store.nativePreviewSimulation.scroll, in: 0...10000, step: 40)
                  }
                }
                if ![1,3,4,5,6,7,8,10,11].contains(store.selectedScreenID) {
                    Text("Template layers only: this section requires live OS services.").foregroundStyle(.orange)
                } else if [7,8].contains(store.selectedScreenID) {
                    Text("Dashboard cards use OS-responsive bounds. Shared surfaces and artwork remain editable.").foregroundStyle(.secondary)
                } else {
                    Text("Native layout editing. Row height and spacing follow the OS minimums.").foregroundStyle(.secondary)
                }
            }
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
    var renderingBackdrop = false
    var interactionOnly = false
    var displayFrame: CGRect? = nil
    var fixedNativeGeometry = false
    var interactionClip: CGRect? = nil
    @State private var acceptedInteraction: Bool?

    private var renderedFrame: CGRect {
        displayFrame ?? CGRect(x: CGFloat(element.frame.x) * canvasScale.width, y: CGFloat(element.frame.y) * canvasScale.height,
            width: CGFloat(element.frame.width) * canvasScale.width, height: CGFloat(element.frame.height) * canvasScale.height)
    }

    var isSelected: Bool { !renderingBackdrop && store.selectedElementIDs.contains(element.id) }
    private var atlasData: Data? { store.uiKit.previewAtlas(target: "assets/ui-design-kit/default/eclipse-precision-v1.rgba") }
    private var progressIndex: Int {
        let layers = (store.selectedScreen?.elements ?? []).filter { !$0.hidden && $0.opacity != 0 && $0.role == .progressSegment }
            .sorted { $0.zIndex == $1.zIndex ? $0.id.uuidString < $1.id.uuidString : $0.zIndex < $1.zIndex }
        return layers.firstIndex { $0.id == element.id } ?? 0
    }

    var body: some View {
        ZStack {
            Group {
                if interactionOnly { Color.clear } else { elementBody }
            }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .opacity(element.role == .console ? 1 : Double(element.opacity) / 100)
                .contentShape(Path(interactionClip ?? CGRect(origin: .zero, size: renderedFrame.size)))
                .highPriorityGesture(elementInteractionGesture)
            if !renderingBackdrop {
                if let interactionClip { selectionOverlay.mask(Path(interactionClip)) }
                else { selectionOverlay }
            }
        }
        .frame(
            width: renderedFrame.width,
            height: renderedFrame.height
        )
            .position(
                x: renderedFrame.midX,
                y: renderedFrame.midY
            )
    }

    @ViewBuilder
    private var elementBody: some View {
        switch element.kind {
        case .panel:
            CanvasPanelPreview(
                element: element,
                canvasScale: canvasScale,
                focused: isSelected,
                configuration: store.selectedCollection == .configuration,
                projectRoot: store.projectRoot,
                atlasData: atlasData,
                surfaceStyle: .template(for: element, configuration: store.selectedCollection == .configuration, styles: store.uiKit.document.styles),
                glyphData: store.uiKit.previewAtlas(target: "assets/ui-design-kit/default/eclipse-access-glyphs-v1.rgba"),
                progressIndex: progressIndex,
                progressStep: max(0, store.selectedScreenID - 1),
                frostImage: element.role == .console ? { CanvasFrostPreview.image(for: element, store: store) } : nil
            )
        case .console:
            CanvasPanelPreview(
                element: element,
                canvasScale: canvasScale,
                focused: isSelected,
                configuration: store.selectedCollection == .configuration,
                projectRoot: store.projectRoot,
                atlasData: atlasData,
                surfaceStyle: .template(for: element, configuration: store.selectedCollection == .configuration, styles: store.uiKit.document.styles),
                glyphData: store.uiKit.previewAtlas(target: "assets/ui-design-kit/default/eclipse-access-glyphs-v1.rgba"),
                progressIndex: progressIndex,
                progressStep: max(0, store.selectedScreenID - 1),
                frostImage: { CanvasFrostPreview.image(for: element, store: store) }
            )
        case .text:
            Text(element.text)
                .font(InfinityUIKit.EclipseAccess.font(size: max(6, CGFloat(element.fontSize) * canvasScale.height),
                    semibold: element.role == .title, projectRoot: store.projectRoot))
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
                focused: isSelected,
                configuration: store.selectedCollection == .configuration,
                projectRoot: store.projectRoot,
                atlasData: atlasData,
                surfaceStyle: .template(for: element, configuration: store.selectedCollection == .configuration, styles: store.uiKit.document.styles)
            )
        case .progressBar:
            CanvasProgressBarPreview(element: element, canvasScale: canvasScale, projectRoot: store.projectRoot, atlasData: atlasData,
                styles: store.uiKit.document.styles)
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
                if acceptedInteraction == nil { acceptedInteraction = acceptsInteraction(at: value.startLocation) }
                guard acceptedInteraction == true else { return }
                guard !element.locked && !fixedNativeGeometry else { return }
                let distance = max(abs(value.translation.width), abs(value.translation.height))
                guard distance >= CanvasInteractionMetrics.dragThreshold else { return }
                store.beginGesture(elementID: element.id)
                store.moveSelected(translation: value.translation, canvasScale: canvasScale)
            }
            .onEnded { value in
                defer { acceptedInteraction = nil }
                guard acceptedInteraction ?? acceptsInteraction(at: value.startLocation) else { return }
                let distance = max(abs(value.translation.width), abs(value.translation.height))
                if distance < CanvasInteractionMetrics.dragThreshold {
                    store.activateCanvasElement(element.id)
                } else {
                    store.endGesture()
                }
            }
    }

    // ------------------------=
    // FUNC: acceptsInteraction
    // DESC: Rejects pointer starts clipped by the native parent or viewport while retaining full artboard coordinates during a drag.
    // ------------------=
    private func acceptsInteraction(at point: CGPoint) -> Bool {
        interactionClip?.contains(CGPoint(x: point.x - renderedFrame.minX, y: point.y - renderedFrame.minY)) ?? true
    }

    // ------------------------=
    // FUNC: resolveImage
    // DESC: Resolves a template image from the repository, absolute path, or app resource bundle.
    // ------------------=
    private func resolveImage() -> NSImage? {
        guard !element.imageAsset.isEmpty else { return nil }
        if let asset = store.uiKit.assets.first(where: { $0.format != "rgba"
            && ($0.target == element.imageAsset || $0.target == "assets/boot/\(element.imageAsset)") }),
           let preview = store.uiKit.previewImage(for: asset) { return preview }
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
        element.crop.previewImage(image)
    }
}

extension ImageCrop {
    // ------------------------=
    // FUNC: previewImage
    // DESC: Crops top-origin source pixels exactly as the native template decoder; NSImage point-coordinate orientation does not apply here.
    // ------------------=
    func previewImage(_ image: NSImage) -> NSImage? {
        let crop = clamped()
        guard crop != .none,
              let source = image.cgImage(forProposedRect: nil, context: nil, hints: nil)
        else { return image }
        let width = CGFloat(source.width)
        let height = CGFloat(source.height)
        let rect = CGRect(
            x: width * CGFloat(crop.left) / 100,
            y: height * CGFloat(crop.top) / 100,
            width: width * CGFloat(100 - crop.left - crop.right) / 100,
            height: height * CGFloat(100 - crop.top - crop.bottom) / 100
        ).integral
        guard rect.width >= 1, rect.height >= 1, let result = source.cropping(to: rect) else { return image }
        return NSImage(cgImage: result, size: NSSize(width: rect.width, height: rect.height))
    }
}

struct CanvasProgressBarPreview: View {
    let element: StudioElement
    let canvasScale: CGSize
    let projectRoot: URL?
    var atlasData: Data? = nil
    var styles: [UIKitSurfaceStyle] = UIKitSurfaceStyle.defaults

    var body: some View {
        VStack(spacing: 14 * canvasScale.height) {
            HStack {
                Text(element.text)
                    .font(InfinityUIKit.EclipseAccess.font(size: max(1, 18 * canvasScale.height), semibold: true, projectRoot: projectRoot))
                    .lineLimit(1)
                Spacer()
                Text("42%")
                    .font(InfinityUIKit.EclipseAccess.font(size: max(1, 18 * canvasScale.height), semibold: true, projectRoot: projectRoot))
                    .foregroundStyle(element.fill.color)
            }
            GeometryReader { geometry in
                ZStack(alignment: .leading) {
                    PrecisionSurface(kind: .field, material: PrecisionMaterial(fill: InfinityUIKit.EclipseAccess.background,
                        edge: element.border, opacity: 199, style: .resolved(for: .field, styles: styles)), projectRoot: projectRoot, logicalScale: canvasScale.height, atlasData: atlasData)
                    PrecisionSurface(kind: .primary, material: PrecisionMaterial(fill: InfinityUIKit.EclipseAccess.primary,
                        edge: element.border, opacity: element.fill.alpha, style: .resolved(for: .primary, styles: styles)), projectRoot: projectRoot, logicalScale: canvasScale.height, atlasData: atlasData)
                        .frame(width: geometry.size.width * 0.42)
                }
            }
            .frame(height: max(1, 16 * canvasScale.height))
        }
        .foregroundStyle(InfinityUIKit.Palette.nativeTextPrimary)
        .padding(18 * canvasScale.height)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background {
            PrecisionSurface(kind: .panel, material: PrecisionMaterial(fill: InfinityUIKit.EclipseAccess.surface,
                edge: element.border, opacity: 240, style: .resolved(for: .panel, styles: styles)), projectRoot: projectRoot, logicalScale: canvasScale.height, atlasData: atlasData)
        }
        .clipped()
    }
}

private struct CanvasPanelPreview: View {
    let element: StudioElement
    let canvasScale: CGSize
    let focused: Bool
    let configuration: Bool
    let projectRoot: URL?
    var atlasData: Data? = nil
    var surfaceStyle: UIKitSurfaceStyle = .authored
    var glyphData: Data? = nil
    var progressIndex = 0
    var progressStep = 0
    var frostImage: (() -> CGImage?)? = nil
    @State private var hovered = false

    private var interactionState: InfinityUIKitInteractionState {
        focused ? .focused : (hovered ? .hover : .idle)
    }

    private var material: PrecisionMaterial {
        var material = InfinityUIKit.precisionMaterial(state: interactionState, fill: element.fill, border: element.border)
        material.style = surfaceStyle
        return material
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
        if element.role == .console, let image = frostImage?() {
            Image(decorative: image, scale: 1).resizable().interpolation(.high)
        } else if configuration && element.role == .progressSegment {
            GeometryReader { geometry in
                let diameter = InfinityUIKit.EclipseAccess.progressDiameter(geometry.size)
                PrecisionSurface(kind: progressIndex <= progressStep ? .current : .inactive,
                    material: PrecisionMaterial(fill: element.fill, edge: element.border),
                    projectRoot: projectRoot, logicalScale: canvasScale.height, atlasData: atlasData)
                    .overlay {
                        if progressIndex < progressStep,
                           let glyph = InfinityUIKit.EclipseAccess.glyph(15, projectRoot: projectRoot, atlasData: glyphData) {
                            Image(nsImage: glyph).resizable().scaledToFit()
                                .frame(width: diameter * 0.75, height: diameter * 0.75)
                        }
                    }
                    .frame(width: diameter, height: diameter)
                    .frame(maxWidth: .infinity, alignment: .center)
            }
        } else if element.role == .progressSegment {
            RoundedRectangle(cornerRadius: CGFloat(element.cornerRadius) * canvasScale.height)
                .fill(element.fill.color)
                .frame(height: max(2, 4 * canvasScale.height))
        } else {
            PrecisionSurface(kind: InfinityUIKit.precisionKind(role: element.role, kind: element.kind, configuration: configuration),
                material: material, projectRoot: projectRoot, logicalScale: canvasScale.height, atlasData: atlasData)
                .overlay(alignment: .leading) { fieldContent }
                .contentShape(Rectangle())
                .onHover { hovered = $0 }
                .animation(InfinityUIKit.EclipseAccess.focusAnimation, value: interactionState)
        }
    }

    @ViewBuilder
    private var fieldContent: some View {
        if (isField || [
            .metadata, .liveDetails, .settingsNavigationItem, .settingsTab,
            .settingsSummaryCard, .settingsSidebarCard,
        ].contains(element.role)), !element.text.isEmpty {
            HStack(spacing: 8 * canvasScale.height) {
                Text(element.text)
                    .font(InfinityUIKit.EclipseAccess.font(size: max(6, CGFloat(element.fontSize) * canvasScale.height), projectRoot: projectRoot))
                    .foregroundStyle(element.role == .input
                        ? (configuration ? InfinityUIKit.EclipseAccess.secondary.color : InfinityUIKit.Palette.placeholder.color)
                        : InfinityUIKit.EclipseAccess.text.color)
                    .lineLimit(1)
                Spacer(minLength: 4)
                if let trailingIcon {
                    Image(systemName: trailingIcon)
                        .font(.system(size: max(8, 15 * canvasScale.height), weight: .light))
                        .foregroundStyle(configuration ? InfinityUIKit.EclipseAccess.border.color : InfinityUIKit.Palette.nativeAccentBright)
                }
            }
            .padding(.horizontal, max(7, 16 * canvasScale.height))
        }
    }
}

private struct CanvasButtonPreview: View {
    let element: StudioElement
    let canvasScale: CGSize
    let focused: Bool
    let configuration: Bool
    let projectRoot: URL?
    var atlasData: Data? = nil
    var surfaceStyle: UIKitSurfaceStyle = .authored
    @State private var hovered = false

    private var interactionState: InfinityUIKitInteractionState {
        focused ? .focused : (hovered ? .hover : .idle)
    }

    private var material: PrecisionMaterial {
        var material = InfinityUIKit.precisionMaterial(state: interactionState, fill: element.fill, border: element.border)
        material.style = surfaceStyle
        return material
    }

    var body: some View {
        PrecisionSurface(kind: InfinityUIKit.precisionKind(role: element.role, kind: .button, configuration: configuration),
            material: material,
            projectRoot: projectRoot, logicalScale: canvasScale.height, atlasData: atlasData)
            .overlay { buttonLabel }
            .contentShape(Rectangle())
            .onHover { hovered = $0 }
            .animation(InfinityUIKit.EclipseAccess.focusAnimation, value: interactionState)
    }

    private var buttonLabel: some View {
        Text(element.text)
            .font(InfinityUIKit.EclipseAccess.font(size: max(6, CGFloat(element.fontSize) * canvasScale.height), semibold: true, projectRoot: projectRoot))
            .foregroundStyle(InfinityUIKit.EclipseAccess.text.color)
            .lineLimit(1)
            .padding(.horizontal, max(8, 14 * canvasScale.height))
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
    var atlasData: Data? = nil
    var glyphData: Data? = nil
    var styles: [UIKitSurfaceStyle] = UIKitSurfaceStyle.defaults
    private let labels = ["Wired network", "Wi-Fi", "Continue offline"]
    private let details = ["Wired adapter detected", "No wireless adapter detected", "Set up networking later in Settings"]
    private let icons = [5, 0, 8]

    var body: some View {
        ZStack(alignment: .topLeading) {
            ForEach(Array(labels.enumerated()), id: \.offset) { index, label in
                let frame = ConfigurationNetworkLayout.row(in: content, index: index)
                PrecisionSurface(kind: .panel, material: PrecisionMaterial(
                    fill: index == 0 ? InfinityUIKit.EclipseAccess.elevated : InfinityUIKit.EclipseAccess.background,
                    edge: index == 0 ? InfinityUIKit.EclipseAccess.border : InfinityUIKit.EclipseAccess.mutedBorder,
                    style: .resolved(for: .panel, styles: styles)),
                    projectRoot: projectRoot, logicalScale: canvasScale.height, atlasData: atlasData)
                    .overlay(alignment: .leading) {
                        HStack(spacing: CGFloat(10) * canvasScale.height) {
                            if let icon = InfinityUIKit.EclipseAccess.glyph(icons[index], projectRoot: projectRoot, atlasData: glyphData) {
                                Image(nsImage: icon).resizable().scaledToFit()
                                    .frame(width: 24 * canvasScale.height, height: 24 * canvasScale.height)
                            }
                            VStack(alignment: .leading, spacing: 5 * canvasScale.height) {
                                HStack {
                                    Text(label).font(InfinityUIKit.EclipseAccess.font(size: max(6, 20 * canvasScale.height), semibold: true, projectRoot: projectRoot))
                                    Spacer(minLength: 0)
                                    if index == 0 {
                                        Text("SELECTED")
                                            .font(InfinityUIKit.EclipseAccess.font(size: max(6, 16 * canvasScale.height), projectRoot: projectRoot))
                                            .foregroundStyle(InfinityUIKit.EclipseAccess.border.color)
                                    }
                                }
                                Text(details[index]).font(InfinityUIKit.EclipseAccess.font(size: max(6, 18 * canvasScale.height), projectRoot: projectRoot))
                                    .lineLimit(2).fixedSize(horizontal: false, vertical: true)
                                    .foregroundStyle(InfinityUIKit.EclipseAccess.secondary.color)
                            }
                            .foregroundStyle(InfinityUIKit.EclipseAccess.text.color)
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
    var displaySize: CGSize? = nil
    var interactionClip: CGRect? = nil
    var displayOrigin: CGPoint? = nil
    @State private var acceptedInteraction: Bool?

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
        .contentShape(Path(handleClip))
        .mask(Path(handleClip))
        .position(handlePosition)
        .highPriorityGesture(
            DragGesture(minimumDistance: 0, coordinateSpace: .named(CanvasInteractionMetrics.coordinateSpaceName))
                .onChanged { value in
                    if acceptedInteraction == nil {
                        if let interactionClip, let displayOrigin {
                            acceptedInteraction = interactionClip.contains(CGPoint(x: value.startLocation.x - displayOrigin.x,
                                                                                  y: value.startLocation.y - displayOrigin.y))
                        } else { acceptedInteraction = true }
                    }
                    guard acceptedInteraction == true else { return }
                    store.beginGesture(elementID: elementID)
                    store.resizeSelected(handle: handle, translation: value.translation, canvasScale: canvasScale)
                }
                .onEnded { _ in
                    if acceptedInteraction == true { store.endGesture() }
                    acceptedInteraction = nil
                }
        )
        .accessibilityLabel("Resize \(handle.accessibilityName)")
        .accessibilityHint("Drag to resize the selected element")
    }

    private var handleClip: CGRect {
        let size = CanvasInteractionMetrics.resizeHandleHitSize
        let full = CGRect(x: 0, y: 0, width: size, height: size)
        guard let interactionClip else { return full }
        let clipped = full.intersection(interactionClip.offsetBy(dx: size / 2 - handlePosition.x,
                                                               dy: size / 2 - handlePosition.y))
        return clipped.isNull ? .zero : clipped
    }

    private var handlePosition: CGPoint {
        let width = displaySize?.width ?? CGFloat(elementFrame.width) * canvasScale.width
        let height = displaySize?.height ?? CGFloat(elementFrame.height) * canvasScale.height
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
