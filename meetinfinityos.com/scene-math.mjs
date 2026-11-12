// ------------------------=
// FUNC: scenePose
// DESC: Maps elapsed time and native scroll to reversible, reduced-motion-aware axis rotation.
// ------------------=
export function scenePose(time, scroll, paused) {
  if (paused) return { x: 0, y: 0, z: 0, pulse: 0 };
  const p = Math.max(0, Math.min(1, scroll / .58));
  const turn = p * p * (3 - 2 * p);
  return { x: .045 + Math.sin(time * .23) * .045 - Math.sin(turn * Math.PI) * .19,
    y: Math.sin(time * .21) * .32 + turn * Math.PI * 2,
    z: Math.sin(turn * Math.PI * 2) * .055, pulse: time };
}

// ------------------------=
// FUNC: circuitPaths
// DESC: Builds deterministic right-angle and diagonal electrical routes on a stepped backplane behind the glass.
// ------------------=
export function circuitPaths() {
  const paths = [];
  for (const side of [-1, 1]) {
    for (let i = 0; i < 14; i++) {
      const lane = i % 7, bank = i < 7 ? -1 : 1;
      const start = .78 + lane * .09;
      const elbow = 1.28 + (6 - lane) * .24;
      const rise = .66 + lane * .20;
      const shelf = elbow + rise + .42 + (lane % 3) * .33;
      const tip = shelf + .34;
      const z = -1.38 - lane * .035;
      paths.push([[side * start, bank * (.12 + lane * .105), z],
        [side * elbow, bank * (.12 + lane * .105), z],
        [side * (elbow + rise), bank * (.12 + lane * .105 + rise), z],
        [side * shelf, bank * (.12 + lane * .105 + rise), z],
        [side * tip, bank * (.46 + lane * .105 + rise), z],
        [side * (tip + .48 + lane % 3 * .8), bank * (.46 + lane * .105 + rise), z]]);
    }
  }
  return paths;
}
