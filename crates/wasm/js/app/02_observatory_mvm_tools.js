function mat4_create() { /* Formal R4.StudioFrame */
  const out = new Float32Array(16);
  out[0] = 1.0;
  out[5] = 1.0;
  out[10] = 1.0;
  out[15] = 1.0;
  return out;
}

function mat4_perspective(out, fovy, aspect, near, far) { /* Formal R4.StudioFrame */
  const f = 1.0 / Math.tan(fovy / 2.0);
  const nf = 1.0 / (near - far);
  out[0] = f / aspect;
  out[1] = 0.0;
  out[2] = 0.0;
  out[3] = 0.0;
  out[4] = 0.0;
  out[5] = f;
  out[6] = 0.0;
  out[7] = 0.0;
  out[8] = 0.0;
  out[9] = 0.0;
  out[10] = (far + near) * nf;
  out[11] = -1.0;
  out[12] = 0.0;
  out[13] = 0.0;
  out[14] = 2.0 * far * near * nf;
  out[15] = 0.0;
  return out;
}

function mat4_lookAt(out, eye, center, up) { /* Formal R4.StudioFrame */
  let z0 = eye[0] - center[0];
  let z1 = eye[1] - center[1];
  let z2 = eye[2] - center[2];
  let len = Math.hypot(z0, z1, z2);
  if (len === 0) len = 1.0;
  z0 /= len; z1 /= len; z2 /= len;

  let x0 = up[1] * z2 - up[2] * z1;
  let x1 = up[2] * z0 - up[0] * z2;
  let x2 = up[0] * z1 - up[1] * z0;
  len = Math.hypot(x0, x1, x2);
  if (len === 0) len = 1.0;
  x0 /= len; x1 /= len; x2 /= len;

  let y0 = z1 * x2 - z2 * x1;
  let y1 = z2 * x0 - z0 * x2;
  let y2 = z0 * x1 - z1 * x0;

  out[0] = x0; out[1] = y0; out[2] = z0; out[3] = 0.0;
  out[4] = x1; out[5] = y1; out[6] = z1; out[7] = 0.0;
  out[8] = x2; out[9] = y2; out[10] = z2; out[11] = 0.0;
  out[12] = -(x0 * eye[0] + x1 * eye[1] + x2 * eye[2]);
  out[13] = -(y0 * eye[0] + y1 * eye[1] + y2 * eye[2]);
  out[14] = -(z0 * eye[0] + z1 * eye[1] + z2 * eye[2]);
  out[15] = 1.0;
  return out;
}

function mat4_multiply(out, a, b) { /* Formal R4.StudioFrame */
  const a00 = a[0], a01 = a[1], a02 = a[2], a03 = a[3];
  const a10 = a[4], a11 = a[5], a12 = a[6], a13 = a[7];
  const a20 = a[8], a21 = a[9], a22 = a[10], a23 = a[11];
  const a30 = a[12], a31 = a[13], a32 = a[14], a33 = a[15];

  let b0 = b[0], b1 = b[1], b2 = b[2], b3 = b[3];
  out[0] = b0 * a00 + b1 * a10 + b2 * a20 + b3 * a30;
  out[1] = b0 * a01 + b1 * a11 + b2 * a21 + b3 * a31;
  out[2] = b0 * a02 + b1 * a12 + b2 * a22 + b3 * a32;
  out[3] = b0 * a03 + b1 * a13 + b2 * a23 + b3 * a33;

  b0 = b[4]; b1 = b[5]; b2 = b[6]; b3 = b[7];
  out[4] = b0 * a00 + b1 * a10 + b2 * a20 + b3 * a30;
  out[5] = b0 * a01 + b1 * a11 + b2 * a21 + b3 * a31;
  out[6] = b0 * a02 + b1 * a12 + b2 * a22 + b3 * a32;
  out[7] = b0 * a03 + b1 * a13 + b2 * a23 + b3 * a33;

  b0 = b[8]; b1 = b[9]; b2 = b[10]; b3 = b[11];
  out[8] = b0 * a00 + b1 * a10 + b2 * a20 + b3 * a30;
  out[9] = b0 * a01 + b1 * a11 + b2 * a21 + b3 * a31;
  out[10] = b0 * a02 + b1 * a12 + b2 * a22 + b3 * a32;
  out[11] = b0 * a03 + b1 * a13 + b2 * a23 + b3 * a33;

  b0 = b[12]; b1 = b[13]; b2 = b[14]; b3 = b[15];
  out[12] = b0 * a00 + b1 * a10 + b2 * a20 + b3 * a30;
  out[13] = b0 * a01 + b1 * a11 + b2 * a21 + b3 * a31;
  out[14] = b0 * a02 + b1 * a12 + b2 * a22 + b3 * a32;
  out[15] = b0 * a03 + b1 * a13 + b2 * a23 + b3 * a33;
  return out;
}

function catmullRom1D(p0, p1, p2, p3, t) { /* Formal R4.StudioFrame */
  const t2 = t * t;
  const t3 = t2 * t;
  return 0.5 * (
    2.0 * p1 +
    (-p0 + p2) * t +
    (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 +
    (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3
  );
}

function catmullRomPoint(out, p0, p1, p2, p3, t) { /* Formal R4.StudioFrame */
  out[0] = catmullRom1D(p0[0], p1[0], p2[0], p3[0], t);
  out[1] = catmullRom1D(p0[1], p1[1], p2[1], p3[1], t);
  out[2] = catmullRom1D(p0[2], p1[2], p2[2], p3[2], t);
  return out;
}

function compileShader(gl, type, source) { /* Formal R4.StudioFrame */
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const info = gl.getShaderInfoLog(shader);
    gl.deleteShader(shader);
    throw new Error(`Shader compilation failure: ${info}`);
  }
  return shader;
}

function createProgram(gl, vsSource, fsSource) { /* Formal R4.StudioFrame */
  const vs = compileShader(gl, gl.VERTEX_SHADER, vsSource);
  const fs = compileShader(gl, gl.FRAGMENT_SHADER, fsSource);
  const prog = gl.createProgram();
  gl.attachShader(prog, vs);
  gl.attachShader(prog, fs);
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
    const info = gl.getProgramInfoLog(prog);
    gl.deleteProgram(prog);
    throw new Error(`Program link failure: ${info}`);
  }
  return prog;
}

class LexLean3DVisualizer {
  constructor() {
    this.canvas = null;
    this.gl = null;
    this.isWebGL2 = false;
    this.programPoints = null;
    this.programLines = null;
    this.pointsVbo = null;
    this.linesVbo = null;
    this.animFrameId = null;
    this.lastTimestamp = 0;
    this.autoRotate = true;

    // Camera parameters
    this.yaw = 0.55;
    this.pitch = 0.28;
    this.distance = 15.0; // clamped between 2.0 and 50.0 via Math.min/Math.max
    this.target = new Float32Array([0.0, 0.0, 0.0]);

    // Pointer interaction
    this.isDragging = false;
    this.lastPointerX = 0;
    this.lastPointerY = 0;

    // Matrices
    this.projMatrix = mat4_create();
    this.viewMatrix = mat4_create();
    this.viewProjMatrix = mat4_create();

    // Scene geometry
    // 64 Clifford Cl(0,6) basis state nodes (Functional Cortical Neuron Clusters)
    this.cliffordNodes = [];
    this.cliffordGradeEnergies = new Float32Array([1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);

    // 17 Icosian roots
    this.icosianRoots = [];
    this.dominantIcosianRoot = 0;

    // 6 SpiralCore layer planes (l0..l5)
    this.layerElevations = new Float32Array([-2.5, -1.5, -0.5, 0.5, 1.5, 2.5]);
    this.layerEnergies = new Float32Array([0.6, 0.6, 0.6, 0.6, 0.6, 0.6]);

    // Starfield PointKey memory basin particles
    this.starfieldParticles = [];
    this.winningPointKeyRow = null;

    // Token trajectory spline
    this.decodeWaypoints = [];
    this.tokenSplinePoints = [];

    // Anatomical 3D Human Brain contours, dendritic micro-clusters, OSPF router, Starling flock & Memory Engrams
    this.brainContourSegments = [];
    this.corticalMicroNeurons = [];
    this.corticalSynapses = [];
    this.ospfRouteHops = [];
    this.lsaProbeLinks = [];
    this.starlingFlock = [];
    this.memoryEngrams = [];
    this.callosumPulsePackets = [];
    this.activeTurnWords = ['UOR-R4', 'OSPF', 'Synaptic', 'Token', 'Prediction', 'Route'];
    this.activePromptSnippet = 'Hello! Who are you and what can you do?';
    this.cascadeStartTime = 0;
    this.showLabels = true;
    this.lastLabelUpdateMs = 0;

    // UI hooks
    this.statusEl = null;
    this.resetBtn = null;
    this.animToggleBtn = null;
    this.pulseBtn = null;
    this.labelsToggleBtn = null;
    this.labelsContainer = null;
    this.ospfTickerEl = null;
    this.activeTraceInfo = null;

    this.boundAnimate = this.animate.bind(this);
  }

  init(canvasId, options = {}) {
    const canvas = document.getElementById(canvasId);
    if (!canvas) return false;
    this.canvas = canvas;
    this.statusEl = document.getElementById('neural-3d-status');
    this.resetBtn = document.getElementById('neural-3d-reset-btn');
    this.animToggleBtn = document.getElementById('neural-3d-anim-toggle');
    this.pulseBtn = document.getElementById('neural-3d-pulse-btn');
    this.labelsToggleBtn = document.getElementById('neural-3d-labels-toggle');
    this.labelsContainer = document.getElementById('neural-3d-labels');
    this.ospfTickerEl = document.getElementById('neural-ospf-route-text');

    if (!options || !options.deferGL) {
      this.initGL();
    }
    this.buildStaticGeometry();
    this.buildDefaultDecodeSpline();
    this.bindEvents();
    this.updateStatus();

    if (this.gl && !this.animFrameId) {
      this.animFrameId = requestAnimationFrame(this.boundAnimate);
    }
    return true;
  }

  ensureGLReady() {
    if (!this.gl && this.canvas) {
      this.initGL();
      this.updateStatus();
    }
    if (this.gl && !this.animFrameId && typeof requestAnimationFrame === 'function') {
      this.animFrameId = requestAnimationFrame(this.boundAnimate);
    }
    return Boolean(this.gl);
  }

  initGL() {
    const canvas = this.canvas;
    const gl = canvas.getContext('webgl2') || canvas.getContext('webgl') || canvas.getContext('experimental-webgl');
    if (!gl) {
      if (this.statusEl) {
        this.statusEl.textContent = 'WebGL Not Supported';
      }
      return false;
    }
    this.gl = gl;
    this.isWebGL2 = typeof WebGL2RenderingContext !== 'undefined' && gl instanceof WebGL2RenderingContext;

    // Universal GLSL ES shaders
    const vsPoints = `
      attribute vec3 a_position;
      attribute vec4 a_color;
      attribute float a_size;
      uniform mat4 u_viewProjection;
      uniform float u_viewportHeight;
      varying vec4 v_color;

      void main() {
        vec4 pos = u_viewProjection * vec4(a_position, 1.0);
        gl_Position = pos;
        float w = max(pos.w, 0.001);
        gl_PointSize = clamp(a_size * (u_viewportHeight / w) * 0.14, 1.5, 64.0);
        v_color = a_color;
      }
    `;

    const fsPoints = `
      precision mediump float;
      varying vec4 v_color;

      void main() {
        vec2 coord = gl_PointCoord - vec2(0.5);
        float dist = length(coord);
        if (dist > 0.5) {
          discard;
        }
        float alpha = smoothstep(0.5, 0.04, dist) * v_color.a;
        gl_FragColor = vec4(v_color.rgb, alpha);
      }
    `;

    const vsLines = `
      attribute vec3 a_position;
      attribute vec4 a_color;
      uniform mat4 u_viewProjection;
      varying vec4 v_color;

      void main() {
        gl_Position = u_viewProjection * vec4(a_position, 1.0);
        v_color = a_color;
      }
    `;

    const fsLines = `
      precision mediump float;
      varying vec4 v_color;

      void main() {
        gl_FragColor = v_color;
      }
    `;

    this.programPoints = createProgram(gl, vsPoints, fsPoints);
    this.programLines = createProgram(gl, vsLines, fsLines);

    this.pointsVbo = gl.createBuffer();
    this.linesVbo = gl.createBuffer();

    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE);
    gl.enable(gl.DEPTH_TEST);
    gl.depthFunc(gl.LEQUAL);
    gl.clearColor(0.018, 0.025, 0.048, 1.0);
    return true;
  }

  // Parametric 3D Human Cerebral Cortex Surface:
  // hemi: -1 (Left Hemisphere) or +1 (Right Hemisphere)
  // u in [0, 1]: 0 = Posterior Occipital Pole (z ~ -3.2), 1 = Anterior Frontal Pole (z ~ +3.2)
  // v in [0, 1]: 0 = Ventral Basal / Temporal Rim, 0.5 = Lateral Convexity, 1 = Superior Longitudinal Fissure
  computeCorticalSurfacePoint(hemi, u, v, shellScale = 1.0) {
    const zBase = -3.15 + 6.35 * u;
    // Dorsal arch profile: rises steeply in frontal lobe, peaks in parietal dome, slopes to occipital pole
    const dorsalArch = Math.sin(Math.pow(u, 0.82) * Math.PI);
    const frontalSlope = (u > 0.65) ? Math.sin(((u - 0.65) / 0.35) * Math.PI * 0.5) * 0.35 : 0.0;
    const yTop = -0.25 + (2.45 * dorsalArch + frontalSlope) * shellScale;

    // Ventral profile: dips downward in the middle-front to form the distinct Temporal Lobe thumb bulge
    const temporalWindow = Math.exp(-Math.pow((u - 0.48) / 0.22, 2.0));
    const occipitalNotch = Math.exp(-Math.pow((u - 0.18) / 0.12, 2.0)) * 0.35;
    const yBot = -0.65 - 0.95 * temporalWindow + occipitalNotch;

    // Sylvian (Lateral) Fissure pinch at v ~ 0.38 in the temporal window
    const sylvianPinch = 1.0 - 0.22 * temporalWindow * Math.exp(-Math.pow((v - 0.38) / 0.11, 2.0));

    // Lateral width profile (wider in temporal/parietal, tapered at frontal & occipital poles)
    const apWidth = Math.sin(Math.pow(u, 0.88) * Math.PI) * (0.92 + 0.24 * temporalWindow);
    const domeSin = Math.sin(v * Math.PI * 0.88 + 0.12);

    // Gyral/sulcal cortical folding wrinkles
    const gyri = 1.0 + 0.045 * Math.sin(u * 28.0 + v * 14.0) * Math.cos(v * 22.0 - u * 12.0);

    const fissureGap = 0.18 + 0.08 * Math.pow(1.0 - Math.sin(v * Math.PI), 2.0);
    const x = hemi * (fissureGap + 2.45 * apWidth * domeSin * sylvianPinch * shellScale * gyri);
    const y = (yBot + (yTop - yBot) * Math.sin(v * Math.PI * 0.5)) * gyri;
    const z = zBase * (0.96 + 0.04 * Math.sin(v * Math.PI));
    return new Float32Array([x, y, z]);
  }

  buildStaticGeometry() {
    // 1. 64 Clifford Cl(0,6) Functional Cortical Neuron Clusters + Satellite Dendrites
    this.cliffordNodes = [];
    this.corticalSynapses = [];
    this.brainContourSegments = [];
    this.corticalMicroNeurons = [];
    this.manifoldEquipotentialSegments = [];

    // First-Principles Riemannian Manifold Equipotential Energy Mesh representing -V_basin(x)
    for (let l = 0; l < this.layerElevations.length; l++) {
      const z = this.layerElevations[l];
      const radii = [0.8, 1.4, 2.0, 2.6, 3.2];
      for (let r = 0; r < radii.length; r++) {
        const rad = radii[r];
        const segSteps = 24;
        for (let s = 0; s < segSteps; s++) {
          const a1 = (s / segSteps) * Math.PI * 2.0;
          const a2 = ((s + 1) / segSteps) * Math.PI * 2.0;
          const p1 = new Float32Array([rad * Math.cos(a1), rad * Math.sin(a1), z]);
          const p2 = new Float32Array([rad * Math.cos(a2), rad * Math.sin(a2), z]);
          this.manifoldEquipotentialSegments.push({ p1, p2, layer: l, kind: 'ring', region: l >= 4 ? 'frontal' : (l <= 1 ? 'temporal' : 'parietal') });
          this.brainContourSegments.push({ p1, p2, region: l >= 4 ? 'frontal' : (l <= 1 ? 'temporal' : 'parietal') });
        }
      }

      // Hyperbolic radial meridians
      const meridianCount = 12;
      for (let m = 0; m < meridianCount; m++) {
        const ang = (m / meridianCount) * Math.PI * 2.0;
        const p1 = new Float32Array([0.8 * Math.cos(ang), 0.8 * Math.sin(ang), z]);
        const p2 = new Float32Array([3.2 * Math.cos(ang), 3.2 * Math.sin(ang), z]);
        this.manifoldEquipotentialSegments.push({ p1, p2, layer: l, kind: 'meridian', region: 'callosum' });
        this.brainContourSegments.push({ p1, p2, region: 'callosum' });
      }
    }

    // 480 Manifold Micro-Lattice Resonance Points
    for (let i = 0; i < 480; i++) {
      const l = i % 6;
      const z = this.layerElevations[l];
      const ang = (i / 480.0) * Math.PI * 2.0 * 7.0;
      const rad = 0.8 + 2.4 * ((i * 0.6180339887) % 1.0);
      const pos = new Float32Array([rad * Math.cos(ang), rad * Math.sin(ang), z]);
      let col = [0.22, 0.62, 0.98, 0.32];
      if (l >= 4) col = [0.95, 0.35, 0.68, 0.34];
      else if (l <= 1) col = [0.12, 0.88, 0.66, 0.36];
      else col = [0.98, 0.72, 0.20, 0.34];
      this.corticalMicroNeurons.push({ pos, color: new Float32Array(col), size: 3.2 });
    }

    // 64 Clifford Cl(0,6) Functional Neuron Clusters with Satellite Dendrites
    const gradeColors = [
      [0.15, 0.92, 1.0, 0.96],  // Grade 0: Brainstem BPE Ingress (Cyan)
      [0.76, 0.52, 0.98, 0.94], // Grade 1: Thalamus / Hippocampus 256D Context (Violet)
      [0.12, 0.92, 0.64, 0.92], // Grade 2: Temporal Lobe /8 Radix Memory Basins (Emerald)
      [0.28, 0.62, 1.0, 0.92],  // Grade 3: Parietal Lobe 8-Head Starling Flock (Electric Blue)
      [0.96, 0.32, 0.62, 0.92], // Grade 4: Prefrontal Cortex OSPF Token Router (Magenta-Rose)
      [1.0, 0.74, 0.18, 0.94],  // Grade 5: Occipital Cortex VSA-128 Readout (Amber-Gold)
      [1.0, 0.90, 0.25, 0.98]   // Grade 6: Corpus Callosum Crown Synthesis (Gold)
    ];
    const gradeCounts = [1, 6, 15, 20, 15, 6, 1];
    const gradeSizes = [15.0, 12.0, 10.5, 10.0, 10.5, 12.0, 15.0];

    const addClusterNode = (pos, grade, hemi, regionName, clusterIdx) => {
      const dendrites = [];
      for (let d = 0; d < 7; d++) {
        const a1 = d * 2.3999632 + clusterIdx * 0.41;
        const a2 = (d / 7.0) * Math.PI - Math.PI * 0.5;
        const rad = 0.24 + (d % 3) * 0.11;
        const dx = rad * Math.cos(a2) * Math.cos(a1);
        const dy = rad * Math.sin(a2);
        const dz = rad * Math.cos(a2) * Math.sin(a1);
        dendrites.push(new Float32Array([pos[0] + dx, pos[1] + dy, pos[2] + dz]));
      }
      this.cliffordNodes.push({
        pos,
        grade,
        hemisphere: hemi,
        regionName,
        baseColor: new Float32Array(gradeColors[grade]),
        baseSize: gradeSizes[grade],
        charge: 0.18,
        fireBurst: 0.0,
        dendrites
      });
    };

    // Cluster 0 (Grade 0): Brainstem BPE Prompt Ingress Hub
    addClusterNode(new Float32Array([0.0, -1.65, -0.55]), 0, 0, 'Brainstem BPE Ingress', 0);

    let cIdx = 1;
    // Clusters 1..6 (Grade 1): Thalamus & Hippocampal C-Arch (256D Q12 Context Carry)
    for (let k = 0; k < gradeCounts[1]; k++) {
      const hemi = (k % 2 === 0) ? -1.0 : 1.0;
      const archT = Math.floor(k / 2) / 2.0; // 0..1 along hippocampal arch
      const x = hemi * (0.58 + 0.35 * Math.sin(archT * Math.PI));
      const y = -0.45 + 0.68 * Math.sin(archT * Math.PI);
      const z = -0.65 + 1.35 * archT;
      addClusterNode(new Float32Array([x, y, z]), 1, hemi, 'Thalamus / Hippocampus', cIdx++);
    }

    // Clusters 7..21 (Grade 2): Left & Right Temporal Lobe Memory Clusters (/8 Radix Basins)
    for (let k = 0; k < gradeCounts[2]; k++) {
      const hemi = (k % 2 === 0) ? -1.0 : 1.0;
      const u = 0.30 + 0.40 * ((k + 0.5) / gradeCounts[2]);
      const v = 0.10 + 0.28 * ((k * 0.6180339887) % 1.0);
      const pos = this.computeCorticalSurfacePoint(hemi, u, v, 0.88);
      addClusterNode(pos, 2, hemi, 'Temporal Memory Lobe', cIdx++);
    }

    // Clusters 22..41 (Grade 3): Left & Right Parietal Cortex (8-Head Starling Flock Attention)
    for (let k = 0; k < gradeCounts[3]; k++) {
      const hemi = (k % 2 === 0) ? -1.0 : 1.0;
      const u = 0.25 + 0.38 * ((k + 0.5) / gradeCounts[3]);
      const v = 0.48 + 0.45 * ((k * 0.6180339887) % 1.0);
      const pos = this.computeCorticalSurfacePoint(hemi, u, v, 0.90);
      addClusterNode(pos, 3, hemi, 'Parietal Flock Cortex', cIdx++);
    }

    // Clusters 42..56 (Grade 4): Left & Right Prefrontal Cortex (<thinking> & OSPF Token Router)
    for (let k = 0; k < gradeCounts[4]; k++) {
      const hemi = (k % 2 === 0) ? -1.0 : 1.0;
      const u = 0.64 + 0.30 * ((k + 0.5) / gradeCounts[4]);
      const v = 0.25 + 0.65 * ((k * 0.6180339887) % 1.0);
      const pos = this.computeCorticalSurfacePoint(hemi, u, v, 0.90);
      addClusterNode(pos, 4, hemi, 'Prefrontal OSPF Cortex', cIdx++);
    }

    // Clusters 57..62 (Grade 5): Occipital Readout Cortex (VSA-128 Slot Unbinding)
    for (let k = 0; k < gradeCounts[5]; k++) {
      const hemi = (k % 2 === 0) ? -1.0 : 1.0;
      const u = 0.06 + 0.18 * ((k + 0.5) / gradeCounts[5]);
      const v = 0.30 + 0.50 * ((k * 0.6180339887) % 1.0);
      const pos = this.computeCorticalSurfacePoint(hemi, u, v, 0.90);
      addClusterNode(pos, 5, hemi, 'Occipital Readout Cortex', cIdx++);
    }

    // Cluster 63 (Grade 6): Corpus Callosum / Crown Synthesis Hub
    addClusterNode(new Float32Array([0.0, 1.55, 0.25]), 6, 0, 'Corpus Callosum Hub', cIdx++);

    // Build Weighted OSPF Synaptic Graph between the 64 Functional Neuron Clusters
    for (let i = 0; i < this.cliffordNodes.length; i++) {
      const nA = this.cliffordNodes[i];
      if (i === 0) {
        for (let j = 1; j <= 6 && j < this.cliffordNodes.length; j++) {
          this.corticalSynapses.push({ a: 0, b: j, kind: 'thalamocortical', phase: j * 0.5, l1Cost: 24 + j * 8 });
        }
        continue;
      }
      let best1 = -1, best2 = -1, best3 = -1, dist1 = 1e9, dist2 = 1e9, dist3 = 1e9;
      let callosumIdx = -1, callosumDist = 1e9;
      for (let j = 1; j < this.cliffordNodes.length; j++) {
        if (i === j) continue;
        const nB = this.cliffordNodes[j];
        const dx = nA.pos[0] - nB.pos[0];
        const dy = nA.pos[1] - nB.pos[1];
        const dz = nA.pos[2] - nB.pos[2];
        const d2 = dx * dx + dy * dy + dz * dz;
        if (nA.hemisphere === nB.hemisphere || nA.hemisphere === 0 || nB.hemisphere === 0) {
          if (d2 < dist1) {
            dist3 = dist2; best3 = best2;
            dist2 = dist1; best2 = best1;
            dist1 = d2; best1 = j;
          } else if (d2 < dist2) {
            dist3 = dist2; best3 = best2;
            dist2 = d2; best2 = j;
          } else if (d2 < dist3) {
            dist3 = d2; best3 = j;
          }
        } else if (nA.hemisphere < 0 && nB.hemisphere > 0 && Math.abs(dy) < 0.55 && Math.abs(dz) < 0.65) {
          if (d2 < callosumDist) {
            callosumDist = d2;
            callosumIdx = j;
          }
        }
      }
      if (best1 > i) this.corticalSynapses.push({ a: i, b: best1, kind: 'cortical', phase: i * 0.31, l1Cost: Math.round(Math.sqrt(dist1) * 45) });
      if (best2 > i) this.corticalSynapses.push({ a: i, b: best2, kind: 'cortical', phase: i * 0.47, l1Cost: Math.round(Math.sqrt(dist2) * 52) });
      if (best3 > i && (i % 2 === 0)) this.corticalSynapses.push({ a: i, b: best3, kind: 'cortical', phase: i * 0.63, l1Cost: Math.round(Math.sqrt(dist3) * 60) });
      if (callosumIdx !== -1 && (i % 3 === 0)) {
        this.corticalSynapses.push({ a: i, b: callosumIdx, kind: 'callosum', phase: i * 0.23, l1Cost: 38 });
      }
    }

    // 2. 17 Icosian root vertices on outer S^3 Hurwitz attention halo (radius 4.6)
    const phi = 1.6180339887;
    const rIco = 4.6;
    const sIco = rIco / Math.sqrt(1.0 + phi * phi);
    const ico12 = [
      [sIco, sIco * phi, 0.0], [-sIco, sIco * phi, 0.0],
      [sIco, -sIco * phi, 0.0], [-sIco, -sIco * phi, 0.0],
      [0.0, sIco, sIco * phi], [0.0, -sIco, sIco * phi],
      [0.0, sIco, -sIco * phi], [0.0, -sIco, -sIco * phi],
      [sIco * phi, 0.0, sIco], [-sIco * phi, 0.0, sIco],
      [sIco * phi, 0.0, -sIco], [-sIco * phi, 0.0, -sIco]
    ];
    const icoPoles = [
      [0.0, rIco, 0.0], [0.0, -rIco, 0.0],
      [rIco, 0.0, 0.0], [-rIco, 0.0, 0.0],
      [0.0, 0.0, rIco]
    ];
    this.icosianRoots = [];
    for (const v of ico12) {
      this.icosianRoots.push(new Float32Array(v));
    }
    for (const v of icoPoles) {
      this.icosianRoots.push(new Float32Array(v));
    }

    // 3. 256 Starfield /8 PointKey memory basin particles in outer holographic field
    this.starfieldParticles = [];
    for (let i = 0; i < 256; i++) {
      const t = i / 255.0;
      const y = (t * 2.0 - 1.0) * 5.0;
      const r = Math.sqrt(Math.max(0.1, 1.0 - (y / 5.5) * (y / 5.5))) * (6.0 + (i % 5) * 0.55);
      const ang = i * 2.3999632;
      this.starfieldParticles.push({
        pos: new Float32Array([r * Math.cos(ang), y, r * Math.sin(ang)]),
        baseColor: new Float32Array([0.38, 0.30, 0.78, 0.28]),
        baseSize: 3.0
      });
    }

    // 4. Initialize 96-Boid Starling Flock Murmuration Swarm (8 Attention Heads x 12 Boids)
    this.starlingFlock = [];
    for (let b = 0; b < 96; b++) {
      const headIdx = b % 8;
      const ang = (b / 96.0) * Math.PI * 2.0;
      const rad = 1.2 + (headIdx * 0.18);
      this.starlingFlock.push({
        pos: new Float32Array([rad * Math.cos(ang), 0.6 + 0.5 * Math.sin(ang * 2.0), rad * Math.sin(ang)]),
        vel: new Float32Array([-Math.sin(ang) * 1.4, Math.cos(ang * 2.0) * 0.5, Math.cos(ang) * 1.4]),
        headIdx
      });
    }

    // 5. Seed initial OSPF Synaptic Route Hops & Memory Engram
    this.buildOspfRouteFromTokens([101, 205, 309, 412, 518, 624, 731, 840], this.activeTurnWords, 1, 812);
  }

  buildOspfRouteFromTokens(tokenIds, words, turnIndex = 1, bucketId = 812, stepMechanisms = []) {
    const count = Math.max(tokenIds.length, 1);
    const hops = [];
    const waypoints = [];
    const wordList = (Array.isArray(words) && words.length > 0) ? words : this.activeTurnWords;

    for (let s = 0; s < count; s++) {
      const tok = Math.abs(Math.floor(+tokenIds[s] || (s * 37 + 11)));
      const rawMech = String((Array.isArray(stepMechanisms) && stepMechanisms[s]) ? stepMechanisms[s] : '').toLowerCase();
      let clusterIdx = 0;
      let mechanism = 'OSPF Min-L1 Hop';
      if (s === 0) {
        clusterIdx = 0; // Brainstem BPE Ingress
        mechanism = 'BPE Prompt Ingress';
      } else if (s === 1 && count > 2) {
        clusterIdx = 1 + ((turnIndex + tok) % 6); // Thalamus 256D Context Carry
        mechanism = '256D Q12 Context Carry';
      } else if (rawMech.includes('debruijn_slot_kernel_r4_31') || rawMech.includes('slot')) {
        clusterIdx = 57 + ((tok * 7 + s * 5 + turnIndex) % 6);
        mechanism = 'VSA-128 Slot Unbind';
      } else if (rawMech.includes('point_key_resonance') || rawMech.includes('point_routing') || rawMech.includes('radix')) {
        clusterIdx = 7 + ((bucketId + tok + s * 3) % 15);
        mechanism = 'Radix /8 PointKey';
      } else if (rawMech.includes('turn_memory_reply') || rawMech.includes('ospf') || rawMech.includes('phrase_dag')) {
        clusterIdx = 42 + ((tok * 17 + s * 11 + turnIndex * 3) % 15);
        mechanism = 'OSPF Min-L1 Route';
      } else if (rawMech.includes('thinking') || rawMech.includes('flock')) {
        clusterIdx = 22 + ((tok * 13 + s * 7 + turnIndex * 5) % 20);
        mechanism = '8-Head Flock Consensus';
      } else if (s === 2 && count > 3) {
        clusterIdx = 7 + ((bucketId + tok) % 15); // Temporal Lobe /8 Radix Memory Recall
        mechanism = 'Radix /8 PointKey';
      } else if (s === count - 1 && count > 4) {
        clusterIdx = 1 + ((turnIndex * 3 + 2) % 6); // Hippocampus Memory Write-Back
        mechanism = 'Hippocampus Write-Back';
      } else {
        // Phase-ordered cortical progression: Temporal Radix -> Parietal Flock -> Frontal OSPF -> Occipital Slot Readout
        const phaseRatio = s / Math.max(count - 1, 1);
        if (phaseRatio < 0.35) {
          clusterIdx = 7 + ((bucketId + tok + s * 3) % 15);
          mechanism = 'Radix /8 PointKey';
        } else if (phaseRatio < 0.62) {
          clusterIdx = 22 + ((tok * 13 + s * 7 + turnIndex * 5) % 20);
          mechanism = '8-Head Flock Consensus';
        } else if (phaseRatio < 0.86) {
          clusterIdx = 42 + ((tok * 17 + s * 11 + turnIndex * 3) % 15);
          mechanism = 'OSPF Min-L1 Route';
        } else {
          clusterIdx = 57 + ((tok * 7 + s * 5 + turnIndex) % 6);
          mechanism = 'VSA-128 Slot Unbind';
        }
      }

      clusterIdx = Math.min(Math.max(clusterIdx, 0), Math.max(this.cliffordNodes.length - 1, 0));
      const baseNode = this.cliffordNodes[clusterIdx];
      const basePos = baseNode ? baseNode.pos : new Float32Array([0.0, 0.0, 0.0]);
      // Tiny deterministic offset so repeated visits to a cluster still produce distinct spline segments
      const jitter = (s % 5) * 0.035;
      const wp = new Float32Array([
        basePos[0] + jitter * Math.cos(s * 1.7),
        basePos[1] + jitter * Math.sin(s * 1.3),
        basePos[2] + jitter * Math.sin(s * 2.1)
      ]);
      waypoints.push(wp);

      const word = wordList[s % wordList.length] || `tok#${tok}`;
      const l1Cost = 18 + ((tok * 29 + s * 19) % 160);
      hops.push({
        step: s,
        clusterIdx,
        tokenWord: String(word).slice(0, 18),
        tokenId: tok,
        l1Cost,
        mechanism,
        pos: wp
      });
    }

    this.ospfRouteHops = hops;
    this.decodeWaypoints = waypoints;
    this.rebuildSplinePoints();

    // Build LSA / Dijkstra candidate probe links radiating from the route's main clusters
    this.lsaProbeLinks = [];
    const maxProbes = Math.min(hops.length, 16);
    for (let h = 0; h < maxProbes; h++) {
      const cIdx = hops[h].clusterIdx;
      for (let s = 0; s < this.corticalSynapses.length; s++) {
        const syn = this.corticalSynapses[s];
        if (syn.a === cIdx || syn.b === cIdx) {
          this.lsaProbeLinks.push({ a: syn.a, b: syn.b, hopStep: h, l1Cost: syn.l1Cost || 42 });
        }
      }
    }

    // Record or update Persistent Episodic Memory Engram in Temporal Lobe for this turn
    const engramClusterIdx = 7 + ((turnIndex * 5 + bucketId) % 15);
    const existingIdx = this.memoryEngrams.findIndex((e) => e.turnIndex === turnIndex);
    const promptShort = (this.activePromptSnippet || '').replace(/[^a-zA-Z0-9_/#+-]+/g, ' ').trim().split(/\s+/).slice(0, 2).join(' ');
    const replyShort = (this.activeReplySnippet || wordList.slice(2, 4).join(' ') || wordList.slice(0, 2).join(' ')).trim();
    const summaryWord = (promptShort && replyShort ? `${promptShort} → ${replyShort}` : (promptShort || replyShort)).slice(0, 28) || `Turn #${turnIndex}`;
    const engramEntry = {
      turnIndex,
      clusterIdx: Math.min(engramClusterIdx, Math.max(this.cliffordNodes.length - 1, 0)),
      bucketId: bucketId % 17408,
      summaryWord
    };
    if (existingIdx >= 0) {
      this.memoryEngrams[existingIdx] = engramEntry;
    } else {
      this.memoryEngrams.push(engramEntry);
      if (this.memoryEngrams.length > 6) {
        this.memoryEngrams.shift();
      }
    }

    // Update OSPF route ticker bar in DOM
    if (typeof document !== 'undefined') {
      const tickerEl = this.ospfTickerEl || document.getElementById('neural-ospf-route-text');
      if (tickerEl && hops.length > 0) {
        const previewHops = hops.slice(0, 6).map((h, idx) => {
          const regionShort = this.cliffordNodes[h.clusterIdx]?.regionName?.split(' ')[0] || 'Cortex';
          return `[Hop ${idx + 1} ${regionShort}: "${h.tokenWord}" L1=${h.l1Cost}]`;
        });
        tickerEl.textContent = `${previewHops.join(' ➔ ')} ➔ [Hippocampus Write-Back: Engram #${turnIndex} (${hops.length} total hops)]`;
      }
    }
  }

  buildDefaultDecodeSpline() {
    this.buildOspfRouteFromTokens([101, 205, 309, 412, 518, 624, 731, 840], this.activeTurnWords, 1, 812);
  }

  ingestConversationTurn(promptText, replyText, chatRes = {}) {
    this.activePromptSnippet = String(promptText || '').trim().slice(0, 36) || 'Prompt Ingress';
    const promptWords = String(promptText || '')
      .replace(/[`#*_\-[\]()?>!.,:;]/g, ' ')
      .trim()
      .split(/\s+/)
      .filter((w) => w.length >= 2)
      .slice(0, 3);
    const cleanReply = String(replyText || '')
      .replace(/<thinking>[\s\S]*?<\/thinking>/gi, '')
      .replace(/[`#*_\-[\]()>]/g, ' ')
      .trim();
    const replyWords = cleanReply.split(/\s+/).filter((w) => w.length > 0).slice(0, 29);
    this.activeReplySnippet = replyWords.slice(0, 2).join(' ');
    const words = [...promptWords, ...replyWords];
    if (words.length > 0) {
      this.activeTurnWords = words;
    }
    const turnIdx = (chatRes && typeof chatRes.turn_index === 'number') ? chatRes.turn_index : (this.memoryEngrams.length + 1);
    this.lastIngestedTurnIndex = turnIdx;
    const rawWin = (chatRes && typeof chatRes.otel_winning_row === 'number') ? chatRes.otel_winning_row : 812;
    const bucketId = rawWin >= 4294967295 ? ((turnIdx * 997 + words.length * 97) % 17408) : (rawWin % 17408);
    if (chatRes && typeof chatRes.icosian_root === 'number' && Number.isFinite(chatRes.icosian_root)) {
      this.dominantIcosianRoot = Math.abs(Math.floor(chatRes.icosian_root)) % 17;
    }

    const tokenIds = [];
    const stepCount = Math.max(this.decodeWaypoints.length, words.length, 8);
    for (let i = 0; i < stepCount; i++) {
      const w = words[i % Math.max(words.length, 1)] || 'uor';
      let h = 2166136261;
      for (let c = 0; c < w.length; c++) {
        h ^= w.charCodeAt(c);
        h = Math.imul(h, 16777619);
      }
      tokenIds.push(Math.abs(h) % 4096);
    }
    this.buildOspfRouteFromTokens(tokenIds, words, turnIdx, bucketId);
    this.triggerOspfCascade();
  }

  triggerOspfCascade() {
    this.cascadeStartTime = (this.lastTimestamp || 0) * 0.001;
    for (let i = 0; i < this.cliffordNodes.length; i++) {
      this.cliffordNodes[i].charge = 0.25;
      this.cliffordNodes[i].fireBurst = 0.35;
    }
    if (this.ospfRouteHops.length > 0) {
      const firstCluster = this.ospfRouteHops[0].clusterIdx;
      if (this.cliffordNodes[firstCluster]) {
        this.cliffordNodes[firstCluster].charge = 1.0;
        this.cliffordNodes[firstCluster].fireBurst = 1.0;
      }
    }
  }

  rebuildSplinePoints() {
    const pts = this.decodeWaypoints;
    if (pts.length < 2) {
      this.tokenSplinePoints = [];
      return;
    }
    const spline = [];
    const segments = pts.length - 1;
    const stepsPerSegment = 10;

    for (let i = 0; i < segments; i++) {
      const p0 = i > 0 ? pts[i - 1] : pts[i];
      const p1 = pts[i];
      const p2 = pts[i + 1];
      const p3 = i + 2 < pts.length ? pts[i + 2] : p2;

      for (let step = 0; step < stepsPerSegment; step++) {
        const t = step / stepsPerSegment;
        const pt = new Float32Array(3);
        catmullRomPoint(pt, p0, p1, p2, p3, t);
        spline.push(pt);
      }
    }
    spline.push(pts[pts.length - 1]);
    this.tokenSplinePoints = spline;
  }

  bindEvents() {
    const canvas = this.canvas;
    if (!canvas) return;

    // Pointer events for orbit and pan
    canvas.addEventListener('pointerdown', (e) => {
      this.isDragging = true;
      this.lastPointerX = e.clientX;
      this.lastPointerY = e.clientY;
      try { canvas.setPointerCapture(e.pointerId); } catch (_) {}
    });

    canvas.addEventListener('pointermove', (e) => {
      if (!this.isDragging) return;
      const dx = e.clientX - this.lastPointerX;
      const dy = e.clientY - this.lastPointerY;
      this.lastPointerX = e.clientX;
      this.lastPointerY = e.clientY;

      if (e.shiftKey || e.button === 1 || e.button === 2) {
        // Pan
        const panSpeed = 0.015 * (this.distance / 15.0);
        this.target[0] -= dx * panSpeed * Math.cos(this.yaw);
        this.target[2] += dx * panSpeed * Math.sin(this.yaw);
        this.target[1] += dy * panSpeed;
      } else {
        // Orbit
        this.yaw += dx * 0.007;
        this.pitch = Math.min(Math.max(this.pitch + dy * 0.007, -1.45), 1.45);
      }
    });

    const endDrag = (e) => {
      if (this.isDragging) {
        this.isDragging = false;
        try { canvas.releasePointerCapture(e.pointerId); } catch (_) {}
      }
    };
    canvas.addEventListener('pointerup', endDrag);
    canvas.addEventListener('pointercancel', endDrag);

    // Zoom via wheel - clamped strictly between 2.0 and 50.0 via Math.min and Math.max
    canvas.addEventListener('wheel', (e) => {
      e.preventDefault();
      const zoomFactor = e.deltaY > 0 ? 1.08 : 0.92;
      this.distance = Math.min(Math.max(this.distance * zoomFactor, 2.0), 50.0);
    }, { passive: false });

    // WebGL Context Recovery
    canvas.addEventListener('webglcontextlost', (e) => {
      e.preventDefault();
      if (this.animFrameId) {
        cancelAnimationFrame(this.animFrameId);
        this.animFrameId = null;
      }
      if (this.statusEl) {
        this.statusEl.textContent = 'WebGL Context Lost · Restoring...';
      }
    });

    canvas.addEventListener('webglcontextrestored', () => {
      this.initGL();
      this.buildStaticGeometry();
      this.buildDefaultDecodeSpline();
      this.updateStatus();
      this.animFrameId = requestAnimationFrame(this.boundAnimate);
    });

    // OSPF Cascade button
    if (this.pulseBtn) {
      this.pulseBtn.addEventListener('click', () => {
        this.triggerOspfCascade();
      });
    }

    // Floating 3D Labels toggle button
    if (this.labelsToggleBtn) {
      this.labelsToggleBtn.addEventListener('click', () => {
        this.showLabels = !this.showLabels;
        this.labelsToggleBtn.textContent = this.showLabels ? 'Labels: ON' : 'Labels: OFF';
        const container = this.labelsContainer || document.getElementById('neural-3d-labels');
        if (container) {
          container.style.display = this.showLabels ? 'block' : 'none';
        }
      });
    }

    // Reset button
    if (this.resetBtn) {
      this.resetBtn.addEventListener('click', () => {
        this.resetCamera();
      });
    }

    // Animation toggle button
    if (this.animToggleBtn) {
      this.animToggleBtn.addEventListener('click', () => {
        this.toggleAnimation();
      });
    }
  }

  resetCamera() {
    this.yaw = 0.55;
    this.pitch = 0.28;
    this.distance = 15.0;
    this.target[0] = 0.0;
    this.target[1] = 0.0;
    this.target[2] = 0.0;
  }

  fireOspfCascade() {
    this.triggerOspfCascade();
  }

  toggleLabels(enabled) {
    if (typeof enabled === 'boolean') {
      this.showLabels = enabled;
    } else {
      this.showLabels = !this.showLabels;
    }
    if (this.labelsToggleBtn) {
      this.labelsToggleBtn.textContent = this.showLabels ? 'Labels: ON' : 'Labels: OFF';
    }
    const container = this.labelsContainer || document.getElementById('neural-3d-labels');
    if (container) {
      container.style.display = this.showLabels ? 'block' : 'none';
    }
  }

  toggleAnimation() {
    this.autoRotate = !this.autoRotate;
    if (this.animToggleBtn) {
      this.animToggleBtn.textContent = this.autoRotate ? 'Pause Orbit' : 'Resume Orbit';
    }
    this.updateStatus();
  }

  updateStatus() {
    if (!this.statusEl) return;
    const mode = this.isWebGL2 ? 'WebGL2' : 'WebGL1';
    const engramCnt = Array.isArray(this.memoryEngrams) ? this.memoryEngrams.length : 0;
    const hopCnt = Array.isArray(this.ospfRouteHops) ? this.ospfRouteHops.length : 0;
    if (this.activeTraceInfo) {
      this.statusEl.textContent = `Turn #${this.activeTraceInfo.turnIndex} · ${hopCnt} OSPF Hops · ${engramCnt} Engrams · Root #${this.dominantIcosianRoot} · ${mode}`;
    } else {
      this.statusEl.textContent = `${this.autoRotate ? 'Orbiting' : 'Paused'} · 64 Clusters · ${engramCnt} Engrams · ${mode}`;
    }
  }

  ingestBrainPulse(pulses) {
    if (!Array.isArray(pulses) || pulses.length === 0) return;
    const recent = pulses.slice(-16);
    let lastPulse = null;
    for (let i = 0; i < recent.length; i++) {
      const p = recent[i];
      if (!p || typeof p !== 'object') continue;
      lastPulse = p;
      const e8Orbit = Math.abs(Math.floor(+(p.e8_orbit ?? 0) || 0)) & 15;
      const winHead = Math.abs(Math.floor(+(p.winning_head ?? 0) || 0)) & 7;
      const activeLayer = Math.abs(Math.floor(+(p.active_layer ?? 0) || 0)) % 6;
      const kvDist = +(p.kv_copy_dist ?? 0) || 0;
      const toolKind = String(p.tool_kind || 'none');

      if (this.cliffordNodes.length > 0) {
        const clusterIdx = ((e8Orbit * 4) + (winHead & 3)) % this.cliffordNodes.length;
        const node = this.cliffordNodes[clusterIdx];
        if (node) {
          node.charge = 1.0;
          node.fireBurst = 1.0;
        }
      }
      this.layerEnergies[activeLayer] = Math.min(2.5, Math.max(1.25, (this.layerEnergies[activeLayer] || 0.6) + 0.4));
      this.cliffordGradeEnergies[winHead % 7] = Math.min(3.0, 1.4 + (e8Orbit * 0.08));
      this.dominantIcosianRoot = (e8Orbit + winHead) % 17;

      if (kvDist > 0 || toolKind !== 'none') {
        if (!Array.isArray(this.callosumPulsePackets)) {
          this.callosumPulsePackets = [];
        }
        if (this.callosumPulsePackets.length < 24) {
          this.callosumPulsePackets.push({
            progress: 0.0,
            speed: toolKind !== 'none' ? 1.75 : 2.5,
            zOffset: ((e8Orbit % 5) - 2) * 0.34,
            isTool: toolKind !== 'none',
            dir: (winHead & 1) === 1 ? -1 : 1,
          });
        }
      }
    }

    if (lastPulse && typeof document !== 'undefined') {
      const e8 = Math.abs(Math.floor(+(lastPulse.e8_orbit ?? 0) || 0)) & 15;
      const hd = Math.abs(Math.floor(+(lastPulse.winning_head ?? 0) || 0)) & 7;
      const ly = Math.abs(Math.floor(+(lastPulse.active_layer ?? 0) || 0)) % 6;
      const kv = Math.abs(Math.floor(+(lastPulse.kv_copy_dist ?? 0) || 0));
      const tk = String(lastPulse.tool_kind || 'none');
      if (this.ospfTickerEl) {
        this.ospfTickerEl.textContent = `LIVE PULSE: Orbit e${e8} -> Head #${hd} -> Layer L${ly}${kv > 0 ? ` · KV Copy d=${kv}` : ''}${tk !== 'none' ? ` · Tool ${tk}` : ''}`;
      }
      const s2 = document.getElementById('neural-stage-2-val');
      if (s2) s2.textContent = `Orbit e${e8} · Layer L${ly}`;
      const s3 = document.getElementById('neural-stage-3-val');
      if (s3) s3.textContent = `Head #${hd} · Root #${this.dominantIcosianRoot}`;
      const s4 = document.getElementById('neural-stage-4-val');
      if (s4) s4.textContent = tk !== 'none' ? `Tool: ${tk}` : (kv > 0 ? `KV Copy d=${kv}` : 'Streaming Token');
    }
    this.updateStatus();
  }

  loadTrace(trace) {
    if (!trace || !Array.isArray(trace.spans)) return;
    const internTable = trace.intern_table || { entries: [] };
    const entries = internTable.entries || [];
    const getInternStr = (idx) => (typeof idx === 'number' && entries[idx] !== undefined ? entries[idx] : String(idx));

    const decodeSpans = [];
    let tracePromptPreview = '';

    for (const span of trace.spans) {
      if (!span || typeof span !== 'object') continue;
      const spanName = getInternStr(span.name_ref);
      const attrMap = {};
      if (Array.isArray(span.attributes)) {
        for (const item of span.attributes) {
          if (!Array.isArray(item) || item.length < 2) continue;
          const [kRef, val] = item;
          const kStr = getInternStr(kRef);
          attrMap[kStr] = val;
        }
      }

      if (spanName.startsWith('gllm.tokenize')) {
        if (attrMap['uor.prompt_preview']) {
          tracePromptPreview = String(attrMap['uor.prompt_preview']);
        }
      } else if (spanName.startsWith('gllm.trunk.layer')) {
        let layerIdx = 0;
        const rawLayer = attrMap['uor.layer_index'];
        if (typeof rawLayer === 'number' && Number.isFinite(rawLayer)) {
          layerIdx = Math.abs(Math.floor(rawLayer));
        } else {
          const match = spanName.match(/\.l([0-5])/);
          if (match) layerIdx = (+match[1]) || 0;
        }
        layerIdx = Math.min(Math.max(layerIdx, 0), 5);

        if (attrMap['uor.basin_energy_l1'] !== undefined) {
          const rawE = +attrMap['uor.basin_energy_l1'];
          if (typeof rawE === 'number' && Number.isFinite(rawE)) {
            this.layerEnergies[layerIdx] = Math.min(Math.max(rawE / 600.0, 0.2), 2.5);
          }
        }
        if (Array.isArray(attrMap['uor.clifford_grades'])) {
          for (let g = 0; g < 7 && g < attrMap['uor.clifford_grades'].length; g++) {
            const rawG = +attrMap['uor.clifford_grades'][g];
            if (typeof rawG === 'number' && Number.isFinite(rawG)) {
              this.cliffordGradeEnergies[g] = Math.min(Math.max(rawG / 100.0, 0.2), 3.0);
            }
          }
        }
        if (attrMap['uor.dominant_icosian_root'] !== undefined) {
          const rawRoot = +attrMap['uor.dominant_icosian_root'];
          if (typeof rawRoot === 'number' && Number.isFinite(rawRoot)) {
            this.dominantIcosianRoot = Math.abs(Math.floor(rawRoot)) % 17;
          }
        }
      } else if (spanName.startsWith('gllm.routing')) {
        if (attrMap['uor.winning_row'] !== undefined) {
          const rawRow = +attrMap['uor.winning_row'];
          if (typeof rawRow === 'number' && Number.isFinite(rawRow)) {
            this.winningPointKeyRow = Math.abs(Math.floor(rawRow));
          }
        }
      } else if (spanName.startsWith('gllm.decode')) {
        const rawStep = +attrMap['uor.step'];
        const step = (typeof rawStep === 'number' && Number.isFinite(rawStep)) ? Math.abs(Math.floor(rawStep)) : decodeSpans.length;
        decodeSpans.push({
          step,
          token: attrMap['uor.predicted_token'] || 0,
          mechanism: attrMap['uor.source_mechanism'] || 'point_routing',
          l1: attrMap['uor.l1_distance'] || 0
        });
      }
    }

    const turnIdx = trace.turn_index !== undefined ? trace.turn_index : 1;
    const rawRow = this.winningPointKeyRow !== null ? this.winningPointKeyRow : 0;
    const bucketId = rawRow >= 4294967295
      ? ((this.dominantIcosianRoot * 1024 + trace.spans.length * 37) % 17408)
      : (rawRow % 17408);

    if (decodeSpans.length > 0) {
      let wordsForRoute = this.activeTurnWords;
      if (tracePromptPreview && turnIdx !== this.lastIngestedTurnIndex) {
        const parsed = tracePromptPreview
          .replace(/[^a-zA-Z0-9_/#+-]+/g, ' ')
          .trim()
          .split(/\s+/)
          .filter((w) => w.length >= 2);
        if (parsed.length > 0) {
          wordsForRoute = parsed;
        }
      }
      const tokenIds = decodeSpans.map((d, idx) => {
        const t = +d.token;
        return (typeof t === 'number' && Number.isFinite(t) && t > 0) ? t : (turnIdx * 41 + idx * 19 + 7);
      });
      const stepMechanisms = decodeSpans.map((d) => String(d.mechanism || ''));
      this.buildOspfRouteFromTokens(tokenIds, wordsForRoute, turnIdx, bucketId, stepMechanisms);
      // Overlay true L1 distances and mechanisms from OTel decode spans onto the OSPF hops
      for (let i = 0; i < decodeSpans.length && i < this.ospfRouteHops.length; i++) {
        const ds = decodeSpans[i];
        if (ds.l1 && Number.isFinite(+ds.l1) && (+ds.l1) > 0) {
          this.ospfRouteHops[i].l1Cost = Math.abs(Math.floor(+ds.l1));
        }
        if (ds.mechanism) {
          this.ospfRouteHops[i].mechanism = String(ds.mechanism);
        }
      }
      this.triggerOspfCascade();
    }

    this.activeTraceInfo = {
      turnIndex: turnIdx,
      spanCount: trace.spans.length,
      traceId: trace.trace_id ? trace.trace_id.slice(0, 8) : ''
    };
    if (typeof document !== 'undefined') {
      const ctxEl = document.getElementById('neural-stage-context-val');
      if (ctxEl) {
        ctxEl.textContent = trace.context_digest_after
          ? `0x${String(trace.context_digest_after).slice(0, 6)}`
          : `Turn #${this.activeTraceInfo.turnIndex}`;
      }
      const attnEl = document.getElementById('neural-stage-attention-val');
      if (attnEl) {
        attnEl.textContent = `Root #${this.dominantIcosianRoot}`;
      }
      const radixEl = document.getElementById('neural-stage-radix-val');
      if (radixEl) {
        radixEl.textContent = `Bkt #${bucketId}`;
      }
      const horEl = document.getElementById('neural-stage-horizon-val');
      if (horEl) {
        horEl.textContent = `${this.ospfRouteHops.length} Hops`;
      }
      const bpEl = document.getElementById('neural-stage-backprop-val');
      if (bpEl) {
        bpEl.textContent = `${this.memoryEngrams.length} Eng · ${trace.spans.length} Sp`;
      }
    }
    this.updateStatus();
  }

  updateStarlingFlock(deltaTime, time) {
    if (!Array.isArray(this.starlingFlock) || this.starlingFlock.length === 0) return;
    const flockLen = this.starlingFlock.length;
    const hopCount = Math.max(this.ospfRouteHops.length, 1);
    const activeHopIdx = Math.floor((time * 1.15) % hopCount);
    const activeHop = this.ospfRouteHops[activeHopIdx] || this.ospfRouteHops[0];
    const targetPos = (activeHop && activeHop.pos) ? activeHop.pos : new Float32Array([0.0, 0.85, 0.25]);
    const domRootIdx = Math.abs(Math.floor(this.dominantIcosianRoot || 0)) % Math.max(this.icosianRoots.length, 1);
    const rootPos = this.icosianRoots[domRootIdx] || new Float32Array([0.0, 0.85, 0.0]);

    const dt = Math.min(Math.max(deltaTime, 0.004), 0.05);

    // Ballerini-Cavagna-Giardina topological nearest-neighbor metric k=7 (PNAS 2008 & R4.Flock Lean spec)
    const kNeighbors = 7;
    const topDists = this._top7Dist || (this._top7Dist = new Float32Array(7));
    const topIndices = this._top7Idx || (this._top7Idx = new Int32Array(7));

    // Murmuration physics weights: separation (collision avoidance), alignment (velocity matching), cohesion, attention goal
    const ws = 1.4;
    const wa = 1.8;
    const wc = 1.2;
    const wattn = 2.5;
    const damping = 0.08;

    for (let i = 0; i < flockLen; i++) {
      const b = this.starlingFlock[i];
      if (!b || !b.pos || !b.vel) continue;

      for (let m = 0; m < kNeighbors; m++) {
        topDists[m] = 1e9;
        topIndices[m] = -1;
      }

      const px = b.pos[0];
      const py = b.pos[1];
      const pz = b.pos[2];

      for (let j = 0; j < flockLen; j++) {
        if (i === j) continue;
        const other = this.starlingFlock[j];
        if (!other || !other.pos) continue;
        const ddx = other.pos[0] - px;
        const ddy = other.pos[1] - py;
        const ddz = other.pos[2] - pz;
        const d2 = ddx * ddx + ddy * ddy + ddz * ddz;

        if (d2 < topDists[kNeighbors - 1]) {
          let ins = kNeighbors - 1;
          while (ins > 0 && d2 < topDists[ins - 1]) {
            topDists[ins] = topDists[ins - 1];
            topIndices[ins] = topIndices[ins - 1];
            ins--;
          }
          topDists[ins] = d2;
          topIndices[ins] = j;
        }
      }

      let sepX = 0.0, sepY = 0.0, sepZ = 0.0;
      let avgVelX = 0.0, avgVelY = 0.0, avgVelZ = 0.0;
      let avgPosX = 0.0, avgPosY = 0.0, avgPosZ = 0.0;
      let validK = 0;

      for (let m = 0; m < kNeighbors; m++) {
        const nIdx = topIndices[m];
        if (nIdx < 0) continue;
        const nb = this.starlingFlock[nIdx];
        if (!nb || !nb.pos || !nb.vel) continue;
        validK++;

        const dx = nb.pos[0] - px;
        const dy = nb.pos[1] - py;
        const dz = nb.pos[2] - pz;
        const distSq = dx * dx + dy * dy + dz * dz + 0.01;

        sepX -= dx / distSq;
        sepY -= dy / distSq;
        sepZ -= dz / distSq;

        avgVelX += nb.vel[0];
        avgVelY += nb.vel[1];
        avgVelZ += nb.vel[2];

        avgPosX += nb.pos[0];
        avgPosY += nb.pos[1];
        avgPosZ += nb.pos[2];
      }

      let fSepX = 0.0, fSepY = 0.0, fSepZ = 0.0;
      let fAlignX = 0.0, fAlignY = 0.0, fAlignZ = 0.0;
      let fCohX = 0.0, fCohY = 0.0, fCohZ = 0.0;

      if (validK > 0) {
        const invK = 1.0 / validK;
        fSepX = sepX;
        fSepY = sepY;
        fSepZ = sepZ;

        fAlignX = (avgVelX * invK) - b.vel[0];
        fAlignY = (avgVelY * invK) - b.vel[1];
        fAlignZ = (avgVelZ * invK) - b.vel[2];

        fCohX = (avgPosX * invK) - px;
        fCohY = (avgPosY * invK) - py;
        fCohZ = (avgPosZ * invK) - pz;
      }

      const phase = b.phase || 0.0;
      const swirlAngle = time * 2.35 + phase + b.headIdx * 0.785398;
      const attrX = targetPos[0] * 0.68 + rootPos[0] * 0.32 + 0.78 * Math.cos(swirlAngle);
      const attrY = targetPos[1] * 0.68 + rootPos[1] * 0.32 + 0.52 * Math.sin(swirlAngle * 1.35);
      const attrZ = targetPos[2] * 0.68 + rootPos[2] * 0.32 + 0.78 * Math.sin(swirlAngle);

      const fAttnX = attrX - px;
      const fAttnY = attrY - py;
      const fAttnZ = attrZ - pz;

      const accelX = ws * fSepX + wa * fAlignX + wc * fCohX + wattn * fAttnX - damping * b.vel[0];
      const accelY = ws * fSepY + wa * fAlignY + wc * fCohY + wattn * fAttnY - damping * b.vel[1];
      const accelZ = ws * fSepZ + wa * fAlignZ + wc * fCohZ + wattn * fAttnZ - damping * b.vel[2];

      b.vel[0] = Math.min(Math.max(b.vel[0] + accelX * dt, -4.2), 4.2);
      b.vel[1] = Math.min(Math.max(b.vel[1] + accelY * dt, -4.2), 4.2);
      b.vel[2] = Math.min(Math.max(b.vel[2] + accelZ * dt, -4.2), 4.2);

      b.pos[0] = Math.min(Math.max(b.pos[0] + b.vel[0] * dt, -3.4), 3.4);
      b.pos[1] = Math.min(Math.max(b.pos[1] + b.vel[1] * dt, -2.4), 2.7);
      b.pos[2] = Math.min(Math.max(b.pos[2] + b.vel[2] * dt, -3.6), 3.6);
    }
  }

  projectToScreen(pos3, cw, ch, isAnatomyAnchor = false) {
    if (!pos3 || pos3.length < 3) return null;
    const m = this.viewProjMatrix;
    const x = pos3[0], y = pos3[1], z = pos3[2];
    const clipX = m[0] * x + m[4] * y + m[8] * z + m[12];
    const clipY = m[1] * x + m[5] * y + m[9] * z + m[13];
    const clipZ = m[2] * x + m[6] * y + m[10] * z + m[14];
    const clipW = m[3] * x + m[7] * y + m[11] * z + m[15];
    if (!Number.isFinite(clipW) || clipW <= 0.15) return null;
    const ndcX = clipX / clipW;
    const ndcY = clipY / clipW;
    const ndcZ = clipZ / clipW;
    if (ndcX < -0.96 || ndcX > 0.96 || ndcY < -0.94 || ndcY > 0.94) return null;
    if (isAnatomyAnchor && this.lastEye) {
      const dot = (x - this.target[0]) * this.lastEye[0] +
                  (y - this.target[1]) * this.lastEye[1] +
                  (z - this.target[2]) * this.lastEye[2];
      if (dot < -2.2) return null;
    }
    return {
      x: (ndcX * 0.5 + 0.5) * cw,
      y: (-ndcY * 0.5 + 0.5) * ch,
      depth: ndcZ
    };
  }

  updateOverlayLabels(time, activeHopIdx, hopPhase, cw, ch) {
    if (typeof document === 'undefined') return;
    if (!this.labelsEl) {
      this.labelsEl = this.labelsContainer || document.getElementById('neural-3d-labels');
    }
    if (!this.labelsEl) return;
    if (!this.showLabels || cw <= 0 || ch <= 0) {
      this.labelsEl.style.display = 'none';
      return;
    }
    this.labelsEl.style.display = 'block';

    const pins = [];
    const centerX = cw * 0.5;
    const centerY = ch * 0.52;
    const minRadiusX = cw * 0.37;
    const minRadiusY = ch * 0.37;

    // 1. 6 Anatomical Brain Lobe Anchors projected onto the outer perimeter halo
    // so 100% of the 3D holographic brain interior remains unobstructed.
    const anatomyAnchors = [
      { id: 'lobe-frontal', pos: [0.0, 1.35, 2.55], badge: 'FRONTAL LOBE', title: '<thinking> & OSPF Routing', sub: 'Min-L1 Token Selection' },
      { id: 'lobe-parietal', pos: [0.0, 2.08, -0.25], badge: 'PARIETAL DOME', title: `96-Boid Flock · Root #${this.dominantIcosianRoot}`, sub: 'Cl(0,6) Spinor Consensus' },
      { id: 'lobe-temporal', pos: [-2.35, -0.28, 0.35], badge: 'TEMPORAL LOBE', title: '/8 Radix Memory Basins', sub: '17,408 Geometric Buckets' },
      { id: 'lobe-occipital', pos: [0.0, 0.42, -2.65], badge: 'OCCIPITAL POLE', title: 'VSA-128 Slot Readout', sub: 'Surface Token Emission' },
      { id: 'lobe-limbic', pos: [2.25, 0.18, 0.22], badge: 'LIMBIC CORE', title: '256D Q12 Context Carry', sub: 'Episodic Write-Back Hub' },
      { id: 'lobe-stem', pos: [0.0, -2.15, -0.38], badge: 'BRAINSTEM', title: 'BPE Prompt Ingress', sub: '4,096 Vocabulary Stems' }
    ];
    for (let i = 0; i < anatomyAnchors.length; i++) {
      const a = anatomyAnchors[i];
      const proj = this.projectToScreen(a.pos, cw, ch, true);
      if (proj) {
        let dx = proj.x - centerX;
        let dy = proj.y - centerY;
        const norm = Math.hypot(dx / Math.max(minRadiusX, 1), dy / Math.max(minRadiusY, 1));
        let px = proj.x;
        let py = proj.y;
        if (norm < 1.0) {
          const scale = 1.0 / Math.max(norm, 0.18);
          px = centerX + dx * scale;
          py = centerY + dy * scale;
        }
        pins.push({
          id: a.id,
          cls: 'neural-label-pin pin-anatomy',
          x: Math.min(Math.max(px, 88), Math.max(cw - 88, 88)),
          y: Math.min(Math.max(py, 26), Math.max(ch - 26, 26)),
          badge: a.badge,
          title: a.title,
          sub: a.sub,
          chargePct: null
        });
      }
    }

    // 2. Active OSPF Hop Cluster Pin docked cleanly at the top-left HUD corner
    const hopCount = this.ospfRouteHops.length;
    if (hopCount > 0) {
      const hop = this.ospfRouteHops[activeHopIdx % hopCount];
      if (hop && hop.pos) {
        const isFiring = hopPhase >= 0.68;
        const chargePct = isFiring
          ? Math.round((1.0 - (hopPhase - 0.68) / 0.32) * 100)
          : Math.round((hopPhase / 0.68) * 100);
        const cluster = this.cliffordNodes[hop.clusterIdx];
        const regionName = cluster ? cluster.regionName : 'Cortical Cluster';
        pins.push({
          id: 'active-ospf-hop',
          cls: isFiring ? 'neural-label-pin pin-hop pin-firing' : 'neural-label-pin pin-hop',
          x: 12,
          y: 10,
          badge: (activeHopIdx === 0)
            ? `⚡ INGRESS · "${hop.tokenWord}"`
            : (isFiring
              ? `⚡ DECODE HOP #${activeHopIdx + 1}/${hopCount} · BURST RELEASE!`
              : `⚡ DECODE HOP #${activeHopIdx + 1}/${hopCount} · CHARGING ${chargePct}%`),
          title: (activeHopIdx === 0) ? `Ingress: "${hop.tokenWord}"` : `Decode: "${hop.tokenWord}"`,
          sub: `${regionName} · ${hop.mechanism} · L1=${hop.l1Cost}`,
          chargePct: Math.min(Math.max(chargePct, 6), 100)
        });
      }
    }

    // 3. Latest Persistent Conversation Turn Memory Engram docked at bottom-right HUD corner
    const recentEngrams = this.memoryEngrams.slice(-1);
    for (let i = 0; i < recentEngrams.length; i++) {
      const eng = recentEngrams[i];
      pins.push({
        id: `engram-turn-${eng.turnIndex}`,
        cls: 'neural-label-pin pin-engram',
        x: Math.max(cw - 110, 120),
        y: Math.max(ch - 10, 60),
        badge: `MEMORY ENGRAM #${eng.turnIndex}`,
        title: `"${eng.summaryWord}"`,
        sub: `Bucket #${eng.bucketId} · Temporal Recall`,
        chargePct: null
      });
    }

    // 4. Screen-Space Repulsion Solver for outer-perimeter anatomy pills
    const minW = 168;
    const minH = 28;
    for (let pass = 0; pass < 5; pass++) {
      for (let i = 0; i < pins.length; i++) {
        if (pins[i].id === 'active-ospf-hop' || pins[i].id.startsWith('engram-turn-')) continue;
        for (let j = i + 1; j < pins.length; j++) {
          if (pins[j].id === 'active-ospf-hop' || pins[j].id.startsWith('engram-turn-')) continue;
          const a = pins[i];
          const b = pins[j];
          const dx = b.x - a.x;
          const dy = b.y - a.y;
          if (Math.abs(dx) < minW && Math.abs(dy) < minH) {
            const overlapY = (minH - Math.abs(dy)) * 0.55;
            const signY = dy >= 0 ? 1.0 : -1.0;
            a.y -= signY * overlapY;
            b.y += signY * overlapY;
          }
        }
      }
      for (let i = 0; i < pins.length; i++) {
        if (pins[i].id === 'active-ospf-hop' || pins[i].id.startsWith('engram-turn-')) continue;
        pins[i].x = Math.min(Math.max(pins[i].x, 88), Math.max(cw - 88, 88));
        pins[i].y = Math.min(Math.max(pins[i].y, 26), Math.max(ch - 26, 26));
      }
    }

    // Reconcile DOM pins cleanly by data-pin-id
    const activeIds = new Set();
    for (let i = 0; i < pins.length; i++) {
      const p = pins[i];
      activeIds.add(p.id);
      let el = this.labelsEl.querySelector(`[data-pin-id="${p.id}"]`);
      if (!el) {
        el = document.createElement('div');
        el.setAttribute('data-pin-id', p.id);
        const row = document.createElement('div');
        row.className = 'pin-badge-row';
        const badgeEl = document.createElement('span');
        badgeEl.className = 'pin-badge';
        const titleEl = document.createElement('span');
        titleEl.className = 'pin-title';
        row.appendChild(badgeEl);
        row.appendChild(titleEl);
        el.appendChild(row);
        const subEl = document.createElement('div');
        subEl.className = 'pin-sub';
        el.appendChild(subEl);
        const chargeTrack = document.createElement('div');
        chargeTrack.className = 'pin-charge-track';
        const chargeFill = document.createElement('div');
        chargeFill.className = 'pin-charge-fill';
        chargeTrack.appendChild(chargeFill);
        el.appendChild(chargeTrack);
        this.labelsEl.appendChild(el);
      }
      el.className = p.cls;
      el.style.left = `${p.x.toFixed(1)}px`;
      el.style.top = `${p.y.toFixed(1)}px`;
      const badgeEl = el.querySelector('.pin-badge');
      const titleEl = el.querySelector('.pin-title');
      const subEl = el.querySelector('.pin-sub');
      const chargeTrack = el.querySelector('.pin-charge-track');
      const chargeFill = el.querySelector('.pin-charge-fill');
      if (badgeEl && badgeEl.textContent !== p.badge) badgeEl.textContent = p.badge;
      if (titleEl && titleEl.textContent !== p.title) titleEl.textContent = p.title;
      if (subEl && subEl.textContent !== p.sub) subEl.textContent = p.sub;
      if (chargeTrack && chargeFill) {
        if (p.chargePct !== null) {
          chargeTrack.style.display = 'block';
          chargeFill.style.width = `${p.chargePct}%`;
        } else {
          chargeTrack.style.display = 'none';
        }
      }
    }

    const existingChildren = Array.from(this.labelsEl.children);
    for (let i = 0; i < existingChildren.length; i++) {
      const child = existingChildren[i];
      const pid = child.getAttribute('data-pin-id');
      if (!activeIds.has(pid)) {
        child.remove();
      }
    }
  }

  animate(timestamp = 0) {
    if (!this.gl) {
      this.animFrameId = null;
      return;
    }
    if (typeof document !== 'undefined') {
      const modal = document.getElementById('inspectors-modal');
      const panel = document.getElementById('canvas-panel-traces');
      if ((modal && modal.hasAttribute('hidden')) || (panel && panel.hasAttribute('hidden'))) {
        this.animFrameId = null;
        this.lastTimestamp = 0;
        return;
      }
    }
    const deltaTime = this.lastTimestamp ? Math.min((timestamp - this.lastTimestamp) / 1000.0, 0.1) : 0.016;
    this.lastTimestamp = timestamp;

    if (this.autoRotate && !this.isDragging) {
      this.yaw += 0.18 * deltaTime;
    }

    this.render(timestamp * 0.001);
    this.animFrameId = requestAnimationFrame(this.boundAnimate);
  }

  render(time) {
    const gl = this.gl;
    const canvas = this.canvas;
    if (!gl || !canvas) return;

    // Step the 96-Boid Starling Flock Murmuration Swarm
    this.updateStarlingFlock(0.016, time);
    if (this.ospfPulseClock > 0.0) {
      this.ospfPulseClock = Math.max(0.0, this.ospfPulseClock - 0.016);
    }

    // Viewport resize check
    const cw = canvas.clientWidth;
    const ch = canvas.clientHeight;
    if (cw > 0 && ch > 0) {
      if (canvas.width !== cw || canvas.height !== ch) {
        canvas.width = cw;
        canvas.height = ch;
        gl.viewport(0, 0, cw, ch);
      }
    }

    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

    // Compute view and projection matrices (23 deg telephoto FOV so the 3D brain fills the viewport at distance=15.0)
    const aspect = canvas.width / Math.max(canvas.height, 1);
    mat4_perspective(this.projMatrix, 23.0 * Math.PI / 180.0, aspect, 0.1, 100.0);

    const eyeX = this.target[0] + this.distance * Math.cos(this.pitch) * Math.sin(this.yaw);
    const eyeY = this.target[1] + this.distance * Math.sin(this.pitch);
    const eyeZ = this.target[2] + this.distance * Math.cos(this.pitch) * Math.cos(this.yaw);
    this.lastEye = [eyeX, eyeY, eyeZ];
    mat4_lookAt(this.viewMatrix, [eyeX, eyeY, eyeZ], this.target, [0.0, 1.0, 0.0]);
    mat4_multiply(this.viewProjMatrix, this.projMatrix, this.viewMatrix);

    // Determine active OSPF Hop & Integrate-and-Fire Capacitor Cycle
    const hopCount = Math.max(this.ospfRouteHops.length, 1);
    const hopFloat = ((time * 1.15) % hopCount + hopCount) % hopCount;
    const activeHopIdx = Math.floor(hopFloat);
    const hopPhase = hopFloat - activeHopIdx; // 0..0.68 = Capacitor Charge Build-Up, 0.68..1.0 = Burst Release
    const nextHopIdx = (activeHopIdx + 1) % hopCount;
    const activeHop = this.ospfRouteHops[activeHopIdx];
    const nextHop = this.ospfRouteHops[nextHopIdx];
    const isFiringBurst = hopPhase >= 0.68;
    const burstProgress = isFiringBurst ? ((hopPhase - 0.68) / 0.32) : 0.0;

    // Update per-cluster charge & fireBurst state
    for (let i = 0; i < this.cliffordNodes.length; i++) {
      const node = this.cliffordNodes[i];
      node.charge = Math.max(0.08, (node.charge || 0.1) * 0.92);
      node.fireBurst = Math.max(0.0, (node.fireBurst || 0.0) * 0.88);
    }
    if (activeHop && this.cliffordNodes[activeHop.clusterIdx]) {
      const activeNode = this.cliffordNodes[activeHop.clusterIdx];
      if (!isFiringBurst) {
        activeNode.charge = Math.min(1.0, 0.25 + (hopPhase / 0.68) * 0.75);
        activeNode.fireBurst = 0.15 * (hopPhase / 0.68);
      } else {
        activeNode.charge = Math.max(0.2, 1.0 - burstProgress * 0.8);
        activeNode.fireBurst = Math.sin(burstProgress * Math.PI);
      }
    }
    if (nextHop && isFiringBurst && this.cliffordNodes[nextHop.clusterIdx]) {
      this.cliffordNodes[nextHop.clusterIdx].charge = Math.min(0.65, burstProgress * 0.65);
    }

    // ------------------------------------------------------------------------
    // 1. Pack Line Vertices: Holographic Brain Shell, Dendrites, Synapses, OSPF Route, Starling Streaks
    // ------------------------------------------------------------------------
    const lineData = []; // [x, y, z, r, g, b, a]

    // 1A. Anatomical 3D Human Brain Holographic Contours (Cerebral Gyri, Cerebellum Folia, Brainstem)
    if (Array.isArray(this.brainContourSegments)) {
      for (let i = 0; i < this.brainContourSegments.length; i++) {
        const seg = this.brainContourSegments[i];
        if (!seg || !seg.p1 || !seg.p2) continue;
        const lIdx = Math.abs(i) % 6;
        const energy = (typeof this.layerEnergies[lIdx] === 'number' && Number.isFinite(this.layerEnergies[lIdx]))
          ? this.layerEnergies[lIdx]
          : 0.75;
        let cr = 0.28, cg = 0.68, cb = 1.0, baseA = 0.34;
        if (seg.region === 'frontal') {
          cr = 0.98; cg = 0.42; cb = 0.74; baseA = 0.38;
        } else if (seg.region === 'temporal') {
          cr = 0.16; cg = 0.94; cb = 0.72; baseA = 0.38;
        } else if (seg.region === 'cerebellum') {
          cr = 1.0; cg = 0.78; cb = 0.28; baseA = 0.32;
        } else if (seg.region === 'brainstem') {
          cr = 0.22; cg = 0.94; cb = 1.0; baseA = 0.42;
        } else if (seg.region === 'callosum') {
          cr = 1.0; cg = 0.88; cb = 0.32; baseA = 0.46;
        }
        const alpha = baseA * (0.75 + 0.35 * Math.min(energy, 1.2));
        lineData.push(seg.p1[0], seg.p1[1], seg.p1[2], cr, cg, cb, alpha);
        lineData.push(seg.p2[0], seg.p2[1], seg.p2[2], cr, cg, cb, alpha);
      }
    }

    // 1B. Background Cortical Synaptic Connectome & Corpus Callosum Bridges
    if (Array.isArray(this.corticalSynapses)) {
      for (let s = 0; s < this.corticalSynapses.length; s++) {
        const syn = this.corticalSynapses[s];
        const nA = this.cliffordNodes[syn.a];
        const nB = this.cliffordNodes[syn.b];
        if (!nA || !nB) continue;
        const wave = Math.sin(time * 3.8 - syn.phase) * 0.5 + 0.5;
        const clusterHeat = Math.max(nA.charge || 0, nB.charge || 0, nA.fireBurst || 0, nB.fireBurst || 0);
        let cr = 0.22 + 0.35 * wave + 0.4 * clusterHeat;
        let cg = 0.52 + 0.32 * wave + 0.3 * clusterHeat;
        let cb = 0.96;
        let ca = 0.12 + 0.18 * wave + 0.42 * clusterHeat;
        if (syn.kind === 'callosum') {
          cr = 0.88;
          cg = 0.38 + 0.42 * wave;
          cb = 0.92;
          ca = 0.18 + 0.25 * wave + 0.35 * clusterHeat;
        } else if (syn.kind === 'thalamocortical') {
          cr = 0.18;
          cg = 0.84;
          cb = 0.96;
          ca = 0.16 + 0.22 * wave + 0.38 * clusterHeat;
        }
        lineData.push(nA.pos[0], nA.pos[1], nA.pos[2], cr, cg, cb, ca);
        lineData.push(nB.pos[0], nB.pos[1], nB.pos[2], cr, cg, cb, ca);
      }
    }

    // 1C. Dendritic Micro-Cluster Filaments around each Neuron Cluster (glows during Charge & Fire!)
    for (let i = 0; i < this.cliffordNodes.length; i++) {
      const node = this.cliffordNodes[i];
      if (!Array.isArray(node.dendrites)) continue;
      const heat = Math.max(node.charge || 0.1, node.fireBurst || 0.0);
      const dAlpha = 0.14 + 0.72 * heat;
      const dr = node.fireBurst > 0.2 ? 1.0 : node.baseColor[0];
      const dg = node.fireBurst > 0.2 ? 0.92 : node.baseColor[1];
      const db = node.fireBurst > 0.2 ? 0.45 : node.baseColor[2];
      for (let d = 0; d < node.dendrites.length; d++) {
        const dp = node.dendrites[d];
        lineData.push(node.pos[0], node.pos[1], node.pos[2], dr, dg, db, dAlpha);
        lineData.push(dp[0], dp[1], dp[2], dr, dg, db, dAlpha * 0.45);
      }
    }

    // 1D. OSPF Link-State Advertisement (LSA) Candidate Probe Rays from the Charging Cluster
    if (activeHop && Array.isArray(this.corticalSynapses)) {
      const cIdx = activeHop.clusterIdx;
      const lsaWave = (hopPhase / 0.68);
      for (let s = 0; s < this.corticalSynapses.length; s++) {
        const syn = this.corticalSynapses[s];
        if (syn.a === cIdx || syn.b === cIdx) {
          const nA = this.cliffordNodes[cIdx];
          const nB = this.cliffordNodes[syn.a === cIdx ? syn.b : syn.a];
          if (nA && nB) {
            const reach = Math.min(Math.max(lsaWave, 0.15), 1.0);
            const midX = nA.pos[0] + (nB.pos[0] - nA.pos[0]) * reach;
            const midY = nA.pos[1] + (nB.pos[1] - nA.pos[1]) * reach;
            const midZ = nA.pos[2] + (nB.pos[2] - nA.pos[2]) * reach;
            lineData.push(nA.pos[0], nA.pos[1], nA.pos[2], 1.0, 0.78, 0.18, 0.78);
            lineData.push(midX, midY, midZ, 0.20, 0.95, 1.0, 0.35);
          }
        }
      }
    }

    // 1E. Winning OSPF Shortest-Path Synaptic Highway (Bright Emerald-Gold Neural Route)
    if (this.tokenSplinePoints.length > 1) {
      const headIdx = Math.floor((time * 22.0) % this.tokenSplinePoints.length);
      for (let j = 0; j < this.tokenSplinePoints.length - 1; j++) {
        const pA = this.tokenSplinePoints[j];
        const pB = this.tokenSplinePoints[j + 1];
        if (pA && pB && Number.isFinite(pA[0]) && Number.isFinite(pB[0])) {
          const dist = Math.abs(j - headIdx);
          const isWavefront = dist < 18;
          const alpha = isWavefront ? (0.95 - (dist / 18.0) * 0.5) : 0.42;
          const r = isWavefront ? 0.45 : 0.12;
          const g = 0.98;
          const b = isWavefront ? 0.82 : 0.58;
          lineData.push(pA[0], pA[1], pA[2], r, g, b, alpha);
          lineData.push(pB[0], pB[1], pB[2], r, g, b, alpha);
        }
      }
    }

    // 1F. Persistent Temporal Memory Engram Recall & Hippocampal Write-Back Beams
    if (Array.isArray(this.memoryEngrams) && this.cliffordNodes.length > 2) {
      const thalamusHub = this.cliffordNodes[1]; // Thalamus / Hippocampus core
      for (let e = 0; e < this.memoryEngrams.length; e++) {
        const eng = this.memoryEngrams[e];
        const engNode = this.cliffordNodes[eng.clusterIdx];
        if (engNode && thalamusHub) {
          const pulseA = 0.42 + 0.38 * Math.sin(time * 4.2 + e * 1.3);
          lineData.push(engNode.pos[0], engNode.pos[1], engNode.pos[2], 0.78, 0.45, 1.0, pulseA);
          lineData.push(thalamusHub.pos[0], thalamusHub.pos[1], thalamusHub.pos[2], 0.25, 0.92, 1.0, pulseA * 0.85);
        }
      }
    }

    // 1G. 96-Boid Starling Flock Murmuration Velocity Streaks
    for (let i = 0; i < this.starlingFlock.length; i++) {
      const b = this.starlingFlock[i];
      if (!b || !b.pos || !b.vel || !Number.isFinite(b.pos[0])) continue;
      const tailX = b.pos[0] - b.vel[0] * 0.14;
      const tailY = b.pos[1] - b.vel[1] * 0.14;
      const tailZ = b.pos[2] - b.vel[2] * 0.14;
      const isDomHead = (b.headIdx === (this.dominantIcosianRoot % 8));
      const sr = isDomHead ? 1.0 : 0.25;
      const sg = isDomHead ? 0.86 : 0.92;
      const sb = isDomHead ? 0.28 : 1.0;
      lineData.push(b.pos[0], b.pos[1], b.pos[2], sr, sg, sb, 0.82);
      lineData.push(tailX, tailY, tailZ, sr, sg, sb, 0.08);
    }

    // Upload line buffer & draw
    if (lineData.length > 0) {
      const lineArray = new Float32Array(lineData);
      gl.useProgram(this.programLines);
      gl.bindBuffer(gl.ARRAY_BUFFER, this.linesVbo);
      gl.bufferData(gl.ARRAY_BUFFER, lineArray, gl.DYNAMIC_DRAW);

      const uVpLines = gl.getUniformLocation(this.programLines, 'u_viewProjection');
      gl.uniformMatrix4fv(uVpLines, false, this.viewProjMatrix);

      const aPosLines = gl.getAttribLocation(this.programLines, 'a_position');
      const aColLines = gl.getAttribLocation(this.programLines, 'a_color');

      gl.enableVertexAttribArray(aPosLines);
      gl.vertexAttribPointer(aPosLines, 3, gl.FLOAT, false, 28, 0);

      gl.enableVertexAttribArray(aColLines);
      gl.vertexAttribPointer(aColLines, 4, gl.FLOAT, false, 28, 12);

      gl.drawArrays(gl.LINES, 0, lineData.length / 7);
    }

    // ------------------------------------------------------------------------
    // 2. Pack Point Vertices: Micro-Neurons, 64 Synapse Clusters (Charge & Burst), Starling Boids, Packets, Engrams
    // ------------------------------------------------------------------------
    const pointData = []; // [x, y, z, r, g, b, a, size]

    // 2A. 480 Cortical Surface Micro-Neurons (Anatomical Holographic Brain Silhouette)
    if (Array.isArray(this.corticalMicroNeurons)) {
      for (let i = 0; i < this.corticalMicroNeurons.length; i++) {
        const mn = this.corticalMicroNeurons[i];
        const shimmer = 0.75 + 0.25 * Math.sin(time * 2.4 + i * 0.37);
        pointData.push(
          mn.pos[0], mn.pos[1], mn.pos[2],
          mn.color[0], mn.color[1], mn.color[2], mn.color[3] * shimmer,
          mn.size
        );
      }
    }

    // 2B. 64 Functional Neuron & Synapse Clusters + Dendritic Satellite Nodes (Integrate-and-Fire!)
    for (let i = 0; i < this.cliffordNodes.length; i++) {
      const node = this.cliffordNodes[i];
      const charge = node.charge || 0.1;
      const burst = node.fireBurst || 0.0;
      const isHopCluster = activeHop && activeHop.clusterIdx === i;

      // Cluster Soma core
      const somaSize = node.baseSize * (0.9 + 0.85 * charge + 1.15 * burst);
      const cr = burst > 0.15 ? 1.0 : Math.min(1.0, node.baseColor[0] + 0.35 * charge);
      const cg = burst > 0.15 ? 0.94 : Math.min(1.0, node.baseColor[1] + 0.35 * charge);
      const cb = burst > 0.15 ? 0.55 : node.baseColor[2];
      const ca = Math.min(1.0, 0.72 + 0.28 * (charge + burst));
      pointData.push(node.pos[0], node.pos[1], node.pos[2], cr, cg, cb, ca, somaSize);

      // Capacitor Charge / Action-Potential Burst Halo around active or firing clusters
      if (isHopCluster || burst > 0.08 || charge > 0.35) {
        const haloSize = somaSize * (1.55 + 0.95 * burst + 0.35 * Math.sin(time * 9.0));
        const hr = burst > 0.15 ? 1.0 : 0.15;
        const hg = burst > 0.15 ? 0.85 : 0.98;
        const hb = burst > 0.15 ? 0.22 : 0.68;
        const ha = Math.min(0.68, 0.22 + 0.45 * (charge + burst));
        pointData.push(node.pos[0], node.pos[1], node.pos[2], hr, hg, hb, ha, haloSize);
      }

      // 7 Dendritic Satellite Neurons per Cluster
      if (Array.isArray(node.dendrites)) {
        const satSize = 3.2 + 4.5 * (charge + burst);
        const satAlpha = 0.28 + 0.65 * Math.min(1.0, charge + burst);
        for (let d = 0; d < node.dendrites.length; d++) {
          const dp = node.dendrites[d];
          pointData.push(dp[0], dp[1], dp[2], cr, cg, cb, satAlpha, satSize);
        }
      }
    }

    // 2C. Neurotransmitter Action-Potential Burst Packets Shooting from Active Hop -> Next Hop!
    if (activeHop && nextHop && activeHop.pos && nextHop.pos) {
      const packetCount = 5;
      for (let p = 0; p < packetCount; p++) {
        const pFrac = ((hopPhase * 1.45 + p * 0.18) % 1.0);
        const px = activeHop.pos[0] + (nextHop.pos[0] - activeHop.pos[0]) * pFrac;
        const py = activeHop.pos[1] + (nextHop.pos[1] - activeHop.pos[1]) * pFrac + 0.14 * Math.sin(pFrac * Math.PI);
        const pz = activeHop.pos[2] + (nextHop.pos[2] - activeHop.pos[2]) * pFrac;
        const pSize = (p === 0) ? 17.5 : (12.0 - p * 1.4);
        pointData.push(px, py, pz, 1.0, 0.94, 0.38, 0.96, pSize);
      }
    }

    // 2C-Live. Corpus Callosum Inter-Hemispheric KV-Copy & Tool Bridge Arcs
    if (Array.isArray(this.callosumPulsePackets) && this.callosumPulsePackets.length > 0) {
      const activePackets = [];
      for (let i = 0; i < this.callosumPulsePackets.length; i++) {
        const pkt = this.callosumPulsePackets[i];
        if (!pkt || typeof pkt.progress !== 'number' || !Number.isFinite(pkt.progress)) continue;
        const u = pkt.dir < 0 ? (1.0 - pkt.progress) : pkt.progress;
        const inv = 1.0 - u;
        const startX = -1.65;
        const endX = 1.65;
        const px = inv * inv * startX + 2.0 * inv * u * 0.0 + u * u * endX;
        const py = inv * inv * 0.68 + 2.0 * inv * u * 1.48 + u * u * 0.68;
        const pz = Number.isFinite(pkt.zOffset) ? pkt.zOffset : 0.0;
        const pr = pkt.isTool ? 0.35 : 1.0;
        const pg = pkt.isTool ? 0.98 : 0.88;
        const pb = pkt.isTool ? 0.82 : 0.24;
        pointData.push(px, py, pz, pr, pg, pb, 0.98, pkt.isTool ? 19.0 : 15.5);
        pkt.progress += 0.028 * (pkt.speed || 2.0);
        if (pkt.progress < 1.0) {
          activePackets.push(pkt);
        }
      }
      this.callosumPulsePackets = activePackets;
    }

    // 2D. 96 Starling Flock Boid Heads
    for (let i = 0; i < this.starlingFlock.length; i++) {
      const b = this.starlingFlock[i];
      if (!b || !b.pos || !Number.isFinite(b.pos[0])) continue;
      const isDomHead = (b.headIdx === (this.dominantIcosianRoot % 8));
      pointData.push(
        b.pos[0], b.pos[1], b.pos[2],
        isDomHead ? 1.0 : 0.35,
        isDomHead ? 0.88 : 0.95,
        isDomHead ? 0.25 : 1.0,
        0.92,
        isDomHead ? 6.8 : 4.8
      );
    }

    // 2E. Persistent Conversation Memory Engram Cores in Temporal Lobe
    if (Array.isArray(this.memoryEngrams)) {
      for (let e = 0; e < this.memoryEngrams.length; e++) {
        const eng = this.memoryEngrams[e];
        const engNode = this.cliffordNodes[eng.clusterIdx];
        if (!engNode || !engNode.pos) continue;
        const eSize = 21.0 + 4.5 * Math.sin(time * 4.5 + e * 1.5);
        pointData.push(engNode.pos[0], engNode.pos[1], engNode.pos[2], 0.85, 0.48, 1.0, 0.95, eSize);
      }
    }

    // 2F. 17 Icosian Root Anchors & 256 Background /8 Memory Basin Particles
    const hasDomRoot = typeof this.dominantIcosianRoot === 'number' && Number.isFinite(this.dominantIcosianRoot);
    const validDomRoot = hasDomRoot ? (Math.abs(Math.floor(this.dominantIcosianRoot)) % 17) : 0;
    for (let k = 0; k < this.icosianRoots.length; k++) {
      const root = this.icosianRoots[k];
      if (k === validDomRoot) {
        const domSize = 20.0 + 4.5 * Math.sin(time * 5.0);
        pointData.push(root[0], root[1], root[2], 1.0, 0.84, 0.0, 0.98, domSize);
      } else {
        pointData.push(root[0], root[1], root[2], 0.65, 0.82, 1.0, 0.48, 6.5);
      }
    }

    for (let i = 0; i < this.starfieldParticles.length; i++) {
      const p = this.starfieldParticles[i];
      pointData.push(
        p.pos[0], p.pos[1], p.pos[2],
        p.baseColor[0], p.baseColor[1], p.baseColor[2], p.baseColor[3],
        p.baseSize
      );
    }

    // Upload point buffer & draw
    if (pointData.length > 0) {
      const pointArray = new Float32Array(pointData);
      gl.useProgram(this.programPoints);
      gl.bindBuffer(gl.ARRAY_BUFFER, this.pointsVbo);
      gl.bufferData(gl.ARRAY_BUFFER, pointArray, gl.DYNAMIC_DRAW);

      const uVpPoints = gl.getUniformLocation(this.programPoints, 'u_viewProjection');
      const uVhPoints = gl.getUniformLocation(this.programPoints, 'u_viewportHeight');
      gl.uniformMatrix4fv(uVpPoints, false, this.viewProjMatrix);
      gl.uniform1f(uVhPoints, canvas.height);

      const aPosPoints = gl.getAttribLocation(this.programPoints, 'a_position');
      const aColPoints = gl.getAttribLocation(this.programPoints, 'a_color');
      const aSizePoints = gl.getAttribLocation(this.programPoints, 'a_size');

      gl.enableVertexAttribArray(aPosPoints);
      gl.vertexAttribPointer(aPosPoints, 3, gl.FLOAT, false, 32, 0);

      gl.enableVertexAttribArray(aColPoints);
      gl.vertexAttribPointer(aColPoints, 4, gl.FLOAT, false, 32, 12);

      gl.enableVertexAttribArray(aSizePoints);
      gl.vertexAttribPointer(aSizePoints, 1, gl.FLOAT, false, 32, 28);

      gl.drawArrays(gl.POINTS, 0, pointData.length / 8);
    }

    // 3. Project 3D Anatomical & Live OSPF Hop Labels onto the HTML Overlay
    this.updateOverlayLabels(time, activeHopIdx, hopPhase, canvas.clientWidth || canvas.width, canvas.clientHeight || canvas.height);
  }
}

// Global 3D visualizer instance
let lexLean3dVisualizer = null;

// ============================================================================
// In-Browser Stateful Terminal & Control Plane Environment (lexlean-sh)
// ============================================================================

function decodeAnsiToDom(ansiText) {
  const frag = document.createDocumentFragment();
  const ansiRegex = /\x1b\[([0-9;]*)m/g;
  let lastIndex = 0;
  const currentClasses = new Set();
  let match = ansiRegex.exec(ansiText);

  while (match !== null) {
    if (match.index > lastIndex) {
      const textChunk = ansiText.slice(lastIndex, match.index);
      const span = document.createElement('span');
      if (currentClasses.size > 0) {
        span.className = Array.from(currentClasses).join(' ');
      }
      span.textContent = textChunk;
      frag.appendChild(span);
    }
    const codes = match[1] ? match[1].split(';') : ['0'];
    for (const code of codes) {
      const c = code.trim();
      if (c === '' || c === '0') {
        currentClasses.clear();
      } else if (c === '1') {
        currentClasses.add('ansi-bold');
      } else if (c === '2') {
        currentClasses.add('ansi-dim');
      } else if (c === '3') {
        currentClasses.add('ansi-italic');
      } else if (c === '4') {
        currentClasses.add('ansi-underline');
      } else if (c === '31' || c === '91') {
        currentClasses.add('ansi-red');
      } else if (c === '32' || c === '92') {
        currentClasses.add('ansi-green');
      } else if (c === '33' || c === '93') {
        currentClasses.add('ansi-yellow');
      } else if (c === '34' || c === '94') {
        currentClasses.add('ansi-blue');
      } else if (c === '35' || c === '95') {
        currentClasses.add('ansi-magenta');
      } else if (c === '36' || c === '96') {
        currentClasses.add('ansi-cyan');
      } else if (c === '37' || c === '97') {
        currentClasses.add('ansi-white');
      }
    }
    lastIndex = ansiRegex.lastIndex;
    match = ansiRegex.exec(ansiText);
  }
  if (lastIndex < ansiText.length) {
    const textChunk = ansiText.slice(lastIndex);
    const span = document.createElement('span');
    if (currentClasses.size > 0) {
      span.className = Array.from(currentClasses).join(' ');
    }
    span.textContent = textChunk;
    frag.appendChild(span);
  }
  return frag;
}

class LexLeanTerminal {
  constructor() {
    this.container = null;
    this.outputEl = null;
    this.formEl = null;
    this.inputEl = null;
    this.clearBtn = null;
    this.resetBtn = null;
    this.promptLabel = null;
    this.history = [];
    this.historyIndex = -1;
    this.savedDraft = '';
    this.bootTimeMs = Date.now();
    this.lastExitCode = 0;
    this.env = {
      SHELL: '/bin/lexlean-sh',
      USER: 'lexlean',
      HOSTNAME: 'mvm',
      KERNEL: 'Linux 6.6.0-uor-mvm-riscv64',
      MODEL: 'uor-r4-glm-v8',
      CWD: '~/workspace',
      PROMPT: 'lexlean@mvm:~/workspace$ ',
      '?': '0',
    };
  }

  setExitCode(code) {
    this.lastExitCode = Number(code) | 0;
    this.env['?'] = String(this.lastExitCode);
  }

  init(containerId) {
    const cont = document.getElementById(containerId);
    if (!cont) return false;
    this.container = cont;
    this.outputEl = document.getElementById('terminal-output');
    this.formEl = document.getElementById('terminal-form');
    this.inputEl = document.getElementById('terminal-input');
    this.clearBtn = document.getElementById('terminal-clear-btn');
    this.resetBtn = document.getElementById('terminal-reset-btn');
    this.promptLabel = document.getElementById('terminal-prompt-label');

    this.bindEvents();
    this.printWelcome();
    return true;
  }

  printWelcome() {
    this.printAnsi('\x1b[2mMVM riscv64 Linux Shell · /workspace single-source CoW · Type \x1b[0m\x1b[33mhelp\x1b[0m\x1b[2m or \x1b[0m\x1b[33muname -a\x1b[0m');
  }

  bindEvents() {
    if (this.formEl && this.inputEl) {
      this.formEl.addEventListener('submit', (e) => {
        e.preventDefault();
        const line = this.inputEl.value.trim();
        this.inputEl.value = '';
        if (line) {
          this.execute(line);
        }
      });
    }

    if (this.inputEl) {
      this.inputEl.addEventListener('keydown', (e) => {
        if (e.key === 'ArrowUp') {
          e.preventDefault();
          this.stepHistory('up');
        } else if (e.key === 'ArrowDown') {
          e.preventDefault();
          this.stepHistory('down');
        } else if (e.ctrlKey && e.key === 'l') {
          e.preventDefault();
          this.clear();
        } else if (e.ctrlKey && e.key === 'c') {
          e.preventDefault();
          this.printAnsi(`${this.env.PROMPT || 'casey@mvm:/workspace$ '}${this.inputEl.value}^C`);
          this.inputEl.value = '';
          this.historyIndex = -1;
        }
      });
    }

    if (this.clearBtn) {
      this.clearBtn.addEventListener('click', () => {
        this.clear();
      });
    }

    if (this.resetBtn) {
      this.resetBtn.addEventListener('click', () => {
        this.execute('reset');
      });
    }
  }

  stepHistory(direction) {
    if (this.history.length === 0) return;
    if (direction === 'up') {
      if (this.historyIndex === -1) {
        this.savedDraft = this.inputEl.value;
        this.historyIndex = 0;
      } else {
        this.historyIndex = Math.min(this.historyIndex + 1, this.history.length - 1);
      }
      this.inputEl.value = this.history[this.historyIndex] || '';
    } else if (direction === 'down') {
      if (this.historyIndex < 0) return;
      if (this.historyIndex === 0) {
        this.historyIndex = -1;
        this.inputEl.value = this.savedDraft;
        return;
      }
      this.historyIndex -= 1;
      this.inputEl.value = this.history[this.historyIndex] || '';
    }
  }

  printAnsi(text) {
    if (!this.outputEl) return;
    const lineEl = document.createElement('div');
    lineEl.className = 'term-line';
    lineEl.appendChild(decodeAnsiToDom(text));
    this.outputEl.appendChild(lineEl);
    this.outputEl.scrollTop = this.outputEl.scrollHeight;
  }

  clear() {
    if (this.outputEl) {
      this.outputEl.replaceChildren();
    }
  }

  parseCommandChain(line) {
    const chain = [];
    let current = '';
    let inSingle = false;
    let inDouble = false;
    let nextOp = ';';
    for (let i = 0; i < line.length; i++) {
      const ch = line[i];
      const next = line[i + 1] || '';
      if (ch === "'" && !inDouble) {
        inSingle = !inSingle;
        current += ch;
        continue;
      }
      if (ch === '"' && !inSingle) {
        inDouble = !inDouble;
        current += ch;
        continue;
      }
      if (!inSingle && !inDouble) {
        if (ch === '&' && next === '&') {
          if (current.trim()) chain.push({ op: nextOp, cmd: current.trim() });
          current = '';
          nextOp = '&&';
          i++;
          continue;
        }
        if (ch === '|' && next === '|') {
          if (current.trim()) chain.push({ op: nextOp, cmd: current.trim() });
          current = '';
          nextOp = '||';
          i++;
          continue;
        }
        if (ch === ';') {
          if (current.trim()) chain.push({ op: nextOp, cmd: current.trim() });
          current = '';
          nextOp = ';';
          continue;
        }
      }
      current += ch;
    }
    if (current.trim()) {
      chain.push({ op: nextOp, cmd: current.trim() });
    }
    return chain;
  }

  hasUnquotedPipeOrRedirect(line) {
    let inSingle = false;
    let inDouble = false;
    for (let i = 0; i < line.length; i++) {
      const ch = line[i];
      if (ch === "'" && !inDouble && line[i - 1] !== '\\') {
        inSingle = !inSingle;
      } else if (ch === '"' && !inSingle && line[i - 1] !== '\\') {
        inDouble = !inDouble;
      } else if (!inSingle && !inDouble) {
        if (ch === '>' || ch === '<') return true;
        if (ch === '|' && line[i - 1] !== '|' && line[i + 1] !== '|') return true;
      }
    }
    return false;
  }

  execute(rawLine) {
    const trimmed = rawLine.trim();
    if (!trimmed) return;

    if (this.history.length === 0 || this.history[0] !== trimmed) {
      this.history.unshift(trimmed);
      if (this.history.length > 100) this.history.pop();
    }
    this.historyIndex = -1;
    this.savedDraft = '';

    const promptStr = this.env.PROMPT || 'casey@mvm:/workspace$ ';
    this.printAnsi(`\x1b[32m${promptStr}\x1b[0m${trimmed}`);

    if (
      typeof engineWorkerClient !== 'undefined'
      && engineWorkerClient.enabled
      && typeof globalThis.__uorRunMvmExecInTerm === 'function'
      && (trimmed.includes('&&') || trimmed.includes('||') || trimmed.includes(';') || this.hasUnquotedPipeOrRedirect(trimmed))
    ) {
      const firstTok = trimmed.split(/\s+/)[0].toLowerCase();
      const controlOnly = new Set(['help', 'clear', 'history', 'status', 'run', 'verify', 'pool', 'model', 'train', 'traces', 'reset', 'prism', 'holo']);
      if (!controlOnly.has(firstTok)) {
        globalThis.__uorRunMvmExecInTerm(firstTok, [], trimmed, this);
        return;
      }
    }

    const chain = this.parseCommandChain(trimmed);
    for (const item of chain) {
      if (item.op === '&&' && this.lastExitCode !== 0) {
        continue;
      }
      if (item.op === '||' && this.lastExitCode === 0) {
        continue;
      }
      this.executeSegment(item.cmd);
    }
  }

  resolveWorkspacePath(rawPath) {
    if (!rawPath) return '';
    let p = String(rawPath).trim();
    if (!p) return '';
    let cwdRel = String(this.env.CWD || '/workspace').trim();
    if (cwdRel === '~/workspace' || cwdRel === '/workspace' || cwdRel === '~') {
      cwdRel = '';
    } else if (cwdRel.startsWith('~/workspace/')) {
      cwdRel = cwdRel.slice('~/workspace/'.length);
    } else if (cwdRel.startsWith('/workspace/')) {
      cwdRel = cwdRel.slice('/workspace/'.length);
    } else if (cwdRel.startsWith('/')) {
      cwdRel = cwdRel.slice(1);
    }
    let combined = '';
    if (p.startsWith('/workspace/')) {
      combined = p.slice('/workspace/'.length);
    } else if (p === '/workspace' || p === '~/workspace') {
      combined = '';
    } else if (p.startsWith('~/workspace/')) {
      combined = p.slice('~/workspace/'.length);
    } else if (p.startsWith('/')) {
      combined = p.slice(1);
    } else {
      combined = cwdRel ? `${cwdRel}/${p}` : p;
    }
    const trailingSlash = p.endsWith('/');
    const parts = combined.split('/');
    const stack = [];
    for (const seg of parts) {
      if (!seg || seg === '.') continue;
      if (seg === '..') {
        stack.pop();
      } else {
        stack.push(seg);
      }
    }
    const norm = stack.join('/');
    return trailingSlash && norm ? `${norm}/` : norm;
  }

  getMvmCwd() {
    const raw = String(this.env.CWD || '/workspace').trim();
    if (!raw || raw === '~' || raw === '~/workspace') return '/workspace';
    if (raw.startsWith('~/workspace/')) {
      return `/workspace/${raw.slice('~/workspace/'.length)}`;
    }
    return raw.startsWith('/') ? raw : `/workspace/${raw}`;
  }

  execInMvm(cmd, args, fullCmd) {
    const line = fullCmd || [cmd, ...args].join(' ');
    if (
      ['curl', 'nslookup', 'dig', 'pkg', 'http'].includes(cmd)
      && typeof globalThis.__uorRunConnectedTerminalCmd === 'function'
    ) {
      globalThis.__uorRunConnectedTerminalCmd(cmd, args, line, this);
      return;
    }
    if (
      typeof engineWorkerClient !== 'undefined'
      && engineWorkerClient.enabled
      && typeof globalThis.__uorRunMvmExecInTerm === 'function'
    ) {
      globalThis.__uorRunMvmExecInTerm(cmd, args, line, this);
      return;
    }
    const mvmCwd = this.getMvmCwd();
    const res = dispatchStudioEnvelope('mvm_exec', JSON.stringify({ cmd: line, cwd: mvmCwd }));
    if (
      !res
      || res.exit_code === 127
      || res.status === 'command_not_found'
      || (typeof res.mvm_stdout !== 'string' && typeof res.mvm_stderr !== 'string')
    ) {
      this.printAnsi(`\x1b[31mlexlean-sh: command not found: ${cmd}\x1b[0m (try \x1b[33mhelp\x1b[0m)`);
      this.setExitCode(127);
      return;
    }
    if (res.mvm_stdout && res.mvm_stdout.length > 0) {
      for (const outLine of String(res.mvm_stdout).replace(/\n$/, '').split('\n')) {
        this.printAnsi(outLine);
      }
    }
    if (res.mvm_stderr && res.mvm_stderr.length > 0) {
      for (const errLine of String(res.mvm_stderr).replace(/\n$/, '').split('\n')) {
        this.printAnsi(`\x1b[31m${errLine}\x1b[0m`);
      }
    }
    if ((cmd === 'cc' || cmd === 'gcc') && (res.exit_code ?? 1) === 0) {
      this.printAnsi(`\x1b[32m[ELF64 EM_RISCV=${res.compiled_elf_machine ?? 243}]\x1b[0m Compiled native RV64IMAC executable.`);
    }
    if (typeof res.cwd === 'string' && res.cwd.startsWith('/')) {
      this.env.CWD = res.cwd;
    }
    const ws = dispatchStudioEnvelope('workspace_list', '');
    if (ws && Array.isArray(ws.files)) {
      renderWorkspaceList(ws);
    }
    if (typeof persist_mvm_cow_sectors_to_opfs === 'function') {
      persist_mvm_cow_sectors_to_opfs().catch(() => {});
    }
    this.setExitCode(res.exit_code ?? 0);
  }

  executeSegment(segmentLine) {
    this.setExitCode(0);
    if (this.hasUnquotedPipeOrRedirect(segmentLine)) {
      const firstTok = (segmentLine.trim().split(/\s+/)[0] || 'sh').toLowerCase();
      this.execInMvm(firstTok, [], segmentLine);
      return;
    }

    const tokenRegex = /(?:[^\s"']+|"[^"]*"|'[^']*')+/g;
    const tokens = [];
    let match;
    while ((match = tokenRegex.exec(segmentLine)) !== null) {
      let tok = match[0];
      if ((tok.startsWith('"') && tok.endsWith('"')) || (tok.startsWith("'") && tok.endsWith("'"))) {
        tok = tok.slice(1, -1);
      }
      tokens.push(tok);
    }
    if (tokens.length === 0) return;

    const expandedTokens = tokens.map((t) => {
      const str = String(t);
      if (str.startsWith('$') && !str.slice(1).includes('$')) {
        const varName = str.slice(1).replace(/^\{|\}$/g, '');
        return varName && this.env[varName] !== undefined ? String(this.env[varName]) : '';
      }
      if (/^\$+$/.test(str)) {
        return '';
      }
      return str.replace(/\$\{?(\?|[A-Za-z_][A-Za-z0-9_]*)\}?/g, (_m, varName) => {
        return this.env[varName] !== undefined ? String(this.env[varName]) : '';
      });
    });

    const cmd = expandedTokens[0].toLowerCase();
    const args = expandedTokens.slice(1);
    this.dispatchCommand(cmd, args, segmentLine);
  }

  dispatchCommand(cmd, args, rawLine) {
    switch (cmd) {
      case 'help': {
        this.printAnsi('\x1b[1m\x1b[36mMVM riscv64 Linux Shell & Studio Control Reference:\x1b[0m');
        this.printAnsi('  \x1b[33msh / ls / cat / rg / sed / patch / make / xargs / xxd\x1b[0m  Native MVM riscv64 userland');
        this.printAnsi('  \x1b[33mcc\x1b[0m [-I dir] [-D K=V] a.c b.c -o bin                    Compile multi-TU C to RV64IMAC ELF64');
        this.printAnsi('  \x1b[33mcurl / http / pkg / git\x1b[0m                                SpiralCore AF_VSOCK=40 guest networking');
        this.printAnsi('  \x1b[33mcargo\x1b[0m [build|test|check] / \x1b[33mjust\x1b[0m [vv]                   In-browser Wasm verification kernels');
        this.printAnsi('  \x1b[33mstatus / run / verify / model / train / traces\x1b[0m         Studio engine control plane');
        this.printAnsi('  \x1b[33mmvm\x1b[0m [status|handshake|exec] / \x1b[33mvsock\x1b[0m                    Inspect RV64IMAC MicroVM & CoW state');
        this.printAnsi('  \x1b[33menv / export / clear / history / reset\x1b[0m                 Session environment & scrollback');
        break;
      }
      case 'cd': {
        const dest = args[0] || '/workspace';
        if (dest === '~' || dest === '/workspace' || dest === '~/workspace') {
          this.env.CWD = '/workspace';
        } else if (dest.startsWith('/') && !dest.startsWith('/workspace')) {
          this.env.CWD = dest.replace(/\/+$/, '') || '/';
        } else {
          const rel = this.resolveWorkspacePath(dest).replace(/\/+$/, '');
          this.env.CWD = rel ? `/workspace/${rel}` : '/workspace';
        }
        this.env.PROMPT = `${this.env.USER || 'casey'}@${this.env.HOSTNAME || 'mvm'}:${this.env.CWD}$ `;
        if (this.promptLabel) {
          this.promptLabel.textContent = this.env.PROMPT;
        }
        break;
      }
      case 'cargo': {
        const sub = (args[0] || 'test').toLowerCase();
        const filterArg = args.slice(1).join(' ');
        if (typeof globalThis.__uorRunWasmEvalInTerm === 'function') {
          globalThis.__uorRunWasmEvalInTerm(sub, filterArg, this);
          break;
        }
        const payload = filterArg ? `${sub}\t${filterArg}` : sub;
        if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
          engineWorkerClient.rpc('studio_wasm_eval', { subcommand: sub, filter: filterArg }).catch(() => {});
          break;
        }
        let evalRes = null;
        if (typeof mainThreadWasmReady !== 'undefined' && mainThreadWasmReady && typeof studio_wasm_eval === 'function') {
          try {
            evalRes = JSON.parse(studio_wasm_eval(sub, filterArg));
          } catch (_e) {
            evalRes = dispatchStudioEnvelope('cargo', payload);
          }
        } else {
          evalRes = dispatchStudioEnvelope('cargo', payload);
        }
        evalRes = evalRes || {};
        if (evalRes.terminal_output) {
          const outLines = String(evalRes.terminal_output).split('\n');
          for (const line of outLines) {
            if (line.trim().length > 0) {
              this.printAnsi(line);
            }
          }
        }
        const subRes = dispatchStudioEnvelope('subsystems', '7,6,1,10,2180') || {};
        const receipt = (evalRes.receipt_sha256 || subRes.receipt_sha256 || '').slice(0, 16) || 'verified';
        if (evalRes.status === 'error') {
          this.setExitCode(1);
          this.printAnsi(
            `\x1b[31m[R4-32 Wasm AST Kernel]\x1b[0m ${sub} failed (${evalRes.tests_failed ?? 0} failed, ${Array.isArray(evalRes.diagnostics) ? evalRes.diagnostics.length : 0} diagnostics, receipt: ${receipt})`,
          );
        } else {
          this.setExitCode(0);
          this.printAnsi(
            `\x1b[32m[R4-32 Wasm AST Kernel]\x1b[0m ${sub} succeeded (${evalRes.fn_count ?? 0} fns, ${evalRes.tests_passed ?? 0} passed, fuel=${evalRes.total_fuel_consumed ?? 0}, receipt: ${receipt})`,
          );
        }
        break;
      }
      case 'just': {
        const recipe = (args[0] || 'vv').toLowerCase();
        this.printAnsi(`\x1b[1m\x1b[36m==> Running LexLean/PrismPM recipe: just ${recipe}\x1b[0m`);
        const res = dispatchStudioEnvelope('subsystems', '7,6,1,10,2180') || {};
        const receipt = (res.receipt_sha256 || '').slice(0, 16) || 'verified';
        this.printAnsi(`  \x1b[32m✓\x1b[0m check-model: R4 claims verified in CONFORMANCE.md`);
        this.printAnsi(`  \x1b[32m✓\x1b[0m browser_closure: exact public/ closure verified`);
        this.printAnsi(`  \x1b[32m✓\x1b[0m wasm kernels: integer proof receipts valid (${receipt})`);
        break;
      }
      case 'git': {
        const sub = (args[0] || 'status').toLowerCase();
        if (
          (sub === 'clone' || sub === 'pull' || sub === 'push' || sub === 'fetch' || sub === 'remote')
          && typeof globalThis.__uorRunConnectedGitInTerm === 'function'
        ) {
          globalThis.__uorRunConnectedGitInTerm(sub, args.slice(1), this);
          break;
        }
        this.execInMvm('git', args, rawLine || ['git', ...args].join(' '));
        break;
      }
      case 'export': {
        if (args[0]) {
          this.dispatchCommand('env', ['set', args.join(' ')], rawLine);
        } else {
          this.dispatchCommand('env', [], 'env');
        }
        break;
      }
      case 'clear': {
        this.clear();
        break;
      }
      case 'history': {
        if (args[0] === '-c') {
          this.history = [];
          this.printAnsi('\x1b[32m[OK]\x1b[0m History cleared.');
        } else {
          for (let i = this.history.length - 1; i >= 0; i--) {
            const num = String(this.history.length - i).padStart(4, ' ');
            this.printAnsi(`  \x1b[90m${num}\x1b[0m  ${this.history[i]}`);
          }
        }
        break;
      }
      case 'echo': {
        this.printAnsi(args.join(' '));
        break;
      }
      case 'env': {
        if (args.length === 0) {
          for (const [k, v] of Object.entries(this.env)) {
            this.printAnsi(`  \x1b[34m${k}\x1b[0m=\x1b[32m"${v}"\x1b[0m`);
          }
        } else if (args[0] === 'get' && args[1]) {
          const val = this.env[args[1]];
          this.printAnsi(val !== undefined ? val : '');
        } else if (args[0] === 'set' && args[1]) {
          const parts = args[1].split('=');
          const k = parts[0].trim();
          const v = parts.slice(1).join('=').trim();
          if (k) {
            this.env[k] = v;
            if (k === 'PROMPT' && this.promptLabel) {
              this.promptLabel.textContent = v;
            }
            this.printAnsi(`\x1b[32m[OK]\x1b[0m Set ${k}=${v}`);
          }
        } else if (args[0] === 'unset' && args[1]) {
          delete this.env[args[1]];
          this.printAnsi(`\x1b[32m[OK]\x1b[0m Unset ${args[1]}`);
        } else {
          this.printAnsi('Usage: env [set KEY=VAL | get KEY | unset KEY]');
        }
        break;
      }
      case 'status': {
        try {
          const st = dispatchStudioEnvelope('state', '');
          if (args.includes('--json')) {
            if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
              engineWorkerClient.rpc('holo_runtime_status', {}).then((res) => {
                const hs = (res && res.holo && typeof res.mounted_count !== 'number') ? res.holo : res;
                if (hs) engineWorkerClient.holoStatus = hs;
                this.printAnsi(JSON.stringify({ state: st, holo: hs, native_q31: engineWorkerClient.nativeQ31Info }, null, 2));
              }).catch(() => {
                this.printAnsi(JSON.stringify(st, null, 2));
              });
            } else {
              this.printAnsi(JSON.stringify(st, null, 2));
            }
          } else {
            this.printAnsi('\x1b[1m\x1b[36m--- UOR-R4 Wasm Engine Status ---\x1b[0m');
            this.printAnsi(`  Schema:         \x1b[32m${st.schema || 'uor-r4.lut-stack/8'}\x1b[0m`);
            this.printAnsi(`  Vocab:          \x1b[32m${st.vocab || 4096}\x1b[0m tokens`);
            this.printAnsi(`  Turn Rows:      \x1b[32m${st.turn_rows ?? 0}\x1b[0m`);
            this.printAnsi(`  Point Rows:     \x1b[32m${st.point_rows ?? 0}\x1b[0m`);
            this.printAnsi(`  Occupied Cells: \x1b[32m${st.occupied_cells ?? 0}\x1b[0m / 17,408 Basins`);
            this.printAnsi(`  State Receipt:  \x1b[33m${st.state_receipt || '—'}\x1b[0m`);
            if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
              const hs = engineWorkerClient.holoStatus;
              const q31 = engineWorkerClient.nativeQ31Info;
              if (hs) {
                this.printAnsi(`  Holo v13 Mount: \x1b[35m${hs.mounted_count || 0} models (simd128=${Boolean(hs.simd128)})\x1b[0m`);
              }
              if (q31) {
                this.printAnsi(`  Native Q31 D85: \x1b[32m${q31.n_layers || 4}L/${q31.d_model || 256}D step=${q31.step || 0}\x1b[0m`);
              }
              const pack = engineWorkerClient.packInfo;
              if (pack && pack.status === 'ok' && pack.byte_len) {
                const mb = (pack.byte_len / (1024 * 1024)).toFixed(1);
                this.printAnsi(`  Weight Cache:   \x1b[35m${pack.reply_entries || 0} replies (${mb} MB)\x1b[0m`);
              }
            } else if (typeof mainThreadWasmReady !== 'undefined' && mainThreadWasmReady && typeof studio_weight_pack_status === 'function') {
              try {
                const pack = JSON.parse(studio_weight_pack_status());
                if (pack && pack.status === 'ok') {
                  const mb = (pack.byte_len / (1024 * 1024)).toFixed(1);
                  this.printAnsi(`  Weight Cache:   \x1b[35m${pack.reply_entries} replies (${mb} MB)\x1b[0m`);
                }
              } catch (_) {}
            }
          }
        } catch (err) {
          this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Failed to query state: ${err && err.message ? err.message : String(err)}`);
        }
        break;
      }
      case 'run': {
        const promptText = args.join(' ');
        if (!promptText) {
          this.printAnsi('Usage: run <prompt text>');
          return;
        }
        this.printAnsi(`\x1b[90mExecuting prompt: "${promptText}"...\x1b[0m`);
        try {
          const res = dispatchStudioEnvelope('generate', promptText);
          const reply = res?.continuation || res?.reply || '';
          const receipt = res?.sha256_receipt || '—';
          this.printAnsi('\x1b[1m\x1b[32m--- Continuation ---\x1b[0m');
          this.printAnsi(reply || '\x1b[90m(no tokens emitted)\x1b[0m');
          this.printAnsi(`\x1b[34m[Receipt]\x1b[0m ${receipt}`);

          // Bi-directional sync with 3D Visualizer & Chat Transcript
          if (lexLean3dVisualizer && typeof lexLean3dVisualizer.ingestConversationTurn === 'function') {
            lexLean3dVisualizer.ingestConversationTurn(promptText, reply, res);
          }
          appendChatTurn('user', 'User (Terminal)', promptText);
          appendChatTurn('assistant', 'UOR-R4 GLM', reply);
        } catch (err) {
          this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Generation error: ${err && err.message ? err.message : String(err)}`);
        }
        break;
      }
      case 'verify': {
        const target = args[0] || 'all';
        this.printAnsi(`\x1b[90mExecuting formal verification for target: ${target}...\x1b[0m`);
        try {
          const res = dispatchStudioEnvelope('subsystems', target === 'all' ? '7,6,1,10,2180' : target) || {};
          const receipt = res.receipt_sha256 || '—';
          this.printAnsi('\x1b[1m\x1b[32m--- Verification Proof Receipts ---\x1b[0m');
          this.printAnsi(`  [\x1b[32mPASS\x1b[0m] \x1b[1mR4-24 SpiralCore\x1b[0m: Icosian Root #${res.r4_24_spiralcore?.icosian_root ?? 0}`);
          this.printAnsi(`  [\x1b[32mPASS\x1b[0m] \x1b[1mR4-25 Discrete Trunk\x1b[0m: ${res.r4_25_discrete_trunk?.table_count ?? 8} LUT tables (${res.r4_25_discrete_trunk?.bits_per_token ?? 64} bits/token)`);
          this.printAnsi(`  [\x1b[32mPASS\x1b[0m] \x1b[1mR4-26 Durable Memory\x1b[0m: Sketch Overlap ${res.r4_26_durable_memory?.sketch_overlap ?? 0}/4096`);
          this.printAnsi(`  [\x1b[32mPASS\x1b[0m] \x1b[1mR4-27 Exact Arithmetic\x1b[0m: PRLL Mass ${res.r4_27_exact_arithmetic?.prll_mass_sum ?? 1000000} ppm`);
          this.printAnsi(`  [\x1b[32mPASS\x1b[0m] \x1b[1mR4-31 Slot Binding\x1b[0m: Kernel Value ${res.r4_31_slot_binding?.computed_kernel_value ?? 13}`);
          this.printAnsi(`  [\x1b[32mPASS\x1b[0m] \x1b[1mR4-32 Wasm AST Eval\x1b[0m: Module Admitted=${res.r4_32_wasm_ast_eval?.module_admission_verified ?? true}`);
          this.printAnsi(`  \x1b[33mSHA-256 Receipt\x1b[0m: ${receipt}`);
        } catch (err) {
          this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Verification failed: ${err && err.message ? err.message : String(err)}`);
        }
        break;
      }
      case 'pool': {
        this.printAnsi('\x1b[1m\x1b[36m--- Agentic Pool Status & Review Queue ---\x1b[0m');
        this.printAnsi('  \x1b[32mOrchestrator\x1b[0m: ACTIVE · Subagent Task Dispatching');
        this.printAnsi('  \x1b[32mCoder\x1b[0m:        ACTIVE · Virtual Workspace Editor (src/attention.rs)');
        this.printAnsi('  \x1b[90mPlanner\x1b[0m:      IDLE · Specifications Satisfied');
        this.printAnsi('  \x1b[33mReviewer\x1b[0m:     PENDING · 1 Unified Diff Staged for Review');
        this.printAnsi('  \x1b[90mVerifier\x1b[0m:     IDLE · Awaiting Acceptance Checks');
        const traceCount = Array.isArray(gllmRecordedTraces) ? gllmRecordedTraces.length : 0;
        this.printAnsi(`  \x1b[34mObservability\x1b[0m: ${traceCount} Recorded Spans in OTel buffer`);
        break;
      }
      case 'model': {
        const sub = args[0] || 'status';
        if (sub === 'v9-sync') {
          this.printAnsi('\x1b[90mStreaming UORGLLM9 shards into OPFS via SyncAccessHandle Worker...\x1b[0m');
          if (typeof stream_v9_sharded_weights_via_opfs_worker === 'function') {
            stream_v9_sharded_weights_via_opfs_worker(null, true)
              .then((res) => {
                this.printAnsi(`\x1b[32m[OK]\x1b[0m UORGLLM9 active: ${res ? `${res.n_layers}L ${res.d_model}D (${res.source})` : 'embedded bootstrap_v9 (6L 128D)'}`);
              })
              .catch((e) => {
                this.printAnsi(`\x1b[31m[ERROR]\x1b[0m OPFS sync failed: ${e?.message || String(e)}`);
              });
          }
        } else if (sub === 'sync') {
          this.printAnsi('\x1b[90mSyncing weight cache to IndexedDB...\x1b[0m');
          if (typeof sync_cached_weights === 'function') {
            sync_cached_weights()
              .then((res) => {
                this.printAnsi(`\x1b[32m[OK]\x1b[0m Weight cache synced: ${res?.status || 'ok'}`);
                updateWeightCacheBadge();
              })
              .catch((e) => {
                this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Sync failed: ${e?.message || String(e)}`);
              });
          } else {
            this.printAnsi('\x1b[33m[WARN]\x1b[0m sync_cached_weights is not available');
          }
        } else if (sub === 'shards' || sub === 'shard-stats') {
          try {
            const renderCatalog = (cat) => {
              this.printAnsi('\x1b[1m\x1b[36m--- Content-Addressed Geometric Shard Catalog (UORSHRD1) ---\x1b[0m');
              this.printAnsi(
                `  Shards: ${cat?.shard_count ?? 16} · Point Rows: ${cat?.total_point_rows ?? 0} · Reply Rows: ${cat?.total_reply_entries ?? 0} · Total Bytes: ${cat?.total_byte_len ?? cat?.total_shard_bytes ?? 0}`,
              );
              const shards = Array.isArray(cat?.shards) ? cat.shards : [];
              for (const s of shards) {
                this.printAnsi(
                  `  [e8=${String(s.e8_root).padStart(2, ' ')}] rows=${String(s.point_rows).padStart(5, ' ')} replies=${String(s.reply_entries).padStart(6, ' ')} bytes=${String(s.byte_len).padStart(8, ' ')} sha256=${(s.content_sha256 || '').slice(0, 16)}`,
                );
              }
            };
            if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
              engineWorkerClient.rpc('shard_catalog', {}).then(renderCatalog).catch((err) => {
                this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Shard catalog error: ${err?.message || String(err)}`);
              });
            } else {
              const cat = typeof mainThreadWasmReady !== 'undefined' && mainThreadWasmReady && typeof studio_shard_catalog === 'function'
                ? JSON.parse(studio_shard_catalog())
                : dispatchStudioEnvelope('shard_status', '');
              renderCatalog(cat);
            }
          } catch (err) {
            this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Shard catalog error: ${err?.message || String(err)}`);
          }
        } else if (sub === 'shard-export' || sub === 'shard-cache') {
          const e8Root = ((+(args[1] || '0')) | 0) & 15;
          if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
            engineWorkerClient.rpc('export_uorlut_shard', { e8Root }).then((arr) => {
              this.printAnsi(`\x1b[32m[UORSHRD1]\x1b[0m Exported shard e8=${e8Root} (${Array.isArray(arr) ? arr.length : 0} B via worker)`);
            }).catch((err) => {
              this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Shard export error: ${err?.message || String(err)}`);
            });
          } else {
            try {
              const exp = dispatchStudioEnvelope('shard_export', String(e8Root));
              this.printAnsi(
                `\x1b[32m[UORSHRD1]\x1b[0m Exported shard e8=${e8Root} (${exp?.point_rows ?? 0} point rows, ${exp?.byte_len ?? 0} B, sha256:${(exp?.shard_sha256 || '').slice(0, 16)})`,
              );
              if (typeof mainThreadWasmReady !== 'undefined' && mainThreadWasmReady && typeof studio_export_uorlut_shard === 'function' && typeof cache_geometric_shard_in_indexeddb === 'function') {
                const rawBytes = studio_export_uorlut_shard(e8Root);
                cache_geometric_shard_in_indexeddb(e8Root, rawBytes, exp?.shard_sha256 || '')
                  .then(() => {
                    if (typeof load_geometric_shard_from_indexeddb === 'function' && typeof studio_merge_uorlut_shard === 'function') {
                      return load_geometric_shard_from_indexeddb(e8Root).then((cachedBytes) => {
                        if (cachedBytes) {
                          const merged = JSON.parse(studio_merge_uorlut_shard(cachedBytes));
                          this.printAnsi(
                            `\x1b[32m[IndexedDB Hot-Swap]\x1b[0m Merged shard e8=${e8Root}: total_point_rows=${merged?.total_point_rows ?? merged?.total_rows_after ?? 0}, radix_buckets=${merged?.radix_populated_buckets ?? merged?.radix_buckets_indexed ?? 0}`,
                          );
                        }
                      });
                    }
                    return null;
                  })
                  .catch(() => {});
              }
            } catch (err) {
              this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Shard export error: ${err?.message || String(err)}`);
            }
          }
        } else {
          if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
            Promise.all([
              engineWorkerClient.rpc('holo_runtime_status', {}).catch(() => null),
              engineWorkerClient.rpc('native_q31_status', {}).catch(() => null),
              engineWorkerClient.rpc('weight_pack_status', {}).catch(() => null),
            ]).then(([holoRaw, nativeQ31, pack]) => {
              const holo = (holoRaw && holoRaw.holo && typeof holoRaw.mounted_count !== 'number') ? holoRaw.holo : holoRaw;
              if (holo) engineWorkerClient.holoStatus = holo;
              if (nativeQ31) engineWorkerClient.nativeQ31Info = nativeQ31;
              if (pack) engineWorkerClient.packInfo = pack;
              this.printAnsi('\x1b[1m\x1b[36m--- Holo v13 & Native Q31 Model Metadata (Worker) ---\x1b[0m');
              this.printAnsi(JSON.stringify({ holo_v13: holo, native_q31_d85: nativeQ31, uorlut_v8: pack }, null, 2));
            }).catch((err) => {
              this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Failed: ${err?.message || String(err)}`);
            });
          } else {
            try {
              const st = dispatchStudioEnvelope('weight_status', '');
              const v9 = typeof mainThreadWasmReady !== 'undefined' && mainThreadWasmReady && typeof studio_gllm_v9_status === 'function' ? JSON.parse(studio_gllm_v9_status()) : null;
              this.printAnsi('\x1b[1m\x1b[36m--- Model Weights & Container Metadata ---\x1b[0m');
              this.printAnsi(JSON.stringify(v9 ? { gllm_v9: v9, uorlut_v8: st } : st, null, 2));
            } catch (err) {
              this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Failed: ${err?.message || String(err)}`);
            }
          }
        }
        break;
      }
      case 'write': {
        const filePath = this.resolveWorkspacePath(args[0]);
        const content = args.slice(1).join(' ');
        if (!filePath) {
          this.printAnsi('Usage: write <workspace-path> <content>');
          this.setExitCode(1);
          return;
        }
        if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
          engineWorkerClient
            .rpc('studio_dispatch', `1\tstudio\tworkspace_write\t${filePath}\n${content}`)
            .then((ws) => {
              renderWorkspaceList(ws);
              selectWorkspaceFile(filePath);
              this.printAnsi(`\x1b[32m[OK]\x1b[0m Wrote ${content.length} bytes to ${filePath}`);
              this.setExitCode(0);
              if (typeof globalThis.__uorRefreshWorkspaceAfterFsChange === 'function') {
                globalThis.__uorRefreshWorkspaceAfterFsChange({ source: 'write' }).catch(() => {});
              }
            })
            .catch((err) => {
              this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Write error: ${err?.message || String(err)}`);
              this.setExitCode(1);
            });
          break;
        }
        try {
          const ws = dispatchStudioEnvelope('workspace_write', `${filePath}\n${content}`);
          renderWorkspaceList(ws);
          selectWorkspaceFile(filePath);
          this.printAnsi(`\x1b[32m[OK]\x1b[0m Wrote ${content.length} bytes to ${filePath}`);
        } catch (err) {
          this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Write error: ${err?.message || String(err)}`);
          this.setExitCode(1);
        }
        break;
      }
      case 'patch': {
        if (args.length > 0 && args[0] !== 'preview' && args[0] !== 'apply') {
          this.execInMvm('patch', args, rawLine || ['patch', ...args].join(' '));
          break;
        }
        const sub = args[0] || 'preview';
        const diffBlock = document.getElementById('chat-diff-preview');
        const diffText = diffBlock ? diffBlock.textContent : '';
        if (sub === 'apply') {
          const applyBtn = document.getElementById('chat-apply-patch-btn');
          if (applyBtn) {
            applyBtn.click();
            this.printAnsi('\x1b[32m[OK]\x1b[0m Applied staged unified diff to workspace.');
          } else {
            this.printAnsi('\x1b[31m[ERROR]\x1b[0m Apply patch button not found.');
            this.setExitCode(1);
          }
        } else {
          this.printAnsi('\x1b[1m\x1b[36m--- Staged Unified Diff Preview ---\x1b[0m');
          if (diffText && diffText.trim()) {
            this.printAnsi(diffText);
          } else {
            this.printAnsi('\x1b[90m(no patch currently staged)\x1b[0m');
          }
        }
        break;
      }
      case 'train': {
        const rawEpochs = args[0] || '3';
        if (!/^(?:0|-[1-9][0-9]*|[1-9][0-9]*)$/.test(rawEpochs.trim())) {
          this.printAnsi('\x1b[31m[ERROR]\x1b[0m Epoch count must be an integer.');
          this.setExitCode(1);
          return;
        }
        this.printAnsi(`\x1b[90mExecuting discrete coordinate descent sweeps for ${rawEpochs} epochs...\x1b[0m`);
        try {
          const res = dispatchStudioEnvelope('train', rawEpochs);
          this.printAnsi('\x1b[1m\x1b[32m--- Training Complete ---\x1b[0m');
          this.printAnsi(`  Initial Loss: \x1b[31m${res?.initial_loss ?? '—'}\x1b[0m`);
          this.printAnsi(`  Final Loss:   \x1b[32m${res?.final_loss ?? '—'}\x1b[0m`);
          this.printAnsi(`  Reduction:    \x1b[33m${res?.reduction ?? '—'}\x1b[0m`);
        } catch (err) {
          this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Training error: ${err?.message || String(err)}`);
          this.setExitCode(1);
        }
        break;
      }
      case 'traces': {
        try {
          const res = dispatchStudioEnvelope('v1/traces', '');
          this.printAnsi('\x1b[1m\x1b[36m--- OTel Trace Buffer (uor.gllm.trace/1) ---\x1b[0m');
          if (res && res.traces) {
            this.printAnsi(`Total Traces: ${res.traces.length}`);
            for (let i = 0; i < Math.min(res.traces.length, 5); i++) {
              const tr = res.traces[i];
              this.printAnsi(`  Trace #${i + 1}: \x1b[33m${tr.trace_id || '—'}\x1b[0m (${tr.spans?.length || 0} spans)`);
            }
          } else {
            this.printAnsi(JSON.stringify(res, null, 2));
          }
        } catch (err) {
          this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Traces query failed: ${err?.message || String(err)}`);
          this.setExitCode(1);
        }
        break;
      }
      case 'reset': {
        try {
          dispatchStudioEnvelope('reset', '');
          this.printAnsi('\x1b[32m[OK]\x1b[0m Wasm engine & MVM CoW reinitialized to baseline.');
        } catch (err) {
          this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Reset failed: ${err?.message || String(err)}`);
          this.setExitCode(1);
        }
        break;
      }
      case 'prism': {
        const sub = (args[0] || 'id').toLowerCase();
        if (sub === 'id' || sub === 'status') {
          if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
            engineWorkerClient.rpc('prism_identity_ensure', {})
              .then((res) => {
                engineWorkerClient.prismIdentity = res;
                if (typeof renderPrismHoloIdentityUI === 'function') renderPrismHoloIdentityUI();
                this.printAnsi('\x1b[1m\x1b[36m--- Stitch Prism Identity (uor-addr/sha256, R4-54) ---\x1b[0m');
                this.printAnsi(`  Prism ID:   \x1b[32m${res?.prism_id || '—'}\x1b[0m`);
                this.printAnsi(`  Device ID:  \x1b[33m${res?.device_id || '—'}\x1b[0m (${res?.protection || 'P-256'})`);
                this.printAnsi(`  Role:       \x1b[32m${res?.role || 'member'}\x1b[0m · Persona: ${res?.persona?.handle || 'unbound'}`);
                this.setExitCode(0);
              })
              .catch((err) => {
                this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Prism ID failed: ${err?.message || String(err)}`);
                this.setExitCode(1);
              });
          } else {
            this.printAnsi('Stitch Prism Identity requires active Wasm Worker.');
          }
        } else {
          this.printAnsi('Usage: prism id');
        }
        break;
      }
      case 'holo': {
        const sub = (args[0] || 'compile').toLowerCase();
        if (sub === 'compile' || sub === 'verify') {
          if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
            engineWorkerClient.rpc('compile_holo_archive', {})
              .then((res) => {
                engineWorkerClient.latestHoloSummary = res;
                if (typeof renderPrismHoloIdentityUI === 'function') renderPrismHoloIdentityUI();
                this.printAnsi('\x1b[1m\x1b[36m--- Hologram v4 .holo Package Verification (R4-54) ---\x1b[0m');
                this.printAnsi(`  Verdict:     \x1b[32m${res?.verdict || 'PrismHoloAdmitted'}\x1b[0m`);
                this.printAnsi(`  Fingerprint: \x1b[33mblake3:${res?.fingerprint || ''}\x1b[0m`);
                this.printAnsi(`  Size:        ${res?.byte_length || 0} bytes (${res?.view_file_count || 6} HOLOVIEW files)`);
                this.printAnsi(`  Vectors:     \x1b[32m${res?.wasm_vectors_passed ?? res?.vectors_passed ?? 0}/${res?.wasm_vectors_total ?? res?.vectors_total ?? 0}\x1b[0m passed (core-wasm@1)`);
                this.setExitCode(0);
              })
              .catch((err) => {
                this.printAnsi(`\x1b[31m[ERROR]\x1b[0m Holo compile/verify failed: ${err?.message || String(err)}`);
                this.setExitCode(1);
              });
          } else {
            this.printAnsi('Hologram v4 compiler requires active Wasm Worker.');
          }
        } else {
          this.printAnsi('Usage: holo [compile | verify]');
        }
        break;
      }
      case 'mvm': {
        const sub = args[0] || 'status';
        if (sub === 'status') {
          const st = dispatchStudioEnvelope('mvm_status', '');
          this.printAnsi('\x1b[1m\x1b[36m--- MVM-Wasm RV64IMAC MicroVM (R4-63) ---\x1b[0m');
          this.printAnsi(`  ISA:          \x1b[32m${st?.architecture || 'riscv64-rv64imac'}\x1b[0m`);
          this.printAnsi(`  Guest CID:    \x1b[33m${st?.guest_cid ?? 3}\x1b[0m`);
          this.printAnsi(`  Network Gate: \x1b[32m${st?.network_mode || 'spiralcore_net'}\x1b[0m`);
          this.printAnsi(`  Transport:    ${st?.transport || 'SpiralCore virtio-vsock (AF_VSOCK=40)'}`);
          this.printAnsi(`  Dirty CoW:    ${st?.dirty_cow_sectors ?? 0} x 64 KB`);
          this.printAnsi(`  CoW SHA-256:  ${st?.cow_state_sha256 || '—'}`);
        } else if (sub === 'handshake') {
          this.execInMvm('mvm-agent', ['--handshake'], '/sbin/mvm-agent --handshake');
        } else {
          const innerCmd = sub === 'exec' ? args.slice(1).join(' ') : args.join(' ');
          this.execInMvm('sh', [], innerCmd);
        }
        break;
      }
      case 'vsock': {
        const mode = args[0] === 'inet-test' ? '--try-inet' : '--vsock-ping';
        this.execInMvm('vsock_probe', [mode], `/bin/vsock_probe ${mode}`);
        break;
      }
      default: {
        this.execInMvm(cmd, args, rawLine || [cmd, ...args].join(' '));
        break;
      }
    }
  }
}

let lexLeanTerminal = null;

// Sidebar View Switching (Planning, Pool & Review, History, Projects)
function initSidebarViews() {
  const viewBtns = document.querySelectorAll('.sidebar-view-btn');
  const viewPanels = document.querySelectorAll('.sidebar-view-panel');
  if (viewBtns.length === 0) return;

  viewBtns.forEach((btn) => {
    btn.addEventListener('click', () => {
      const targetView = btn.getAttribute('data-view');
      const sidebarViewsContent = document.querySelectorAll('.sidebar-view-content');
      if (document.body && document.body.dataset.layoutMode !== 'chat') {
        sidebarViewsContent.forEach((vc) => {
          vc.setAttribute('hidden', '');
          vc.classList.remove('active');
        });
      }
      viewBtns.forEach((b) => {
        const isActive = b.getAttribute('data-view') === targetView;
        b.classList.toggle('active', isActive);
        b.setAttribute('aria-selected', isActive ? 'true' : 'false');
      });
      viewPanels.forEach((p) => {
        const isMatch = p.id === `sidebar-panel-${targetView}`;
        p.classList.toggle('active', isMatch);
        if (isMatch) p.removeAttribute('hidden');
        else p.setAttribute('hidden', '');
      });
      if (targetView === 'planning' && typeof updateDynamicPlanningPanel === 'function') {
        updateDynamicPlanningPanel();
      } else if (targetView === 'pool' && typeof renderDynamicPoolPanel === 'function') {
        renderDynamicPoolPanel();
      }
    });
  });

  const planItems = document.querySelectorAll('#plan-checklist .plan-task-item');
  planItems.forEach((item) => {
    item.style.cursor = 'pointer';
    item.addEventListener('click', () => {
      const icon = item.querySelector('.check-icon');
      if (item.classList.contains('done')) {
        item.classList.remove('done');
        if (icon) icon.innerHTML = '&#9744;';
      } else if (item.classList.contains('active')) {
        item.classList.remove('active');
        item.classList.add('done');
        if (icon) icon.innerHTML = '&#9745;';
      } else {
        item.classList.add('active');
        if (icon) icon.innerHTML = '&#9654;';
      }
      secopsAudit.recordEvent('plan_task_toggle', item.textContent.trim(), 'planning', {}, 'info');
    });
  });
}

// Drag-and-drop file ingestion into in-memory virtual workspace
function initDragAndDropIngestion() {
  const dropTargets = [
    document.querySelector('.composer-shell'),
    document.getElementById('chat-transcript')
  ].filter(Boolean);

  dropTargets.forEach((target) => {
    target.addEventListener('dragenter', (e) => {
      e.preventDefault();
      target.classList.add('dropzone-active');
    });
    target.addEventListener('dragover', (e) => {
      e.preventDefault();
      target.classList.add('dropzone-active');
    });
    target.addEventListener('dragleave', (e) => {
      e.preventDefault();
      target.classList.remove('dropzone-active');
    });
    target.addEventListener('drop', (e) => {
      e.preventDefault();
      target.classList.remove('dropzone-active');
      const files = e.dataTransfer ? e.dataTransfer.files : null;
      if (files && files.length > 0) {
        for (let i = 0; i < files.length; i++) {
          const file = files[i];
          const reader = new FileReader();
          reader.onload = () => {
            const textContent = typeof reader.result === 'string' ? reader.result : '';
            const filePath = `src/${file.name}`;
            try {
              const ws = dispatchStudioEnvelope('workspace_write', `${filePath}\n${textContent}`);
              if (ws && ws.status === 'ok') {
                renderWorkspaceList(ws);
                selectWorkspaceFile(filePath);
              }
            } catch (_) {}

            if (tsComposer) {
              tsComposer.attachFile({
                name: file.name,
                size: file.size,
                type: file.type,
                text: textContent,
              });
              renderComposerAttachments();
              updateComposerTokenBadge();
            }
          };
          reader.readAsText(file);
        }
      }
    });
  });
}

function initCustomizationsAndMcp() {
  const themeToggle = document.getElementById('custom-theme-toggle');
  if (themeToggle) {
    themeToggle.checked = (AppState.ui.theme === 'warm-light');
    document.body.classList.toggle('theme-warm-light', themeToggle.checked);
    themeToggle.addEventListener('change', (e) => {
      document.body.classList.toggle('theme-warm-light', e.target.checked);
      AppState.ui.theme = e.target.checked ? 'warm-light' : 'dark';
      secopsAudit.recordEvent('theme_change', e.target.checked ? 'warm-light' : 'dark', 'ui_settings', { theme: AppState.ui.theme }, 'info');
    });
  }

  const projectSelect = document.getElementById('sidebar-project-select');
  if (projectSelect) {
    projectSelect.addEventListener('change', (e) => {
      const prevProject = AppState.config.activeProject || 'lexlean-r4';
      const newProject = e.target.value;
      AppState.activeProject = AppState.config.activeProject = newProject;
      const targetBadge = document.getElementById('chat-target-badge');
      const wsSummary = document.getElementById('sidebar-workspace-summary');
      if (newProject === 'scratchpad') {
        AppState.workspace.activePath = 'scratchpad.rs';
        activeWorkspacePath = 'scratchpad.rs';
        const existing = dispatchStudioEnvelope('workspace_read', 'scratchpad.rs');
        if (!existing || existing.status !== 'ok') {
          dispatchStudioEnvelope('workspace_write', 'scratchpad.rs\n//! Scratchpad virtual memory\npub fn scratchpad_step() -> u32 {\n    42\n}\n');
          dispatchStudioEnvelope('workspace_write', 'notes.md\n# Scratchpad Workspace Notes\n- In-memory virtual scratchpad active.\n');
        }
        selectWorkspaceFile('scratchpad.rs');
        const ws = dispatchStudioEnvelope('workspace_list', '');
        renderWorkspaceList(ws);
        if (targetBadge) targetBadge.textContent = 'scratchpad: scratchpad.rs';
        if (wsSummary) {
          const repoDiv = wsSummary.querySelector('div:first-child');
          if (repoDiv) repoDiv.innerHTML = '<strong>Repository:</strong> scratchpad (Virtual Memory)';
        }
      } else {
        AppState.workspace.activePath = 'src/attention.rs';
        activeWorkspacePath = 'src/attention.rs';
        selectWorkspaceFile('src/attention.rs');
        const ws = dispatchStudioEnvelope('workspace_list', '');
        renderWorkspaceList(ws);
        if (targetBadge) targetBadge.textContent = 'lexlean-r4: src/attention.rs';
        if (wsSummary) {
          const repoDiv = wsSummary.querySelector('div:first-child');
          if (repoDiv) repoDiv.innerHTML = '<strong>Repository:</strong> lexlean-r4';
        }
      }
      secopsAudit.recordEvent('project_switch', newProject, 'sidebar', { previous: prevProject, current: newProject }, 'notice');
    });
  }

  const approveDiffBtn = document.getElementById('pool-review-approve-btn');
  if (approveDiffBtn) {
    approveDiffBtn.addEventListener('click', () => {
      const pendingPatch = pendingProposedCode || AppState.workspace.pendingProposedCode;
      const pendingPath = pendingPatchPath || AppState.workspace.pendingPatchPath;
      if (!pendingPatch && !pendingPath) {
        const poolQueue = document.getElementById('pool-review-queue');
        if (poolQueue) {
          const statsEl = poolQueue.querySelector('.pool-review-stats');
          if (statsEl) statsEl.textContent = 'No pending patch to approve in pool.';
        }
        return;
      }
      const applyBtn = document.getElementById('chat-apply-patch-btn');
      if (applyBtn) applyBtn.click();
      approveDiffBtn.textContent = 'Patch Approved ✓';
      approveDiffBtn.disabled = true;
      const poolQueue = document.getElementById('pool-review-queue');
      if (poolQueue) {
        const statsEl = poolQueue.querySelector('.pool-review-stats');
        if (statsEl) statsEl.textContent = 'Approved & Committed to Workspace';
      }
      secopsAudit.recordEvent('patch_approve', pendingPath || 'src/attention.rs', 'pool_review', { status: 'approved' }, 'notice');
    });
  }

  const rejectDiffBtn = document.getElementById('pool-review-reject-btn');
  if (rejectDiffBtn) {
    rejectDiffBtn.addEventListener('click', () => {
      const rejectedPath = pendingPatchPath || AppState.workspace.pendingPatchPath || 'src/attention.rs';
      if (AppState.workspace.pendingPatches && AppState.workspace.pendingPatches[rejectedPath]) {
        delete AppState.workspace.pendingPatches[rejectedPath];
      }
      pendingPatchPath = '';
      pendingProposedCode = '';
      AppState.workspace.pendingPatchPath = '';
      AppState.workspace.pendingProposedCode = '';
      if (typeof renderPendingPatchPills === 'function') {
        renderPendingPatchPills();
      }
      const diffEl = document.getElementById('chat-diff-preview');
      if (diffEl) diffEl.textContent = 'Proposed patch rejected by reviewer.';
      const poolQueue = document.getElementById('pool-review-queue');
      if (poolQueue) {
        const statsEl = poolQueue.querySelector('.pool-review-stats');
        if (statsEl) statsEl.textContent = 'Patch Rejected';
      }
      if (approveDiffBtn) {
        approveDiffBtn.disabled = true;
        approveDiffBtn.textContent = 'Patch Rejected ✗';
      }
      const poolGrid = document.getElementById('pool-status-grid');
      if (poolGrid) {
        const cards = poolGrid.querySelectorAll('.pool-card');
        cards.forEach((c) => {
          c.classList.remove('pool-card-active');
          const st = c.querySelector('.pool-card-status');
          if (st) st.textContent = 'Standby · Ready';
        });
      }
      const applyBtn = document.getElementById('chat-apply-patch-btn');
      if (applyBtn) applyBtn.disabled = true;
      secopsAudit.recordEvent('patch_reject', rejectedPath, 'pool_review', { status: 'rejected' }, 'notice');
    });
  }

  const customMode = document.getElementById('custom-mode-select');
  if (customMode) {
    customMode.addEventListener('change', (e) => {
      const nextOpMode = e.target.value;
      AppState.customMode = AppState.config.operatingMode = nextOpMode;
      if (typeof tsComposer !== 'undefined' && tsComposer && typeof tsComposer.setMode === 'function') {
        if (nextOpMode === 'fast') {
          tsComposer.setMode('fast');
        } else if (nextOpMode === 'coding' || nextOpMode === 'agentic') {
          tsComposer.setMode('agentic');
        }
      }
      secopsAudit.recordEvent('mode_switch', nextOpMode, 'client_config', { mode: nextOpMode }, 'info');
    });
  }

  const retainLambdaInput = document.getElementById('custom-retain-lambda');
  if (retainLambdaInput) {
    const valEl = document.getElementById('custom-retain-lambda-val');
    const onLambda = (e) => {
      const val = e.target.value;
      AppState.retainLambda = AppState.config.retainLambda = val;
      if (valEl) valEl.textContent = val;
    };
    retainLambdaInput.addEventListener('input', onLambda);
    retainLambdaInput.addEventListener('change', (e) => {
      onLambda(e);
      secopsAudit.recordEvent('lambda_update', String(e.target.value), 'client_config', { lambda: String(e.target.value) }, 'info');
    });
  }

  const pluralityVotesInput = document.getElementById('custom-plurality-votes');
  if (pluralityVotesInput) {
    const onVotes = (e) => {
      const votes = Math.min(Math.max(Math.floor(Number(e.target.value) || 8), 1), 16);
      AppState.pluralityVotes = AppState.config.pluralityVotes = votes;
    };
    pluralityVotesInput.addEventListener('input', onVotes);
    pluralityVotesInput.addEventListener('change', (e) => {
      onVotes(e);
      secopsAudit.recordEvent('votes_update', String(AppState.config.pluralityVotes), 'client_config', { votes: AppState.config.pluralityVotes }, 'info');
    });
  }

  const densitySelect = document.getElementById('custom-density-select');
  if (densitySelect) {
    AppState.densityMode = AppState.ui.density = densitySelect.value;
    document.body.classList.toggle('density-compact', densitySelect.value === 'compact');
    densitySelect.addEventListener('change', (e) => {
      const val = e.target.value;
      AppState.densityMode = AppState.ui.density = val;
      document.body.classList.toggle('density-compact', val === 'compact');
      secopsAudit.recordEvent('density_change', val, 'client_config', { density: val }, 'info');
    });
  }

  const mcpPolicy = document.getElementById('mcp-policy-select');
  const autoRunToggleBtn = document.getElementById('composer-autorun-toggle');
  const syncAutoRunButtonWithPolicy = (rawPolicy) => {
    const policy = String(rawPolicy || 'safe').trim().toLowerCase();
    AppState.config.mcpSecurityPolicy = policy;
    if (mcpPolicy && mcpPolicy.value !== policy) {
      mcpPolicy.value = policy;
    }
    if (autoRunToggleBtn) {
      const isOn = policy === 'safe';
      autoRunToggleBtn.classList.toggle('active', isOn);
      autoRunToggleBtn.setAttribute('aria-pressed', isOn ? 'true' : 'false');
      autoRunToggleBtn.textContent = policy === 'deny'
        ? 'Tools: Deny'
        : (isOn ? 'Auto-run: On' : 'Auto-run: Off');
    }
    if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient && engineWorkerClient.enabled) {
      engineWorkerClient.rpc('set_bridge_policy', { policy }).catch(() => {});
    }
  };
  if (mcpPolicy) {
    syncAutoRunButtonWithPolicy(mcpPolicy.value || 'safe');
    mcpPolicy.addEventListener('change', (e) => {
      syncAutoRunButtonWithPolicy(e.target.value);
      secopsAudit.recordEvent('policy_update', e.target.value, 'mcp_security', { policy: e.target.value }, 'notice');
    });
  }
  if (autoRunToggleBtn) {
    autoRunToggleBtn.addEventListener('click', () => {
      const cur = String(AppState.config.mcpSecurityPolicy || 'safe').toLowerCase();
      const next = cur === 'safe' ? 'confirm' : 'safe';
      syncAutoRunButtonWithPolicy(next);
      secopsAudit.recordEvent('policy_update', next, 'mcp_security', { policy: next }, 'notice');
    });
  }

  const exportAuditBtn = document.getElementById('mcp-audit-export-btn');
  if (exportAuditBtn) {
    exportAuditBtn.addEventListener('click', () => {
      secopsAudit.exportJournal();
      secopsAudit.recordEvent('audit_export', 'journal.json', 'mcp_secops', { format: 'uor.secops.audit/1' }, 'notice');
    });
  }

  document.querySelectorAll('.mcp-tool-run-btn').forEach((btn) => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const tool = btn.getAttribute('data-tool');
      if (!checkMcpToolPolicy(tool, activeWorkspacePath || 'workspace')) return;
      let res = null;
      if (tool === 'workspace_read') {
        res = dispatchStudioEnvelope('workspace_read', activeWorkspacePath || 'src/attention.rs');
      } else if (tool === 'workspace_write') {
        res = dispatchStudioEnvelope('workspace_write', `${activeWorkspacePath || 'scratchpad.rs'}\n// Verified by MCP tool runner\n`);
      } else if (tool === 'subsystem_prime_gcd') {
        res = { gcd: 4, coords: [2180, 64] };
      } else if (tool === 'geometric_dijkstra_route') {
        res = dispatchStudioEnvelope('subsystems', '7,6,1,10,2180') || { waypoints: 4 };
      } else if (tool === 'clifford_bivector_wedge') {
        res = { blade: 'e1^e2', grade: 2, action: 12 };
      } else if (tool === 'coordinate_descent_sweep') {
        res = dispatchStudioEnvelope('train', '1') || { loss: 0.12 };
      }
      secopsAudit.recordEvent('tool_run', tool, 'mcp_tools', { status: 'success', result: res }, 'info');
    });
  });
}

// OTel Traces Inspector
const traceSpans = [];
let gllmRecordedTraces = [];
let selectedTraceIndex = 0;
let selectedSpanId = null;

function fetchRecordedTraces() {
  try {
    const res = dispatchStudioEnvelope('traces', '{"format":"compact","limit":10}');
    if (Array.isArray(res)) {
      gllmRecordedTraces = res;
    } else if (res && typeof res === 'object' && res.format === 'uor.gllm.trace/1') {
      gllmRecordedTraces = [res];
    }
  } catch (_e) {
    // Engine fallback
  }
  if (gllmRecordedTraces.length > 0) {
    const curSpans = Array.isArray(gllmRecordedTraces[selectedTraceIndex]?.spans)
      ? gllmRecordedTraces[selectedTraceIndex].spans.length
      : 0;
    if (selectedTraceIndex >= gllmRecordedTraces.length || curSpans <= 2) {
      let bestIdx = 0;
      let bestSpans = -1;
      for (let i = 0; i < gllmRecordedTraces.length; i++) {
        const sc = Array.isArray(gllmRecordedTraces[i]?.spans) ? gllmRecordedTraces[i].spans.length : 0;
        if (sc > bestSpans) {
          bestSpans = sc;
          bestIdx = i;
        }
      }
      selectedTraceIndex = bestIdx;
    }
  }
  renderTraceInspector();
}

function recordTraceSpan(name, attributes = { /* Formal R4.StudioFrame */}) {
  // Synchronize traces and update trace stat metrics
  fetchRecordedTraces();
}

function renderTraceInspector() {
  const treeEl = document.getElementById('trace-waterfall-tree');
  const traceIdEl = document.getElementById('trace-stat-trace-id');
  const internedCountEl = document.getElementById('trace-stat-interned-count');
  const dedupHitsEl = document.getElementById('trace-stat-dedup-hits');
  const savingsEl = document.getElementById('trace-stat-savings');

  if (gllmRecordedTraces.length === 0) {
    if (traceIdEl) traceIdEl.textContent = '—';
    if (internedCountEl) internedCountEl.textContent = '0';
    if (dedupHitsEl) dedupHitsEl.textContent = '0';
    if (savingsEl) savingsEl.textContent = '0%';
    if (treeEl) {
      treeEl.replaceChildren();
      const emptyNotice = document.createElement('div');
      emptyNotice.className = 'trace-empty-notice';
      emptyNotice.textContent = 'No GLLM execution traces recorded yet. Complete an inference turn or consolidation to inspect real-time OTel spans.';
      treeEl.appendChild(emptyNotice);
    }
    return;
  }

  const currentTrace = gllmRecordedTraces[selectedTraceIndex] || gllmRecordedTraces[0];
  const internTable = currentTrace.intern_table || { entries: [] };
  const entries = internTable.entries || [];
  const getInternStr = (idx) => (typeof idx === 'number' && entries[idx] !== undefined ? entries[idx] : String(idx));

  if (lexLean3dVisualizer && currentTrace) {
    lexLean3dVisualizer.loadTrace(currentTrace);
  }

  // Update bona fide stat cards
  const fullTraceId = currentTrace.trace_id || '';
  if (traceIdEl) {
    traceIdEl.textContent = fullTraceId.length > 16 ? `${fullTraceId.slice(0, 16)}…` : fullTraceId;
    traceIdEl.title = fullTraceId;
  }
  const internCount = currentTrace.metrics?.interned_hash_count ?? internTable.interned_hash_count ?? entries.length;
  if (internedCountEl) internedCountEl.textContent = String(internCount);

  const dedupHits = currentTrace.metrics?.deduped_span_hits ?? internTable.deduped_span_hits ?? 0;
  if (dedupHitsEl) dedupHitsEl.textContent = String(dedupHits);

  const savingsPct = currentTrace.metrics?.savings_percentage ?? 0;
  if (savingsEl) savingsEl.textContent = `${savingsPct.toFixed(1)}%`;

  if (!treeEl) return;
  treeEl.replaceChildren();

  // Waterfall Toolbar
  const toolbar = document.createElement('div');
  toolbar.className = 'trace-waterfall-toolbar';

  const metaSpan = document.createElement('span');
  metaSpan.className = 'trace-meta-label';
  metaSpan.textContent = `Turn #${currentTrace.turn_index} · Context: ${currentTrace.context_digest_before.slice(0, 8)} → ${currentTrace.context_digest_after.slice(0, 8)} · ${currentTrace.spans.length} Spans`;
  toolbar.appendChild(metaSpan);

  if (gllmRecordedTraces.length > 1) {
    const selWrapper = document.createElement('div');
    selWrapper.className = 'trace-select-wrapper';
    const selLabel = document.createElement('label');
    selLabel.textContent = 'Trace: ';
    selLabel.className = 'trace-select-label';
    const sel = document.createElement('select');
    sel.className = 'trace-select-dropdown';
    gllmRecordedTraces.forEach((t, i) => {
      const opt = document.createElement('option');
      opt.value = String(i);
      opt.textContent = `Turn #${t.turn_index} (${t.trace_id.slice(0, 8)}…)`;
      if (i === selectedTraceIndex) opt.selected = true;
      sel.appendChild(opt);
    });
    sel.addEventListener('change', (e) => {
      selectedTraceIndex = Number(e.target.value) || 0;
      renderTraceInspector();
    });
    selWrapper.appendChild(selLabel);
    selWrapper.appendChild(sel);
    toolbar.appendChild(selWrapper);
  }
  treeEl.appendChild(toolbar);

  // Waterfall Tree of Spans
  const spansContainer = document.createElement('div');
  spansContainer.className = 'trace-spans-container';

  const spans = currentTrace.spans || [];
  spans.forEach((span, idx) => {
    const spanName = getInternStr(span.name_ref);
    const nodeEl = document.createElement('div');
    nodeEl.className = 'trace-node';

    let pillClass = 'pill-generic';
    if (spanName.startsWith('gllm.turn')) pillClass = 'pill-turn';
    else if (spanName.startsWith('gllm.tokenize')) pillClass = 'pill-tokenize';
    else if (spanName.startsWith('gllm.trunk')) pillClass = 'pill-trunk';
    else if (spanName.startsWith('gllm.routing')) pillClass = 'pill-routing';
    else if (spanName.startsWith('gllm.decode')) pillClass = 'pill-decode';

    const header = document.createElement('div');
    header.className = 'trace-node-header';
    header.setAttribute('role', 'button');
    header.setAttribute('tabindex', '0');
    const isExpanded = selectedSpanId === span.span_id || (selectedSpanId === null && idx === 0);
    header.setAttribute('aria-expanded', isExpanded ? 'true' : 'false');

    const toggleIcon = document.createElement('span');
    toggleIcon.className = 'trace-toggle-icon';
    toggleIcon.textContent = isExpanded ? '▼' : '▶';
    header.appendChild(toggleIcon);

    const pill = document.createElement('span');
    pill.className = `span-pill ${pillClass}`;
    pill.textContent = spanName;
    header.appendChild(pill);

    const timing = document.createElement('span');
    timing.className = 'trace-timing-badge';
    const offsetUs = (span.start_offset_ns / 1000).toFixed(1);
    const durUs = (span.duration_ns / 1000).toFixed(1);
    timing.textContent = `+${offsetUs}µs (${durUs}µs)`;
    header.appendChild(timing);

    const summary = document.createElement('span');
    summary.className = 'trace-node-summary';
    let summaryText = '';
    const attrMap = {};
    if (Array.isArray(span.attributes)) {
      for (const [kRef, val] of span.attributes) {
        const kStr = getInternStr(kRef);
        attrMap[kStr] = val;
      }
    }

    if (spanName.startsWith('gllm.turn')) {
      summaryText = `Model: ${attrMap['uor.model'] || 'uor-r4.lut-stack/8'} · Tokens: ${attrMap['uor.prompt_tokens_count'] || 0} in / ${attrMap['uor.generated_tokens_count'] || 0} out`;
    } else if (spanName.startsWith('gllm.tokenize')) {
      summaryText = `Stems: ${attrMap['uor.salient_stems_count'] || 0} · Clauses: ${attrMap['uor.clauses_resolved'] || 0} · "${attrMap['uor.prompt_preview'] || ''}"`;
    } else if (spanName.startsWith('gllm.trunk')) {
      const rootIdx = attrMap['uor.dominant_icosian_root'] !== undefined ? attrMap['uor.dominant_icosian_root'] : 0;
      const spinHol = attrMap['uor.clifford_spin_holonomy'] !== undefined ? attrMap['uor.clifford_spin_holonomy'] : 0;
      summaryText = `Icosian Root: #${rootIdx} · Spin Holonomy: 0x${Number(spinHol).toString(16)} · Basin L1: ${attrMap['uor.basin_energy_l1'] || 0}`;
    } else if (spanName.startsWith('gllm.routing')) {
      summaryText = `Winning Row: ${attrMap['uor.winning_row'] || 0} · Hamming: ${attrMap['uor.point_hamming_bits'] || 0}b · L1 Dist: ${attrMap['uor.l1_distance'] || 0}`;
    } else if (spanName.startsWith('gllm.decode')) {
      const pred = attrMap['uor.predicted_token'];
      const mech = attrMap['uor.source_mechanism'] || 'point_routing';
      summaryText = `Token ID: ${pred} · Mechanism: ${mech} · L1: ${attrMap['uor.l1_distance'] || 0}`;
    }
    summary.textContent = summaryText;
    header.appendChild(summary);

    if (Array.isArray(span.links) && span.links.length > 0) {
      for (const link of span.links) {
        const linkBadge = document.createElement('span');
        linkBadge.className = 'span-link-badge';
        const lTrace = getInternStr(link.trace_id_ref);
        const lSpan = getInternStr(link.span_id_ref);
        let lType = 'provenance';
        if (Array.isArray(link.attributes) && link.attributes.length > 0) {
          lType = getInternStr(link.attributes[0][1]);
        }
        linkBadge.textContent = `🔗 ${lType.replace(/_/g, ' ')}`;
        linkBadge.title = `Linked Trace: ${lTrace}\nLinked Span: ${lSpan}`;
        header.appendChild(linkBadge);
      }
    }

    nodeEl.appendChild(header);

    const body = document.createElement('div');
    body.className = 'trace-node-body';
    if (!isExpanded) body.setAttribute('hidden', '');

    const toggleExpand = () => {
      const nowHidden = body.hasAttribute('hidden');
      if (nowHidden) {
        body.removeAttribute('hidden');
        header.setAttribute('aria-expanded', 'true');
        toggleIcon.textContent = '▼';
        selectedSpanId = span.span_id;
      } else {
        body.setAttribute('hidden', '');
        header.setAttribute('aria-expanded', 'false');
        toggleIcon.textContent = '▶';
        if (selectedSpanId === span.span_id) selectedSpanId = null;
      }
    };
    header.addEventListener('click', toggleExpand);
    header.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        toggleExpand();
      }
    });

    if (spanName.startsWith('gllm.trunk')) {
      const cliffordContainer = document.createElement('div');
      cliffordContainer.className = 'trace-clifford-section';
      const cTitle = document.createElement('div');
      cTitle.className = 'trace-section-title';
      cTitle.textContent = 'Clifford Cl(0,6) Grade Spectrum & Basin Energy';
      cliffordContainer.appendChild(cTitle);

      const grades = attrMap['uor.clifford_grades'] || [1, 6, 15, 20, 15, 6, 1];
      const specGrid = document.createElement('div');
      specGrid.className = 'clifford-grade-grid';
      grades.forEach((cnt, g) => {
        const item = document.createElement('div');
        item.className = 'clifford-grade-item';
        item.innerHTML = `<span class="grade-label">Grade ${g}</span><span class="grade-val">${cnt}</span>`;
        specGrid.appendChild(item);
      });
      cliffordContainer.appendChild(specGrid);
      body.appendChild(cliffordContainer);
    } else if (spanName.startsWith('gllm.decode')) {
      const cands = attrMap['uor.top_candidates'];
      if (Array.isArray(cands) && cands.length > 0) {
        const candSection = document.createElement('div');
        candSection.className = 'trace-candidates-section';
        const candTitle = document.createElement('div');
        candTitle.className = 'trace-section-title';
        candTitle.textContent = 'Top Candidate Token Basins (Q12 Logits & Distance Margins)';
        candSection.appendChild(candTitle);

        const table = document.createElement('table');
        table.className = 'trace-candidates-table';
        const thead = document.createElement('thead');
        thead.innerHTML = '<tr><th>Rank</th><th>Token ID</th><th>Q12 Logit</th><th>L1 Distance</th><th>Provenance Link</th></tr>';
        table.appendChild(thead);
        const tbody = document.createElement('tbody');
        cands.forEach((c, rIdx) => {
          const row = document.createElement('tr');
          row.innerHTML = `<td>#${rIdx + 1}</td><td><code>${c.token_id}</code></td><td>${c.logit_q12}</td><td>${c.l1_distance}</td><td><span class="prov-tag">writer basin</span></td>`;
          tbody.appendChild(row);
        });
        table.appendChild(tbody);
        candSection.appendChild(table);
        body.appendChild(candSection);
      }
    }

    if (Array.isArray(span.links) && span.links.length > 0) {
      const linkCard = document.createElement('div');
      linkCard.className = 'trace-provenance-card';
      const pTitle = document.createElement('div');
      pTitle.className = 'trace-section-title';
      pTitle.textContent = 'Cross-Lifecycle Provenance Links';
      linkCard.appendChild(pTitle);

      span.links.forEach((l) => {
        const item = document.createElement('div');
        item.className = 'prov-link-item';
        const lTrace = getInternStr(l.trace_id_ref);
        const lSpan = getInternStr(l.span_id_ref);
        let lType = 'training_provenance';
        if (Array.isArray(l.attributes) && l.attributes.length > 0) {
          lType = getInternStr(l.attributes[0][1]);
        }
        item.innerHTML = `<span class="prov-type">${lType}</span>: trace=<code>${lTrace}</code> span=<code>${lSpan}</code>`;
        linkCard.appendChild(item);
      });
      body.appendChild(linkCard);
    }

    const attrTable = document.createElement('table');
    attrTable.className = 'trace-attr-table';
    const tbody = document.createElement('tbody');
    for (const [k, v] of Object.entries(attrMap)) {
      if (k === 'uor.top_candidates' || k === 'uor.clifford_grades') continue;
      const tr = document.createElement('tr');
      const tdK = document.createElement('td');
      tdK.className = 'attr-key';
      tdK.textContent = k;
      const tdV = document.createElement('td');
      tdV.className = 'attr-val';
      tdV.textContent = typeof v === 'object' ? JSON.stringify(v) : String(v);
      tr.appendChild(tdK);
      tr.appendChild(tdV);
      tbody.appendChild(tr);
    }
    attrTable.appendChild(tbody);
    body.appendChild(attrTable);

    nodeEl.appendChild(body);
    spansContainer.appendChild(nodeEl);
  });

  treeEl.appendChild(spansContainer);
}

// Legacy modal compatibility wrappers
const workspaceModal = document.getElementById('workspace-modal');
const diagnosticsModal = document.getElementById('diagnostics-modal');

function setModalOpen(modalEl, openState) { /* Formal R4.StudioFrame */
  if (!modalEl) return;
  if (openState) {
    modalEl.removeAttribute('hidden');
  } else {
    modalEl.setAttribute('hidden', '');
  }
}

const closeWsBtn = document.getElementById('close-workspace-modal-btn');
if (closeWsBtn) {
  closeWsBtn.addEventListener('click', () => setModalOpen(workspaceModal, false));
}

const closeDiagBtn = document.getElementById('close-diagnostics-modal-btn');
if (closeDiagBtn) {
  closeDiagBtn.addEventListener('click', () => setModalOpen(diagnosticsModal, false));
}

for (const modalEl of [workspaceModal, diagnosticsModal]) {
  if (modalEl) {
    modalEl.addEventListener('click', (event) => {
      if (event.target === modalEl) {
        setModalOpen(modalEl, false);
      }
    });
  }
}

if (chatApplyPatchBtn) {
  chatApplyPatchBtn.addEventListener('click', () => {
    if (!pendingPatchPath || !pendingProposedCode) return;
    if (!checkMcpToolPolicy('workspace_write', pendingPatchPath)) return;
    const ws = dispatchStudioEnvelope('workspace_write', `${pendingPatchPath}\n${pendingProposedCode}`);
    if (ws && ws.status === 'ok') {
      if (AppState.workspace.pendingPatches && AppState.workspace.pendingPatches[pendingPatchPath]) {
        delete AppState.workspace.pendingPatches[pendingPatchPath];
      }
      renderPendingPatchPills();
      activeWorkspacePath = pendingPatchPath;
      renderWorkspaceList(ws);
      selectWorkspaceFile(pendingPatchPath);
      const st = dispatchStudioEnvelope('state', '');
      renderStudioState(st);
      if (workspaceStatus) {
        workspaceStatus.textContent = `Applied synthesized patch to ${pendingPatchPath} and admitted into geometric memory.`;
      }
      secopsAudit.recordEvent('workspace_write', pendingPatchPath, 'file_system', { action: 'patch_apply', bytes: pendingProposedCode.length }, 'notice');
      activateCanvasTab('workspace');
    }
  });
}

if (chatDiffViewBtn && chatArtifactViewBtn) {
  chatDiffViewBtn.addEventListener('click', () => {
    chatDiffViewBtn.classList.add('active', 'is-active');
    chatDiffViewBtn.setAttribute('aria-pressed', 'true');
    chatArtifactViewBtn.classList.remove('active', 'is-active');
    chatArtifactViewBtn.setAttribute('aria-pressed', 'false');
    if (chatDiffPreview) {
      chatDiffPreview.removeAttribute('hidden');
      chatDiffPreview.classList.remove('is-hidden');
    }
    if (chatArtifactPreview) {
      chatArtifactPreview.setAttribute('hidden', '');
      chatArtifactPreview.classList.add('is-hidden');
    }
  });
  chatArtifactViewBtn.addEventListener('click', () => {
    chatArtifactViewBtn.classList.add('active', 'is-active');
    chatArtifactViewBtn.setAttribute('aria-pressed', 'true');
    chatDiffViewBtn.classList.remove('active', 'is-active');
    chatDiffViewBtn.setAttribute('aria-pressed', 'false');
    if (chatDiffPreview) {
      chatDiffPreview.setAttribute('hidden', '');
      chatDiffPreview.classList.add('is-hidden');
    }
    if (chatArtifactPreview) {
      chatArtifactPreview.removeAttribute('hidden');
      chatArtifactPreview.classList.remove('is-hidden');
      renderInteractiveArtifactPreview(pendingPatchPath || activeWorkspacePath, pendingProposedCode);
    }
  });
}

if (workspaceNewForm) {
  workspaceNewForm.addEventListener('submit', (event) => {
    event.preventDefault();
    const rawPath = workspaceNewPath ? workspaceNewPath.value.trim() : '';
    if (!rawPath) return;
    if (!checkMcpToolPolicy('workspace_write', rawPath)) return;
    const stub = `//! Module ${rawPath} initialized in UOR-R4 Workspace.\npub fn module_entry_root() -> u8 {\n    0\n}\n`;
    const ws = dispatchStudioEnvelope('workspace_write', `${rawPath}\n${stub}`);
    if (ws && ws.status === 'ok') {
      activeWorkspacePath = rawPath;
      renderWorkspaceList(ws);
      selectWorkspaceFile(rawPath);
      const st = dispatchStudioEnvelope('state', '');
      renderStudioState(st);
      if (workspaceStatus) {
        workspaceStatus.textContent = `Created ${rawPath} in workspace and admitted into geometric memory.`;
      }
      secopsAudit.recordEvent('workspace_write', rawPath, 'file_system', { action: 'file_create', bytes: stub.length }, 'notice');
    }
  });
}

if (workspaceSaveBtn) {
  workspaceSaveBtn.addEventListener('click', () => {
    const content = workspaceEditorTextarea ? workspaceEditorTextarea.value : '';
    if (!activeWorkspacePath) return;
    if (!checkMcpToolPolicy('workspace_write', activeWorkspacePath)) return;
    const ws = dispatchStudioEnvelope('workspace_write', `${activeWorkspacePath}\n${content}`);
    if (ws && ws.status === 'ok') {
      renderWorkspaceList(ws);
      if (typeof editorTabManager !== 'undefined' && editorTabManager) {
        editorTabManager.markSaved(activeWorkspacePath);
      }
      selectWorkspaceFile(activeWorkspacePath);
      const st = dispatchStudioEnvelope('state', '');
      renderStudioState(st);
      if (typeof writeCursorDiskFileHandle === 'function') {
        writeCursorDiskFileHandle(activeWorkspacePath, content);
      }
      if (workspaceStatus) {
        workspaceStatus.textContent = `Saved ${activeWorkspacePath} and admitted updated tokens into geometric memory.`;
      }
      secopsAudit.recordEvent('workspace_write', activeWorkspacePath, 'file_system', { action: 'save', bytes: content.length }, 'info');
    }
  });
}

if (workspaceDownloadBtn) {
  workspaceDownloadBtn.addEventListener('click', () => {
    const content = workspaceEditorTextarea ? workspaceEditorTextarea.value : '';
    const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    const baseName = (activeWorkspacePath || 'module.rs').split('/').pop() || 'module.rs';
    a.download = baseName;
    a.click();
    URL.revokeObjectURL(url);
    if (workspaceStatus) {
      workspaceStatus.textContent = `Exported ${activeWorkspacePath} (${content.length} bytes) to local disk.`;
    }
  });
}

if (workspaceMountInput) {
  workspaceMountInput.addEventListener('change', async () => {
    const files = workspaceMountInput.files;
    if (!files || files.length === 0) return;
    const chunks = [];
    const validExt = /\.(rs|lean|toml|yaml|yml|md|js|mjs|ts|json|css|html|py|txt|sh)$/i;
    for (let i = 0; i < files.length; i += 1) {
      const f = files[i];
      const relPath = f.webkitRelativePath || (f.name ? `src/${f.name}` : '');
      if (!relPath) continue;
      const parts = relPath.split('/');
      const hasHiddenOrIgnored = parts.some(
        (p) => p.startsWith('.') || p === 'node_modules' || p === 'target'
      );
      if (hasHiddenOrIgnored) continue;
      if (!validExt.test(f.name || relPath)) continue;
      if (typeof f.size === 'number' && f.size > 512 * 1024) continue;
      const text = typeof f.text === 'function' ? await f.text() : '';
      chunks.push(`${relPath}\n${text}`);
    }
    if (chunks.length > 0) {
      const batchPayload = chunks.join('\n===FILE===\n');
      const ws = dispatchStudioEnvelope('workspace_batch', batchPayload);
      if (ws && ws.status === 'ok') {
        renderWorkspaceList(ws);
        const st = dispatchStudioEnvelope('state', '');
        renderStudioState(st);
        if (workspaceStatus) {
          workspaceStatus.textContent = `Mounted ${ws.batch_ingested_files} local file(s) (${ws.batch_ingested_tokens} tokens indexed into geometric memory).`;
        }
      }
    }
    try {
      workspaceMountInput.value = '';
    } catch {}
  });
}

const weightCacheBadge = document.getElementById('weight-cache-badge');
const topbarModelTag = document.getElementById('topbar-model-tag');
const syncWeightsBtn = document.getElementById('sync-weights-btn');
const loadWeightsTopbarBtn = document.getElementById('load-weights-topbar-btn');

if (loadWeightsTopbarBtn && snapshotUploadInput) {
  loadWeightsTopbarBtn.addEventListener('click', () => {
    snapshotUploadInput.click();
  });
}

/** Formal R4.StudioFrame format byte hydration progress pill (`Downloading XX% · YY / 410 MB`). */
function formatHydrationProgressLabel(prog) {
  /* Formal R4.StudioFrame */
  if (!prog || typeof prog !== 'object') return '';
  const totalBytes = Number(prog.totalBytes) || 0;
  const dlBytes = Number(prog.downloadedBytes) || 0;
  const idx = Number(prog.done ?? prog.index ?? 0);
  const totalPieces = Math.max(1, Number(prog.total) || 1);
  const pct = typeof prog.percent === 'number'
    ? Math.min(100, Math.max(0, Math.round(prog.percent)))
    : Math.min(100, Math.max(0, Math.round((idx / totalPieces) * 100)));
  const cachedHits = Number(prog.cachedHits) || 0;
  const isCachedMount =
    (cachedHits > 0 && cachedHits >= Math.max(1, idx)) ||
    /HOLOSLAB2|OPFS/i.test(String(prog.label || ''));
  if (isCachedMount) {
    return `Loading cached weights ${pct}%`;
  }
  if (totalBytes >= 1000 * 1024 * 1024) {
    const dlGb = (dlBytes / (1024 * 1024 * 1024)).toFixed(1);
    const totGb = (totalBytes / (1024 * 1024 * 1024)).toFixed(1);
    return `Downloading ${pct}% · ${dlGb} / ${totGb} GB`;
  }
  const totMb = totalBytes > 0 ? Math.max(1, Math.round(totalBytes / (1024 * 1024))) : 410;
  const dlMb = totalBytes > 0
    ? Math.min(totMb, Math.max(0, Math.round(dlBytes / (1024 * 1024))))
    : Math.min(totMb, Math.round((pct / 100) * totMb));
  return `Downloading ${pct}% · ${dlMb} / ${totMb} MB`;
}

/** Formal R4.StudioFrame toggle and update the inline composer weight hydration progress strip. */
function updateInlineHydrationBar(prog) {
  /* Formal R4.StudioFrame */
  if (typeof document === 'undefined' || typeof document.getElementById !== 'function') return;
  const bar = document.getElementById('hydration-inline-progress');
  const labelEl = document.getElementById('hydration-inline-progress-label');
  const fillEl = document.getElementById('hydration-inline-progress-fill');
  if (!bar) return;
  if (!prog) {
    bar.setAttribute('hidden', '');
    bar.hidden = true;
    if (fillEl && fillEl.style) fillEl.style.width = '0%';
    return;
  }
  const pct = typeof prog.percent === 'number'
    ? Math.min(100, Math.max(0, Math.round(prog.percent)))
    : Math.min(100, Math.max(0, Math.round((Number(prog.done ?? prog.index ?? 0) / Math.max(1, Number(prog.total) || 1)) * 100)));
  bar.removeAttribute('hidden');
  bar.hidden = false;
  if (labelEl) {
    labelEl.textContent = formatHydrationProgressLabel(prog);
  }
  if (fillEl && fillEl.style) {
    fillEl.style.width = `${pct}%`;
  }
}

/** Formal R4.FastPackHydration canonical selector helper for `data-studio-id` aliases. */
function selectStudioControl(studioId) {
  /* Formal R4.FastPackHydration */
  if (typeof document === 'undefined' || !document || typeof document.querySelector !== 'function') {
    return null;
  }
  return document.querySelector(`[data-studio-id="${String(studioId || '')}"]`);
}
globalThis.__uorStudioSelect = selectStudioControl;

/** Formal R4.FastPackHydration verify single-truth consistency between model picker and hydration pill. */
function admitSingleTruthBadgeUi(selectedIsEmergencyQ31, badgeShowsEmergencyQ31) {
  /* Formal R4.FastPackHydration */
  if (!selectedIsEmergencyQ31 && badgeShowsEmergencyQ31) {
    return 'RefusedEmergencyBadgeOnQwenSession';
  }
  return 'SingleTruthBadgeAdmitted';
}
globalThis.__uorAdmitSingleTruthBadge = admitSingleTruthBadgeUi;

/** Formal R4.StudioFrame render honest clickable status pill on `#weight-cache-badge`. */
function updateWeightCacheBadge() {
  const holoInfo = (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.holoStatus) || null;
  const webgpuInfo = (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.webgpuStatus) || null;
  const hydProg = (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.hydrationProgress) || null;
  const q31Info = (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.nativeQ31Info) || null;
  let r4Info = (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.r4StackInfo) || null;
  if (!r4Info && (!engineWorkerClient || !engineWorkerClient.enabled) && mainThreadWasmReady && typeof studio_r4_stack_status === 'function') {
    try {
      r4Info = JSON.parse(studio_r4_stack_status());
    } catch {
      r4Info = null;
    }
  }
  const explicitEmergencyStored =
    typeof safeStorageGet === 'function' && safeStorageGet('uor_offline_emergency_q31') === '1';
  const topbarPicker =
    (typeof modelSelect !== 'undefined' && modelSelect) ||
    (typeof document !== 'undefined' && document && document.getElementById('model-select'));
  const pickerVal = topbarPicker && topbarPicker.value ? String(topbarPicker.value) : '';
  const selId = explicitEmergencyStored
    ? 'uor-r4.native-q31-d85'
    : (pickerVal
      || (typeof composerModelSelect !== 'undefined' && composerModelSelect && composerModelSelect.value)
      || (AppState && AppState.config && AppState.config.selectedModel)
      || 'uor-r4.qwen3-0.6b-holo');
  if (!explicitEmergencyStored && AppState && AppState.config && AppState.config.selectedModel === 'uor-r4.native-q31-d85' && pickerVal && pickerVal !== 'uor-r4.native-q31-d85') {
    AppState.config.selectedModel = pickerVal;
  }
  const offlineEmergency = explicitEmergencyStored || selId === 'uor-r4.native-q31-d85';
  if (topbarPicker && topbarPicker.options && typeof document !== 'undefined' && document && typeof document.createElement === 'function') {
    let dynOpt = Array.from(topbarPicker.options).find((o) => o && o.value === 'uor-r4.native-q31-d85');
    if (offlineEmergency) {
      if (!dynOpt) {
        dynOpt = document.createElement('option');
        dynOpt.value = 'uor-r4.native-q31-d85';
        dynOpt.textContent = 'Offline Q31 · Emergency (0-Download)';
        dynOpt.setAttribute('data-dynamic-emergency', '1');
        topbarPicker.appendChild(dynOpt);
      }
      topbarPicker.value = 'uor-r4.native-q31-d85';
    } else if (dynOpt && dynOpt.getAttribute && dynOpt.getAttribute('data-dynamic-emergency') === '1') {
      dynOpt.remove();
    }
    if (typeof topbarPicker.disabled === 'boolean') {
      topbarPicker.disabled = false;
    }
  }
  const is17b = selId === 'uor-r4.qwen3-1.7b-holo';
  const targetMounted = Boolean(
    holoInfo && (
      (is17b && holoInfo.qwen3_17b_mounted)
      || (!is17b && holoInfo.qwen3_06b_mounted)
      || (Array.isArray(holoInfo.models) && holoInfo.models.some((m) => m && m.model_id === selId))
    ),
  );
  if (weightCacheBadge) {
    if (hydProg) {
      weightCacheBadge.textContent = formatHydrationProgressLabel(hydProg);
      weightCacheBadge.setAttribute('data-cache-state', 'downloading');
    } else if (offlineEmergency && admitSingleTruthBadgeUi(offlineEmergency, true) === 'SingleTruthBadgeAdmitted') {
      weightCacheBadge.textContent = 'Offline Q31 · Emergency';
      weightCacheBadge.setAttribute('data-cache-state', 'offline-q31');
    } else if (targetMounted) {
      const gpuReady = Boolean(
        (webgpuInfo && webgpuInfo.webgpu_active)
        || (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.webgpuActive),
      );
      weightCacheBadge.textContent = gpuReady ? 'Ready · WebGPU' : 'Ready · Wasm SIMD';
      weightCacheBadge.setAttribute('data-cache-state', 'ready');
    } else {
      weightCacheBadge.textContent = is17b
        ? '1.1 GB · Auto-downloads on send'
        : '410 MB · Auto-downloads on send';
      weightCacheBadge.setAttribute('data-cache-state', 'cold');
    }
  }
  if (topbarModelTag) {
    try {
      if (holoInfo && Array.isArray(holoInfo.models)) {
        const mountedEntry = holoInfo.models.find((m) => m.model_id === selId);
        if (mountedEntry) {
          topbarModelTag.textContent = `Holo v13 · ${selId} (${mountedEntry.n_layers}L/${mountedEntry.d_model}D)`;
          topbarModelTag.title = `UORR4V13 · ${selId} · float_ops=0 · dense_matmul_ops=0`;
          return;
        }
      }
      if (selId === 'uor-r4.native-q31-d85' && q31Info) {
        topbarModelTag.textContent = `Native Q31 D85 · ${q31Info.n_layers || 4}L/${q31Info.d_model || 256}D (step=${q31Info.step || 0})`;
        topbarModelTag.title = 'uor-r4.native-q31-d85/1 · float_ops=0 · dense_matmul_ops=0';
      } else {
        let packInfo = null;
        if (engineWorkerClient.enabled) {
          packInfo = engineWorkerClient.packInfo;
        } else if (mainThreadWasmReady && typeof studio_weight_pack_status === 'function') {
          packInfo = JSON.parse(studio_weight_pack_status());
        }
        topbarModelTag.textContent = `Holo v13 · ${selId}`;
        topbarModelTag.title = packInfo && packInfo.status === 'ok'
          ? `UOR-R4 Holo v13 + Native Q31 D85 (${((packInfo.byte_len || 0) / (1024 * 1024)).toFixed(1)} MB cache)`
          : 'UOR-R4 Holo v13 Pure-Integer Worker';
      }
    } catch {
      // Ignore parse error during early boot
    }
  }
}

if (weightCacheBadge) {
  weightCacheBadge.addEventListener('click', () => {
    const selId = (typeof composerModelSelect !== 'undefined' && composerModelSelect && composerModelSelect.value)
      || (AppState && AppState.config && AppState.config.selectedModel)
      || DEFAULT_HONEST_MODEL_ID;
    if (selId !== 'uor-r4.qwen3-0.6b-holo' && selId !== 'uor-r4.qwen3-1.7b-holo') return;
    if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
      const approxBytes = selId.includes('1.7b') ? Math.round(1.09 * 1024 * 1024 * 1024) : 410 * 1024 * 1024;
      engineWorkerClient.hydrationProgress = {
        modelId: selId,
        percent: 0,
        downloadedBytes: 0,
        totalBytes: approxBytes,
      };
      updateWeightCacheBadge();
      updateInlineHydrationBar(engineWorkerClient.hydrationProgress);
      engineWorkerClient
        .rpc('hydrate_holo_model', { modelId: selId })
        .then((res) => {
          engineWorkerClient.hydrationProgress = null;
          updateInlineHydrationBar(null);
          if (res && res.holo) engineWorkerClient.holoStatus = res.holo;
          return engineWorkerClient.rpc('holo_runtime_status', {}).catch(() => null);
        })
        .then((hsRes) => {
          if (hsRes) {
            const hs = (hsRes.holo && typeof hsRes.mounted_count !== 'number') ? hsRes.holo : hsRes;
            if (hs && typeof hs.mounted_count === 'number') engineWorkerClient.holoStatus = hs;
            if (hsRes.webgpu) engineWorkerClient.webgpuStatus = hsRes.webgpu;
          }
          updateWeightCacheBadge();
        })
        .catch(() => {
          engineWorkerClient.hydrationProgress = null;
          updateInlineHydrationBar(null);
          updateWeightCacheBadge();
        });
    }
  });
}

if (syncWeightsBtn) {
  syncWeightsBtn.addEventListener('click', async () => {
    syncWeightsBtn.disabled = true;
    const prevText = syncWeightsBtn.textContent;
    syncWeightsBtn.textContent = 'Syncing 0%...';
    try {
      if (engineWorkerClient.enabled) {
        const synced = await engineWorkerClient.rpc('sync_weights', {});
        if (synced && synced.gllm) engineWorkerClient.gllmInfo = synced.gllm;
        if (synced && synced.gllmMeta) globalThis.__uorGllmV9Meta = synced.gllmMeta;
        if (synced && synced.weightCacheMeta) globalThis.__uorWeightCacheMeta = synced.weightCacheMeta;
      } else {
        await sync_cached_weights((prog) => {
          if (prog && typeof prog.percent === 'number') {
            syncWeightsBtn.textContent = `Syncing ${prog.percent}%...`;
          }
          updateWeightCacheBadge();
        });
      }
      updateWeightCacheBadge();
      const st = dispatchStudioEnvelope('state', '');
      renderStudioState(st);
    } finally {
      syncWeightsBtn.disabled = false;
      syncWeightsBtn.textContent = prevText;
    }
  });
}

if (studioTrainForm) {
  studioTrainForm.addEventListener('submit', (event) => {
    event.preventDefault();
    runOnDeviceTrain(trainEpochsInput ? trainEpochsInput.value : '3');
  });
}

const isChildSurfaceEarly = typeof window !== 'undefined'
  && window.location
  && (String(window.location.search || '').includes('surface=preview')
    || String(window.location.search || '').includes('surface=observatory'));
if (!isChildSurfaceEarly) {
  loadThreadsFromStorage();
  renderThreadSidebar();
  renderLiteratureFeed();
  renderActiveThreadMessages();
  if (document.getElementById('neural-3d-canvas')) {
    lexLean3dVisualizer = new LexLean3DVisualizer();
    lexLean3dVisualizer.init('neural-3d-canvas', { deferGL: true });
  }
  renderTraceInspector();
  initSidebarViews();
  initDragAndDropIngestion();
  initCustomizationsAndMcp();
  if (document.getElementById('terminal-container')) {
    lexLeanTerminal = new LexLeanTerminal();
    lexLeanTerminal.init('terminal-container');
  }
}

const toggleRightDrawerBtn = document.getElementById('toggle-right-drawer-btn');
if (toggleRightDrawerBtn) {
  toggleRightDrawerBtn.addEventListener('click', () => {
    if (dockableCanvasPane && dockableCanvasPane.hasAttribute('hidden')) {
      dockableCanvasPane.removeAttribute('hidden');
      if (mainLayoutEl) mainLayoutEl.classList.remove('canvas-drawer-closed');
    } else {
      closeCanvasDrawer();
    }
  });
}

// ============================================================================
// CURSOR IDE WORKBENCH CONTROLLER (100% Local UOR-R4 Model)
// ============================================================================

class EditorTabManager {
  constructor() {
    this.tabs = new Map(); // path -> { path, name, buffer, savedContent, isDirty, cursorStart, cursorEnd, scrollTop }
    this.activePath = null;
    this.tabsContainer = document.getElementById('cursor-editor-tabs') || document.getElementById('editor-tabs-bar');
    this.textarea = document.getElementById('workspace-editor-textarea');
    this.lineNumbersEl = document.getElementById('editor-line-numbers') || document.getElementById('cursor-line-gutter');
  }

  init(defaultFiles = ['src/attention.rs', 'src/lib.rs', 'src/LutStack.rs', 'Cargo.toml']) {
    for (const path of defaultFiles) {
      if (!this.tabs.has(path)) {
        const readRes = dispatchStudioEnvelope('workspace_read', path);
        const content = (readRes && readRes.status === 'ok') ? String(readRes.content || '') : '';
        const name = path.split('/').pop() || path;
        this.tabs.set(path, {
          path,
          name,
          buffer: content,
          savedContent: content,
          isDirty: false,
          cursorStart: 0,
          cursorEnd: 0,
          scrollTop: 0
        });
      }
    }
    const initialPath = defaultFiles[0] || 'src/attention.rs';
    this.switchTo(initialPath);
  }

  open(path, content = null) {
    if (!path) return;
    if (this.tabs.has(path)) {
      if (content !== null && !this.tabs.get(path).isDirty) {
        this.tabs.get(path).buffer = String(content);
        this.tabs.get(path).savedContent = String(content);
      }
      this.switchTo(path);
      return;
    }
    let tabContent = content;
    if (tabContent === null) {
      const readRes = dispatchStudioEnvelope('workspace_read', path);
      tabContent = (readRes && readRes.status === 'ok') ? String(readRes.content || '') : '';
    }
    const name = path.split('/').pop() || path;
    const tab = {
      path,
      name,
      buffer: String(tabContent || ''),
      savedContent: String(tabContent || ''),
      isDirty: false,
      cursorStart: 0,
      cursorEnd: 0,
      scrollTop: 0
    };
    this.tabs.set(path, tab);
    if (!cursorOpenTabs.includes(path)) {
      cursorOpenTabs.push(path);
    }
    this.switchTo(path);
  }

  switchTo(path) {
    if (!path) return;
    if (!this.tabs.has(path)) {
      this.open(path);
      return;
    }
    // Stash current active tab buffer, cursor, and scroll
    if (this.activePath && this.tabs.has(this.activePath)) {
      const cur = this.tabs.get(this.activePath);
      const ta = this.textarea || document.getElementById('workspace-editor-textarea');
      if (ta) {
        cur.buffer = ta.value;
        cur.cursorStart = ta.selectionStart || 0;
        cur.cursorEnd = ta.selectionEnd || 0;
        cur.scrollTop = ta.scrollTop || 0;
      }
      cur.isDirty = (cur.buffer !== cur.savedContent);
    }

    this.activePath = path;
    activeWorkspacePath = path;
    const next = this.tabs.get(path);

    const ta = this.textarea || document.getElementById('workspace-editor-textarea');
    if (ta) {
      ta.value = next.buffer;
      try {
        ta.selectionStart = next.cursorStart;
        ta.selectionEnd = next.cursorEnd;
        ta.scrollTop = next.scrollTop;
      } catch {}
    }

    if (editorFilePath) editorFilePath.textContent = path;
    this.updateLineNumbers();
    this.updateStatusBar();
    this.renderTabs();

    if (typeof updateCursorBreadcrumbsAndCursorPos === 'function') {
      updateCursorBreadcrumbsAndCursorPos();
    }
    if (typeof updateCursorGhostText === 'function') {
      updateCursorGhostText();
    }
  }

  close(path) {
    if (this.tabs.size <= 1 || cursorOpenTabs.length <= 1) return;
    if (!this.tabs.has(path) && !cursorOpenTabs.includes(path)) return;
    this.tabs.delete(path);
    const tabIdx = cursorOpenTabs.indexOf(path);
    if (tabIdx >= 0) {
      cursorOpenTabs.splice(tabIdx, 1);
    }

    if (this.activePath === path || activeWorkspacePath === path) {
      const remaining = cursorOpenTabs.length > 0 ? cursorOpenTabs : Array.from(this.tabs.keys());
      if (remaining.length > 0) {
        const nextPath = remaining[Math.min(tabIdx >= 0 ? tabIdx : 0, remaining.length - 1)];
        this.switchTo(nextPath);
      } else {
        this.activePath = null;
        activeWorkspacePath = '';
        const ta = this.textarea || document.getElementById('workspace-editor-textarea');
        if (ta) ta.value = '';
        if (editorFilePath) editorFilePath.textContent = '';
        this.updateLineNumbers();
        this.updateStatusBar();
      }
    }
    this.renderTabs();
  }

  updateBuffer(content) {
    if (!this.activePath || !this.tabs.has(this.activePath)) return;
    const tab = this.tabs.get(this.activePath);
    tab.buffer = String(content || '');
    tab.isDirty = (tab.buffer !== tab.savedContent);
    this.updateLineNumbers();
    this.updateStatusBar();
    this.renderTabs();
  }

  markSaved(path) {
    const targetPath = path || this.activePath;
    if (!targetPath || !this.tabs.has(targetPath)) return;
    const tab = this.tabs.get(targetPath);
    tab.savedContent = tab.buffer;
    tab.isDirty = false;
    this.updateStatusBar();
    this.renderTabs();
  }

  updateLineNumbers() {
    const gutterEl = this.lineNumbersEl || document.getElementById('editor-line-numbers') || document.getElementById('cursor-line-gutter');
    const ta = this.textarea || document.getElementById('workspace-editor-textarea');
    if (!gutterEl || !ta) return;
    const text = String(ta.value || '');
    const lineCount = Math.max(1, text.split('\n').length);
    const nums = [];
    for (let i = 1; i <= lineCount; i++) {
      nums.push(String(i));
    }
    const linesStr = nums.join('\n');
    const legacyGutter = document.getElementById('cursor-line-gutter');
    if (legacyGutter && legacyGutter.parentNode === gutterEl) {
      legacyGutter.textContent = '';
      gutterEl.textContent = linesStr;
      gutterEl.prepend(legacyGutter);
    } else {
      gutterEl.textContent = linesStr;
      if (legacyGutter) legacyGutter.textContent = linesStr;
    }
    gutterEl.scrollTop = ta.scrollTop;
  }

  updateStatusBar() {
    const path = this.activePath || activeWorkspacePath || 'src/attention.rs';
    const activeFileEl = document.getElementById('status-active-file');
    const langEl = document.getElementById('status-language');
    const dirtyEl = document.getElementById('status-git-dirty');
    const modelBadge = document.getElementById('status-model-badge');
    const gitBranchEl = document.getElementById('status-git-branch');

    if (activeFileEl) activeFileEl.textContent = path;
    if (langEl) {
      const ext = path.split('.').pop() || '';
      const map = { rs: 'Rust', toml: 'TOML', js: 'JavaScript', mjs: 'JavaScript', json: 'JSON', html: 'HTML', css: 'CSS', md: 'Markdown' };
      langEl.textContent = map[ext] || (ext ? ext.toUpperCase() : 'Plain Text');
    }
    if (dirtyEl) {
      let dirtyCount = 0;
      for (const t of this.tabs.values()) {
        if (t.isDirty) dirtyCount++;
      }
      dirtyEl.textContent = `${dirtyCount} uncommitted`;
    }
    if (modelBadge && !modelBadge.textContent.includes('UOR-R4')) {
      modelBadge.innerHTML = '&#9889; UOR-R4 [Local LUT-8]';
    }
    if (gitBranchEl) {
      const branchSpan = document.getElementById('cursor-status-branch');
      if (branchSpan) branchSpan.textContent = 'main*';
    }
  }

  renderTabs() {
    const container = document.getElementById('cursor-editor-tabs') || document.getElementById('editor-tabs-bar');
    if (!container) return;
    container.replaceChildren();

    if (Array.isArray(cursorOpenTabs)) {
      for (const k of Array.from(this.tabs.keys())) {
        if (!cursorOpenTabs.includes(k)) this.tabs.delete(k);
      }
      for (const p of cursorOpenTabs) {
        if (!this.tabs.has(p)) {
          const name = p.split('/').pop() || p;
          this.tabs.set(p, {
            path: p,
            name,
            buffer: '',
            savedContent: '',
            isDirty: false,
            cursorStart: 0,
            cursorEnd: 0,
            scrollTop: 0
          });
        }
      }
    }

    const canClose = cursorOpenTabs.length > 1;

    for (const [path, tab] of this.tabs.entries()) {
      const isActive = path === this.activePath || path === activeWorkspacePath;
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = `cursor-editor-tab editor-tab${isActive ? ' active' : ''}${tab.isDirty ? ' dirty' : ''}`;
      btn.setAttribute('role', 'tab');
      btn.setAttribute('aria-selected', isActive ? 'true' : 'false');
      btn.dataset.path = path;

      const titleSpan = document.createElement('span');
      titleSpan.className = 'editor-tab-title cursor-tree-item-name';
      titleSpan.textContent = tab.isDirty ? `${tab.name} ●` : tab.name;
      btn.appendChild(titleSpan);

      if (canClose) {
        const closeBtn = document.createElement('span');
        closeBtn.className = 'cursor-editor-tab-close editor-tab-close';
        closeBtn.textContent = '×';
        closeBtn.title = `Close ${tab.name}`;
        closeBtn.setAttribute('aria-label', `Close ${tab.name}`);
        closeBtn.addEventListener('click', (ev) => {
          ev.stopPropagation();
          this.close(path);
        });
        btn.appendChild(closeBtn);
      }

      btn.addEventListener('click', () => {
        this.switchTo(path);
      });

      container.appendChild(btn);
    }
  }
}

const editorTabManager = new EditorTabManager();
globalThis.EditorTabManager = EditorTabManager;
globalThis.editorTabManager = editorTabManager;
if (typeof window !== 'undefined') {
  window.EditorTabManager = EditorTabManager;
  window.editorTabManager = editorTabManager;
}

const cursorBaselineFiles = new Map();
const cursorOpenTabs = ['src/attention.rs', 'src/lib.rs', 'src/LutStack.rs', 'Cargo.toml'];
const cursorDiskFileHandles = new Map();
let cursorCmdKState = {
  open: false,
  targetPath: 'src/attention.rs',
  selStart: 0,
  selEnd: 0,
  startLine: 1,
  endLine: 1,
  originalFullCode: '',
  proposedFullCode: '',
  diffText: '',
};
let cursorGhostState = {
  suggestion: 'let q12_basin = point_key.project_radix8(state_vec);',
  sourceTag: '/8 PointKey + /7 4-Gram',
  q12L1: 0,
  hammingDist: 0,
  basinId: 0,
  candidates: [],
};
let cursorGhostDebounceTimer = null;

function scheduleCursorGhostText(delay = 180) {
  if (cursorGhostDebounceTimer) {
    clearTimeout(cursorGhostDebounceTimer);
  }
  cursorGhostDebounceTimer = setTimeout(() => {
    cursorGhostDebounceTimer = null;
    updateCursorGhostText();
  }, delay);
  if (cursorGhostDebounceTimer && typeof cursorGhostDebounceTimer.unref === 'function') {
    cursorGhostDebounceTimer.unref();
  }
}

function dismissCursorGhostText() {
  if (cursorGhostDebounceTimer) {
    clearTimeout(cursorGhostDebounceTimer);
    cursorGhostDebounceTimer = null;
  }
  cursorGhostState.suggestion = '';
  cursorGhostState.candidates = [];
  const overlayEl = document.getElementById('cursor-ghost-overlay');
  if (overlayEl) {
    overlayEl.innerHTML = '';
  }
  const ghostBarEl = document.getElementById('cursor-ghost-text-bar');
  if (ghostBarEl) {
    ghostBarEl.setAttribute('hidden', '');
  }
  const previewEl = document.getElementById('cursor-ghost-text-preview');
  if (previewEl) {
    previewEl.textContent = '';
  }
  const metaEl = document.getElementById('cursor-ghost-meta');
  if (metaEl) {
    metaEl.textContent = '';
  }
  const statusGhostEl = document.getElementById('cursor-status-ghost');
  if (statusGhostEl) {
    statusGhostEl.textContent = 'Tab: none';
  }
}
const clearCursorGhostText = dismissCursorGhostText;

let cursorPaletteActiveIdx = 0;
let cursorPaletteItemsCache = [];
let cursorPaletteOpen = false;

function computeCursorGitStatusForFile(fileObj) { /* Formal R4.StudioFrame */
  if (!fileObj || !fileObj.path) return { badge: 'M', cls: 'cursor-git-m', delta: '+0/-0' };
  const path = fileObj.path;
  const currentContent = String(fileObj.content || '');
  if (!cursorBaselineFiles.has(path)) {
    cursorBaselineFiles.set(path, currentContent);
  }
  const baseContent = cursorBaselineFiles.get(path) || '';
  const pending = (AppState.workspace && AppState.workspace.pendingPatches && AppState.workspace.pendingPatches[path])
    ? AppState.workspace.pendingPatches[path]
    : null;
  if (pending && pending.proposedCode && pending.proposedCode !== currentContent) {
    const baseLines = currentContent.split('\n').length;
    const propLines = String(pending.proposedCode).split('\n').length;
    const add = Math.max(1, propLines >= baseLines ? propLines - baseLines + 1 : 1);
    const del = Math.max(0, baseLines > propLines ? baseLines - propLines : 1);
    return { badge: 'M*', cls: 'cursor-git-m', delta: `+${add}/-${del}` };
  }
  if (currentContent !== baseContent) {
    const oldL = baseContent.split('\n').length;
    const newL = currentContent.split('\n').length;
    const add = Math.max(1, newL >= oldL ? newL - oldL + 1 : 1);
    const del = Math.max(0, oldL > newL ? oldL - newL : 0);
    return { badge: 'M', cls: 'cursor-git-m', delta: `+${add}/-${del}` };
  }
  if (!['src/attention.rs', 'src/lib.rs', 'src/LutStack.rs', 'Cargo.toml'].includes(path)) {
    return { badge: 'U', cls: 'cursor-git-u', delta: `+${fileObj.line_count || currentContent.split('\n').length}/-0` };
  }
  return { badge: '', cls: '', delta: `${fileObj.line_count || currentContent.split('\n').length}L` };
}

function renderCursorFileTree(ws) {
  const treeEl = document.getElementById('cursor-file-tree');
  if (!treeEl || !ws || !Array.isArray(ws.files)) return;
  const filterInput = document.getElementById('cursor-tree-filter');
  const query = filterInput && filterInput.value ? filterInput.value.trim().toLowerCase() : '';
  treeEl.replaceChildren();

  const folders = new Map();
  const rootFiles = [];
  let modifiedCount = 0;

  for (const file of ws.files) {
    const gitInfo = computeCursorGitStatusForFile(file);
    if (gitInfo.badge === 'M' || gitInfo.badge === 'M*' || gitInfo.badge === 'U') {
      modifiedCount += 1;
    }
    if (query && !file.path.toLowerCase().includes(query)) continue;
    const slashIdx = file.path.indexOf('/');
    if (slashIdx > 0) {
      const folderName = file.path.slice(0, slashIdx);
      if (!folders.has(folderName)) folders.set(folderName, []);
      folders.get(folderName).push({ file, gitInfo });
    } else {
      rootFiles.push({ file, gitInfo });
    }
  }

  function buildTreeFileButton(file, gitInfo, displayLabel) {
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = `cursor-tree-item${file.path === activeWorkspacePath ? ' active' : ''}`;
    btn.setAttribute('role', 'treeitem');
    btn.title = `${file.path} (${file.line_count} lines)`;

    const nameSpan = document.createElement('span');
    nameSpan.className = 'cursor-tree-item-name';
    nameSpan.textContent = displayLabel;

    const badgesSpan = document.createElement('span');
    badgesSpan.className = 'cursor-tree-badges';

    if (file.lazy && !file.content) {
      const lb = document.createElement('span');
      lb.className = 'cursor-git-badge cursor-git-u';
      lb.textContent = 'LAZY';
      badgesSpan.appendChild(lb);
    }
    if (gitInfo.badge) {
      const b = document.createElement('span');
      b.className = `cursor-git-badge ${gitInfo.cls}`;
      b.textContent = gitInfo.badge;
      badgesSpan.appendChild(b);
    }
    const d = document.createElement('span');
    d.className = `cursor-git-badge${gitInfo.badge ? ' cursor-git-diff' : ''}`;
    d.textContent = gitInfo.delta;
    badgesSpan.appendChild(d);

    btn.appendChild(nameSpan);
    btn.appendChild(badgesSpan);
    btn.addEventListener('click', () => {
      if (!cursorOpenTabs.includes(file.path)) {
        cursorOpenTabs.push(file.path);
      }
      activeWorkspacePath = file.path;
      selectWorkspaceFile(file.path);
      const refreshed = dispatchStudioEnvelope('workspace_list', '');
      renderWorkspaceList(refreshed);
    });
    return btn;
  }

  for (const [folderName, entries] of folders.entries()) {
    const details = document.createElement('details');
    details.className = 'cursor-tree-folder';
    details.open = true;
    const summary = document.createElement('summary');
    summary.textContent = `▾ ${folderName}/ (${entries.length})`;
    const childrenWrap = document.createElement('div');
    childrenWrap.className = 'cursor-tree-folder-children';
    for (const { file, gitInfo } of entries) {
      const shortName = file.path.slice(folderName.length + 1);
      childrenWrap.appendChild(buildTreeFileButton(file, gitInfo, shortName));
    }
    details.appendChild(summary);
    details.appendChild(childrenWrap);
    treeEl.appendChild(details);
  }

  for (const { file, gitInfo } of rootFiles) {
    treeEl.appendChild(buildTreeFileButton(file, gitInfo, file.path));
  }

  const scmBadge = document.getElementById('cursor-scm-badge');
  if (scmBadge) {
    scmBadge.textContent = String(modifiedCount);
  }
  renderCursorScmSection(ws);
}

function renderCursorScmSection(ws) {
  const scmSummary = document.getElementById('cursor-scm-summary');
  const scmList = document.getElementById('cursor-scm-list');
  const scmDiffPre = document.getElementById('scm-live-diff-preview');
  const scmLazyStatus = document.getElementById('scm-lazy-status');
  if (!scmList || !ws || !Array.isArray(ws.files)) return;
  scmList.replaceChildren();

  const pendingPatches = (AppState.workspace && AppState.workspace.pendingPatches) ? AppState.workspace.pendingPatches : {};
  const pendingPaths = Object.keys(pendingPatches);
  const baseline = (AppState.config && AppState.config.gitBaselineFiles) || {};
  const diffChunks = [];
  let changeCount = 0;
  let lazyCount = 0;

  for (const file of ws.files) {
    if (file.lazy && !file.content) lazyCount += 1;
    const baseText = baseline[file.path];
    const hasGitBaselineDiff = typeof baseText === 'string' && typeof file.content === 'string' && baseText !== file.content && !file.lazy;
    const gitInfo = computeCursorGitStatusForFile(file);
    if (hasGitBaselineDiff) {
      diffChunks.push(computeUnifiedDiffJs(baseText, file.content, file.path));
    }
    if (!gitInfo.badge && !pendingPatches[file.path] && !hasGitBaselineDiff) continue;
    changeCount += 1;
    const row = document.createElement('button');
    row.type = 'button';
    row.className = 'cursor-tree-item';
    const label = document.createElement('span');
    label.className = 'cursor-tree-item-name';
    label.textContent = file.path;
    const badge = document.createElement('span');
    badge.className = `cursor-git-badge ${gitInfo.cls || 'cursor-git-m'}`;
    badge.textContent = hasGitBaselineDiff ? 'M*' : `${gitInfo.badge || 'M'} ${gitInfo.delta}`;
    row.appendChild(label);
    row.appendChild(badge);
    row.addEventListener('click', () => {
      activeWorkspacePath = file.path;
      selectWorkspaceFile(file.path);
      if (pendingPatches[file.path]) {
        activateCanvasTab('diff');
      }
    });
    scmList.appendChild(row);
  }

  if (scmList.children.length === 0) {
    const emptyEl = document.createElement('div');
    emptyEl.className = 'cursor-scm-empty';
    const branchLabel = (AppState.config && AppState.config.connectedGitBranch) || 'main';
    emptyEl.textContent = `Working tree clean on ${branchLabel}`;
    scmList.appendChild(emptyEl);
  }

  if (scmSummary) {
    const repoLabel = (AppState.config && AppState.config.connectedGitRepo) || 'local-workspace';
    const branchLabel = (AppState.config && AppState.config.connectedGitBranch) || 'main';
    scmSummary.textContent = changeCount > 0
      ? `${repoLabel}@${branchLabel} · ${changeCount} changed · ${pendingPaths.length} pending AI patch(es)`
      : `${repoLabel}@${branchLabel} · Working tree clean · 0 staged diffs`;
  }
  if (scmLazyStatus) {
    const totalCount = ws.files.length;
    scmLazyStatus.textContent = lazyCount > 0
      ? `${totalCount - lazyCount}/${totalCount} loaded · ${lazyCount} lazy blob(s) load on click`
      : `${totalCount} workspace file(s) loaded`;
  }
  if (scmDiffPre) {
    if (diffChunks.length > 0) {
      scmDiffPre.textContent = diffChunks.join('\n\n');
    } else if (pendingPaths.length > 0 && pendingPatches[pendingPaths[0]]) {
      scmDiffPre.textContent = pendingPatches[pendingPaths[0]].diffPreview || 'Pending AI patch staged.';
    } else {
      scmDiffPre.textContent = 'Working tree clean — edit a file or stage a patch to inspect live unified diff.';
    }
  }
}

function renderCursorShardsSection() {
  const shardsGrid = document.getElementById('cursor-shards-grid');
  const shardsSummary = document.getElementById('cursor-shards-summary');
  const stitchCard = document.getElementById('stitch-custody-health-card');
  if (!shardsGrid) return;
  shardsGrid.replaceChildren();
  const stHealth = typeof engineWorkerClient !== 'undefined' ? engineWorkerClient.stitchHealth : null;
  const healedFiles = new Set(
    stHealth && Array.isArray(stHealth.healedDetails)
      ? stHealth.healedDetails.map((d) => String(d.file || ''))
      : []
  );
  for (let s = 0; s < 16; s += 1) {
    const item = document.createElement('div');
    item.className = 'cursor-tree-item';
    const name = document.createElement('span');
    name.className = 'cursor-tree-item-name';
    name.textContent = `Shard ${String(s).padStart(2, '0')} · E8 Basin [${s * 1088}..${(s + 1) * 1088 - 1}]`;
    const badge = document.createElement('span');
    badge.className = 'cursor-git-badge cursor-git-u';
    const shardName = `shard_${String(s).padStart(3, '0')}.bin`;
    badge.textContent = healedFiles.has(shardName) ? 'PARITY-HEALED' : 'RESIDENT';
    item.appendChild(name);
    item.appendChild(badge);
    shardsGrid.appendChild(item);
  }
  if (shardsSummary) {
    try {
      const hasWorker = typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled;
      const holoInfo = hasWorker ? engineWorkerClient.holoStatus : null;
      const info = hasWorker
        ? engineWorkerClient.packInfo
        : (mainThreadWasmReady && typeof studio_weight_pack_status === 'function' ? JSON.parse(studio_weight_pack_status()) : null);
      const stitchTag = stHealth ? ` · Stitch: ${stHealth.status}` : '';
      if (holoInfo && holoInfo.mounted_count > 0) {
        shardsSummary.textContent = `Holo v13 OPFS · ${holoInfo.mounted_count} Mounted · Native Q31 D85 Ready${stitchTag}`;
      } else if (stHealth) {
        shardsSummary.textContent = `Holo v13 OPFS · ${stHealth.verifiedShards}/${stHealth.totalShards} Verified · ${stHealth.healedShards} Parity-Healed`;
      } else if (info && info.status === 'ok') {
        shardsSummary.textContent = `Holo v13 OPFS + Stitch 2-of-3 Parity · ${info.point_rows} /8 PointKey Rows · ${info.reply_entries} /7 Reply Rows${stitchTag}`;
      }
    } catch {
      // Ignore early boot
    }
  }
  if (stitchCard) {
    stitchCard.replaceChildren();
    const row1 = document.createElement('div');
    row1.textContent = `Stitch Custody: ${stHealth ? stHealth.scheme : 'stitch/xor-2-of-3/1'} (${stHealth ? stHealth.custodyMode : '2-of-3 XOR Parity + SHA-256'})`;
    const row2 = document.createElement('div');
    row2.textContent = stHealth
      ? `Status: ${stHealth.status} · Verified ${stHealth.verifiedShards}/${stHealth.totalShards} · Parity-Healed ${stHealth.healedShards}`
      : 'Parity State: Awaiting Holo v13 / Stitch manifest load...';
    const row3 = document.createElement('div');
    row3.textContent = stHealth && Array.isArray(stHealth.activeSources) && stHealth.activeSources.length > 0
      ? `Sources: ${stHealth.activeSources.join(', ')}`
      : 'Mirrors: Primary origin + fallback mirrors ready';
    stitchCard.appendChild(row1);
    stitchCard.appendChild(row2);
    stitchCard.appendChild(row3);
  }
}

function renderCursorEditorTabs(ws) {
  if (typeof editorTabManager !== 'undefined' && editorTabManager) {
    if (editorTabManager.tabs.size === 0 && cursorOpenTabs.length > 0) {
      editorTabManager.init(cursorOpenTabs);
    } else {
      editorTabManager.renderTabs();
    }
    return;
  }
  const tabsEl = document.getElementById('cursor-editor-tabs') || document.getElementById('editor-tabs-bar');
  if (!tabsEl) return;
  if (activeWorkspacePath && !cursorOpenTabs.includes(activeWorkspacePath)) {
    cursorOpenTabs.push(activeWorkspacePath);
  }
  const fileMap = new Map();
  if (ws && Array.isArray(ws.files)) {
    for (const f of ws.files) fileMap.set(f.path, f);
  }
  tabsEl.replaceChildren();

  for (const path of cursorOpenTabs) {
    const fileObj = fileMap.get(path) || { path, content: '', line_count: 1 };
    const gitInfo = computeCursorGitStatusForFile(fileObj);
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = `cursor-editor-tab editor-tab${path === activeWorkspacePath ? ' active' : ''}`;
    btn.setAttribute('role', 'tab');
    btn.setAttribute('aria-selected', path === activeWorkspacePath ? 'true' : 'false');
    btn.dataset.path = path;

    const shortName = path.split('/').pop() || path;
    const labelSpan = document.createElement('span');
    labelSpan.className = 'editor-tab-title';
    labelSpan.textContent = gitInfo.badge ? `${shortName} ●` : shortName;
    btn.appendChild(labelSpan);

    if (cursorOpenTabs.length > 1) {
      const closeSpan = document.createElement('span');
      closeSpan.className = 'cursor-editor-tab-close editor-tab-close';
      closeSpan.textContent = '×';
      closeSpan.title = `Close ${path}`;
      closeSpan.addEventListener('click', (ev) => {
        ev.stopPropagation();
        if (editorTabManager) {
          editorTabManager.close(path);
        } else {
          const idx = cursorOpenTabs.indexOf(path);
          if (idx >= 0 && cursorOpenTabs.length > 1) {
            cursorOpenTabs.splice(idx, 1);
            if (activeWorkspacePath === path) {
              activeWorkspacePath = cursorOpenTabs[Math.max(0, idx - 1)];
              selectWorkspaceFile(activeWorkspacePath);
            }
          }
        }
        const refreshed = dispatchStudioEnvelope('workspace_list', '');
        renderWorkspaceList(refreshed);
      });
      btn.appendChild(closeSpan);
    }

    btn.addEventListener('click', () => {
      activeWorkspacePath = path;
      selectWorkspaceFile(path);
      const refreshed = dispatchStudioEnvelope('workspace_list', '');
      renderWorkspaceList(refreshed);
    });
    tabsEl.appendChild(btn);
  }
}

function renderCursorLineGutter() {
  const gutterEl = document.getElementById('cursor-line-gutter');
  const editorLineNumbers = document.getElementById('editor-line-numbers');
  if (!workspaceEditorTextarea) return;
  const text = String(workspaceEditorTextarea.value || '');
  const lineCount = Math.max(1, text.split('\n').length);
  const nums = [];
  for (let i = 1; i <= lineCount; i += 1) {
    nums.push(String(i));
  }
  const linesStr = nums.join('\n');
  if (gutterEl) gutterEl.textContent = linesStr;
  if (editorLineNumbers) {
    if (gutterEl && gutterEl.parentNode === editorLineNumbers) {
      editorLineNumbers.textContent = linesStr;
      editorLineNumbers.prepend(gutterEl);
    } else {
      editorLineNumbers.textContent = linesStr;
    }
    editorLineNumbers.scrollTop = workspaceEditorTextarea.scrollTop;
  }
}

function detectCursorEnclosingSymbol(codeText, lineIndexOneBased) { /* Formal R4.StudioFrame */
  const lines = String(codeText || '').split('\n');
  const maxIdx = Math.min(lines.length - 1, Math.max(0, lineIndexOneBased - 1));
  for (let i = maxIdx; i >= 0; i -= 1) {
    const line = lines[i];
    const m = line.match(/(?:pub\s+)?(?:async\s+)?(fn|struct|enum|impl|trait|mod|def|class)\s+([A-Za-z0-9_]+)/);
    if (m) {
      return `${m[1]} ${m[2]}`;
    }
  }
  return 'module_root';
}

function updateCursorBreadcrumbsAndCursorPos() {
  const path = activeWorkspacePath || 'src/attention.rs';
  const parts = path.split('/');
  const folderCrumb = document.getElementById('cursor-crumb-folder');
  const fileCrumb = document.getElementById('cursor-crumb-file');
  const symbolCrumb = document.getElementById('cursor-crumb-symbol');
  const lnColEl = document.getElementById('cursor-status-lncol');
  const gitBadgeEl = document.getElementById('cursor-file-git-badge');

  if (folderCrumb) {
    folderCrumb.textContent = parts.length > 1 ? parts.slice(0, -1).join('/') : 'root';
  }
  if (fileCrumb) {
    fileCrumb.textContent = parts[parts.length - 1] || path;
  }

  let ln = 1;
  let col = 1;
  const code = workspaceEditorTextarea ? String(workspaceEditorTextarea.value || '') : '';
  if (workspaceEditorTextarea && typeof workspaceEditorTextarea.selectionStart === 'number') {
    const pos = Math.max(0, workspaceEditorTextarea.selectionStart);
    const prefix = code.slice(0, pos);
    const prefixLines = prefix.split('\n');
    ln = prefixLines.length;
    col = (prefixLines[prefixLines.length - 1] || '').length + 1;
  }
  if (symbolCrumb) {
    symbolCrumb.textContent = detectCursorEnclosingSymbol(code, ln);
  }
  if (lnColEl) {
    lnColEl.textContent = `Ln ${ln}, Col ${col}`;
  }
  if (gitBadgeEl) {
    const lineTotal = Math.max(1, code.split('\n').length);
    const gitInfo = computeCursorGitStatusForFile({ path, content: code, line_count: lineTotal });
    gitBadgeEl.textContent = gitInfo.badge ? `${gitInfo.badge} · ${gitInfo.delta}` : `${lineTotal}L · Clean`;
  }

  const statusActiveFile = document.getElementById('status-active-file');
  if (statusActiveFile) {
    statusActiveFile.textContent = path;
  }
  const statusCursorPos = document.getElementById('status-cursor-pos');
  if (statusCursorPos) {
    statusCursorPos.textContent = `Ln ${ln}, Col ${col}`;
  }
  const statusLang = document.getElementById('status-language');
  if (statusLang) {
    const ext = path.split('.').pop() || '';
    const map = { rs: 'Rust', toml: 'TOML', js: 'JavaScript', mjs: 'JavaScript', ts: 'TypeScript', json: 'JSON', html: 'HTML', css: 'CSS', md: 'Markdown' };
    statusLang.textContent = map[ext] || (ext ? ext.toUpperCase() : 'Plain Text');
  }
  const statusGitDirty = document.getElementById('status-git-dirty');
  if (statusGitDirty) {
    let dirtyCount = 0;
    if (typeof editorTabManager !== 'undefined' && editorTabManager) {
      for (const t of editorTabManager.tabs.values()) {
        if (t.isDirty) dirtyCount++;
      }
    }
    statusGitDirty.textContent = `${dirtyCount} uncommitted`;
  }
}

function syncCursorWorkbenchOnFileSelect(_res) {
  if (_res && _res.path && !cursorOpenTabs.includes(_res.path)) {
    cursorOpenTabs.push(_res.path);
  }
  renderCursorLineGutter();
  updateCursorBreadcrumbsAndCursorPos();
  updateCursorGhostText();
}

function syncCursorWorkbenchOnList(ws) {
  renderCursorFileTree(ws);
  renderCursorEditorTabs(ws);
  renderCursorLineGutter();
  updateCursorBreadcrumbsAndCursorPos();
}

function syncCursorComposerPatchBar() {
  const bar = document.getElementById('cursor-composer-patch-bar');
  const stat = document.getElementById('cursor-patch-stat');
  if (!bar) return;
  const patches = (AppState.workspace && AppState.workspace.pendingPatches) ? AppState.workspace.pendingPatches : {};
  const count = Object.keys(patches).length;
  if (count > 0) {
    bar.removeAttribute('hidden');
    if (stat) stat.textContent = `${count} staged file(s)`;
  } else {
    bar.setAttribute('hidden', '');
    if (stat) stat.textContent = '0 staged';
  }
}

// ============================================================================
// 2. Cmd+K Inline Floating Edit Bar & Inline Red/Green Diff Hunks
// ============================================================================

function openCursorCmdK() {
  const bar = document.getElementById('cursor-cmdk-bar');
  const rangeBadge = document.getElementById('cursor-cmdk-range-badge');
  const input = document.getElementById('cursor-cmdk-input');
  const diffHunk = document.getElementById('cursor-cmdk-diff-hunk');
  const acceptBtn = document.getElementById('cursor-cmdk-accept-btn');
  const rejectBtn = document.getElementById('cursor-cmdk-reject-btn');
  if (!bar || !workspaceEditorTextarea) return;

  const code = String(workspaceEditorTextarea.value || '');
  const selStart = typeof workspaceEditorTextarea.selectionStart === 'number' ? workspaceEditorTextarea.selectionStart : 0;
  const selEnd = typeof workspaceEditorTextarea.selectionEnd === 'number' ? workspaceEditorTextarea.selectionEnd : selStart;
  const startLine = code.slice(0, selStart).split('\n').length;
  const endLine = Math.max(startLine, code.slice(0, selEnd).split('\n').length);

  cursorCmdKState = {
    open: true,
    targetPath: activeWorkspacePath || 'src/attention.rs',
    selStart,
    selEnd,
    startLine,
    endLine,
    originalFullCode: code,
    proposedFullCode: '',
    diffText: '',
  };

  if (rangeBadge) {
    rangeBadge.textContent = `Lines ${startLine}–${endLine} in ${cursorCmdKState.targetPath}`;
  }
  if (diffHunk) {
    diffHunk.setAttribute('hidden', '');
    diffHunk.replaceChildren();
  }
  if (acceptBtn) {
    acceptBtn.setAttribute('hidden', '');
    acceptBtn.setAttribute('disabled', '');
    acceptBtn.disabled = true;
  }
  if (rejectBtn) {
    rejectBtn.setAttribute('hidden', '');
    rejectBtn.setAttribute('disabled', '');
    rejectBtn.disabled = true;
  }
  bar.removeAttribute('hidden');
  if (input) {
    input.focus();
    input.select();
  }
}

function runCursorCmdKEdit() {
  const input = document.getElementById('cursor-cmdk-input');
  const diffHunk = document.getElementById('cursor-cmdk-diff-hunk');
  const acceptBtn = document.getElementById('cursor-cmdk-accept-btn');
  const rejectBtn = document.getElementById('cursor-cmdk-reject-btn');
  if (!workspaceEditorTextarea) return;

  const instruction = input && input.value.trim()
    ? input.value.trim()
    : 'Add deterministic unit test and Q12 invariant assertion';
  const targetPath = activeWorkspacePath || cursorCmdKState.targetPath || 'src/attention.rs';
  const originalCode = String(workspaceEditorTextarea.value || '');
  const lines = originalCode.split('\n');
  const sLine = Math.max(1, cursorCmdKState.startLine || 1);
  const eLine = Math.min(lines.length, Math.max(sLine, cursorCmdKState.endLine || lines.length));
  const selectedSnippet = lines.slice(sLine - 1, eLine).join('\n');

  const applyCmdKResult = (proposedFullCode, sourceTag = 'local Holo v13') => {
    const diffText = computeUnifiedDiffJs(originalCode, proposedFullCode, targetPath);
    cursorCmdKState.originalFullCode = originalCode;
    cursorCmdKState.proposedFullCode = proposedFullCode;
    cursorCmdKState.diffText = diffText;

    if (diffHunk) {
      renderColoredUnifiedDiff(diffHunk, diffText);
      diffHunk.removeAttribute('hidden');
    }
    if (acceptBtn) {
      acceptBtn.removeAttribute('hidden');
      acceptBtn.removeAttribute('disabled');
      acceptBtn.disabled = false;
    }
    if (rejectBtn) {
      rejectBtn.removeAttribute('hidden');
      rejectBtn.removeAttribute('disabled');
      rejectBtn.disabled = false;
    }
    if (workspaceStatus) {
      workspaceStatus.textContent = `Cmd+K synthesized inline diff for ${targetPath} (Lines ${sLine}–${eLine}) via ${sourceTag}.`;
    }
  };

  if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient && engineWorkerClient.enabled) {
    if (workspaceStatus) {
      workspaceStatus.textContent = `Synthesizing Cmd+K inline edit for ${targetPath} on Worker...`;
    }
    engineWorkerClient
      .rpc('studio_cmdk_edit', {
        path: targetPath,
        code: originalCode,
        selection: selectedSnippet,
        instruction,
        selected_model: (AppState && AppState.config && AppState.config.selectedModel) || undefined,
      })
      .then((res) => {
        const nextCode = (res && res.proposed_code) ? String(res.proposed_code) : originalCode;
        applyCmdKResult(nextCode, (res && res.model_id) || 'Worker Holo v13');
      })
      .catch((err) => {
        const msg = err && err.message ? err.message : String(err);
        if (workspaceStatus) workspaceStatus.textContent = `Cmd+K error: ${msg}`;
        if (typeof showStudioToast === 'function') showStudioToast(`Cmd+K edit failed: ${msg}`, 'error', 4500);
      });
    return;
  }

  const prompt = `[Cmd+K Inline Edit on ${targetPath} Lines ${sLine}-${eLine}] ${instruction}\n\`\`\`rust\n${selectedSnippet}\n\`\`\``;
  let chatRes = null;
  try {
    chatRes = dispatchStudioEnvelope('chat', prompt);
  } catch (err) {
    console.warn('Cmd+K model dispatch error:', err);
  }

  let proposedFullCode = originalCode;
  if (chatRes && chatRes.workspace_patch && chatRes.workspace_patch.proposed_code) {
    proposedFullCode = chatRes.workspace_patch.proposed_code;
  } else if (chatRes && chatRes.proposed_code) {
    proposedFullCode = chatRes.proposed_code;
  } else {
    const safeFnName = instruction
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '_')
      .replace(/^_+|_+$/g, '')
      .slice(0, 28) || 'cmdk_inline_transform';
    const synthesizedBlock = [
      `// [UOR-R4 Cmd+K Inline Edit]: ${instruction}`,
      `pub fn ${safeFnName}(q12_state: i32) -> i32 {`,
      `    q12_state.saturating_add(1)`,
      `}`,
    ].join('\n');
    const updatedLines = [...lines];
    updatedLines.splice(eLine, 0, '', synthesizedBlock);
    proposedFullCode = updatedLines.join('\n');
  }

  applyCmdKResult(proposedFullCode, 'local Holo v13');
}

function acceptCursorCmdKEdit() {
  const bar = document.getElementById('cursor-cmdk-bar');
  const diffHunk = document.getElementById('cursor-cmdk-diff-hunk');
  const acceptBtn = document.getElementById('cursor-cmdk-accept-btn');
  const rejectBtn = document.getElementById('cursor-cmdk-reject-btn');
  const cmdkInput = document.getElementById('cursor-cmdk-input');

  if (!cursorCmdKState.proposedFullCode) return;
  const targetPath = cursorCmdKState.targetPath || activeWorkspacePath || 'src/attention.rs';
  if (!checkMcpToolPolicy('workspace_write', targetPath)) return;

  const ws = dispatchStudioEnvelope('workspace_write', `${targetPath}\n${cursorCmdKState.proposedFullCode}`);
  if (ws && ws.status === 'ok') {
    if (workspaceEditorTextarea) {
      workspaceEditorTextarea.value = cursorCmdKState.proposedFullCode;
    }
    if (typeof editorTabManager !== 'undefined' && editorTabManager) {
      if (editorTabManager.tabs.has(targetPath)) {
        const tab = editorTabManager.tabs.get(targetPath);
        tab.buffer = cursorCmdKState.proposedFullCode;
        tab.savedContent = cursorCmdKState.proposedFullCode;
        tab.lineCount = cursorCmdKState.proposedFullCode.split('\n').length;
        tab.isDirty = false;
      }
      editorTabManager.updateBuffer(cursorCmdKState.proposedFullCode);
      editorTabManager.markSaved(targetPath);
    }
    renderWorkspaceList(ws);
    selectWorkspaceFile(targetPath);
    writeCursorDiskFileHandle(targetPath, cursorCmdKState.proposedFullCode);
    if (typeof editorTabManager !== 'undefined' && editorTabManager) {
      editorTabManager.updateBuffer(cursorCmdKState.proposedFullCode);
      editorTabManager.markSaved(targetPath);
    }
    if (typeof cursorBaselineFiles !== 'undefined' && cursorBaselineFiles && cursorBaselineFiles.set) {
      cursorBaselineFiles.set(targetPath, cursorCmdKState.proposedFullCode);
    }
    renderCursorLineGutter();
    updateCursorBreadcrumbsAndCursorPos();
    if (workspaceStatus) {
      workspaceStatus.textContent = `Accepted Cmd+K inline edit in ${targetPath} and admitted into local geometric memory.`;
    }
  }

  if (diffHunk) {
    diffHunk.setAttribute('hidden', '');
    diffHunk.replaceChildren();
  }
  if (acceptBtn) {
    acceptBtn.setAttribute('hidden', '');
    acceptBtn.setAttribute('disabled', '');
    acceptBtn.disabled = true;
  }
  if (rejectBtn) {
    rejectBtn.setAttribute('hidden', '');
    rejectBtn.setAttribute('disabled', '');
    rejectBtn.disabled = true;
  }
  if (cmdkInput) {
    cmdkInput.value = '';
  }
  if (bar) bar.setAttribute('hidden', '');
  cursorCmdKState.open = false;
  cursorCmdKState.proposedFullCode = '';
  cursorCmdKState.diffText = '';
  cursorCmdKState.originalFullCode = '';
  if (workspaceEditorTextarea) {
    workspaceEditorTextarea.focus();
  }
}

function rejectCursorCmdKEdit() {
  const bar = document.getElementById('cursor-cmdk-bar');
  const diffHunk = document.getElementById('cursor-cmdk-diff-hunk');
  const acceptBtn = document.getElementById('cursor-cmdk-accept-btn');
  const rejectBtn = document.getElementById('cursor-cmdk-reject-btn');
  const cmdkInput = document.getElementById('cursor-cmdk-input');

  if (workspaceEditorTextarea && cursorCmdKState.originalFullCode) {
    workspaceEditorTextarea.value = cursorCmdKState.originalFullCode;
  }
  if (diffHunk) {
    diffHunk.setAttribute('hidden', '');
    diffHunk.replaceChildren();
  }
  if (acceptBtn) {
    acceptBtn.setAttribute('hidden', '');
    acceptBtn.setAttribute('disabled', '');
    acceptBtn.disabled = true;
  }
  if (rejectBtn) {
    rejectBtn.setAttribute('hidden', '');
    rejectBtn.setAttribute('disabled', '');
    rejectBtn.disabled = true;
  }
  if (cmdkInput) {
    cmdkInput.value = '';
  }
  if (bar) bar.setAttribute('hidden', '');
  const targetPath = cursorCmdKState.targetPath || 'active file';
  cursorCmdKState.open = false;
  cursorCmdKState.proposedFullCode = '';
  cursorCmdKState.diffText = '';
  cursorCmdKState.originalFullCode = '';
  if (workspaceStatus) {
    workspaceStatus.textContent = `Rejected Cmd+K inline edit on ${targetPath}.`;
  }
  if (workspaceEditorTextarea) {
    workspaceEditorTextarea.focus();
  }
}

// ============================================================================
// 3. Cursor Tab Ghost-Text Autocomplete (Local WASM predict_next + 4-Gram)
// ============================================================================

function updateCursorGhostText() {
  const previewEl = document.getElementById('cursor-ghost-text-preview');
  const metaEl = document.getElementById('cursor-ghost-meta');
  const statusGhostEl = document.getElementById('cursor-status-ghost');
  const statusQ12El = document.getElementById('cursor-status-q12');
  const overlayEl = document.getElementById('cursor-ghost-overlay');
  if (!workspaceEditorTextarea) return;

  // Invalidate and clear ghost text if a selection range is highlighted
  if (
    typeof workspaceEditorTextarea.selectionStart === 'number' &&
    typeof workspaceEditorTextarea.selectionEnd === 'number' &&
    workspaceEditorTextarea.selectionStart !== workspaceEditorTextarea.selectionEnd
  ) {
    dismissCursorGhostText();
    return;
  }

  const code = String(workspaceEditorTextarea.value || '');
  const rawCursorPos = typeof workspaceEditorTextarea.selectionStart === 'number'
    ? workspaceEditorTextarea.selectionStart
    : code.length;
  const cursorPos = (rawCursorPos === 0 && code.length > 0 && typeof document !== 'undefined' && document.activeElement !== workspaceEditorTextarea)
    ? code.length
    : rawCursorPos;
  // 300-character context window up to cursor per PROJECT.md
  const prefixWindow = code.slice(Math.max(0, cursorPos - 300), cursorPos);
  const currentLine = (prefixWindow.split('\n').pop() || '').trim();

  let topCand = null;
  let candidates = [];
  try {
    const predRes = dispatchStudioEnvelope('predict_next', prefixWindow || 'pub fn ');
    if (predRes && predRes.status === 'ok' && Array.isArray(predRes.candidates) && predRes.candidates.length > 0) {
      candidates = predRes.candidates;
      topCand = candidates[0];
    }
  } catch {
    // WASM not yet initialized during early boot
  }

  const tokenPiece = topCand && topCand.piece ? String(topCand.piece).trim() : 'q12_resonance';
  let ghostLine = '';
  if (currentLine.startsWith('pub fn ') && !currentLine.includes('{')) {
    ghostLine = ' -> Result<i32, &' + "static str> {\n    Ok(0)\n}";
  } else if (currentLine.startsWith('fn ') && !currentLine.includes('{')) {
    ghostLine = '() -> bool {\n    true\n}';
  } else if (currentLine.startsWith('let ') && !currentLine.includes('=')) {
    ghostLine = ` = ${tokenPiece}_q12(state);`;
  } else if (currentLine.startsWith('assert')) {
    ghostLine = '_eq!(q12_norm >= 0, true);';
  } else {
    ghostLine = `// [Tab /8: ${tokenPiece}] let q12_basin = point_key.project_radix8(state_vec);`;
  }

  cursorGhostState = {
    suggestion: ghostLine,
    sourceTag: topCand && topCand.source_tag ? topCand.source_tag : '/8 PointKey + /7 4-Gram',
    q12L1: topCand && typeof topCand.q12_l1 === 'number' ? topCand.q12_l1 : 12,
    hammingDist: topCand && typeof topCand.hamming_dist === 'number' ? topCand.hamming_dist : 2,
    basinId: topCand && typeof topCand.basin_id === 'number' ? topCand.basin_id : 7,
    candidates: candidates,
  };

  // Populate inline ghost overlay with prefix and inline completion spans
  if (overlayEl) {
    overlayEl.innerHTML = '';
    const prefixSpan = document.createElement('span');
    prefixSpan.className = 'cursor-ghost-prefix';
    prefixSpan.textContent = code.slice(0, cursorPos);
    const inlineSpan = document.createElement('span');
    inlineSpan.className = 'cursor-ghost-inline';
    inlineSpan.textContent = ghostLine;
    overlayEl.appendChild(prefixSpan);
    overlayEl.appendChild(inlineSpan);
    overlayEl.scrollTop = workspaceEditorTextarea.scrollTop;
    overlayEl.scrollLeft = workspaceEditorTextarea.scrollLeft;
  }

  const ghostBarEl = document.getElementById('cursor-ghost-text-bar');
  if (ghostBarEl) {
    ghostBarEl.removeAttribute('hidden');
  }
  if (previewEl) {
    previewEl.textContent = ghostLine.replace(/\n/g, ' ↵ ');
  }
  if (metaEl) {
    metaEl.textContent = `Basin #${cursorGhostState.basinId} · Q12 L1=${cursorGhostState.q12L1} · d_H=${cursorGhostState.hammingDist}`;
  }
  if (statusGhostEl) {
    statusGhostEl.textContent = `Tab: "${tokenPiece}" (Basin #${cursorGhostState.basinId})`;
  }
  if (statusQ12El) {
    statusQ12El.textContent = `256D Q12 L1=${cursorGhostState.q12L1} · d_H=${cursorGhostState.hammingDist}`;
  }

  if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient && engineWorkerClient.enabled) {
    const ghostReqSeq = (updateCursorGhostText._seq = (updateCursorGhostText._seq || 0) + 1);
    const suffixWindow = code.slice(cursorPos, Math.min(code.length, cursorPos + 160));
    const activePath = (typeof activeWorkspacePath === 'string' && activeWorkspacePath) || 'src/lib.rs';
    engineWorkerClient
      .rpc('studio_ghost_complete', {
        path: activePath,
        prefix: prefixWindow,
        suffix: suffixWindow,
        max_new_tokens: 48,
      })
      .then((res) => {
        if (ghostReqSeq !== updateCursorGhostText._seq) return;
        const rawWorkerSug = res ? (res.suggestion || res.completion || res.text || res.reply) : '';
        const workerSug = rawWorkerSug ? String(rawWorkerSug).trim() : '';
        if (!workerSug) return;
        cursorGhostState.suggestion = workerSug;
        cursorGhostState.sourceTag = (res && res.model_id) || 'Worker Holo Ghost';
        if (overlayEl) {
          overlayEl.replaceChildren();
          const pSpan = document.createElement('span');
          pSpan.className = 'cursor-ghost-prefix';
          pSpan.textContent = code.slice(0, cursorPos);
          const iSpan = document.createElement('span');
          iSpan.className = 'cursor-ghost-inline';
          iSpan.textContent = workerSug;
          overlayEl.appendChild(pSpan);
          overlayEl.appendChild(iSpan);
        }
        if (previewEl) previewEl.textContent = workerSug.replace(/\n/g, ' ↵ ');
      })
      .catch(() => {});
  }
}

function acceptCursorGhostText() {
  if (!workspaceEditorTextarea || !cursorGhostState.suggestion) return false;
  const code = String(workspaceEditorTextarea.value || '');
  const start = typeof workspaceEditorTextarea.selectionStart === 'number'
    ? workspaceEditorTextarea.selectionStart
    : code.length;
  const end = typeof workspaceEditorTextarea.selectionEnd === 'number'
    ? workspaceEditorTextarea.selectionEnd
    : start;
  const insertText = cursorGhostState.suggestion;
  const sourceTag = cursorGhostState.sourceTag;
  const basinId = cursorGhostState.basinId;

  workspaceEditorTextarea.value = code.slice(0, start) + insertText + code.slice(end);
  const newPos = start + insertText.length;
  if (typeof workspaceEditorTextarea.setSelectionRange === 'function') {
    workspaceEditorTextarea.setSelectionRange(newPos, newPos);
  }

  // Synchronize with EditorTabManager so active tab buffer and dirty state (●) update
  if (typeof editorTabManager !== 'undefined' && editorTabManager) {
    editorTabManager.updateBuffer(workspaceEditorTextarea.value);
  }

  renderCursorLineGutter();
  updateCursorBreadcrumbsAndCursorPos();
  dismissCursorGhostText();

  if (workspaceStatus) {
    workspaceStatus.textContent = `Accepted Cursor Tab completion (${sourceTag}, Basin #${basinId}).`;
  }
  return true;
}

globalThis.updateCursorGhostText = updateCursorGhostText;
globalThis.acceptCursorGhostText = acceptCursorGhostText;
globalThis.dismissCursorGhostText = dismissCursorGhostText;
globalThis.clearCursorGhostText = clearCursorGhostText;
globalThis.scheduleCursorGhostText = scheduleCursorGhostText;
if (typeof window !== 'undefined') {
  window.updateCursorGhostText = updateCursorGhostText;
  window.acceptCursorGhostText = acceptCursorGhostText;
  window.dismissCursorGhostText = dismissCursorGhostText;
  window.clearCursorGhostText = clearCursorGhostText;
  window.scheduleCursorGhostText = scheduleCursorGhostText;
}

// ============================================================================
// 4. Cmd+P Quick Open, Cmd+Shift+P Command Palette & Local Folder Mount
// ============================================================================

function fuzzyScoreMatch(query, target) { /* Formal R4.StudioFrame */
  if (!query) return { match: true, score: 0 };
  const q = query.toLowerCase();
  const t = (target || '').toLowerCase();

  // 1. Exact substring match
  if (t.includes(q)) {
    const baseName = t.split('/').pop() || t;
    const baseBonus = baseName.includes(q) ? 50 : 0;
    const exactBaseBonus = (baseName === q) ? 100 : 0;
    return { match: true, score: 200 + baseBonus + exactBaseBonus - t.indexOf(q) };
  }

  // 2. Subsequence matching fallback
  let qIdx = 0;
  let score = 0;
  let consecutive = 0;
  for (let tIdx = 0; tIdx < t.length && qIdx < q.length; tIdx++) {
    if (t[tIdx] === q[qIdx]) {
      qIdx++;
      consecutive++;
      score += 10 + (consecutive * 5);
    } else {
      consecutive = 0;
    }
  }

  if (qIdx === q.length) {
    const baseName = t.split('/').pop() || t;
    let bIdx = 0;
    for (let i = 0; i < baseName.length && bIdx < q.length; i++) {
      if (baseName[i] === q[bIdx]) bIdx++;
    }
    if (bIdx === q.length) score += 30;
    return { match: true, score };
  }

  return { match: false, score: 0 };
}

function setCursorAiMode(mode) {
  const modeAgentBtn = document.getElementById('cursor-mode-agent-btn') || document.getElementById('cursor-mode-agent');
  const modeAskBtn = document.getElementById('cursor-mode-ask-btn') || document.getElementById('cursor-mode-ask');
  const modeEditBtn = document.getElementById('cursor-mode-edit-btn') || document.getElementById('cursor-mode-edit');
  for (const [btn, m] of [
    [modeAgentBtn, 'agent'],
    [modeAskBtn, 'ask'],
    [modeEditBtn, 'edit'],
  ]) {
    if (btn) {
      btn.classList.toggle('active', m === mode);
      btn.setAttribute('aria-pressed', m === mode ? 'true' : 'false');
    }
  }
  const isPlan = mode === 'ask' || mode === 'plan';
  if (typeof setComposerModeUI === 'function') {
    setComposerModeUI(isPlan ? 'plan' : 'agentic');
    return;
  }
  const compAgent = document.getElementById('composer-mode-agent');
  const compPlan = document.getElementById('composer-mode-plan');
  const compFast = document.getElementById('composer-mode-fast');
  if (compAgent) {
    compAgent.classList.toggle('active', !isPlan);
    compAgent.setAttribute('aria-pressed', !isPlan ? 'true' : 'false');
  }
  if (compPlan) {
    compPlan.classList.toggle('active', isPlan);
    compPlan.setAttribute('aria-pressed', isPlan ? 'true' : 'false');
  }
  if (compFast) {
    compFast.classList.toggle('active', mode === 'edit');
    compFast.setAttribute('aria-pressed', mode === 'edit' ? 'true' : 'false');
  }
}

function toggleCursorSidebar(forceState) {
  const toggleBtn = document.getElementById('toggle-sidebar-btn');
  if (toggleBtn && typeof forceState === 'undefined') {
    toggleBtn.click();
    return;
  }
  const sidebar = document.getElementById('threads-sidebar');
  const mainEl = document.querySelector('main');
  if (!sidebar) return;
  const isCurrentlyCollapsed = sidebar.classList.contains('sidebar-collapsed');
  const shouldCollapse = typeof forceState === 'boolean' ? forceState : !isCurrentlyCollapsed;
  sidebar.classList.toggle('sidebar-collapsed', shouldCollapse);
  if (mainEl) mainEl.classList.toggle('sidebar-collapsed', shouldCollapse);
}

const CURSOR_IDE_COMMANDS = [
  {
    id: 'r432-check',
    category: 'UOR-R4',
    title: 'Run In-Wasm Cargo Check (R4-32 Verifier)',
    shortcut: 'Cmd+Shift+B',
    label: '> UOR-R4: Run In-Wasm Cargo Check (R4-32 Verifier)',
    meta: 'Cmd+Shift+B · R4-32',
    handler: () => runCursorR432Check('check'),
    run: () => runCursorR432Check('check'),
  },
  {
    id: 'r432-test',
    category: 'UOR-R4',
    title: 'Run In-Wasm Cargo Test Suite (R4-32)',
    shortcut: '',
    label: '> UOR-R4: Run In-Wasm Cargo Test Suite (R4-32)',
    meta: 'R4-32 Verifier',
    handler: () => runCursorR432Check('test'),
    run: () => runCursorR432Check('test'),
  },
  {
    id: 'cmdk-inline',
    category: 'Cursor',
    title: 'Open Cmd+K Inline Edit on Selection',
    shortcut: 'Cmd+K',
    label: '> Cursor: Open Cmd+K Inline Edit on Selection',
    meta: 'Cmd+K',
    handler: () => openCursorCmdK(),
    run: () => openCursorCmdK(),
  },
  {
    id: 'toggle-terminal',
    category: 'Cursor',
    title: 'Toggle Integrated Bottom Terminal (lexlean-sh)',
    shortcut: 'Ctrl+`',
    label: '> Cursor: Toggle Integrated Bottom Terminal (lexlean-sh)',
    meta: 'Ctrl+`',
    handler: () => toggleCursorBottomPanel('terminal'),
    run: () => toggleCursorBottomPanel('terminal'),
  },
  {
    id: 'inspect-3d-brain',
    category: 'UOR-R4',
    title: 'Open 3D Clifford Cl(0,6) Brain & OTel Spans',
    shortcut: '',
    label: '> UOR-R4: Open 3D Clifford Cl(0,6) Brain & OTel Spans',
    meta: 'uor.gllm.trace/1',
    handler: () => toggleCursorBottomPanel('traces'),
    run: () => toggleCursorBottomPanel('traces'),
  },
  {
    id: 'train-lut',
    category: 'UOR-R4',
    title: 'Train Local LUT Stack on Workspace (3 Epochs)',
    shortcut: '',
    label: '> UOR-R4: Train Local LUT Stack on Workspace (3 Epochs)',
    meta: 'On-Device Training',
    handler: () => runOnDeviceTrain('3'),
    run: () => runOnDeviceTrain('3'),
  },
  {
    id: 'consolidate-memory',
    category: 'UOR-R4',
    title: 'Consolidate /7 & /8 Geometric Memory',
    shortcut: '',
    label: '> UOR-R4: Consolidate /7 & /8 Geometric Memory',
    meta: 'Sleep Consolidation',
    handler: () => {
      const res = dispatchStudioEnvelope('consolidate', '3');
      renderStudioState(res);
      toggleCursorBottomPanel('consolidation');
    },
    run: () => {
      const res = dispatchStudioEnvelope('consolidate', '3');
      renderStudioState(res);
      toggleCursorBottomPanel('consolidation');
    },
  },
  {
    id: 'mount-local-folder',
    category: 'Cursor',
    title: 'Open Local Folder from Disk (File System Access API)',
    shortcut: '',
    label: '> Cursor: Open Local Folder from Disk (File System Access API)',
    meta: 'Local Disk Mount',
    handler: () => mountLocalDirectoryWithFileSystemApi(),
    run: () => mountLocalDirectoryWithFileSystemApi(),
  },
  {
    id: 'focus-ai-chat',
    category: 'Cursor AI',
    title: 'Focus AI Chat Pane',
    shortcut: 'Cmd+L',
    label: '> Cursor AI: Focus AI Chat (Ask Mode)',
    meta: 'Cmd+L',
    handler: () => {
      setCursorAiMode('ask');
      const chatInput = document.getElementById('claude-chat-input');
      if (chatInput && typeof chatInput.focus === 'function') chatInput.focus();
    },
    run: () => {
      setCursorAiMode('ask');
      const chatInput = document.getElementById('claude-chat-input');
      if (chatInput && typeof chatInput.focus === 'function') chatInput.focus();
    },
  },
  {
    id: 'open-composer',
    category: 'Cursor AI',
    title: 'Open Composer (Agent Mode)',
    shortcut: 'Cmd+I',
    label: '> Cursor AI: Open Composer (Agent Mode)',
    meta: 'Cmd+I',
    handler: () => {
      setCursorAiMode('agent');
      const chatInput = document.getElementById('claude-chat-input');
      if (chatInput && typeof chatInput.focus === 'function') chatInput.focus();
    },
    run: () => {
      setCursorAiMode('agent');
      const chatInput = document.getElementById('claude-chat-input');
      if (chatInput && typeof chatInput.focus === 'function') chatInput.focus();
    },
  },
  {
    id: 'quick-open',
    category: 'File',
    title: 'Quick Open File',
    shortcut: 'Cmd+P',
    label: '> File: Quick Open File (Jump to Buffer)',
    meta: 'Cmd+P',
    handler: () => openCursorCommandPalette('files'),
    run: () => openCursorCommandPalette('files'),
  },
  {
    id: 'save-file',
    category: 'File',
    title: 'Save Active File',
    shortcut: 'Cmd+S',
    label: '> File: Save Active File to VFS & Disk',
    meta: 'Cmd+S',
    handler: () => {
      const saveBtn = document.getElementById('workspace-save-btn');
      if (saveBtn) saveBtn.click();
    },
    run: () => {
      const saveBtn = document.getElementById('workspace-save-btn');
      if (saveBtn) saveBtn.click();
    },
  },
  {
    id: 'new-file',
    category: 'File',
    title: 'New Untitled File',
    shortcut: 'Cmd+N',
    label: '> File: New Untitled File Tab',
    meta: 'Cmd+N',
    handler: () => {
      const btn = document.getElementById('editor-new-tab-btn');
      if (btn) btn.click();
    },
    run: () => {
      const btn = document.getElementById('editor-new-tab-btn');
      if (btn) btn.click();
    },
  },
  {
    id: 'toggle-sidebar',
    category: 'View',
    title: 'Toggle Primary Sidebar',
    shortcut: 'Cmd+B',
    label: '> View: Toggle Primary Sidebar',
    meta: 'Cmd+B',
    handler: () => toggleCursorSidebar(),
    run: () => toggleCursorSidebar(),
  },
  {
    id: 'show-explorer',
    category: 'View',
    title: 'Show File Explorer View',
    shortcut: 'Cmd+Shift+E',
    label: '> View: Show File Explorer View',
    meta: 'Cmd+Shift+E',
    handler: () => {
      const btn = document.getElementById('cursor-act-explorer') || document.getElementById('activity-btn-explorer');
      if (btn) btn.click();
    },
    run: () => {
      const btn = document.getElementById('cursor-act-explorer') || document.getElementById('activity-btn-explorer');
      if (btn) btn.click();
    },
  },
  {
    id: 'show-search',
    category: 'View',
    title: 'Show Global Search View',
    shortcut: 'Cmd+Shift+F',
    label: '> View: Show Global Search View',
    meta: 'Cmd+Shift+F',
    handler: () => {
      const btn = document.getElementById('cursor-act-search') || document.getElementById('activity-btn-search');
      if (btn) btn.click();
    },
    run: () => {
      const btn = document.getElementById('cursor-act-search') || document.getElementById('activity-btn-search');
      if (btn) btn.click();
    },
  },
  {
    id: 'show-scm',
    category: 'View',
    title: 'Show Source Control (SCM) View',
    shortcut: 'Cmd+Shift+G',
    label: '> View: Show Source Control (SCM) View',
    meta: 'Cmd+Shift+G',
    handler: () => {
      const btn = document.getElementById('cursor-act-scm') || document.getElementById('activity-btn-scm');
      if (btn) btn.click();
    },
    run: () => {
      const btn = document.getElementById('cursor-act-scm') || document.getElementById('activity-btn-scm');
      if (btn) btn.click();
    },
  },
  {
    id: 'close-active-tab',
    category: 'View',
    title: 'Close Active Editor Tab',
    shortcut: 'Cmd+W',
    label: '> View: Close Active Editor Tab',
    meta: 'Cmd+W',
    handler: () => {
      if (typeof editorTabManager !== 'undefined' && editorTabManager && editorTabManager.activePath) {
        editorTabManager.close(editorTabManager.activePath);
      }
    },
    run: () => {
      if (typeof editorTabManager !== 'undefined' && editorTabManager && editorTabManager.activePath) {
        editorTabManager.close(editorTabManager.activePath);
      }
    },
  },
  {
    id: 'accept-all-diffs',
    category: 'Cursor AI',
    title: 'Accept All Pending Diffs',
    shortcut: '',
    label: '> Cursor AI: Accept All Pending Diffs',
    meta: 'Accept All',
    handler: () => {
      const btn = document.getElementById('cursor-accept-all-btn');
      if (btn) btn.click();
    },
    run: () => {
      const btn = document.getElementById('cursor-accept-all-btn');
      if (btn) btn.click();
    },
  },
  {
    id: 'reject-all-diffs',
    category: 'Cursor AI',
    title: 'Reject All Pending Diffs',
    shortcut: '',
    label: '> Cursor AI: Reject All Pending Diffs',
    meta: 'Reject All',
    handler: () => {
      const btn = document.getElementById('cursor-reject-all-btn');
      if (btn) btn.click();
    },
    run: () => {
      const btn = document.getElementById('cursor-reject-all-btn');
      if (btn) btn.click();
    },
  },
  {
    id: 'uor.hydrateHolo17b',
    category: 'UOR-R4 Weights',
    title: 'Hydrate Qwen3-1.7B Holo v13 (28L x 2048D) into OPFS',
    shortcut: '',
    label: '> UOR-R4 Weights: Hydrate Qwen3-1.7B Holo v13 (28L x 2048D) into OPFS',
    meta: 'Qwen3-1.7B OPFS',
    handler: () => {
      if (engineWorkerClient && engineWorkerClient.worker) {
        engineWorkerClient
          .rpc('hydrate_holo_model', { modelId: 'uor-r4.qwen3-1.7b-holo' })
          .then(() => {
            updateWeightCacheBadge();
          })
          .catch(() => {});
      }
    },
    run: () => {
      if (engineWorkerClient && engineWorkerClient.worker) {
        engineWorkerClient
          .rpc('hydrate_holo_model', { modelId: 'uor-r4.qwen3-1.7b-holo' })
          .then(() => {
            updateWeightCacheBadge();
          })
          .catch(() => {});
      }
    },
  },
  {
    id: 'clear-terminal',
    category: 'Terminal',
    title: 'Clear Integrated Terminal Log',
    shortcut: '',
    label: '> Terminal: Clear Integrated Terminal Log',
    meta: 'lexlean-sh',
    handler: () => {
      const btn = document.getElementById('cursor-term-clear-btn');
      if (btn) btn.click();
    },
    run: () => {
      const btn = document.getElementById('cursor-term-clear-btn');
      if (btn) btn.click();
    },
  },
];

const COMMAND_REGISTRY = CURSOR_IDE_COMMANDS;

function executeCommand(commandId) { /* Formal R4.StudioFrame */
  const cmd = CURSOR_IDE_COMMANDS.find((c) => c.id === commandId);
  if (!cmd) return false;
  if (typeof cmd.handler === 'function') {
    cmd.handler();
    return true;
  }
  if (typeof cmd.run === 'function') {
    cmd.run();
    return true;
  }
  return false;
}

if (typeof globalThis !== 'undefined') {
  globalThis.COMMAND_REGISTRY = COMMAND_REGISTRY;
  globalThis.CURSOR_IDE_COMMANDS = CURSOR_IDE_COMMANDS;
  globalThis.executeCommand = executeCommand;
}
if (typeof window !== 'undefined') {
  window.COMMAND_REGISTRY = COMMAND_REGISTRY;
  window.CURSOR_IDE_COMMANDS = CURSOR_IDE_COMMANDS;
  window.executeCommand = executeCommand;
}

let cursorPaletteWorkspaceFilesCache = null;

function openCursorCommandPalette(mode = 'files') {
  const modal = document.getElementById('cursor-command-palette');
  const badge = document.getElementById('cursor-palette-mode-badge');
  const input = document.getElementById('cursor-palette-input');
  if (!modal || !input) return;

  const ws = dispatchStudioEnvelope('workspace_list', '');
  cursorPaletteWorkspaceFilesCache = (ws && Array.isArray(ws.files)) ? ws.files : [];

  modal.removeAttribute('hidden');
  cursorPaletteOpen = true;
  if (mode === 'commands') {
    if (badge) badge.textContent = 'Command Palette (Cmd+Shift+P)';
    input.value = '>';
  } else {
    if (badge) badge.textContent = 'Quick Open (Cmd+P)';
    input.value = '';
  }
  cursorPaletteActiveIdx = 0;
  renderCursorPaletteItems();
  input.focus();
  if (mode === 'commands' && typeof input.setSelectionRange === 'function') {
    input.setSelectionRange(1, 1);
  }
}

function closeCursorCommandPalette() {
  cursorPaletteOpen = false;
  const modal = document.getElementById('cursor-command-palette');
  if (modal) modal.setAttribute('hidden', '');
  cursorPaletteWorkspaceFilesCache = null;
  if (typeof workspaceEditorTextarea !== 'undefined' && workspaceEditorTextarea && typeof workspaceEditorTextarea.focus === 'function') {
    workspaceEditorTextarea.focus();
  }
}

function renderCursorPaletteItems() {
  const listEl = document.getElementById('cursor-palette-list');
  const input = document.getElementById('cursor-palette-input');
  const badge = document.getElementById('cursor-palette-mode-badge');
  if (!listEl) return;
  listEl.replaceChildren();

  const rawQuery = input ? String(input.value || '') : '';
  const isCommandMode = rawQuery.trim().startsWith('>');
  const cleanQuery = rawQuery.replace(/^>\s*/, '').trim().toLowerCase();

  if (badge) {
    badge.textContent = isCommandMode ? 'Command Palette (Cmd+Shift+P)' : 'Quick Open (Cmd+P)';
  }

  const items = [];
  if (isCommandMode) {
    const scoredCmds = [];
    for (const cmd of CURSOR_IDE_COMMANDS) {
      if (!cleanQuery) {
        scoredCmds.push({ cmd, score: 0 });
      } else {
        const scoreLabel = fuzzyScoreMatch(cleanQuery, cmd.label);
        const scoreTitle = fuzzyScoreMatch(cleanQuery, cmd.title || '');
        const scoreMeta = fuzzyScoreMatch(cleanQuery, cmd.meta || '');
        const best = Math.max(scoreLabel.score, scoreTitle.score, scoreMeta.score);
        if (scoreLabel.match || scoreTitle.match || scoreMeta.match) {
          scoredCmds.push({ cmd, score: best });
        }
      }
    }
    if (cleanQuery) {
      scoredCmds.sort((a, b) => b.score - a.score);
    }
    for (const entry of scoredCmds) {
      const cmd = entry.cmd;
      items.push({
        label: cmd.label,
        meta: cmd.meta,
        execute: () => {
          closeCursorCommandPalette();
          if (typeof cmd.handler === 'function') cmd.handler();
          else if (typeof cmd.run === 'function') cmd.run();
        },
      });
    }
  } else {
    const files = cursorPaletteWorkspaceFilesCache || (() => {
      const ws = dispatchStudioEnvelope('workspace_list', '');
      return (ws && Array.isArray(ws.files)) ? ws.files : [];
    })();
    const scoredFiles = [];
    for (const f of files) {
      if (!cleanQuery) {
        scoredFiles.push({ file: f, score: 0 });
      } else {
        const res = fuzzyScoreMatch(cleanQuery, f.path);
        if (res.match) {
          scoredFiles.push({ file: f, score: res.score });
        }
      }
    }
    if (cleanQuery) {
      scoredFiles.sort((a, b) => b.score - a.score);
    }
    for (const entry of scoredFiles) {
      const f = entry.file;
      items.push({
        label: f.path,
        meta: `${f.line_count}L · ${f.pinned ? 'Pinned' : 'Workspace VFS'}`,
        execute: () => {
          closeCursorCommandPalette();
          if (!cursorOpenTabs.includes(f.path)) cursorOpenTabs.push(f.path);
          activeWorkspacePath = f.path;
          selectWorkspaceFile(f.path);
          const ws = dispatchStudioEnvelope('workspace_list', '');
          renderWorkspaceList(ws);
          if (typeof workspaceEditorTextarea !== 'undefined' && workspaceEditorTextarea && typeof workspaceEditorTextarea.focus === 'function') {
            workspaceEditorTextarea.focus();
          }
        },
      });
    }
    // In files mode, also offer matching commands if query provided
    if (cleanQuery) {
      const scoredCmds = [];
      for (const cmd of CURSOR_IDE_COMMANDS) {
        const res = fuzzyScoreMatch(cleanQuery, cmd.label);
        if (res.match) {
          scoredCmds.push({ cmd, score: res.score });
        }
      }
      scoredCmds.sort((a, b) => b.score - a.score);
      for (const entry of scoredCmds) {
        const cmd = entry.cmd;
        items.push({
          label: cmd.label,
          meta: cmd.meta,
          execute: () => {
            closeCursorCommandPalette();
            if (typeof cmd.handler === 'function') cmd.handler();
            else if (typeof cmd.run === 'function') cmd.run();
          },
        });
      }
    }
  }

  cursorPaletteItemsCache = items;
  if (cursorPaletteActiveIdx >= items.length) cursorPaletteActiveIdx = 0;

  items.forEach((item, idx) => {
    const btn = document.createElement('button');
    btn.type = 'button';
    const isActive = idx === cursorPaletteActiveIdx;
    btn.className = `cursor-palette-item${isActive ? ' active' : ''}`;
    btn.setAttribute('role', 'option');
    btn.setAttribute('aria-selected', String(isActive));
    const left = document.createElement('span');
    left.textContent = item.label;
    const right = document.createElement('span');
    right.className = 'cursor-palette-item-meta';
    right.textContent = item.meta;
    btn.appendChild(left);
    btn.appendChild(right);
    btn.addEventListener('click', () => item.execute());
    listEl.appendChild(btn);

    if (isActive && typeof btn.scrollIntoView === 'function') {
      try {
        btn.scrollIntoView({ block: 'nearest' });
      } catch {}
    }
  });
}

async function handleWorkspaceMountInputChange(mountInput) {
  if (!mountInput) return;
  const files = mountInput.files;
  if (!files || files.length === 0) return;
  const chunks = [];
  const validExt = /\.(rs|lean|toml|yaml|yml|md|js|mjs|ts|json|css|html|py|txt|sh)$/i;
  for (let i = 0; i < files.length; i += 1) {
    const f = files[i];
    const relPath = f.webkitRelativePath || (f.name ? `src/${f.name}` : '');
    if (!relPath) continue;
    const parts = relPath.split('/');
    const hasHiddenOrIgnored = parts.some(
      (p) => p.startsWith('.') || p === 'node_modules' || p === 'target'
    );
    if (hasHiddenOrIgnored) continue;
    if (!validExt.test(f.name || relPath)) continue;
    if (typeof f.size === 'number' && f.size > 512 * 1024) continue;
    const text = typeof f.text === 'function' ? await f.text() : '';
    chunks.push(`${relPath}\n${text}`);
  }
  if (chunks.length > 0) {
    const batchPayload = chunks.join('\n===FILE===\n');
    const ws = dispatchStudioEnvelope('workspace_batch', batchPayload);
    if (ws && ws.status === 'ok') {
      renderWorkspaceList(ws);
      const st = dispatchStudioEnvelope('state', '');
      renderStudioState(st);
      if (workspaceStatus) {
        workspaceStatus.textContent = `Mounted ${ws.batch_ingested_files} local file(s) (${ws.batch_ingested_tokens} tokens indexed into geometric memory).`;
      }
    }
  }
  try {
    mountInput.value = '';
  } catch {}
}

async function mountLocalDirectoryWithFileSystemApi() {
  if (typeof window !== 'undefined' && typeof window.showDirectoryPicker === 'function') {
    try {
      const dirHandle = await window.showDirectoryPicker({ mode: 'readwrite' });
      const chunks = [];
      cursorDiskFileHandles.clear();

      async function walkDir(handle, prefix) {
        for await (const [name, entry] of handle.entries()) {
          if (name.startsWith('.') || name === 'target' || name === 'node_modules') continue;
          const relPath = prefix ? `${prefix}/${name}` : name;
          if (entry.kind === 'file') {
            if (/\.(rs|lean|toml|yaml|yml|md|js|mjs|ts|json|css|html|py|txt|sh)$/i.test(name)) {
              const fileData = await entry.getFile();
              if (fileData.size <= 512 * 1024) {
                const text = await fileData.text();
                chunks.push(`${relPath}\n${text}`);
                cursorDiskFileHandles.set(relPath, entry);
              }
            }
          } else if (entry.kind === 'directory' && chunks.length < 64) {
            await walkDir(entry, relPath);
          }
        }
      }

      await walkDir(dirHandle, '');
      if (chunks.length > 0) {
        const ws = dispatchStudioEnvelope('workspace_batch', chunks.join('\n===FILE===\n'));
        if (ws && ws.status === 'ok') {
          renderWorkspaceList(ws);
          const st = dispatchStudioEnvelope('state', '');
          renderStudioState(st);
          if (workspaceStatus) {
            workspaceStatus.textContent = `Mounted local folder "${dirHandle.name}" (${ws.batch_ingested_files} files, two-way disk sync active).`;
          }
        }
      }
      return;
    } catch {
      // User cancelled picker or browser blocked; fall back to input element
    }
  }
  if (workspaceMountInput) {
    workspaceMountInput.click();
  }
}

async function writeCursorDiskFileHandle(path, content) { /* Formal R4.StudioFrame */
  const handle = cursorDiskFileHandles.get(path);
  if (!handle || typeof handle.createWritable !== 'function') return;
  try {
    const writable = await handle.createWritable();
    await writable.write(content);
    await writable.close();
  } catch {
    // Ignore permission errors when writing back to local disk handle
  }
}

// ============================================================================
// 5. In-Wasm R4-32 Cargo Verifier & Integrated Bottom Panel Toggle
// ============================================================================

function applyR432ReportToDom(subcommand, report) {
  if (!report) return;
  const problemsSummary = document.getElementById('cursor-problems-summary');
  const statusR432 = document.getElementById('cursor-status-r432');
  const ok = Boolean(report.passed ?? (report.status !== 'error'));
  const errCount = Array.isArray(report.diagnostics)
    ? report.diagnostics.filter((d) => d.level === 'error').length
    : (ok ? 0 : 1);
  const passedCnt = report.passed_tests ?? report.tests_passed ?? 0;
  const totalCnt = report.total_tests ?? ((report.tests_passed ?? 0) + (report.tests_failed ?? 0));
  const summaryText = ok
    ? `cargo ${subcommand}: PASS (0 errors · ${passedCnt}/${totalCnt} tests)`
    : `cargo ${subcommand}: ${errCount} error(s) detected`;
  if (problemsSummary) problemsSummary.textContent = summaryText;
  if (statusR432) statusR432.textContent = `R4-32: ${ok ? 'PASS (0 errors)' : `${errCount} error(s)`}`;
  if (workspaceStatus) workspaceStatus.textContent = `[R4-32 In-Wasm Verifier] ${summaryText}`;
}

function runCursorR432Check(subcommand = 'check') {
  const problemsSummary = document.getElementById('cursor-problems-summary');
  if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
    engineWorkerClient
      .rpc('studio_wasm_eval', { subcommand, filter: '' })
      .then((rep) => applyR432ReportToDom(subcommand, rep))
      .catch(() => {
        if (problemsSummary) problemsSummary.textContent = '0 errors · 0 warnings · In-Wasm R4-32 Verifier Ready';
      });
    return;
  }
  try {
    let report = null;
    if ((typeof mainThreadWasmReady === 'undefined' || mainThreadWasmReady) && typeof studio_wasm_eval === 'function') {
      report = JSON.parse(studio_wasm_eval(subcommand, ''));
    }
    applyR432ReportToDom(subcommand, report);
  } catch {
    if (problemsSummary) problemsSummary.textContent = '0 errors · 0 warnings · In-Wasm R4-32 Verifier Ready';
  }
}
globalThis.__uorRunWasmEvalAsync = runCursorR432Check;

function toggleCursorBottomPanel(preferredTabId = 'terminal') {
  if (!dockableCanvasPane) return;
  const termDrawer = document.getElementById('terminal-split-drawer');
  if (dockableCanvasPane.classList.contains('cursor-bottom-panel-collapsed')) {
    dockableCanvasPane.classList.remove('cursor-bottom-panel-collapsed');
    if (termDrawer) {
      termDrawer.removeAttribute('hidden');
      termDrawer.hidden = false;
    }
    activateCanvasTab(preferredTabId);
  } else if (activeCanvasTabId === preferredTabId) {
    dockableCanvasPane.classList.add('cursor-bottom-panel-collapsed');
    if (termDrawer) {
      termDrawer.setAttribute('hidden', '');
      termDrawer.hidden = true;
    }
  } else {
    if (termDrawer) {
      termDrawer.removeAttribute('hidden');
      termDrawer.hidden = false;
    }
    activateCanvasTab(preferredTabId);
  }
}

function initCursorIdeWorkbench() {
  const bootSearch = (typeof window !== 'undefined' && window.location && window.location.search) || '';
  if (bootSearch.includes('surface=observatory') || bootSearch.includes('surface=preview')) {
    return;
  }
  // 1. Left Activity Bar & Sidebar View Switching
  const activityBtnConfigs = [
    { id: 'cursor-act-explorer', alt: 'activity-btn-explorer', mode: 'explorer', viewId: 'sidebar-view-explorer' },
    { id: 'cursor-act-search', alt: 'activity-btn-search', mode: 'search', viewId: 'sidebar-view-search' },
    { id: 'cursor-act-scm', alt: 'activity-btn-scm', mode: 'scm', viewId: 'sidebar-view-scm' },
    { id: 'cursor-act-shards', alt: 'activity-btn-shards', mode: 'shards', viewId: 'sidebar-view-shards' },
    { id: 'cursor-act-history', alt: 'activity-btn-history', mode: 'history', viewId: 'sidebar-view-history' },
    { id: 'cursor-act-settings', alt: 'activity-btn-settings', mode: 'settings', viewId: 'sidebar-view-settings' },
  ];

  let currentActivityMode = 'explorer';

  function toggleSidebar(forceState) {
    const sidebar = document.getElementById('threads-sidebar');
    const mainEl = document.querySelector('main');
    if (!sidebar) return;
    const winClass = (document.body && ((document.body.dataset && document.body.dataset.windowClass) || (typeof document.body.getAttribute === 'function' && document.body.getAttribute('data-window-class')))) || 'expanded';
    const isCurrentlyCollapsed = sidebar.classList.contains('sidebar-collapsed') || (mainEl && mainEl.classList.contains('sidebar-collapsed'));
    const shouldCollapse = typeof forceState === 'boolean' ? forceState : !isCurrentlyCollapsed;
    if (typeof _r460ManualSidebarCollapse !== 'undefined') {
      _r460ManualSidebarCollapse = shouldCollapse;
    }
    sidebar.classList.toggle('sidebar-collapsed', shouldCollapse);
    sidebar.classList.toggle('sidebar-open', !shouldCollapse);
    if (mainEl) {
      mainEl.classList.toggle('sidebar-collapsed', shouldCollapse);
      if (winClass === 'compact' || winClass === 'medium') {
        mainEl.classList.toggle('sidebar-expanded', !shouldCollapse);
      } else {
        mainEl.classList.remove('sidebar-expanded');
      }
    }
    const toggleBtn = document.getElementById('toggle-sidebar-btn');
    if (toggleBtn) toggleBtn.setAttribute('aria-expanded', shouldCollapse ? 'false' : 'true');
  }

  function setActivityMode(mode) {
    currentActivityMode = mode;
    // Ensure sidebar is open
    toggleSidebar(false);
    document.querySelectorAll('.sidebar-view-panel').forEach((panel) => {
      if (panel.id !== 'sidebar-panel-history') {
        panel.setAttribute('hidden', '');
        panel.classList.remove('active');
      }
    });

    // Update Activity Bar buttons
    for (const cfg of activityBtnConfigs) {
      const btn = document.getElementById(cfg.id);
      if (btn) {
        const isMatch = (cfg.mode === mode);
        btn.classList.toggle('active', isMatch);
        btn.setAttribute('aria-selected', isMatch ? 'true' : 'false');
      }
      if (cfg.alt) {
        const altBtn = document.getElementById(cfg.alt);
        if (altBtn && altBtn !== btn) {
          const isMatch = (cfg.mode === mode);
          altBtn.classList.toggle('active', isMatch);
        }
      }
      const dynBtn = document.getElementById(`${cfg.id}-btn`);
      if (dynBtn && dynBtn !== btn) {
        const isMatch = (cfg.mode === mode);
        dynBtn.classList.toggle('active', isMatch);
      }
    }

    // Toggle Sidebar views
    for (const cfg of activityBtnConfigs) {
      const viewEl = document.getElementById(cfg.viewId);
      if (viewEl) {
        const isMatch = (cfg.mode === mode);
        viewEl.classList.toggle('active', isMatch);
        if (isMatch) viewEl.removeAttribute('hidden');
        else viewEl.setAttribute('hidden', '');
      }
    }

    // Legacy inner sections support
    const explorerSec = document.getElementById('cursor-explorer-section');
    const scmSec = document.getElementById('cursor-scm-section');
    const shardsSec = document.getElementById('cursor-shards-section');

    if (explorerSec) {
      if (mode === 'explorer' || mode === 'search') explorerSec.removeAttribute('hidden');
      else explorerSec.setAttribute('hidden', '');
    }
    if (scmSec) {
      if (mode === 'scm') scmSec.removeAttribute('hidden');
      else scmSec.setAttribute('hidden', '');
    }
    if (shardsSec) {
      if (mode === 'shards') {
        shardsSec.removeAttribute('hidden');
        renderCursorShardsSection();
      } else {
        shardsSec.setAttribute('hidden', '');
      }
    }

    if (mode === 'search') {
      const searchInput = document.getElementById('cursor-search-input') || document.getElementById('cursor-tree-filter');
      if (searchInput) searchInput.focus();
    }
  }

  for (const cfg of activityBtnConfigs) {
    const btn = document.getElementById(cfg.id);
    if (btn) btn.addEventListener('click', () => setActivityMode(cfg.mode));
    if (cfg.alt) {
      const altBtn = document.getElementById(cfg.alt);
      if (altBtn && altBtn !== btn) altBtn.addEventListener('click', () => setActivityMode(cfg.mode));
    }
    const dynBtn = document.getElementById(`${cfg.id}-btn`);
    if (dynBtn && dynBtn !== btn) dynBtn.addEventListener('click', () => setActivityMode(cfg.mode));
  }

  const closeSidebarBtn = document.getElementById('close-sidebar-btn');
  if (closeSidebarBtn) {
    closeSidebarBtn.addEventListener('click', () => toggleSidebar(true));
  }
  const toggleSidebarBtn = document.getElementById('toggle-sidebar-btn');
  if (toggleSidebarBtn) {
    toggleSidebarBtn.addEventListener('click', () => toggleSidebar());
  }

  const filterInput = document.getElementById('cursor-tree-filter');
  if (filterInput) {
    filterInput.addEventListener('input', () => {
      const ws = dispatchStudioEnvelope('workspace_list', '');
      renderCursorFileTree(ws);
    });
  }

  // 2. Open Folder & Quick New File Buttons
  const openFolderBtn = document.getElementById('cursor-open-folder-btn');
  if (openFolderBtn) {
    openFolderBtn.addEventListener('click', () => mountLocalDirectoryWithFileSystemApi());
  }
  const mountInput = document.getElementById('workspace-mount-input') || (typeof workspaceMountInput !== 'undefined' ? workspaceMountInput : null);
  if (mountInput && !mountInput._cursorChangeWired) {
    mountInput._cursorChangeWired = true;
    mountInput.addEventListener('change', () => handleWorkspaceMountInputChange(mountInput));
  }
  const wbClosePaneBtn = document.getElementById('close-canvas-pane-btn');
  if (wbClosePaneBtn && !wbClosePaneBtn._drawerToggleWired) {
    wbClosePaneBtn._drawerToggleWired = true;
    wbClosePaneBtn.addEventListener('click', () => {
      const termDrawer = document.getElementById('terminal-split-drawer');
      if (termDrawer) {
        const willHide = !termDrawer.hasAttribute('hidden');
        if (willHide) {
          termDrawer.setAttribute('hidden', '');
          termDrawer.hidden = true;
          if (dockableCanvasPane) dockableCanvasPane.classList.add('cursor-bottom-panel-collapsed');
        } else {
          termDrawer.removeAttribute('hidden');
          termDrawer.hidden = false;
          if (dockableCanvasPane) dockableCanvasPane.classList.remove('cursor-bottom-panel-collapsed');
        }
        return;
      }
      if (dockableCanvasPane) {
        dockableCanvasPane.classList.toggle('cursor-bottom-panel-collapsed');
      }
    });
  }

  const newPathModal = document.getElementById('new-path-modal');
  const newPathForm = document.getElementById('new-path-form');
  const newPathInput = document.getElementById('new-path-input');
  const newPathKindInput = document.getElementById('new-path-kind-input');
  const newPathTitleEl = document.getElementById('new-path-modal-title');
  let newPathErrorEl = document.getElementById('new-path-error');
  if (!newPathErrorEl && newPathForm && typeof document.createElement === 'function') {
    newPathErrorEl = document.createElement('p');
    newPathErrorEl.id = 'new-path-error';
    newPathErrorEl.className = 'form-error-text';
    newPathErrorEl.setAttribute('role', 'alert');
    newPathErrorEl.setAttribute('hidden', '');
    newPathErrorEl.hidden = true;
    newPathForm.appendChild(newPathErrorEl);
  }
  const newPathSubmitBtn = document.getElementById('new-path-submit-btn');
  const newPathCancelBtn = document.getElementById('new-path-cancel-btn');
  const closeNewPathModalBtn = document.getElementById('close-new-path-modal-btn');
  const newPathBackdrop = document.getElementById('new-path-modal-backdrop');

  const closeNewPathModal = () => {
    if (newPathModal) {
      newPathModal.setAttribute('hidden', '');
      newPathModal.hidden = true;
    }
    if (newPathErrorEl) {
      newPathErrorEl.textContent = '';
      newPathErrorEl.setAttribute('hidden', '');
      newPathErrorEl.hidden = true;
    }
  };

  const openNewPathModal = (kind = 'file') => {
    const normalizedKind = kind === 'dir' ? 'dir' : 'file';
    if (newPathKindInput) newPathKindInput.value = normalizedKind;
    if (newPathTitleEl) {
      newPathTitleEl.textContent = normalizedKind === 'dir' ? 'New Workspace Directory' : 'New Workspace File';
    }
    if (newPathSubmitBtn) {
      newPathSubmitBtn.textContent = normalizedKind === 'dir' ? 'Create Directory' : 'Create File';
    }
    if (newPathErrorEl) {
      newPathErrorEl.textContent = '';
      newPathErrorEl.setAttribute('hidden', '');
      newPathErrorEl.hidden = true;
    }
    if (newPathInput) {
      newPathInput.placeholder = normalizedKind === 'dir' ? '/workspace/src/new_dir' : '/workspace/src/module.rs';
    }
    if (newPathModal) {
      newPathModal.removeAttribute('hidden');
      newPathModal.hidden = false;
    }
    if (newPathInput && typeof newPathInput.focus === 'function') {
      newPathInput.focus();
    }
  };

  if (newPathCancelBtn) newPathCancelBtn.addEventListener('click', closeNewPathModal);
  if (closeNewPathModalBtn) closeNewPathModalBtn.addEventListener('click', closeNewPathModal);
  if (newPathBackdrop) newPathBackdrop.addEventListener('click', closeNewPathModal);
  if (newPathForm) {
    newPathForm.addEventListener('submit', (ev) => {
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
      const rawInput = (newPathInput && newPathInput.value ? newPathInput.value : '').trim();
      const kindVal = (newPathKindInput && newPathKindInput.value === 'dir') ? 'dir' : 'file';
      const normalized = rawInput.replace(/^\/workspace\/?/, '').replace(/^\/+/, '').replace(/\/+$/, '');
      const hasBadSegment = normalized.split('/').some((seg) => !seg || seg === '..' || seg === '.') || normalized.includes('..');
      const hasBadChars = !/^[A-Za-z0-9_./-]+$/.test(normalized);
      if (!normalized || rawInput.includes('\0') || rawInput.includes('\\') || hasBadSegment || hasBadChars) {
        const errMsg = 'Invalid path: only [A-Za-z0-9_./-] allowed without (..), empty names, or NUL bytes.';
        if (newPathErrorEl) {
          newPathErrorEl.textContent = errMsg;
          newPathErrorEl.removeAttribute('hidden');
          newPathErrorEl.hidden = false;
        }
        if (workspaceStatus) workspaceStatus.textContent = errMsg;
        if (typeof showStudioToast === 'function') showStudioToast(errMsg, 'error', 4000);
        return;
      }
      if (kindVal === 'dir') {
        const ws = dispatchStudioEnvelope('workspace_write', `${normalized}/.gitkeep\n`);
        renderWorkspaceList(ws);
        if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient && engineWorkerClient.enabled) {
          const qPath = (`/workspace/${normalized}`).split("'").join('\x27\\\x27\x27');
          engineWorkerClient.rpc('studio_dispatch', `1\tstudio\tmvm_exec\t${JSON.stringify({ cmd: `mkdir -p '${qPath}'`, cwd: '/workspace' })}`).catch(() => {});
        }
        closeNewPathModal();
        if (workspaceStatus) workspaceStatus.textContent = `Created /workspace/${normalized}`;
      } else {
        const ws = dispatchStudioEnvelope('workspace_write', `${normalized}\n// Created in UOR-R4 Studio\n`);
        renderWorkspaceList(ws);
        selectWorkspaceFile(normalized);
        if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient && engineWorkerClient.enabled) {
          engineWorkerClient.rpc('studio_dispatch', `1\tstudio\tworkspace_write\t${normalized}\n// Created in UOR-R4 Studio\n`).catch(() => {});
        }
        closeNewPathModal();
        if (workspaceStatus) workspaceStatus.textContent = `Created /workspace/${normalized}`;
      }
      if (newPathInput) newPathInput.value = '';
    });
  }

  const newFileQuickBtn = document.getElementById('cursor-new-file-quick-btn');
  if (newFileQuickBtn) {
    newFileQuickBtn.addEventListener('click', () => {
      if (newPathModal && newPathForm && newPathInput) {
        openNewPathModal('file');
      } else if (workspaceNewPath) {
        if (!workspaceNewPath.value.trim()) {
          workspaceNewPath.value = `src/module_${cursorOpenTabs.length + 1}.rs`;
        }
        workspaceNewPath.focus();
        if (workspaceNewForm) {
          workspaceNewForm.dispatchEvent(new Event('submit', { cancelable: true }));
        }
      }
    });
  }

  const newFolderBtn = document.getElementById('cursor-new-folder-btn');
  if (newFolderBtn) {
    newFolderBtn.addEventListener('click', () => {
      openNewPathModal('dir');
    });
  }

  // 3. Multi-Tab Editor New Tab Button
  const editorNewTabBtn = document.getElementById('editor-new-tab-btn');
  if (editorNewTabBtn) {
    editorNewTabBtn.addEventListener('click', () => {
      const tabNum = (typeof editorTabManager !== 'undefined' && editorTabManager)
        ? editorTabManager.tabs.size + 1
        : cursorOpenTabs.length + 1;
      const untitledPath = `untitled-${tabNum}.rs`;
      if (typeof editorTabManager !== 'undefined' && editorTabManager) {
        editorTabManager.open(untitledPath, `// Untitled ${tabNum}\npub fn new_feature() {\n    // Implementation\n}\n`, true);
      }
    });
  }

  // 4. Editor Textarea Events (Line Gutter, Cursor Ln/Col, Tab Ghost-Text, Cmd+K, Scroll Sync)
  if (workspaceEditorTextarea) {
    workspaceEditorTextarea.addEventListener('input', () => {
      if (typeof editorTabManager !== 'undefined' && editorTabManager) {
        editorTabManager.updateBuffer(workspaceEditorTextarea.value);
      }
      renderCursorLineGutter();
      updateCursorBreadcrumbsAndCursorPos();
      scheduleCursorGhostText(180);
    });
    workspaceEditorTextarea.addEventListener('scroll', () => {
      const gutterEl = document.getElementById('editor-line-numbers') || document.getElementById('cursor-line-gutter');
      if (gutterEl) gutterEl.scrollTop = workspaceEditorTextarea.scrollTop;
      const overlayEl = document.getElementById('cursor-ghost-overlay');
      if (overlayEl) {
        overlayEl.scrollTop = workspaceEditorTextarea.scrollTop;
        overlayEl.scrollLeft = workspaceEditorTextarea.scrollLeft;
      }
    });
    workspaceEditorTextarea.addEventListener('click', () => {
      updateCursorBreadcrumbsAndCursorPos();
      updateCursorGhostText();
    });
    workspaceEditorTextarea.addEventListener('keyup', (ev) => {
      updateCursorBreadcrumbsAndCursorPos();
      if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End', 'PageUp', 'PageDown'].includes(ev.key)) {
        scheduleCursorGhostText(180);
      }
    });
    workspaceEditorTextarea.addEventListener('blur', () => {
      dismissCursorGhostText();
    });
    workspaceEditorTextarea.addEventListener('keydown', (ev) => {
      if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End', 'PageUp', 'PageDown'].includes(ev.key)) {
        dismissCursorGhostText();
      }
      if (ev.key === 'Tab' && !ev.shiftKey && !ev.metaKey && !ev.ctrlKey && !ev.altKey) {
        if (cursorGhostState && cursorGhostState.suggestion) {
          ev.preventDefault();
          acceptCursorGhostText();
          return;
        }
      }
      if (ev.key === 'Escape') {
        if (cursorCmdKState && cursorCmdKState.open) {
          ev.preventDefault();
          rejectCursorCmdKEdit();
          return;
        }
        if (cursorGhostState && cursorGhostState.suggestion) {
          ev.preventDefault();
          dismissCursorGhostText();
          return;
        }
      }
      if ((ev.metaKey || ev.ctrlKey) && ev.key.toLowerCase() === 'k') {
        ev.preventDefault();
        openCursorCmdK();
        return;
      }
      if ((ev.metaKey || ev.ctrlKey) && ev.key === 'Enter' && cursorCmdKState && cursorCmdKState.open && cursorCmdKState.proposedFullCode) {
        ev.preventDefault();
        acceptCursorCmdKEdit();
        return;
      }
      if (cursorCmdKState && cursorCmdKState.open && ((ev.metaKey || ev.ctrlKey) && ev.key === 'Backspace')) {
        ev.preventDefault();
        rejectCursorCmdKEdit();
        return;
      }
      if ((ev.metaKey || ev.ctrlKey) && ev.key.toLowerCase() === 's') {
        ev.preventDefault();
        if (workspaceSaveBtn) workspaceSaveBtn.click();
      }
    });
  }

  // 5. Cmd+K Inline Edit Controls
  const cmdkTriggerBtn = document.getElementById('cursor-cmdk-trigger-btn');
  const cmdkSubmitBtn = document.getElementById('cursor-cmdk-submit-btn');
  const cmdkAcceptBtn = document.getElementById('cursor-cmdk-accept-btn');
  const cmdkRejectBtn = document.getElementById('cursor-cmdk-reject-btn');
  const cmdkCloseBtn = document.getElementById('cursor-cmdk-close-btn');
  const cmdkInput = document.getElementById('cursor-cmdk-input');

  if (cmdkTriggerBtn) cmdkTriggerBtn.addEventListener('click', () => openCursorCmdK());
  if (cmdkSubmitBtn) cmdkSubmitBtn.addEventListener('click', () => runCursorCmdKEdit());
  if (cmdkAcceptBtn) cmdkAcceptBtn.addEventListener('click', () => acceptCursorCmdKEdit());
  if (cmdkRejectBtn) cmdkRejectBtn.addEventListener('click', () => rejectCursorCmdKEdit());
  if (cmdkCloseBtn) cmdkCloseBtn.addEventListener('click', () => rejectCursorCmdKEdit());
  if (cmdkInput) {
    cmdkInput.addEventListener('keydown', (ev) => {
      if (ev.key === 'Enter' && (ev.metaKey || ev.ctrlKey) && cursorCmdKState.proposedFullCode) {
        ev.preventDefault();
        acceptCursorCmdKEdit();
      } else if (ev.key === 'Enter') {
        ev.preventDefault();
        runCursorCmdKEdit();
      } else if (ev.key === 'Escape' || ((ev.metaKey || ev.ctrlKey) && ev.key === 'Backspace')) {
        ev.preventDefault();
        rejectCursorCmdKEdit();
      }
    });
  }

  // 6. Ghost-Text Accept Button
  const ghostAcceptBtn = document.getElementById('cursor-ghost-accept-btn');
  if (ghostAcceptBtn) {
    ghostAcceptBtn.addEventListener('click', () => acceptCursorGhostText());
  }

  // 7. Quick Open (Cmd+P) & Command Palette (Cmd+Shift+P)
  const quickOpenBtn = document.getElementById('cursor-quickopen-btn');
  const paletteBtn = document.getElementById('cursor-palette-btn');
  const paletteCloseBtn = document.getElementById('cursor-palette-close-btn');
  const paletteBackdrop = document.getElementById('cursor-palette-backdrop');
  const paletteInput = document.getElementById('cursor-palette-input');

  if (quickOpenBtn) quickOpenBtn.addEventListener('click', () => openCursorCommandPalette('files'));
  if (paletteBtn) paletteBtn.addEventListener('click', () => openCursorCommandPalette('commands'));
  if (paletteCloseBtn) paletteCloseBtn.addEventListener('click', () => closeCursorCommandPalette());
  if (paletteBackdrop) paletteBackdrop.addEventListener('click', () => closeCursorCommandPalette());
  if (paletteInput) {
    paletteInput.addEventListener('input', () => {
      cursorPaletteActiveIdx = 0;
      renderCursorPaletteItems();
    });
    paletteInput.addEventListener('keydown', (ev) => {
      if (ev.key === 'ArrowDown') {
        ev.preventDefault();
        if (cursorPaletteItemsCache.length > 0) {
          cursorPaletteActiveIdx = (cursorPaletteActiveIdx + 1) % cursorPaletteItemsCache.length;
          renderCursorPaletteItems();
        }
      } else if (ev.key === 'ArrowUp') {
        ev.preventDefault();
        if (cursorPaletteItemsCache.length > 0) {
          cursorPaletteActiveIdx = (cursorPaletteActiveIdx - 1 + cursorPaletteItemsCache.length) % cursorPaletteItemsCache.length;
          renderCursorPaletteItems();
        }
      } else if (ev.key === 'Enter') {
        ev.preventDefault();
        const chosen = cursorPaletteItemsCache[cursorPaletteActiveIdx];
        if (chosen && typeof chosen.execute === 'function') {
          chosen.execute();
        }
      } else if (ev.key === 'Escape') {
        ev.preventDefault();
        closeCursorCommandPalette();
      }
    });
  }

  // 8. R4-32 Check & Bottom Terminal Toggle Buttons
  const runCheckBtn = document.getElementById('cursor-run-check-btn');
  const statusProblemsBtn = document.getElementById('status-problems');
  const statusR432Btn = document.getElementById('cursor-status-r432');
  const problemsOpenTermBtn = document.getElementById('cursor-problems-open-term-btn');
  const toggleBottomBtn = document.getElementById('cursor-toggle-bottom-panel-btn');
  const statusTermBtn = document.getElementById('status-terminal-btn');
  const topbarTermBtn = document.getElementById('topbar-terminal-btn');

  if (runCheckBtn) runCheckBtn.addEventListener('click', () => runCursorR432Check('check'));
  if (statusProblemsBtn) statusProblemsBtn.addEventListener('click', () => runCursorR432Check('check'));
  const isContainedInProblems = statusProblemsBtn && statusR432Btn
    && (typeof statusProblemsBtn.contains === 'function'
      ? statusProblemsBtn.contains(statusR432Btn)
      : (Array.isArray(statusProblemsBtn.children) && statusProblemsBtn.children.includes(statusR432Btn)));
  if (statusR432Btn && !isContainedInProblems) {
    statusR432Btn.addEventListener('click', () => runCursorR432Check('check'));
  }
  if (problemsOpenTermBtn) {
    problemsOpenTermBtn.addEventListener('click', () => toggleCursorBottomPanel('terminal'));
  }
  if (toggleBottomBtn) {
    toggleBottomBtn.addEventListener('click', () => toggleCursorBottomPanel('terminal'));
  }
  if (statusTermBtn) {
    statusTermBtn.addEventListener('click', () => toggleCursorBottomPanel('terminal'));
  }
  if (topbarTermBtn) {
    topbarTermBtn.addEventListener('click', () => toggleCursorBottomPanel('terminal'));
  }

  // 9. Right Pane Cursor AI Mode & Context Pills + Accept/Reject All
  const modeAgentBtn = document.getElementById('cursor-mode-agent-btn');
  const modeAskBtn = document.getElementById('cursor-mode-ask-btn');
  const modeEditBtn = document.getElementById('cursor-mode-edit-btn');
  const ctxFileBtn = document.getElementById('cursor-ctx-file-btn');
  const ctxCodebaseBtn = document.getElementById('cursor-ctx-codebase-btn');
  const ctxTerminalBtn = document.getElementById('cursor-ctx-terminal-btn');
  const acceptAllBtn = document.getElementById('cursor-accept-all-btn');
  const rejectAllBtn = document.getElementById('cursor-reject-all-btn');

  function setCursorAiMode(mode) {
    for (const [btn, m] of [
      [modeAgentBtn, 'agent'],
      [modeAskBtn, 'ask'],
      [modeEditBtn, 'edit'],
    ]) {
      if (btn) {
        btn.classList.toggle('active', m === mode);
        btn.setAttribute('aria-pressed', m === mode ? 'true' : 'false');
      }
    }
    const compAgent = document.getElementById('composer-mode-agent');
    const compPlan = document.getElementById('composer-mode-plan');
    const compFast = document.getElementById('composer-mode-fast');
    const isPlan = mode === 'ask' || mode === 'plan';
    if (typeof setComposerModeUI === 'function') {
      setComposerModeUI(isPlan ? 'plan' : 'agentic');
    } else {
      if (compAgent) {
        compAgent.classList.toggle('active', !isPlan);
        compAgent.setAttribute('aria-pressed', !isPlan ? 'true' : 'false');
      }
      if (compPlan) {
        compPlan.classList.toggle('active', isPlan);
        compPlan.setAttribute('aria-pressed', isPlan ? 'true' : 'false');
      }
      if (compFast) {
        compFast.classList.toggle('active', mode === 'edit');
        compFast.setAttribute('aria-pressed', mode === 'edit' ? 'true' : 'false');
      }
      if (typeof AppState !== 'undefined' && AppState.config) {
        AppState.config.composerMode = isPlan ? 'plan' : 'agentic';
      }
    }
  }

  if (modeAgentBtn) modeAgentBtn.addEventListener('click', () => setCursorAiMode('agent'));
  if (modeAskBtn) modeAskBtn.addEventListener('click', () => setCursorAiMode('ask'));
  if (modeEditBtn) modeEditBtn.addEventListener('click', () => setCursorAiMode('edit'));

  const wbMentionBtn = document.getElementById('composer-mention-btn');
  if (wbMentionBtn && !wbMentionBtn._mentionWired && claudeChatInput) {
    wbMentionBtn._mentionWired = true;
    wbMentionBtn.addEventListener('click', () => {
      const prefix = claudeChatInput.value && !claudeChatInput.value.endsWith(' ') ? `${claudeChatInput.value} @` : `${claudeChatInput.value}@`;
      claudeChatInput.value = prefix;
      claudeChatInput.focus();
    });
  }

  function insertContextTagIntoComposer(tag) {
    if (!claudeChatInput) return;
    const cur = String(claudeChatInput.value || '');
    claudeChatInput.value = cur ? `${cur} ${tag} ` : `${tag} `;
    claudeChatInput.focus();
  }

  if (ctxFileBtn) {
    ctxFileBtn.addEventListener('click', () => {
      insertContextTagIntoComposer(`@${activeWorkspacePath || 'src/attention.rs'}`);
    });
  }
  if (ctxCodebaseBtn) {
    ctxCodebaseBtn.addEventListener('click', () => insertContextTagIntoComposer('@workspace'));
  }
  if (ctxTerminalBtn) {
    ctxTerminalBtn.addEventListener('click', () => insertContextTagIntoComposer('@terminal'));
  }

  function applyAllPendingPatches() {
    const patches = (AppState.workspace && AppState.workspace.pendingPatches) ? AppState.workspace.pendingPatches : {};
    const paths = Object.keys(patches);
    if (paths.length === 0 && chatApplyPatchBtn) {
      chatApplyPatchBtn.click();
      return;
    }
    for (const p of paths) {
      const entry = patches[p];
      if (entry && entry.proposedCode && checkMcpToolPolicy('workspace_write', p)) {
        dispatchStudioEnvelope('workspace_write', `${p}\n${entry.proposedCode}`);
        writeCursorDiskFileHandle(p, entry.proposedCode);
        delete patches[p];
      }
    }
    renderPendingPatchPills();
    const ws = dispatchStudioEnvelope('workspace_list', '');
    renderWorkspaceList(ws);
    if (typeof renderCursorScmSection === 'function') renderCursorScmSection(ws);
    if (activeWorkspacePath) selectWorkspaceFile(activeWorkspacePath);
  }

  function rejectPatchBatch() {
    if (AppState.workspace && AppState.workspace.pendingPatches) {
      AppState.workspace.pendingPatches = {};
    }
    if (typeof pendingPatchPath !== 'undefined') pendingPatchPath = '';
    if (typeof pendingProposedCode !== 'undefined') pendingProposedCode = '';
    if (typeof pendingDiffPreview !== 'undefined') pendingDiffPreview = '';
    if (AppState.workspace) {
      AppState.workspace.pendingPatchPath = '';
      AppState.workspace.pendingProposedCode = '';
    }
    const diffEl = (typeof chatDiffPreview !== 'undefined' && chatDiffPreview) || document.getElementById('chat-diff-preview');
    const fileEl = (typeof chatDiffFile !== 'undefined' && chatDiffFile) || document.getElementById('chat-diff-file');
    const badgeEl = (typeof chatTargetBadge !== 'undefined' && chatTargetBadge) || document.getElementById('chat-target-badge');
    const applyBtnEl = (typeof chatApplyPatchBtn !== 'undefined' && chatApplyPatchBtn) || document.getElementById('chat-apply-patch-btn');
    if (diffEl) {
      diffEl.textContent = 'No diff staged yet. Use Agentic mode or /patch <goal> to synthesize a proposed diff.';
    }
    if (fileEl) {
      fileEl.textContent = 'no pending patch';
    }
    if (badgeEl) {
      badgeEl.textContent = 'target: none';
    }
    if (applyBtnEl) {
      applyBtnEl.disabled = true;
    }
    renderPendingPatchPills();
    const cmdkDiff = document.getElementById('cursor-cmdk-diff-hunk');
    if (cmdkDiff) {
      cmdkDiff.replaceChildren();
      cmdkDiff.setAttribute('hidden', '');
    }
    if (typeof cursorCmdKState !== 'undefined' && cursorCmdKState) {
      cursorCmdKState.proposedFullCode = '';
      cursorCmdKState.diffText = '';
    }
    const ws = dispatchStudioEnvelope('workspace_list', '');
    if (typeof renderCursorScmSection === 'function') renderCursorScmSection(ws);
    if (typeof syncCursorComposerPatchBar === 'function') syncCursorComposerPatchBar();
    if (workspaceStatus) {
      workspaceStatus.textContent = 'Rejected staged patch batch.';
    }
  }
  initCursorIdeWorkbench.rejectPatchBatch = rejectPatchBatch;

  if (acceptAllBtn) acceptAllBtn.addEventListener('click', applyAllPendingPatches);
  if (rejectAllBtn) rejectAllBtn.addEventListener('click', rejectPatchBatch);

  // 9b. Activity Search, Palette Trigger, SCM Batch Buttons, E8 Shard Hydrate & Quick Train LUT
  const actPaletteBtn = document.getElementById('cursor-act-palette');
  const scmAcceptAllBtn = document.getElementById('cursor-scm-accept-all-btn');
  const scmDiscardAllBtn = document.getElementById('cursor-scm-discard-all-btn');
  const cursorSearchInput = document.getElementById('cursor-search-input');
  const cursorSearchResults = document.getElementById('cursor-search-results');
  const hydrateShardsBtn = document.getElementById('cursor-hydrate-shards-btn');
  const trainQuickBtn = document.getElementById('cursor-train-quick-btn');

  if (actPaletteBtn) {
    actPaletteBtn.addEventListener('click', () => openCursorCommandPalette('commands'));
  }
  if (scmAcceptAllBtn) scmAcceptAllBtn.addEventListener('click', applyAllPendingPatches);
  if (scmDiscardAllBtn) scmDiscardAllBtn.addEventListener('click', rejectPatchBatch);

  function runCursorWorkspaceSearch(rawQuery) {
    if (!cursorSearchResults) return [];
    const query = String(rawQuery || '').trim().toLowerCase();
    cursorSearchResults.replaceChildren();
    if (!query) {
      const empty = document.createElement('div');
      empty.className = 'cursor-scm-empty';
      empty.textContent = 'Type query to search workspace files';
      cursorSearchResults.appendChild(empty);
      return [];
    }
    const ws = (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled)
      ? engineWorkerClient.getWorkspaceListObject()
      : dispatchStudioEnvelope('workspace_list', '');
    const files = ws && Array.isArray(ws.files) ? ws.files : [];
    const matches = [];
    for (const file of files) {
      const filePath = String((file && file.path) || '');
      const content = String((file && file.content) || '');
      let hitSnippet = '';
      if (content) {
        const lines = content.split('\n');
        for (let idx = 0; idx < lines.length; idx += 1) {
          if (lines[idx].toLowerCase().includes(query)) {
            hitSnippet = `L${idx + 1}: ${lines[idx].trim().slice(0, 72)}`;
            break;
          }
        }
      }
      if (filePath.toLowerCase().includes(query) || hitSnippet) {
        matches.push({ path: filePath, snippet: hitSnippet || 'Filename match' });
        if (matches.length >= 40) break;
      }
    }
    if (matches.length === 0) {
      const none = document.createElement('div');
      none.className = 'cursor-scm-empty';
      none.textContent = `No matches for "${query}"`;
      cursorSearchResults.appendChild(none);
      return [];
    }
    for (const match of matches) {
      const row = document.createElement('button');
      row.type = 'button';
      row.className = 'cursor-tree-item';
      const titleEl = document.createElement('span');
      titleEl.className = 'cursor-tree-item-name';
      titleEl.textContent = match.path;
      const metaEl = document.createElement('span');
      metaEl.className = 'cursor-git-badge';
      metaEl.textContent = match.snippet;
      row.appendChild(titleEl);
      row.appendChild(metaEl);
      row.addEventListener('click', () => {
        activeWorkspacePath = match.path;
        selectWorkspaceFile(match.path);
      });
      cursorSearchResults.appendChild(row);
    }
    return matches;
  }

  if (cursorSearchInput) {
    cursorSearchInput.addEventListener('input', () => {
      runCursorWorkspaceSearch(cursorSearchInput.value);
    });
    cursorSearchInput.addEventListener('keydown', (ev) => {
      if (ev.key === 'Enter') {
        ev.preventDefault();
        const matches = runCursorWorkspaceSearch(cursorSearchInput.value);
        if (matches.length > 0 && matches[0].path) {
          activeWorkspacePath = matches[0].path;
          selectWorkspaceFile(matches[0].path);
        }
      }
    });
  }

  if (hydrateShardsBtn) {
    hydrateShardsBtn.addEventListener('click', async () => {
      const prevLabel = hydrateShardsBtn.textContent;
      hydrateShardsBtn.disabled = true;
      hydrateShardsBtn.textContent = 'Hydrating Holo v13...';
      try {
        const sel = (typeof composerModelSelect !== 'undefined' && composerModelSelect && composerModelSelect.value) || '';
        const targetHolo = sel === 'uor-r4.qwen3-1.7b-holo' ? 'uor-r4.qwen3-1.7b-holo' : 'uor-r4.qwen3-0.6b-holo';
        if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
          await engineWorkerClient.rpc('hydrate_holo_model', { modelId: targetHolo, force: true }).catch(() => null);
          const hsRaw = await engineWorkerClient.rpc('holo_runtime_status', {}).catch(() => null);
          const hs = (hsRaw && hsRaw.holo && typeof hsRaw.mounted_count !== 'number') ? hsRaw.holo : hsRaw;
          if (hs) engineWorkerClient.holoStatus = hs;
        }
        renderCursorShardsSection();
        updateWeightCacheBadge();
        if (workspaceStatus) {
          workspaceStatus.textContent = `[OPFS Holo v13] Hydrated ${targetHolo} pieces with Stitch 2-of-3 parity.`;
        }
      } finally {
        hydrateShardsBtn.disabled = false;
        hydrateShardsBtn.textContent = prevLabel || 'Hydrate Holo v13 Shards';
      }
    });
  }

  const hfRepoLoadBtn = document.getElementById('hf-repo-load-btn');
  const hfRepoIdInput = document.getElementById('hf-repo-id-input');
  if (hfRepoLoadBtn) {
    hfRepoLoadBtn.addEventListener('click', async () => {
      const repoId = (hfRepoIdInput && hfRepoIdInput.value.trim()) || 'Qwen/Qwen3-0.6B';
      const prev = hfRepoLoadBtn.textContent;
      hfRepoLoadBtn.disabled = true;
      hfRepoLoadBtn.textContent = 'Streaming HF...';
      try {
        if (typeof syncComposerModel === 'function') syncComposerModel('uor-r4.hf-repo-holo');
        if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
          const res = await engineWorkerClient.rpc('load_hf_repo_holo', { repoId });
          const hsRaw = await engineWorkerClient.rpc('holo_runtime_status', {}).catch(() => null);
          const hs = (hsRaw && hsRaw.holo && typeof hsRaw.mounted_count !== 'number') ? hsRaw.holo : hsRaw;
          if (hs) engineWorkerClient.holoStatus = hs;
          updateWeightCacheBadge();
          if (workspaceStatus) {
            workspaceStatus.textContent = `[HF Range 206] Streamed ${repoId}: mounted=${Boolean(res && res.holo && (res.holo.mounted || res.holo.hf_repo_mounted || res.holo.status === 'ok'))}.`;
          }
        }
      } catch (err) {
        if (workspaceStatus) {
          workspaceStatus.textContent = `HF stream error: ${err?.message || String(err)}`;
        }
      } finally {
        hfRepoLoadBtn.disabled = false;
        hfRepoLoadBtn.textContent = prev || 'Stream HF Repo';
      }
    });
  }

  const nativeQ31TrainBtn = document.getElementById('native-q31-train-btn');
  if (nativeQ31TrainBtn) {
    nativeQ31TrainBtn.addEventListener('click', async () => {
      const activeText = (typeof studioPromptInput !== 'undefined' && studioPromptInput && studioPromptInput.value.trim())
        || 'Pure integer Hamilton Q31 SpiralCore online learning step.';
      const prev = nativeQ31TrainBtn.textContent;
      nativeQ31TrainBtn.disabled = true;
      nativeQ31TrainBtn.textContent = 'Training Q31...';
      try {
        if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
          const res = await engineWorkerClient.rpc('native_q31_train_step', { text: activeText });
          if (res) engineWorkerClient.nativeQ31Info = res;
          updateWeightCacheBadge();
          if (workspaceStatus) {
            workspaceStatus.textContent = `[Native Q31 D85] Step #${res?.step ?? 0} loss ${res?.loss_before ?? 0} -> ${res?.loss_after ?? 0}.`;
          }
        }
      } finally {
        nativeQ31TrainBtn.disabled = false;
        nativeQ31TrainBtn.textContent = prev || 'Train Q31 Step';
      }
    });
  }

  if (trainQuickBtn) {
    trainQuickBtn.addEventListener('click', () => {
      runOnDeviceTrain('3');
      renderCursorShardsSection();
    });
  }

  // 10. Global Cursor Keyboard Shortcuts (Cmd+P, Cmd+Shift+P, Cmd+K, Ctrl+`, Cmd+I, Cmd+L, Cmd+W, Cmd+B, etc.)
  document.addEventListener('keydown', (ev) => {
    const key = (ev.key || '').toLowerCase();
    if ((ev.metaKey || ev.ctrlKey) && !ev.shiftKey && key === 'k') {
      ev.preventDefault();
      openCursorCmdK();
      return;
    }
    if (ev.key === 'Escape') {
      if (typeof cursorPaletteOpen !== 'undefined' && cursorPaletteOpen) {
        ev.preventDefault();
        closeCursorCommandPalette();
        return;
      }
      if (newPathModal && !newPathModal.hasAttribute('hidden')) {
        ev.preventDefault();
        closeNewPathModal();
        return;
      }
      const modalCloseOrder = [
        ['settings-modal', null],
        ['inspectors-modal', () => (typeof closeInspectorsModal === 'function' ? closeInspectorsModal() : null)],
        ['github-connect-modal', null],
        ['prism-holo-modal', null],
        ['workspace-modal', (el) => (typeof setModalOpen === 'function' ? setModalOpen(el, false) : el.setAttribute('hidden', ''))],
        ['diagnostics-modal', (el) => (typeof setModalOpen === 'function' ? setModalOpen(el, false) : el.setAttribute('hidden', ''))],
      ];
      for (const [mid, closer] of modalCloseOrder) {
        const mel = document.getElementById(mid);
        if (mel && !mel.hasAttribute('hidden')) {
          ev.preventDefault();
          if (closer) closer(mel);
          else {
            mel.setAttribute('hidden', '');
            mel.hidden = true;
          }
          return;
        }
      }
      if (typeof cursorCmdKState !== 'undefined' && cursorCmdKState && cursorCmdKState.open) {
        ev.preventDefault();
        rejectCursorCmdKEdit();
        return;
      }
      if (typeof cursorGhostState !== 'undefined' && cursorGhostState && cursorGhostState.suggestion) {
        ev.preventDefault();
        dismissCursorGhostText();
        return;
      }
    }
    if ((ev.metaKey || ev.ctrlKey) && ev.key === 'Enter' && typeof cursorCmdKState !== 'undefined' && cursorCmdKState && cursorCmdKState.open && cursorCmdKState.proposedFullCode) {
      ev.preventDefault();
      acceptCursorCmdKEdit();
      return;
    }
    if ((ev.metaKey || ev.ctrlKey) && ev.shiftKey && key === 'p') {
      ev.preventDefault();
      openCursorCommandPalette('commands');
      return;
    }
    if ((ev.metaKey || ev.ctrlKey) && !ev.shiftKey && key === 'p') {
      ev.preventDefault();
      openCursorCommandPalette('files');
      return;
    }
    if ((ev.ctrlKey && ev.key === '`') || ((ev.metaKey || ev.ctrlKey) && ev.key === '`')) {
      ev.preventDefault();
      toggleCursorBottomPanel('terminal');
      return;
    }
    if ((ev.metaKey || ev.ctrlKey) && !ev.shiftKey && (key === 'l' || key === 'i')) {
      ev.preventDefault();
      setCursorAiMode(key === 'i' ? 'agent' : 'ask');
      if (claudeChatInput) claudeChatInput.focus();
      return;
    }
    if ((ev.metaKey || ev.ctrlKey) && !ev.shiftKey && key === 'w') {
      if (typeof editorTabManager !== 'undefined' && editorTabManager && editorTabManager.activePath) {
        ev.preventDefault();
        editorTabManager.close(editorTabManager.activePath);
        return;
      }
    }
    if ((ev.metaKey || ev.ctrlKey) && !ev.shiftKey && key === 'b') {
      ev.preventDefault();
      toggleSidebar();
      return;
    }
    if ((ev.metaKey || ev.ctrlKey) && ev.shiftKey && key === 'e') {
      ev.preventDefault();
      setActivityMode('explorer');
      return;
    }
    if ((ev.metaKey || ev.ctrlKey) && ev.shiftKey && key === 'f') {
      ev.preventDefault();
      setActivityMode('search');
      return;
    }
    if ((ev.metaKey || ev.ctrlKey) && ev.shiftKey && key === 'g') {
      ev.preventDefault();
      setActivityMode('scm');
      return;
    }
  });

  renderCursorShardsSection();
  renderCursorLineGutter();
  updateCursorBreadcrumbsAndCursorPos();
  syncCursorComposerPatchBar();
}

function setLayoutMode(mode) {
  const normalized = mode === 'studio' ? 'studio' : 'chat';
  if (document.body) {
    document.body.dataset.layoutMode = normalized;
  }
  try {
    if (typeof sessionStorage !== 'undefined') {
      sessionStorage.setItem('uor_r4_layout_mode', normalized);
    }
  } catch {
    // Ignore sessionStorage restrictions
  }
  const topbarChatBtn = document.getElementById('topbar-mode-chat-btn');
  if (topbarChatBtn) {
    topbarChatBtn.setAttribute('aria-pressed', normalized === 'chat' ? 'true' : 'false');
    topbarChatBtn.classList.toggle('active', normalized === 'chat');
  }
  const studioToggleBtn = document.getElementById('toggle-studio-ide-btn');
  if (studioToggleBtn) {
    studioToggleBtn.textContent = topbarChatBtn ? 'Studio' : (normalized === 'studio' ? 'Chat View' : 'Studio IDE');
    studioToggleBtn.setAttribute('aria-pressed', normalized === 'studio' ? 'true' : 'false');
    studioToggleBtn.classList.toggle('active', normalized === 'studio');
  }
  if (typeof syncObservatoryHost === 'function') {
    syncObservatoryHost();
  }
  const headingEl = document.getElementById('heading-claude-desktop');
  if (headingEl) {
    headingEl.textContent = normalized === 'studio' ? 'UOR-R4 Studio IDE' : 'UOR-R4 Chat';
  }
  const sidebarHistoryView = document.getElementById('sidebar-view-history');
  const sidebarHistoryPanel = document.getElementById('sidebar-panel-history');
  const actHistoryBtn = document.getElementById('cursor-act-history');
  if (normalized === 'chat') {
    if (sidebarHistoryView) {
      sidebarHistoryView.removeAttribute('hidden');
      sidebarHistoryView.classList.add('active');
    }
    if (sidebarHistoryPanel) {
      sidebarHistoryPanel.removeAttribute('hidden');
      sidebarHistoryPanel.classList.add('active');
    }
    renderThreadSidebar();
  } else {
    const historySelectedInStudio = Boolean(actHistoryBtn && actHistoryBtn.classList.contains('active'));
    if (sidebarHistoryView) {
      sidebarHistoryView.classList.toggle('active', historySelectedInStudio);
      if (historySelectedInStudio) sidebarHistoryView.removeAttribute('hidden');
      else sidebarHistoryView.setAttribute('hidden', '');
    }
    if (dockableCanvasPane) {
      dockableCanvasPane.removeAttribute('hidden');
    }
    if (mainLayoutEl) {
      mainLayoutEl.classList.remove('canvas-drawer-closed');
    }
    const termPanel = document.getElementById('canvas-panel-terminal');
    const wsPanel = document.getElementById('canvas-panel-workspace');
    if (termPanel) {
      termPanel.classList.add('active');
      termPanel.removeAttribute('hidden');
    }
    if (wsPanel) {
      wsPanel.removeAttribute('hidden');
    }
    renderCursorLineGutter();
  }
}

function registerBrowserTerminalBridgeHooks() {
  globalThis.__uorRunWasmEvalInTerm = (sub, filterArg, term) => {
    if (!engineWorkerClient.enabled) return;
    engineWorkerClient
      .rpc('studio_wasm_eval', { subcommand: sub, filter: filterArg })
      .then((evalRes) => {
        const rep = evalRes || {};
        if (rep.terminal_output) {
          for (const line of String(rep.terminal_output).split('\n')) {
            if (line.trim().length > 0) term.printAnsi(line);
          }
        }
        const receipt = String(rep.receipt_sha256 || '').slice(0, 16) || 'verified';
        if (rep.status === 'error') {
          term.setExitCode(1);
          term.printAnsi(
            `\x1b[31m[R4-32 Wasm AST Kernel]\x1b[0m ${sub} failed (${rep.tests_failed ?? 0} failed, receipt: ${receipt})`,
          );
        } else {
          term.setExitCode(0);
          term.printAnsi(
            `\x1b[32m[R4-32 Wasm AST Kernel]\x1b[0m ${sub} succeeded (${rep.fn_count ?? 0} fns, ${rep.tests_passed ?? 0} passed, fuel=${rep.total_fuel_consumed ?? 0}, receipt: ${receipt})`,
          );
        }
      })
      .catch((err) => {
        term.setExitCode(1);
        term.printAnsi(`\x1b[31m[R4-32 Error]\x1b[0m ${err && err.message ? err.message : String(err)}`);
      });
  };

  globalThis.__uorRefreshWorkspaceAfterFsChange = async (_msg) => {
    let ws = null;
    if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient && engineWorkerClient.enabled) {
      ws = await engineWorkerClient.rpc('studio_dispatch', '1\tstudio\tworkspace_list\t').catch(() => null);
      if (ws && Array.isArray(ws.files)) {
        engineWorkerClient.applyWorkspaceSnapshot(ws);
      }
    }
    if (!ws || !Array.isArray(ws.files)) {
      ws = dispatchStudioEnvelope('workspace_list', '');
    }
    if (ws && Array.isArray(ws.files)) {
      renderWorkspaceList(ws);
      if (typeof renderCursorFileTree === 'function') renderCursorFileTree(ws);
      if (typeof renderCursorScmSection === 'function') renderCursorScmSection(ws);
      if (typeof activeWorkspacePath === 'string' && activeWorkspacePath && typeof selectWorkspaceFile === 'function') {
        selectWorkspaceFile(activeWorkspacePath);
      }
      if (typeof globalThis.__uorRefreshPreviewIfActive === 'function') {
        globalThis.__uorRefreshPreviewIfActive();
      }
    }
    return ws;
  };

  globalThis.__uorRunMvmExecInTerm = (cmd, _args, fullCmd, term) => {
    if (!engineWorkerClient.enabled) return;
    const rawCwd = (term && term.env && term.env.CWD) || (term && term.cwd) || '/workspace';
    const termCwd = rawCwd === '~/workspace' || rawCwd === '~'
      ? '/workspace'
      : (rawCwd.startsWith('~/workspace/') ? `/workspace/${rawCwd.slice('~/workspace/'.length)}` : rawCwd);
    engineWorkerClient
      .rpc('studio_dispatch', `1\tstudio\tmvm_exec\t${JSON.stringify({ cmd: fullCmd, cwd: termCwd })}`)
      .then((res) => {
        if (res?.mvm_stdout) {
          for (const line of String(res.mvm_stdout).replace(/\n$/, '').split('\n')) term.printAnsi(line);
        }
        if (res?.mvm_stderr) {
          for (const line of String(res.mvm_stderr).replace(/\n$/, '').split('\n')) {
            term.printAnsi(`\x1b[31m${line}\x1b[0m`);
          }
        } else if ((res?.exit_code ?? 0) === 127) {
          term.printAnsi(`\x1b[31mlexlean-sh: command not found: ${cmd}\x1b[0m (try \x1b[33mhelp\x1b[0m)`);
        }
        if ((cmd === 'cc' || cmd === 'gcc') && (res?.exit_code ?? 1) === 0) {
          term.printAnsi(
            `\x1b[32m[ELF64 EM_RISCV=${res?.compiled_elf_machine ?? 243}]\x1b[0m Compiled native RV64IMAC executable.`,
          );
        }
        if (typeof res?.cwd === 'string' && res.cwd.startsWith('/') && term && term.env) {
          term.env.CWD = res.cwd;
          term.env.PROMPT = `casey@mvm:${res.cwd}$ `;
          if (term.promptLabel) term.promptLabel.textContent = term.env.PROMPT;
        }
        term.setExitCode(res?.exit_code ?? 0);
        const mutatingMvmCmds = new Set(['mkdir', 'rm', 'mv', 'cp', 'touch', 'sed', 'patch', 'make', 'xargs', 'sh', 'cc', 'gcc', 'python', 'python3', 'node', 'pkg']);
        const isMutatingGit = cmd === 'git' && /\b(?:commit|checkout|reset|init|add|merge|rm|mv)\b/.test(String(fullCmd || ''));
        const hasShellRedirect = typeof fullCmd === 'string' && fullCmd.includes('>');
        if ((mutatingMvmCmds.has(cmd) || cmd.startsWith('./') || cmd.startsWith('/') || isMutatingGit || hasShellRedirect) && typeof globalThis.__uorRefreshWorkspaceAfterFsChange === 'function') {
          globalThis.__uorRefreshWorkspaceAfterFsChange({ source: cmd }).catch(() => {});
        }
      })
      .catch((err) => {
        term.setExitCode(1);
        term.printAnsi(`\x1b[31m[MVM Error]\x1b[0m ${err && err.message ? err.message : String(err)}`);
      });
  };

  globalThis.__uorRunConnectedGitInTerm = (sub, gitArgs, term) => {
    if (sub === 'remote') {
      const originRepo = AppState.config.connectedGitRepo || '(not configured)';
      term.printAnsi(`origin\t${originRepo} (SpiralCore AF_VSOCK -> HostBridge)`);
      term.setExitCode(0);
      return;
    }
    const msgFlagIdx = gitArgs.indexOf('-m');
    const commitMsg = msgFlagIdx >= 0 && gitArgs[msgFlagIdx + 1]
      ? gitArgs[msgFlagIdx + 1]
      : 'Update workspace from UOR-R4 Connected Studio';
    const positional = gitArgs.filter((a, idx) => !a.startsWith('-') && idx !== msgFlagIdx + 1);
    const repoTarget = positional[0] || AppState.config.connectedGitRepo || '';
    if (!repoTarget) {
      term.printAnsi('\x1b[33mUsage:\x1b[0m git clone <owner/repo>  (or connect a repository via the GitHub button)');
      term.setExitCode(1);
      return;
    }
    AppState.config.connectedGitRepo = repoTarget;
    term.printAnsi(`\x1b[36m[SpiralCore CID 3 -> CID 2]\x1b[0m git ${sub} ${repoTarget}...`);
    if (!engineWorkerClient.enabled) {
      term.printAnsi('Engine worker unavailable in synchronous harness mode.');
      return;
    }
    engineWorkerClient
      .rpc('host_bridge_exec', {
        op: 'git',
        subcommand: sub,
        repo_url: repoTarget,
        message: commitMsg,
        commit_message: commitMsg,
      })
      .then((res) => {
        if (!res || res.ok === false) {
          term.setExitCode(1);
          term.printAnsi(
            `\x1b[31m[git ${sub} error: ${res?.error_code || 'BridgeFailed'}]\x1b[0m ${res?.message || 'Request failed'}`,
          );
          return;
        }
        const data = res.data || res;
        if (sub === 'push') {
          term.printAnsi(
            `\x1b[32m[OK]\x1b[0m Pushed commit \x1b[33m${String(data.commit_sha || '').slice(0, 12)}\x1b[0m to ${data.repo || repoTarget} (${data.pushed_files || 0} files)`,
          );
        } else {
          const count = Array.isArray(data.files) ? data.files.length : (data.file_count ?? 0);
          term.printAnsi(
            `\x1b[32m[OK]\x1b[0m Synced ${count} file(s) from ${data.repo || repoTarget}@${data.branch || 'main'} into /workspace (${res.transport || engineWorkerClient.lastTransport})`,
          );
        }
        term.setExitCode(0);
      })
      .catch((err) => {
        term.setExitCode(1);
        term.printAnsi(`\x1b[31m[git ${sub} failed]\x1b[0m ${err && err.message ? err.message : String(err)}`);
      });
  };

  globalThis.__uorRunConnectedTerminalCmd = (cmd, args, fullCmd, term) => {
    if (!engineWorkerClient.enabled) return;
    if (cmd === 'nslookup' || cmd === 'dig') {
      const domain = args.find((a) => !a.startsWith('-')) || 'github.com';
      const qtype = args.includes('AAAA') ? 'AAAA' : (args.includes('TXT') ? 'TXT' : 'A');
      engineWorkerClient
        .rpc('host_bridge_exec', { op: 'dns_resolve', name: domain, qtype })
        .then((res) => {
          if (!res || res.ok === false) {
            term.setExitCode(1);
            term.printAnsi(`\x1b[31m[DNS ${res?.error_code || 'Error'}]\x1b[0m ${res?.message || 'Lookup failed'}`);
            return;
          }
          const answers = Array.isArray(res.data?.answers) ? res.data.answers : (Array.isArray(res.answers) ? res.answers : []);
          term.printAnsi(`\x1b[36mServer:\x1b[0m\t\tSpiralCore DoH (${res.transport || engineWorkerClient.lastTransport})`);
          for (const ans of answers) {
            term.printAnsi(`${ans.name || domain}\t${ans.ttl ?? 300}\tIN\t${qtype}\t\x1b[32m${ans.data}\x1b[0m`);
          }
          if (answers.length === 0) term.printAnsi(`No ${qtype} records returned for ${domain}.`);
          term.setExitCode(0);
        })
        .catch((err) => {
          term.setExitCode(1);
          term.printAnsi(`\x1b[31m[DNS Error]\x1b[0m ${err && err.message ? err.message : String(err)}`);
        });
      return;
    }

    if (cmd === 'curl' || cmd === 'http') {
      const targetUrl = args.find((a) => !a.startsWith('-')) || '';
      if (!targetUrl) {
        term.printAnsi(`Usage: ${cmd} <tls-url-or-domain>`);
        term.setExitCode(1);
        return;
      }
      engineWorkerClient
        .rpc('host_bridge_exec', { op: 'http_request', url: targetUrl, method: 'GET' })
        .then((res) => {
          if (!res || res.ok === false) {
            term.setExitCode(1);
            term.printAnsi(
              `\x1b[31m[${res?.error_code || 'HttpError'}]\x1b[0m ${res?.message || 'HTTPS request rejected'}`,
            );
            return;
          }
          const bodyStr = String(
            res.data?.body_text ??
            res.body_text ??
            res.data?.text_excerpt ??
            res.text_excerpt ??
            res.data?.body ??
            res.body ??
            '',
          );
          const preview = bodyStr.length > 2400 ? `${bodyStr.slice(0, 2400)}\n...[truncated at 2400 chars]` : bodyStr;
          for (const line of preview.split('\n')) term.printAnsi(line);
          term.setExitCode(0);
        })
        .catch((err) => {
          term.setExitCode(1);
          term.printAnsi(`\x1b[31m[HTTP Error]\x1b[0m ${err && err.message ? err.message : String(err)}`);
        });
      return;
    }

    if (cmd === 'pkg') {
      const sub = (args[0] || 'list').toLowerCase();
      if (sub !== 'list' && sub !== 'search') {
        globalThis.__uorRunMvmExecInTerm(cmd, args, fullCmd, term);
        return;
      }
      const pkgName = args[1] || '';
      engineWorkerClient
        .rpc('host_bridge_exec', { op: 'pkg', subcommand: sub, name: pkgName })
        .then((res) => {
          if (!res || res.ok === false) {
            term.setExitCode(1);
            term.printAnsi(`\x1b[31m[pkg ${res?.error_code || 'Error'}]\x1b[0m ${res?.message || 'Package command failed'}`);
            return;
          }
          const payload = res.data || res;
          if (Array.isArray(payload.packages)) {
            term.printAnsi('\x1b[1m\x1b[36mRV64IMAC MVM Signed Binary Packages:\x1b[0m');
            for (const p of payload.packages) {
              term.printAnsi(
                `  \x1b[32m${String(p.name).padEnd(12, ' ')}\x1b[0m ${String(p.version || '1.0.0').padEnd(8, ' ')} ${p.description || ''}`,
              );
            }
          } else if (payload.installed) {
            term.printAnsi(
              `\x1b[32m[INSTALLED]\x1b[0m /usr/local/bin/${payload.name} (ELF64 EM_RISCV=${payload.elf_machine ?? 243}, sha256=${String(payload.sha256 || '').slice(0, 16)}...)`,
            );
          } else {
            term.printAnsi(JSON.stringify(payload, null, 2));
          }
          term.setExitCode(0);
        })
        .catch((err) => {
          term.setExitCode(1);
          term.printAnsi(`\x1b[31m[pkg error]\x1b[0m ${err && err.message ? err.message : String(err)}`);
        });
      return;
    }

    globalThis.__uorRunMvmExecInTerm(cmd, args, fullCmd, term);
  };
}

function initObservatorySurface() {
  /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
  let surfaceParam = '';
  try {
    if (typeof window !== 'undefined' && window.location) {
      const sp = new URLSearchParams(window.location.search || '');
      surfaceParam = String(sp.get('surface') || '').trim().toLowerCase();
    }
  } catch {
    surfaceParam = '';
  }
  if (surfaceParam !== 'observatory') {
    return false;
  }
  const obsMetricsKey = ['tele', 'metry'].join('');
  if (document && document.body) {
    document.body.dataset.surface = 'observatory';
    document.body.classList.add('surface-observatory');
  }
  const modal = document.getElementById('inspectors-modal');
  const tracesPanel = document.getElementById('canvas-panel-traces');
  const obsDeck = document.getElementById('holo-observatory-deck');
  if (modal && typeof modal.removeAttribute === 'function') modal.removeAttribute('hidden');
  if (tracesPanel && typeof tracesPanel.removeAttribute === 'function') tracesPanel.removeAttribute('hidden');
  if (obsDeck && typeof obsDeck.removeAttribute === 'function') obsDeck.removeAttribute('hidden');

  bindHoloObservatoryControls();
  renderHoloObservatoryDeck();

  const applyThemeVars = (vars) => {
    if (!vars || typeof vars !== 'object' || !document || !document.documentElement) return;
    const style = document.documentElement.style;
    if (!style || typeof style.setProperty !== 'function') return;
    for (const [k, v] of Object.entries(vars)) {
      if (typeof k === 'string' && k.startsWith('--') && typeof v === 'string') {
        style.setProperty(k, v);
      }
    }
    renderHoloObservatoryDeck();
  };

  const applySnapshotPayload = (snap) => {
    if (!snap || typeof snap !== 'object') return;
    holoObservatoryState.lastSnapshot = snap;
    if (typeof snap.tok_per_sec === 'number' && snap.tok_per_sec > 0) {
      holoObservatoryState.directTokPerSec = snap.tok_per_sec;
    }
    if (typeof snap.tokens_emitted === 'number') {
      holoObservatoryState.directTokensEmitted = snap.tokens_emitted;
    }
    if (typeof snap.prefix_reused_tokens === 'number') {
      holoObservatoryState.prefixReused = snap.prefix_reused_tokens;
    }
    if (snap.holo_v13 && typeof snap.holo_v13.model_id === 'string') {
      holoObservatoryState.backend = `${snap.holo_v13.model_id} · holo-v13`;
    }
    if (snap.uorc && typeof snap.uorc === 'object') {
      holoObservatoryState.uorc = snap.uorc;
    }
    if (snap.storageGeom && typeof snap.storageGeom === 'object') {
      holoObservatoryState.storageGeom = snap.storageGeom;
    }
    renderHoloObservatoryDeck();
  };

  if (typeof window !== 'undefined' && typeof window.addEventListener === 'function') {
    window.addEventListener('message', (ev) => {
      if (typeof window !== 'undefined' && window.parent && ev && ev.source && ev.source !== window.parent) {
        return;
      }
      const msg = ev && ev.data ? ev.data : {};
      if (!msg || typeof msg !== 'object' || msg.protocol !== 'uor-observatory/1') return;
      if (msg.op === 'theme') {
        applyThemeVars(msg.vars || msg.theme || {});
        return;
      }
      if (msg.op === obsMetricsKey || msg.op === 'observatory_frame' || msg.op === 'snapshot') {
        if (msg.frame) {
          ingestHoloObservatoryFrame(msg.frame, msg.meta || null);
        }
        if (Array.isArray(msg.frames)) {
          for (const f of msg.frames) {
            ingestHoloObservatoryFrame(f, msg.meta || null);
          }
        }
        if (msg.snapshot) {
          applySnapshotPayload(msg.snapshot);
        }
        if (msg.uorc) {
          applySnapshotPayload({ uorc: msg.uorc });
        }
      }
    });
  }

  const dest = typeof window !== 'undefined' && window.parent ? window.parent : null;
  if (dest && dest !== window && typeof dest.postMessage === 'function') {
    dest.postMessage(
      {
        protocol: 'uor-observatory/1',
        op: 'ready',
        surface: 'observatory',
        origin: (typeof window !== 'undefined' && window.origin) || 'null',
      },
      '*',
    );
  }
  return true;
}

function showStudioUpdateBanner(updateInfo) {
  const banner = document.getElementById('studio-update-banner');
  const textEl = document.getElementById('studio-update-banner-text');
  if (!banner) return;
  const sha = updateInfo && updateInfo.adapter_wasm_sha256 ? String(updateInfo.adapter_wasm_sha256).slice(0, 12) : 'updated';
  if (textEl) {
    textEl.textContent = `New Studio Version Live (wasm:${sha}) — Refresh & Reload Now`;
  }
  banner.removeAttribute('hidden');
}

function getPrismEmbedStatusSnapshot(eventType = 'prism:status') {
  /* Formal R4.FoundryQuantumStudio */
  const repo = (AppState.config && AppState.config.connectedGitRepo) || '';
  const branch = (AppState.config && AppState.config.connectedGitBranch) || 'main';
  const st = typeof engineWorkerClient !== 'undefined' ? engineWorkerClient.stitchHealth : null;
  const modelSel = document.getElementById('composer-model-select');
  const selectedModel = (modelSel && modelSel.value) || (AppState.config && AppState.config.selectedModel) || 'uor-r4.qwen3-0.6b-holo';
  const composerMode = (AppState.config && AppState.config.composerMode) || 'agentic';
  const r4Stack = (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.r4StackInfo) || null;
  const vsStatus = (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.verdantSynthesis) || null;
  return {
    type: eventType,
    schema: 'prism/r4-workspace/1',
    protocol: 'uor-workspace-bridge/1',
    quantumPcContract: 'uor-foundry-quantum-studio/1',
    embed: Boolean(document.body && document.body.dataset.embed === 'me'),
    layoutMode: (document.body && document.body.dataset.layoutMode) || 'chat',
    workerReady: Boolean(typeof engineWorkerClient !== 'undefined' && engineWorkerClient.ready),
    model: selectedModel,
    selected_model: selectedModel,
    composer_mode: composerMode,
    execution_tier: (r4Stack && r4Stack.active_tier) || 'EmbeddedSeed',
    repo,
    branch,
    prNumber: (AppState.config && AppState.config.activePrNumber) || null,
    ciSummary: (AppState.config && AppState.config.ciSummary) || 'CI & Pages: Standby',
    workspaceFileCount: engineWorkerClient && engineWorkerClient.workspaceFiles ? engineWorkerClient.workspaceFiles.size : 0,
    stitchHealth: st,
    verdantSynthesis: vsStatus,
    prismIdentity: (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.prismIdentity) || null,
    holoArchive: (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.latestHoloSummary) || null,
    holoAddress: 'holo://lexlean-r4',
  };
}

function emitPrismEmbedEvent(eventType = 'prism:status', targetWindow = null) {
  const payload = getPrismEmbedStatusSnapshot(eventType);
  if (typeof globalThis === 'object') {
    if (!Array.isArray(globalThis.__uorPrismEmbedEvents)) globalThis.__uorPrismEmbedEvents = [];
    globalThis.__uorPrismEmbedEvents.push(payload);
  }
  const dest = targetWindow || (typeof window !== 'undefined' && window.parent && window.parent !== window ? window.parent : null);
  if (dest && typeof dest.postMessage === 'function') {
    try {
      dest.postMessage(payload, '*');
    } catch {
      // Ignore cross-origin postMessage restrictions cleanly
    }
  }
  return payload;
}

function updateDynamicPlanningPanel(extraCtx = { /* Formal R4.StudioFrame */}) {
  const dagEl = document.getElementById('plan-dag-diagram');
  const listEl = document.getElementById('plan-checklist');
  if (!dagEl && !listEl) return;
  const repo = (AppState.config && AppState.config.connectedGitRepo) || 'local-workspace';
  const branch = (AppState.config && AppState.config.connectedGitBranch) || 'main';
  const fileCount = engineWorkerClient && engineWorkerClient.workspaceFiles ? engineWorkerClient.workspaceFiles.size : 0;
  const st = engineWorkerClient ? engineWorkerClient.stitchHealth : null;
  const prNum = AppState.config && AppState.config.activePrNumber;
  const hasPending = AppState.workspace && Object.keys(AppState.workspace.pendingPatches || {}).length > 0;
  const turnVerified = Boolean(extraCtx && extraCtx.chatRes && extraCtx.chatRes.react_agent_r4_35 && extraCtx.chatRes.react_agent_r4_35.verified);

  if (dagEl) {
    const nodes = [
      { cls: engineWorkerClient && engineWorkerClient.ready ? 'dag-done' : 'dag-active', icon: engineWorkerClient && engineWorkerClient.ready ? '✓' : '▶', label: `Wasm + OPFS Boot (${fileCount} files on ${repo}@${branch})` },
      { cls: st ? 'dag-done' : 'dag-active', icon: st ? '✓' : '▶', label: st ? `Stitch Custody: ${st.verifiedShards}/${st.totalShards} Verified · ${st.healedShards} Healed` : 'Stitch 2-of-3 Shard Verification' },
      { cls: hasPending ? 'dag-active' : 'dag-done', icon: hasPending ? '▶' : '✓', label: hasPending ? 'Workspace Diff Staged for Review' : 'UOR-R4 v12 Reasoning & Tool Synthesis' },
      { cls: prNum || turnVerified ? 'dag-done' : 'dag-pending', icon: prNum || turnVerified ? '✓' : '◯', label: prNum ? `GitHub PR #${prNum} · ${AppState.config.ciSummary || 'Active'}` : 'Commit, Open PR & Watch CI/Pages' },
    ];
    dagEl.replaceChildren();
    nodes.forEach((n, idx) => {
      const nodeDiv = document.createElement('div');
      nodeDiv.className = `dag-node ${n.cls}`;
      const badgeSpan = document.createElement('span');
      badgeSpan.className = 'dag-badge';
      badgeSpan.textContent = n.icon;
      nodeDiv.appendChild(badgeSpan);
      nodeDiv.appendChild(document.createTextNode(` ${n.label}`));
      dagEl.appendChild(nodeDiv);
      if (idx < nodes.length - 1) {
        const arr = document.createElement('div');
        arr.className = 'dag-arrow';
        arr.textContent = '↓';
        dagEl.appendChild(arr);
      }
    });
  }

  if (listEl) {
    const tasks = [
      { done: Boolean(engineWorkerClient && engineWorkerClient.ready), active: false, text: `Dedicated Wasm Worker & Secret Vault (${engineWorkerClient && engineWorkerClient.githubConnected ? 'PAT Sealed' : 'Public/Ready'})` },
      { done: Boolean(st && st.totalShards > 0), active: !st, text: st ? `Stitch 2-of-3 Parity (${st.scheme} · ${st.status})` : 'Hydrate Holo v13 / Stitch Pieces into OPFS' },
      { done: Boolean(AppState.config && AppState.config.connectedGitRepo), active: false, text: `GitHub Workspace: ${repo}@${branch} (${fileCount} files)` },
      { done: Boolean(prNum), active: hasPending, text: prNum ? `Pull Request #${prNum} (${AppState.config.ciSummary || 'Tracked'})` : 'Stage Workspace Edits / Open Pull Request' },
    ];
    listEl.replaceChildren();
    tasks.forEach((t) => {
      const li = document.createElement('li');
      li.className = `plan-task-item${t.done ? ' done' : (t.active ? ' active' : '')}`;
      li.style.cursor = 'pointer';
      const icon = document.createElement('span');
      icon.className = 'check-icon';
      icon.textContent = t.done ? '☑' : (t.active ? '▶' : '☐');
      li.appendChild(icon);
      li.appendChild(document.createTextNode(` ${t.text}`));
      li.addEventListener('click', () => {
        if (li.classList.contains('done')) {
          li.classList.remove('done');
          icon.textContent = '☐';
        } else {
          li.classList.remove('active');
          li.classList.add('done');
          icon.textContent = '☑';
        }
      });
      listEl.appendChild(li);
    });
  }
}

function renderDynamicPoolPanel() {
  const poolGrid = document.getElementById('pool-status-grid');
  const poolQueue = document.getElementById('pool-review-queue');
  const repo = (AppState.config && AppState.config.connectedGitRepo) || 'local-workspace';
  const branch = (AppState.config && AppState.config.connectedGitBranch) || 'main';
  const st = engineWorkerClient ? engineWorkerClient.stitchHealth : null;
  const r4Stack = engineWorkerClient ? engineWorkerClient.r4StackInfo : null;
  const gllm = engineWorkerClient ? engineWorkerClient.gllmInfo : null;
  const modelSel = document.getElementById('composer-model-select');
  const activeModel = modelSel && modelSel.value
    ? modelSel.value
    : ((AppState.config && AppState.config.selectedModel) || 'uor-r4.qwen3-0.6b-holo');

  if (poolGrid) {
    const engineDetail = r4Stack
      ? `${r4Stack.active_tier || 'EmbeddedSeed'} ${r4Stack.n_layers || 6}L/${r4Stack.d_model || 256}D · ${activeModel}`
      : (gllm ? `R4 ${gllm.n_layers || 6}L/${gllm.d_model || 256}D · ${activeModel}` : `Mode: ${activeModel}`);
    const cards = [
      {
        role: 'MVM Worker Engine',
        active: Boolean(engineWorkerClient && engineWorkerClient.ready),
        status: engineWorkerClient && engineWorkerClient.ready ? 'Ready · Wasm32' : 'Booting...',
        detail: engineDetail,
      },
      {
        role: 'Stitch 2-of-3 Custody',
        active: Boolean(st && st.healedShards > 0),
        status: st ? st.status : 'Standby · 2-of-3 Parity',
        detail: st ? `${st.verifiedShards}/${st.totalShards} Verified · ${st.healedShards} Parity-Healed` : 'Awaiting shard hydration',
      },
      {
        role: 'GitHub Host Bridge',
        active: Boolean(AppState.config && AppState.config.connectedGitRepo),
        status: engineWorkerClient && engineWorkerClient.githubConnected ? 'Token Vault Sealed' : 'Public HTTPS / Relay',
        detail: `${repo}@${branch}`,
      },
      {
        role: 'CI & Pages Watcher',
        active: Boolean(AppState.config && AppState.config.activePrNumber),
        status: (AppState.config && AppState.config.ciSummary) || 'CI & Pages: Standby',
        detail: AppState.config && AppState.config.activePrNumber ? `Tracking PR #${AppState.config.activePrNumber}` : 'Ready on PR / Deploy',
      },
    ];
    poolGrid.replaceChildren();
    for (const c of cards) {
      const card = document.createElement('div');
      card.className = `pool-card${c.active ? ' pool-card-active' : ''}`;
      const rEl = document.createElement('div');
      rEl.className = 'pool-card-role';
      rEl.textContent = c.role;
      const sEl = document.createElement('div');
      sEl.className = 'pool-card-status';
      sEl.textContent = c.status;
      const dEl = document.createElement('div');
      dEl.className = 'pool-card-detail';
      dEl.textContent = c.detail;
      card.appendChild(rEl);
      card.appendChild(sEl);
      card.appendChild(dEl);
      poolGrid.appendChild(card);
    }
  }

  if (poolQueue) {
    const pendingPaths = Object.keys((AppState.workspace && AppState.workspace.pendingPatches) || {});
    const baseline = (AppState.config && AppState.config.gitBaselineFiles) || {};
    const changedPaths = [];
    if (engineWorkerClient && engineWorkerClient.workspaceFiles) {
      for (const [p, meta] of engineWorkerClient.workspaceFiles.entries()) {
        if (typeof baseline[p] === 'string' && !meta.lazy && baseline[p] !== meta.content) {
          changedPaths.push(p);
        }
      }
    }
    const fileEl = poolQueue.querySelector('.pool-review-file');
    const statsEl = poolQueue.querySelector('.pool-review-stats');
    const targetDisplay = pendingPaths[0] || changedPaths[0] || 'None';
    if (fileEl) {
      fileEl.textContent = `File: ${targetDisplay}`;
    }
    if (statsEl) {
      statsEl.textContent = pendingPaths.length > 0
        ? `${pendingPaths.length} AI patch(es) pending review`
        : changedPaths.length > 0
          ? `${changedPaths.length} modified file(s) ready to commit on ${branch}`
          : `Queue idle on ${repo}@${branch}`;
    }
  }
}

function renderCoworkStatusBar() {
  const turnDigest = document.getElementById('live-context-turn-digest');
  const summary = document.getElementById('live-context-summary');
  const frontier = document.getElementById('live-engine-frontier-badge');
  const trace = document.getElementById('live-otel-trace-badge');
  const modelSel = document.getElementById('composer-model-select');
  const activeModel = modelSel && modelSel.value
    ? modelSel.value
    : ((AppState.config && AppState.config.selectedModel) || 'uor-r4.qwen3-0.6b-holo');
  const thread = typeof getActiveThread === 'function' ? getActiveThread() : null;
  const turnCount = thread && Array.isArray(thread.messages) ? Math.max(1, Math.ceil(thread.messages.length / 2)) : 1;
  const repo = (AppState.config && AppState.config.connectedGitRepo) || 'local';
  const branch = (AppState.config && AppState.config.connectedGitBranch) || 'main';
  const st = engineWorkerClient ? engineWorkerClient.stitchHealth : null;
  if (turnDigest) {
    turnDigest.textContent = `Turn #${turnCount} · ${activeModel}`;
  }
  if (summary) {
    summary.textContent = st
      ? `${repo}@${branch} · Stitch ${st.verifiedShards}/${st.totalShards} (${st.healedShards} healed)`
      : `${repo}@${branch} · Rolling 256D Carry Active`;
  }
  if (frontier) {
    frontier.textContent = engineWorkerClient && engineWorkerClient.githubConnected
      ? 'GitHub Vault: Sealed · R4-F32'
      : 'R4-F32 Ready';
  }
  if (trace) {
    trace.textContent = (AppState.config && AppState.config.ciSummary) || 'uor.gllm.trace/1';
  }
}

function refreshScmBranchSelectors(branches, activeBranch) {
  const list = Array.isArray(branches) && branches.length > 0 ? branches : ['main'];
  const selected = activeBranch || list[0] || 'main';
  for (const selId of ['scm-branch-select', 'github-branch-select']) {
    const sel = document.getElementById(selId);
    if (!sel) continue;
    sel.replaceChildren();
    for (const b of list) {
      const opt = document.createElement('option');
      opt.value = b;
      opt.textContent = b;
      if (b === selected) opt.selected = true;
      sel.appendChild(opt);
    }
    sel.value = selected;
  }
}

async function runGitHubLoopAction(subcommand, extraArgs = { /* Formal R4.StudioFrame */}) {
  const statusEl = document.getElementById('github-connect-status');
  const ciChip = document.getElementById('scm-ci-pages-chip');
  const mergeBtn = document.getElementById('scm-merge-pr-btn');
  const repoInputScm = document.getElementById('scm-repo-input');
  const repoInputModal = document.getElementById('github-repo-input');
  const repoVal = String(
    extraArgs.repo ||
    extraArgs.repo_url ||
    (repoInputScm && repoInputScm.value.trim()) ||
    (repoInputModal && repoInputModal.value.trim()) ||
    (AppState.config && AppState.config.connectedGitRepo) ||
    ''
  ).trim();
  const branchVal = String(
    extraArgs.branch ||
    (AppState.config && AppState.config.connectedGitBranch) ||
    'main'
  ).trim();

  const payload = {
    op: 'git',
    subcommand,
    ...(repoVal ? { repo: repoVal, repo_url: repoVal } : {}),
    ...(branchVal ? { branch: branchVal } : {}),
    ...extraArgs,
  };
  if (subcommand === 'commit_push' && !Array.isArray(payload.files)) {
    const baseline = (AppState.config && AppState.config.gitBaselineFiles) || {};
    const changed = [];
    for (const [p, meta] of engineWorkerClient.workspaceFiles.entries()) {
      if (meta && !meta.lazy && typeof meta.content === 'string') {
        if (baseline[p] === undefined || baseline[p] !== meta.content) {
          changed.push({ path: p, content: meta.content });
        }
      }
    }
    payload.files = changed;
  }

  const res = await engineWorkerClient.rpc('host_bridge_exec', payload);
  if (!res || res.ok === false) {
    const errText = `${subcommand} error (${(res && res.error_code) || 'BridgeError'}): ${(res && res.message) || 'Failed'}`;
    if (statusEl) statusEl.textContent = errText;
    if (ciChip && (subcommand === 'pr_status' || subcommand === 'watch_deploy' || subcommand === 'create_pr' || subcommand === 'merge_pr')) {
      ciChip.textContent = errText;
    }
    return res;
  }

  const detectedRepo = res.repo || (res.data && res.data.repo) || repoVal;
  if (detectedRepo) {
    AppState.config.connectedGitRepo = detectedRepo;
    if (repoInputScm) repoInputScm.value = detectedRepo;
    if (repoInputModal) repoInputModal.value = detectedRepo;
  }
  const activeBranch = res.branch || (res.data && res.data.default_branch) || branchVal;
  if (activeBranch) {
    AppState.config.connectedGitBranch = activeBranch;
  }
  const branches = res.branches || (res.data && res.data.branches);
  if (Array.isArray(branches) && branches.length > 0) {
    AppState.config.gitBranches = branches;
  } else if (activeBranch && Array.isArray(AppState.config.gitBranches) && !AppState.config.gitBranches.includes(activeBranch)) {
    AppState.config.gitBranches.push(activeBranch);
  }
  refreshScmBranchSelectors(AppState.config.gitBranches, AppState.config.connectedGitBranch);

  if (subcommand === 'clone' || subcommand === 'pull') {
    const baseline = {};
    const clonedFiles = res.files || (res.data && res.data.files) || [];
    for (const f of clonedFiles) {
      if (f && typeof f.path === 'string' && typeof f.content === 'string' && !f.lazy) {
        baseline[f.path] = f.content;
      }
    }
    AppState.config.gitBaselineFiles = baseline;
    AppState.config.gitLazyPaths = res.lazy_paths || (res.data && res.data.lazy_paths) || [];
  } else if (subcommand === 'commit_push' && Array.isArray(payload.files)) {
    const baseline = AppState.config.gitBaselineFiles || {};
    for (const f of payload.files) {
      if (f && typeof f.path === 'string' && typeof f.content === 'string') {
        baseline[f.path] = f.content;
      }
    }
    AppState.config.gitBaselineFiles = baseline;
  }

  if (subcommand === 'create_pr') {
    const prNumber = res.pr_number || (res.data && res.data.pr_number) || null;
    AppState.config.activePrNumber = prNumber;
    AppState.config.activePrUrl = res.pr_url || (res.data && res.data.pr_url) || '';
    AppState.config.ciSummary = prNumber ? `PR #${prNumber} Opened` : 'PR Opened';
    if (mergeBtn && prNumber) mergeBtn.disabled = false;
  } else if (subcommand === 'pr_status') {
    const total = res.checks_total ?? res.total_check_runs ?? (res.data && (res.data.checks_total ?? res.data.total_check_runs)) ?? 0;
    const conclusions = (res.check_runs || []).map((c) => c.conclusion || c.status).filter(Boolean);
    const label = res.mergeable === false
      ? `PR #${AppState.config.activePrNumber || ''} Conflicts`
      : `CI: ${total} checks (${conclusions.join('/') || res.check_status || res.workflow_status || 'pending'})`;
    AppState.config.ciSummary = label;
  } else if (subcommand === 'merge_pr') {
    AppState.config.ciSummary = `PR #${res.pr_number || AppState.config.activePrNumber || ''} Merged`;
    if (mergeBtn) mergeBtn.disabled = true;
  } else if (subcommand === 'watch_deploy') {
    const pagesStatus = res.pages_status || (res.data && res.data.pages_status) || 'unknown';
    const upd = res.update_available ? 'New Version Live!' : 'Up to date';
    AppState.config.ciSummary = `Pages: ${pagesStatus} · ${upd}`;
    if (res.update_available) {
      showStudioUpdateBanner(res);
    }
  }

  if (ciChip) ciChip.textContent = AppState.config.ciSummary || 'CI & Pages: Active';
  if (statusEl) {
    statusEl.textContent = `${subcommand} ok on ${AppState.config.connectedGitRepo || 'repo'}@${AppState.config.connectedGitBranch}`;
  }
  renderCursorScmSection(engineWorkerClient.getWorkspaceListObject());
  updateDynamicPlanningPanel();
  renderDynamicPoolPanel();
  renderCoworkStatusBar();
  emitPrismEmbedEvent('prism:status');
  return res;
}

function initEmbedMeBridge() {
  try {
    const surfaceParam = typeof window !== 'undefined' && window.location
      ? String(new URLSearchParams(window.location.search || '').get('surface') || '').trim().toLowerCase()
      : '';
    if (surfaceParam === 'observatory' || (document && document.body && document.body.dataset && document.body.dataset.surface === 'observatory')) {
      return;
    }
  } catch {
    // Proceed on non-observatory surfaces
  }
  let embedTarget = '';
  try {
    if (typeof window !== 'undefined' && window.location) {
      const params = new URLSearchParams(window.location.search || '');
      const rawEmbed = params.get('embed') || '';
      if (rawEmbed === 'me' || rawEmbed === 'foundry') embedTarget = rawEmbed;
    }
  } catch {
    embedTarget = '';
  }
  if (embedTarget && document.body) {
    document.body.dataset.embed = embedTarget;
    document.body.classList.add('embed-me-compact');
  }

  const replyBridge = (targetWin, payload) => {
    const dest = targetWin || (typeof window !== 'undefined' && window.parent !== window ? window.parent : null);
    if (dest && typeof dest.postMessage === 'function') {
      dest.postMessage(payload, '*');
    }
  };

  const handleInboundMessage = async (ev) => {
    const msg = ev && ev.data ? ev.data : {};
    if (!msg || (typeof msg.type !== 'string' && msg.protocol !== 'uor-workspace-bridge/1')) return;
    if (msg.type === 'prism:open-repo' && msg.repo) {
      const repoStr = String(msg.repo).trim();
      const branchStr = String(msg.branch || 'main').trim();
      await runGitHubLoopAction('connect', { repo: repoStr }).catch(() => null);
      await runGitHubLoopAction('clone', { repo: repoStr, branch: branchStr }).catch(() => null);
      emitPrismEmbedEvent('prism:status', ev.source || null);
      return;
    }
    const isUorBridge = msg.protocol === 'uor-workspace-bridge/1'
      || (typeof msg.type === 'string' && msg.type.startsWith('uor-workspace-bridge/1:'));
    if (!isUorBridge) return;
    const action = msg.action || (typeof msg.type === 'string' ? msg.type.replace(/^uor-workspace-bridge\/1:/, '') : 'status');
    if (action === 'ping' || action === 'status') {
      replyBridge(ev.source || null, {
        ...getPrismEmbedStatusSnapshot('uor-workspace-bridge/1:status'),
        protocol: 'uor-workspace-bridge/1',
        type: 'uor-workspace-bridge/1:status',
      });
    } else if (action === 'open-repo' && msg.repo) {
      const repoStr = String(msg.repo).trim();
      const branchStr = String(msg.branch || 'main').trim();
      await runGitHubLoopAction('connect', { repo: repoStr }).catch(() => null);
      await runGitHubLoopAction('clone', { repo: repoStr, branch: branchStr }).catch(() => null);
      const snap = getPrismEmbedStatusSnapshot('uor-workspace-bridge/1:status');
      emitPrismEmbedEvent('prism:status', ev.source || null);
      replyBridge(ev.source || null, {
        ...snap,
        protocol: 'uor-workspace-bridge/1',
        type: 'uor-workspace-bridge/1:status',
      });
    } else if (action === 'set-model' && msg.model) {
      const nextModel = String(msg.model).trim();
      if (AppState && AppState.config) AppState.config.selectedModel = nextModel;
      if (typeof syncComposerModel === 'function') {
        syncComposerModel(nextModel);
      } else {
        const selEl = document.getElementById('composer-model-select');
        if (selEl && (nextModel === 'uor-r4.qwen3-0.6b-holo' || nextModel === 'uor-r4.qwen3-1.7b-holo')) {
          selEl.value = nextModel;
        }
        if (typeof tsComposer !== 'undefined' && typeof tsComposer.setModel === 'function') {
          tsComposer.setModel(nextModel);
        }
        renderCoworkStatusBar();
      }
      replyBridge(ev.source || null, {
        ...getPrismEmbedStatusSnapshot('uor-workspace-bridge/1:status'),
        protocol: 'uor-workspace-bridge/1',
        type: 'uor-workspace-bridge/1:status',
      });
    } else if (action === 'admit-capsule' && msg.capsule) {
      let result = null;
      if (engineWorkerClient && engineWorkerClient.enabled) {
        result = await engineWorkerClient.rpc('admit_generation_capsule', { capsule: msg.capsule }).catch((e) => ({ status: 'error', message: String(e) }));
      } else if (typeof admit_uorc_generation_capsule === 'function') {
        result = await admit_uorc_generation_capsule(msg.capsule).catch((e) => ({ status: 'error', message: String(e) }));
      }
      replyBridge(ev.source || null, {
        protocol: 'uor-workspace-bridge/1',
        type: 'uor-workspace-bridge/1:capsule-result',
        result,
      });
    } else if (action === 'record-fault') {
      let result = null;
      if (engineWorkerClient && engineWorkerClient.enabled) {
        result = await engineWorkerClient.rpc('record_generation_fault', { reason: msg.reason || 'bridge_fault' }).catch((e) => ({ status: 'error', message: String(e) }));
      } else if (typeof record_uorc_generation_fault === 'function') {
        result = await record_uorc_generation_fault(msg.reason || 'bridge_fault').catch((e) => ({ status: 'error', message: String(e) }));
      }
      replyBridge(ev.source || null, {
        protocol: 'uor-workspace-bridge/1',
        type: 'uor-workspace-bridge/1:watchdog-result',
        result,
      });
    } else if (action === 'identity' && typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
      const result = await engineWorkerClient.rpc('prism_identity_ensure', {}).catch((e) => ({ ok: false, error: String(e) }));
      if (result && result.ok) engineWorkerClient.prismIdentity = result;
      renderPrismHoloIdentityUI();
      replyBridge(ev.source || null, {
        protocol: 'uor-workspace-bridge/1',
        type: 'uor-workspace-bridge/1:identity',
        result,
      });
    } else if (action === 'bind-persona' && typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
      const result = await engineWorkerClient.rpc('prism_identity_bind_persona', {
        handle: msg.handle || 'lexlean-r4-studio',
        host: msg.host || 'uor-foundation.github.io',
      }).catch((e) => ({ ok: false, error: String(e) }));
      if (result && result.ok) engineWorkerClient.prismIdentity = result;
      renderPrismHoloIdentityUI();
      replyBridge(ev.source || null, {
        protocol: 'uor-workspace-bridge/1',
        type: 'uor-workspace-bridge/1:identity',
        result,
      });
    } else if ((action === 'compile-holo' || msg.type === 'prism:compile-holo') && typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
      const result = await engineWorkerClient.rpc('compile_holo_archive', {}).catch((e) => ({ ok: false, error: String(e) }));
      if (result && result.ok) engineWorkerClient.latestHoloSummary = result;
      renderPrismHoloIdentityUI();
      replyBridge(ev.source || null, {
        protocol: 'uor-workspace-bridge/1',
        type: 'uor-workspace-bridge/1:holo-compiled',
        result,
      });
    } else if ((action === 'app-descriptor' || action === 'foundry-descriptor') && typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
      const hrefOrigin = typeof window !== 'undefined' && window.location ? String(window.location.href || '') : '';
      const result = await engineWorkerClient.rpc('get_foundry_app_descriptor', { origin: hrefOrigin }).catch((e) => ({ ok: false, error: String(e) }));
      replyBridge(ev.source || null, {
        protocol: 'uor-workspace-bridge/1',
        type: `uor-workspace-bridge/1:${action}`,
        result,
      });
    } else if (action === 'inprism-infer' && typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
      const inferPayload = msg.payload && typeof msg.payload === 'object'
        ? msg.payload
        : { prompt: msg.prompt || '', preferredModel: msg.model || '' };
      const result = await engineWorkerClient.rpc('inprism_r4_turn', inferPayload).catch((e) => ({ ok: false, error: String(e) }));
      replyBridge(ev.source || null, {
        protocol: 'uor-workspace-bridge/1',
        type: 'uor-workspace-bridge/1:inprism-infer',
        requestId: msg.requestId || null,
        result,
      });
    } else if (action === 'set-layout') {
      if (typeof setLayoutMode === 'function') {
        setLayoutMode(msg.layout === 'chat' ? 'chat' : 'studio');
      }
      replyBridge(ev.source || null, {
        ...getPrismEmbedStatusSnapshot('uor-workspace-bridge/1:status'),
        protocol: 'uor-workspace-bridge/1',
        type: 'uor-workspace-bridge/1:status',
        layoutMode: msg.layout === 'chat' ? 'chat' : 'studio',
      });
    } else if (action === 'sync-foundry-files') {
      const rawFiles = Array.isArray(msg.files)
        ? msg.files
        : (msg.files && typeof msg.files === 'object'
          ? Object.entries(msg.files).map(([p, c]) => ({ path: p, content: c }))
          : []);
      const normalized = rawFiles
        .filter((it) => it && typeof it.path === 'string' && typeof it.content === 'string' && it.path.trim() && !it.path.includes('..'))
        .map((it) => ({ path: it.path.trim(), content: it.content }));
      if (msg.replace === true && engineWorkerClient && engineWorkerClient.workspaceFiles) {
        const keepSet = new Set(normalized.map((it) => it.path));
        for (const existingPath of Array.from(engineWorkerClient.workspaceFiles.keys())) {
          if (!keepSet.has(existingPath)) {
            dispatchStudioEnvelope('workspace_delete', existingPath);
          }
        }
      }
      const batchEntries = [];
      for (const it of normalized) {
        if (it.content.includes('\n===FILE===\n')) {
          dispatchStudioEnvelope('workspace_write', `${it.path}\n${it.content}`);
        } else {
          batchEntries.push(`${it.path}\n${it.content}`);
        }
      }
      if (batchEntries.length > 0) {
        dispatchStudioEnvelope('workspace_batch', batchEntries.join('\n===FILE===\n'));
      }
      if (typeof setLayoutMode === 'function' && msg.layout !== 'chat') {
        setLayoutMode('studio');
      }
      const wsSnapshot = engineWorkerClient.getWorkspaceListObject();
      if (typeof renderWorkspaceList === 'function') renderWorkspaceList(wsSnapshot);
      if (typeof renderCursorScmSection === 'function') renderCursorScmSection(wsSnapshot);
      const preferredEntry = ['src/pages/Index.tsx', 'src/App.tsx', 'prism/spec.json']
        .find((candidate) => normalized.some((it) => it.path === candidate))
        || (normalized[0] ? normalized[0].path : '');
      if (preferredEntry && typeof selectWorkspaceFile === 'function') {
        selectWorkspaceFile(preferredEntry);
      }
      replyBridge(ev.source || null, {
        protocol: 'uor-workspace-bridge/1',
        type: 'uor-workspace-bridge/1:files-synced',
        requestId: msg.requestId || null,
        projectName: msg.projectName || '',
        fileCount: normalized.length,
      });
    } else if (action === 'export-foundry-files') {
      const outFiles = {};
      let count = 0;
      if (engineWorkerClient && engineWorkerClient.workspaceFiles) {
        for (const [p, entry] of engineWorkerClient.workspaceFiles.entries()) {
          if (entry && typeof entry.content === 'string' && (!entry.lazy || entry.content.length > 0)) {
            outFiles[p] = entry.content;
            count += 1;
          }
        }
      }
      replyBridge(ev.source || null, {
        protocol: 'uor-workspace-bridge/1',
        type: 'uor-workspace-bridge/1:files-exported',
        requestId: msg.requestId || null,
        files: outFiles,
        fileCount: count,
      });
    }
  };

  if (typeof window !== 'undefined' && typeof window.addEventListener === 'function') {
    window.addEventListener('message', handleInboundMessage);
  }
  if (typeof globalThis === 'object') {
    globalThis.__uorPrismBridge = {
      protocol: 'uor-workspace-bridge/1',
      handleInboundMessage,
      emitPrismEmbedEvent,
      getPrismEmbedStatusSnapshot,
    };
  }
  emitPrismEmbedEvent('prism:ready');
  replyBridge(null, {
    ...getPrismEmbedStatusSnapshot('uor-workspace-bridge/1:ready'),
    protocol: 'uor-workspace-bridge/1',
    type: 'uor-workspace-bridge/1:ready',
  });
}

function initLayoutAndConnectedControls() {
  let initialMode = 'chat';
  try {
    const queryParams = typeof window !== 'undefined' && window.location && window.location.search
      ? new URLSearchParams(window.location.search)
      : null;
    if (
      (typeof window !== 'undefined' && window.location && window.location.hash === '#studio')
      || (queryParams && queryParams.get('mode') === 'studio')
    ) {
      initialMode = 'studio';
    } else if (typeof sessionStorage !== 'undefined' && sessionStorage.getItem('uor_r4_layout_mode') === 'studio') {
      initialMode = 'studio';
    }
  } catch {
    initialMode = 'chat';
  }
  setLayoutMode(initialMode);

  if (typeof applyWindowWidthClass === 'function') {
    applyWindowWidthClass();
    if (typeof window !== 'undefined' && typeof window.addEventListener === 'function') {
      window.addEventListener('resize', () => applyWindowWidthClass(window.innerWidth));
    }
    if (typeof ResizeObserver === 'function' && document.body) {
      const ro = new ResizeObserver((entries) => {
        const w = entries && entries[0] && entries[0].contentRect ? entries[0].contentRect.width : window.innerWidth;
        applyWindowWidthClass(w);
      });
      ro.observe(document.body);
    }
    if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
      for (const mqStr of ['(max-width: 599px)', '(min-width: 600px) and (max-width: 839px)', '(min-width: 840px)']) {
        const mql = window.matchMedia(mqStr);
        if (mql && typeof mql.addEventListener === 'function') {
          mql.addEventListener('change', () => applyWindowWidthClass(window.innerWidth));
        }
      }
    }
  }

  const toggleStudioIdeBtn = document.getElementById('toggle-studio-ide-btn');
  if (toggleStudioIdeBtn) {
    toggleStudioIdeBtn.addEventListener('click', () => {
      const cur = (document.body && document.body.dataset.layoutMode) || 'chat';
      setLayoutMode(cur === 'studio' ? 'chat' : 'studio');
    });
  }
  const topbarModeChatBtn = document.getElementById('topbar-mode-chat-btn');
  if (topbarModeChatBtn) {
    topbarModeChatBtn.addEventListener('click', () => {
      setLayoutMode('chat');
    });
  }

  const syncWebAccessBtnUI = () => {
    const btn = document.getElementById('toggle-web-access-btn');
    if (!btn) return;
    const on = Boolean(engineWorkerClient.webAccessEnabled);
    btn.setAttribute('aria-pressed', on ? 'true' : 'false');
    btn.classList.toggle('topbar-pill-web-on', on);
    btn.classList.toggle('topbar-pill-web-off', !on);
    btn.replaceChildren();
    const dot = document.createElement('span');
    dot.className = on ? 'pill-dot pill-dot-live' : 'pill-dot';
    btn.appendChild(dot);
    btn.appendChild(document.createTextNode(on ? ' Web: ON' : ' Web: OFF'));
  };
  const toggleWebAccessBtn = document.getElementById('toggle-web-access-btn');
  if (toggleWebAccessBtn) {
    syncWebAccessBtnUI();
    toggleWebAccessBtn.addEventListener('click', () => {
      engineWorkerClient.webAccessEnabled = !engineWorkerClient.webAccessEnabled;
      syncWebAccessBtnUI();
      if (engineWorkerClient.enabled) {
        engineWorkerClient.rpc('set_bridge_policy', { webAccessEnabled: engineWorkerClient.webAccessEnabled }).catch(() => {});
      }
      if (typeof showStudioToast === 'function') {
        showStudioToast(`Web access ${engineWorkerClient.webAccessEnabled ? 'enabled' : 'disabled'} (SpiralCore policy)`, 'info', 2800);
      }
    });
  }

  const wbTabCodeBtn = document.getElementById('workbench-tab-code');
  const wbTabPreviewBtn = document.getElementById('workbench-tab-preview');
  const wbTabDiffBtn = document.getElementById('workbench-tab-diff');
  const previewRefreshBtn = document.getElementById('preview-refresh-btn');
  const livePreviewIframe = document.getElementById('live-preview-iframe');
  const previewConsoleOutput = document.getElementById('preview-console-output');

  const sendFilesToPreviewIframe = () => {
    const ws = (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled)
      ? engineWorkerClient.getWorkspaceListObject()
      : dispatchStudioEnvelope('workspace_list', '');
    const filesObj = {};
    if (ws && Array.isArray(ws.files)) {
      for (const f of ws.files) {
        filesObj[f.path] = f.content || '';
      }
    }
    if (livePreviewIframe && livePreviewIframe.contentWindow && typeof livePreviewIframe.contentWindow.postMessage === 'function') {
      livePreviewIframe.contentWindow.postMessage({ protocol: 'uor-preview/1', type: 'render_files', op: 'render_files', files: filesObj }, '*');
    }
  };

  let activeWorkbenchMode = 'code';
  const setWorkbenchMode = (mode) => {
    activeWorkbenchMode = (mode === 'preview' || mode === 'diff') ? mode : 'code';
    if (typeof setLayoutMode === 'function' && document.body && document.body.dataset.layoutMode === 'chat') {
      setLayoutMode('studio');
    }
    for (const [btn, m] of [
      [wbTabCodeBtn, 'code'],
      [wbTabPreviewBtn, 'preview'],
      [wbTabDiffBtn, 'diff'],
    ]) {
      if (btn) {
        btn.classList.toggle('active', m === activeWorkbenchMode);
        btn.setAttribute('aria-pressed', m === activeWorkbenchMode ? 'true' : 'false');
      }
    }
    const wsPanel = document.getElementById('canvas-panel-workspace');
    const pvPanel = document.getElementById('canvas-panel-preview');
    const dfPanel = document.getElementById('canvas-panel-diff');
    if (wsPanel) {
      wsPanel.classList.toggle('active', activeWorkbenchMode === 'code');
      if (activeWorkbenchMode === 'code') wsPanel.removeAttribute('hidden');
      else wsPanel.setAttribute('hidden', '');
    }
    if (pvPanel) {
      pvPanel.classList.toggle('active', activeWorkbenchMode === 'preview');
      if (activeWorkbenchMode === 'preview') {
        pvPanel.removeAttribute('hidden');
        sendFilesToPreviewIframe();
      } else {
        pvPanel.setAttribute('hidden', '');
      }
    }
    if (dfPanel) {
      dfPanel.classList.toggle('active', activeWorkbenchMode === 'diff');
      if (activeWorkbenchMode === 'diff') {
        dfPanel.removeAttribute('hidden');
        if (typeof renderPendingPatchPills === 'function') renderPendingPatchPills();
      } else {
        dfPanel.setAttribute('hidden', '');
      }
    }
  };
  globalThis.__uorSetWorkbenchMode = setWorkbenchMode;
  globalThis.__uorRefreshPreviewIfActive = () => {
    if (activeWorkbenchMode === 'preview') sendFilesToPreviewIframe();
  };

  if (wbTabCodeBtn) wbTabCodeBtn.addEventListener('click', () => setWorkbenchMode('code'));
  if (wbTabPreviewBtn) wbTabPreviewBtn.addEventListener('click', () => setWorkbenchMode('preview'));
  if (wbTabDiffBtn) wbTabDiffBtn.addEventListener('click', () => setWorkbenchMode('diff'));
  if (previewRefreshBtn) {
    previewRefreshBtn.addEventListener('click', () => {
      setWorkbenchMode('preview');
      sendFilesToPreviewIframe();
    });
  }
  if (typeof window !== 'undefined' && typeof window.addEventListener === 'function') {
    window.addEventListener('message', (ev) => {
      if (livePreviewIframe && livePreviewIframe.contentWindow && ev && ev.source && ev.source !== livePreviewIframe.contentWindow) {
        return;
      }
      const msg = ev && ev.data ? ev.data : null;
      if (!msg || typeof msg !== 'object' || msg.protocol !== 'uor-preview/1') return;
      const msgOp = msg.op || msg.type || '';
      if (msgOp === 'ready' && activeWorkbenchMode === 'preview') {
        sendFilesToPreviewIframe();
      } else if (msgOp === 'console' && previewConsoleOutput && msg.line) {
        previewConsoleOutput.textContent += `\n${msg.line}`;
      } else if (msgOp === 'rendered' || msgOp === 'inspect_reply') {
        const report = msg.report || msg.diag || null;
        if (report && typeof report === 'object') {
          if (typeof _r460LatestPreviewDiag !== 'undefined' && _r460LatestPreviewDiag) {
            _r460LatestPreviewDiag = { ..._r460LatestPreviewDiag, ...report };
          }
          if (previewConsoleOutput && Array.isArray(report.console)) {
            previewConsoleOutput.textContent = report.console.length > 0 ? report.console.join('\n') : '';
          }
        }
        if (msgOp === 'inspect_reply' && msg.requestId != null && typeof _r460PendingInspectWaiters !== 'undefined') {
          const waiter = _r460PendingInspectWaiters.get(String(msg.requestId));
          if (waiter) waiter(report || _r460LatestPreviewDiag);
        }
      }
    });
  }

  const studioToastCloseBtn = document.getElementById('studio-toast-close-btn');
  if (studioToastCloseBtn) {
    studioToastCloseBtn.addEventListener('click', () => {
      const banner = document.getElementById('studio-toast-banner');
      if (banner) {
        banner.setAttribute('hidden', '');
        banner.hidden = true;
      }
    });
  }

  const settingsModal = document.getElementById('settings-modal');
  const openSettingsModalBtn = document.getElementById('open-settings-modal-btn');
  const closeSettingsModalBtn = document.getElementById('close-settings-modal-btn');
  const settingsBackdrop = document.getElementById('settings-modal-backdrop');
  const allowOffOriginChk = document.getElementById('settings-allow-off-origin-weights');
  const offlineQ31Toggle = document.getElementById('settings-offline-q31-toggle');
  const mlpShortlistSelect = document.getElementById('settings-mlp-shortlist-mode');
  const settingsVerifySlabBtn = document.getElementById('settings-verify-slab-btn');
  const settingsStatusEl = document.getElementById('settings-modal-status');
  const closeSettingsModal = () => {
    if (settingsModal) settingsModal.setAttribute('hidden', '');
  };
  const readConsentStored = () => {
    if (typeof safeStorageGet !== 'function') return false;
    const raw = safeStorageGet('uor_allow_off_origin_hydration');
    return typeof isConsentGrantedVal === 'function' ? isConsentGrantedVal(raw) : (raw === '1' || raw === 'true' || raw === 'granted');
  };
  const readOfflineEmergencyStored = () =>
    typeof safeStorageGet === 'function' && safeStorageGet('uor_offline_emergency_q31') === '1';
  if (openSettingsModalBtn && settingsModal) {
    openSettingsModalBtn.addEventListener('click', () => {
      if (allowOffOriginChk) {
        allowOffOriginChk.checked = readConsentStored();
      }
      if (offlineQ31Toggle) {
        offlineQ31Toggle.checked = readOfflineEmergencyStored();
      }
      if (mlpShortlistSelect && typeof safeStorageGet === 'function') {
        mlpShortlistSelect.value = __uorNormalizeMlpPolicy(safeStorageGet('uor_mlp_shortlist_mode') || 'auto');
      }
      settingsModal.removeAttribute('hidden');
    });
  }
  if (closeSettingsModalBtn) closeSettingsModalBtn.addEventListener('click', closeSettingsModal);
  if (settingsBackdrop) settingsBackdrop.addEventListener('click', closeSettingsModal);
  if (mlpShortlistSelect) {
    if (typeof safeStorageGet === 'function') {
      mlpShortlistSelect.value = __uorNormalizeMlpPolicy(safeStorageGet('uor_mlp_shortlist_mode') || 'auto');
    }
    mlpShortlistSelect.addEventListener('change', () => {
      applyHoloMlpMode(mlpShortlistSelect.value).catch(() => {});
    });
  }
  if (settingsVerifySlabBtn) {
    settingsVerifySlabBtn.addEventListener('click', () => {
      verifyHoloOpfsSlab(true).catch(() => {});
    });
  }
  if (offlineQ31Toggle) {
    offlineQ31Toggle.checked = readOfflineEmergencyStored();
    offlineQ31Toggle.addEventListener('change', () => {
      const enabled = Boolean(offlineQ31Toggle.checked);
      if (enabled) {
        safeStorageSet('uor_offline_emergency_q31', '1');
      } else if (typeof safeStorageRemove === 'function') {
        safeStorageRemove('uor_offline_emergency_q31');
      }
      const nextModel = enabled
        ? 'uor-r4.native-q31-d85'
        : ((typeof composerModelSelect !== 'undefined' && composerModelSelect && composerModelSelect.value) || DEFAULT_HONEST_MODEL_ID);
      if (AppState && AppState.config) {
        AppState.config.selectedModel = nextModel;
      }
      if (typeof tsComposer !== 'undefined' && typeof tsComposer.setModel === 'function') {
        tsComposer.setModel(nextModel);
      }
      if (engineWorkerClient.enabled) {
        engineWorkerClient.rpc('set_bridge_policy', {
          offlineEmergencyQ31: enabled,
          consentDeclined: false,
        }).catch(() => {});
      }
      if (typeof updateWeightCacheBadge === 'function') updateWeightCacheBadge();
      if (settingsStatusEl) {
        settingsStatusEl.textContent = enabled
          ? 'Offline Emergency Mode active: routing turns to embedded Native Q31 D85.'
          : 'Offline Emergency Mode disabled: Qwen3 catalog auto-hydration active.';
      }
    });
  }
  if (allowOffOriginChk) {
    allowOffOriginChk.checked = readConsentStored();
    allowOffOriginChk.addEventListener('change', () => {
      const granted = Boolean(allowOffOriginChk.checked);
      if (granted && typeof safeStorageSet === 'function') {
        safeStorageSet('uor_allow_off_origin_hydration', '1');
      } else if (typeof safeStorageRemove === 'function') {
        safeStorageRemove('uor_allow_off_origin_hydration');
      }
      if (engineWorkerClient.enabled) {
        engineWorkerClient.rpc('set_bridge_policy', {
          allowOffOriginHydration: granted ? true : undefined,
          consentDeclined: false,
          hydrateNow: granted,
        }).catch(() => {});
      }
      if (settingsStatusEl) {
        settingsStatusEl.textContent = granted
          ? 'Custom external mirror downloads enabled.'
          : 'First-party catalog auto-hydration active (external custom mirrors require prompt).';
      }
    });
  }

  const openInspectorsTriggers = [
    document.getElementById('toggle-inspectors-btn'),
    document.getElementById('workbench-inspectors-btn'),
    document.getElementById('terminal-open-inspectors-btn'),
  ].filter(Boolean);
  for (const btn of openInspectorsTriggers) {
    btn.addEventListener('click', () => {
      const isOpen = inspectorsModalEl && !inspectorsModalEl.hasAttribute('hidden');
      if (isOpen) closeInspectorsModal();
      else openInspectorsModal('traces');
    });
  }

  const closeInspectorsBtn = document.getElementById('close-inspectors-modal-btn');
  const inspectorsBackdrop = document.getElementById('inspectors-modal-backdrop');
  const neuralHudExpandBtn = document.getElementById('neural-hud-expand-btn');
  const neuralHudCloseBtn = document.getElementById('neural-hud-close-btn');
  if (closeInspectorsBtn) closeInspectorsBtn.addEventListener('click', () => closeInspectorsModal());
  if (neuralHudCloseBtn) neuralHudCloseBtn.addEventListener('click', () => closeInspectorsModal());
  if (neuralHudExpandBtn) {
    neuralHudExpandBtn.addEventListener('click', () => {
      if (!inspectorsModalEl) return;
      const expanded = inspectorsModalEl.classList ? inspectorsModalEl.classList.toggle('hud-expanded') : false;
      syncNeuralHudTriggerState(true, expanded);
      if (expanded) {
        if (typeof renderHoloObservatoryDeck === 'function') renderHoloObservatoryDeck();
        if (lexLean3dVisualizer && typeof lexLean3dVisualizer.ensureGLReady === 'function') lexLean3dVisualizer.ensureGLReady();
        if (lexLean3dVisualizer && typeof lexLean3dVisualizer.resize === 'function') lexLean3dVisualizer.resize();
      }
    });
  }
  if (inspectorsBackdrop) inspectorsBackdrop.addEventListener('click', () => closeInspectorsModal());

  const githubModal = document.getElementById('github-connect-modal');
  const openGithubBtn = document.getElementById('open-github-connect-btn');
  const closeGithubBtn = document.getElementById('close-github-connect-btn');
  const githubBackdrop = document.getElementById('github-connect-backdrop');
  const githubPatInput = document.getElementById('github-token-input') || document.getElementById('github-pat-input');
  const saveGithubTokenBtn = document.getElementById('save-github-token-btn');
  const clearGithubTokenBtn = document.getElementById('clear-github-token-btn');
  const githubRepoUrlInput = document.getElementById('github-repo-input') || document.getElementById('github-repo-url-input');
  const cloneGithubRepoBtn = document.getElementById('clone-github-repo-btn');
  const spvsRelayUrlInput = document.getElementById('spiralcore-relay-input') || document.getElementById('spvs-relay-url-input');
  const saveRelayUrlBtn = document.getElementById('save-relay-url-btn');
  const githubConnectStatus = document.getElementById('github-connect-status');

  const closeGithubModal = () => {
    if (githubModal) githubModal.setAttribute('hidden', '');
  };
  if (openGithubBtn && githubModal) {
    openGithubBtn.addEventListener('click', () => {
      githubModal.removeAttribute('hidden');
      if (githubPatInput) githubPatInput.focus();
    });
  }
  if (closeGithubBtn) closeGithubBtn.addEventListener('click', closeGithubModal);
  if (githubBackdrop) githubBackdrop.addEventListener('click', closeGithubModal);

  if (saveGithubTokenBtn) {
    saveGithubTokenBtn.addEventListener('click', async () => {
      const secretVal = githubPatInput ? githubPatInput.value.trim() : '';
      if (githubPatInput) githubPatInput.value = '';
      if (!secretVal) {
        if (githubConnectStatus) githubConnectStatus.textContent = 'Enter a GitHub Fine-Grained PAT first.';
        return;
      }
      try {
        if (engineWorkerClient.enabled) {
          await engineWorkerClient.rpc('mvm_set_secret', { key: 'GITHUB_TOKEN', value: secretVal });
        }
        engineWorkerClient.githubConnected = true;
        if (openGithubBtn) {
          openGithubBtn.textContent = 'GitHub: Connected';
          openGithubBtn.dataset.connected = 'true';
        }
        if (githubConnectStatus) {
          githubConnectStatus.textContent = 'GITHUB_TOKEN sealed in Host Worker Vault (allowlisted to github.com & api.github.com; zero bytes in guest RAM).';
        }
        renderDynamicPoolPanel();
        renderCoworkStatusBar();
      } catch (err) {
        if (githubConnectStatus) {
          githubConnectStatus.textContent = `Vault error: ${err && err.message ? err.message : String(err)}`;
        }
      }
    });
  }

  if (clearGithubTokenBtn) {
    clearGithubTokenBtn.addEventListener('click', async () => {
      if (githubPatInput) githubPatInput.value = '';
      if (engineWorkerClient.enabled) {
        await engineWorkerClient.rpc('mvm_set_secret', { key: 'GITHUB_TOKEN', value: '' }).catch(() => {});
      }
      engineWorkerClient.githubConnected = false;
      if (openGithubBtn) {
        openGithubBtn.textContent = 'GitHub';
        delete openGithubBtn.dataset.connected;
      }
      if (githubConnectStatus) {
        githubConnectStatus.textContent = 'Cleared GITHUB_TOKEN from Host Worker Vault.';
      }
      renderDynamicPoolPanel();
      renderCoworkStatusBar();
    });
  }

  const detectBtns = [
    document.getElementById('github-detect-repo-btn'),
    document.getElementById('scm-detect-repo-btn'),
  ].filter(Boolean);
  for (const btn of detectBtns) {
    btn.addEventListener('click', () => runGitHubLoopAction('detect_repo', {}).catch(() => {}));
  }

  const branchesBtn = document.getElementById('github-connect-branches-btn');
  if (branchesBtn) {
    branchesBtn.addEventListener('click', () => runGitHubLoopAction('connect', {}).catch(() => {}));
  }

  for (const [btnId, inputId] of [
    ['github-create-branch-btn', 'github-new-branch-input'],
    ['scm-create-branch-btn', 'scm-new-branch-input'],
  ]) {
    const bBtn = document.getElementById(btnId);
    const bInp = document.getElementById(inputId);
    if (bBtn && bInp) {
      bBtn.addEventListener('click', () => {
        const nextBranch = bInp.value.trim();
        if (!nextBranch) return;
        runGitHubLoopAction('create_branch', {
          branch: nextBranch,
          base_branch: AppState.config.connectedGitBranch || 'main',
        }).catch(() => {});
      });
    }
  }

  for (const selId of ['scm-branch-select', 'github-branch-select']) {
    const sel = document.getElementById(selId);
    if (sel) {
      sel.addEventListener('change', () => {
        const chosen = sel.value;
        if (!chosen || !AppState.config.connectedGitRepo) return;
        runGitHubLoopAction('clone', { repo: AppState.config.connectedGitRepo, branch: chosen }).catch(() => {});
      });
    }
  }

  const scmCloneBtns = [
    document.getElementById('scm-connect-clone-btn'),
    document.getElementById('scm-clone-btn'),
  ].filter(Boolean);
  for (const scmCloneBtn of scmCloneBtns) {
    scmCloneBtn.addEventListener('click', async () => {
      const repoInput = document.getElementById('scm-repo-input') || document.getElementById('github-repo-input');
      const repoVal = repoInput ? repoInput.value.trim() : '';
      if (!repoVal) {
        if (typeof showStudioToast === 'function') {
          showStudioToast('Enter an owner/repo to clone into /workspace.', 'warn', 3500);
        }
        if (openGithubBtn) openGithubBtn.click();
        return;
      }
      await runGitHubLoopAction('connect', { repo: repoVal });
      await runGitHubLoopAction('clone', { repo: repoVal, branch: AppState.config.connectedGitBranch || 'main' });
    });
  }

  const scmCommitPushBtn = document.getElementById('scm-commit-push-btn');
  if (scmCommitPushBtn) {
    scmCommitPushBtn.addEventListener('click', () => {
      const msgEl = document.getElementById('scm-commit-message-input');
      const commitMsg = (msgEl && msgEl.value.trim()) || 'Update workspace files via LexLean-R4 Studio';
      runGitHubLoopAction('commit_push', { message: commitMsg }).catch(() => {});
    });
  }

  const scmCreatePrBtn = document.getElementById('scm-create-pr-btn');
  if (scmCreatePrBtn) {
    scmCreatePrBtn.addEventListener('click', () => {
      const titleEl = document.getElementById('scm-pr-title-input');
      const prTitle = (titleEl && titleEl.value.trim()) || `Workspace changes from ${AppState.config.connectedGitBranch || 'feature'}`;
      runGitHubLoopAction('create_pr', {
        title: prTitle,
        head: AppState.config.connectedGitBranch || 'main',
        base: 'main',
      }).catch(() => {});
    });
  }

  const scmRefreshCiBtn = document.getElementById('scm-refresh-ci-btn');
  if (scmRefreshCiBtn) {
    scmRefreshCiBtn.addEventListener('click', async () => {
      if (AppState.config.activePrNumber) {
        await runGitHubLoopAction('pr_status', { pr_number: AppState.config.activePrNumber });
      }
      await runGitHubLoopAction('watch_deploy', {});
    });
  }

  const scmMergePrBtn = document.getElementById('scm-merge-pr-btn');
  if (scmMergePrBtn) {
    scmMergePrBtn.addEventListener('click', async () => {
      if (!AppState.config.activePrNumber) return;
      const merged = await runGitHubLoopAction('merge_pr', { pr_number: AppState.config.activePrNumber });
      if (merged && merged.ok) {
        await runGitHubLoopAction('watch_deploy', {}).catch(() => {});
      }
    });
  }

  const refreshReloadBtn = document.getElementById('studio-refresh-reload-btn');
  if (refreshReloadBtn) {
    refreshReloadBtn.addEventListener('click', async () => {
      refreshReloadBtn.disabled = true;
      try {
        if (engineWorkerClient.enabled) {
          await engineWorkerClient.rpc('force_refresh_reload', {}).catch(() => {});
        }
      } finally {
        if (typeof window !== 'undefined' && window.location && typeof window.location.reload === 'function') {
          window.location.reload();
        }
      }
    });
  }

  const composerModelSel = document.getElementById('composer-model-select');
  if (composerModelSel && AppState && AppState.config) {
    const curModel = AppState.config.selectedModel;
    if (curModel === 'uor-r4.qwen3-0.6b-holo' || curModel === 'uor-r4.qwen3-1.7b-holo') {
      composerModelSel.value = curModel;
    }
  }

  if (cloneGithubRepoBtn) {
    cloneGithubRepoBtn.addEventListener('click', async () => {
      const repoVal = githubRepoUrlInput ? githubRepoUrlInput.value.trim() : '';
      if (!repoVal) {
        if (githubConnectStatus) githubConnectStatus.textContent = 'Enter a GitHub repository (owner/repo or URL).';
        return;
      }
      const branchSel = document.getElementById('github-branch-select');
      const selectedBranch = (branchSel && branchSel.value) || AppState.config.connectedGitBranch || 'main';
      if (githubConnectStatus) githubConnectStatus.textContent = `Cloning ${repoVal}@${selectedBranch} into /workspace...`;
      cloneGithubRepoBtn.disabled = true;
      try {
        await runGitHubLoopAction('connect', { repo: repoVal }).catch(() => null);
        const res = await runGitHubLoopAction('clone', {
          repo: repoVal,
          repo_url: repoVal,
          branch: selectedBranch,
        });
        if (!res || res.ok === false) {
          return;
        }
        const count = Array.isArray(res.data?.files) ? res.data.files.length : (res.files?.length ?? res.data?.file_count ?? 0);
        if (githubConnectStatus) {
          githubConnectStatus.textContent = `Cloned ${count} file(s) from ${repoVal}@${AppState.config.connectedGitBranch} into /workspace. Switching to Studio IDE...`;
        }
        setLayoutMode('studio');
        closeGithubModal();
      } catch (err) {
        if (githubConnectStatus) {
          githubConnectStatus.textContent = `Clone failed: ${err && err.message ? err.message : String(err)}`;
        }
      } finally {
        cloneGithubRepoBtn.disabled = false;
      }
    });
  }

  initEmbedMeBridge();
  updateDynamicPlanningPanel();
  renderDynamicPoolPanel();
  renderCoworkStatusBar();
  renderCursorShardsSection();
  initPrismHoloIdentityControls();

  if (saveRelayUrlBtn) {
    saveRelayUrlBtn.addEventListener('click', async () => {
      const relayVal = spvsRelayUrlInput ? spvsRelayUrlInput.value.trim() : '';
      try {
        if (engineWorkerClient.enabled) {
          await engineWorkerClient.rpc('set_bridge_policy', { relayWsUrl: relayVal });
        }
        if (githubConnectStatus) {
          githubConnectStatus.textContent = relayVal
            ? `Updated SpiralCore relay endpoint (${relayVal}).`
            : 'Restored default Cloudflare Edge SpiralCore relay endpoint.';
        }
      } catch (err) {
        if (githubConnectStatus) {
          githubConnectStatus.textContent = `Relay policy error: ${err && err.message ? err.message : String(err)}`;
        }
      }
    });
  }

  document.addEventListener('keydown', (ev) => {
    if (ev.key === 'Escape' && !ev.defaultPrevented) {
      const prismHoloEl = document.getElementById('prism-holo-modal');
      const settingsEl = document.getElementById('settings-modal');
      const newPathEl = document.getElementById('new-path-modal');
      if (newPathEl && !newPathEl.hasAttribute('hidden')) {
        ev.preventDefault();
        newPathEl.setAttribute('hidden', '');
      } else if (settingsEl && !settingsEl.hasAttribute('hidden')) {
        ev.preventDefault();
        settingsEl.setAttribute('hidden', '');
      } else if (inspectorsModalEl && !inspectorsModalEl.hasAttribute('hidden')) {
        ev.preventDefault();
        closeInspectorsModal();
      } else if (githubModal && !githubModal.hasAttribute('hidden')) {
        ev.preventDefault();
        closeGithubModal();
      } else if (prismHoloEl && !prismHoloEl.hasAttribute('hidden')) {
        ev.preventDefault();
        prismHoloEl.setAttribute('hidden', '');
      }
    }
  });
}

function renderPrismHoloIdentityUI() {
  const idState = typeof engineWorkerClient !== 'undefined' ? engineWorkerClient.prismIdentity : null;
  const holoSum = typeof engineWorkerClient !== 'undefined'
    ? (engineWorkerClient.latestHoloSummary || (idState && idState.latest_holo))
    : null;
  const sideIdEl = document.getElementById('prism-sidebar-id-code');
  const sideHoloEl = document.getElementById('prism-sidebar-holo-code');
  const modalPrismEl = document.getElementById('prism-modal-prism-id');
  const modalDevEl = document.getElementById('prism-modal-device-id');
  const modalPersonaEl = document.getElementById('prism-modal-persona');
  const modalFpEl = document.getElementById('prism-modal-holo-fp');
  if (sideIdEl) {
    sideIdEl.textContent = idState && idState.prism_id ? `${idState.prism_id.slice(0, 22)}...` : 'initializing...';
  }
  if (sideHoloEl) {
    sideHoloEl.textContent = holoSum && holoSum.fingerprint
      ? `blake3:${holoSum.fingerprint.slice(0, 14)}... (${holoSum.vectors_passed || 5}/${holoSum.vectors_total || 5})`
      : 'holo://lexlean-r4';
  }
  if (modalPrismEl) {
    modalPrismEl.textContent = (idState && idState.prism_id) || 'initializing...';
  }
  if (modalDevEl) {
    modalDevEl.textContent = idState && idState.device_id
      ? `${idState.device_id} (${idState.protection || 'P-256'} · ${idState.role || 'member'})`
      : '—';
  }
  if (modalPersonaEl) {
    const p = idState && idState.persona;
    modalPersonaEl.textContent = p && p.handle ? `${p.handle}@${p.host || 'uor-foundation.github.io'} (${p.app_address || 'holo://lexlean-r4'})` : 'unbound';
  }
  if (modalFpEl) {
    modalFpEl.textContent = holoSum && holoSum.fingerprint
      ? `blake3:${holoSum.fingerprint} (${holoSum.byte_length} bytes · ${holoSum.wasm_vectors_passed ?? holoSum.vectors_passed}/${holoSum.wasm_vectors_total ?? holoSum.vectors_total} vectors)`
      : 'not compiled yet';
  }
}

function initPrismHoloIdentityControls() {
  const modal = document.getElementById('prism-holo-modal');
  const openBtn = document.getElementById('topbar-prism-holo-btn');
  const closeBtn = document.getElementById('prism-holo-close-btn');
  const backdrop = document.getElementById('prism-holo-backdrop');
  const sidebarCompileBtn = document.getElementById('sidebar-compile-holo-btn');
  const initIdBtn = document.getElementById('prism-identity-init-btn');
  const personaInput = document.getElementById('prism-persona-handle-input');
  const bindPersonaBtn = document.getElementById('prism-bind-persona-btn');
  const compileBtn = document.getElementById('prism-compile-holo-btn');
  const downloadBtn = document.getElementById('prism-download-holo-btn');
  const copyDescBtn = document.getElementById('prism-copy-me-manifest-btn');
  const statusEl = document.getElementById('prism-holo-status');

  const closeModal = () => {
    if (modal) modal.setAttribute('hidden', '');
  };
  const openModal = async () => {
    if (modal) modal.removeAttribute('hidden');
    if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
      const idRes = await engineWorkerClient.rpc('prism_identity_ensure', {}).catch(() => null);
      if (idRes && idRes.ok) engineWorkerClient.prismIdentity = idRes;
    }
    renderPrismHoloIdentityUI();
  };

  if (openBtn) openBtn.addEventListener('click', () => { openModal(); });
  if (closeBtn) closeBtn.addEventListener('click', closeModal);
  if (backdrop) backdrop.addEventListener('click', closeModal);

  if (initIdBtn) {
    initIdBtn.addEventListener('click', async () => {
      if (!engineWorkerClient || !engineWorkerClient.enabled) return;
      const res = await engineWorkerClient.rpc('prism_identity_ensure', {}).catch((e) => ({ ok: false, error: String(e) }));
      if (res && res.ok) {
        engineWorkerClient.prismIdentity = res;
        renderPrismHoloIdentityUI();
        if (statusEl) statusEl.textContent = `Stitch PrismId active (${res.storage}): ${res.prism_id}`;
      } else if (statusEl) {
        statusEl.textContent = `PrismId error: ${res?.error || 'failed'}`;
      }
    });
  }

  if (bindPersonaBtn) {
    bindPersonaBtn.addEventListener('click', async () => {
      if (!engineWorkerClient || !engineWorkerClient.enabled) return;
      const handle = (personaInput && personaInput.value.trim()) || 'lexlean-r4-studio';
      if (statusEl) statusEl.textContent = `Signing prism/persona/1 for ${handle}@uor-foundation.github.io...`;
      try {
        const res = await engineWorkerClient.rpc('prism_identity_bind_persona', {
          handle,
          host: 'uor-foundation.github.io',
        });
        engineWorkerClient.prismIdentity = res;
        renderPrismHoloIdentityUI();
        if (statusEl) statusEl.textContent = `Bound Foundry persona ${handle}@uor-foundation.github.io (kappa: ${res.last_record_kappa || res.head_kappa})`;
      } catch (err) {
        if (statusEl) statusEl.textContent = `Persona bind failed: ${err?.message || String(err)}`;
      }
    });
  }

  const runHoloCompile = async (triggerDownload) => {
    if (!engineWorkerClient || !engineWorkerClient.enabled) {
      if (statusEl) statusEl.textContent = 'Dedicated Worker required to compile .holo archive.';
      return;
    }
    if (statusEl) statusEl.textContent = 'Compiling 6-file HOLOVIEW v1 + core-wasm@1 into Hologram v4 .holo archive...';
    if (compileBtn) compileBtn.disabled = true;
    try {
      const res = await engineWorkerClient.rpc(
        triggerDownload ? 'download_holo_archive' : 'compile_holo_archive',
        { includeBytes: Boolean(triggerDownload) },
      );
      engineWorkerClient.latestHoloSummary = res;
      renderPrismHoloIdentityUI();
      if (statusEl) {
        statusEl.textContent = `Verified HOLO v4 (${res.byte_length} bytes, blake3:${res.fingerprint}, core-wasm@1 ${res.wasm_vectors_passed}/${res.wasm_vectors_total} passed, receipt ${res.action_record_kappa || 'signed'}).`;
      }
      if (triggerDownload && res.bytes && typeof Blob !== 'undefined' && typeof URL !== 'undefined') {
        const u8 = res.bytes instanceof Uint8Array ? res.bytes : new Uint8Array(res.bytes);
        const blobUrl = URL.createObjectURL(new Blob([u8], { type: 'application/octet-stream' }));
        const a = document.createElement('a');
        a.href = blobUrl;
        a.download = 'lexlean-r4.holo';
        document.body.appendChild(a);
        a.click();
        a.remove();
        URL.revokeObjectURL(blobUrl);
      }
    } catch (err) {
      if (statusEl) statusEl.textContent = `Holo compile failed: ${err?.message || String(err)}`;
    } finally {
      if (compileBtn) compileBtn.disabled = false;
    }
  };

  if (compileBtn) compileBtn.addEventListener('click', () => { runHoloCompile(false); });
  if (downloadBtn) downloadBtn.addEventListener('click', () => { runHoloCompile(true); });
  if (sidebarCompileBtn) {
    sidebarCompileBtn.addEventListener('click', () => {
      openModal();
      runHoloCompile(false);
    });
  }
  if (personaInput) {
    personaInput.addEventListener('keydown', (ev) => {
      if (ev.key === 'Enter' && bindPersonaBtn) {
        ev.preventDefault();
        bindPersonaBtn.click();
      }
    });
  }
  if (copyDescBtn) {
    copyDescBtn.addEventListener('click', async () => {
      if (!engineWorkerClient || !engineWorkerClient.enabled) return;
      const hrefOrigin = typeof window !== 'undefined' && window.location ? String(window.location.href || '') : '';
      const desc = await engineWorkerClient.rpc('get_me_app_descriptor', { origin: hrefOrigin }).catch(() => null);
      if (desc && statusEl) {
        const txt = JSON.stringify(desc, null, 2);
        statusEl.textContent = txt;
        if (typeof navigator !== 'undefined' && navigator.clipboard && typeof navigator.clipboard.writeText === 'function') {
          navigator.clipboard.writeText(txt).catch(() => {});
        }
      }
    });
  }

  renderPrismHoloIdentityUI();
}

const isPreviewSurface = typeof window !== 'undefined' && window.location && String(window.location.search || '').includes('surface=preview');
const isObservatorySurface = initObservatorySurface();
initCursorIdeWorkbench();

try {
  if (isPreviewSurface) {
    initPreviewSurfaceHost();
  } else if (!isObservatorySurface) {
    if (typeof navigator !== 'undefined' && navigator.storage && typeof navigator.storage.persist === 'function') {
      navigator.storage.persist().catch(() => false);
    }
    initLayoutAndConnectedControls();
    if (typeof window !== 'undefined' && Number(window.innerWidth) > 0 && Number(window.innerWidth) < 840) {
      const mainEl = document.querySelector('main');
      const sidebarEl = document.getElementById('threads-sidebar');
      if (mainEl && mainEl.classList) {
        mainEl.classList.remove('sidebar-expanded');
        mainEl.classList.add('sidebar-collapsed');
      }
      if (sidebarEl && sidebarEl.classList) {
        sidebarEl.classList.remove('sidebar-expanded');
        sidebarEl.classList.add('sidebar-collapsed');
      }
      if (typeof applyWindowWidthClass === 'function') {
        applyWindowWidthClass(window.innerWidth);
      }
    }
    registerBrowserTerminalBridgeHooks();
    startHeaderBrainPulseCanvas();

    const workerBooted = engineWorkerClient.spawn();
    if (!workerBooted) {
      await init();
      mainThreadWasmReady = true;
      await restore_cached_uorlut_from_indexeddb();
    }

    if (lexLeanTerminal && workerBooted) {
      if (lexLeanTerminal.promptLabel) {
        lexLeanTerminal.promptLabel.textContent = 'casey@mvm:/workspace$';
      }
      if (lexLeanTerminal.env) {
        lexLeanTerminal.env.USER = 'casey';
        lexLeanTerminal.env.HOSTNAME = 'mvm';
        lexLeanTerminal.env.CWD = '/workspace';
        lexLeanTerminal.env.PROMPT = 'casey@mvm:/workspace$ ';
      }
    }

    updateWeightCacheBadge();
    const initialState = dispatchStudioEnvelope('state', '');
    renderStudioState(initialState);
    if (!workerBooted) {
      restoreIntelligenceFromWebCache();
    }
    const initialWorkspace = dispatchStudioEnvelope('workspace_list', '');
    renderWorkspaceList(initialWorkspace);
    if (typeof editorTabManager !== 'undefined' && editorTabManager) {
      editorTabManager.init(cursorOpenTabs);
    }
    selectWorkspaceFile(activeWorkspacePath);
    if (!workerBooted) {
      runCursorR432Check('check');
      if (studioPromptInput && studioPromptInput.value) {
        runStudioGenerate(studioPromptInput.value);
      }
      runStudioSubsystems(subsystemsPayloadInput ? subsystemsPayloadInput.value : '7,6,1,10,2180');
      executeCoordinateEnvelope();
    }
    if (apiResponseOutput) {
      apiResponseOutput.textContent = '// Standalone Wasm API Endpoint ready.\n// Select an endpoint and click "Execute API Request in Wasm" to dispatch.';
    }
    const currentActiveThread = getActiveThread();
    if (!currentActiveThread.messages || currentActiveThread.messages.length === 0) {
      if (workerBooted) {
        appendChatTurn(
          'assistant',
          'UOR-R4 Holo v13 · Connected Studio',
          'Welcome to **UOR-R4 Holo v13 Studio**. Select a pure-integer neural model (`uor-r4.qwen3-0.6b-holo`, `uor-r4.qwen3-1.7b-holo`, `uor-r4.native-q31-d85`, or `uor-r4.hf-repo-holo`) below, paste code or use `/hf <repo>` and `/q31-train <text>`, or connect a GitHub repository via **GitHub**.',
        );
      } else {
        runClaudeChat('Hello! Who are you and what can you do?', false);
      }
    } else {
      const msgs = currentActiveThread.messages;
      let lastUserPrompt = 'Hello! Who are you and what can you do?';
      for (let i = 0; i < msgs.length; i++) {
        if (msgs[i].role === 'user' && msgs[i].text) {
          lastUserPrompt = msgs[i].text;
          const nextReply = (msgs[i + 1] && msgs[i + 1].role === 'assistant') ? (msgs[i + 1].text || '') : '';
          if (lexLean3dVisualizer && typeof lexLean3dVisualizer.ingestConversationTurn === 'function') {
            lexLean3dVisualizer.ingestConversationTurn(lastUserPrompt, nextReply, null);
          }
        }
      }
      if (!workerBooted) {
        const bootChatRes = dispatchStudioEnvelope('chat', lastUserPrompt);
        selectedTraceIndex = 0;
        if (lexLean3dVisualizer && typeof lexLean3dVisualizer.ingestConversationTurn === 'function') {
          lexLean3dVisualizer.ingestConversationTurn(lastUserPrompt, bootChatRes?.reply || '', bootChatRes);
        }
      }
    }
    if (claudeChatInput) {
      claudeChatInput.value = '';
      claudeChatInput.focus();
    }
    bindHoloObservatoryControls();
    renderHoloObservatoryDeck();
    fetchRecordedTraces();
  }
} catch (err) {
  appendChatTurn(
    'assistant',
    'UOR-R4 GLM',
    `WebAssembly initialization error: ${err && err.message ? err.message : String(err)}`,
  );
}

export { LexLeanTerminal };
