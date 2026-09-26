"""Avoid resolving recursive template layouts that have no bitfields to allocate."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tomllib

# ------------------------=
# FUNC: main
# DESC: Preserves real bitfield allocation while avoiding unnecessary template layout recursion.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    spec = importlib.util.spec_from_file_location("staging", Path(__file__).with_name("prepare-async-net.py"))
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    servo = root / "build/servo-port-audit"
    original = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:Cargo.lock"], text=True)
    directory, package = helper.stage(root, tomllib.loads(original)["package"], "bindgen", "0.72.1")
    path = directory / "ir/context.rs"
    text = path.read_text()
    marker = '                let layout = ty.layout(ctx);'
    if text.count(marker) != 1:
        raise SystemExit("Unexpected bindgen bitfield layout traversal")
    text = text.replace(marker, '''                // Non-bitfield fields are transferred unchanged by CompFields.
                // Do not resolve recursive template layouts while this item is loaned.
                let layout = if ty.as_comp_mut().unwrap().needs_bitfield_layout() {
                    ty.layout(ctx)
                } else { None };''', 1)
    path.write_text(text)
    path = directory / "ir/comp.rs"
    text = path.read_text()
    marker = '    /// Compute this compound structure\'s bitfield allocation units.'
    if text.count(marker) != 1:
        raise SystemExit("Unexpected compound bitfield entry")
    text = text.replace(marker, '''    // ------------------------=
    // FUNC: needs_bitfield_layout
    // DESC: Only real bitfields need packing information during field normalization.
    // ------------------=
    pub(crate) fn needs_bitfield_layout(&self) -> bool {
        match &self.fields {
            CompFields::Before(raws) => raws.iter().any(|field| field.bitfield_width().is_some()),
            _ => panic!("Bitfields already normalized"),
        }
    }

''' + marker, 1)
    path.write_text(text)
    lock = servo / "Cargo.lock"
    header = 'name = "bindgen"\nversion = "0.72.1"\n'
    entry = header + 'source = "' + package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n'
    lock.write_text(lock.read_text().replace(entry, header))

if __name__ == "__main__":
    main()
