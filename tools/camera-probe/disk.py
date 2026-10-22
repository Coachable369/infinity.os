"""Package the bounded firmware descriptor probe using the existing GPT builder."""
import importlib.util
from pathlib import Path

root = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("timer_disk", root / "tools/arm-capture-timer-probe/run.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
module.OUT = root / "build/camera-probe/gpt"
module.OUT.mkdir(parents=True, exist_ok=True)
# The reused disk builder copies a kernel companion. This diagnostic never
# executes it: the EFI application reads descriptors and returns to firmware.
(module.OUT / "probe.elf").write_bytes(b"Camera descriptor diagnostic: no kernel payload")
module.disk_image(root / "build/camera-probe/BOOTAA64.EFI")
