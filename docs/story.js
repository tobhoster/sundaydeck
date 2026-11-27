(() => {
  "use strict";

  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  // 1. Reading progress thread & active navigation chapter highlight
  const progressBar = document.getElementById("reading-progress");
  const navLinks = Array.from(document.querySelectorAll(".nav-links a[href^='#']"));
  const navSections = navLinks
    .map((a) => document.querySelector(a.getAttribute("href")))
    .filter(Boolean);

  function updateScrollChrome() {
    if (progressBar) {
      const scrollable = document.documentElement.scrollHeight - window.innerHeight;
      const pct = scrollable > 0 ? Math.min(100, Math.max(0, (window.scrollY / scrollable) * 100)) : 0;
      progressBar.style.width = pct.toFixed(2) + "%";
    }
    if (navSections.length) {
      let currentId = navSections[0].id;
      const triggerY = window.scrollY + 140;
      for (const sec of navSections) {
        if (sec.offsetTop <= triggerY) currentId = sec.id;
      }
      navLinks.forEach((a) => {
        a.classList.toggle("active", a.getAttribute("href") === "#" + currentId);
      });
    }
  }

  window.addEventListener("scroll", updateScrollChrome, { passive: true });
  window.addEventListener("resize", updateScrollChrome, { passive: true });
  updateScrollChrome();

  // 2. Choreographed editorial scroll reveals
  const revealNodes = Array.from(document.querySelectorAll("[data-reveal]"));
  if (reducedMotion || !("IntersectionObserver" in window)) {
    revealNodes.forEach((el) => el.classList.add("is-visible"));
  } else {
    const revealObs = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add("is-visible");
            revealObs.unobserve(entry.target);
          }
        });
      },
      { threshold: 0.12, rootMargin: "0px 0px -48px 0px" }
    );
    revealNodes.forEach((el) => revealObs.observe(el));
  }

  // 3. Sunday Morning Scrollytelling Chapters (9:45 AM -> 10:45 AM)
  const chapters = Array.from(document.querySelectorAll(".story-chapter"));
  const slides = Array.from(document.querySelectorAll(".story-slide"));
  const dots = Array.from(document.querySelectorAll(".story-dot"));
  const storyViewport = document.getElementById("story-viewport");
  const stageTime = document.getElementById("story-stage-time");
  const stageLabel = document.getElementById("story-stage-label");
  const stageCaption = document.getElementById("story-stage-caption");

  let activeStep = 0;

  function setStoryStep(index) {
    if (index < 0 || index >= chapters.length) return;
    activeStep = index;
    const ch = chapters[index];
    chapters.forEach((el, i) => el.classList.toggle("is-active", i === index));
    slides.forEach((el, i) => el.classList.toggle("is-active", i === index));
    dots.forEach((el, i) => el.classList.toggle("is-active", i === index));
    if (storyViewport) storyViewport.dataset.step = String(index);
    if (stageTime) stageTime.textContent = ch.dataset.time || "";
    if (stageLabel) stageLabel.textContent = ch.dataset.stage || "";
    if (stageCaption) stageCaption.textContent = ch.dataset.caption || "";
    window.dispatchEvent(new CustomEvent("sundaydeck:story-step", { detail: { index } }));
  }

  chapters.forEach((ch, idx) => {
    ch.addEventListener("click", () => setStoryStep(idx));
  });
  dots.forEach((dot, idx) => {
    dot.addEventListener("click", () => setStoryStep(idx));
  });

  if ("IntersectionObserver" in window && chapters.length) {
    const storyObs = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            const idx = chapters.indexOf(entry.target);
            if (idx !== -1 && idx !== activeStep) {
              setStoryStep(idx);
            }
          }
        });
      },
      { threshold: 0.55, rootMargin: "-12% 0px -25% 0px" }
    );
    chapters.forEach((ch) => storyObs.observe(ch));
  }
  setStoryStep(0);

  // 4. Interactive Sanctuary Booth Recovery Simulator (Space, F1, F2, F3 + Live Backdrops)
  const shortcutsSec = document.getElementById("shortcuts");
  const boothSim = document.querySelector(".booth-sim");
  const screen = document.getElementById("sim-screen");
  const lyricsEl = document.getElementById("sim-lyrics");
  const subEl = document.getElementById("sim-sub");
  const badgeDot = document.getElementById("sim-badge-dot");
  const badgeText = document.getElementById("sim-badge-text");
  const simBtns = Array.from(document.querySelectorAll(".kbd-btn[data-sim]"));
  const simBgPhoto = document.getElementById("sim-bg-photo");
  const simBgVideo = document.getElementById("sim-bg-video");
  const simBgPills = Array.from(document.querySelectorAll(".bg-pill[data-sim-bg]"));
  const backdropCards = Array.from(document.querySelectorAll(".backdrop-card[data-preview-bg]"));

  const SIM_BACKDROPS = {
    "golden-motion": {
      kind: "video",
      src: "media/bg/golden-light-loop.mp4",
      poster: "media/bg/golden-light.jpg",
    },
    "starry-clouds": {
      kind: "video",
      src: "media/bg/starry-clouds-loop.mp4",
      poster: "media/bg/starry-clouds-poster.jpg",
    },
    "twilight-mountains": {
      kind: "photo",
      src: "media/bg/sanctuary-mountains.jpg",
    },
    "still-waters": {
      kind: "photo",
      src: "media/bg/still-waters.jpg",
    },
    "forest-mist": {
      kind: "photo",
      src: "media/bg/autumn-forest-mist.jpg",
    },
    "warm-dawn": {
      kind: "photo",
      src: "media/bg/warm-dawn-shore.jpg",
    },
  };

  function setSimBackdrop(key) {
    const spec = SIM_BACKDROPS[key];
    if (!spec || !screen) return;
    let resolvedKind = spec.kind;
    let resolvedSrc = spec.src;
    if (spec.kind === "video" && simBgVideo) {
      if (reducedMotion) {
        resolvedKind = "photo";
        resolvedSrc = spec.poster;
        if (simBgPhoto) simBgPhoto.style.backgroundImage = `url("${spec.poster}")`;
      } else {
        simBgVideo.poster = spec.poster;
        const source = simBgVideo.querySelector("source");
        if (source && source.getAttribute("src") !== spec.src) {
          source.setAttribute("src", spec.src);
          simBgVideo.load();
        }
        simBgVideo.play().catch(() => {});
      }
    } else if (simBgPhoto) {
      simBgPhoto.style.backgroundImage = `url("${spec.src}")`;
      if (simBgVideo) simBgVideo.pause();
    }
    screen.dataset.bgKind = resolvedKind;
    simBgPills.forEach((p) => p.classList.toggle("is-active", p.dataset.simBg === key));
    window.dispatchEvent(
      new CustomEvent("sundaydeck:sim-backdrop", {
        detail: { key, kind: resolvedKind, src: resolvedSrc },
      })
    );
  }

  // Respect prefers-reduced-motion on all ambient looping videos
  if (reducedMotion) {
    document.querySelectorAll(".hero-ambient-video, .backdrop-media video, #sim-bg-video").forEach((v) => {
      v.pause();
      v.removeAttribute("autoplay");
    });
  }

  simBgPills.forEach((pill) => {
    pill.addEventListener("click", () => {
      setSimBackdrop(pill.dataset.simBg);
      if (simMode === "black" || simMode === "logo") {
        simMode = "live";
        renderSim();
      }
    });
  });

  backdropCards.forEach((card) => {
    card.addEventListener("click", () => {
      setSimBackdrop(card.dataset.previewBg);
      simMode = "live";
      renderSim();
      const target = document.getElementById("shortcuts");
      if (target) target.scrollIntoView({ behavior: reducedMotion ? "auto" : "smooth" });
    });
  });

  const verses = [
    {
      text: "Amazing grace, how sweet the sound\nThat saved a wretch like me",
      sub: "Verse 1 · Amazing Grace",
    },
    {
      text: "I once was lost, but now am found\nWas blind, but now I see",
      sub: "Verse 1 (2/2) · Amazing Grace",
    },
    {
      text: "The Lord is my shepherd;\nI shall not want.",
      sub: "Psalm 23:1 · Sanctuary Scripture",
    },
  ];

  let verseIdx = 0;
  let simMode = "live";

  function renderSim() {
    if (!screen) return;
    screen.dataset.mode = simMode;
    if (lyricsEl) lyricsEl.innerHTML = verses[verseIdx].text.replace(/\n/g, "<br />");
    if (subEl) subEl.textContent = verses[verseIdx].sub;

    const statusMap = {
      live: { dot: "dot-emerald", label: "Live on Sanctuary Screen" },
      clear: { dot: "dot-amber", label: "Words Hidden · Background Steady (F1)" },
      black: { dot: "dot-coral", label: "Black Screen Active (F2)" },
      logo: { dot: "dot-amber", label: "Church Logo Showing (F3)" },
    };
    const st = statusMap[simMode] || statusMap.live;
    if (badgeDot) badgeDot.className = "dot " + st.dot;
    if (badgeText) badgeText.textContent = st.label;

    simBtns.forEach((btn) => {
      const action = btn.dataset.sim;
      btn.classList.toggle("is-latched", action === simMode && action !== "next" && action !== "live");
      btn.classList.toggle("is-live-btn", action === "next" && simMode === "live");
    });
  }

  function pressKeycap(btn) {
    if (!btn || reducedMotion) return;
    btn.classList.remove("is-pressed");
    void btn.offsetWidth;
    btn.classList.add("is-pressed");
    window.setTimeout(() => btn.classList.remove("is-pressed"), 180);
  }

  function triggerSimAction(action) {
    const btn = simBtns.find((b) => b.dataset.sim === action);
    pressKeycap(btn);
    if (action === "next") {
      verseIdx = (verseIdx + 1) % verses.length;
      simMode = "live";
    } else {
      simMode = simMode === action ? "live" : action;
    }
    renderSim();
  }

  simBtns.forEach((btn) => {
    btn.addEventListener("click", () => triggerSimAction(btn.dataset.sim));
  });

  // Tactile real-keyboard controls when booth simulator is in view / engaged
  let simInView = false;
  let simHovered = false;
  if (shortcutsSec && "IntersectionObserver" in window) {
    const simObs = new IntersectionObserver(
      (entries) => {
        for (const e of entries) simInView = e.isIntersecting;
      },
      { threshold: 0.35 }
    );
    simObs.observe(shortcutsSec);
  }
  if (boothSim) {
    boothSim.addEventListener("pointerenter", () => { simHovered = true; });
    boothSim.addEventListener("pointerleave", () => { simHovered = false; });
  }

  window.addEventListener("keydown", (e) => {
    if (!simInView || e.metaKey || e.ctrlKey || e.altKey) return;
    const tag = (e.target && e.target.tagName) || "";
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
    const focusedInside = Boolean(boothSim && boothSim.contains(document.activeElement));

    if (e.key === "F1") {
      e.preventDefault();
      triggerSimAction("clear");
    } else if (e.key === "F2") {
      e.preventDefault();
      triggerSimAction("black");
    } else if (e.key === "F3") {
      e.preventDefault();
      triggerSimAction("logo");
    } else if (e.code === "Space" && (simHovered || focusedInside)) {
      e.preventDefault();
      triggerSimAction("next");
    }
  });

  setSimBackdrop("golden-motion");
  renderSim();
})();
