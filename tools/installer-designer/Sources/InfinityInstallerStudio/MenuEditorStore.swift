import Foundation

extension TemplateStore {
    var menu: StudioMenu { desktopDocument.infinityMenu ?? .factory }
    var menuIsEditable: Bool {
        desktopDocument.screens.first?.elements.first(where: { $0.role == .desktopBrand }).map { !$0.locked } ?? false
    }
    // ------------------------=
    // FUNC: editInfinityMenu
    // DESC: Opens the authored menu workspace without changing the document merely by inspecting it.
    // ------------------=
    func editInfinityMenu() {
        selectScreenCollection(.desktop, screen: 1)
        selectedElementID = selectedScreen?.elements.first(where: { $0.role == .desktopBrand })?.id
        editingMenu = true
        selectedMenuItemID = menu.items.first?.id
    }
    // ------------------------=
    // FUNC: deleteSelectedMenuItem
    // DESC: Routes keyboard deletion to the selected menu subtree rather than the underlying desktop layer.
    // ------------------=
    func deleteSelectedMenuItem() {
        guard editingMenu, let id = selectedMenuItemID else { return }
        mutateMenu("Menu item deleted") { $0.remove(id) }
    }
    // ------------------------=
    // FUNC: mutateMenu
    // DESC: Records each hierarchy or property edit as one shared undoable document transaction.
    // ------------------=
    func mutateMenu(_ label: String, mutation: (inout StudioMenu) -> Void) {
        guard menuIsEditable else { status = "Unlock Infinity Menu to edit its items"; return }
        var updated = menu; mutation(&updated)
        guard updated != menu else { return }
        recordUndo(); desktopDocument.infinityMenu = updated; status = label
    }
    // ------------------------=
    // FUNC: updateMenuItem
    // DESC: Updates a menu item by stable identity rather than current selection or outline index.
    // ------------------=
    func updateMenuItem(_ id: UUID, mutation: (inout StudioMenuItem) -> Void) {
        mutateMenu("Menu item updated") { $0.update(id, mutation: mutation) }
    }
    // ------------------------=
    // FUNC: changeMenuAction
    // DESC: Switches typed destinations without silently deleting a populated submenu.
    // ------------------=
    func changeMenuAction(_ id: UUID, to action: StudioMenuAction) {
        guard let item = menu.rows.first(where: { $0.id == id })?.item else { return }
        guard item.children.isEmpty || action == .submenu else { status = "Move or remove submenu children before changing its action"; return }
        updateMenuItem(id) {
            $0.action = action
            $0.target = action == .settings ? 1 : (action == .screen || action == .window ? (desktopDocument.screens.first { $0.id >= 4 }?.id ?? 0) : 0)
            if action == .separator { $0.label = ""; $0.checked = false }
            else if $0.label.isEmpty { $0.label = action.title }
        }
    }
    // ------------------------=
    // FUNC: addMenuItem
    // DESC: Inserts an item or separator at a predictable hierarchy location and returns its stable selection ID.
    // ------------------=
    @discardableResult func addMenuItem(action: StudioMenuAction, after id: UUID?, inside: Bool = false) -> UUID? {
        let item = StudioMenuItem(label: action == .separator ? "" : action == .submenu ? "Submenu" : "Menu item", action: action)
        var inserted = false
        mutateMenu("Menu item added") { inserted = $0.insert(item, after: id, inside: inside) }
        if !inserted { status = "Item could not be added: check the menu size and nesting limit" }
        return inserted ? item.id : nil
    }
    // ------------------------=
    // FUNC: duplicateMenuItem
    // DESC: Duplicates a selected subtree as one undoable edit with fresh IDs.
    // ------------------=
    @discardableResult func duplicateMenuItem(_ id: UUID) -> UUID? {
        guard let item = menu.rows.first(where: { $0.id == id })?.item else { return nil }
        let copy = StudioMenu.duplicate(item); var inserted = false
        mutateMenu("Menu item duplicated") { inserted = $0.insert(copy, after: id) }
        return inserted ? copy.id : nil
    }
    // ------------------------=
    // FUNC: createMenuDestination
    // DESC: Atomically creates a real desktop canvas and links it to the requested item.
    // ------------------=
    @discardableResult func createMenuDestination(for itemID: UUID, title: String, window: Bool) -> Int? {
        guard menuIsEditable, let item = menu.rows.first(where: { $0.id == itemID })?.item, item.children.isEmpty else { return nil }
        var updated = desktopDocument
        guard let id = updated.addingMenuScreen(title: title) else { status = "Use a name of 1-63 UTF-8 bytes; at most 32 desktop screens are supported"; return nil }
        var navigation = menu
        navigation.update(itemID) { $0.action = window ? .window : .screen; $0.target = id }
        updated.infinityMenu = navigation
        recordUndo(); desktopDocument = updated; status = "Destination created and linked"
        return id
    }
    // ------------------------=
    // FUNC: openMenuDestination
    // DESC: Opens the linked editable canvas; preview never executes host system or session actions.
    // ------------------=
    func openMenuDestination(_ item: StudioMenuItem) {
        switch item.action {
        case .screen, .window:
            guard desktopDocument.screens.contains(where: { $0.id == item.target }) else { status = "Choose a destination first"; return }
            selectScreenCollection(.desktop, screen: item.target)
        case .settings: selectScreenCollection(.systemSettings, screen: item.target)
        default: status = "Native action: \(item.label). Host execution is disabled in Studio preview."
        }
    }
    // ------------------------=
    // FUNC: removeMenuScreen
    // DESC: Removes only unreferenced authored screens and remaps later destination IDs in the same transaction.
    // ------------------=
    func removeMenuScreen() {
        let id = selectedScreenID
        guard id >= 4, let index = desktopDocument.screens.firstIndex(where: { $0.id == id }) else { return }
        guard !menu.rows.contains(where: { [.screen, .window].contains($0.item.action) && $0.item.target == id }) else {
            status = "This screen is linked from a menu item. Reassign or remove that link first."; return
        }
        recordUndo(); desktopDocument.screens.remove(at: index)
        for i in desktopDocument.screens.indices { desktopDocument.screens[i].id = i + 1 }
        if var navigation = desktopDocument.infinityMenu {
            for row in navigation.rows where [.screen, .window].contains(row.item.action) && row.item.target > id {
                navigation.update(row.id) { $0.target -= 1 }
            }
            desktopDocument.infinityMenu = navigation
        }
        selectedScreenID = min(id, desktopDocument.screens.count); selectedElementID = nil; status = "Screen removed"
    }
}
