import SwiftUI

struct StudioRootView: View {
    @ObservedObject var store: TemplateStore

    var body: some View {
        VStack(spacing: 0) {
            StudioToolbar(store: store)
            Divider()
            HSplitView {
                ScreenSidebar(store: store)
                    .frame(minWidth: 220, idealWidth: 248, maxWidth: 290)
                InstallerCanvas(store: store)
                    .frame(minWidth: 560)
                ElementInspector(store: store)
                    .frame(minWidth: 280, idealWidth: 310, maxWidth: 360)
            }
            Divider()
            StudioStatusBar(store: store)
        }
        .background(InfinityUIKit.Palette.nativeCanvas)
        .tint(InfinityUIKit.Palette.nativeAccent)
        .alert("Template Validation", isPresented: .constant(!store.validationIssues.isEmpty)) {
            Button("OK") { store.validationIssues = [] }
        } message: {
            Text(store.validationIssues.joined(separator: "\n"))
        }
    }
}

private struct StudioToolbar: View {
    @ObservedObject var store: TemplateStore

    var body: some View {
        HStack(spacing: 10) {
            if let image = studioIcon() {
                Image(nsImage: image)
                    .resizable()
                    .frame(width: 30, height: 30)
                    .clipShape(RoundedRectangle(cornerRadius: 7))
            } else {
                Image(systemName: "infinity")
                    .font(.title2.weight(.semibold))
                    .foregroundStyle(InfinityUIKit.Palette.nativeAccent)
            }
            VStack(alignment: .leading, spacing: 0) {
                Text("Installer Studio").font(.headline)
                Text("InfinityOS UI Templates").font(.caption).foregroundStyle(.secondary)
            }
            Divider().frame(height: 28).padding(.horizontal, 4)
            Button(action: store.toggleMarqueeSelection) {
                Label(
                    store.marqueeSelectionEnabled ? "Marquee On" : "Marquee Select",
                    systemImage: "rectangle.dashed"
                )
            }
            .buttonStyle(InfinityStudioButtonStyle(
                emphasis: store.marqueeSelectionEnabled ? .primary : .secondary
            ))
            .fixedSize()
            .layoutPriority(20)
            .accessibilityLabel("Marquee Selection")
            .accessibilityValue(store.marqueeSelectionEnabled ? "On" : "Off")
            .help("Select enclosed objects and move them as a group (Shift-Command-M)")
            Button(action: store.undo) { Label("Undo", systemImage: "arrow.uturn.backward") }
                .disabled(!store.canUndo)
            Button(action: store.redo) { Label("Redo", systemImage: "arrow.uturn.forward") }
                .disabled(!store.canRedo)
            Divider().frame(height: 28)
            Menu {
                Button("Panel") { store.addElement(kind: .panel) }
                Button("Text") { store.addElement(kind: .text) }
                Button("Image…") { store.chooseAndAddImage() }
                Button("Console") { store.addElement(kind: .console) }
            } label: {
                Label("Add", systemImage: "plus")
            }
            Menu {
                ForEach(ConsoleLayoutPreset.allCases) { preset in
                    Button(preset.title) { store.applyConsoleLayout(preset) }
                }
            } label: {
                Label("Layouts", systemImage: "rectangle.3.group")
            }
            .help("Insert an editable layout inside the console area")
            Toggle(isOn: $store.showGrid) { Label("Grid", systemImage: "grid") }
                .toggleStyle(.button)
            Toggle(isOn: $store.snapEnabled) { Label("Snap", systemImage: "magnet") }
                .toggleStyle(.button)
            Picker("Grid", selection: $store.gridSize) {
                Text("5").tag(5)
                Text("10").tag(10)
                Text("20").tag(20)
                Text("50").tag(50)
            }
            .frame(width: 86)
            Spacer()
            Button(action: { _ = store.validate() }) {
                Label("Validate", systemImage: "checkmark.shield")
            }
            Button(action: store.save) {
                Label("Save", systemImage: "square.and.arrow.down")
            }
            .buttonStyle(InfinityStudioButtonStyle(emphasis: .primary))
        }
        .buttonStyle(InfinityStudioButtonStyle(emphasis: .secondary))
        .controlSize(.regular)
        .padding(.horizontal, InfinityUIKit.Metrics.gutter)
        .frame(height: InfinityUIKit.Metrics.toolbarHeight)
        .background(InfinityUIKit.Palette.nativePanel)
    }

    // ------------------------=
    // FUNC: studioIcon
    // DESC: Loads the generated Installer Studio identity from the packaged resource bundle.
    // ------------------=
    private func studioIcon() -> NSImage? {
        Bundle.module.url(forResource: "InstallerStudioIcon-v1", withExtension: "png")
            .flatMap(NSImage.init(contentsOf:))
    }
}

private struct ScreenSidebar: View {
    @ObservedObject var store: TemplateStore
    @State private var installationExpanded = true
    @State private var configurationExpanded = true

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            List {
                DisclosureGroup(isExpanded: $installationExpanded) {
                    screenRows(for: .installation, screens: store.document.screens)
                } label: {
                    collectionHeader(.installation)
                }
                DisclosureGroup(isExpanded: $configurationExpanded) {
                    screenRows(for: .configuration, screens: store.configurationDocument.screens)
                } label: {
                    collectionHeader(.configuration)
                }
            }
            .listStyle(.sidebar)
            Divider()
            if store.selectedScreen != nil {
                VStack(alignment: .leading, spacing: 8) {
                    Text("SCREEN NAME")
                        .font(.caption2.weight(.semibold))
                        .foregroundStyle(.secondary)
                    TextField(
                        "Screen name",
                        text: Binding(
                            get: { store.selectedScreen?.title ?? "" },
                            set: { value in store.renameSelectedScreen(value) }
                        )
                    )
                    HStack(spacing: 6) {
                        Button(action: store.duplicateScreen) {
                            Image(systemName: "plus.square.on.square")
                        }
                        .help("Duplicate Screen")
                        .disabled(!store.canAddScreen)
                        Button(action: { store.moveScreen(-1) }) {
                            Image(systemName: "arrow.up")
                        }
                        .help("Move Screen Up")
                        .disabled(store.selectedScreenIndex == 0)
                        Button(action: { store.moveScreen(1) }) {
                            Image(systemName: "arrow.down")
                        }
                        .help("Move Screen Down")
                        .disabled(store.selectedScreenIndex == store.activeScreens.count - 1)
                        Spacer()
                        Button(role: .destructive, action: store.removeScreen) {
                            Image(systemName: "trash")
                        }
                        .help("Remove Screen")
                        .disabled(!store.canRemoveScreen)
                    }
                    .buttonStyle(InfinityStudioButtonStyle(emphasis: .quiet))
                }
                .controlSize(.regular)
                .padding(InfinityUIKit.Metrics.gutter)
            }
            Divider()
            Button(role: .destructive, action: store.resetScreen) {
                Label("Reset Current Screen", systemImage: "arrow.counterclockwise")
            }
            .buttonStyle(InfinityStudioButtonStyle(emphasis: .quiet))
            .foregroundStyle(.red)
            .padding(InfinityUIKit.Metrics.gutter)
        }
        .background(InfinityUIKit.Palette.nativePanel)
    }

    // ------------------------=
    // FUNC: collectionHeader
    // DESC: Renders an expandable collection heading with a scoped add-screen action.
    // ------------------=
    private func collectionHeader(_ collection: ScreenCollection) -> some View {
        HStack(spacing: 8) {
            Image(systemName: collection == .installation ? "shippingbox" : "person.crop.rectangle.stack")
                .foregroundStyle(store.selectedCollection == collection
                    ? InfinityUIKit.Palette.nativeAccent
                    : InfinityUIKit.Palette.nativeTextSecondary)
            Text(collection.title.uppercased())
                .font(.caption.weight(.semibold))
                .foregroundStyle(store.selectedCollection == collection ? .primary : .secondary)
            Spacer()
            Button(action: { store.addScreenToCollection(collection) }) {
                Image(systemName: "plus")
            }
            .buttonStyle(.borderless)
            .disabled(store.selectedCollection == collection && !store.canAddScreen)
            .help("Add \(collection.title.dropLast())")
        }
        .contentShape(Rectangle())
        .onTapGesture { store.selectScreenCollection(collection) }
    }

    // ------------------------=
    // FUNC: screenRows
    // DESC: Renders selectable, reorderable rows for one independently numbered screen flow.
    // ------------------=
    @ViewBuilder
    private func screenRows(
        for collection: ScreenCollection,
        screens: [InstallerScreenTemplate]
    ) -> some View {
        ForEach(screens) { screen in
            let selected = collection == store.selectedCollection && screen.id == store.selectedScreenID
            Button(action: { store.selectScreenCollection(collection, screen: screen.id) }) {
                HStack(spacing: 10) {
                    Text(String(format: "%02d", screen.id))
                        .font(.system(.caption, design: .monospaced).weight(.bold))
                        .foregroundStyle(selected ? .black : InfinityUIKit.Palette.nativeAccent)
                        .frame(width: 28, height: 28)
                        .background(selected
                            ? InfinityUIKit.Palette.nativeAccent
                            : InfinityUIKit.Palette.nativeAccent.opacity(0.12))
                        .clipShape(RoundedRectangle(cornerRadius: 7))
                    VStack(alignment: .leading, spacing: 2) {
                        Text(screen.title).font(.callout.weight(.medium))
                        Text("\(screen.elements.count) layers")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                    Spacer()
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .listRowBackground(selected ? InfinityUIKit.Palette.nativeAccent.opacity(0.13) : Color.clear)
        }
        .onMove { offsets, destination in
            store.moveScreensInCollection(collection, fromOffsets: offsets, toOffset: destination)
        }
    }
}

private struct StudioStatusBar: View {
    @ObservedObject var store: TemplateStore

    var body: some View {
        HStack(spacing: 14) {
            Circle()
                .fill(store.validationIssues.isEmpty ? Color.green : Color.orange)
                .frame(width: 7, height: 7)
            Text(store.status).lineLimit(1)
            if store.selectedElementIDs.count > 1 {
                Divider().frame(height: 14)
                Text("\(store.selectedElementIDs.count) elements")
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(InfinityUIKit.Palette.nativeAccent)
            } else if let element = store.selectedElement {
                Divider().frame(height: 14)
                Text("\(element.frame.x), \(element.frame.y)   \(element.frame.width) × \(element.frame.height)")
                    .font(.system(.caption, design: .monospaced))
            }
            Spacer()
            Text(store.snapEnabled ? "Snap \(store.gridSize)" : "Free positioning")
            HStack(spacing: 5) {
                Button(action: store.zoomOut) {
                    Image(systemName: "minus.magnifyingglass")
                }
                .help("Zoom Out")
                Slider(
                    value: $store.zoom,
                    in: TemplateStore.minimumZoom...TemplateStore.maximumZoom,
                    step: 0.05
                )
                .frame(width: 130)
                Button(action: store.resetZoom) {
                    Text("\(Int(store.zoom * 100))%")
                        .monospacedDigit()
                        .frame(width: 42)
                }
                .buttonStyle(.borderless)
                .help("Reset Zoom")
                Button(action: store.zoomIn) {
                    Image(systemName: "plus.magnifyingglass")
                }
                .help("Zoom In")
            }
            .buttonStyle(InfinityStudioButtonStyle(emphasis: .quiet))
        }
        .font(.caption)
        .foregroundStyle(.secondary)
        .padding(.horizontal, InfinityUIKit.Metrics.gutter)
        .frame(height: InfinityUIKit.Metrics.statusBarHeight)
        .background(InfinityUIKit.Palette.nativePanel)
    }
}
