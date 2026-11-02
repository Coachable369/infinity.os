//! InfinityOS root-page rails, owned by Servo's compositor rather than document scripts or chrome.
use super::*;
#[path = "native_scrollbar_geometry.rs"]
mod geometry;
pub(crate) use geometry::Interaction;
use geometry::{Axis, Pointer, Rect};
use paint_api::display_list::SpatialTreeNodeInfo;

impl WebViewRenderer {
    // ------------------------=
    // FUNC: native_scrollbar_metrics
    // DESC: Reads authoritative root scroll ranges and input sensitivity from the current layout tree.
    // ------------------=
    fn native_scrollbar_metrics(&self) -> Option<(ExternalScrollId, [f32; 2], [f32; 2], [f32; 2])> {
        let pipeline = self.root_pipeline_id?;
        let id = ExternalScrollId(0, pipeline.into());
        let tree = &self.pipelines.get(&pipeline)?.scroll_tree;
        if tree.native_scrollbar_width == 2 { return None; }
        let node = tree.nodes.iter().find(|node| node.external_id() == Some(id))?;
        let SpatialTreeNodeInfo::Scroll(info) = &node.info else { return None; };
        let page = info.clip_rect.size();
        let range = info.content_rect.size() - page;
        Some((id, [page.width, page.height], [
            if info.scroll_sensitivity.x.contains(ScrollType::InputEvents) { range.width.max(0.0) } else { 0.0 },
            if info.scroll_sensitivity.y.contains(ScrollType::InputEvents) { range.height.max(0.0) } else { 0.0 },
        ], [info.offset.x, info.offset.y]))
    }
    // ------------------------=
    // FUNC: native_scrollbar_layout
    // DESC: Keeps rail hit geometry fixed in device pixels while content zoom changes actual scroll ranges.
    // ------------------=
    fn native_scrollbar_layout(&self) -> [Option<geometry::Rail>; 2] {
        let Some((_, page, maximum, offset)) = self.native_scrollbar_metrics() else { return [None, None]; };
        geometry::layout_styled(Rect { x: self.rect.min.x, y: self.rect.min.y, width: self.rect.width(), height: self.rect.height() },
            page, maximum, offset, self.hidpi_scale_factor.get(), self.native_scrollbars_thin())
    }
    // ------------------------=
    // FUNC: native_scrollbars_thin
    // DESC: Reads the root's computed CSS width without querying or modifying the DOM.
    // ------------------=
    fn native_scrollbars_thin(&self) -> bool {
        self.root_pipeline_id.and_then(|id| self.pipelines.get(&id))
            .is_some_and(|pipeline| pipeline.scroll_tree.native_scrollbar_width == 1)
    }
    // ------------------------=
    // FUNC: native_scrollbar_event
    // DESC: Intercepts physical rail gestures before DOM dispatch and applies offsets through Servo's input-authorized scroll tree.
    // ------------------=
    pub(crate) fn native_scrollbar_event(&mut self, event: &InputEvent) -> (bool, bool) {
        if self.native_scrollbar_pipeline != self.root_pipeline_id {
            self.native_scrollbar_input = Interaction::default();
            self.native_scrollbar_pipeline = self.root_pipeline_id;
        }
        let kind = match event {
            InputEvent::MouseButton(button) if button.button == MouseButton::Primary => match button.action {
                MouseButtonAction::Down => Pointer::Down,
                MouseButtonAction::Up => Pointer::Up,
                _ => return (false, false),
            },
            InputEvent::MouseMove(_) => Pointer::Move,
            InputEvent::MouseLeftViewport(_) => Pointer::Leave,
            _ => return (false, false),
        };
        let point = event.point().map(|point| point.as_device_point(self.device_pixels_per_page_pixel())).unwrap_or_default();
        let rails = self.native_scrollbar_layout();
        let outcome = self.native_scrollbar_input.pointer(rails, kind, point.x, point.y);
        if let (Some((axis, value)), Some((id, _, _, offset)), Some(pipeline)) =
            (outcome.scroll, self.native_scrollbar_metrics(), self.root_pipeline_id) {
            let next = match axis { Axis::Horizontal => LayoutVector2D::new(value, offset[1]), Axis::Vertical => LayoutVector2D::new(offset[0], value) };
            if let Some(details) = self.pipelines.get_mut(&pipeline) {
                details.scroll_tree.set_scroll_offset_for_node_with_external_scroll_id(id, next, ScrollType::InputEvents);
            }
            self.send_scroll_positions_to_layout_for_pipeline(pipeline, id);
        }
        (outcome.consumed, outcome.repaint)
    }
    // ------------------------=
    // FUNC: paint_native_scrollbars
    // DESC: Reuses the kit's navy rail, eight-pixel rounded thumb and blue hover/pressed colors inside the engine viewport.
    // ------------------=
    pub(crate) fn paint_native_scrollbars(&self, builder: &mut webrender_api::DisplayListBuilder,
        spatial_id: webrender_api::SpatialId, clip_chain_id: webrender_api::ClipChainId) {
        use webrender_api::{BorderRadius, ClipMode, ColorF, CommonItemProperties, ComplexClipRegion, PrimitiveFlags};
        use webrender_api::units::{LayoutPoint, LayoutRect, LayoutSize};
        let scale = self.hidpi_scale_factor.get();
        let inset = if self.native_scrollbars_thin() { 2.0 } else { 3.0 } * scale;
        for rail in self.native_scrollbar_layout().into_iter().flatten() {
            let active = self.native_scrollbar_input.pressed == Some(rail.axis);
            let hover = self.native_scrollbar_input.hover == Some(rail.axis);
            let mut thumb = rail.thumb;
            if rail.axis == Axis::Vertical { thumb.x += inset; thumb.width -= 2.0 * inset; }
            else { thumb.y += inset; thumb.height -= 2.0 * inset; }
            let color = if active { (62, 111, 255) } else if hover { (134, 199, 255) } else { (108, 137, 174) };
            for (rect, color, rounded) in [(rail.track, (16, 27, 58), false), (thumb, color, true)] {
                let bounds = LayoutRect::from_origin_and_size(LayoutPoint::new(rect.x, rect.y), LayoutSize::new(rect.width, rect.height));
                let clip = if rounded {
                    let id = builder.define_clip_rounded_rect(spatial_id, ComplexClipRegion { rect: bounds,
                        radii: BorderRadius::uniform(if self.native_scrollbars_thin() { 2.0 } else { 4.0 } * scale), mode: ClipMode::Clip });
                    builder.define_clip_chain(Some(clip_chain_id), [id])
                } else { clip_chain_id };
                let common = CommonItemProperties { clip_rect: bounds, spatial_id, clip_chain_id: clip, flags: PrimitiveFlags::default() };
                builder.push_rect(&common, bounds, ColorF::new(color.0 as f32 / 255.0, color.1 as f32 / 255.0, color.2 as f32 / 255.0, 1.0));
            }
        }
    }
}
