// Hearth Theater's viewer: put into every page (and every frame in it) that a saved site opens in, so the site can
// be used from the couch with a controller. It only points and clicks for you, the way a mouse would; it doesn't
// read or take anything from the page. The top frame draws the pointer and reads the controller; frames inside it
// (embedded players) get the clicks, scrolls and play/pause passed down to them.
//   Left stick: pointer   A: click   B: back a page   Right stick or D-pad up/down: scroll
//   D-pad left/right: video back/ahead 10 s   X: play/pause   Y: fill the screen with the video
//   LB/RB: back/forward   Menu: these hints   View (or Esc twice): back to Theater
(() => {
  'use strict';
  if (window.__hearthTheater) return;
  window.__hearthTheater = true;
  const TOP = window.top === window;
  const KEY = '__hearthTheater';
  const CLOSE = 'hearth-theater://close';

  // ------------------------------------------------------------------ what every frame does when asked
  const visible = el => { const r = el.getBoundingClientRect(); return r.width > 40 && r.height > 30 && r.bottom > 0 && r.right > 0 && r.top < innerHeight && r.left < innerWidth; };
  const area = el => { const r = el.getBoundingClientRect(); return Math.max(0, Math.min(r.right, innerWidth) - Math.max(r.left, 0)) * Math.max(0, Math.min(r.bottom, innerHeight) - Math.max(r.top, 0)); };
  const biggest = list => list.filter(visible).sort((a, b) => area(b) - area(a))[0] || null;
  const video = () => biggest([...document.querySelectorAll('video')]);
  const frames = () => [...document.querySelectorAll('iframe, frame')];
  const send = (win, msg) => { try { win.postMessage({ [KEY]: msg }, '*'); } catch { } };
  const toFrames = msg => frames().forEach(f => f.contentWindow && send(f.contentWindow, msg));

  function fire(el, type, x, y, Ctor) {
    const o = { bubbles: true, cancelable: true, composed: true, view: window, clientX: x, clientY: y, screenX: x, screenY: y, button: 0, buttons: type.endsWith('down') ? 1 : 0, pointerId: 1, pointerType: 'mouse', isPrimary: true };
    try { el.dispatchEvent(new Ctor(type, o)); } catch { el.dispatchEvent(new MouseEvent(type, o)); }
  }
  function deepAt(x, y) {
    let el = document.elementFromPoint(x, y);
    while (el && el.shadowRoot) { const inner = el.shadowRoot.elementFromPoint(x, y); if (!inner || inner === el) break; el = inner; }
    return el;
  }
  function clickAt(x, y) {
    const el = deepAt(x, y);
    if (!el) return;
    if (el.tagName === 'IFRAME' || el.tagName === 'FRAME') {
      const r = el.getBoundingClientRect();
      if (el.contentWindow) send(el.contentWindow, { do: 'click', x: x - r.left - el.clientLeft, y: y - r.top - el.clientTop });
      return;
    }
    const P = window.PointerEvent || MouseEvent;
    fire(el, 'pointerover', x, y, P); fire(el, 'mouseover', x, y, MouseEvent);
    fire(el, 'pointerdown', x, y, P); fire(el, 'mousedown', x, y, MouseEvent);
    const field = el.closest('input, textarea, select, [contenteditable=""], [contenteditable="true"]');
    if (field) field.focus();
    fire(el, 'pointerup', x, y, P); fire(el, 'mouseup', x, y, MouseEvent);
    if (typeof el.click === 'function') el.click(); else fire(el, 'click', x, y, MouseEvent);
  }
  function scrollable(el) {
    for (; el && el !== document.body && el !== document.documentElement; el = el.parentElement) {
      const s = getComputedStyle(el);
      if (/(auto|scroll)/.test(s.overflowY) && el.scrollHeight > el.clientHeight + 4) return el;
    }
    return null;
  }
  function scrollAt(x, y, dy) {
    const el = deepAt(x, y);
    if (el && (el.tagName === 'IFRAME' || el.tagName === 'FRAME')) {
      const r = el.getBoundingClientRect();
      if (el.contentWindow) send(el.contentWindow, { do: 'scroll', x: x - r.left, y: y - r.top, dy });
      return;
    }
    const box = scrollable(el);
    if (box) box.scrollBy(0, dy); else window.scrollBy(0, dy);
  }
  // Play/pause and skipping act on the biggest video on screen; frames are asked too when this page has none.
  function playPause() {
    const v = video();
    if (!v) { toFrames({ do: 'toggle' }); return; }
    if (v.paused) { const p = v.play(); if (p && p.catch) p.catch(() => { }); } else v.pause();
  }
  function seek(s) {
    const v = video();
    if (!v) { toFrames({ do: 'seek', s }); return; }
    try { v.currentTime = Math.max(0, Math.min((v.duration || 1e9) - .5, v.currentTime + s)); } catch { }
  }
  // Fill: the biggest video (or the frame holding the player) covers the whole screen; Y again puts it back.
  let filled = null;
  function fill(on) {
    if (filled) {
      filled.el.setAttribute('style', filled.style); if (filled.style === null) filled.el.removeAttribute('style');
      if (filled.el.tagName === 'IFRAME' && filled.el.contentWindow) send(filled.el.contentWindow, { do: 'fill', on: false });
      filled = null; document.documentElement.style.overflow = ''; return false;
    }
    if (on === false) return false;
    const target = biggest([...document.querySelectorAll('video'), ...frames()]);
    if (!target) return false;
    filled = { el: target, style: target.getAttribute('style') };
    target.setAttribute('style', (filled.style || '') + ';position:fixed!important;inset:0!important;left:0!important;top:0!important;width:100vw!important;height:100vh!important;max-width:none!important;max-height:none!important;margin:0!important;z-index:2147483646!important;background:#000!important;object-fit:contain!important;border:0!important;transform:none!important');
    document.documentElement.style.overflow = 'hidden';
    if (target.tagName === 'IFRAME' && target.contentWindow) send(target.contentWindow, { do: 'fill', on: true });
    return true;
  }

  window.addEventListener('message', e => {
    const m = e.data && e.data[KEY];
    // Only a frame's own parent page (where Theater's pointer is) can ask it to do something.
    if (!m || TOP || e.source !== window.parent) return;
    if (m.do === 'click') clickAt(m.x, m.y);
    else if (m.do === 'scroll') scrollAt(m.x, m.y, m.dy);
    else if (m.do === 'toggle') playPause();
    else if (m.do === 'seek') seek(m.s);
    else if (m.do === 'fill') fill(m.on);
  });

  if (!TOP) return;

  // ------------------------------------------------------------------ the pointer and the hints (top frame only)
  let x = innerWidth / 2, y = innerHeight / 2, shown = false, idle = 0, hintTimer = 0;
  const root = document.createElement('hearth-theater-ui');
  const shadow = root.attachShadow({ mode: 'closed' });
  shadow.innerHTML = `<style>
    :host { all: initial; }
    .ptr { position: fixed; left: 0; top: 0; width: 26px; height: 26px; margin: -3px 0 0 -3px; pointer-events: none; z-index: 2147483647;
      transition: opacity .25s; opacity: 0; filter: drop-shadow(0 2px 4px rgba(0,0,0,.7)); }
    .ptr.on { opacity: 1; }
    .ptr.press { transform: scale(.82); }
    .bar { position: fixed; left: 50%; bottom: 28px; transform: translate(-50%, 12px); opacity: 0; transition: opacity .25s, transform .25s; pointer-events: none;
      z-index: 2147483647; display: flex; gap: 18px; flex-wrap: wrap; justify-content: center; max-width: 92vw; padding: 12px 20px; border-radius: 16px;
      background: rgba(23,17,14,.92); border: 1px solid #4a372b; color: #f4ece3; box-shadow: 0 16px 50px rgba(0,0,0,.55);
      font: 500 15px/1.3 system-ui, "Segoe UI", Cantarell, "Noto Sans", sans-serif; }
    .bar.on { opacity: 1; transform: translate(-50%, 0); }
    .bar span { white-space: nowrap; color: #a69484; }
    .bar b { display: inline-grid; place-items: center; min-width: 22px; height: 22px; padding: 0 6px; margin-right: 6px; border-radius: 99px; background: #33271f; color: #f4ece3; font-size: 12px; }
    .bar .t { color: #ff8a3d; font-weight: 650; }
    .toast { position: fixed; left: 50%; top: 26px; transform: translateX(-50%); opacity: 0; transition: opacity .2s; pointer-events: none; z-index: 2147483647;
      padding: 9px 16px; border-radius: 12px; background: rgba(38,29,24,.95); border: 1px solid #4a372b; color: #f4ece3; font: 500 15px system-ui, sans-serif; }
    .toast.on { opacity: 1; }
  </style>
  <svg class="ptr" viewBox="0 0 24 24"><path d="M3 2l17 9.5-7.2 1.6L9.4 20z" fill="#ff8a3d" stroke="#2a1206" stroke-width="1.6" stroke-linejoin="round"/></svg>
  <div class="bar"><span class="t">Hearth Theater</span><span><b>L</b>Pointer</span><span><b>A</b>Click</span><span><b>B</b>Back</span><span><b>R</b>Scroll</span>
    <span><b>X</b>Play/pause</span><span><b>Y</b>Fill screen</span><span><b>◀▶</b>−/+ 10 s</span><span><b>View</b>Theater</span></div>
  <div class="toast"></div>`;
  const ptr = shadow.querySelector('.ptr'), bar = shadow.querySelector('.bar'), toastEl = shadow.querySelector('.toast');
  const attach = () => { if (!root.isConnected) (document.body || document.documentElement).appendChild(root); };
  const place = () => { ptr.style.translate = `${x}px ${y}px`; };
  function showPtr() { attach(); if (!shown) { ptr.classList.add('on'); shown = true; } idle = performance.now(); }
  function hints(ms = 5000) { attach(); bar.classList.add('on'); clearTimeout(hintTimer); hintTimer = setTimeout(() => bar.classList.remove('on'), ms); }
  let toastT = 0;
  function toast(t) { attach(); toastEl.textContent = t; toastEl.classList.add('on'); clearTimeout(toastT); toastT = setTimeout(() => toastEl.classList.remove('on'), 1800); }
  const home = () => { location.href = CLOSE; };

  document.addEventListener('mousemove', e => { if (e.isTrusted && shown) { ptr.classList.remove('on'); shown = false; } }, { passive: true });
  let lastEsc = 0;
  document.addEventListener('keydown', e => {
    if (!e.isTrusted) return;
    if (e.key === 'Escape' && !document.fullscreenElement) {
      const now = performance.now();
      if (now - lastEsc < 700) { e.preventDefault(); home(); return; }
      lastEsc = now; toast('Press Esc again for Theater');
    }
    if (e.key === 'BrowserHome' || (e.altKey && e.key === 'Home')) { e.preventDefault(); home(); }
  }, true);

  // ------------------------------------------------------------------ the controller
  const held = {}; let last = performance.now();
  const DEAD = 0.18;
  const curve = v => { const a = Math.abs(v); if (a < DEAD) return 0; const n = (a - DEAD) / (1 - DEAD); return Math.sign(v) * n * n; };
  function press(k) {
    switch (k) {
      case 'a': showPtr(); ptr.classList.add('press'); setTimeout(() => ptr.classList.remove('press'), 120); clickAt(x, y); break;
      case 'b': if (filled) fill(false); else if (document.fullscreenElement) document.exitFullscreen().catch(() => { }); else history.back(); break;
      case 'x': playPause(); break;
      case 'y': toast(fill() ? 'Filling the screen (Y to undo)' : filled ? 'Back to the page' : 'No video on screen yet'); break;
      case 'lb': history.back(); break;
      case 'rb': history.forward(); break;
      case 'left': seek(-10); break;
      case 'right': seek(10); break;
      case 'up': scrollAt(x, y, -innerHeight * .35); break;
      case 'down': scrollAt(x, y, innerHeight * .35); break;
      case 'start': hints(6000); break;
      case 'select': case 'guide': home(); break;
    }
  }
  function poll(now) {
    const dt = Math.min(50, now - last); last = now;
    const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : [];
    const s = { a: 0, b: 0, x: 0, y: 0, lb: 0, rb: 0, select: 0, start: 0, guide: 0, up: 0, down: 0, left: 0, right: 0 };
    let lx = 0, ly = 0, rx = 0, ry = 0;
    for (const g of pads) {
      const bt = i => !!(g.buttons[i] && g.buttons[i].pressed);
      s.a ||= bt(0); s.b ||= bt(1); s.x ||= bt(2); s.y ||= bt(3); s.lb ||= bt(4); s.rb ||= bt(5);
      s.select ||= bt(8); s.start ||= bt(9); s.guide ||= bt(16);
      s.up ||= bt(12); s.down ||= bt(13); s.left ||= bt(14); s.right ||= bt(15);
      lx += curve(g.axes[0] || 0); ly += curve(g.axes[1] || 0); rx += curve(g.axes[2] || 0); ry += curve(g.axes[3] || 0);
    }
    if (lx || ly) {
      const speed = 1.15 * Math.max(innerWidth, 800) / 1000;   // about a screen's width in under a second at full tilt
      x = Math.max(0, Math.min(innerWidth - 2, x + lx * speed * dt));
      y = Math.max(0, Math.min(innerHeight - 2, y + ly * speed * dt));
      place(); showPtr();
    }
    if (ry) scrollAt(x, y, ry * dt * 1.6);
    if (rx) { const box = scrollable(deepAt(x, y)) || document.scrollingElement; if (box) box.scrollBy(rx * dt * 1.6, 0); }
    for (const [k, on] of Object.entries(s)) {
      if (!on) { held[k] = 0; continue; }
      const dir = k === 'up' || k === 'down';
      const repeat = held[k] && dir && now - held[k] > 400;
      if (!held[k]) { held[k] = now; press(k); } else if (repeat) { held[k] = now - 300; press(k); }
    }
    if (shown && now - idle > 4000) { ptr.classList.remove('on'); shown = false; }
    requestAnimationFrame(poll);
  }
  addEventListener('resize', () => { x = Math.min(x, innerWidth - 2); y = Math.min(y, innerHeight - 2); place(); });
  const start = () => { attach(); place(); hints(5000); requestAnimationFrame(t => { last = t; poll(t); }); };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start, { once: true }); else start();
})();
