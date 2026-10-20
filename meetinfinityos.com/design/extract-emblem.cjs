const fs = require("node:fs");
const path = require("node:path");
const sharp = require("sharp");
const { createHash } = require("node:crypto");

// ------------------------=
// FUNC: area
// DESC: Returns signed contour area in the source image coordinate system.
// ------------------=
function area(points) {
  let sum = 0;
  for (let i = 0; i < points.length; i++) {
    const a = points[i], b = points[(i + 1) % points.length];
    sum += a[0] * b[1] - b[0] * a[1];
  }
  return sum / 2;
}

// ------------------------=
// FUNC: simplify
// DESC: Simplifies an extracted pixel contour to within one source pixel of its original boundary.
// ------------------=
function simplify(points, tolerance = 1) {
  const a = points[0], b = points[points.length - 1], dx = b[0] - a[0], dy = b[1] - a[1];
  let farthest = 0, maximum = 0;
  for (let i = 1; i < points.length - 1; i++) {
    const p = points[i], t = Math.max(0, Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy || 1)));
    const distance = Math.hypot(p[0] - a[0] - dx * t, p[1] - a[1] - dy * t);
    if (distance > maximum) { maximum = distance; farthest = i; }
  }
  if (maximum <= tolerance) return [a, b];
  return [...simplify(points.slice(0, farthest + 1), tolerance).slice(0, -1), ...simplify(points.slice(farthest), tolerance)];
}

// ------------------------=
// FUNC: smoothContour
// DESC: Removes subpixel alpha noise while retaining the authored broad silhouette.
// ------------------=
function smoothContour(points) {
  let result = points;
  for (let pass = 0; pass < 2; pass++) {
    const next = [];
    for (let i = 0; i < result.length; i++) {
      const a = result[i], b = result[(i + 1) % result.length];
      next.push([a[0] * .75 + b[0] * .25, a[1] * .75 + b[1] * .25],
        [a[0] * .25 + b[0] * .75, a[1] * .25 + b[1] * .75]);
    }
    result = next;
  }
  return result;
}

// ------------------------=
// FUNC: extract
// DESC: Separates the authored emblem from its baked floor reflection and exports reusable mesh contours.
// ------------------=
async function extract() {
  const source = path.join(__dirname, "emblem-reference.png");
  const { data, info } = await sharp(source).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  const width = info.width, height = 685, binary = new Uint8Array(width * height);
  for (let i = 0; i < binary.length; i++) binary[i] = data[i * 4 + 3] > 190 ? 1 : 0;
  // Close tiny raster-alpha gaps without changing the two large authored openings.
  const dilated = new Uint8Array(binary.length), closed = new Uint8Array(binary.length);
  for (let y = 1; y < height - 1; y++) for (let x = 1; x < width - 1; x++) {
    for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
      if (binary[(y + dy) * width + x + dx]) dilated[y * width + x] = 1;
    }
  }
  for (let y = 1; y < height - 1; y++) for (let x = 1; x < width - 1; x++) {
    let occupied = 1;
    for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) occupied &= dilated[(y + dy) * width + x + dx];
    closed[y * width + x] = occupied;
  }
  const seen = new Uint8Array(binary.length);
  let largest = [];
  for (let seed = 0; seed < closed.length; seed++) {
    if (!closed[seed] || seen[seed]) continue;
    const component = [seed]; seen[seed] = 1;
    for (let head = 0; head < component.length; head++) {
      const point = component[head];
      for (const neighbor of [point - 1, point + 1, point - width, point + width]) {
        if (neighbor >= 0 && neighbor < closed.length && closed[neighbor] && !seen[neighbor]) {
          seen[neighbor] = 1; component.push(neighbor);
        }
      }
    }
    if (component.length > largest.length) largest = component;
  }
  const mask = new Uint8Array(binary.length);
  for (const index of largest) mask[index] = 1;
  const edges = new Map();
  // ------------------------=
  // FUNC: edge
  // DESC: Indexes an oriented exposed pixel edge for closed-loop tracing.
  // ------------------=
  function edge(ax, ay, bx, by) {
    const key = `${ax},${ay}`;
    if (!edges.has(key)) edges.set(key, []);
    edges.get(key).push([bx, by]);
  }
  for (const index of largest) {
    const x = index % width, y = Math.floor(index / width);
    if (!mask[index - width]) edge(x, y, x + 1, y);
    if (!mask[index + 1]) edge(x + 1, y, x + 1, y + 1);
    if (!mask[index + width]) edge(x + 1, y + 1, x, y + 1);
    if (!mask[index - 1]) edge(x, y + 1, x, y);
  }
  const contours = [];
  while (edges.size) {
    const start = edges.keys().next().value, points = [start.split(",").map(Number)];
    let key = start;
    do {
      const next = edges.get(key)?.pop();
      if (!next) throw new Error("Open emblem contour");
      if (!edges.get(key).length) edges.delete(key);
      points.push(next);
      key = next.join(",");
    } while (key !== start);
    if (Math.abs(area(points)) > 1000) contours.push(smoothContour(simplify(points, 4).slice(0, -1)));
  }
  contours.sort((a, b) => Math.abs(area(b)) - Math.abs(area(a)));
  if (contours.length !== 3) throw new Error(`Expected one outer contour and two openings, got ${contours.length}`);
  const result = { sourceSha256: createHash("sha256").update(fs.readFileSync(source)).digest("hex"),
    sourceWidth: width, sourceHeight: info.height, reflectionCutoff: height,
    crossing: [837, 405], unitsPerPixel: 1 / 220, opaquePixels: largest.length,
    contours, areas: contours.map(area) };
  fs.writeFileSync(path.join(__dirname, "../assets/emblem-outline.json"), JSON.stringify(result));
  console.log(JSON.stringify({ contours: contours.length, points: contours.map(p => p.length), areas: result.areas }));
}
extract().catch(error => { console.error(error); process.exitCode = 1; });
