// Hearth Photos: the viewer. Shows a picture and moves through the others in its folder; zoom, pan, turn, flip,
// full screen, slideshow, info, Trash with undo, copy, wallpaper. The editor is edit.js. Works with mouse, touch,
// keyboard and controllers (the standard mapping), and in a plain browser with sample pictures (for checking).
'use strict';

const T = window.__TAURI__;
const invoke = T ? T.core.invoke : demoInvoke;
const MEDIA = T ? T.core.convertFileSrc('', 'media') : '';
const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const human = b => { if (b == null) return ''; const u = ['bytes', 'KB', 'MB', 'GB', 'TB']; let v = b, i = 0; while (v >= 1000 && i < u.length - 1) { v /= 1000; i++; } return (i ? v.toFixed(v < 10 ? 1 : 0) : v) + ' ' + u[i]; };
const extOf = p => { const m = /\.([^./\\]+)$/.exec(p || ''); return m ? m[1].toLowerCase() : ''; };
const store = { get(k, d) { try { const v = localStorage.getItem('hp.' + k); return v == null ? d : JSON.parse(v); } catch (_) { return d; } }, set(k, v) { try { localStorage.setItem('hp.' + k, JSON.stringify(v)); } catch (_) {} } };

// What both web views draw themselves; AVIF and JPEG XL are tried too (some builds have them), then converted.
const NATIVE = new Set(['jpg', 'jpeg', 'jpe', 'jfif', 'png', 'apng', 'gif', 'webp', 'bmp', 'dib', 'ico', 'svg', 'avif', 'jxl']);
const url = (route, path, q = '') => T ? `${MEDIA}${route}/${encodeURIComponent(path)}${q}` : demoUrl(route, path);

// ---------------------------------------------------------------- orientation (the same eight as EXIF, orient.rs)
const MATS = [null, [1, 0, 0, 1], [-1, 0, 0, 1], [-1, 0, 0, -1], [1, 0, 0, -1], [0, 1, 1, 0], [0, -1, 1, 0], [0, -1, -1, 0], [0, 1, -1, 0]];
const OP = { right: 6, left: 8, flipH: 2, flipV: 4 };
function then(o, op) {
  const u = MATS[op], m = MATS[o];
  const r = [u[0] * m[0] + u[1] * m[2], u[0] * m[1] + u[1] * m[3], u[2] * m[0] + u[3] * m[2], u[2] * m[1] + u[3] * m[3]];
  return MATS.findIndex(x => x && x.every((v, i) => v === r[i]));
}
/** CSS for an orientation: turn by deg after mirroring (flip) left to right. */
function cssOf(o) {
  const m = MATS[o], flip = m[0] * m[3] - m[1] * m[2] < 0;
  const r = flip ? [-m[0], m[1], -m[2], m[3]] : m; // undo the mirror to leave a pure turn
  return { deg: Math.round(Math.atan2(r[2], r[0]) * 180 / Math.PI), flip, swaps: o >= 5 };
}

// ---------------------------------------------------------------- state
const S = {
  host: 'linux', dir: '', pics: [], i: -1, info: null, sort: store.get('sort', 'name'),
  turn: 1,                         // turned/flipped on screen, not saved yet
  nat: { w: 0, h: 0 },             // the shown picture's size, upright
  z: { mode: 'fit', s: 1, x: 0, y: 0 },
  chrome: true, fs: false, infoOpen: store.get('info', false),
  slide: null, slideMs: store.get('slideMs', 5000),
  front: 'A', loadSeq: 0, pad: false, converters: [],
};
const cur = () => S.pics[S.i];

// ---------------------------------------------------------------- showing a picture
const stage = $('#stage');
const imgs = { A: $('#picA'), B: $('#picB') };

function setMsg(title, text) {
  const m = $('#msg');
  if (!title) { m.hidden = true; return; }
  m.hidden = false;
  m.innerHTML = `<div><b>${esc(title)}</b>${esc(text || '')}</div>`;
}

async function show(i, opts = {}) {
  if (!S.pics.length) { S.i = -1; render(); return; }
  if (S.turn !== 1 && !opts.keepTurn) { toast('The turn wasn\'t saved'); }
  if (!opts.keepTurn) S.turn = 1;
  S.i = (i + S.pics.length) % S.pics.length;
  const p = cur();
  const seq = ++S.loadSeq;
  S.info = null;
  render();
  const back = S.front === 'A' ? 'B' : 'A';
  const el = imgs[back];
  $('#spinner').hidden = false;
  const loaded = await loadInto(el, p.path, p.mtime);
  if (seq !== S.loadSeq) return;
  $('#spinner').hidden = true;
  if (!loaded.ok) {
    imgs.A.classList.remove('on'); imgs.B.classList.remove('on');
    setMsg('Photos can\'t show this picture', loaded.error);
    S.nat = { w: 0, h: 0 };
  } else {
    setMsg('');
    S.nat = { w: el.naturalWidth || 1024, h: el.naturalHeight || 768 };
    if (extOf(p.path) === 'svg' && !el.naturalWidth) S.nat = { w: 1024, h: 1024 };
    el.classList.toggle('checker', /^(png|apng|gif|webp|svg|ico|avif)$/.test(extOf(p.path)));
    const fade = !!S.slide;
    for (const k of ['A', 'B']) imgs[k].classList.toggle('fade', fade);
    el.classList.add('on');
    imgs[S.front].classList.remove('on');
    S.front = back;
    S.z.mode = opts.keepZoom ? S.z.mode : 'fit';
    layout();
  }
  invoke('set_title', { title: `${p.name} – Photos` }).catch(() => {});
  loadInfo(seq);
  preload();
  if (S.slide) S.slide.armed = true, armSlide();
}

/** Loads a picture into an <img>: the file itself, or a PNG made of it when the web view can't draw it. */
function loadInto(el, path, v) {
  const q = v ? `?v=${v}` : '';
  const tries = NATIVE.has(extOf(path)) ? [url('raw', path, q), url('view', path, q)] : [url('view', path, q)];
  return new Promise(resolve => {
    let k = 0;
    const next = () => {
      if (k >= tries.length) { el.onload = el.onerror = null; whyNot(path).then(error => resolve({ ok: false, error })); return; }
      el.onload = () => { el.onload = el.onerror = null; const route = tries[k].includes('/view/') ? 'converted' : 'web view'; (el.decode ? el.decode().catch(() => {}) : Promise.resolve()).then(() => resolve({ ok: true, route })); };
      el.onerror = () => { k++; next(); };
      el.src = tries[k];
    };
    next();
  });
}

async function whyNot(path) {
  try { const r = await fetch(url('view', path)); if (!r.ok) return await r.text(); } catch (_) {}
  const e = extOf(path);
  if (/^(heic|heif|hif)$/.test(e)) return 'HEIC pictures need libheif\'s tools (heif-dec).';
  return 'The file may be damaged, or not a picture.';
}

function preload() {
  for (const d of [1, -1]) {
    const p = S.pics[(S.i + d + S.pics.length) % S.pics.length];
    if (p && p !== cur() && NATIVE.has(extOf(p.path))) { const im = new Image(); im.decoding = 'async'; im.src = url('raw', p.path, `?v=${p.mtime}`); }
  }
}

async function loadInfo(seq) {
  const p = cur();
  if (!p) return;
  try {
    const info = await invoke('file_info', { path: p.path });
    if (seq !== S.loadSeq) return;
    S.info = info;
  } catch (e) { S.info = { name: p.name, path: p.path, size: p.size, error: String(e), camera: [] }; }
  renderInfo();
  renderTitle();
}

// ---------------------------------------------------------------- zoom and pan
function stageBox() { const r = stage.getBoundingClientRect(); return { w: r.width || 1, h: r.height || 1 }; }
function upright() { const c = cssOf(S.turn); return c.swaps ? { w: S.nat.h, h: S.nat.w } : { w: S.nat.w, h: S.nat.h }; }
function fitScale(kind = 'fit') {
  const b = stageBox(), u = upright();
  if (!u.w || !u.h) return 1;
  const pad = S.fs || !S.chrome ? 0 : 24;
  const sx = (b.w - pad) / u.w, sy = (b.h - pad) / u.h;
  if (kind === 'fill') return Math.max(sx, sy);
  return Math.min(1, Math.min(sx, sy)) || 1; // small pictures aren't blown up to fit
}
function minScale() { return Math.min(fitScale(), 0.05); }
function clampPan() {
  const b = stageBox(), u = upright(), dw = u.w * S.z.s, dh = u.h * S.z.s;
  const mx = Math.max(0, (dw - b.w) / 2), my = Math.max(0, (dh - b.h) / 2);
  S.z.x = Math.max(-mx, Math.min(mx, S.z.x));
  S.z.y = Math.max(-my, Math.min(my, S.z.y));
}
function layout() {
  if (S.z.mode === 'fit') { S.z.s = fitScale(); S.z.x = S.z.y = 0; }
  else if (S.z.mode === 'fill') { S.z.s = fitScale('fill'); }
  else if (S.z.mode === 'actual') { S.z.s = 1; }
  clampPan();
  const el = imgs[S.front], c = cssOf(S.turn);
  el.style.width = S.nat.w + 'px';
  el.style.height = S.nat.h + 'px';
  el.style.transform = `translate(-50%,-50%) translate(${S.z.x}px,${S.z.y}px) rotate(${c.deg}deg) scaleX(${c.flip ? -1 : 1}) scale(${S.z.s})`;
  el.classList.toggle('pixelated', S.z.s >= 3);
  const zoomed = S.z.s > fitScale() + 1e-3;
  stage.classList.toggle('grab', zoomed);
  $('#zoomLabel').textContent = S.z.mode === 'fit' ? 'Fit' : S.z.mode === 'fill' ? 'Fill' : Math.round(S.z.s * 100) + '%';
}
function zoomTo(s, cx = 0, cy = 0) {
  const s1 = S.z.s, s2 = Math.max(minScale(), Math.min(40, s));
  S.z.x = cx - (cx - S.z.x) * (s2 / s1);
  S.z.y = cy - (cy - S.z.y) * (s2 / s1);
  S.z.s = s2;
  S.z.mode = Math.abs(s2 - fitScale()) < 1e-3 ? 'fit' : 'free';
  layout();
}
const zoomBy = (f, cx, cy) => zoomTo(S.z.s * f, cx, cy);
function zoomCycle() {
  const order = ['fit', 'actual', 'fill'];
  S.z.mode = order[(order.indexOf(S.z.mode) + 1) % order.length] || 'fit';
  if (S.z.mode !== 'fit') { S.z.x = S.z.y = 0; }
  layout();
}
const isZoomed = () => S.z.s > fitScale() + 1e-3;
function panBy(dx, dy) { S.z.x += dx; S.z.y += dy; layout(); }
window.addEventListener('resize', () => { layout(); renderStrip(); });

// Wheel: zoom where the pointer is (touchpad pinch comes as Ctrl+wheel). Shift+wheel or sideways moves through pictures.
let wheelNav = 0;
stage.addEventListener('wheel', e => {
  if (!cur() || $('#editor').hidden === false) return;
  e.preventDefault();
  const r = stage.getBoundingClientRect();
  const cx = e.clientX - r.left - r.width / 2, cy = e.clientY - r.top - r.height / 2;
  if (!e.ctrlKey && (e.shiftKey || Math.abs(e.deltaX) > Math.abs(e.deltaY) * 1.5)) {
    const now = performance.now();
    if (now - wheelNav > 280) { wheelNav = now; step((e.deltaX || e.deltaY) > 0 ? 1 : -1); }
    return;
  }
  const d = e.deltaMode === 1 ? e.deltaY * 33 : e.deltaY;
  zoomBy(Math.exp(-d * (e.ctrlKey ? 0.01 : 0.0022)), cx, cy);
}, { passive: false });

// Pointers: drag to pan when zoomed, swipe to move through pictures when not, two fingers to pinch.
const ptrs = new Map();
let gesture = null, lastTap = 0;
stage.addEventListener('pointerdown', e => {
  if (!cur()) return;
  stage.setPointerCapture(e.pointerId);
  ptrs.set(e.pointerId, { x: e.clientX, y: e.clientY });
  if (ptrs.size === 2) {
    const [a, b] = [...ptrs.values()];
    gesture = { kind: 'pinch', d: Math.hypot(a.x - b.x, a.y - b.y), s: S.z.s, x: S.z.x, y: S.z.y, mx: (a.x + b.x) / 2, my: (a.y + b.y) / 2 };
  } else if (ptrs.size === 1) {
    gesture = { kind: isZoomed() ? 'pan' : 'swipe', x0: e.clientX, y0: e.clientY, px: S.z.x, py: S.z.y, t: performance.now() };
    if (gesture.kind === 'pan') stage.classList.add('grabbing');
  }
  showChrome();
});
stage.addEventListener('pointermove', e => {
  if (!ptrs.has(e.pointerId)) return;
  ptrs.set(e.pointerId, { x: e.clientX, y: e.clientY });
  if (!gesture) return;
  if (gesture.kind === 'pinch' && ptrs.size >= 2) {
    const [a, b] = [...ptrs.values()];
    const r = stage.getBoundingClientRect();
    const d = Math.hypot(a.x - b.x, a.y - b.y), mx = (a.x + b.x) / 2, my = (a.y + b.y) / 2;
    S.z.s = gesture.s; S.z.x = gesture.x + (mx - gesture.mx); S.z.y = gesture.y + (my - gesture.my);
    zoomTo(gesture.s * d / gesture.d, mx - r.left - r.width / 2, my - r.top - r.height / 2);
  } else if (gesture.kind === 'pan') {
    S.z.x = gesture.px + e.clientX - gesture.x0; S.z.y = gesture.py + e.clientY - gesture.y0; layout();
  } else if (gesture.kind === 'swipe') {
    const dx = e.clientX - gesture.x0;
    imgs[S.front].style.translate = `${dx}px 0`;
  }
});
function endPointer(e) {
  if (!ptrs.has(e.pointerId)) return;
  ptrs.delete(e.pointerId);
  stage.classList.remove('grabbing');
  if (gesture && gesture.kind === 'swipe' && ptrs.size === 0) {
    const dx = e.clientX - gesture.x0, dy = e.clientY - gesture.y0, dt = performance.now() - gesture.t;
    imgs[S.front].style.translate = '';
    if (Math.abs(dx) > 70 && Math.abs(dx) > Math.abs(dy) * 1.4) step(dx < 0 ? 1 : -1);
    else if (Math.abs(dx) < 8 && Math.abs(dy) < 8 && dt < 300) {
      const now = performance.now();
      if (now - lastTap < 330) { // double tap / double click: 100% where you tapped, or back to fit
        const r = stage.getBoundingClientRect();
        if (isZoomed()) { S.z.mode = 'fit'; layout(); } else zoomTo(Math.max(1, fitScale() * 2.5), e.clientX - r.left - r.width / 2, e.clientY - r.top - r.height / 2);
        lastTap = 0;
      } else { lastTap = now; if (e.pointerType !== 'mouse') setTimeout(() => { if (lastTap === now) toggleChrome(); }, 340); }
    }
  }
  if (ptrs.size === 0) gesture = null;
  else if (ptrs.size === 1 && gesture && gesture.kind === 'pinch') {
    const [p] = [...ptrs.values()];
    gesture = { kind: 'pan', x0: p.x, y0: p.y, px: S.z.x, py: S.z.y, t: performance.now() };
  }
}
stage.addEventListener('pointerup', endPointer);
stage.addEventListener('pointercancel', endPointer);

// ---------------------------------------------------------------- moving through the folder
function step(d) {
  if (!S.pics.length) return;
  if (S.pics.length === 1) { bump(d); return; }
  show(S.i + d);
}
function bump(d) { const el = imgs[S.front]; el.animate([{ translate: '0 0' }, { translate: `${-d * 24}px 0` }, { translate: '0 0' }], { duration: 220 }); }

function sortPics() {
  const keep = cur();
  const by = { name: null, newest: (a, b) => b.mtime - a.mtime, oldest: (a, b) => a.mtime - b.mtime, largest: (a, b) => b.size - a.size };
  if (S.sort === 'name') S.pics.sort((a, b) => S.nameOrder.get(a.path) - S.nameOrder.get(b.path));
  else S.pics.sort(by[S.sort]);
  if (keep) S.i = S.pics.indexOf(keep);
}

async function openPath(path, opts = {}) {
  try {
    const l = await invoke('list', { path });
    S.dir = l.dir;
    S.pics = l.pics;
    S.nameOrder = new Map(l.pics.map((p, k) => [p.path, k]));
    S.i = l.index;
    sortPics();
    stripRendered = '';
    $('#welcome').hidden = true;
    if (!S.pics.length) { render(); setMsg('No pictures here', l.dir); return; }
    await show(S.i, opts);
    if (opts.slideshow) startSlide();
  } catch (e) { toast(String(e)); render(); }
}

// ---------------------------------------------------------------- what's on screen
function renderTitle() {
  const p = cur();
  $('#name').textContent = p ? p.name : 'Photos';
  const i = S.info, bits = [];
  if (p) {
    const u = upright();
    if (u.w) bits.push(`${u.w} × ${u.h}`);
    bits.push(human((i && i.size) ?? p.size));
  }
  $('#sub').textContent = bits.join('  ·  ');
  $('#count').textContent = p ? `${S.i + 1} of ${S.pics.length}` : '';
}
function render() {
  const has = !!cur();
  $('#welcome').hidden = has || S.pics.length > 0;
  $('#app').classList.toggle('noStrip', S.pics.length < 2);
  document.body.classList.toggle('noStrip', S.pics.length < 2);
  $('#app').classList.toggle('infoOpen', S.infoOpen && has);
  $('#info').hidden = !(S.infoOpen && has);
  $('#infoBtn').classList.toggle('on', S.infoOpen);
  $$('.nav').forEach(n => n.disabled = S.pics.length < 2);
  $('#turnbar').hidden = S.turn === 1 || !has;
  if (S.turn !== 1) $('#turnText').textContent = 'Turned or mirrored, not saved yet';
  $$('#tools [data-f]').forEach(b => { if (b.dataset.act !== 'more') b.disabled = !has; });
  if (!has) { imgs.A.classList.remove('on'); imgs.B.classList.remove('on'); }
  renderTitle();
  renderStrip();
  if (S.infoOpen) renderInfo();
  layout();
}

const dateOf = ms => ms ? new Date(ms).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' }) : '';
function takenText(t) {
  const m = /^(\d{4})[-:](\d\d)[-:](\d\d)[ T](\d\d):(\d\d)/.exec(t || '');
  return m ? new Date(+m[1], m[2] - 1, +m[3], +m[4], +m[5]).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' }) : (t || '');
}
function renderInfo() {
  if (!S.infoOpen) return;
  const p = cur(), i = S.info;
  if (!p) { $('#infoBody').innerHTML = ''; return; }
  const u = upright();
  const kv = rows => `<dl class="kv">${rows.filter(r => r[1]).map(([k, v]) => `<dt>${esc(k)}</dt><dd>${esc(v)}</dd>`).join('')}</dl>`;
  let h = `<div class="big1">${esc(p.name)}</div>`;
  h += `<div class="isec"><h3>Picture</h3>${kv([
    ['Size', u.w ? `${u.w} × ${u.h} pixels (${(u.w * u.h / 1e6).toFixed(1)} MP)` : (i && i.width ? `${i.width} × ${i.height} pixels` : '')],
    ['Type', i ? (i.mime || '').replace('image/', '').toUpperCase().replace('SVG+XML', 'SVG').replace('X-ICON', 'ICO') : ''],
    ['File size', human(i ? i.size : p.size)],
    ['Taken', i && i.taken ? takenText(i.taken) : ''],
  ])}</div>`;
  if (i && i.camera && i.camera.length) h += `<div class="isec"><h3>Camera</h3>${kv(i.camera)}</div>`;
  if (i && i.gps) h += `<div class="isec"><h3>Place</h3>${kv([['Location', `${i.gps[0].toFixed(5)}, ${i.gps[1].toFixed(5)}`]])}</div>`;
  h += `<div class="isec"><h3>File</h3>${kv([['Changed', dateOf(i ? i.modified : p.mtime)], ['Created', i ? dateOf(i.created) : ''], ['Folder', i ? i.folder : S.dir]])}</div>`;
  if (i && i.error) h += `<p class="note">${esc(i.error)}</p>`;
  $('#infoBody').innerHTML = h;
}

// ---------------------------------------------------------------- strip of thumbnails (only the visible ones exist)
const CELL = 84, strip = $('#strip'), stripInner = $('#stripInner');
let stripRendered = '';
function renderStrip(center = true) {
  if (S.pics.length < 2) { stripInner.innerHTML = ''; return; }
  stripInner.style.width = (S.pics.length * CELL + 16) + 'px';
  if (center && S.i >= 0) {
    const want = 8 + S.i * CELL + CELL / 2 - strip.clientWidth / 2;
    if (Math.abs(strip.scrollLeft - want) > strip.clientWidth / 3 || center === 'force') strip.scrollTo({ left: want, behavior: Math.abs(strip.scrollLeft - want) > 3000 ? 'auto' : 'smooth' });
  }
  drawStrip();
}
function drawStrip() {
  const first = Math.max(0, Math.floor((strip.scrollLeft - 8) / CELL) - 4);
  const last = Math.min(S.pics.length - 1, Math.ceil((strip.scrollLeft + strip.clientWidth) / CELL) + 4);
  const key = `${first}-${last}-${S.i}-${S.pics.length}-${S.sort}`;
  if (key === stripRendered) return;
  stripRendered = key;
  const keep = new Map($$('.th', stripInner).map(el => [el.dataset.path, el]));
  const frag = document.createDocumentFragment();
  for (let k = first; k <= last; k++) {
    const p = S.pics[k];
    let el = keep.get(p.path);
    if (el) keep.delete(p.path);
    else {
      el = document.createElement('button');
      el.className = 'th';
      el.dataset.path = p.path;
      el.tabIndex = -1;
      el.innerHTML = `<span class="ph">${esc(extOf(p.name))}</span><img alt="" decoding="async" draggable="false">`;
      const im = el.lastChild;
      im.onload = () => { im.classList.add('ok'); const ph = el.querySelector('.ph'); if (ph) ph.remove(); };
      im.onerror = () => { if (extOf(p.path) === 'svg' && !im.dataset.raw) { im.dataset.raw = 1; im.src = url('raw', p.path); } };
      im.src = extOf(p.path) === 'svg' ? url('raw', p.path) : url('thumb', p.path, `?s=160&v=${p.mtime}`);
    }
    el.style.left = (8 + k * CELL) + 'px';
    el.dataset.i = k;
    el.classList.toggle('cur', k === S.i);
    el.title = p.name;
    frag.appendChild(el);
  }
  for (const el of keep.values()) el.remove(); // scrolled away: their thumbnails stop loading
  stripInner.appendChild(frag);
}
strip.addEventListener('scroll', () => requestAnimationFrame(drawStrip), { passive: true });
strip.addEventListener('click', e => { const t = e.target.closest('.th'); if (t) show(+t.dataset.i); });
strip.addEventListener('wheel', e => { if (Math.abs(e.deltaY) > Math.abs(e.deltaX)) { strip.scrollLeft += e.deltaY; e.preventDefault(); } }, { passive: false });

// ---------------------------------------------------------------- chrome, full screen
let idleTimer = 0;
function showChrome() {
  $('#app').classList.remove('idle');
  if (!S.chrome) { S.chrome = true; $('#app').classList.add('chrome'); layout(); }
  clearTimeout(idleTimer);
  if (S.fs || S.slide) idleTimer = setTimeout(() => { if (!$('#menu').hidden || $('#layer').firstChild || document.activeElement?.closest('#top,#slidectl')?.matches(':focus-within') && S.pad) return; if (S.fs || S.slide) hideChrome(); }, 2600);
}
function hideChrome() { S.chrome = false; $('#app').classList.remove('chrome'); $('#app').classList.add('idle'); if (document.activeElement?.closest('#top')) stage.focus(); layout(); }
function toggleChrome() { S.chrome ? hideChrome() : showChrome(); }
document.addEventListener('mousemove', e => { if (e.movementX || e.movementY) showChrome(); });

async function setFullscreen(on) {
  try { S.fs = T ? await invoke('fullscreen', { on }) : on; } catch (_) { S.fs = on; }
  if (!T) { try { on ? await document.documentElement.requestFullscreen() : document.fullscreenElement && await document.exitFullscreen(); } catch (_) {} }
  $('#app').classList.toggle('fs', S.fs);
  S.fs ? showChrome() : (clearTimeout(idleTimer), showChrome());
  setTimeout(() => { layout(); renderStrip(); }, 120);
}

// ---------------------------------------------------------------- slideshow
function startSlide() {
  if (S.pics.length < 1) return;
  S.slide = { wasFs: S.fs, paused: false, timer: 0, armed: true };
  $('#slidectl').hidden = false;
  $('#slideSpeed').textContent = (S.slideMs / 1000) + ' s';
  if (!S.fs) setFullscreen(true);
  hideChrome();
  armSlide();
}
function armSlide() {
  if (!S.slide) return;
  clearTimeout(S.slide.timer);
  if (S.slide.paused) return;
  S.slide.timer = setTimeout(() => { if (S.slide && !S.slide.paused) show(S.i + 1); }, S.slideMs);
}
function stopSlide() {
  if (!S.slide) return;
  clearTimeout(S.slide.timer);
  const wasFs = S.slide.wasFs;
  S.slide = null;
  $('#slidectl').hidden = true;
  for (const k of ['A', 'B']) imgs[k].classList.remove('fade');
  if (!wasFs) setFullscreen(false);
  showChrome();
}
function pauseSlide() {
  if (!S.slide) return;
  S.slide.paused = !S.slide.paused;
  $('#slidePause').innerHTML = S.slide.paused ? '<svg viewBox="0 0 24 24"><path d="m8 5 11 7-11 7z"/></svg>' : '<svg viewBox="0 0 24 24"><path d="M9 6v12M15 6v12"/></svg>';
  toast(S.slide.paused ? 'Paused' : 'Playing');
  armSlide();
}
function slideSpeed() {
  const opts = [3000, 5000, 10000, 20000];
  S.slideMs = opts[(opts.indexOf(S.slideMs) + 1) % opts.length];
  store.set('slideMs', S.slideMs);
  $('#slideSpeed').textContent = (S.slideMs / 1000) + ' s';
  armSlide();
}

// ---------------------------------------------------------------- turning (saved on request)
function turn(op) {
  if (!cur()) return;
  S.turn = then(S.turn, op);
  S.z.mode = 'fit';
  render();
}
async function saveTurn() {
  const p = cur();
  if (!p || S.turn === 1) return;
  try {
    const r = await invoke('save_turn', { path: p.path, op: S.turn });
    S.turn = 1;
    toast(r.message);
    if (r.path !== p.path) await openPath(r.path);
    else { p.mtime = Date.now(); stripRendered = ''; $$('.th').forEach(t => { if (t.dataset.path === p.path) t.remove(); }); await show(S.i, { keepTurn: true }); }
  } catch (e) { toast(String(e)); }
}
function discardTurn() { S.turn = 1; S.z.mode = 'fit'; render(); }

// ---------------------------------------------------------------- Trash with undo
let lastTrashed = null;
async function trashCurrent() {
  const p = cur();
  if (!p) return;
  try {
    await invoke('trash', { path: p.path });
    lastTrashed = { pic: p, at: S.i };
    S.pics.splice(S.i, 1);
    S.turn = 1;
    stripRendered = '';
    if (!S.pics.length) { S.i = -1; render(); setMsg('No more pictures here', S.dir); }
    else await show(Math.min(S.i, S.pics.length - 1), { keepTurn: true });
    toast(`${p.name} was moved to the Trash`, { label: 'Undo', run: undoTrash }, 9000);
  } catch (e) { toast(String(e)); }
}
async function undoTrash() {
  const t = lastTrashed;
  if (!t) return;
  try {
    await invoke('untrash', { path: t.pic.path });
    lastTrashed = null;
    S.pics.splice(Math.min(t.at, S.pics.length), 0, t.pic);
    setMsg('');
    stripRendered = '';
    await show(Math.min(t.at, S.pics.length - 1), { keepTurn: true });
    toast('Taken back out of the Trash');
  } catch (e) { toast(String(e)); }
}

// ---------------------------------------------------------------- the rest of what you can do
async function doCopy() { const p = cur(); if (!p) return; try { await invoke('copy_image', { path: p.path }); toast('Copied the picture'); } catch (e) { toast(String(e)); } }
async function doWallpaper() { const p = cur(); if (!p) return; try { toast(await invoke('set_wallpaper', { path: p.path })); } catch (e) { toast(String(e)); } }
async function doShowFolder() { const p = cur(); if (!p) return; try { await invoke('show_in_folder', { path: p.path }); } catch (e) { toast(String(e)); } }
async function doOpen() {
  try { const p = await invoke('pick', { from: cur() ? cur().path : null }); if (p) openPath(p); } catch (e) { toast(String(e)); }
}
async function doPrint() {
  const p = cur();
  if (!p) return;
  const box = $('#print');
  box.innerHTML = '';
  const im = new Image();
  const c = cssOf(S.turn);
  await loadInto(im, p.path);
  if (S.turn !== 1) { // print as turned on screen
    const cv = document.createElement('canvas');
    const w = im.naturalWidth, h = im.naturalHeight;
    cv.width = c.swaps ? h : w; cv.height = c.swaps ? w : h;
    const g = cv.getContext('2d');
    g.translate(cv.width / 2, cv.height / 2); g.rotate(c.deg * Math.PI / 180); g.scale(c.flip ? -1 : 1, 1); g.drawImage(im, -w / 2, -h / 2);
    box.appendChild(cv); cv.style.maxWidth = '100%'; cv.style.maxHeight = '100%';
  } else box.appendChild(im);
  try { T ? await invoke('print') : window.print(); } catch (e) { toast(String(e)); }
}
function doEdit() {
  const p = cur();
  if (!p || !window.Editor) return;
  if (S.slide) stopSlide();
  window.Editor.open(p, S.turn);
}

const actions = {
  prev: () => step(-1), next: () => step(1),
  zoomIn: () => zoomBy(1.25), zoomOut: () => zoomBy(0.8), zoomCycle,
  rotateLeft: () => turn(OP.left), rotateRight: () => turn(OP.right), flipH: () => turn(OP.flipH), flipV: () => turn(OP.flipV),
  saveTurn, discardTurn,
  edit: doEdit,
  info: () => { S.infoOpen = !S.infoOpen; store.set('info', S.infoOpen); render(); },
  slideshow: () => S.slide ? stopSlide() : startSlide(),
  slidePause: pauseSlide, slideSpeed, slideStop: stopSlide,
  fullscreen: () => setFullscreen(!S.fs),
  trash: trashCurrent,
  more: () => moreMenu(),
  open: doOpen,
  openPictures: async () => { const r = await invoke('init').catch(() => ({})); if (r.pictures) openPath(r.pictures); else doOpen(); },
};
document.addEventListener('click', e => {
  const b = e.target.closest('[data-act]');
  if (!b || b.disabled || b.closest('#editor')) return;
  const f = actions[b.dataset.act];
  if (f) { e.preventDefault(); f(); }
});

function moreMenu() {
  const has = !!cur();
  const sortItem = (id, label) => ({ label, on: S.sort === id, run: () => { S.sort = id; store.set('sort', id); sortPics(); stripRendered = ''; render(); renderStrip('force'); } });
  openMenu($('#moreBtn'), [
    { label: 'Open a picture…', key: 'Ctrl+O', run: doOpen },
    { sep: true },
    { label: 'Copy picture', key: 'Ctrl+C', run: doCopy, disabled: !has },
    { label: 'Set as wallpaper', run: doWallpaper, disabled: !has },
    { label: 'Show in its folder', run: doShowFolder, disabled: !has },
    { label: 'Print…', key: 'Ctrl+P', run: doPrint, disabled: !has },
    { label: 'Mirror top to bottom', key: 'V', run: () => turn(OP.flipV), disabled: !has },
    { sep: true },
    { title: 'Order' },
    sortItem('name', 'By name'), sortItem('newest', 'Newest first'), sortItem('oldest', 'Oldest first'), sortItem('largest', 'Largest first'),
    { sep: true },
    { title: 'Slideshow' },
    ...[3000, 5000, 10000].map(ms => ({ label: `Every ${ms / 1000} seconds`, on: S.slideMs === ms, run: () => { S.slideMs = ms; store.set('slideMs', ms); } })),
    { sep: true },
    { label: 'Keyboard and controller', run: helpDialog },
  ]);
}

// ---------------------------------------------------------------- menus, dialogs, toasts (shared with edit.js)
let menuReturn = null;
function openMenu(anchor, items) {
  const m = $('#menu');
  menuReturn = document.activeElement;
  m.innerHTML = items.map((it, k) => it.sep ? '<div class="msep"></div>' : it.title ? `<div class="mlabel">${esc(it.title)}</div>`
    : `<button class="mi${it.on ? ' on' : ''}" data-f data-mi="${k}" ${it.disabled ? 'disabled style="opacity:.45"' : ''}>${esc(it.label)}${it.key && !it.on ? `<span class="k">${esc(it.key)}</span>` : ''}</button>`).join('');
  m.hidden = false;
  const r = anchor ? anchor.getBoundingClientRect() : { right: innerWidth / 2 + 130, bottom: innerHeight / 3, top: innerHeight / 3 };
  const mw = m.offsetWidth, mh = m.offsetHeight;
  m.style.left = Math.max(10, Math.min(innerWidth - mw - 10, r.right - mw)) + 'px';
  m.style.top = Math.max(10, Math.min(innerHeight - mh - 10, r.bottom + 6)) + 'px';
  m.onclick = e => { const b = e.target.closest('[data-mi]'); if (!b || b.disabled) return; closeMenu(); items[+b.dataset.mi].run(); };
  const first = $('.mi:not([disabled])', m);
  if (first) { first.focus(); if (S.pad) first.classList.add('kf'); }
}
function closeMenu() { const m = $('#menu'); if (m.hidden) return false; m.hidden = true; if (menuReturn && document.contains(menuReturn)) menuReturn.focus(); return true; }
document.addEventListener('pointerdown', e => { if (!$('#menu').hidden && !e.target.closest('#menu')) closeMenu(); }, true);

/** A dialog: html inside, buttons [{label, kind, value}]. Resolves with the chosen value (null for Back). */
function dialog(html, buttons, onOpen) {
  return new Promise(resolve => {
    const ret = document.activeElement;
    const v = document.createElement('div');
    v.className = 'veil';
    v.innerHTML = `<div class="dlg" role="dialog">${html}<div class="row">${buttons.map((b, k) => `<button class="btn ${b.kind || ''}" data-f data-b="${k}">${esc(b.label)}</button>`).join('')}</div></div>`;
    const done = val => { v.remove(); if (ret && document.contains(ret)) ret.focus(); resolve(val); };
    v._back = () => done(null);
    v.addEventListener('click', e => { const b = e.target.closest('[data-b]'); if (b) done(buttons[+b.dataset.b].value); else if (e.target === v) done(null); });
    $('#layer').appendChild(v);
    if (onOpen) onOpen(v, done);
    const f = $('[autofocus]', v) || $(`[data-b="${buttons.findIndex(b => b.focus)}"]`, v) || $('[data-b]:last-child', v);
    if (f) f.focus();
  });
}
function helpDialog() {
  dialog(`<h2>Keyboard and controller</h2><dl class="kv">
    <dt>← → or D-pad</dt><dd>Previous / next picture (LB / RB too)</dd>
    <dt>+ − 0 1, LT / RT</dt><dd>Zoom in / out, fit, 100% (right stick pans)</dd>
    <dt>R, Shift+R, H, V</dt><dd>Turn right / left, mirror, flip top to bottom</dd>
    <dt>E</dt><dd>Edit (crop, straighten, light and colour, filters, markup)</dd>
    <dt>I or X</dt><dd>Info</dd><dt>F11 or View</dt><dd>Full screen</dd><dt>F5 or Start</dt><dd>Slideshow</dd>
    <dt>Delete</dt><dd>Move to the Trash (Ctrl+Z takes it back)</dd>
    <dt>Ctrl+C, Ctrl+O, Ctrl+P</dt><dd>Copy, open, print</dd>
    <dt>A</dt><dd>Use the tools at the top</dd><dt>B or Esc</dt><dd>Back</dd></dl>`, [{ label: 'Close', kind: 'primary', value: 1 }]);
}

let toastTimer = 0;
function toast(text, action, ms = 3200) {
  const t = $('#toast');
  t.innerHTML = `<span>${esc(text)}</span>${action ? `<button class="btn" data-f id="toastAct">${esc(action.label)}</button>` : ''}`;
  t.hidden = false;
  if (action) $('#toastAct').onclick = () => { t.hidden = true; action.run(); };
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { t.hidden = true; }, ms);
}

// ---------------------------------------------------------------- moving focus with arrows / D-pad
function topLayer() {
  const v = $('#layer .veil:last-child');
  if (v) return v;
  if (!$('#menu').hidden) return $('#menu');
  if (!$('#editor').hidden) return $('#editor');
  return $('#app');
}
function focusables(root) {
  return $$('[data-f]', root).filter(el => !el.disabled && el.offsetParent !== null && el.getClientRects().length);
}
function move(dir, root = topLayer()) {
  const list = focusables(root);
  const a = document.activeElement;
  if (!list.length) return false;
  if (!a || !list.includes(a)) { list[0].focus(); list[0].classList.add('kf'); return true; }
  const r = a.getBoundingClientRect(), cx = r.left + r.width / 2, cy = r.top + r.height / 2;
  let best = null, bestScore = Infinity;
  for (const el of list) {
    if (el === a) continue;
    const q = el.getBoundingClientRect(), x = q.left + q.width / 2, y = q.top + q.height / 2, dx = x - cx, dy = y - cy;
    const ok = dir === 'left' ? dx < -4 : dir === 'right' ? dx > 4 : dir === 'up' ? dy < -4 : dy > 4;
    if (!ok) continue;
    const horiz = dir === 'left' || dir === 'right';
    const score = (horiz ? Math.abs(dx) : Math.abs(dy)) + (horiz ? Math.abs(dy) : Math.abs(dx)) * 2.2;
    if (score < bestScore) { bestScore = score; best = el; }
  }
  if (best) { a.classList.remove('kf'); best.focus(); best.classList.add('kf'); best.scrollIntoView({ block: 'nearest', inline: 'nearest' }); return true; }
  return false;
}
document.addEventListener('focusout', e => e.target.classList?.remove('kf'));

/** Back (Esc, B): close the topmost thing. */
function back() {
  const v = $('#layer .veil:last-child');
  if (v) { v._back(); return; }
  if (closeMenu()) return;
  if (!$('#editor').hidden) { window.Editor.back(); return; }
  if (!$('#toast').hidden && document.activeElement?.id === 'toastAct') { $('#toast').hidden = true; stage.focus(); return; }
  if (document.activeElement && document.activeElement.closest('#top,#turnbar,#slidectl,#welcome')) { document.activeElement.blur(); stage.focus(); if (S.fs) hideChrome(); return; }
  if (S.slide) { stopSlide(); return; }
  if (S.infoOpen) { actions.info(); return; }
  if (isZoomed()) { S.z.mode = 'fit'; layout(); return; }
  if (S.fs) { setFullscreen(false); return; }
}

// ---------------------------------------------------------------- keyboard
document.addEventListener('keydown', e => {
  S.pad = false;
  if (!$('#editor').hidden && window.Editor.key(e)) return;
  const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(e.target.tagName) && e.target.type !== 'range';
  const k = e.key, ctrl = e.ctrlKey || e.metaKey;
  if (k === 'Escape') { e.preventDefault(); back(); return; }
  if (typing) return;
  const inUi = topLayer() !== $('#app') || document.activeElement?.closest('#top,#turnbar,#slidectl,#welcome,#info,#toast');
  const dirs = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
  if (dirs[k] && inUi) { e.preventDefault(); if (!move(dirs[k]) && topLayer() === $('#app') && (k === 'ArrowDown')) stage.focus(); return; }
  if (topLayer() !== $('#app')) { if (k === 'Enter' || k === ' ') return; return; }
  if (S.slide) {
    if (k === ' ') { e.preventDefault(); pauseSlide(); return; }
    if (k === 'ArrowRight' || k === 'ArrowLeft') { e.preventDefault(); step(k === 'ArrowRight' ? 1 : -1); return; }
    if (k === 'F5') { e.preventDefault(); stopSlide(); return; }
  }
  const pan = 80;
  const map = {
    ArrowLeft: () => isZoomed() ? panBy(pan, 0) : step(-1), ArrowRight: () => isZoomed() ? panBy(-pan, 0) : step(1),
    ArrowUp: () => isZoomed() ? panBy(0, pan) : null, ArrowDown: () => isZoomed() ? panBy(0, -pan) : null,
    PageUp: () => step(-1), PageDown: () => step(1), Home: () => show(0), End: () => show(S.pics.length - 1),
    '+': () => zoomBy(1.25), '=': () => zoomBy(1.25), '-': () => zoomBy(0.8), '_': () => zoomBy(0.8),
    '0': () => { S.z.mode = 'fit'; layout(); }, '1': () => { S.z.mode = 'actual'; S.z.x = S.z.y = 0; layout(); }, '2': () => { S.z.mode = 'fill'; layout(); },
    r: () => turn(OP.right), R: () => turn(OP.left), l: () => turn(OP.left), h: () => turn(OP.flipH), v: () => turn(OP.flipV),
    i: actions.info, e: doEdit, f: actions.fullscreen, F11: actions.fullscreen, F5: actions.slideshow, s: actions.slideshow,
    Delete: trashCurrent, Enter: () => { if (document.activeElement === stage || document.activeElement === document.body) { showChrome(); move('right', $('#top')); } },
  };
  const cmap = { o: doOpen, c: doCopy, p: doPrint, z: undoTrash, s: saveTurn, '0': map['0'], '=': map['+'], '+': map['+'], '-': map['-'], e: doEdit, q: () => invoke('quit').catch(() => window.close()), w: () => invoke('quit').catch(() => window.close()) };
  const f = ctrl ? cmap[k.toLowerCase()] || cmap[k] : (e.altKey ? null : map[k]);
  if (f) { e.preventDefault(); f(); }
});

// ---------------------------------------------------------------- controllers (standard mapping)
// Edges become actions; held D-pad repeats. LT/RT zoom smoothly; the right stick pans; the left stick moves focus
// in menus and pans when zoomed in.
const held = {};
let lastPadMove = 0;
const BTN = { a: 0, b: 1, x: 2, y: 3, lb: 4, rb: 5, lt: 6, rt: 7, view: 8, start: 9, ls: 10, rs: 11, up: 12, down: 13, left: 14, right: 15 };
function padFrame() {
  const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : [];
  if (pads.length) {
    const now = performance.now();
    const st = {}, ax = { lx: 0, ly: 0, rx: 0, ry: 0, lt: 0, rt: 0 };
    for (const g of pads) {
      for (const [name, i] of Object.entries(BTN)) st[name] ||= !!g.buttons[i]?.pressed;
      const a = g.axes;
      const dz = v => Math.abs(v) < 0.18 ? 0 : v;
      ax.lx += dz(a[0] || 0); ax.ly += dz(a[1] || 0); ax.rx += dz(a[2] || 0); ax.ry += dz(a[3] || 0);
      ax.lt = Math.max(ax.lt, g.buttons[6]?.value || 0); ax.rt = Math.max(ax.rt, g.buttons[7]?.value || 0);
    }
    const ui = topLayer() !== $('#app') || !!document.activeElement?.closest('#top,#turnbar,#slidectl,#welcome,#toast');
    // The left stick acts as a D-pad in menus and between pictures (unless zoomed in, when it pans).
    const editing = !$('#editor').hidden && !$('#layer .veil');
    if (!editing && (ui || !isZoomed())) { st.up ||= ax.ly < -0.6; st.down ||= ax.ly > 0.6; st.left ||= ax.lx < -0.6; st.right ||= ax.lx > 0.6; }
    for (const [k, on] of Object.entries(st)) {
      if (!on) { held[k] = 0; continue; }
      const first = !held[k];
      const isDir = ['up', 'down', 'left', 'right', 'lb', 'rb'].includes(k);
      const repeat = held[k] && isDir && now - held[k] > 420 && now - lastPadMove > (k === 'lb' || k === 'rb' ? 240 : 120);
      if (first) held[k] = now;
      if (!(first || repeat)) continue;
      lastPadMove = now;
      S.pad = true;
      padButton(k, ui);
    }
    if (!$('#editor').hidden) window.Editor.padFrame(ax, st);
    else if (cur() && !ui) {
      const z = ax.rt - ax.lt;
      if (Math.abs(z) > 0.05) zoomBy(Math.exp(z * 0.045));
      const px = ax.rx + (isZoomed() ? ax.lx : 0), py = ax.ry + (isZoomed() ? ax.ly : 0);
      if (px || py) panBy(-px * 18, -py * 18);
    }
  }
  requestAnimationFrame(padFrame);
}
function padButton(k, ui) {
  if (k === 'b') { back(); return; }
  if (!$('#editor').hidden && window.Editor.padButton(k)) return;
  const dir = { up: 'up', down: 'down', left: 'left', right: 'right' }[k];
  if (ui) {
    if (dir) { if (!move(dir) && k === 'down' && topLayer() === $('#app')) { document.activeElement.blur(); stage.focus(); } return; }
    if (k === 'a') { const a = document.activeElement; if (a && a !== document.body && a.click) a.click(); return; }
  }
  if (topLayer() !== $('#app')) return;
  showChrome();
  switch (k) {
    case 'left': case 'lb': step(-1); break;
    case 'right': case 'rb': step(1); break;
    case 'up': case 'a': if (cur() || !$('#welcome').hidden) { const first = cur() ? $('#tools [data-f]:not([disabled])') : $('#welcome [data-f]'); if (first) { first.focus(); first.classList.add('kf'); } } break;
    case 'down': break;
    case 'x': actions.info(); break;
    case 'y': moreMenu(); break;
    case 'start': actions.slideshow(); break;
    case 'view': actions.fullscreen(); break;
    case 'rs': zoomCycle(); break;
  }
}
requestAnimationFrame(padFrame);
window.addEventListener('gamepadconnected', () => toast('Controller connected'));

// Files dropped on the window open there.
if (T && T.event) T.event.listen('tauri://drag-drop', e => { const p = e.payload && e.payload.paths && e.payload.paths[0]; if (p) openPath(p); }).catch(() => {});

// ---------------------------------------------------------------- start
window.HP = { invoke, url, loadInto, cur, S, toast, dialog, openMenu, closeMenu, move, focusables, back, openPath, extOf, esc, human, then, cssOf, MATS, store,
  afterEditorClose: () => { layout(); stage.focus(); } };


// ---------------------------------------------------------------- without the app (a plain browser): sample pictures
function demoUrl(route, path) { return (window.HP_DEMO && window.HP_DEMO.url) ? window.HP_DEMO.url(route, path) : (DEMO.find(d => d.path === path) || {}).src || path; }
function demoSvg(hue, label, w = 1600, h = 1000) {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${hue},70%,55%)"/><stop offset="1" stop-color="hsl(${(hue + 60) % 360},65%,22%)"/></linearGradient></defs><rect width="100%" height="100%" fill="url(#g)"/><circle cx="${w * 0.72}" cy="${h * 0.3}" r="${h * 0.12}" fill="hsl(${(hue + 30) % 360},90%,80%)"/><path d="M0 ${h * 0.78} L${w * 0.3} ${h * 0.45} L${w * 0.55} ${h * 0.7} L${w * 0.75} ${h * 0.5} L${w} ${h * 0.8} V${h} H0Z" fill="rgba(0,0,0,.35)"/><text x="60" y="${h - 60}" font-family="sans-serif" font-size="64" fill="white" opacity=".85">${label}</text></svg>`;
  return 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(svg);
}
const DEMO = [
  ['Lakeside.jpg', 20], ['Mountains.jpg', 200], ['Sunset.png', 330], ['Forest.webp', 120], ['Harbour.jpg', 180], ['Desert.jpg', 40],
  ['City at night.jpg', 260], ['Fields.heic', 90], ['Snow.tif', 210], ['Portrait.jpg', 300],
].map(([name, hue], k) => ({ name, path: '/home/you/Pictures/' + name, size: 1.2e6 + k * 310000, mtime: Date.now() - k * 864e5, src: demoSvg(hue, name.replace(/\..*/, ''), k === 9 ? 1000 : 1600, k === 9 ? 1500 : 1000) }));
function demoInvoke(cmd, args = {}) {
  const D = window.HP_DEMO && window.HP_DEMO.pics ? window.HP_DEMO.pics : DEMO;
  const ok = v => Promise.resolve(v);
  switch (cmd) {
    case 'init': return ok({ target: (window.HP_DEMO && window.HP_DEMO.target) || (location.hash === '#empty' ? null : D[0].path), platform: 'linux', pictures: '/home/you/Pictures', converters: [] });
    case 'list': { const i = Math.max(0, D.findIndex(d => d.path === args.path)); return ok({ dir: '/home/you/Pictures', pics: D.map(({ name, path, size, mtime }) => ({ name, path, size, mtime })), index: i }); }
    case 'file_info': { const d = D.find(x => x.path === args.path) || D[0]; return ok({ name: d.name, path: d.path, folder: '/home/you/Pictures', size: d.size, mime: 'image/jpeg', modified: d.mtime, created: d.mtime, width: 1600, height: 1000, orientation: 1, taken: '2025:06:14 19:42:10',
      camera: [['Camera', 'FUJIFILM X-T5'], ['Lens', 'XF16-55mmF2.8 R LM WR'], ['Exposure', 'ƒ/5.6  ·  1/250 s  ·  ISO 160  ·  23 mm (35 mm full-frame)'], ['Flash', 'Didn\'t fire']], gps: [46.0207, 7.7491] }); }
    case 'save_turn': return ok({ path: args.path, how: 'exif', message: 'Saved without losing any quality (preview)' });
    case 'trash': case 'untrash': case 'set_title': case 'copy_image': case 'show_in_folder': case 'print': return ok(null);
    case 'set_wallpaper': return ok('Set as your wallpaper (preview)');
    case 'fullscreen': return ok(!!args.on);
    case 'pick': return ok(null);
    case 'read_bytes': return fetch(demoUrl('raw', args.path)).then(r => r.arrayBuffer());
    case 'save_edit': return ok({ path: '/home/you/Pictures/edited.jpg', how: 'copy', message: 'Saved as edited.jpg (preview)' });
    default: return Promise.reject('Not in the preview');
  }
}

(async function start() {
  let init = {};
  try { init = await invoke('init'); } catch (_) {}
  S.host = init.platform || 'linux';
  S.converters = init.converters || [];
  $('#app').classList.toggle('infoOpen', S.infoOpen);
  if (init.target) await openPath(init.target, { slideshow: init.slideshow });
  if (init.selftest) selftest();
  else { render(); $('#welcome').hidden = false; $('[data-act="open"]').focus(); }
  stage.focus();
})();

/** CI's check: every picture in the folder, how it was shown (the web view itself, or converted), then quit. */
async function selftest() {
  const lines = [];
  for (const p of S.pics) {
    const im = new Image();
    const r = await loadInto(im, p.path, p.mtime);
    lines.push(`${p.name}: ${r.ok ? `shown (${r.route}, ${im.naturalWidth}x${im.naturalHeight})` : 'NOT SHOWN: ' + r.error}`);
  }
  // The strip's thumbnails, from the media scheme.
  let thumbs = 0;
  for (const p of S.pics) { try { const r = await fetch(url('thumb', p.path, '?s=128')); if (r.ok) thumbs++; } catch (_) {} }
  lines.push(`thumbnails: ${thumbs} of ${S.pics.length}`);
  for (const l of lines) await invoke('log', { line: 'SELFTEST ' + l });
  await invoke('log', { line: 'SELFTEST done' });
  await invoke('quit');
}
