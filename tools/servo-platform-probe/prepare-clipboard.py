"""Native transactional cut and clipboard protection overlay for the pinned engine."""
import subprocess


# ------------------------=
# FUNC: patch
# DESC: Layers clipboard edits over freshly staged shared files or pinned upstream and rejects drifted anchors.
# ------------------=
def patch(root, relative, replacements):
    if relative in ("components/shared/embedder/lib.rs", "components/servo/servo.rs"):
        text = (root / relative).read_text()
    else:
        text = subprocess.check_output(["git", "-C", str(root), "show", "HEAD:" + relative], text=True)
    for old, new in replacements:
        if text.count(old) != 1:
            raise RuntimeError("Clipboard integration anchor changed: " + relative)
        text = text.replace(old, new)
    (root / relative).write_text(text)


# ------------------------=
# FUNC: apply
# DESC: Requires clipboard acceptance before cut deletion and keeps password selections out of native copy operations.
# ------------------=
def apply(root):
    patch(root, "components/shared/embedder/lib.rs", [(
        "    SetClipboardText(WebViewId, String),",
        "    SetClipboardText(WebViewId, String),\n    SetClipboardTextChecked(WebViewId, String, GenericCallback<bool>),"
    )])
    patch(root, "components/servo/clipboard_delegate.rs", [(
        "    fn set_text(&self, _webview: WebView, _new_contents: String) {}",
        """    fn set_text(&self, _webview: WebView, _new_contents: String) {}
    // ------------------------=
    // FUNC: try_set_text
    // DESC: Rejects transactional cuts unless the embedder explicitly acknowledges clipboard storage.
    // ------------------=
    fn try_set_text(&self, _webview: WebView, _new_contents: String) -> bool { false }"""
    ), (
        "    fn set_text(&self, _webview: WebView, new_contents: String) {\n        clipboard::set_text(new_contents);\n    }",
        """    fn set_text(&self, _webview: WebView, new_contents: String) {
        clipboard::set_text(new_contents);
    }
    // ------------------------=
    // FUNC: try_set_text
    // DESC: Preserves the in-engine fallback clipboard contract for non-native test embedders.
    // ------------------=
    fn try_set_text(&self, webview: WebView, new_contents: String) -> bool {
        self.set_text(webview, new_contents); true
    }"""
    )])
    patch(root, "components/servo/servo.rs", [(
        "            EmbedderMsg::SetCursor(webview_id, cursor) => {",
        """            EmbedderMsg::SetClipboardTextChecked(webview_id, string, reply) => {
                let accepted = self.get_webview_handle(webview_id)
                    .is_some_and(|view| view.clipboard_delegate().try_set_text(view, string));
                let _ = reply.send(accepted);
            },
            EmbedderMsg::SetCursor(webview_id, cursor) => {"""
    )])
    patch(root, "components/script/dom/document/editing.rs", [(
        """                        self.send_to_embedder(EmbedderMsg::SetClipboardText(
                            self.webview_id(),
                            selection,
                        ));

                        // Step 3.1.2.""",
        """                        let Ok((reply, received)) = GenericCallback::new_blocking() else {
                            return InputEventResult::empty();
                        };
                        self.send_to_embedder(EmbedderMsg::SetClipboardTextChecked(
                            self.webview_id(), selection, reply,
                        ));
                        if !matches!(received.recv(), Ok(true)) { return InputEventResult::empty(); }

                        // Step 3.1.2."""
    ), (
        """            // Step 1.1 Clear the clipboard.
            self.send_to_embedder(EmbedderMsg::ClearClipboard(self.webview_id()));""",
        """            // The native text write atomically replaces contents; do not consume
            // the one-shot copy grant with a preliminary clear."""
    ), (
        """    pub(crate) fn selection_content(&self, cx: &mut JSContext) -> Option<String> {
        match self {""",
        """    pub(crate) fn selection_content(&self, cx: &mut JSContext) -> Option<String> {
        if let Self::TextControl(element) = self {
            if element.text_control_element().is_password_field() { return None; }
        }
        match self {"""
    )])
