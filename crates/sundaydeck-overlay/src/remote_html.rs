/// Zero-dependency, touch-friendly Booth Remote & Stream Deck test console served at `/remote`.
pub const REMOTE_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>SundayDeck — Booth Remote &amp; Companion Console</title>
<style>
  :root { color-scheme: dark; }
  * { box-sizing: border-box; }
  body {
    margin: 0; padding: 18px; background: #141518; color: #f4f1ea;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  }
  .wrap { max-width: 520px; margin: 0 auto; display: flex; flex-direction: column; gap: 14px; }
  header { display: flex; justify-content: space-between; align-items: center; }
  h1 { margin: 0; font-size: 16px; letter-spacing: 0.02em; }
  .pill { font-size: 11px; padding: 4px 10px; border-radius: 999px; background: #21242b; color: #9ea4b0; }
  .pill.live { color: #67d49c; border: 1px solid #2c5b43; }
  .card {
    background: #1c1e24; border: 1px solid #2b2e37; border-radius: 10px; padding: 14px;
    min-height: 88px; display: flex; flex-direction: column; gap: 6px;
  }
  .meta { font-size: 11px; color: #e2a83b; text-transform: uppercase; letter-spacing: 0.08em; }
  .body { font-size: 15px; line-height: 1.45; white-space: pre-wrap; }
  .grid2 { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .grid3 { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 8px; }
  button {
    appearance: none; border: 1px solid #353945; border-radius: 9px; padding: 14px 12px;
    background: #23262e; color: #f4f1ea; font-size: 14px; font-weight: 600; cursor: pointer;
  }
  button:active { transform: scale(0.98); }
  button.primary { background: #f3efe6; color: #15161a; border-color: #f3efe6; }
  button.warn { border-color: #5b3b2c; color: #f3c178; }
  .pager { display: flex; gap: 8px; }
  input {
    flex: 1; padding: 11px 12px; border-radius: 8px; border: 1px solid #353945;
    background: #141518; color: #f4f1ea; font-size: 14px;
  }
  .hint { font-size: 11px; color: #8c929f; }
</style>
</head>
<body>
<div class="wrap">
  <header>
    <h1>SundayDeck Booth Remote</h1>
    <span id="status" class="pill">Connecting…</span>
  </header>
  <div class="card">
    <div id="section" class="meta">READY</div>
    <div id="body" class="body">Waiting for live slide…</div>
  </div>
  <button class="primary" onclick="trigger('go-live')">Show Staged Slide on Screen</button>
  <div class="grid2">
    <button onclick="trigger('prev')">← Previous Slide</button>
    <button onclick="trigger('next')">Next Slide →</button>
  </div>
  <div class="grid3">
    <button class="warn" onclick="trigger('clear-text')">Hide Words</button>
    <button class="warn" onclick="trigger('black')">Black Screen</button>
    <button class="warn" onclick="trigger('logo')">Show Logo</button>
  </div>
  <button onclick="trigger('clear-overrides')">Clear Screen Overrides</button>
  <div class="card">
    <div class="meta">Nursery / Parent Security Code Pager</div>
    <div class="pager">
      <input id="code" type="text" maxlength="24" placeholder="Parent code (e.g. 142)" />
      <button onclick="pageParent()">Page</button>
      <button onclick="trigger('clear-alert')">Clear</button>
    </div>
    <div class="hint">Companion / Stream Deck: POST /api/trigger/{next,prev,go-live,clear-text,black,logo,parent-page}</div>
  </div>
</div>
<script>
async function trigger(action, body) {
  await fetch('/api/trigger/' + action, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: body ? JSON.stringify(body) : ''
  });
}
async function pageParent() {
  const el = document.getElementById('code');
  const code = (el.value || '').trim();
  if (!code) return;
  await trigger('parent-page', { code });
  el.value = '';
}
function connect() {
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  const ws = new WebSocket(proto + '//' + location.host + '/ws');
  const st = document.getElementById('status');
  ws.onopen = () => { st.textContent = 'Connected'; st.className = 'pill live'; };
  ws.onmessage = (ev) => {
    try {
      const p = JSON.parse(ev.data);
      document.getElementById('section').textContent = (p.section_label || p.mode || 'LIVE').toUpperCase();
      document.getElementById('body').textContent = p.body || (p.mode === 'black' ? '[Black Screen]' : p.mode === 'logo' ? '[Church Logo]' : '[Clear]');
    } catch (_) {}
  };
  ws.onclose = () => { st.textContent = 'Reconnecting…'; st.className = 'pill'; setTimeout(connect, 1500); };
}
connect();
</script>
</body>
</html>
"##;
