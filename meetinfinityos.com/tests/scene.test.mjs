import assert from "node:assert/strict";
import { test } from "node:test";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { circuitPaths, scenePose } from "../scene-math.mjs";
import { createEmblemGeometry } from "../emblem-geometry.js";

// ------------------------=
// FUNC: authoredEmblem
// DESC: Verifies source provenance, original openings, real depth and exact source-to-face UV mapping.
// ------------------=
function authoredEmblem() {
  const outline = JSON.parse(readFileSync(new URL("../assets/emblem-outline.json", import.meta.url)));
  const source = readFileSync(new URL("../design/emblem-reference.png", import.meta.url));
  assert.equal(createHash("sha256").update(source).digest("hex"), outline.sourceSha256);
  assert.equal(outline.contours.length, 3);
  assert.ok(outline.areas[0] > 500000);
  assert.ok(outline.areas[1] < -60000 && outline.areas[2] < -60000);
  const filledArea = outline.areas.reduce((sum, value) => sum + value, 0);
  assert.ok(Math.abs(filledArea - outline.opaquePixels) / outline.opaquePixels < .005);
  const geometry = createEmblemGeometry(outline);
  const p = geometry.attributes.position, uv = geometry.attributes.uv;
  assert.ok(p.array.every(Number.isFinite));
  assert.ok(geometry.boundingBox.max.z - geometry.boundingBox.min.z > .6);
  assert.ok(geometry.boundingBox.max.x - geometry.boundingBox.min.x > 6);
  for (let i = 0; i < uv.count; i++) {
    const expectedX = (p.getX(i) / outline.unitsPerPixel + outline.crossing[0]) / outline.sourceWidth;
    const expectedY = 1 - (outline.crossing[1] - p.getY(i) / outline.unitsPerPixel) / outline.sourceHeight;
    assert.ok(Math.abs(uv.getX(i) - expectedX) < .00001);
    assert.ok(Math.abs(uv.getY(i) - expectedY) < .00001);
  }
  assert.equal(geometry.groups.length, 2);
  geometry.dispose();
}
test("3D emblem derives its silhouette and face UVs from the actual kit artwork", authoredEmblem);

// ------------------------=
// FUNC: motionStates
// DESC: Proves idle and scroll independently change pose while reduced motion is invariant.
// ------------------=
function motionStates() {
  assert.notDeepEqual(scenePose(0, 0, false), scenePose(5, 0, false));
  assert.ok(scenePose(0, .5, false).y - scenePose(0, 0, false).y > 2);
  assert.deepEqual(scenePose(0, 0, true), scenePose(50, 1, true));
  assert.deepEqual(scenePose(0, -1, false), scenePose(0, 0, false));
  assert.deepEqual(scenePose(0, 2, false), scenePose(0, 1, false));
  assert.equal(scenePose(9, 0, false).pulse, 9);
}
test("idle and scroll animate independently; reduction is stable", motionStates);

// ------------------------=
// FUNC: traceGeometry
// DESC: Verifies deterministic ground-plane routes with nonzero connected segments.
// ------------------=
function traceGeometry() {
  const paths = circuitPaths();
  assert.equal(paths.length, 28);
  assert.deepEqual(paths, circuitPaths());
  for (const path of paths) {
    assert.equal(path.length, 5);
    for (const point of path) {
      assert.ok(point.every(Number.isFinite));
      assert.equal(point[1], -1.95);
    }
    for (let i = 1; i < path.length; i++) assert.ok(Math.hypot(...path[i].map((v, axis) => v - path[i - 1][axis])) > 0);
  }
}
test("electrical paths are deterministic and continuous", traceGeometry);
