import AppKit
import SwiftUI

enum StudioCloseChoice { case save, discard, cancel }

@MainActor final class StudioApplicationDelegate: NSObject, NSApplicationDelegate {
    weak var store: TemplateStore?

    // ------------------------=
    // FUNC: applicationShouldTerminate
    // DESC: Applies the same draft and publication guard to Command-Q and application termination.
    // ------------------=
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        guard let store else { return .terminateNow }
        return StudioClosePrompt.allowClose(store) ? .terminateNow : .terminateCancel
    }
}

@MainActor enum StudioClosePrompt {
    // ------------------------=
    // FUNC: allowClose
    // DESC: Presents one standard native save choice without silently discarding another editor section.
    // ------------------=
    static func allowClose(_ store: TemplateStore) -> Bool {
        if store.isoUpdater.isRunning {
            let alert = NSAlert()
            alert.messageText = "An ISO update is still running"
            alert.informativeText = "Wait for it to finish, or cancel it in the ISO update panel before closing Studio."
            alert.addButton(withTitle: "Keep Studio Open")
            alert.runModal()
            return false
        }
        guard store.hasUnsavedChanges else { return true }
        let alert = NSAlert()
        alert.messageText = "Save changes to your OS design?"
        alert.informativeText = "There are unsaved changes in UIKit or the screen collections. Save All preserves every section."
        alert.addButton(withTitle: "Save All")
        alert.addButton(withTitle: "Cancel")
        alert.addButton(withTitle: "Discard Changes")
        let choice: StudioCloseChoice = switch alert.runModal() {
        case .alertFirstButtonReturn: .save
        case .alertThirdButtonReturn: .discard
        default: .cancel
        }
        return store.resolveClose(choice)
    }
}

struct StudioWindowCloseGuard: NSViewRepresentable {
    let store: TemplateStore

    // ------------------------=
    // FUNC: makeNSView
    // DESC: Attaches a transparent lifecycle observer without replacing the editor's native visual content.
    // ------------------=
    func makeNSView(context: Context) -> GuardView { GuardView(store: store) }

    // ------------------------=
    // FUNC: updateNSView
    // DESC: Retains the current document store if SwiftUI reuses the hosting window.
    // ------------------=
    func updateNSView(_ view: GuardView, context: Context) { view.store = store }

    // ------------------------=
    // FUNC: dismantleNSView
    // DESC: Restores the original delegate when SwiftUI removes the lifecycle observer.
    // ------------------=
    static func dismantleNSView(_ view: GuardView, coordinator: ()) { view.uninstall() }

    @MainActor final class GuardView: NSView {
        var store: TemplateStore
        private var bridge: WindowDelegate?
        private weak var attachedWindow: NSWindow?

        // ------------------------=
        // FUNC: init
        // DESC: Holds the live editor state until the hosting view joins a window.
        // ------------------=
        init(store: TemplateStore) { self.store = store; super.init(frame: .zero) }

        // ------------------------=
        // FUNC: init
        // DESC: Rejects archival construction because this lifecycle view requires an explicit store.
        // ------------------=
        required init?(coder: NSCoder) { return nil }

        // ------------------------=
        // FUNC: viewWillMove
        // DESC: Detaches the old window delegate before a hosting view is removed or reparented.
        // ------------------=
        override func viewWillMove(toWindow newWindow: NSWindow?) {
            if newWindow !== attachedWindow { uninstall() }
            super.viewWillMove(toWindow: newWindow)
        }

        // ------------------------=
        // FUNC: viewDidMoveToWindow
        // DESC: Installs a forwarding close delegate while preserving SwiftUI's other window lifecycle callbacks.
        // ------------------=
        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            guard let window, !(window.delegate is WindowDelegate) else { return }
            let delegate = WindowDelegate(owner: self, previous: window.delegate)
            bridge = delegate
            attachedWindow = window
            window.delegate = delegate
        }

        // ------------------------=
        // FUNC: uninstall
        // DESC: Restores only this observer's delegate replacement and releases the original lifecycle owner.
        // ------------------=
        func uninstall() {
            if let window = attachedWindow, let bridge, window.delegate === bridge {
                window.delegate = bridge.previous
            }
            bridge = nil; attachedWindow = nil
        }
    }

    @MainActor final class WindowDelegate: NSObject, NSWindowDelegate {
        weak var owner: GuardView?
        nonisolated(unsafe) var previous: (any NSWindowDelegate)?

        // ------------------------=
        // FUNC: init
        // DESC: Holds the original lifecycle delegate until detachment while keeping the editor reference weak.
        // ------------------=
        init(owner: GuardView, previous: (any NSWindowDelegate)?) {
            self.owner = owner; self.previous = previous
        }

        // ------------------------=
        // FUNC: windowShouldClose
        // DESC: Cancels closing when the user keeps a draft or saving fails, then honors the original delegate.
        // ------------------=
        func windowShouldClose(_ sender: NSWindow) -> Bool {
            guard let owner else { return previous?.windowShouldClose?(sender) ?? true }
            guard StudioClosePrompt.allowClose(owner.store) else { return false }
            return previous?.windowShouldClose?(sender) ?? true
        }

        // ------------------------=
        // FUNC: windowWillClose
        // DESC: Delivers the original close callback and then releases the forwarding delegate to avoid a retained window cycle.
        // ------------------=
        func windowWillClose(_ notification: Notification) {
            previous?.windowWillClose?(notification)
            owner?.uninstall()
        }

        // ------------------------=
        // FUNC: responds
        // DESC: Keeps optional lifecycle callbacks discoverable through the forwarding delegate.
        // ------------------=
        override func responds(to selector: Selector!) -> Bool {
            super.responds(to: selector) || (previous?.responds(to: selector) ?? false)
        }

        // ------------------------=
        // FUNC: forwardingTarget
        // DESC: Forwards unmodified window callbacks to the original owner rather than swallowing them.
        // ------------------=
        override func forwardingTarget(for selector: Selector!) -> Any? {
            previous?.responds(to: selector) == true ? previous : super.forwardingTarget(for: selector)
        }
    }
}
