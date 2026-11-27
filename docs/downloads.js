(() => {
  "use strict";

  const REPO = "tobhoster/sundaydeck";
  const FALLBACK_VERSION = "0.0.4";
  const CACHE_KEY = "sundaydeck_latest_release_v1";
  const CACHE_TTL_MS = 10 * 60 * 1000;

  function detectPlatform() {
    const ua = (navigator.userAgent || "").toLowerCase();
    const plat = (navigator.platform || "").toLowerCase();
    if (ua.includes("win")) return "windows";
    if (ua.includes("mac") || plat.includes("mac")) return "macos";
    if (ua.includes("linux") && !ua.includes("android")) return "linux";
    return "macos";
  }

  function formatBytes(bytes) {
    if (!bytes || typeof bytes !== "number") return "";
    if (bytes >= 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + " MB";
    if (bytes >= 1024) return Math.round(bytes / 1024) + " KB";
    return bytes + " B";
  }

  function buildFallbackRelease(ver) {
    const clean = ver.replace(/^v/, "");
    const tag = "v" + clean;
    const base = `https://github.com/${REPO}/releases/download/${tag}/`;
    const names = [
      `SundayDeck-${clean}-macOS-arm64.dmg`,
      `SundayDeck-${clean}-macOS-x86_64.dmg`,
      `SundayDeck-${clean}-windows-x86_64-setup.exe`,
      `SundayDeck-${clean}-windows-x86_64.zip`,
      `sundaydeck_${clean}_amd64.deb`,
      `SundayDeck-${clean}-linux-x86_64.tar.gz`,
      "north-america-classic-hymns.sdeck-pack",
      "catholic-parish-73book.sdeck-pack",
      "spanish-himnario-rv1909.sdeck-pack",
      "portuguese-almeida-hinos.sdeck-pack",
      "SHA256SUMS.txt",
    ];
    return {
      tag_name: tag,
      assets: names.map((name) => ({ name, browser_download_url: base + name, size: 0 })),
    };
  }

  const MATCHERS = {
    "mac-arm64": (n) => /macos-arm64\.dmg$/i.test(n),
    "mac-x64": (n) => /macos-x86_64\.dmg$/i.test(n),
    "win-exe": (n) => /windows-x86_64-setup\.exe$/i.test(n),
    "win-zip": (n) => /windows-x86_64\.zip$/i.test(n),
    "linux-deb": (n) => /_amd64\.deb$/i.test(n),
    "linux-tar": (n) => /linux-x86_64\.tar\.gz$/i.test(n),
    "pack-na": (n) => n === "north-america-classic-hymns.sdeck-pack",
    "pack-catholic": (n) => n === "catholic-parish-73book.sdeck-pack",
    "pack-es": (n) => n === "spanish-himnario-rv1909.sdeck-pack",
    "pack-pt": (n) => n === "portuguese-almeida-hinos.sdeck-pack",
    "checksums": (n) => n === "SHA256SUMS.txt",
  };

  function resolveAsset(release, key) {
    const match = MATCHERS[key];
    if (!match || !release || !Array.isArray(release.assets)) return null;
    return release.assets.find((a) => match(a.name)) || null;
  }

  function recommendedSpec(os, release) {
    const tag = release.tag_name || "v" + FALLBACK_VERSION;
    if (os === "windows") {
      const asset = resolveAsset(release, "win-exe");
      return {
        os: "windows",
        key: "win-exe",
        url: asset ? asset.browser_download_url : "",
        size: asset ? formatBytes(asset.size) : "",
        title: `SundayDeck ${tag} for Windows (64-bit)`,
        sub: "Windows 10 / 11 installer with Start Menu shortcut & .sdeck support",
        btnLabel: "Download for Windows (.exe)",
        heroLabel: "Download Free for Windows",
        cmd: "irm https://tobhoster.github.io/sundaydeck/install.ps1 | iex",
      };
    }
    if (os === "linux") {
      const asset = resolveAsset(release, "linux-deb");
      return {
        os: "linux",
        key: "linux-deb",
        url: asset ? asset.browser_download_url : "",
        size: asset ? formatBytes(asset.size) : "",
        title: `SundayDeck ${tag} for Linux (x86_64)`,
        sub: "Debian / Ubuntu .deb package — or run the no-sudo shell installer below",
        btnLabel: "Download for Linux (.deb)",
        heroLabel: "Download Free for Linux",
        cmd: "curl -fsSL https://tobhoster.github.io/sundaydeck/install.sh | sh",
      };
    }
    const asset = resolveAsset(release, "mac-arm64");
    return {
      os: "macos",
      key: "mac-arm64",
      url: asset ? asset.browser_download_url : "",
      size: asset ? formatBytes(asset.size) : "",
      title: `SundayDeck ${tag} for macOS (Apple Silicon)`,
      sub: "macOS 13 Ventura or later · Intel .dmg available directly below",
      btnLabel: "Download for macOS (.dmg)",
      heroLabel: "Download Free for Mac",
      cmd: "curl -fsSL https://tobhoster.github.io/sundaydeck/install.sh | sh",
    };
  }

  let autoTriggered = false;

  function applyRelease(release) {
    const tag = release.tag_name || "v" + FALLBACK_VERSION;
    const os = detectPlatform();
    const rec = recommendedSpec(os, release);

    document.querySelectorAll("[data-release-version]").forEach((el) => {
      el.textContent = tag;
    });

    document.querySelectorAll("[data-dl-key]").forEach((el) => {
      const key = el.getAttribute("data-dl-key");
      const asset = key === "recommended" ? resolveAsset(release, rec.key) : resolveAsset(release, key);
      if (asset && asset.browser_download_url) {
        el.setAttribute("href", asset.browser_download_url);
        el.setAttribute("download", asset.name);
      }
      const sizeNode = el.querySelector("[data-dl-size]");
      if (sizeNode && asset && asset.size) {
        sizeNode.textContent = formatBytes(asset.size);
      }
    });

    const heroBtnLabel = document.getElementById("hero-dl-label");
    if (heroBtnLabel && document.documentElement.lang === "en") {
      heroBtnLabel.textContent = rec.heroLabel;
    }

    // Download page recommended panel + detected card highlight
    ["card-macos", "card-windows", "card-linux"].forEach((id) => {
      const card = document.getElementById(id);
      if (card) card.classList.toggle("is-detected", id === "card-" + os);
    });

    const recTitle = document.getElementById("rec-title");
    const recSub = document.getElementById("rec-sub");
    const recBtn = document.getElementById("rec-btn");
    const recBtnText = document.getElementById("rec-btn-text");
    const heroCmd = document.getElementById("hero-cmd");

    if (recTitle) recTitle.textContent = rec.title;
    if (recSub) recSub.textContent = rec.sub + (rec.size ? ` · ${rec.size}` : "");
    if (recBtn && rec.url) {
      recBtn.setAttribute("href", rec.url);
      recBtn.setAttribute("download", "");
    }
    if (recBtnText) recBtnText.textContent = rec.btnLabel;
    if (heroCmd) heroCmd.textContent = rec.cmd;

    // Optional automatic download when arriving with ?auto=1 (e.g. from direct download links)
    const params = new URLSearchParams(window.location.search);
    if (!autoTriggered && params.get("auto") === "1" && rec.url) {
      autoTriggered = true;
      const notice = document.getElementById("auto-dl-notice");
      if (notice) {
        notice.hidden = false;
        const link = notice.querySelector("a");
        if (link) link.setAttribute("href", rec.url);
      }
      setTimeout(() => {
        const a = document.createElement("a");
        a.href = rec.url;
        a.setAttribute("download", "");
        a.style.display = "none";
        document.body.appendChild(a);
        a.click();
        a.remove();
      }, 350);
    }
  }

  // Print buttons on cheat-sheet.html (avoids inline onclick handlers under strict CSP)
  document.querySelectorAll("[data-print-page]").forEach((btn) => {
    btn.addEventListener("click", () => window.print());
  });

  // Copy-to-clipboard buttons across index.html and download.html
  document.querySelectorAll(".copy-btn[data-copy-target]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const code = document.getElementById(btn.dataset.copyTarget);
      if (!code) return;
      try {
        await navigator.clipboard.writeText(code.textContent.trim());
        const span = btn.querySelector("span");
        const prev = span ? span.textContent : "Copy";
        if (span) span.textContent = "Copied!";
        btn.classList.add("is-copied");
        setTimeout(() => {
          if (span) span.textContent = prev;
          btn.classList.remove("is-copied");
        }, 1600);
      } catch (_) {}
    });
  });

  // 1. Apply cached or fallback release immediately (zero layout flash)
  let cached = null;
  try {
    const raw = sessionStorage.getItem(CACHE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (parsed && Date.now() - parsed.ts < CACHE_TTL_MS && parsed.data) {
        cached = parsed.data;
      }
    }
  } catch (_) {}

  applyRelease(cached || buildFallbackRelease(FALLBACK_VERSION));

  // 2. Refresh dynamically from GitHub Releases API so future releases automate with zero HTML edits
  if (!cached) {
    fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: { Accept: "application/vnd.github+json" },
    })
      .then((r) => (r.ok ? r.json() : null))
      .then((data) => {
        if (data && data.tag_name && Array.isArray(data.assets) && data.assets.length) {
          try {
            sessionStorage.setItem(CACHE_KEY, JSON.stringify({ ts: Date.now(), data }));
          } catch (_) {}
          applyRelease(data);
        }
      })
      .catch(() => {});
  }
})();
