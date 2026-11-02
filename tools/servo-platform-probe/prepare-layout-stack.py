"""Stage bounded native layout recursion checks in the pinned shared engine."""
from pathlib import Path
import subprocess


# ------------------------=
# FUNC: replace_once
# DESC: Rejects upstream drift instead of silently omitting a required recursion boundary.
# ------------------=
def replace_once(text, old, new):
    if text.count(old) != 1:
        raise RuntimeError("Native layout stack integration changed: " + old[:100])
    return text.replace(old, new, 1)


# ------------------------=
# FUNC: apply
# DESC: Rebuilds only the owned layout overlays from pinned source and inserts native-only checks before recursive calls.
# ------------------=
def apply(servo):
    here = Path(__file__).resolve().parent
    relative = "components/layout/lib.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = replace_once(text, "mod dom_traversal;", "mod dom_traversal;\n#[cfg(infinity_native)]\nmod native_layout_stack;")
    (servo / relative).write_text(text)
    (servo / "components/layout/native_layout_stack.rs").write_text(
        (here / "native-layout-stack.rs").read_text())

    relative = "components/layout/dom_traversal.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    # Keep the recursive callee's frame below the check, even in optimized builds.
    text = replace_once(text, "fn traverse_children_of<'dom>(",
                        "#[cfg_attr(infinity_native, inline(never))]\nfn traverse_children_of<'dom>(")
    text = replace_once(text, "    pub(crate) fn traverse<'dom>(",
                        "    #[cfg_attr(infinity_native, inline(never))]\n    pub(crate) fn traverse<'dom>(")
    text = replace_once(text,
                        "                traverse_children_of(&info, context, handler);",
                        "                #[cfg(infinity_native)]\n                crate::native_layout_stack::require();\n"
                        "                traverse_children_of(&info, context, handler);")
    (servo / relative).write_text(text)

    relative = "components/layout/flow/construct.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = replace_once(text, "impl BlockContainer {\n    pub fn construct(",
                        "impl BlockContainer {\n    #[cfg_attr(infinity_native, inline(never))]\n    pub fn construct(")
    text = replace_once(text,
                        "        non_replaced_contents.traverse(self.context, info, self);",
                        "        #[cfg(infinity_native)]\n        crate::native_layout_stack::require();\n"
                        "        non_replaced_contents.traverse(self.context, info, self);")
    text = replace_once(text,
                        "            } => BlockContainer::construct(context, info, contents, propagated_data, is_list_item),",
                        "            } => {\n                #[cfg(infinity_native)]\n"
                        "                crate::native_layout_stack::require();\n"
                        "                BlockContainer::construct(context, info, contents, propagated_data, is_list_item)\n"
                        "            },")
    (servo / relative).write_text(text)
