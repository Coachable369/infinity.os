import * as THREE from "./assets/vendor/three.module.min.js";
import { circuitPaths, scenePose } from "./scene-math.mjs?v=woven1";
import { createEmblemGeometry } from "./emblem-geometry.js";

// ------------------------=
// FUNC: studioEnvironment
// DESC: Bakes a small studio reflection map once, with elongated softbox highlights.
// ------------------=
function studioEnvironment(renderer) {
  const studio = new THREE.Scene();
  studio.background = new THREE.Color("#060b19");
  const geometry = new THREE.PlaneGeometry(1, 1);
  const panels = [
    { p: [-4, 5, 3], s: [5, 8], c: [7, 8, 10] },
    { p: [5, 3, 1], s: [2, 10], c: [3, 5, 10] },
    { p: [0, 6, -4], s: [7, 2], c: [8, 8, 8] },
    { p: [-3, -2, 5], s: [6, 1], c: [2, 4, 8] },
    { p: [2, 0, -5], s: [1, 7], c: [5, 7, 10] }
  ];
  for (const panel of panels) {
    const material = new THREE.MeshBasicMaterial({ color: new THREE.Color().setRGB(...panel.c), side: THREE.DoubleSide });
    const mesh = new THREE.Mesh(geometry, material);
    mesh.position.set(...panel.p);
    mesh.scale.set(...panel.s, 1);
    mesh.lookAt(0, 0, 0);
    studio.add(mesh);
  }
  const generator = new THREE.PMREMGenerator(renderer);
  const target = generator.fromScene(studio, .035, .1, 30);
  generator.dispose();
  geometry.dispose();
  for (const child of studio.children) child.material.dispose();
  return target;
}

// ------------------------=
// FUNC: traceGeometry
// DESC: Batches continuous circuit lanes into one strip mesh with distance-aware pulse attributes.
// ------------------=
function traceGeometry(width) {
  const positions = [], edges = [], along = [], seeds = [], indices = [];
  const paths = circuitPaths();
  for (const [pathIndex, path] of paths.entries()) {
    let total = 0;
    const lengths = [];
    for (let i = 1; i < path.length; i++) {
      const length = Math.hypot(...path[i].map((v, axis) => v - path[i - 1][axis]));
      lengths.push(length);
      total += length;
    }
    let distance = 0;
    const base = positions.length / 3;
    for (let i = 0; i < path.length; i++) {
      const a = path[Math.max(0, i - 1)], b = path[i], c = path[Math.min(path.length - 1, i + 1)];
      const incoming = i ? new THREE.Vector2(a[1] - b[1], b[0] - a[0]).normalize() : null;
      const outgoing = i + 1 < path.length ? new THREE.Vector2(b[1] - c[1], c[0] - b[0]).normalize() : null;
      const normal = (incoming || outgoing).clone().add(outgoing || incoming).normalize();
      normal.multiplyScalar(width / normal.dot(outgoing || incoming));
      positions.push(b[0] + normal.x, b[1] + normal.y, b[2], b[0] - normal.x, b[1] - normal.y, b[2]);
      edges.push(1, -1);
      along.push(distance / total, distance / total);
      seeds.push((pathIndex % 14) / 14, (pathIndex % 14) / 14);
      if (i < lengths.length) {
        const k = base + i * 2;
        indices.push(k, k + 2, k + 1, k + 1, k + 2, k + 3);
        distance += lengths[i];
      }
    }
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
  geometry.setAttribute("aEdge", new THREE.Float32BufferAttribute(edges, 1));
  geometry.setAttribute("aAlong", new THREE.Float32BufferAttribute(along, 1));
  geometry.setAttribute("aSeed", new THREE.Float32BufferAttribute(seeds, 1));
  geometry.setIndex(indices);
  return geometry;
}

// ------------------------=
// FUNC: traceMaterial
// DESC: Lights each electrical route progressively and sends a bright head with a fading trail.
// ------------------=
function traceMaterial(uniforms, glow) {
  return new THREE.ShaderMaterial({
    uniforms: { ...uniforms, uGlow: { value: glow ? 1 : 0 } },
    transparent: true, depthWrite: false, side: THREE.DoubleSide, blending: THREE.AdditiveBlending,
    vertexShader: `
      attribute float aEdge; attribute float aAlong; attribute float aSeed;
      varying float vEdge; varying float vAlong; varying float vSeed;
      // ------------------------=
      // FUNC: main
      // DESC: Carries route distance and cross-strip falloff into the pulse shader.
      // ------------------=
      void main() {
        vEdge = aEdge; vAlong = aAlong; vSeed = aSeed;
        gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
      }`,
    fragmentShader: `
      uniform float uTime; uniform float uProgress; uniform float uGlow;
      varying float vEdge; varying float vAlong; varying float vSeed;
      // ------------------------=
      // FUNC: main
      // DESC: Combines chapter activation with a moving cyan pulse on connected electrical traces.
      // ------------------=
      void main() {
        float activation = smoothstep(vSeed * .68, vSeed * .68 + .22, uProgress + .60);
        float travel = fract(vAlong - uTime * .17 + vSeed * 1.7);
        float pulse = exp(-min(1.0 - travel, travel * 4.0) * 17.0) * activation;
        float edge = pow(max(0.0, 1.0 - abs(vEdge)), uGlow > .5 ? 2.0 : .3);
        float brightness = uGlow > .5 ? (.10 + pulse * .80) : .40 + activation * .32 + pulse * 1.5;
        vec3 color = mix(vec3(.08, .35, .72), vec3(.67, .95, 1.0), pulse);
        gl_FragColor = vec4(color, brightness * edge);
      }`
  });
}

// ------------------------=
// FUNC: create
// DESC: Creates the responsive true-3D emblem and its shared scroll-activated electrical environment.
// ------------------=
export async function create(canvas, image) {
  let renderer;
  try {
    const response = await fetch(new URL("./assets/emblem-outline.json?v=woven1", import.meta.url));
    if (!response.ok) return null;
    const outline = await response.json();
    renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true, powerPreference: "low-power" });
    renderer.setClearColor("#050608", 0);
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.1;
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(36, 1, .1, 80);
    const environment = studioEnvironment(renderer);
    scene.environment = environment.texture;
    scene.add(new THREE.HemisphereLight("#bcdcff", "#020611", 1.1));
    const key = new THREE.DirectionalLight("#e8f5ff", 3);
    key.position.set(-3, 5, 7);
    scene.add(key);
    const rim = new THREE.DirectionalLight("#2765ff", 4);
    rim.position.set(4, 1, -4);
    scene.add(rim);

    const geometry = createEmblemGeometry(outline);
    const texture = new THREE.Texture(image);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.anisotropy = Math.min(8, renderer.capabilities.getMaxAnisotropy());
    texture.needsUpdate = true;
    const material = new THREE.MeshBasicMaterial({
      map: texture, color: "#ffffff", toneMapped: false,
      transparent: true
    });
    // ------------------------=
    // FUNC: preserveFaceOpacity
    // DESC: Uses traced geometry for the edge, avoiding noisy raster alpha cuts through solid faces.
    // ------------------=
    function preserveFaceOpacity(shader) {
      shader.fragmentShader = shader.fragmentShader.replace("#include <map_fragment>",
        "#include <map_fragment>\ndiffuseColor.a = opacity;");
    }
    material.onBeforeCompile = preserveFaceOpacity;
    const edgeMaterial = new THREE.MeshPhysicalMaterial({
      color: "#2963bf", metalness: .68, roughness: .18, clearcoat: 1,
      clearcoatRoughness: .12, envMapIntensity: 1.7, transparent: true
    });
    const emblem = new THREE.Mesh(geometry, [material, edgeMaterial]);
    scene.add(emblem);
    const reflectionMaterial = material.clone();
    reflectionMaterial.onBeforeCompile = preserveFaceOpacity;
    reflectionMaterial.opacity = .11;
    reflectionMaterial.depthWrite = false;
    const reflectionEdgeMaterial = edgeMaterial.clone();
    reflectionEdgeMaterial.opacity = .11; reflectionEdgeMaterial.depthWrite = false;
    const reflection = new THREE.Mesh(geometry, [reflectionMaterial, reflectionEdgeMaterial]);
    scene.add(reflection);

    const uniforms = { uTime: { value: 0 }, uProgress: { value: 0 } };
    const circuits = new THREE.Group();
    circuits.add(new THREE.Mesh(traceGeometry(.018), traceMaterial(uniforms, false)));
    circuits.add(new THREE.Mesh(traceGeometry(.082), traceMaterial(uniforms, true)));
    scene.add(circuits);
    const padGeometry = new THREE.RingGeometry(.036, .063, 16);
    const padMaterial = new THREE.MeshBasicMaterial({ color: "#3c8bb9", side: THREE.DoubleSide, transparent: true, opacity: .85 });
    for (const path of circuitPaths()) {
      const pad = new THREE.Mesh(padGeometry, padMaterial);
      pad.position.set(...path[path.length - 1]);
      circuits.add(pad);
    }
    let lastWidth = 0, lastHeight = 0, lost = false;
    // ------------------------=
    // FUNC: handleContextLoss
    // DESC: Lets the controller reveal its static fallback without scheduling broken render calls.
    // ------------------=
    function handleContextLoss() { lost = true; }
    canvas.addEventListener("webglcontextlost", handleContextLoss);
    canvas.dataset.renderer = "three-webgl";
    canvas.dataset.vertices = String(geometry.attributes.position.count);
    canvas.dataset.triangles = String(geometry.index.count / 3);
    canvas.dataset.contours = String(outline.contours.length);
    canvas.dataset.source = outline.sourceSha256;
    canvas.dataset.paths = String(circuitPaths().length);

    // ------------------------=
    // FUNC: draw
    // DESC: Renders one bounded frame, with actual 3D axis changes and chapter-based scene placement.
    // ------------------=
    function draw(state) {
      if (lost) return false;
      const { width, height, time, paused, progress, pointer } = state;
      const mobile = width < 760;
      if (width !== lastWidth || height !== lastHeight) {
        const ratio = Math.min(window.devicePixelRatio || 1, 1.5, Math.sqrt(1600000 / (width * height)));
        renderer.setPixelRatio(ratio);
        renderer.setSize(width, height, false);
        camera.aspect = width / height;
        camera.updateProjectionMatrix();
        lastWidth = width; lastHeight = height;
      }
      camera.position.set(0, mobile ? .6 : .7, mobile ? 15.5 : 11.8);
      camera.lookAt(0, 0, 0);
      const pose = scenePose(time, progress, paused);
      const travel = THREE.MathUtils.smoothstep(progress, .06, mobile ? .29 : .27);
      const exit = THREE.MathUtils.smoothstep(progress, .57, .81);
      const worldWidth = Math.tan(THREE.MathUtils.degToRad(18)) * camera.position.z * 2 * camera.aspect;
      const scale = (mobile ? Math.min(.64, worldWidth / 6.8) : Math.min(1.05, worldWidth / 12.6)) * (1 - travel * .16);
      const left = mobile ? 0 : worldWidth * -.22;
      const right = mobile ? 0 : worldWidth * .22;
      emblem.position.set(THREE.MathUtils.lerp(right, left, travel), mobile ? THREE.MathUtils.lerp(-.9, 2.25, travel) : .15, 0);
      emblem.rotation.set(pose.x + (paused ? 0 : pointer.y * .05), pose.y, pose.z);
      emblem.scale.setScalar(scale * (1 - exit * .15));
      material.opacity = 1 - exit;
      edgeMaterial.opacity = 1 - exit;
      emblem.visible = exit < .999;
      reflection.position.set(emblem.position.x, -3.9 - emblem.position.y, 0);
      reflection.rotation.copy(emblem.rotation);
      reflection.scale.set(scale, -scale, scale);
      reflectionMaterial.opacity = (1 - exit) * .11;
      reflectionEdgeMaterial.opacity = reflectionMaterial.opacity;
      reflection.visible = emblem.visible && !mobile;
      circuits.position.copy(emblem.position);
      circuits.scale.setScalar(scale);
      circuits.rotation.y = paused ? -.1 : -.1 + progress * .25;
      uniforms.uTime.value = pose.pulse;
      uniforms.uProgress.value = progress;
      renderer.render(scene, camera);
      canvas.dataset.rotation = `${pose.x.toFixed(4)},${pose.y.toFixed(4)},${pose.z.toFixed(4)}`;
      canvas.dataset.pulse = pose.pulse.toFixed(3);
      canvas.dataset.progress = progress.toFixed(4);
      return true;
    }

    // ------------------------=
    // FUNC: dispose
    // DESC: Releases GPU resources before a replacement context or explicit teardown.
    // ------------------=
    function dispose() {
      canvas.removeEventListener("webglcontextlost", handleContextLoss);
      geometry.dispose(); material.dispose(); edgeMaterial.dispose(); texture.dispose();
      reflectionMaterial.dispose(); reflectionEdgeMaterial.dispose();
      for (const child of circuits.children.slice(0, 2)) { child.geometry.dispose(); child.material.dispose(); }
      padGeometry.dispose(); padMaterial.dispose(); environment.dispose(); renderer.dispose();
    }
    return { draw, dispose };
  } catch {
    renderer?.dispose();
    canvas.dataset.renderer = "static";
    return null;
  }
}
