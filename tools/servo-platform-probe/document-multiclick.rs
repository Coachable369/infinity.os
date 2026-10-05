// Native default selection action, inserted after user-select anchor validation.
#[cfg(infinity_native)]
{
    use crate::dom::bindings::codegen::Bindings::UIEventBinding::UIEventMethods;
    use crate::dom::uievent::UIEvent;
    use servo_base::{Rope, RopeMovement};
    let clicks = event.upcast::<UIEvent>().Detail();
    if clicks == 2 {
        if let Some(text) = container.downcast::<Text>() {
            let rope = Rope::new(text.upcast::<CharacterData>().data().to_string());
            let at = rope.move_by(rope.first_index(), RopeMovement::Character, offset.0 as isize);
            let word = rope.relevant_word_boundaries(at);
            selection.collapse_to_dom_position(cx, &container,
                Utf32CodeUnitsOrNodeOffset(rope.index_to_character_offset(word.start).0));
            selection.collapse_or_extend_to_dom_position(cx, &container,
                Utf32CodeUnitsOrNodeOffset(rope.index_to_character_offset(word.end).0));
        }
    } else if clicks >= 3 {
        // Query the containing block edges at the clicked row. Layout's nearest
        // text search follows wrapped lines and inline nodes rather than newlines.
        let block = container.ancestors().find(|node| node.downcast::<Element>()
            .and_then(|element| element.style()).is_some_and(|style| {
                let display = style.get_box().clone_display();
                !display.is_inline_flow() && !display.is_contents()
            }));
        if let Some(block) = block {
            if let Some(rect) = block.content_box() {
                let window = document.window();
                let mut left = hit_test_result.point_in_frame;
                let mut right = left;
                left.x = rect.min_x().to_f32_px() + 0.5;
                right.x = rect.max_x().to_f32_px() - 0.5;
                let flags = layout_api::HitTestFlags::IncludeDomPosition;
                let start = window.hit_test_from_point_in_viewport(flags, left)
                    .and_then(|hit| hit.dom_position_for_selection);
                let end = window.hit_test_from_point_in_viewport(flags, right)
                    .and_then(|hit| hit.dom_position_for_selection);
                if let (Some((start_node, start_offset)), Some((end_node, end_offset))) = (start, end) {
                    if block.is_inclusive_ancestor_of(&start_node) && block.is_inclusive_ancestor_of(&end_node) &&
                        user_select_contain_node.as_ref().is_none_or(|limit|
                            limit.is_inclusive_ancestor_of(&start_node) && limit.is_inclusive_ancestor_of(&end_node)) {
                        if let (Some((start_node, start_offset, _)), Some((end_node, end_offset, _))) =
                            (adjust_anchor_for_user_select(cx, start_node, start_offset),
                             adjust_anchor_for_user_select(cx, end_node, end_offset)) {
                            selection.collapse_to_dom_position(cx, &start_node, start_offset);
                            selection.collapse_or_extend_to_dom_position(cx, &end_node, end_offset);
                        }
                    }
                }
            }
        }
    }
}
