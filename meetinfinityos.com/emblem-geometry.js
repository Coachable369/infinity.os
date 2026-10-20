import * as THREE from "./assets/vendor/three.module.min.js";
import { mergeVertices } from "./assets/vendor/BufferGeometryUtils.js";

// ------------------------=
// FUNC: sourceUV
// DESC: Projects original emblem pixels onto both faces of the traced three-dimensional geometry.
// ------------------=
function sourceUV(vertices, index, outline) {
  return new THREE.Vector2(
    (vertices[index * 3] / outline.unitsPerPixel + outline.crossing[0]) / outline.sourceWidth,
    1 - (outline.crossing[1] - vertices[index * 3 + 1] / outline.unitsPerPixel) / outline.sourceHeight
  );
}

// ------------------------=
// FUNC: createEmblemGeometry
// DESC: Extrudes the actual kit silhouette and two openings, with rounded beveled edges.
// ------------------=
export function createEmblemGeometry(outline) {
  const contours = outline.contours.map(points => points.map(point => new THREE.Vector2(
    (point[0] - outline.crossing[0]) * outline.unitsPerPixel,
    (outline.crossing[1] - point[1]) * outline.unitsPerPixel
  )));
  const shape = new THREE.Shape(contours[0]);
  shape.holes = contours.slice(1).map(points => new THREE.Path(points));
  const geometry = new THREE.ExtrudeGeometry(shape, {
    depth: .42, steps: 1, bevelEnabled: true, bevelSegments: 6,
    bevelThickness: .12, bevelSize: .035, bevelOffset: -.035,
    UVGenerator: {
      // ------------------------=
      // FUNC: generateTopUV
      // DESC: Preserves the authored face texture without stretching it around a generic loop.
      // ------------------=
      generateTopUV(geometry, vertices, a, b, c) {
        return [sourceUV(vertices, a, outline), sourceUV(vertices, b, outline), sourceUV(vertices, c, outline)];
      },
      // ------------------------=
      // FUNC: generateSideWallUV
      // DESC: Supplies stable coordinates for the independently shaded chrome edge material.
      // ------------------=
      generateSideWallUV(geometry, vertices, a, b, c, d) {
        return [sourceUV(vertices, a, outline), sourceUV(vertices, b, outline),
          sourceUV(vertices, c, outline), sourceUV(vertices, d, outline)];
      }
    }
  });
  geometry.translate(0, 0, -.21);
  geometry.deleteAttribute("normal");
  const smooth = mergeVertices(geometry, .0001);
  smooth.computeVertexNormals();
  smooth.computeBoundingBox();
  geometry.dispose();
  return smooth;
}
