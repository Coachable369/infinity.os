// ------------------------=
// FUNC: scenePose
// DESC: Maps elapsed time and native scroll to reversible, reduced-motion-aware axis rotation.
// ------------------=
export function scenePose(time, scroll, paused) {
  if (paused) return { x: -.12, y: -.22, z: -.035, pulse: 0 };
  const progress = Math.max(0, Math.min(1, scroll));
  return { x: -.12 + Math.sin(time * .19) * .055 + progress * .27,
    y: -.22 + Math.sin(time * .12) * .24 + progress * Math.PI * 2.4,
    z: -.035 + Math.sin(time * .14) * .025, pulse: time };
}

// ------------------------=
// FUNC: circuitPaths
// DESC: Builds deterministic right-angle and diagonal electrical routes across a ground plane.
// ------------------=
export function circuitPaths() {
  const paths = [];
  for (const side of [-1, 1]) {
    for (let i = 0; i < 14; i++) {
      const z = -3.1 + i * .44;
      const bend = 2.9 + (i % 4) * .43;
      paths.push([[side * .55, -1.95, z * .24], [side * 1.8, -1.95, z * .24],
        [side * bend, -1.95, z], [side * (bend + 1.1), -1.95, z],
        [side * (6.3 + i % 3), -1.95, z + (i - 6.5) * .27]]);
    }
  }
  return paths;
}
