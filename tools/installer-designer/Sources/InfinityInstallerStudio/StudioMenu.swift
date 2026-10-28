import Foundation

enum StudioMenuAction: UInt8, Codable, CaseIterable, Identifiable, Sendable {
    case system = 0, settings, application, screen, window, submenu, separator
    var id: Self { self }
    var title: String {
        switch self {
        case .system: "System action"
        case .settings: "Settings screen"
        case .application: "Open application"
        case .screen: "Open screen"
        case .window: "Open window"
        case .submenu: "Submenu"
        case .separator: "Separator"
        }
    }
    var icon: String {
        switch self {
        case .system: "gearshape"
        case .settings: "slider.horizontal.3"
        case .application: "app"
        case .screen: "rectangle"
        case .window: "macwindow"
        case .submenu: "list.bullet.indent"
        case .separator: "minus"
        }
    }
    static let systemTitles = ["About InfinityOS", "System Settings", "Users & Accounts", "AI & Voice",
        "Network", "Privacy & Security", "Lock Screen", "Log Out", "Restart", "Shut Down"]
    static let applicationTitles = ["Command Window", "Text Editor", "File Navigator", "Infinity Browser", "App Library"]
}

struct StudioMenuItem: Identifiable, Codable, Hashable, Sendable {
    var id = UUID()
    var label: String
    var action: StudioMenuAction = .system
    var target: Int = 0
    var enabled = true
    var checked = false
    var children: [StudioMenuItem] = []
}

struct StudioMenu: Codable, Hashable, Sendable {
    var items: [StudioMenuItem]
    static let maximumItems = 128
    static let maximumDepth = 4
    static let factory = StudioMenu(items: StudioMenuAction.systemTitles.enumerated().map {
        StudioMenuItem(label: $0.element, target: $0.offset)
    })

    struct Row: Identifiable {
        var item: StudioMenuItem
        var depth: Int
        var id: UUID { item.id }
    }
    var rows: [Row] { Self.flatten(items, depth: 0) }

    // ------------------------=
    // FUNC: flatten
    // DESC: Projects the saved hierarchy in display order without losing stable item identities.
    // ------------------=
    private static func flatten(_ items: [StudioMenuItem], depth: Int) -> [Row] {
        items.flatMap { [Row(item: $0, depth: depth)] + flatten($0.children, depth: depth + 1) }
    }
    // ------------------------=
    // FUNC: update
    // DESC: Updates one stable item in place, preserving its sibling order and descendants.
    // ------------------=
    @discardableResult mutating func update(_ id: UUID, mutation: (inout StudioMenuItem) -> Void) -> Bool {
        Self.edit(&items, id: id) { siblings, index in mutation(&siblings[index]) }
    }
    // ------------------------=
    // FUNC: edit
    // DESC: Locates the containing sibling list for one atomic tree operation.
    // ------------------=
    @discardableResult private static func edit(_ items: inout [StudioMenuItem], id: UUID,
                                                operation: (inout [StudioMenuItem], Int) -> Void) -> Bool {
        if let index = items.firstIndex(where: { $0.id == id }) { operation(&items, index); return true }
        for i in items.indices {
            if edit(&items[i].children, id: id, operation: operation) { return true }
        }
        return false
    }
    // ------------------------=
    // FUNC: insert
    // DESC: Inserts after a selected sibling, or within a selected submenu, with bounded total size.
    // ------------------=
    @discardableResult mutating func insert(_ item: StudioMenuItem, after id: UUID?, inside: Bool = false) -> Bool {
        guard rows.count + Self.flatten([item], depth: 0).count <= Self.maximumItems else { return false }
        if let id {
            if inside {
                guard let row = rows.first(where: { $0.id == id }), row.item.action == .submenu,
                      row.depth + 1 < Self.maximumDepth else { return false }
                return update(id) { $0.children.append(item) }
            }
            return Self.edit(&items, id: id) { $0.insert(item, at: $1 + 1) }
        }
        items.append(item)
        return true
    }
    // ------------------------=
    // FUNC: remove
    // DESC: Removes an item and its entire subtree without dangling child references.
    // ------------------=
    mutating func remove(_ id: UUID) { Self.edit(&items, id: id) { $0.remove(at: $1) } }
    // ------------------------=
    // FUNC: move
    // DESC: Reorders only siblings and leaves child subtrees attached.
    // ------------------=
    mutating func move(_ id: UUID, delta: Int) {
        Self.edit(&items, id: id) { siblings, index in
            let destination = index + delta
            guard siblings.indices.contains(destination) else { return }
            let item = siblings.remove(at: index); siblings.insert(item, at: destination)
        }
    }
    // ------------------------=
    // FUNC: indent
    // DESC: Moves an item under its preceding submenu, rejecting excessive nesting.
    // ------------------=
    mutating func indent(_ id: UUID) {
        guard let row = rows.first(where: { $0.id == id }),
              row.depth + (Self.flatten([row.item], depth: 0).map(\.depth).max() ?? 0) + 1 < Self.maximumDepth else { return }
        Self.edit(&items, id: id) { siblings, index in
            guard index > 0, siblings[index - 1].action == .submenu else { return }
            let item = siblings.remove(at: index); siblings[index - 1].children.append(item)
        }
    }
    // ------------------------=
    // FUNC: outdent
    // DESC: Moves a child immediately after its parent while retaining the child's descendants.
    // ------------------=
    mutating func outdent(_ id: UUID) { Self.promote(&items, id: id) }
    // ------------------------=
    // FUNC: promote
    // DESC: Finds the direct parent before promoting a child one hierarchy level.
    // ------------------=
    @discardableResult private static func promote(_ items: inout [StudioMenuItem], id: UUID) -> Bool {
        for i in items.indices {
            if let child = items[i].children.firstIndex(where: { $0.id == id }) {
                let item = items[i].children.remove(at: child); items.insert(item, at: i + 1); return true
            }
            if promote(&items[i].children, id: id) { return true }
        }
        return false
    }
    // ------------------------=
    // FUNC: duplicate
    // DESC: Copies a complete subtree with fresh stable identities and unchanged destination links.
    // ------------------=
    static func duplicate(_ item: StudioMenuItem) -> StudioMenuItem {
        var copy = item; copy.id = UUID(); copy.children = item.children.map(duplicate); return copy
    }
    // ------------------------=
    // FUNC: validate
    // DESC: Rejects malformed hierarchy, unbounded labels and destinations that cannot execute in the desktop session.
    // ------------------=
    func validate(screens: [InstallerScreenTemplate]) throws {
        let flat = rows
        guard flat.count <= Self.maximumItems, Set(flat.map(\.id)).count == flat.count,
              flat.allSatisfy({ $0.depth < Self.maximumDepth }) else {
            throw TemplateValidationIssue.invalidDocument("Menu exceeds 128 items / four levels, or repeats an item ID")
        }
        for row in flat {
            let item = row.item
            guard item.label.utf8.count <= 63, !item.label.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) }),
                  item.action == .separator || !item.label.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
                  item.action == .submenu || item.children.isEmpty else {
                throw TemplateValidationIssue.invalidDocument("Menu labels require 1-63 UTF-8 bytes; only submenus may contain children")
            }
            let valid: Bool
            switch item.action {
            case .system: valid = StudioMenuAction.systemTitles.indices.contains(item.target)
            case .settings: valid = (1...ScreenCollection.systemSettings.requiredScreenCount).contains(item.target)
            case .application: valid = StudioMenuAction.applicationTitles.indices.contains(item.target)
            case .screen, .window: valid = item.target >= 4 && screens.contains { $0.id == item.target }
            case .submenu, .separator: valid = item.target == 0
            }
            guard valid else { throw TemplateValidationIssue.invalidDocument("Menu item '\(item.label)' has a missing or invalid destination") }
        }
    }
}

extension InstallerStudioDocument {
    // ------------------------=
    // FUNC: addingMenuScreen
    // DESC: Creates an editable desktop destination without cloning native status/control bindings.
    // ------------------=
    mutating func addingMenuScreen(title: String) -> Int? {
        let name = title.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty, name.utf8.count <= 63, screens.count < Self.maximumScreenCount else { return nil }
        let id = screens.count + 1
        let panel = StudioElement.make(name: "Content", kind: .panel, role: .console,
            frame: CanvasRect(x: 0, y: 0, width: 1000, height: 1000))
        let heading = StudioElement.make(name: "Title", kind: .text, role: .title,
            frame: CanvasRect(x: 48, y: 48, width: 904, height: 64), text: name, zIndex: 1)
        screens.append(InstallerScreenTemplate(id: id, title: name, elements: [panel, heading]))
        return id
    }
}
