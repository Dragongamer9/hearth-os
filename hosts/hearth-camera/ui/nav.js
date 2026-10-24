// Controller and keyboard for Hearth's small apps (Recorder, Camera, Share), the same way Disks does it: the
// D-pad, left stick or arrow keys move the ember focus ring to the nearest thing in that direction, A / Enter
// chooses, B / Esc goes back, LB / RB switch tabs, and the app gives X, Y and Menu a job of its own.
// The same file is in each app's ui/ folder; keep them alike.
'use strict';
const Nav = (() => {
  const H = { back: null, x: null, y: null, lb: null, rb: null, start: null };
  let usingPad = false;
  const layer = () => document.getElementById('layer');
  const layerOpen = () => !!(layer() && layer().firstChild);
  function focusables() {
    const scope = layerOpen() ? layer() : document;
    return [...scope.querySelectorAll('.f')].filter(el => !el.disabled && el.offsetParent !== null && !el.closest('[hidden]'));
  }
  function move(dir) {
    const all = focusables();
    const cur = document.activeElement;
    if (!cur || cur === document.body || !all.includes(cur)) { all[0]?.focus(); return; }
    const a = cur.getBoundingClientRect(); const ax = a.left + a.width / 2, ay = a.top + a.height / 2;
    let best = null, bestScore = Infinity;
    for (const el of all) {
      if (el === cur) continue;
      const b = el.getBoundingClientRect(); const bx = b.left + b.width / 2, by = b.top + b.height / 2;
      const ok = dir === 'left' ? b.right <= a.left + 4 : dir === 'right' ? b.left >= a.right - 4 : dir === 'up' ? b.bottom <= a.top + 4 : b.top >= a.bottom - 4;
      if (!ok) continue;
      const horiz = dir === 'left' || dir === 'right';
      const score = (horiz ? Math.abs(bx - ax) : Math.abs(by - ay)) + (horiz ? Math.abs(by - ay) : Math.abs(bx - ax)) * 2.2;
      if (score < bestScore) { bestScore = score; best = el; }
    }
    if (best) { best.focus(); best.scrollIntoView({ block: 'nearest', inline: 'nearest' }); }
  }
  function choose() {
    const el = document.activeElement;
    if (!el || el === document.body) { focusables()[0]?.focus(); return; }
    if (el.tagName === 'INPUT') return;
    el.click();
  }
  function back() {
    if (layerOpen()) { const c = layer().querySelector('[data-cancel]'); if (c) c.click(); else layer().replaceChildren(); return; }
    H.back && H.back();
  }
  document.addEventListener('keydown', e => {
    usingPad = false;
    const typing = e.target.tagName === 'INPUT' && e.target.type !== 'range';
    const k = e.key;
    if (k === 'Escape' || (k === 'Backspace' && !typing)) { e.preventDefault(); back(); return; }
    if (typing && (k === 'ArrowLeft' || k === 'ArrowRight')) return;
    const dirs = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
    if (dirs[k]) { e.preventDefault(); move(dirs[k]); return; }
    if (typing) { if (k === 'Enter') { e.preventDefault(); layer().querySelector('[data-ok]')?.click(); } return; }
    if (layerOpen()) return;
    if (k === 'PageUp' && H.lb) { e.preventDefault(); H.lb(); }
    if (k === 'PageDown' && H.rb) { e.preventDefault(); H.rb(); }
  });
  const held = {}; let lastMove = 0;
  function poll() {
    const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : [];
    const now = performance.now();
    const st = { up: false, down: false, left: false, right: false, a: false, b: false, x: false, y: false, lb: false, rb: false, start: false };
    for (const g of pads) {
      const bt = i => g.buttons[i] && g.buttons[i].pressed;
      const ax = g.axes[0] || 0, ay = g.axes[1] || 0;
      st.up ||= bt(12) || ay < -.6; st.down ||= bt(13) || ay > .6; st.left ||= bt(14) || ax < -.6; st.right ||= bt(15) || ax > .6;
      st.a ||= bt(0); st.b ||= bt(1); st.x ||= bt(2); st.y ||= bt(3); st.lb ||= bt(4); st.rb ||= bt(5); st.start ||= bt(9);
    }
    for (const [k, on] of Object.entries(st)) {
      if (!on) { held[k] = 0; continue; }
      const first = !held[k];
      const isDir = ['up', 'down', 'left', 'right'].includes(k);
      const repeat = held[k] && isDir && now - held[k] > 380 && now - lastMove > 110;
      if (first) held[k] = now;
      if (!first && !repeat) continue;
      usingPad = true; lastMove = now;
      if (isDir) move(k);
      else if (k === 'a') choose();
      else if (k === 'b') back();
      else if (!layerOpen() && H[k]) H[k]();
    }
    requestAnimationFrame(poll);
  }
  requestAnimationFrame(poll);

  // Little helpers every app uses.
  const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  let toastTimer = 0;
  function toast(text, bad) {
    const t = document.getElementById('toast'); if (!t) return;
    t.textContent = text; t.className = 'on' + (bad ? ' bad' : '');
    clearTimeout(toastTimer); toastTimer = setTimeout(() => { t.className = ''; }, bad ? 5200 : 3200);
  }
  /** A question with two answers; resolves true for the first. */
  function ask(title, text, ok, danger) {
    return new Promise(res => {
      const prev = document.activeElement;
      layer().innerHTML = `<div class="veil"><div class="dlg" role="dialog"><h2>${esc(title)}</h2><p class="muted">${esc(text)}</p>
        <div class="row end"><button class="btn f" data-cancel>Cancel</button><button class="btn f ${danger ? 'danger' : 'primary'}" data-ok>${esc(ok)}</button></div></div></div>`;
      const done = v => { layer().replaceChildren(); refocus(prev); res(v); };
      layer().querySelector('[data-ok]').onclick = () => done(true);
      layer().querySelector('[data-cancel]').onclick = () => done(false);
      layer().querySelector('[data-ok]').focus();
    });
  }
  /** Asks for a line of text; resolves with it, or null. */
  function prompt(title, value, ok) {
    return new Promise(res => {
      const prev = document.activeElement;
      layer().innerHTML = `<div class="veil"><div class="dlg" role="dialog"><h2>${esc(title)}</h2><input class="inp f" value="${esc(value)}" spellcheck="false">
        <div class="row end" style="margin-top:1em"><button class="btn f" data-cancel>Cancel</button><button class="btn f primary" data-ok>${esc(ok)}</button></div></div></div>`;
      const inp = layer().querySelector('input');
      const done = v => { layer().replaceChildren(); refocus(prev); res(v); };
      layer().querySelector('[data-ok]').onclick = () => done(inp.value);
      layer().querySelector('[data-cancel]').onclick = () => done(null);
      inp.focus(); inp.select();
    });
  }
  function refocus(prev) { if (prev && prev.isConnected && prev.focus) prev.focus(); else focusables()[0]?.focus(); }
  const human = b => { if (b == null) return ''; const u = ['bytes', 'KB', 'MB', 'GB', 'TB']; let v = b, i = 0; while (v >= 1000 && i < u.length - 1) { v /= 1000; i++; }
    return i === 0 ? `${b} bytes` : `${v >= 100 ? v.toFixed(0) : v.toFixed(1).replace(/\.0$/, '')} ${u[i]}`; };
  const when = ms => { const d = new Date(ms), now = new Date(); const t = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    if (d.toDateString() === now.toDateString()) return 'Today ' + t;
    const y = new Date(now); y.setDate(now.getDate() - 1); if (d.toDateString() === y.toDateString()) return 'Yesterday ' + t;
    return d.toLocaleDateString([], { day: 'numeric', month: 'short', year: d.getFullYear() === now.getFullYear() ? undefined : 'numeric' }) + ' ' + t; };
  const clock = ms => { const s = Math.max(0, Math.floor(ms / 1000)); const h = Math.floor(s / 3600), m = Math.floor(s / 60) % 60, x = s % 60;
    return (h ? h + ':' + String(m).padStart(2, '0') : String(m).padStart(2, '0')) + ':' + String(x).padStart(2, '0'); };
  return { init(h) { Object.assign(H, h); }, move, focusables, choose, back, pad: () => usingPad, esc, toast, ask, prompt, human, when, clock };
})();
