import SwiftUI

struct MenuEditor: View {
    @ObservedObject var store: TemplateStore
    private var selection: UUID? {
        get { store.selectedMenuItemID }
        nonmutating set { store.selectedMenuItemID = newValue }
    }
    private var selected: StudioMenuItem? { store.menu.rows.first { $0.id == selection }?.item }

    var body: some View {
        HSplitView {
            VStack(spacing: 0) {
                HStack(spacing: 12) {
                    Image(systemName: "list.bullet.rectangle").foregroundStyle(InfinityUIKit.Palette.nativeAccent)
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Infinity Menu").font(.headline)
                        Text("\(store.menu.rows.count) items").font(.caption).foregroundStyle(.secondary)
                    }
                    Spacer()
                    Menu {
                        MenuPreviewItems(store: store, items: store.menu.items)
                    } label: { Label("Preview", systemImage: "play") }
                    .disabled(store.menu.items.isEmpty)
                    Button("Canvas", systemImage: "rectangle.on.rectangle") { store.editingMenu = false }
                }.padding(16)
                Divider()
                HStack(spacing: 14) {
                    Menu {
                        Button("Menu Item") { add(.system) }
                        Button("Submenu") { add(.submenu) }
                        Button("Separator") { add(.separator) }
                        if selected?.action == .submenu {
                            Divider()
                            Button("Child Item") { add(.system, inside: true) }
                            Button("Child Submenu") { add(.submenu, inside: true) }
                        }
                    } label: { Image(systemName: "plus") }.help("Add menu item")
                    icon("square.on.square", "Duplicate item", disabled: selected == nil) {
                        if let selection { self.selection = store.duplicateMenuItem(selection) ?? selection }
                    }
                    Divider().frame(height: 20)
                    icon("arrow.up", "Move up", disabled: !canTransform { $0.move($1, delta: -1) }) { transform { $0.move($1, delta: -1) } }
                    icon("arrow.down", "Move down", disabled: !canTransform { $0.move($1, delta: 1) }) { transform { $0.move($1, delta: 1) } }
                    icon("decrease.indent", "Move out of submenu", disabled: !canTransform { $0.outdent($1) }) { transform { $0.outdent($1) } }
                    icon("increase.indent", "Move into preceding submenu", disabled: !canTransform { $0.indent($1) }) { transform { $0.indent($1) } }
                    Spacer()
                    icon("trash", "Delete item and children", disabled: selected == nil) { remove() }
                }.buttonStyle(.borderless).padding(.horizontal, 16).frame(height: 40).disabled(!store.menuIsEditable)
                Divider()
                List(selection: $store.selectedMenuItemID) {
                    ForEach(store.menu.rows) { row in
                        HStack(spacing: 10) {
                            Image(systemName: row.item.action.icon)
                                .foregroundStyle(row.item.enabled ? InfinityUIKit.Palette.nativeAccent : .secondary)
                                .frame(width: 20)
                            VStack(alignment: .leading, spacing: 3) {
                                Text(row.item.action == .separator ? "Separator" : row.item.label).lineLimit(1)
                                Text(destination(row.item)).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                            }
                            Spacer(minLength: 4)
                            if row.item.checked { Image(systemName: "checkmark").accessibilityLabel("Checked") }
                            if !row.item.enabled { Image(systemName: "nosign").accessibilityLabel("Disabled") }
                            if row.item.action == .submenu { Text("\(row.item.children.count)").foregroundStyle(.secondary).monospacedDigit() }
                        }.padding(.leading, CGFloat(row.depth * 22)).padding(.vertical, 5)
                            .tag(row.id).contextMenu {
                                Button("Duplicate") { selection = store.duplicateMenuItem(row.id) }
                                Button("Delete", role: .destructive) { store.mutateMenu("Menu item deleted") { $0.remove(row.id) } }
                            }
                    }
                }.listStyle(.inset).onDeleteCommand { remove() }
                if !store.menuIsEditable {
                    Label("Infinity Menu is locked", systemImage: "lock.fill").foregroundStyle(.orange).padding(12)
                }
            }.frame(minWidth: 440)
            Group {
                if let selected {
                    MenuItemInspector(store: store, id: selected.id).id(selected.id)
                } else {
                    ContentUnavailableView("Menu Items", systemImage: "list.bullet", description: Text("Select an item to edit."))
                }
            }.frame(minWidth: 290, idealWidth: 330, maxWidth: 390)
                .background(InfinityUIKit.Palette.nativePanel)
        }.background(InfinityUIKit.Palette.nativeCanvas)
            .onAppear { selection = store.menu.items.first?.id }
    }
    // ------------------------=
    // FUNC: icon
    // DESC: Uses native tool symbols with accessible names and stable toolbar targets.
    // ------------------=
    private func icon(_ symbol: String, _ title: String, disabled: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) { Image(systemName: symbol).frame(width: 24, height: 28) }
            .help(title).accessibilityLabel(title).disabled(disabled)
    }
    // ------------------------=
    // FUNC: add
    // DESC: Selects a newly created item without disturbing the selected submenu's descendants.
    // ------------------=
    private func add(_ action: StudioMenuAction, inside: Bool = false) {
        selection = store.addMenuItem(action: action, after: selection, inside: inside) ?? selection
    }
    // ------------------------=
    // FUNC: remove
    // DESC: Removes the selected subtree through the undoable document controller.
    // ------------------=
    private func remove() {
        guard let selection else { return }
        store.mutateMenu("Menu item deleted") { $0.remove(selection) }
    }
    // ------------------------=
    // FUNC: canTransform
    // DESC: Enables reorder controls only when the corresponding tree operation changes state.
    // ------------------=
    private func canTransform(_ operation: (inout StudioMenu, UUID) -> Void) -> Bool {
        guard let selection else { return false }
        var next = store.menu; operation(&next, selection); return next != store.menu
    }
    // ------------------------=
    // FUNC: transform
    // DESC: Applies a hierarchy transformation as one undo unit.
    // ------------------=
    private func transform(_ operation: (inout StudioMenu, UUID) -> Void) {
        guard let selection else { return }
        store.mutateMenu("Menu order updated") { operation(&$0, selection) }
    }
    // ------------------------=
    // FUNC: destination
    // DESC: Presents the actual linked target or a missing-link state in the item outline.
    // ------------------=
    private func destination(_ item: StudioMenuItem) -> String {
        switch item.action {
        case .system: StudioMenuAction.systemTitles.indices.contains(item.target) ? StudioMenuAction.systemTitles[item.target] : "Missing action"
        case .application: StudioMenuAction.applicationTitles.indices.contains(item.target) ? StudioMenuAction.applicationTitles[item.target] : "Missing application"
        case .settings: store.settingsDocument.screens.first { $0.id == item.target }?.title ?? "Missing screen"
        case .screen, .window: "\(item.action.title) · \(store.desktopDocument.screens.first { $0.id == item.target }?.title ?? "Choose a destination")"
        case .submenu: "Submenu"
        case .separator: ""
        }
    }
}

private struct MenuItemInspector: View {
    @ObservedObject var store: TemplateStore
    let id: UUID
    @State private var destinationName = ""
    @State private var createWindow = true
    private var item: StudioMenuItem? { store.menu.rows.first { $0.id == id }?.item }
    var body: some View {
        if let item {
            Form {
                Section("Menu Item") {
                    if item.action != .separator {
                        TextField("Label", text: binding(\.label, fallback: item.label))
                        Toggle("Enabled", isOn: binding(\.enabled, fallback: item.enabled))
                        Toggle("Checked", isOn: binding(\.checked, fallback: item.checked))
                    }
                    Picker("Action", selection: Binding(get: { self.item?.action ?? .system }, set: { store.changeMenuAction(id, to: $0) })) {
                        ForEach(StudioMenuAction.allCases) { action in Text(action.title).tag(action) }
                    }
                }
                if ![.separator, .submenu].contains(item.action) {
                    Section("Destination") {
                        Picker("Target", selection: binding(\.target, fallback: item.target)) {
                            switch item.action {
                            case .system:
                                ForEach(StudioMenuAction.systemTitles.indices, id: \.self) { Text(StudioMenuAction.systemTitles[$0]).tag($0) }
                            case .application:
                                ForEach(StudioMenuAction.applicationTitles.indices, id: \.self) { Text(StudioMenuAction.applicationTitles[$0]).tag($0) }
                            case .settings:
                                ForEach(store.settingsDocument.screens) { Text($0.title).tag($0.id) }
                            default:
                                Text("Choose a screen").tag(0)
                                ForEach(store.desktopDocument.screens.filter { $0.id >= 4 }) { Text($0.title).tag($0.id) }
                            }
                        }
                        if [.settings, .screen, .window].contains(item.action) {
                            Button("Edit Destination", systemImage: "arrow.up.right.square") { store.openMenuDestination(item) }
                                .disabled(item.target == 0)
                        }
                    }
                    Section("Create Destination") {
                        TextField("Name", text: $destinationName)
                        Picker("Presentation", selection: $createWindow) {
                            Text("Window").tag(true)
                            Text("Screen").tag(false)
                        }.pickerStyle(.segmented)
                        Button("Create & Link", systemImage: "plus") {
                            if store.createMenuDestination(for: id, title: destinationName, window: createWindow) != nil { destinationName = "" }
                        }.disabled(destinationName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || destinationName.utf8.count > 63 || store.desktopDocument.screens.count >= 32)
                    }
                }
            }.formStyle(.grouped).disabled(!store.menuIsEditable)
        }
    }
    // ------------------------=
    // FUNC: binding
    // DESC: Routes inspector changes through stable-ID document transactions, including undo and dirty tracking.
    // ------------------=
    private func binding<Value>(_ key: WritableKeyPath<StudioMenuItem, Value>, fallback: Value) -> Binding<Value> {
        Binding(get: { item?[keyPath: key] ?? fallback }, set: { value in store.updateMenuItem(id) { $0[keyPath: key] = value } })
    }
}

private struct MenuPreviewItems: View {
    @ObservedObject var store: TemplateStore
    var items: [StudioMenuItem]
    var body: some View {
        ForEach(items) { item in
            if item.action == .separator { Divider() }
            else if item.action == .submenu {
                Menu(item.label) { MenuPreviewItems(store: store, items: item.children) }.disabled(!item.enabled)
            } else {
                Button { store.openMenuDestination(item) } label: {
                    if item.checked { Label(item.label, systemImage: "checkmark") } else { Text(item.label) }
                }.disabled(!item.enabled)
            }
        }
    }
}
