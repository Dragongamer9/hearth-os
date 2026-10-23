// Hearth Photos: the editor. Crop (free or a fixed shape), turn and mirror, straighten, resize, light and colour
// (brightness, contrast, saturation, warmth) with a live preview, one-tap filters, and markup (pen, highlighter,
// arrow, text). Undo and redo for everything. Saving makes a copy next to the original unless you choose to
// overwrite it (and confirm). All drawing is a canvas, so it's the same on WebKitGTK and WebView2.
//
// How a picture is made: the original, turned upright (EXIF) and by the editor's turn/mirror ("orientation"),
// straightened (turned a little and zoomed in so no empty corners show) is the "geo" picture; the crop is a part of
// it (0..1 of its width and height); light and colour, then markup (kept in geo's 0..1 space too) go on top; the
// size is changed last.
'use strict';

(function () {
const H = window.HP;
const { $, $$ } = { $: (s, el = document) => el.querySelector(s), $$: (s, el = document) => [...el.querySelectorAll(s)] };
const esc = H.esc;

const RATIOS = [['free', 'Free'], ['orig', 'Original'], ['1:1', 'Square'], ['4:3', '4:3'], ['16:9', '16:9']];
const FILTERS = {
  none: { label: 'Original', a: { b: 0, c: 0, s: 0, w: 0 } },
  bw: { label: 'Black & white', a: { b: 0, c: 12, s: -100, w: 0 } },
  warm: { label: 'Warm', a: { b: 4, c: 4, s: 12, w: 40 } },
  cool: { label: 'Cool', a: { b: 0, c: 4, s: -6, w: -40 } },
  vivid: { label: 'Vivid', a: { b: 2, c: 22, s: 45, w: 0 } },
  fade: { label: 'Faded', a: { b: 12, c: -30, s: -25, w: 6 } },
};
const SLIDERS = [['b', 'Brightness'], ['c', 'Contrast'], ['s', 'Saturation'], ['w', 'Warmth']];
const COLORS = ['#ff8a3d', '#ff4d4d', '#ffd166', '#6fd08c', '#4aa3ff', '#ffffff', '#111111'];
const TOOLS = [['pen', 'Pen'], ['hl', 'Highlighter'], ['arrow', 'Arrow'], ['text', 'Text']];

let E = null; // the open editor

const fresh = o => ({ o, angle: 0, crop: null, ratio: 'free', portrait: false, adj: { b: 0, c: 0, s: 0, w: 0 }, filter: 'none', marks: [], resize: null });
const clone = v => JSON.parse(JSON.stringify(v));

// ---------------------------------------------------------------- drawing
/** Draws `img` (natural size nw×nh) centred on the origin, turned/mirrored by EXIF orientation o. */
function drawOriented(g, img, o, nw, nh) {
  const c = H.cssOf(o);
  g.rotate(c.deg * Math.PI / 180);
  if (c.flip) g.scale(-1, 1);
  g.drawImage(img, -nw / 2, -nh / 2, nw, nh);
}
/** How much straightening by `deg` has to zoom in so no empty corners show. */
function fillScale(deg, w, h) {
  const t = Math.abs(deg) * Math.PI / 180, c = Math.cos(t), s = Math.sin(t);
  return 1 / Math.min(w / (w * c + h * s), h / (w * s + h * c));
}
/** The geo picture's size in the source's pixels. */
function geoSize(src, st) {
  const o = H.then(src.base, st.o);
  return H.cssOf(o).swaps ? { w: src.h, h: src.w, o } : { w: src.w, h: src.h, o };
}
/** The crop in pixels at `scale` (all of it without a crop). */
function cropPx(st, gw, gh) {
  const c = st.crop || { x: 0, y: 0, w: 1, h: 1 };
  return { x: c.x * gw, y: c.y * gh, w: Math.max(1, c.w * gw), h: Math.max(1, c.h * gh) };
}

/**
 * Makes the picture: img is the source (or a smaller copy of it, `k` = its size / the source's), scale is pixels
 * out per source pixel, crop false shows all of geo (the crop tab). Returns a canvas.
 */
function render(src, st, { img, k = 1, scale = 1, crop = true, marks = true, adjust = true, into = null }) {
  const geo = geoSize(src, st);
  const gw = geo.w * scale, gh = geo.h * scale;
  const cp = crop ? cropPx(st, gw, gh) : { x: 0, y: 0, w: gw, h: gh };
  const cv = into || document.createElement('canvas');
  cv.width = Math.max(1, Math.round(cp.w)); cv.height = Math.max(1, Math.round(cp.h));
  const g = cv.getContext('2d', { willReadFrequently: false });
  g.save();
  g.imageSmoothingEnabled = true; g.imageSmoothingQuality = 'high';
  g.translate(-cp.x + gw / 2, -cp.y + gh / 2);
  g.rotate(st.angle * Math.PI / 180);
  const z = fillScale(st.angle, gw, gh);
  g.scale(z * scale / k, z * scale / k);
  drawOriented(g, img, geo.o, src.w * k, src.h * k);
  g.restore();
  if (adjust) adjustCanvas(cv, st.adj);
  if (marks && st.marks.length) drawMarks(g, st.marks, gw, gh, cp);
  return cv;
}

// Light and colour: a lookup table per channel for brightness, contrast and warmth, then saturation per pixel.
function adjustCanvas(cv, a) {
  if (!a.b && !a.c && !a.s && !a.w) return;
  const g = cv.getContext('2d');
  // Big pictures in bands, so the page doesn't hold two copies of a huge picture at once.
  const band = Math.max(1, Math.floor(4e6 / cv.width));
  const C = a.c * 1.28, f = (259 * (C + 255)) / (255 * (259 - C));
  const luts = [0.6, 0.1, -0.6].map(wk => { const l = new Uint8ClampedArray(256); for (let v = 0; v < 256; v++) l[v] = f * (v + a.b * 1.1 + a.w * wk - 128) + 128; return l; });
  const [R, G, B] = luts, sat = 1 + a.s / 100;
  for (let y = 0; y < cv.height; y += band) {
    const h = Math.min(band, cv.height - y);
    const id = g.getImageData(0, y, cv.width, h), d = id.data;
    for (let i = 0; i < d.length; i += 4) {
      let r = R[d[i]], gg = G[d[i + 1]], b = B[d[i + 2]];
      if (sat !== 1) { const l = 0.2126 * r + 0.7152 * gg + 0.0722 * b; r = l + (r - l) * sat; gg = l + (gg - l) * sat; b = l + (b - l) * sat; }
      d[i] = r; d[i + 1] = gg; d[i + 2] = b;
    }
    g.putImageData(id, 0, y);
  }
}

function drawMarks(g, marks, gw, gh, cp) {
  const diag = Math.hypot(gw, gh);
  const P = ([x, y]) => [x * gw - cp.x, y * gh - cp.y];
  g.save();
  g.lineCap = 'round'; g.lineJoin = 'round';
  for (const m of marks) {
    g.strokeStyle = g.fillStyle = m.color;
    g.globalAlpha = m.t === 'hl' ? 0.38 : 1;
    g.lineWidth = m.w * diag * (m.t === 'hl' ? 3 : 1);
    if (m.t === 'pen' || m.t === 'hl') {
      g.beginPath();
      m.pts.forEach((p, i) => { const [x, y] = P(p); i ? g.lineTo(x, y) : g.moveTo(x, y); });
      if (m.pts.length === 1) { const [x, y] = P(m.pts[0]); g.lineTo(x + 0.1, y); }
      g.stroke();
    } else if (m.t === 'arrow') {
      const [x1, y1] = P(m.a), [x2, y2] = P(m.b), ang = Math.atan2(y2 - y1, x2 - x1), head = Math.max(g.lineWidth * 3.5, 12);
      g.beginPath(); g.moveTo(x1, y1); g.lineTo(x2 - Math.cos(ang) * head * 0.6, y2 - Math.sin(ang) * head * 0.6); g.stroke();
      g.beginPath(); g.moveTo(x2, y2);
      g.lineTo(x2 - head * Math.cos(ang - 0.45), y2 - head * Math.sin(ang - 0.45));
      g.lineTo(x2 - head * Math.cos(ang + 0.45), y2 - head * Math.sin(ang + 0.45));
      g.closePath(); g.fill();
    } else if (m.t === 'text') {
      const [x, y] = P([m.x, m.y]), size = m.size * diag;
      g.font = `600 ${size}px "Segoe UI", Inter, "Noto Sans", Cantarell, sans-serif`;
      g.textBaseline = 'middle';
      g.lineWidth = Math.max(2, size / 9);
      g.strokeStyle = m.color === '#111111' ? 'rgba(255,255,255,.85)' : 'rgba(0,0,0,.65)';
      g.strokeText(m.text, x, y);
      g.fillText(m.text, x, y);
    }
  }
  g.restore();
}

/** Marks moved along when the picture is turned or mirrored by `op` (geo space changes shape). */
function turnMarks(marks, op, gw, gh) {
  const M = H.MATS[op];
  const swaps = op >= 5;
  const nw = swaps ? gh : gw, nh = swaps ? gw : gh;
  const T = ([x, y]) => { const px = x * gw - gw / 2, py = y * gh - gh / 2; return [(M[0] * px + M[1] * py + nw / 2) / nw, (M[2] * px + M[3] * py + nh / 2) / nh]; };
  return marks.map(m => m.pts ? { ...m, pts: m.pts.map(T) } : m.t === 'arrow' ? { ...m, a: T(m.a), b: T(m.b) } : (([x, y]) => ({ ...m, x, y }))(T([m.x, m.y])));
}

// ---------------------------------------------------------------- opening
async function open(pic, viewTurn) {
  if (E) return;
  const ed = $('#editor');
  ed.innerHTML = `<div class="ebar"><h2>Edit</h2><span class="hint" id="eHint">${esc(pic.name)}</span><span class="sp" style="flex:1"></span>
      <button class="ib" data-f data-e="undo" title="Undo (Ctrl+Z, X)"><svg viewBox="0 0 24 24"><path d="M9 14 4 9l5-5"/><path d="M4 9h10a6 6 0 0 1 0 12h-3"/></svg></button>
      <button class="ib" data-f data-e="redo" title="Redo (Ctrl+Y)"><svg viewBox="0 0 24 24"><path d="m15 14 5-5-5-5"/><path d="M20 9H10a6 6 0 0 0 0 12h3"/></svg></button>
      <button class="btn" data-f data-e="cancel">Cancel</button>
      <button class="btn primary" data-f data-e="save" title="Save (Ctrl+S, Start)">Save…</button></div>
    <div id="ecanvasWrap" tabindex="0" data-f><canvas id="ecanvas"></canvas><div id="ecrop" hidden></div><div id="ecursor" hidden></div><div id="eload" style="position:absolute;inset:0;display:grid;place-items:center;color:var(--muted)">Opening…</div></div>
    <div id="epanel"><div class="etabs">${[['crop', 'Crop'], ['adjust', 'Light'], ['filters', 'Filters'], ['markup', 'Markup'], ['resize', 'Size']].map(([id, l]) => `<button class="etab" data-f data-tab="${id}">${l}</button>`).join('')}</div>
      <div class="ebody" id="ebody"></div>
      <div class="efoot"><span class="note" id="eOut"></span></div></div>`;
  ed.hidden = false;
  E = { pic, st: fresh(viewTurn || 1), undo: [], redo: [], tab: 'crop', tool: 'pen', color: COLORS[0], size: 0.006, src: null, dirty: false, cursor: null, drawing: null };
  ed.onclick = onClick;
  ed.oninput = onInput;
  ed.onchange = onChange;
  try {
    const [buf, info] = await Promise.all([H.invoke('read_bytes', { path: pic.path }), H.invoke('file_info', { path: pic.path }).catch(() => null)]);
    // SVG needs its type to be drawn; the others are recognised by their first bytes.
    const bytes = buf instanceof ArrayBuffer ? new Uint8Array(buf) : new Uint8Array(buf);
    const head = new TextDecoder().decode(bytes.subarray(0, 256)).trimStart();
    const isSvg = H.extOf(pic.path) === 'svg' || head.startsWith('<svg') || (head.startsWith('<?xml') && head.includes('<svg'));
    const blob = new Blob([bytes], isSvg ? { type: 'image/svg+xml' } : {});
    const u = URL.createObjectURL(blob);
    const img = new Image();
    img.src = u;
    await img.decode();
    let w = img.naturalWidth, h = img.naturalHeight;
    if (H.extOf(pic.path) === 'svg' && (!w || !h)) { w = 2048; h = 2048; }
    // Web views turn JPEGs upright by their EXIF orientation. If this one didn't (its size isn't swapped for a
    // sideways picture), the editor turns it.
    let base = 1;
    if (info && info.orientation >= 5 && info.width && info.height && info.width !== info.height && w === info.width && h === info.height) base = info.orientation;
    E.src = { img, url: u, w, h, base };
    // A smaller copy for the live preview (the full picture is only used when saving).
    const long = Math.max(w, h), k = Math.min(1, 2400 / long);
    const proxy = document.createElement('canvas');
    proxy.width = Math.max(1, Math.round(w * k)); proxy.height = Math.max(1, Math.round(h * k));
    proxy.getContext('2d').drawImage(img, 0, 0, proxy.width, proxy.height);
    E.proxy = { img: proxy, k };
    E.canWebp = (() => { try { const c = document.createElement('canvas'); c.width = c.height = 2; return c.toDataURL('image/webp').startsWith('data:image/webp'); } catch (_) { return false; } })();
    $('#eload').remove();
  } catch (e) {
    H.toast('The editor couldn\'t open this picture: ' + e);
    close();
    return;
  }
  E.last = clone(E.st);
  setTab('crop');
  setupPointer();
  window.addEventListener('resize', draw);
  $('[data-tab="crop"]').focus();
}

function close() {
  if (!E) return;
  if (E.src) URL.revokeObjectURL(E.src.url);
  window.removeEventListener('resize', draw);
  E = null;
  const ed = $('#editor');
  ed.hidden = true;
  ed.innerHTML = '';
  H.afterEditorClose();
}

async function back() {
  if (!E) return;
  if ($('#etext')) { $('#etext').remove(); return; }
  if (!E.dirty) { close(); return; }
  const v = await H.dialog('<h2>Leave without saving?</h2><p>Your changes to this picture will be lost.</p>', [{ label: 'Keep editing', value: 0, focus: true }, { label: 'Discard changes', kind: 'danger', value: 1 }]);
  if (v === 1) close();
}

// ---------------------------------------------------------------- history
function commit() {
  E.undo.push(clone(E.last || fresh(1)));
  if (E.undo.length > 100) E.undo.shift();
  E.redo = [];
  E.last = clone(E.st);
  E.dirty = true;
  updateBar();
}
function undo() { if (!E.undo.length) return; E.redo.push(clone(E.st)); E.st = E.undo.pop(); E.last = clone(E.st); E.dirty = true; panel(); draw(); updateBar(); }
function redo() { if (!E.redo.length) return; E.undo.push(clone(E.st)); E.st = E.redo.pop(); E.last = clone(E.st); E.dirty = true; panel(); draw(); updateBar(); }
function updateBar() {
  $('[data-e="undo"]').disabled = !E.undo.length;
  $('[data-e="redo"]').disabled = !E.redo.length;
  const o = outSize();
  $('#eOut').textContent = `Saves at ${o.w} × ${o.h}`;
}

/** The size the saved picture will have. */
function outSize() {
  const geo = geoSize(E.src, E.st), cp = cropPx(E.st, geo.w, geo.h);
  const w = Math.round(cp.w), h = Math.round(cp.h);
  const r = E.st.resize;
  if (!r) return { w, h };
  // Keeping the shape: the height follows the width (also after the crop changes).
  return r.keep ? { w: r.w, h: Math.max(1, Math.round(r.w * cp.h / cp.w)) } : { w: r.w, h: r.h };
}

// ---------------------------------------------------------------- the preview
function previewScale(cropOnly) {
  const wrap = $('#ecanvasWrap').getBoundingClientRect();
  const geo = geoSize(E.src, E.st);
  const cp = cropOnly ? cropPx(E.st, geo.w, geo.h) : { w: geo.w, h: geo.h };
  const dpr = window.devicePixelRatio || 1;
  const fit = Math.min((wrap.width - 48) / cp.w, (wrap.height - 48) / cp.h);
  return { css: fit, px: Math.min(fit * dpr, E.proxy.k) };
}
let drawQueued = false;
function draw() {
  if (!E || !E.src || drawQueued) return;
  drawQueued = true;
  requestAnimationFrame(() => {
    drawQueued = false;
    if (!E) return;
    const cropTab = E.tab === 'crop';
    const sc = previewScale(!cropTab);
    const cv = $('#ecanvas');
    render(E.src, E.st, { img: E.proxy.img, k: E.proxy.k, scale: sc.px, crop: !cropTab, into: cv });
    cv.style.width = (cv.width * sc.css / sc.px) + 'px';
    cv.style.height = (cv.height * sc.css / sc.px) + 'px';
    E.view = { css: sc.css, px: sc.px, crop: !cropTab };
    drawCrop();
  });
}

/** Where on screen a geo point (0..1) is, and back. */
function geoToScreen(x, y) {
  const cv = $('#ecanvas').getBoundingClientRect(), wrap = $('#ecanvasWrap').getBoundingClientRect();
  const geo = geoSize(E.src, E.st);
  if (!E.view.crop) return [cv.left - wrap.left + x * cv.width, cv.top - wrap.top + y * cv.height];
  const c = E.st.crop || { x: 0, y: 0, w: 1, h: 1 };
  void geo;
  return [cv.left - wrap.left + (x - c.x) / c.w * cv.width, cv.top - wrap.top + (y - c.y) / c.h * cv.height];
}
function screenToGeo(sx, sy) {
  const cv = $('#ecanvas').getBoundingClientRect();
  let x = (sx - cv.left) / cv.width, y = (sy - cv.top) / cv.height;
  if (E.view.crop) { const c = E.st.crop || { x: 0, y: 0, w: 1, h: 1 }; x = c.x + x * c.w; y = c.y + y * c.h; }
  return [x, y];
}

// ---------------------------------------------------------------- crop
function ratioValue() {
  const geo = geoSize(E.src, E.st);
  const r = E.st.ratio;
  let v = r === 'orig' ? geo.w / geo.h : r === 'free' ? 0 : (([a, b]) => a / b)(r.split(':').map(Number));
  if (v && E.st.portrait && r !== 'orig') v = 1 / v;
  return v; // width / height in pixels, 0 for free
}
/** The biggest crop of the chosen shape, centred on the current one. */
function applyRatio() {
  const v = ratioValue();
  if (!v) return;
  const geo = geoSize(E.src, E.st);
  const c = E.st.crop || { x: 0, y: 0, w: 1, h: 1 };
  const cx = c.x + c.w / 2, cy = c.y + c.h / 2;
  // In 0..1 units, a crop of shape v has w/h = v * gh / gw.
  const q = v * geo.h / geo.w;
  let w = 1, h = w / q;
  if (h > 1) { h = 1; w = q; }
  const x = Math.min(1 - w, Math.max(0, cx - w / 2)), y = Math.min(1 - h, Math.max(0, cy - h / 2));
  E.st.crop = { x, y, w, h };
}
function clampCrop(c) {
  const min = 0.03;
  c.w = Math.max(min, Math.min(1, c.w)); c.h = Math.max(min, Math.min(1, c.h));
  c.x = Math.max(0, Math.min(1 - c.w, c.x)); c.y = Math.max(0, Math.min(1 - c.h, c.y));
  return c;
}
/** Grows or shrinks the crop by (dw, dh) around its centre (keeping its shape when one is chosen). */
function resizeCrop(dw, dh) {
  const c = clone(E.st.crop || { x: 0, y: 0, w: 1, h: 1 });
  const v = ratioValue();
  const geo = geoSize(E.src, E.st);
  let w = c.w + dw, h = c.h + dh;
  if (v) { const q = v * geo.h / geo.w; if (dw) h = w / q; else w = h * q; if (w > 1) { w = 1; h = w / q; } if (h > 1) { h = 1; w = h * q; } }
  const cx = c.x + c.w / 2, cy = c.y + c.h / 2;
  E.st.crop = clampCrop({ x: cx - w / 2, y: cy - h / 2, w, h });
}
function drawCrop() {
  const box = $('#ecrop');
  if (!E || E.tab !== 'crop') { box.hidden = true; return; }
  box.hidden = false;
  const c = E.st.crop || { x: 0, y: 0, w: 1, h: 1 };
  const [x1, y1] = geoToScreen(c.x, c.y), [x2, y2] = geoToScreen(c.x + c.w, c.y + c.h);
  box.innerHTML = `<div class="box" style="left:${x1}px;top:${y1}px;width:${x2 - x1}px;height:${y2 - y1}px" data-drag="move"></div>` +
    [['nw', x1, y1], ['ne', x2, y1], ['sw', x1, y2], ['se', x2, y2]].map(([k, x, y]) => `<div class="h" data-drag="${k}" style="left:${x}px;top:${y}px"></div>`).join('');
}

// ---------------------------------------------------------------- pointer: crop handles, markup
function setupPointer() {
  const wrap = $('#ecanvasWrap');
  let drag = null;
  wrap.addEventListener('pointerdown', e => {
    if (!E || !E.src) return;
    if (E.tab === 'crop') {
      const t = e.target.closest('[data-drag]');
      if (!t) return;
      wrap.setPointerCapture(e.pointerId);
      drag = { kind: t.dataset.drag, start: screenToGeo(e.clientX, e.clientY), crop: clone(E.st.crop || { x: 0, y: 0, w: 1, h: 1 }) };
      e.preventDefault();
    } else if (E.tab === 'markup') {
      wrap.setPointerCapture(e.pointerId);
      startMark(screenToGeo(e.clientX, e.clientY), e.clientX, e.clientY);
      e.preventDefault();
    }
  });
  wrap.addEventListener('pointermove', e => {
    if (!E) return;
    if (drag) {
      const [gx, gy] = screenToGeo(e.clientX, e.clientY), dx = gx - drag.start[0], dy = gy - drag.start[1];
      const c = clone(drag.crop);
      if (drag.kind === 'move') { c.x += dx; c.y += dy; E.st.crop = clampCrop(c); }
      else {
        let { x, y, w, h } = c;
        let x2 = x + w, y2 = y + h;
        if (drag.kind.includes('w')) x = Math.min(x2 - 0.03, Math.max(0, x + dx)); else x2 = Math.max(x + 0.03, Math.min(1, x2 + dx));
        if (drag.kind.includes('n')) y = Math.min(y2 - 0.03, Math.max(0, y + dy)); else y2 = Math.max(y + 0.03, Math.min(1, y2 + dy));
        w = x2 - x; h = y2 - y;
        const v = ratioValue();
        if (v) {
          const geo = geoSize(E.src, E.st), q = v * geo.h / geo.w;
          if (w / h > q) w = h * q; else h = w / q;
          if (drag.kind.includes('w')) x = x2 - w;
          if (drag.kind.includes('n')) y = y2 - h;
        }
        E.st.crop = clampCrop({ x, y, w, h });
      }
      drawCrop(); updateBar();
    } else if (E.drawing) moveMark(screenToGeo(e.clientX, e.clientY));
  });
  const up = () => { if (drag) { drag = null; commit(); } if (E && E.drawing) endMark(); };
  wrap.addEventListener('pointerup', up);
  wrap.addEventListener('pointercancel', up);
}

function startMark(p, sx, sy) {
  if (E.tool === 'text') { askText(p, sx, sy); return; }
  const m = E.tool === 'arrow' ? { t: 'arrow', color: E.color, w: E.size, a: p, b: p } : { t: E.tool, color: E.color, w: E.size, pts: [p] };
  E.st.marks.push(m);
  E.drawing = m;
  draw();
}
function moveMark(p) {
  const m = E.drawing;
  if (m.t === 'arrow') m.b = p;
  else { const l = m.pts[m.pts.length - 1]; if (Math.hypot(p[0] - l[0], p[1] - l[1]) > 0.002) m.pts.push(p); }
  draw();
}
function endMark() {
  const m = E.drawing;
  E.drawing = null;
  if (m.t === 'arrow' && Math.hypot(m.b[0] - m.a[0], m.b[1] - m.a[1]) < 0.01) { E.st.marks.pop(); draw(); return; }
  commit();
}
async function askText(p) {
  E._lastText = '';
  const val = await H.dialog(`<h2>Add text</h2><div class="field"><input class="inp" id="etextIn" maxlength="200" autofocus placeholder="Type here"></div>`,
    [{ label: 'Cancel', value: null }, { label: 'Add', kind: 'primary', value: 'ok' }],
    (v, done) => { const i = $('#etextIn', v); i.focus(); i.addEventListener('keydown', e => { e.stopPropagation(); if (e.key === 'Enter') done('ok'); if (e.key === 'Escape') done(null); }); });
  if (!E || val !== 'ok') return;
  const t = E._lastText;
  if (!t) return;
  E.st.marks.push({ t: 'text', color: E.color, size: E.size * 5, x: p[0], y: p[1], text: t });
  commit(); draw();
}
// The dialog's input is gone when it resolves: remember what was typed as it changes.
document.addEventListener('input', e => { if (e.target.id === 'etextIn' && E) E._lastText = e.target.value.trim(); });

// ---------------------------------------------------------------- the side panel
function setTab(t) {
  E.tab = t;
  $$('.etab').forEach(b => b.classList.toggle('on', b.dataset.tab === t));
  $('#ecursor').hidden = t !== 'markup' || !E.cursor;
  $('#ecanvasWrap').style.cursor = t === 'markup' ? 'crosshair' : '';
  panel();
  draw();
}
function slider(id, label, val, min, max, step = 1, unit = '') {
  return `<div class="slider"><label for="es-${id}">${esc(label)}</label><output id="eo-${id}">${val > 0 && min < 0 ? '+' : ''}${val}${unit}</output><input type="range" data-f id="es-${id}" data-s="${id}" min="${min}" max="${max}" step="${step}" value="${val}"></div>`;
}
function panel() {
  const st = E.st, b = $('#ebody');
  if (E.tab === 'crop') {
    b.innerHTML = `<h3>Shape</h3><div class="chips">${RATIOS.map(([id, l]) => `<button class="chip${st.ratio === id ? ' on' : ''}" data-f data-ratio="${id}">${l}</button>`).join('')}
        <button class="chip${st.portrait ? ' on' : ''}" data-f data-e="portrait" title="Tall instead of wide">Tall</button></div>
      <h3>Turn</h3><div class="chips">
        <button class="chip" data-f data-e="left">Turn left</button><button class="chip" data-f data-e="right">Turn right</button>
        <button class="chip" data-f data-e="flipH">Mirror</button><button class="chip" data-f data-e="flipV">Flip</button></div>
      <h3>Straighten</h3>${slider('angle', 'Angle', st.angle, -45, 45, 0.5, '°')}
      <div class="chips"><button class="chip" data-f data-e="resetCrop">Reset crop and turn</button></div>
      <p class="note">Drag the corners or the middle. Keys: arrows move it, Shift+arrows change its size, [ and ] straighten. Controller: left stick moves, right stick sizes, LT/RT straighten.</p>`;
  } else if (E.tab === 'adjust') {
    b.innerHTML = `<h3>Light and colour</h3>${SLIDERS.map(([id, l]) => slider(id, l, st.adj[id], -100, 100)).join('')}
      <div class="chips"><button class="chip" data-f data-e="resetAdj">Reset</button></div>`;
  } else if (E.tab === 'filters') {
    b.innerHTML = `<div class="filters">${Object.entries(FILTERS).map(([id, f]) => `<button class="filter${st.filter === id ? ' on' : ''}" data-f data-filter="${id}"><canvas data-fc="${id}"></canvas><span>${esc(f.label)}</span></button>`).join('')}</div>
      <p class="note">A filter sets Light and colour; change them there afterwards.</p>`;
    requestAnimationFrame(filterThumbs);
  } else if (E.tab === 'markup') {
    b.innerHTML = `<h3>Tool</h3><div class="chips">${TOOLS.map(([id, l]) => `<button class="chip${E.tool === id ? ' on' : ''}" data-f data-tool="${id}">${l}</button>`).join('')}</div>
      <h3>Colour</h3><div class="colors">${COLORS.map(c => `<button class="swatch${E.color === c ? ' on' : ''}" data-f data-color="${c}" style="background:${c}" title="${c}"></button>`).join('')}</div>
      <h3>Size</h3>${slider('size', 'Thickness', Math.round(E.size * 1000), 2, 30)}
      <div class="chips"><button class="chip" data-f data-e="clearMarks" ${st.marks.length ? '' : 'disabled'}>Remove all markup</button></div>
      <p class="note">Draw on the picture with the mouse or a finger. Controller: the right stick moves the pointer, hold RT to draw (RT places text).</p>`;
  } else if (E.tab === 'resize') {
    const geo = geoSize(E.src, st), cp = cropPx(st, geo.w, geo.h), o = outSize();
    const pct = Math.round(o.w / cp.w * 100);
    b.innerHTML = `<h3>Size</h3><p class="note">The cropped picture is ${Math.round(cp.w)} × ${Math.round(cp.h)}.</p>
      <div class="chips">${[100, 75, 50, 25].map(p => `<button class="chip${pct === p ? ' on' : ''}" data-f data-pct="${p}">${p}%</button>`).join('')}</div>
      <h3>Pixels</h3><div style="display:grid;grid-template-columns:1fr auto 1fr;gap:8px;align-items:center">
        <input class="inp" data-f id="erw" type="number" min="1" max="30000" value="${o.w}"><span>×</span><input class="inp" data-f id="erh" type="number" min="1" max="30000" value="${o.h}"></div>
      <label class="note" style="display:flex;gap:8px;align-items:center;margin-top:12px"><input type="checkbox" id="erk" ${E.keepRatio !== false ? 'checked' : ''} style="width:22px;height:22px;accent-color:var(--ember)"> Keep the shape</label>
      ${o.w > cp.w ? '<p class="note">Making a picture bigger than it is doesn\'t add detail.</p>' : ''}`;
  }
  updateBar();
}
function filterThumbs() {
  if (!E || E.tab !== 'filters') return;
  const st = E.st;
  const geo = geoSize(E.src, st), cp = cropPx(st, geo.w, geo.h);
  const scale = Math.min(140 / cp.w, 140 / cp.h);
  const base = render(E.src, { ...st, adj: { b: 0, c: 0, s: 0, w: 0 }, marks: [] }, { img: E.proxy.img, k: E.proxy.k, scale, adjust: false, marks: false });
  for (const c of $$('[data-fc]')) {
    const f = FILTERS[c.dataset.fc];
    c.width = base.width; c.height = base.height;
    c.getContext('2d').drawImage(base, 0, 0);
    adjustCanvas(c, f.a);
  }
}

function setOutput(w, h) {
  const geo = geoSize(E.src, E.st), cp = cropPx(E.st, geo.w, geo.h);
  w = Math.max(1, Math.min(30000, Math.round(w))); h = Math.max(1, Math.min(30000, Math.round(h)));
  E.st.resize = (Math.abs(w - Math.round(cp.w)) <= 1 && Math.abs(h - Math.round(cp.h)) <= 1) ? null : { w, h, keep: E.keepRatio !== false };
}

function onClick(e) {
  const t = e.target.closest('button');
  if (!t || !E || t.disabled) return;
  const st = E.st, d = t.dataset;
  if (d.tab) { setTab(d.tab); return; }
  if (d.ratio) { st.ratio = d.ratio; if (d.ratio === 'free') { /* keep the crop */ } else applyRatio(); commit(); panel(); draw(); return; }
  if (d.filter) { st.filter = d.filter; st.adj = clone(FILTERS[d.filter].a); commit(); panel(); draw(); return; }
  if (d.tool) { E.tool = d.tool; panel(); return; }
  if (d.color) { E.color = d.color; panel(); return; }
  if (d.pct) { const geo = geoSize(E.src, st), cp = cropPx(st, geo.w, geo.h); setOutput(cp.w * d.pct / 100, cp.h * d.pct / 100); commit(); panel(); return; }
  const turnBy = op => {
    const geo = geoSize(E.src, st);
    st.marks = turnMarks(st.marks, op, geo.w, geo.h);
    st.o = H.then(st.o, op); st.crop = null; st.resize = null;
    if (st.ratio !== 'free') applyRatio();
    commit(); panel(); draw();
  };
  switch (d.e) {
    case 'undo': undo(); break;
    case 'redo': redo(); break;
    case 'cancel': back(); break;
    case 'save': saveDialog(); break;
    case 'portrait': st.portrait = !st.portrait; applyRatio(); commit(); panel(); draw(); break;
    case 'left': turnBy(8); break;
    case 'right': turnBy(6); break;
    case 'flipH': turnBy(2); break;
    case 'flipV': turnBy(4); break;
    case 'resetCrop': st.crop = null; st.angle = 0; st.ratio = 'free'; st.portrait = false; st.resize = null; commit(); panel(); draw(); break;
    case 'resetAdj': st.adj = { b: 0, c: 0, s: 0, w: 0 }; st.filter = 'none'; commit(); panel(); draw(); break;
    case 'clearMarks': st.marks = []; commit(); panel(); draw(); break;
  }
}
function onInput(e) {
  if (!E) return;
  const t = e.target, id = t.dataset.s;
  if (id) {
    const v = +t.value;
    if (id === 'angle') E.st.angle = v;
    else if (id === 'size') E.size = v / 1000;
    else { E.st.adj[id] = v; E.st.filter = 'custom'; }
    const o = $('#eo-' + id);
    if (o) o.textContent = (v > 0 && t.min < 0 ? '+' : '') + v + (id === 'angle' ? '°' : '');
    if (id !== 'size') draw();
    return;
  }
  if (t.id === 'erw' || t.id === 'erh') {
    const geo = geoSize(E.src, E.st), cp = cropPx(E.st, geo.w, geo.h), keep = $('#erk').checked;
    let w = +$('#erw').value || 1, h = +$('#erh').value || 1;
    if (keep) { if (t.id === 'erw') { h = Math.round(w * cp.h / cp.w); $('#erh').value = h; } else { w = Math.round(h * cp.w / cp.h); $('#erw').value = w; } }
    setOutput(w, h); updateBar();
  }
}
function onChange(e) {
  if (!E) return;
  const t = e.target;
  if (t.dataset.s && t.dataset.s !== 'size') commit();
  if (t.id === 'erw' || t.id === 'erh') { commit(); }
  if (t.id === 'erk') E.keepRatio = t.checked;
}

// ---------------------------------------------------------------- saving
async function saveDialog() {
  if (!E || !E.src) return;
  const ext = H.extOf(E.pic.path);
  const origFmt = /^(jpe?g|jpe|jfif)$/.test(ext) ? 'jpeg' : ext === 'png' ? 'png' : ext === 'webp' ? 'webp' : null;
  const opts = { mode: 'copy', fmt: origFmt || 'jpeg', q: H.store.get('quality', 90) };
  const o = outSize();
  const html = () => `<h2>Save the edited picture</h2><p>${o.w} × ${o.h} pixels</p>
    <div class="field"><label>Where</label><div class="chips">
      <button class="chip${opts.mode === 'copy' ? ' on' : ''}" data-f data-mode="copy">Save as a copy</button>
      <button class="chip${opts.mode === 'over' ? ' on' : ''}" data-f data-mode="over" ${origFmt ? '' : 'disabled title="This kind of picture can only be saved as a copy"'}>Overwrite the original</button></div></div>
    <div class="field"><label>Format</label><div class="chips">${[['jpeg', 'JPEG'], ['png', 'PNG'], ['webp', 'WebP']].map(([id, l]) => `<button class="chip${opts.fmt === id ? ' on' : ''}" data-f data-fmt="${id}" ${opts.mode === 'over' && id !== origFmt ? 'disabled' : ''}>${l}</button>`).join('')}</div></div>
    <div class="field" ${opts.fmt === 'png' || (opts.fmt === 'webp' && !E.canWebp) ? 'hidden' : ''}><label>Quality: <b id="eqv">${opts.q}</b></label><input type="range" data-f id="eq" min="40" max="100" value="${opts.q}"></div>
    ${opts.fmt === 'webp' && !E.canWebp ? '<p class="note">WebP is saved without loss here (larger files).</p>' : ''}
    ${opts.mode === 'over' ? '<p class="note" style="color:var(--flame)">The original is replaced. It can\'t be brought back.</p>' : '<p class="note">The original stays as it is.</p>'}`;
  const val = await H.dialog(`<div id="esave">${html()}</div>`, [{ label: 'Cancel', value: null }, { label: 'Save', kind: 'primary', value: 'save', focus: true }], v => {
    v.addEventListener('click', e => {
      const t = e.target.closest('button');
      if (!t || t.disabled) return;
      if (t.dataset.mode) { opts.mode = t.dataset.mode; if (opts.mode === 'over') opts.fmt = origFmt; }
      else if (t.dataset.fmt) opts.fmt = t.dataset.fmt;
      else return;
      const keep = t.dataset.mode ? `[data-mode="${t.dataset.mode}"]` : `[data-fmt="${t.dataset.fmt}"]`;
      $('#esave', v).innerHTML = html();
      const again = $(keep, v); if (again) again.focus();
    });
    v.addEventListener('input', e => { if (e.target.id === 'eq') { opts.q = +e.target.value; $('#eqv', v).textContent = opts.q; } });
  });
  if (val !== 'save') return;
  H.store.set('quality', opts.q);
  if (opts.mode === 'over') {
    const sure = await H.dialog(`<h2>Replace ${esc(E.pic.name)}?</h2><p>The original picture is replaced by the edited one and can't be brought back.</p>`,
      [{ label: 'Cancel', value: 0, focus: true }, { label: 'Replace it', kind: 'danger', value: 1 }]);
    if (sure !== 1) return;
  }
  await doSave(opts);
}

async function doSave(opts) {
  H.toast('Saving…', null, 60000);
  await new Promise(r => setTimeout(r, 30));
  try {
    let cv = render(E.src, E.st, { img: E.src.img, k: 1, scale: 1 });
    if (E.st.resize) { const o = outSize(); cv = resizeCanvas(cv, o.w, o.h); }
    const webpHere = opts.fmt === 'webp' && E.canWebp;
    const type = opts.fmt === 'jpeg' ? 'image/jpeg' : webpHere ? 'image/webp' : 'image/png';
    if (type === 'image/jpeg') { // JPEG has no see-through parts: put the picture on white
      const flat = document.createElement('canvas'); flat.width = cv.width; flat.height = cv.height;
      const g = flat.getContext('2d'); g.fillStyle = '#fff'; g.fillRect(0, 0, flat.width, flat.height); g.drawImage(cv, 0, 0); cv = flat;
    }
    const blob = await new Promise((res, rej) => cv.toBlob(b => b ? res(b) : rej('The picture is too big to save here'), type, opts.q / 100));
    const bytes = new Uint8Array(await blob.arrayBuffer());
    const encoded = type !== 'image/png' || opts.fmt === 'png';
    const headers = { 'x-original': encodeURIComponent(E.pic.path), 'x-format': opts.fmt, 'x-quality': String(opts.q), 'x-encoded': encoded ? '1' : '0', 'x-overwrite': opts.mode === 'over' ? '1' : '0' };
    const r = await (window.__TAURI__ ? window.__TAURI__.core.invoke('save_edit', bytes, { headers }) : H.invoke('save_edit', {}));
    E.dirty = false;
    H.toast(r.message);
    close();
    await H.openPath(r.path);
  } catch (e) { H.toast('Couldn\'t save: ' + e); }
}

/** Smaller in halving steps (sharper than one big step); bigger in one step. */
function resizeCanvas(cv, w, h) {
  let cur = cv;
  while (cur.width / 2 >= w && cur.height / 2 >= h) {
    const n = document.createElement('canvas'); n.width = Math.round(cur.width / 2); n.height = Math.round(cur.height / 2);
    const g = n.getContext('2d'); g.imageSmoothingQuality = 'high'; g.drawImage(cur, 0, 0, n.width, n.height); cur = n;
  }
  const out = document.createElement('canvas'); out.width = w; out.height = h;
  const g = out.getContext('2d'); g.imageSmoothingQuality = 'high'; g.drawImage(cur, 0, 0, w, h);
  return out;
}

// ---------------------------------------------------------------- keyboard and controller
function key(e) {
  if (!E) return false;
  if ($('#layer .veil')) return false; // dialogs handle their own keys (Esc through app.js)
  const ctrl = e.ctrlKey || e.metaKey, k = e.key;
  if (ctrl && k.toLowerCase() === 'z' && !e.shiftKey) { e.preventDefault(); undo(); return true; }
  if (ctrl && (k.toLowerCase() === 'y' || (k.toLowerCase() === 'z' && e.shiftKey))) { e.preventDefault(); redo(); return true; }
  if (ctrl && k.toLowerCase() === 's') { e.preventDefault(); saveDialog(); return true; }
  if (k === 'Escape') { e.preventDefault(); back(); return true; }
  const onCanvas = document.activeElement === $('#ecanvasWrap');
  if (E.tab === 'crop' && onCanvas && k.startsWith('Arrow')) {
    e.preventDefault();
    const s = 0.01 * (e.altKey ? 0.2 : 1);
    const dx = k === 'ArrowLeft' ? -s : k === 'ArrowRight' ? s : 0, dy = k === 'ArrowUp' ? -s : k === 'ArrowDown' ? s : 0;
    if (e.shiftKey) resizeCrop(dx * 2, dy * 2); else { const c = clone(E.st.crop || { x: 0, y: 0, w: 1, h: 1 }); c.x += dx; c.y += dy; E.st.crop = clampCrop(c); }
    drawCrop(); updateBar(); clearTimeout(E.kt); E.kt = setTimeout(commit, 500);
    return true;
  }
  if (E.tab === 'crop' && (k === '[' || k === ']')) { E.st.angle = Math.max(-45, Math.min(45, E.st.angle + (k === ']' ? 0.5 : -0.5))); panel(); draw(); clearTimeout(E.kt); E.kt = setTimeout(commit, 500); return true; }
  const typing = /^(INPUT|TEXTAREA)$/.test(e.target.tagName) && e.target.type !== 'range';
  if (typing) return false;
  const dirs = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
  if (dirs[k] && !(e.target.type === 'range' && (k === 'ArrowLeft' || k === 'ArrowRight'))) { e.preventDefault(); H.move(dirs[k], $('#editor')); return true; }
  return false;
}

function padButton(k) {
  if (!E) return false;
  if ($('#layer .veil')) return false;
  const tabs = ['crop', 'adjust', 'filters', 'markup', 'resize'];
  const a = document.activeElement;
  if (k === 'lb' || k === 'rb') { setTab(tabs[(tabs.indexOf(E.tab) + (k === 'rb' ? 1 : tabs.length - 1)) % tabs.length]); $(`[data-tab="${E.tab}"]`).focus(); return true; }
  if (k === 'x') { undo(); return true; }
  if (k === 'y') { redo(); return true; }
  if (k === 'start') { saveDialog(); return true; }
  // A range slider with focus: left/right change it.
  if (a && a.type === 'range' && (k === 'left' || k === 'right')) {
    const st = +a.step || 1; a.value = +a.value + (k === 'right' ? st : -st) * (a.max - a.min > 50 ? 5 : 1);
    a.dispatchEvent(new Event('input', { bubbles: true })); a.dispatchEvent(new Event('change', { bubbles: true }));
    return true;
  }
  const dir = { up: 'up', down: 'down', left: 'left', right: 'right' }[k];
  if (dir) { H.move(dir, $('#editor')); return true; }
  if (k === 'a') { if (a && a.click && a !== document.body) a.click(); return true; }
  return false;
}

/** Every frame while a controller is connected: sticks and triggers. */
function padFrame(ax, st) {
  if (!E || !E.src || $('#layer .veil')) return;
  if (E.tab === 'crop') {
    if (ax.lx || ax.ly) { const c = clone(E.st.crop || { x: 0, y: 0, w: 1, h: 1 }); c.x += ax.lx * 0.008; c.y += ax.ly * 0.008; E.st.crop = clampCrop(c); E.padMoved = true; }
    if (ax.rx || ax.ry) { if (Math.abs(ax.rx) > Math.abs(ax.ry)) resizeCrop(ax.rx * 0.008, 0); else resizeCrop(0, ax.ry * 0.008); E.padMoved = true; }
    const t = ax.rt - ax.lt;
    if (Math.abs(t) > 0.1) { E.st.angle = Math.max(-45, Math.min(45, Math.round((E.st.angle + t * 0.25) * 10) / 10)); E.padMoved = 'angle'; draw(); }
    if (E.padMoved) {
      drawCrop(); updateBar();
      if (E.padMoved === 'angle') { const s = $('#es-angle'); if (s) { s.value = E.st.angle; $('#eo-angle').textContent = E.st.angle + '°'; } }
      E.padMoved = false; clearTimeout(E.kt); E.kt = setTimeout(commit, 600);
    }
  } else if (E.tab === 'markup') {
    const wrap = $('#ecanvasWrap').getBoundingClientRect(), cur = $('#ecursor');
    if (!E.cursor) E.cursor = [wrap.left + wrap.width / 2, wrap.top + wrap.height / 2];
    if (ax.rx || ax.ry) {
      E.cursor[0] = Math.max(wrap.left, Math.min(wrap.right, E.cursor[0] + ax.rx * 9));
      E.cursor[1] = Math.max(wrap.top, Math.min(wrap.bottom, E.cursor[1] + ax.ry * 9));
    }
    cur.hidden = false;
    cur.style.left = (E.cursor[0] - wrap.left) + 'px'; cur.style.top = (E.cursor[1] - wrap.top) + 'px';
    const down = ax.rt > 0.5;
    const p = screenToGeo(E.cursor[0], E.cursor[1]);
    if (down && !E.drawing && !E.rtHeld) { E.rtHeld = true; startMark(p, E.cursor[0], E.cursor[1]); }
    else if (down && E.drawing) moveMark(p);
    else if (!down) { E.rtHeld = false; if (E.drawing) endMark(); }
  }
  void st;
}

window.Editor = { open, back, key, padButton, padFrame, get open_() { return !!E; } };
})();
