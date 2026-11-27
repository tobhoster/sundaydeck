(() => {
  "use strict";

  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const MAX_DPR = 1.5;

  function clampDpr() {
    return Math.min(window.devicePixelRatio || 1, MAX_DPR);
  }

  function compileShader(gl, type, src) {
    const sh = gl.createShader(type);
    if (!sh) return null;
    gl.shaderSource(sh, src);
    gl.compileShader(sh);
    if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
      gl.deleteShader(sh);
      return null;
    }
    return sh;
  }

  function createProgram(gl, vsSrc, fsSrc) {
    const vs = compileShader(gl, gl.VERTEX_SHADER, vsSrc);
    const fs = compileShader(gl, gl.FRAGMENT_SHADER, fsSrc);
    if (!vs || !fs) return null;
    const prog = gl.createProgram();
    if (!prog) return null;
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);
    gl.deleteShader(vs);
    gl.deleteShader(fs);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      gl.deleteProgram(prog);
      return null;
    }
    return prog;
  }

  function setupFullscreenQuad(gl, prog) {
    const buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(
      gl.ARRAY_BUFFER,
      new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
      gl.STATIC_DRAW
    );
    const loc = gl.getAttribLocation(prog, "aPos");
    gl.enableVertexAttribArray(loc);
    gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
  }

  function observeVisibility(el, onChange) {
    if (!("IntersectionObserver" in window)) {
      onChange(true);
      return;
    }
    const obs = new IntersectionObserver(
      (entries) => {
        for (const e of entries) onChange(e.isIntersecting);
      },
      { threshold: 0.01 }
    );
    obs.observe(el);
  }

  // -------------------------------------------------------------------------
  // 1. Hero Ambient Sanctuary Light Shafts, Dust Motes & 2.5D Console Depth
  // -------------------------------------------------------------------------
  function initHeroSanctuary() {
    const hero = document.querySelector(".hero");
    const canvas = document.getElementById("hero-gl-canvas");
    const heroFrame = document.querySelector(".hero-frame");
    if (!hero || !canvas || reducedMotion) return;

    const gl = canvas.getContext("webgl", {
      alpha: true,
      antialias: false,
      depth: false,
      stencil: false,
      powerPreference: "low-power",
    });

    let mouseX = 0.5;
    let mouseY = 0.35;
    let smoothX = 0.5;
    let smoothY = 0.35;
    let pointerActive = false;

    hero.addEventListener(
      "pointermove",
      (e) => {
        const r = hero.getBoundingClientRect();
        if (r.width <= 0 || r.height <= 0) return;
        mouseX = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
        mouseY = Math.min(1, Math.max(0, (e.clientY - r.top) / r.height));
        pointerActive = true;
      },
      { passive: true }
    );
    hero.addEventListener(
      "pointerleave",
      () => {
        mouseX = 0.5;
        mouseY = 0.35;
        pointerActive = false;
      },
      { passive: true }
    );

    if (!gl) return;

    const vs = `
      attribute vec2 aPos;
      varying vec2 vUv;
      void main() {
        vUv = aPos * 0.5 + 0.5;
        gl_Position = vec4(aPos, 0.0, 1.0);
      }
    `;

    const fs = `
      precision mediump float;
      varying vec2 vUv;
      uniform vec2 uRes;
      uniform float uTime;
      uniform vec2 uMouse;

      float hash(vec2 p) {
        return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
      }

      float noise(vec2 p) {
        vec2 i = floor(p);
        vec2 f = fract(p);
        f = f * f * (3.0 - 2.0 * f);
        float a = hash(i);
        float b = hash(i + vec2(1.0, 0.0));
        float c = hash(i + vec2(0.0, 1.0));
        float d = hash(i + vec2(1.0, 1.0));
        return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
      }

      void main() {
        vec2 uv = vUv;
        float aspect = uRes.x / max(uRes.y, 1.0);
        vec2 p = vec2((uv.x - 0.5) * aspect, uv.y);

        // Soft diagonal morning light shafts filtering through sanctuary glass
        float shift = (uMouse.x - 0.5) * 0.14;
        float diag = p.x * 0.72 + (1.0 - uv.y) * 0.52 + shift;
        float beam1 = pow(max(0.0, 0.5 + 0.5 * sin(diag * 5.2 + uTime * 0.11)), 3.2);
        float beam2 = pow(max(0.0, 0.5 + 0.5 * sin(diag * 8.6 - uTime * 0.07 + 1.4)), 4.0);
        float mist = noise(vec2(p.x * 1.4 + uTime * 0.03, uv.y * 1.8 - uTime * 0.02));
        float topFade = smoothstep(0.08, 0.96, uv.y) * (1.0 - smoothstep(0.55, 1.0, abs(uv.x - 0.52) * 1.5));
        float shafts = (beam1 * 0.58 + beam2 * 0.42) * (0.55 + 0.45 * mist) * topFade;

        // Gentle warm sunlight bloom anchored near upper right sanctuary sky
        vec2 sunPos = vec2(0.66 + (uMouse.x - 0.5) * 0.08, 0.88 - (uMouse.y - 0.5) * 0.05);
        float sunDist = length(vec2((uv.x - sunPos.x) * aspect, uv.y - sunPos.y));
        float bloom = exp(-sunDist * 2.1) * 0.34;

        // Subtle slow-drifting sanctuary dust motes in morning light
        float motes = 0.0;
        for (int i = 0; i < 18; i++) {
          float fi = float(i);
          float rx = hash(vec2(fi, 1.37));
          float ry = hash(vec2(fi, 7.91));
          float speed = 0.012 + 0.018 * hash(vec2(fi, 3.19));
          float mx = fract(rx + uTime * speed * 0.35 + sin(uTime * 0.22 + fi) * 0.018);
          float my = fract(ry + uTime * speed);
          vec2 d = vec2((uv.x - mx) * aspect, uv.y - my);
          float radius = 0.0045 + 0.0055 * hash(vec2(fi, 5.41));
          float glow = smoothstep(radius * 2.4, 0.0, length(d));
          float twinkle = 0.45 + 0.55 * sin(uTime * 0.7 + fi * 2.3);
          motes += glow * twinkle * smoothstep(0.15, 0.95, my);
        }

        vec3 warmGold = vec3(0.92, 0.71, 0.34);
        vec3 ivoryLight = vec3(0.96, 0.91, 0.80);
        vec3 col = warmGold * (shafts * 0.24 + bloom * 0.55) + ivoryLight * (motes * 0.28);
        float alpha = clamp(shafts * 0.22 + bloom * 0.32 + motes * 0.22, 0.0, 0.42);
        gl_FragColor = vec4(col, alpha);
      }
    `;

    const prog = createProgram(gl, vs, fs);
    if (!prog) return;
    gl.useProgram(prog);
    setupFullscreenQuad(gl, prog);

    const uRes = gl.getUniformLocation(prog, "uRes");
    const uTime = gl.getUniformLocation(prog, "uTime");
    const uMouse = gl.getUniformLocation(prog, "uMouse");

    let width = 0;
    let height = 0;
    let visible = true;
    let rafId = 0;
    const startMs = performance.now();

    function resize() {
      const rect = hero.getBoundingClientRect();
      const dpr = clampDpr();
      width = Math.max(1, Math.round(rect.width * dpr));
      height = Math.max(1, Math.round(rect.height * dpr));
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
        gl.viewport(0, 0, width, height);
      }
    }

    function applyParallax() {
      if (!heroFrame || window.innerWidth < 768) return;
      const scrollRatio = Math.min(1, Math.max(0, window.scrollY / 520));
      const tiltY = (smoothX - 0.5) * 2.4;
      const tiltX = (0.45 - smoothY) * 1.9;
      const liftY = -scrollRatio * 10;
      if (heroFrame.classList.contains("is-visible")) {
        heroFrame.style.transform =
          `perspective(1500px) translate3d(0, ${liftY.toFixed(1)}px, 0) ` +
          `rotateX(${tiltX.toFixed(2)}deg) rotateY(${tiltY.toFixed(2)}deg)`;
      }
    }

    function frame(now) {
      if (!visible || document.hidden) {
        rafId = 0;
        return;
      }
      smoothX += (mouseX - smoothX) * 0.05;
      smoothY += (mouseY - smoothY) * 0.05;
      const t = (now - startMs) * 0.001;

      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.uniform2f(uRes, width, height);
      gl.uniform1f(uTime, t);
      gl.uniform2f(uMouse, smoothX, smoothY);
      gl.drawArrays(gl.TRIANGLES, 0, 6);

      if (pointerActive || window.scrollY < 600) {
        applyParallax();
      }
      rafId = requestAnimationFrame(frame);
    }

    function start() {
      if (!rafId && visible && !document.hidden) {
        resize();
        rafId = requestAnimationFrame(frame);
      }
    }

    window.addEventListener("resize", resize, { passive: true });
    document.addEventListener("visibilitychange", () => {
      if (!document.hidden) start();
    });
    observeVisibility(hero, (isVis) => {
      visible = isVis;
      if (visible) start();
    });
    resize();
    start();
  }

  // -------------------------------------------------------------------------
  // 2. Story 10:20 AM Organic Offline Microphone Audio Ribbon (2D Canvas)
  // -------------------------------------------------------------------------
  function initStoryMicRibbon() {
    const canvas = document.getElementById("story-mic-canvas");
    const stageWrap = document.querySelector(".story-stage-wrap");
    if (!canvas || !stageWrap) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let visible = false;
    let activeChapter = 0;
    let rafId = 0;
    const startMs = performance.now();

    function resize() {
      const rect = canvas.getBoundingClientRect();
      const dpr = clampDpr();
      const w = Math.max(1, Math.round(rect.width * dpr));
      const h = Math.max(1, Math.round(rect.height * dpr));
      if (canvas.width !== w || canvas.height !== h) {
        canvas.width = w;
        canvas.height = h;
      }
    }

    function drawStatic(t) {
      resize();
      const w = canvas.width;
      const h = canvas.height;
      const cy = h * 0.5;
      ctx.clearRect(0, 0, w, h);

      const ribbons = [
        { amp: h * 0.26, freq: 2.8, speed: 1.5, color: "rgba(226, 168, 59, 0.85)", width: 2.0 },
        { amp: h * 0.18, freq: 4.3, speed: -1.1, color: "rgba(56, 183, 128, 0.55)", width: 1.5 },
        { amp: h * 0.12, freq: 6.1, speed: 0.9, color: "rgba(245, 243, 239, 0.34)", width: 1.1 },
      ];

      for (const r of ribbons) {
        ctx.beginPath();
        const steps = 64;
        for (let i = 0; i <= steps; i++) {
          const u = i / steps;
          const x = u * w;
          const env = Math.sin(u * Math.PI);
          const breath = 0.65 + 0.35 * Math.sin(t * 1.4 + u * 3.0);
          const y =
            cy +
            Math.sin(u * Math.PI * r.freq + t * r.speed) * r.amp * env * breath +
            Math.cos(u * Math.PI * (r.freq * 0.55) - t * 0.8) * (r.amp * 0.32) * env;
          if (i === 0) ctx.moveTo(x, y);
          else ctx.lineTo(x, y);
        }
        ctx.strokeStyle = r.color;
        ctx.lineWidth = r.width * clampDpr();
        ctx.stroke();
      }
    }

    function loop(now) {
      if (!visible || activeChapter !== 3 || document.hidden || reducedMotion) {
        rafId = 0;
        return;
      }
      drawStatic((now - startMs) * 0.001);
      rafId = requestAnimationFrame(loop);
    }

    function sync() {
      if (activeChapter === 3) {
        if (reducedMotion) {
          drawStatic(1.2);
        } else if (visible && !rafId && !document.hidden) {
          rafId = requestAnimationFrame(loop);
        }
      }
    }

    window.addEventListener("sundaydeck:story-step", (e) => {
      activeChapter = Number(e.detail && e.detail.index) || 0;
      sync();
    });
    window.addEventListener("resize", () => {
      if (activeChapter === 3) drawStatic((performance.now() - startMs) * 0.001);
    }, { passive: true });
    document.addEventListener("visibilitychange", () => {
      if (!document.hidden) sync();
    });
    observeVisibility(stageWrap, (isVis) => {
      visible = isVis;
      sync();
    });
  }

  // -------------------------------------------------------------------------
  // 3. Booth Simulator GLSL Organic Light-Bloom Crossfade & Drift Shader
  // -------------------------------------------------------------------------
  function initSimBackdropShader() {
    const screen = document.getElementById("sim-screen");
    const canvas = document.getElementById("sim-gl-canvas");
    if (!screen || !canvas || reducedMotion) return;

    const gl = canvas.getContext("webgl", {
      alpha: false,
      antialias: false,
      depth: false,
      stencil: false,
      powerPreference: "low-power",
    });
    if (!gl) return;

    const vs = `
      attribute vec2 aPos;
      varying vec2 vUv;
      void main() {
        vUv = vec2(aPos.x * 0.5 + 0.5, 0.5 - aPos.y * 0.5);
        gl_Position = vec4(aPos, 0.0, 1.0);
      }
    `;

    const fs = `
      precision mediump float;
      varying vec2 vUv;
      uniform sampler2D uPrev;
      uniform sampler2D uCurr;
      uniform float uMix;
      uniform float uTime;

      float hash(vec2 p) {
        return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
      }

      float noise(vec2 p) {
        vec2 i = floor(p);
        vec2 f = fract(p);
        f = f * f * (3.0 - 2.0 * f);
        return mix(
          mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x),
          mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), f.x),
          f.y
        );
      }

      vec2 driftUv(vec2 uv, float t, float phase) {
        float zoom = 1.055 + 0.018 * sin(t * 0.14 + phase);
        vec2 pan = vec2(sin(t * 0.09 + phase) * 0.014, cos(t * 0.07 + phase) * 0.010);
        return (uv - 0.5) / zoom + 0.5 + pan;
      }

      void main() {
        float n = noise(vUv * 4.5 + vec2(uTime * 0.06, -uTime * 0.04));
        float horizon = mix(vUv.y * 0.45 + n * 0.55, n, 0.4);
        float m = smoothstep(0.0, 1.0, clamp(uMix * 1.38 - horizon * 0.38, 0.0, 1.0));

        vec2 disp = vec2((n - 0.5) * 0.025 * sin(uMix * 3.14159));
        vec4 prevCol = texture2D(uPrev, clamp(driftUv(vUv + disp, uTime, 0.0), 0.002, 0.998));
        vec4 currCol = texture2D(uCurr, clamp(driftUv(vUv - disp, uTime, 1.7), 0.002, 0.998));

        // Soft warm sanctuary light bloom peaking gently mid-dissolve
        float bloom = sin(clamp(uMix, 0.0, 1.0) * 3.14159) * 0.11;
        vec3 col = mix(prevCol.rgb, currCol.rgb, m) + vec3(0.95, 0.78, 0.46) * bloom;
        gl_FragColor = vec4(col, 1.0);
      }
    `;

    const prog = createProgram(gl, vs, fs);
    if (!prog) return;
    gl.useProgram(prog);
    setupFullscreenQuad(gl, prog);

    const uPrev = gl.getUniformLocation(prog, "uPrev");
    const uCurr = gl.getUniformLocation(prog, "uCurr");
    const uMix = gl.getUniformLocation(prog, "uMix");
    const uTime = gl.getUniformLocation(prog, "uTime");

    function makeTexture() {
      const tex = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texImage2D(
        gl.TEXTURE_2D,
        0,
        gl.RGBA,
        1,
        1,
        0,
        gl.RGBA,
        gl.UNSIGNED_BYTE,
        new Uint8Array([16, 18, 22, 255])
      );
      return tex;
    }

    const cache = new Map();
    let texPrev = makeTexture();
    let texCurr = texPrev;
    let mixVal = 1.0;
    let mixStart = 0;
    const MIX_MS = 1050;
    let visible = false;
    let hasPhoto = false;
    let rafId = 0;
    const startMs = performance.now();

    function loadTexture(src, onReady) {
      if (cache.has(src)) {
        onReady(cache.get(src));
        return;
      }
      const img = new Image();
      img.decoding = "async";
      img.onload = () => {
        const tex = makeTexture();
        gl.bindTexture(gl.TEXTURE_2D, tex);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
        cache.set(src, tex);
        onReady(tex);
      };
      img.src = src;
    }

    function resize() {
      const rect = screen.getBoundingClientRect();
      const dpr = clampDpr();
      const w = Math.max(1, Math.round(rect.width * dpr));
      const h = Math.max(1, Math.round(rect.height * dpr));
      if (canvas.width !== w || canvas.height !== h) {
        canvas.width = w;
        canvas.height = h;
        gl.viewport(0, 0, w, h);
      }
    }

    function render(now) {
      if (!visible || !hasPhoto || screen.dataset.bgKind !== "photo" || document.hidden) {
        rafId = 0;
        return;
      }
      if (mixVal < 1.0) {
        const elapsed = (now - mixStart) / MIX_MS;
        const clamped = Math.min(1, Math.max(0, elapsed));
        mixVal = clamped * clamped * (3 - 2 * clamped);
      }
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, texPrev);
      gl.uniform1i(uPrev, 0);
      gl.activeTexture(gl.TEXTURE1);
      gl.bindTexture(gl.TEXTURE_2D, texCurr);
      gl.uniform1i(uCurr, 1);
      gl.uniform1f(uMix, mixVal);
      gl.uniform1f(uTime, (now - startMs) * 0.001);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
      rafId = requestAnimationFrame(render);
    }

    function start() {
      if (!rafId && visible && hasPhoto && screen.dataset.bgKind === "photo" && !document.hidden) {
        resize();
        rafId = requestAnimationFrame(render);
      }
    }

    screen.dataset.glReady = "true";

    window.addEventListener("sundaydeck:sim-backdrop", (e) => {
      const detail = e.detail || {};
      if (detail.kind !== "photo" || !detail.src) return;
      loadTexture(detail.src, (nextTex) => {
        if (hasPhoto && nextTex !== texCurr) {
          texPrev = texCurr;
          texCurr = nextTex;
          mixVal = 0.0;
          mixStart = performance.now();
        } else {
          texPrev = nextTex;
          texCurr = nextTex;
          mixVal = 1.0;
        }
        hasPhoto = true;
        start();
      });
    });

    window.addEventListener("resize", resize, { passive: true });
    document.addEventListener("visibilitychange", () => {
      if (!document.hidden) start();
    });
    observeVisibility(screen, (isVis) => {
      visible = isVis;
      if (visible) start();
    });

    // Warm default mountain photo texture quietly in background
    loadTexture("media/bg/sanctuary-mountains.jpg", (tex) => {
      if (!hasPhoto) {
        texPrev = tex;
        texCurr = tex;
        hasPhoto = true;
      }
    });
  }

  initHeroSanctuary();
  initStoryMicRibbon();
  initSimBackdropShader();
})();
