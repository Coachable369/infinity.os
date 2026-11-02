"""Stage native compositor scrollbars for the pinned engine that has no scrollbar implementation."""
from pathlib import Path
import subprocess


# ------------------------=
# FUNC: replace_once
# DESC: Fails preparation on an upstream integration change instead of silently omitting a required hook.
# ------------------=
def replace_once(text, old, new):
    if text.count(old) != 1:
        raise RuntimeError("Native scrollbar integration anchor changed: " + old[:100])
    return text.replace(old, new, 1)


# ------------------------=
# FUNC: apply
# DESC: Adds one engine-owned root rail implementation; no DOM injection, duplicate browser overlay or host service.
# ------------------=
def apply(servo):
    here = Path(__file__).resolve().parent
    paint = servo / "components/paint"
    (paint / "native_scrollbars.rs").write_text((here / "native-scrollbars.rs").read_text())
    (paint / "native_scrollbar_geometry.rs").write_text((here.parents[1] / "sdk/infinity-browser-core/scrollbars.rs").read_text())
    relative = "components/paint/webview_renderer.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text += '\n#[cfg(infinity_native)]\n#[path = "native_scrollbars.rs"]\nmod native_scrollbars;\n'
    text = replace_once(text, "    pending_wheel_events: FxHashMap<InputEventId, WheelEvent>,", """    pending_wheel_events: FxHashMap<InputEventId, WheelEvent>,
    #[cfg(infinity_native)]
    native_scrollbar_input: native_scrollbars::Interaction,
    #[cfg(infinity_native)]
    native_scrollbar_pipeline: Option<PipelineId>,""")
    text = replace_once(text, "            pending_wheel_events: Default::default(),", """            pending_wheel_events: Default::default(),
            #[cfg(infinity_native)]
            native_scrollbar_input: Default::default(),
            #[cfg(infinity_native)]
            native_scrollbar_pipeline: None,""")
    (servo / relative).write_text(text)
    relative = "components/shared/paint/display_list.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = replace_once(text, "pub struct ScrollTree {", """pub struct ScrollTree {
    #[cfg(infinity_native)]
    pub native_scrollbar_width: u8,""")
    (servo / relative).write_text(text)
    relative = "components/layout/display_list/stacking_context.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = replace_once(text, "        let paint_info = PaintDisplayListInfo::new(", "        let mut paint_info = PaintDisplayListInfo::new(")
    text = replace_once(text, "        let root_scroll_node_id = paint_info.root_scroll_node_id;", """        #[cfg(infinity_native)]
        if let Some(root) = fragment_tree.root_box_fragment() {
            use style::computed_values::scrollbar_width::T as ScrollbarWidth;
            paint_info.scroll_tree.native_scrollbar_width = match root.style().get_ui().scrollbar_width {
                ScrollbarWidth::Thin => 1,
                ScrollbarWidth::None => 2,
                _ => 0,
            };
        }
        let root_scroll_node_id = paint_info.root_scroll_node_id;""")
    (servo / relative).write_text(text)
    # Only this now-implemented property is enabled; other experimental layout stays disabled.
    style = servo.parent / "servo-native-deps/stylo/style/properties/longhands.toml"
    text = style.read_text()
    old = '[scrollbar-width]\nstruct = "ui"\nservo_pref = "layout.unimplemented"\n'
    new = '[scrollbar-width]\nstruct = "ui"\n'
    if old in text:
        text = replace_once(text, old, new)
    elif text.count(new) != 1:
        raise RuntimeError("Pinned scrollbar-width property changed")
    style.write_text(text)


# ------------------------=
# FUNC: patch_painter
# DESC: Composes rail hooks with the software renderer's source in its owning staging step.
# ------------------=
def patch_painter(text):
    text = replace_once(text, "                    if offset == LayoutVector2D::zero() {", "                    if !cfg!(infinity_native) && offset == LayoutVector2D::zero() {")
    text = replace_once(text, "        let built_display_list = builder.end();", """        #[cfg(infinity_native)]
        for webview_renderer in self.webview_renderers.values().filter(|view| !view.hidden()) {
            webview_renderer.paint_native_scrollbars(&mut builder, root_reference_frame, clip_chain_id);
        }
        let built_display_list = builder.end();""")
    text = replace_once(text, "        if need_zoom {\n            self.send_root_pipeline_display_list_in_transaction(&mut transaction);", "        if need_zoom || cfg!(infinity_native) {\n            self.send_root_pipeline_display_list_in_transaction(&mut transaction);")
    text = replace_once(text, "        if is_root_pipeline && old_scale != webview_renderer.device_pixels_per_page_pixel() {", "        if is_root_pipeline && (cfg!(infinity_native) || old_scale != webview_renderer.device_pixels_per_page_pixel()) {")
    text = replace_once(text, """        self.generate_frame(&mut transaction, RenderReasons::APZ);
        self.send_transaction(transaction);
    }

    pub(crate) fn scroll_viewport_by_delta""", """        #[cfg(infinity_native)]
        self.send_root_pipeline_display_list_in_transaction(&mut transaction);
        self.generate_frame(&mut transaction, RenderReasons::APZ);
        self.send_transaction(transaction);
    }

    pub(crate) fn scroll_viewport_by_delta""")
    text = replace_once(text, """        event: InputEventAndId,
    ) -> bool {
        self.webview_renderers""", """        event: InputEventAndId,
    ) -> bool {
        #[cfg(infinity_native)]
        {
            let (consumed, changed) = self.webview_renderers.get_mut(&webview_id)
                .map(|view| view.native_scrollbar_event(&event.event)).unwrap_or((false, false));
            if changed { self.send_root_pipeline_display_list(); }
            if consumed { return true; }
        }
        self.webview_renderers""")
    return text
