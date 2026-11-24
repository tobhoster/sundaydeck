/// Alpha-transparent OBS Browser Source HTML/CSS/WebSocket lower-third view.
/// Zero external CDN requests — works 100% offline on localhost (`http://127.0.0.1:8787/overlay`).
pub const OVERLAY_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>SundayDeck — OBS Lower-Third Overlay</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    html, body {
      width: 100%;
      height: 100%;
      background: transparent;
      overflow: hidden;
      font-family: "Plus Jakarta Sans", "Manrope", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      -webkit-font-smoothing: antialiased;
    }
    .stage {
      position: fixed;
      inset: 0;
      display: flex;
      flex-direction: column;
      justify-content: flex-end;
      align-items: center;
      padding: 0 6vw 5.5vh 6vw;
      pointer-events: none;
    }
    .lower-third {
      max-width: 82vw;
      min-width: 44vw;
      padding: 20px 32px;
      border-radius: 12px;
      background: rgba(16, 18, 22, 0.84);
      border: 1px solid rgba(242, 239, 233, 0.14);
      color: #f5f3ef;
      text-align: center;
      opacity: 0;
      transform: translateY(10px);
      transition: opacity 280ms ease, transform 280ms ease;
    }
    .lower-third.visible {
      opacity: 1;
      transform: translate3d(0, 0, 0) scale(1);
    }
    .lower-third.instant { transition: none !important; }
    .lower-third.anim-soft-rise { transform: translateY(12px); }
    .lower-third.anim-slide-fwd { transform: translateX(36px); }
    .lower-third.anim-slide-bwd { transform: translateX(-36px); }
    .lower-third.anim-zoom { transform: scale(0.975); }
    .meta {
      display: inline-flex;
      align-items: center;
      gap: 8px;
      font-size: 15px;
      font-weight: 600;
      color: #b4b8c2;
      margin-bottom: 8px;
    }
    .meta-dot {
      width: 7px;
      height: 7px;
      border-radius: 999px;
      background: #e2a83b;
    }
    .body {
      font-size: clamp(24px, 2.7vw, 44px);
      line-height: 1.32;
      font-weight: 700;
      white-space: pre-line;
      text-shadow: 0 2px 10px rgba(0, 0, 0, 0.55);
    }
    .footer {
      margin-top: 8px;
      font-size: 16px;
      font-weight: 500;
      color: rgba(245, 243, 239, 0.72);
    }
    .alert {
      position: fixed;
      top: 28px;
      right: 32px;
      padding: 10px 18px;
      border-radius: 8px;
      background: rgba(20, 21, 24, 0.92);
      border: 1.5px solid #e2a83b;
      color: #f5f3ef;
      font-size: 20px;
      font-weight: 600;
      opacity: 0;
      transition: opacity 220ms ease;
    }
    .alert.visible { opacity: 1; }
  </style>
</head>
<body>
  <div id="alert" class="alert"></div>
  <div class="stage">
    <div id="card" class="lower-third">
      <div id="meta" class="meta" style="display:none;"><span class="meta-dot"></span><span id="meta-text"></span></div>
      <div id="body" class="body"></div>
      <div id="footer" class="footer" style="display:none;"></div>
    </div>
  </div>
  <script>
    const card = document.getElementById('card');
    const meta = document.getElementById('meta');
    const metaText = document.getElementById('meta-text');
    const bodyEl = document.getElementById('body');
    const footerEl = document.getElementById('footer');
    const alertEl = document.getElementById('alert');

    function applyState(st) {
      if (st.alert) {
        alertEl.textContent = st.alert;
        alertEl.classList.add('visible');
      } else {
        alertEl.classList.remove('visible');
      }

      if (st.mode !== 'slide' || !st.body || !st.body.trim()) {
        card.classList.remove('visible');
        return;
      }

      const dur = Math.max(0, Number(st.duration_ms || 280));
      card.style.transitionDuration = dur + 'ms';
      card.className = 'lower-third';
      if (st.transition === 'instant' || dur === 0) {
        card.classList.add('instant');
      } else if (st.transition === 'soft_rise') {
        card.classList.add('anim-soft-rise');
      } else if (st.transition === 'gentle_slide') {
        card.classList.add(st.direction === 'backward' ? 'anim-slide-bwd' : 'anim-slide-fwd');
      } else if (st.transition === 'zoom_dissolve') {
        card.classList.add('anim-zoom');
      }

      if (st.section_label) {
        metaText.textContent = st.section_label;
        meta.style.display = 'inline-flex';
      } else {
        meta.style.display = 'none';
      }

      bodyEl.textContent = st.body;
      if (st.footer) {
        footerEl.textContent = st.footer;
        footerEl.style.display = 'block';
      } else {
        footerEl.style.display = 'none';
      }

      requestAnimationFrame(() => card.classList.add('visible'));
    }

    function connect() {
      const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
      const ws = new WebSocket(proto + '//' + location.host + '/ws');
      ws.onmessage = (ev) => {
        try { applyState(JSON.parse(ev.data)); } catch (_) {}
      };
      ws.onclose = () => setTimeout(connect, 1200);
    }

    fetch('/state.json').then(r => r.json()).then(applyState).catch(() => {});
    connect();
  </script>
</body>
</html>
"#;
