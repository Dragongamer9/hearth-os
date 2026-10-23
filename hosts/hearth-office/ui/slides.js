// Hearth Slides: presentations. A deck is plain data (P: slide size, theme, slides with text boxes, pictures and
// shapes, and speaker notes); every view (the editor, thumbnails, presenting, printing) draws it the same way.
// Files: Hearth's own .hslides (JSON, keeps everything), .pptx written with PptxGenJS and read by
// formats/pptx.js (best effort: text, pictures, shapes, tables as text, notes, backgrounds).
'use strict';
(() => {
const { el, esc, $, $$ } = H;
const THEMES = {
  ember: { name: 'Ember', bg: '#1c1512', bg2: '#2a1a10', title: '#ffd166', text: '#f4ece3', accent: '#ff8a3d', font: 'Inter, "Noto Sans", Carlito, sans-serif', titleFont: 'Inter, "Noto Sans", Carlito, sans-serif' },
  paper: { name: 'Paper', bg: '#ffffff', title: '#1f2a44', text: '#2b2b2b', accent: '#2f5496', font: 'Carlito, Calibri, "Liberation Sans", sans-serif', titleFont: 'Carlito, Calibri, "Liberation Sans", sans-serif' },
  ocean: { name: 'Ocean', bg: '#0b2545', bg2: '#13315c', title: '#ffffff', text: '#dbe7f5', accent: '#13c4a3', font: '"Noto Sans", "Liberation Sans", sans-serif', titleFont: '"Noto Sans", "Liberation Sans", sans-serif' },
  forest: { name: 'Forest', bg: '#f3f7f0', title: '#1f3d2b', text: '#2c3a2f', accent: '#3a7d44', font: '"Noto Serif", "Liberation Serif", Georgia, serif', titleFont: '"Noto Serif", "Liberation Serif", Georgia, serif' },
  slate: { name: 'Slate', bg: '#24292f', title: '#e6edf3', text: '#c9d1d9', accent: '#58a6ff', font: '"Noto Sans", "Liberation Sans", sans-serif', titleFont: '"Noto Sans", "Liberation Sans", sans-serif' },
  sunrise: { name: 'Sunrise', bg: '#fff4e6', bg2: '#ffe1c2', title: '#7a2e12', text: '#3d2c1e', accent: '#e2552b', font: 'Carlito, Calibri, "Liberation Sans", sans-serif', titleFont: 'Georgia, "Liberation Serif", serif' },
};
const SHAPES = { rect: 'Rectangle', round: 'Rounded rectangle', ellipse: 'Oval', triangle: 'Triangle', arrow: 'Arrow', line: 'Line', star: 'Star' };
const P0 = () => ({ w: 960, h: 540, theme: 'paper', slides: [] });
const D = {
  id: 'slides', name: 'Hearth Slides', untitled: 'Untitled presentation', tagline: 'Presentations to show on a screen or a TV',
  credits: 'PptxGenJS (MIT) and JSZip (MIT)',
  formats: [{ ext: 'pptx', label: 'PowerPoint' }, { ext: 'hslides', label: 'Hearth Slides' }],
  openExts: ['pptx', 'hslides', 'odp', 'ppt'],
  canWrite: (ext) => ['pptx', 'hslides'].includes(ext),
  lossy: () => null,
  P: P0(), cur: 0, sel: [], editing: null,
};
const uid = () => 'e' + H.uid();
const theme = () => THEMES[D.P.theme] || THEMES.paper;
const slide = () => D.P.slides[D.cur];

// ------------------------------------------------------------------ layouts
const LAYOUTS = {
  title: { name: 'Title slide', make: (W, Hh) => [tbox('title', W * 0.08, Hh * 0.3, W * 0.84, Hh * 0.2, 56, 'center', 'Click to add title'), tbox('subtitle', W * 0.12, Hh * 0.54, W * 0.76, Hh * 0.12, 28, 'center', 'Click to add subtitle')] },
  content: { name: 'Title and content', make: (W, Hh) => [tbox('title', W * 0.06, Hh * 0.06, W * 0.88, Hh * 0.16, 42, 'left', 'Click to add title'), tbox('body', W * 0.06, Hh * 0.26, W * 0.88, Hh * 0.66, 26, 'left', 'Click to add text', true)] },
  section: { name: 'Section header', make: (W, Hh) => [tbox('title', W * 0.08, Hh * 0.38, W * 0.84, Hh * 0.18, 52, 'left', 'Section title'), tbox('subtitle', W * 0.08, Hh * 0.58, W * 0.84, Hh * 0.1, 24, 'left', 'A line about it')] },
  two: { name: 'Two columns', make: (W, Hh) => [tbox('title', W * 0.06, Hh * 0.06, W * 0.88, Hh * 0.16, 42, 'left', 'Click to add title'), tbox('body', W * 0.06, Hh * 0.26, W * 0.42, Hh * 0.66, 24, 'left', 'Left side', true), tbox('body', W * 0.52, Hh * 0.26, W * 0.42, Hh * 0.66, 24, 'left', 'Right side', true)] },
  titleonly: { name: 'Title only', make: (W, Hh) => [tbox('title', W * 0.06, Hh * 0.06, W * 0.88, Hh * 0.16, 42, 'left', 'Click to add title')] },
  blank: { name: 'Blank', make: () => [] },
};
function tbox(role, x, y, w, h, size, align, ph, bullets) {
  return { id: uid(), type: 'text', role, x: Math.round(x), y: Math.round(y), w: Math.round(w), h: Math.round(h), size, align, valign: role === 'body' ? 'top' : 'middle', html: bullets ? '<ul><li></li></ul>' : '', ph };
}
D.newSlide = (layout = 'content', at = D.cur + 1) => ({ id: uid(), layout, els: LAYOUTS[layout].make(D.P.w, D.P.h), notes: '' });

// ------------------------------------------------------------------ drawing a slide
// The same element tree for the editor, thumbnails, presenting and printing (mode: edit, thumb, show, print).
D.render = (s, mode) => {
  const t = theme();
  const root = el('div', { class: 'slide' + (mode === 'edit' ? ' editing' : '') });
  Object.assign(root.style, { width: D.P.w + 'px', height: D.P.h + 'px', color: t.text, fontFamily: t.font });
  const bg = s.bg || t.bg;
  root.style.background = s.bgImage ? `center / cover no-repeat url("${s.bgImage}"), ${bg}` : (!s.bg && t.bg2 ? `linear-gradient(135deg, ${t.bg} 0%, ${t.bg2} 100%)` : bg);
  for (const e of s.els) root.append(D.renderEl(e, mode, t));
  return root;
};
D.renderEl = (e, mode, t = theme()) => {
  const n = el('div', { class: `el el-${e.type}`, 'data-id': e.id });
  Object.assign(n.style, { left: e.x + 'px', top: e.y + 'px', width: e.w + 'px', height: e.h + 'px', transform: e.rot ? `rotate(${e.rot}deg)` : '' });
  if (e.type === 'image') { n.append(el('img', { src: e.src, alt: e.alt || '', draggable: 'false' })); if (e.fit === 'cover') n.firstChild.style.objectFit = 'cover'; return n; }
  if (e.type === 'shape') n.append(shapeSvg(e, t));
  if (e.type === 'text' || e.type === 'shape') {
    const tx = el('div', { class: 'tx v-' + (e.valign || (e.type === 'shape' ? 'middle' : 'top')) });
    const inner = el('div', { class: 'txi', html: mode !== 'edit' && e.ph && isEmpty(e.html) ? '' : (e.html || '') });
    const role = e.role;
    Object.assign(inner.style, {
      fontSize: (e.size || 24) + 'px', textAlign: e.align || 'left',
      color: e.color || (role === 'title' ? t.title : e.type === 'shape' ? '#ffffff' : t.text),
      fontFamily: e.font ? `"${e.font}", ${t.font}` : (role === 'title' ? t.titleFont : t.font),
      fontWeight: e.bold ? '700' : (role === 'title' ? '700' : ''), lineHeight: e.lineHeight ? String(e.lineHeight) : '',
    });
    if (e.type === 'text' && e.fill) n.style.background = e.fill;
    if (e.type === 'text' && e.line) n.style.boxShadow = `inset 0 0 0 ${e.lineW || 2}px ${e.line}`;
    if (mode === 'edit' && e.ph && isEmpty(e.html)) inner.dataset.ph = e.ph;
    tx.append(inner); n.append(tx);
  }
  return n;
};
const isEmpty = (html) => !html || !html.replace(/<(?!img)[^>]*>/g, '').replace(/&nbsp;|\s|​/g, '');
function shapeSvg(e, t) {
  const fill = e.fill === undefined ? t.accent : (e.fill || 'none');
  const line = e.line || 'none', lw = e.line ? (e.lineW || 2) : 0;
  const w = e.w, h = e.h, i = lw / 2;
  let body;
  switch (e.shape) {
    case 'ellipse': body = `<ellipse cx="${w / 2}" cy="${h / 2}" rx="${Math.max(0, w / 2 - i)}" ry="${Math.max(0, h / 2 - i)}"/>`; break;
    case 'round': body = `<rect x="${i}" y="${i}" width="${Math.max(0, w - lw)}" height="${Math.max(0, h - lw)}" rx="${Math.min(w, h) * 0.16}"/>`; break;
    case 'triangle': body = `<polygon points="${w / 2},${i} ${w - i},${h - i} ${i},${h - i}"/>`; break;
    case 'arrow': body = `<polygon points="${i},${h * 0.3} ${w * 0.65},${h * 0.3} ${w * 0.65},${i} ${w - i},${h / 2} ${w * 0.65},${h - i} ${w * 0.65},${h * 0.7} ${i},${h * 0.7}"/>`; break;
    case 'star': { const pts = []; for (let k = 0; k < 10; k++) { const a = -Math.PI / 2 + k * Math.PI / 5, r = k % 2 ? 0.4 : 0.5; pts.push(`${(w / 2 + Math.cos(a) * w * r).toFixed(1)},${(h / 2 + Math.sin(a) * h * r).toFixed(1)}`); } body = `<polygon points="${pts.join(' ')}"/>`; break; }
    case 'line': return lineSvg(e, t);
    default: body = `<rect x="${i}" y="${i}" width="${Math.max(0, w - lw)}" height="${Math.max(0, h - lw)}"/>`;
  }
  const d = el('div', { class: 'shp', html: `<svg viewBox="0 0 ${w} ${h}" width="${w}" height="${h}" preserveAspectRatio="none"><g fill="${fill}" stroke="${line}" stroke-width="${lw}">${body}</g></svg>` });
  return d;
}
function lineSvg(e, t) {
  const c = e.line || t.text;
  return el('div', { class: 'shp', html: `<svg viewBox="0 0 ${e.w} ${Math.max(1, e.h)}" width="${e.w}" height="${Math.max(1, e.h)}" preserveAspectRatio="none"><line x1="0" y1="${e.flip ? e.h : 0}" x2="${e.w}" y2="${e.flip ? 0 : e.h}" stroke="${c}" stroke-width="${e.lineW || 3}"/></svg>` });
}

// ------------------------------------------------------------------ undo
const U = { stack: [], redo: [], lastT: 0, lastKind: '' };
const snapP = () => JSON.stringify({ P: D.P, cur: D.cur });
D.record = (kind = 'cmd') => {
  const now = Date.now();
  if (kind !== 'cmd' && kind === U.lastKind && now - U.lastT < 1500) { U.lastT = now; return; }
  U.stack.push(snapP()); if (U.stack.length > 150) U.stack.shift();
  U.redo = []; U.lastT = now; U.lastKind = kind;
};
const restoreP = (s) => { const o = JSON.parse(s); D.P = o.P; D.cur = Math.min(o.cur, D.P.slides.length - 1); D.sel = []; D.endEdit(true); D.refresh(); H.changed(); };
D.undo = () => { if (D.editing) D.endEdit(); const s = U.stack.pop(); if (!s) return H.toast('Nothing to undo'); U.redo.push(snapP()); restoreP(s); U.lastKind = ''; };
D.redo = () => { const s = U.redo.pop(); if (!s) return H.toast('Nothing to redo'); U.stack.push(snapP()); restoreP(s); U.lastKind = ''; };
D.changed = () => { H.changed(); D.thumbSoon(); };

// ------------------------------------------------------------------ the window
D.mount = async (main, toolbar, status) => {
  main.innerHTML = `
    <div class="sl-wrap">
      <aside id="thumbs" class="thumbs" tabindex="0" aria-label="Slides"></aside>
      <section class="sl-main">
        <div id="stage" class="stage"><div id="stageInner"></div><div id="selbox"></div></div>
        <div class="notes"><textarea id="notes" placeholder="Speaker notes: only you see these when presenting" spellcheck="true"></textarea></div>
      </section>
    </div>
    <div id="present" hidden></div>`;
  buildToolbar(toolbar);
  status.querySelector('#statusLeft').innerHTML = '<span id="slCount"></span>';
  status.querySelector('#statusRight').innerHTML = '<button id="slPresent" title="Present (F5)">▶ Present</button>';
  $('#slPresent').onclick = () => D.present(0);
  wire();
  new ResizeObserver(() => D.fit()).observe($('#stage'));
  setTimeout(() => H.need('vendor/jszip.min.js'), 600);
};
D.scroller = () => $('#thumbs');
D.focus = () => { $('#stage').focus({ preventScroll: true }); };

function buildToolbar(tb) {
  const I = H.ICON;
  const fonts = Array.from(new Set(['Calibri', 'Carlito', 'Arial', 'Liberation Sans', 'Georgia', 'Times New Roman', 'Inter', 'Noto Sans', 'Noto Serif', 'Verdana', 'Courier New', ...H.fonts])).sort();
  D.fontSel = H.tsel([['', 'Theme font'], ...fonts.map((f) => [f, f])], 'Font', (v) => D.setText({ font: v || null }), { cls: 'font' });
  D.sizeSel = H.tsel([12, 14, 16, 18, 20, 24, 28, 32, 36, 40, 44, 48, 56, 64, 72, 88, 96, 120].map((s) => [s, s]), 'Text size', (v) => D.setText({ size: +v }), { cls: 'size' });
  tb.append(
    H.tbtn(I.undo, 'Undo (Ctrl+Z)', () => D.undo()), H.tbtn(I.redo, 'Redo (Ctrl+Y)', () => D.redo()), H.tsep(),
    H.tbtn(I.plus + '<span>Slide</span>', 'New slide (Ctrl+M)', (e, b) => { const r = b.getBoundingClientRect(); H.showMenu(Object.entries(LAYOUTS).map(([k, l]) => ({ label: l.name, run: () => D.addSlide(k) })), r.left, r.bottom + 4); }, { wide: true }),
    H.tbtn(I.theme + '<span>Theme</span>', 'Theme', (e, b) => { const r = b.getBoundingClientRect(); H.showMenu(themeItems(), r.left, r.bottom + 4); }, { wide: true }), H.tsep(),
    H.tbtn(I.text, 'Text box', () => D.addText()), H.tbtn(I.image, 'Picture', () => D.addImage()),
    H.tbtn(I.shape, 'Shape', (e, b) => { const r = b.getBoundingClientRect(); H.showMenu(Object.entries(SHAPES).map(([k, l]) => ({ label: l, run: () => D.addShape(k) })), r.left, r.bottom + 4); }), H.tsep(),
    D.fontSel, D.sizeSel,
    H.tbtn(I.bold, 'Bold (Ctrl+B)', () => D.textCmd('bold')), H.tbtn(I.italic, 'Italic (Ctrl+I)', () => D.textCmd('italic')), H.tbtn(I.underline, 'Underline (Ctrl+U)', () => D.textCmd('underline')),
    H.colorBtn(I.color, 'Text colour', '#c00000', (c) => D.textColor(c)), H.tsep(),
    H.tbtn(I.left, 'Align left', () => D.setText({ align: 'left' })), H.tbtn(I.center, 'Centre', () => D.setText({ align: 'center' })), H.tbtn(I.right, 'Align right', () => D.setText({ align: 'right' })),
    H.tbtn(I.bullets, 'Bullets', () => D.textCmd('insertUnorderedList')), H.tbtn(I.numbers, 'Numbering', () => D.textCmd('insertOrderedList')), H.tsep(),
    H.colorBtn(I.fill, 'Fill', '#2f5496', (c) => D.setProp({ fill: c }), { none: 'No fill' }),
    H.colorBtn('<svg viewBox="0 0 24 24"><rect x="4" y="4" width="16" height="16" rx="2"/></svg>', 'Outline', '#000000', (c) => D.setProp({ line: c }), { none: 'No outline' }),
    H.tbtn(I.front, 'Bring to front', () => D.arrange('front')), H.tbtn(I.back, 'Send to back', () => D.arrange('back')), H.tbtn(I.trash, 'Delete (Delete)', () => D.deleteSel()), H.tsep(),
    H.tbtn(I.play + '<span>Present</span>', 'Present from the start (F5)', () => D.present(0), { wide: true }),
  );
}
const themeItems = () => Object.entries(THEMES).map(([k, t]) => ({ label: t.name, checked: () => D.P.theme === k, run: () => { D.record(); D.P.theme = k; D.refresh(); D.changed(); } }))
  .concat([{ sep: true }, { label: 'Slide background colour…', run: () => D.bgDialog() }, { label: 'Slide size', sub: [{ label: 'Widescreen 16:9', checked: () => D.P.h === 540, run: () => D.setSize(540) }, { label: 'Standard 4:3', checked: () => D.P.h === 720, run: () => D.setSize(720) }] }]);
D.setSize = (h) => {
  if (D.P.h === h) return;
  D.record(); const k = h / D.P.h;
  for (const s of D.P.slides) for (const e of s.els) { e.y = Math.round(e.y * k); e.h = Math.round(e.h * k); }
  D.P.h = h; D.refresh(); D.changed();
};
D.bgDialog = async () => {
  const s = slide();
  const v = await H.form('Background', [{ id: 'color', label: 'Colour', type: 'color', value: s.bg || theme().bg }, { id: 'all', label: 'Use on every slide', type: 'check', value: false }, { id: 'reset', label: 'Back to the theme\'s background', type: 'check', value: false }], 'Apply');
  if (!v) return;
  D.record();
  for (const x of v.all ? D.P.slides : [s]) x.bg = v.reset ? null : v.color;
  D.refresh(); D.changed();
};

// ------------------------------------------------------------------ the editor view
D.fit = () => {
  const st = $('#stage'); if (!st) return;
  const pad = 36;
  const k = Math.min((st.clientWidth - pad * 2) / D.P.w, (st.clientHeight - pad * 2) / D.P.h);
  D.k = Math.max(0.1, k);
  const inner = $('#stageInner');
  Object.assign(inner.style, { width: D.P.w + 'px', height: D.P.h + 'px', transform: `scale(${D.k})`, left: Math.round((st.clientWidth - D.P.w * D.k) / 2) + 'px', top: Math.round((st.clientHeight - D.P.h * D.k) / 2) + 'px' });
  D.drawSel();
};
D.refresh = () => {
  if (!D.P.slides.length) D.P.slides.push(D.newSlide('title'));
  D.cur = Math.max(0, Math.min(D.cur, D.P.slides.length - 1));
  const inner = $('#stageInner'); inner.innerHTML = '';
  inner.append(D.render(slide(), 'edit'));
  if (D.editing) { const n = inner.querySelector(`[data-id="${D.editing}"] .txi`); if (n) { n.contentEditable = 'true'; } else D.editing = null; }
  D.fit(); D.thumbs(); D.updateNotes();
  $('#slCount').textContent = `Slide ${D.cur + 1} of ${D.P.slides.length}`;
};
D.updateNotes = () => { const n = $('#notes'); if (document.activeElement !== n) n.value = slide().notes || ''; };
D.thumbSoon = H.debounce(() => D.thumbs(), 250);
D.thumbs = () => {
  const box = $('#thumbs'); const sc = box.scrollTop; box.innerHTML = '';
  const tw = 176, k = tw / D.P.w;
  D.P.slides.forEach((s, i) => {
    const t = el('div', { class: 'thumb' + (i === D.cur ? ' on' : ''), draggable: 'true', 'data-i': i });
    const holder = el('div', { class: 'th-in', style: { width: tw + 'px', height: Math.round(D.P.h * k) + 'px' } });
    const r = D.render(s, 'thumb'); r.style.transform = `scale(${k})`; r.style.transformOrigin = '0 0';
    holder.append(r);
    t.append(el('span', { class: 'th-n', text: i + 1 }), holder);
    t.addEventListener('mousedown', () => { D.go(i); });
    t.addEventListener('contextmenu', (e) => { e.preventDefault(); D.go(i); H.showMenu(slideMenu(i), e.clientX, e.clientY); });
    t.addEventListener('dragstart', (e) => { e.dataTransfer.setData('text/x-slide', String(i)); e.dataTransfer.effectAllowed = 'move'; });
    t.addEventListener('dragover', (e) => { if (e.dataTransfer.types.includes('text/x-slide')) { e.preventDefault(); t.classList.add('drop'); } });
    t.addEventListener('dragleave', () => t.classList.remove('drop'));
    t.addEventListener('drop', (e) => { e.preventDefault(); t.classList.remove('drop'); const from = +e.dataTransfer.getData('text/x-slide'); D.moveSlide(from, i); });
    box.append(t);
  });
  box.scrollTop = sc;
  const on = box.querySelector('.thumb.on'); if (on) { const r = on.getBoundingClientRect(), b = box.getBoundingClientRect(); if (r.top < b.top || r.bottom > b.bottom) on.scrollIntoView({ block: 'nearest' }); }
};
const slideMenu = (i) => [
  { label: 'New slide', key: 'Ctrl+M', run: () => D.addSlide('content') }, { label: 'Duplicate slide', key: 'Ctrl+Shift+D', run: () => D.dupSlide(i) },
  { label: 'Move up', disabled: () => i === 0, run: () => D.moveSlide(i, i - 1) }, { label: 'Move down', disabled: () => i === D.P.slides.length - 1, run: () => D.moveSlide(i, i + 1) },
  { label: 'Layout', sub: Object.entries(LAYOUTS).map(([k, l]) => ({ label: l.name, run: () => D.applyLayout(i, k) })) },
  { label: 'Background…', run: () => D.bgDialog() }, { sep: true },
  { label: 'Delete slide', disabled: () => D.P.slides.length < 2, run: () => D.delSlide(i) },
];
D.go = (i) => { if (D.editing) D.endEdit(); saveNotes(); D.cur = Math.max(0, Math.min(i, D.P.slides.length - 1)); D.sel = []; D.refresh(); };
D.addSlide = (layout) => { D.endEdit(); saveNotes(); D.record(); D.P.slides.splice(D.cur + 1, 0, D.newSlide(layout)); D.cur++; D.sel = []; D.refresh(); D.changed(); };
D.dupSlide = (i) => { D.record(); const c = JSON.parse(JSON.stringify(D.P.slides[i])); c.id = uid(); c.els.forEach((e) => { e.id = uid(); }); D.P.slides.splice(i + 1, 0, c); D.cur = i + 1; D.refresh(); D.changed(); };
D.delSlide = (i) => { if (D.P.slides.length < 2) return; D.record(); D.P.slides.splice(i, 1); D.cur = Math.min(D.cur, D.P.slides.length - 1); D.sel = []; D.refresh(); D.changed(); };
D.moveSlide = (from, to) => { if (from === to || to < 0 || to >= D.P.slides.length) return; D.record(); const [s] = D.P.slides.splice(from, 1); D.P.slides.splice(to, 0, s); D.cur = to; D.refresh(); D.changed(); };
D.applyLayout = (i, k) => {
  D.record();
  const s = D.P.slides[i]; const fresh = LAYOUTS[k].make(D.P.w, D.P.h);
  // Text already typed moves into the new layout's boxes with the same role.
  for (const f of fresh) { const old = s.els.find((e) => e.role === f.role && !e._used && !isEmpty(e.html)); if (old) { f.html = old.html; old._used = true; } }
  s.els = fresh.concat(s.els.filter((e) => !e.role || (!e._used && !isEmpty(e.html) && !fresh.some((f) => f.role === e.role)))).map((e) => { delete e._used; return e; });
  s.layout = k; D.refresh(); D.changed();
};
const saveNotes = () => { const n = $('#notes'); if (n && slide() && n.value !== (slide().notes || '')) { D.record('notes'); slide().notes = n.value; D.changed(); } };

// Selection and handles.
D.elById = (id) => slide().els.find((e) => e.id === id);
D.drawSel = () => {
  const box = $('#selbox'); if (!box) return;
  box.innerHTML = '';
  const st = $('#stage'), inner = $('#stageInner');
  const ox = parseFloat(inner.style.left) || 0, oy = parseFloat(inner.style.top) || 0;
  for (const id of D.sel) {
    const e = D.elById(id); if (!e) continue;
    const f = el('div', { class: 'selframe' + (D.editing === id ? ' editing' : '') });
    Object.assign(f.style, { left: ox + e.x * D.k + 'px', top: oy + e.y * D.k + 'px', width: e.w * D.k + 'px', height: Math.max(2, e.h * D.k) + 'px' });
    if (D.sel.length === 1 && D.editing !== id) for (const h of ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w']) f.append(el('div', { class: 'hd hd-' + h, 'data-h': h }));
    box.append(f);
  }
  void st;
  D.updateTools();
};
D.updateTools = () => {
  const e = D.sel.length === 1 ? D.elById(D.sel[0]) : null;
  if (!e) return;
  D.sizeSel.value = String(e.size || 24); if (![...D.sizeSel.options].some((o) => o.value === String(e.size))) D.sizeSel.append(el('option', { value: e.size, text: e.size })), (D.sizeSel.value = String(e.size));
  D.fontSel.value = e.font || '';
};
// Slide coordinates from a mouse event.
const toSlide = (ev) => { const r = $('#stageInner').getBoundingClientRect(); return { x: (ev.clientX - r.left) / D.k, y: (ev.clientY - r.top) / D.k }; };

function wire() {
  const st = $('#stage');
  st.tabIndex = 0;
  st.addEventListener('mousedown', (ev) => {
    if (ev.button !== 0) return;
    const hd = ev.target.closest('.hd');
    const target = ev.target.closest('.el');
    if (D.editing && target && target.dataset.id === D.editing) return; // clicking inside the text being edited
    if (D.editing) D.endEdit();
    if (hd) { ev.preventDefault(); startDrag(ev, 'resize', hd.dataset.h); return; }
    if (target) {
      ev.preventDefault();
      const id = target.dataset.id;
      if (ev.shiftKey) { D.sel = D.sel.includes(id) ? D.sel.filter((x) => x !== id) : D.sel.concat(id); }
      else if (!D.sel.includes(id)) D.sel = [id];
      D.drawSel(); st.focus({ preventScroll: true });
      startDrag(ev, 'move');
      return;
    }
    // Empty space: drag a box to select several.
    D.sel = []; D.drawSel(); st.focus({ preventScroll: true });
    const p0 = toSlide(ev);
    const band = el('div', { class: 'band' }); $('#selbox').append(band);
    const inner = $('#stageInner'); const ox = parseFloat(inner.style.left), oy = parseFloat(inner.style.top);
    const mv = (e2) => { const p = toSlide(e2); const x = Math.min(p.x, p0.x), y = Math.min(p.y, p0.y), w = Math.abs(p.x - p0.x), h = Math.abs(p.y - p0.y); Object.assign(band.style, { left: ox + x * D.k + 'px', top: oy + y * D.k + 'px', width: w * D.k + 'px', height: h * D.k + 'px' }); band._r = { x, y, w, h }; };
    const up = () => { removeEventListener('mousemove', mv); removeEventListener('mouseup', up); const r = band._r; band.remove(); if (r && r.w > 4) { D.sel = slide().els.filter((e) => e.x < r.x + r.w && e.x + e.w > r.x && e.y < r.y + r.h && e.y + e.h > r.y).map((e) => e.id); D.drawSel(); } };
    addEventListener('mousemove', mv); addEventListener('mouseup', up);
  });
  st.addEventListener('dblclick', (ev) => { const t = ev.target.closest('.el'); if (t) { const e = D.elById(t.dataset.id); if (e && (e.type === 'text' || e.type === 'shape')) D.startEdit(e.id, ev); else if (e && e.type === 'image') D.replaceImage(e); } });
  st.addEventListener('contextmenu', (ev) => { ev.preventDefault(); const t = ev.target.closest('.el'); if (t && !D.sel.includes(t.dataset.id)) { D.sel = [t.dataset.id]; D.drawSel(); } H.showMenu(elMenu(), ev.clientX, ev.clientY); });
  st.addEventListener('keydown', onKey);
  $('#notes').addEventListener('input', () => { D.record('notes'); slide().notes = $('#notes').value; H.changed(); });
  $('#thumbs').addEventListener('keydown', (e) => {
    const k = H.keyOf(e);
    if (k === 'ArrowDown' || k === 'ArrowUp') { e.preventDefault(); D.go(D.cur + (k === 'ArrowDown' ? 1 : -1)); }
    else if (k === 'Ctrl+ArrowDown' || k === 'Ctrl+ArrowUp') { e.preventDefault(); D.moveSlide(D.cur, D.cur + (k === 'Ctrl+ArrowDown' ? 1 : -1)); }
    else if (k === 'Delete' || k === 'Backspace') { e.preventDefault(); D.delSlide(D.cur); }
    else if (k === 'Enter') { e.preventDefault(); D.addSlide('content'); }
  });
  // Pasted pictures go on the slide.
  document.addEventListener('paste', (e) => {
    if (H.app !== D || D.editing || document.activeElement === $('#notes')) return;
    const f = Array.from((e.clipboardData && e.clipboardData.files) || []).find((x) => x.type.startsWith('image/'));
    if (f) { e.preventDefault(); const fr = new FileReader(); fr.onload = () => D.placeImage(fr.result); fr.readAsDataURL(f); return; }
    if (D.clip && document.activeElement && document.activeElement.closest && document.activeElement.closest('#stage')) { e.preventDefault(); D.pasteEls(); }
  });
}
function startDrag(ev, kind, handle) {
  const p0 = toSlide(ev);
  const start = D.sel.map((id) => ({ id, ...JSON.parse(JSON.stringify(D.elById(id))) }));
  let moved = false;
  const mv = (e2) => {
    const p = toSlide(e2); let dx = p.x - p0.x, dy = p.y - p0.y;
    if (!moved && Math.hypot(dx, dy) < 2) return;
    if (!moved) { D.record(); moved = true; }
    for (const s of start) {
      const e = D.elById(s.id); if (!e) continue;
      if (kind === 'move') {
        let nx = s.x + dx, ny = s.y + dy;
        // Snap to the slide's centre and edges.
        if (!e2.altKey && start.length === 1) { const cx = nx + e.w / 2, cy = ny + e.h / 2; if (Math.abs(cx - D.P.w / 2) < 6) nx = D.P.w / 2 - e.w / 2; if (Math.abs(cy - D.P.h / 2) < 6) ny = D.P.h / 2 - e.h / 2; if (Math.abs(nx) < 6) nx = 0; if (Math.abs(ny) < 6) ny = 0; }
        e.x = Math.round(nx); e.y = Math.round(ny);
      } else {
        let { x, y, w, h } = s;
        if (handle.includes('e')) w = Math.max(10, s.w + dx);
        if (handle.includes('s')) h = Math.max(e.type === 'shape' && e.shape === 'line' ? 1 : 10, s.h + dy);
        if (handle.includes('w')) { w = Math.max(10, s.w - dx); x = s.x + s.w - w; }
        if (handle.includes('n')) { h = Math.max(10, s.h - dy); y = s.y + s.h - h; }
        // Pictures keep their shape from the corners (unless Shift).
        if (e.type === 'image' && handle.length === 2 && !e2.shiftKey) { const ar = s.w / s.h; if (w / h > ar) w = h * ar; else h = w / ar; if (handle.includes('w')) x = s.x + s.w - w; if (handle.includes('n')) y = s.y + s.h - h; }
        Object.assign(e, { x: Math.round(x), y: Math.round(y), w: Math.round(w), h: Math.round(h) });
      }
      const n = $(`#stageInner [data-id="${e.id}"]`);
      if (n) { const fresh = D.renderEl(e, 'edit'); n.replaceWith(fresh); }
    }
    D.drawSel();
  };
  const up = () => { removeEventListener('mousemove', mv); removeEventListener('mouseup', up); if (moved) D.changed(); };
  addEventListener('mousemove', mv); addEventListener('mouseup', up);
}
const elMenu = () => [
  { label: 'Cut', key: 'Ctrl+X', disabled: () => !D.sel.length, run: () => { D.copyEls(); D.deleteSel(); } }, { label: 'Copy', key: 'Ctrl+C', disabled: () => !D.sel.length, run: () => D.copyEls() },
  { label: 'Paste', key: 'Ctrl+V', disabled: () => !D.clip, run: () => D.pasteEls() }, { label: 'Duplicate', key: 'Ctrl+D', disabled: () => !D.sel.length, run: () => D.duplicate() }, { sep: true },
  { label: 'Bring to front', run: () => D.arrange('front') }, { label: 'Send to back', run: () => D.arrange('back') },
  { label: 'Align', sub: [['left', 'Left of slide'], ['hcenter', 'Centre of slide'], ['right', 'Right of slide'], ['top', 'Top'], ['vcenter', 'Middle'], ['bottom', 'Bottom']].map(([k, l]) => ({ label: l, run: () => D.alignEls(k) })) },
  { label: 'Edit text', disabled: () => !(D.sel.length === 1 && ['text', 'shape'].includes((D.elById(D.sel[0]) || {}).type)), run: () => D.startEdit(D.sel[0]) },
  { label: 'Change picture…', disabled: () => !(D.sel.length === 1 && (D.elById(D.sel[0]) || {}).type === 'image'), run: () => D.replaceImage(D.elById(D.sel[0])) },
  { label: 'Position and size…', disabled: () => D.sel.length !== 1, run: () => D.posDialog() }, { sep: true },
  { label: 'Delete', key: 'Delete', disabled: () => !D.sel.length, run: () => D.deleteSel() },
];
function onKey(e) {
  if (D.editing) {
    if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); D.endEdit(); }
    return;
  }
  const k = H.keyOf(e);
  const nav = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
  const base = k.replace(/^Shift\+/, '');
  if (nav[base] && D.sel.length) {
    e.preventDefault(); D.record('nudge');
    const step = e.shiftKey ? 20 : 2; const [dx, dy] = nav[base];
    for (const id of D.sel) { const el2 = D.elById(id); el2.x += dx * step; el2.y += dy * step; }
    D.refresh(); D.drawSel(); D.changed(); return;
  }
  if (nav[base] && !D.sel.length && (base === 'ArrowDown' || base === 'ArrowUp' || base === 'ArrowLeft' || base === 'ArrowRight')) { e.preventDefault(); D.go(D.cur + (base === 'ArrowDown' || base === 'ArrowRight' ? 1 : -1)); return; }
  if ((k === 'Delete' || k === 'Backspace') && D.sel.length) { e.preventDefault(); D.deleteSel(); return; }
  if ((k === 'Enter' || k === 'F2') && D.sel.length === 1) { e.preventDefault(); D.startEdit(D.sel[0]); return; }
  if (k === 'Escape') { D.sel = []; D.drawSel(); return; }
  if (k === 'Tab' || k === 'Shift+Tab') { e.preventDefault(); const els = slide().els; if (!els.length) return; const i = els.findIndex((x) => x.id === D.sel[0]); const n = els[((i < 0 ? -1 : i) + (k === 'Tab' ? 1 : -1) + els.length) % els.length]; D.sel = [n.id]; D.drawSel(); return; }
  // Typing on a selected text box starts editing it.
  if (k.length === 1 && D.sel.length === 1 && !e.ctrlKey && !e.altKey) { const el2 = D.elById(D.sel[0]); if (el2 && (el2.type === 'text' || el2.type === 'shape')) { D.startEdit(el2.id, null, true); } }
}

// Editing text in place.
D.startEdit = (id, ev, replace) => {
  const e = D.elById(id); if (!e) return;
  D.sel = [id]; D.editing = id;
  const n = $(`#stageInner [data-id="${id}"] .txi`); if (!n) return;
  D.record('text-' + id);
  delete n.dataset.ph;
  n.contentEditable = 'true';
  n.focus();
  const r = document.createRange();
  if (replace && e.ph && isEmpty(e.html)) { r.selectNodeContents(n); r.collapse(false); }
  else if (ev && document.caretRangeFromPoint) { const cr = document.caretRangeFromPoint(ev.clientX, ev.clientY); if (cr && n.contains(cr.startContainer)) { r.setStart(cr.startContainer, cr.startOffset); r.collapse(true); } else { r.selectNodeContents(n); r.collapse(false); } }
  else { r.selectNodeContents(n); if (!replace) r.collapse(false); }
  const s = getSelection(); s.removeAllRanges(); s.addRange(r);
  n.oninput = () => { const v = D.elById(id); if (v) { v.html = n.innerHTML; H.changed(); D.thumbSoon(); } };
  n.onkeydown = (ke) => {
    const kk = H.keyOf(ke);
    if (kk === 'Escape') { ke.preventDefault(); ke.stopPropagation(); D.endEdit(); }
    if (kk === 'Tab' || kk === 'Shift+Tab') { ke.preventDefault(); document.execCommand(kk === 'Tab' ? 'indent' : 'outdent'); }
  };
  n.onpaste = (pe) => { pe.preventDefault(); const t = pe.clipboardData.getData('text/plain'); document.execCommand('insertText', false, t); };
  D.drawSel();
};
D.endEdit = (quiet) => {
  if (!D.editing) return;
  const id = D.editing; D.editing = null;
  const n = $(`#stageInner [data-id="${id}"] .txi`);
  const e = D.elById(id);
  if (n && e) { e.html = H.clean(n.innerHTML).replace(/<br>$/, ''); n.contentEditable = 'false'; if (isEmpty(e.html) && !e.ph) { slide().els = slide().els.filter((x) => x.id !== id); D.sel = []; } }
  if (!quiet) { D.refresh(); D.changed(); }
};
// Formatting: inside the text being edited it applies to the selected words; otherwise to whole boxes.
D.textCmd = (cmd) => {
  if (D.editing) { document.execCommand('styleWithCSS', false, true); document.execCommand(cmd, false, null); const n = $(`#stageInner [data-id="${D.editing}"] .txi`); if (n) { D.elById(D.editing).html = n.innerHTML; H.changed(); } return; }
  const els = D.sel.map(D.elById).filter((e) => e && (e.type === 'text' || e.type === 'shape'));
  if (!els.length) return;
  D.record();
  for (const e of els) {
    if (cmd === 'bold') e.bold = !e.bold;
    else if (cmd === 'italic' || cmd === 'underline') { const tag = cmd === 'italic' ? 'i' : 'u'; const re = new RegExp(`^<${tag}>([\\s\\S]*)</${tag}>$`); e.html = re.test(e.html) ? e.html.replace(re, '$1') : `<${tag}>${e.html}</${tag}>`; }
    else if (cmd === 'insertUnorderedList' || cmd === 'insertOrderedList') {
      const tag = cmd === 'insertUnorderedList' ? 'ul' : 'ol';
      const box = el('div', { html: e.html });
      const list = box.querySelector('ul,ol');
      if (list && list.tagName.toLowerCase() === tag) e.html = Array.from(box.querySelectorAll('li')).map((li) => `<div>${li.innerHTML}</div>`).join('');
      else { const lines = list ? Array.from(box.querySelectorAll('li')).map((li) => li.innerHTML) : (box.innerHTML.split(/<br\s*\/?>|<\/div>|<\/p>/i).map((x) => x.replace(/<\/?(div|p)[^>]*>/gi, '')).filter((x) => x.trim())); e.html = `<${tag}>${(lines.length ? lines : ['']).map((l) => `<li>${l}</li>`).join('')}</${tag}>`; }
    }
  }
  D.refresh(); D.changed();
};
D.textColor = (c) => {
  if (D.editing) { document.execCommand('styleWithCSS', false, true); document.execCommand('foreColor', false, c); const n = $(`#stageInner [data-id="${D.editing}"] .txi`); if (n) { D.elById(D.editing).html = n.innerHTML; H.changed(); } return; }
  D.setText({ color: c });
};
D.setText = (props) => {
  const els = D.sel.map(D.elById).filter((e) => e && (e.type === 'text' || e.type === 'shape'));
  if (!els.length) { H.toast('Select a text box first'); return; }
  if (D.editing && props.size) { /* the whole box's size, also while editing */ }
  D.record();
  for (const e of els) for (const [k, v] of Object.entries(props)) { if (v == null) delete e[k]; else e[k] = v; }
  const was = D.editing; if (was) D.endEdit(true);
  D.refresh(); D.changed(); if (was) D.startEdit(was);
};
D.setProp = (props) => {
  const els = D.sel.map(D.elById).filter(Boolean);
  if (!els.length) { H.toast('Select something on the slide first'); return; }
  D.record();
  for (const e of els) for (const [k, v] of Object.entries(props)) { if (v == null) { if (k === 'fill' && e.type === 'shape') e.fill = null; else delete e[k]; } else e[k] = v; }
  D.refresh(); D.changed();
};

// Adding things.
D.addText = () => { D.endEdit(); D.record(); const e = { id: uid(), type: 'text', x: D.P.w * 0.25, y: D.P.h * 0.4, w: D.P.w * 0.5, h: 80, size: 28, align: 'left', valign: 'top', html: '' }; e.ph = 'Type here'; slide().els.push(e); D.sel = [e.id]; D.refresh(); D.startEdit(e.id); D.changed(); };
D.addShape = (shape) => {
  D.endEdit(); D.record();
  const e = { id: uid(), type: 'shape', shape, x: D.P.w * 0.35, y: D.P.h * 0.32, w: shape === 'line' ? 240 : 220, h: shape === 'line' ? 1 : 150, size: 24, align: 'center', valign: 'middle', html: '' };
  if (shape === 'line') { e.line = theme().text; e.lineW = 3; }
  slide().els.push(e); D.sel = [e.id]; D.refresh(); D.changed();
};
D.addImage = async () => {
  const p = await H.invoke('pick_open', { kind: 'image' }); if (!p) return;
  const r = await H.invoke('read_file', { path: p });
  let url = `data:${H.mimeOf(H.extOf(p))};base64,${r.data}`;
  if (['bmp', 'tif', 'tiff'].includes(H.extOf(p))) { const png = await H.toPng(url); if (png) url = png.url; }
  D.placeImage(url, H.stem(p));
};
D.placeImage = async (url, alt) => {
  const { w, h } = await H.imageSize(url);
  const k = Math.min(1, (D.P.w * 0.6) / (w || 1), (D.P.h * 0.6) / (h || 1));
  const iw = Math.round((w || 400) * k), ih = Math.round((h || 300) * k);
  D.record();
  const e = { id: uid(), type: 'image', src: url, alt: alt || '', x: Math.round((D.P.w - iw) / 2), y: Math.round((D.P.h - ih) / 2), w: iw, h: ih };
  slide().els.push(e); D.sel = [e.id]; D.refresh(); D.changed();
};
D.replaceImage = async (e) => {
  const p = await H.invoke('pick_open', { kind: 'image' }); if (!p) return;
  const r = await H.invoke('read_file', { path: p });
  D.record(); e.src = `data:${H.mimeOf(H.extOf(p))};base64,${r.data}`; D.refresh(); D.changed();
};
D.deleteSel = () => { if (!D.sel.length) return; D.record(); slide().els = slide().els.filter((e) => !D.sel.includes(e.id)); D.sel = []; D.refresh(); D.changed(); };
D.copyEls = () => { D.clip = D.sel.map((id) => JSON.parse(JSON.stringify(D.elById(id)))).filter(Boolean); try { navigator.clipboard.writeText(D.clip.map((e) => (e.html || '').replace(/<[^>]+>/g, ' ')).join('\n')); } catch {} };
D.pasteEls = () => { if (!D.clip) return; D.record(); const add = D.clip.map((e) => ({ ...JSON.parse(JSON.stringify(e)), id: uid(), x: e.x + 16, y: e.y + 16 })); slide().els.push(...add); D.sel = add.map((e) => e.id); D.clip = add.map((e) => JSON.parse(JSON.stringify(e))); D.refresh(); D.changed(); };
D.duplicate = () => { D.copyEls(); D.pasteEls(); };
D.arrange = (where) => { if (!D.sel.length) return; D.record(); const s = slide(); const picked = s.els.filter((e) => D.sel.includes(e.id)); const rest = s.els.filter((e) => !D.sel.includes(e.id)); s.els = where === 'front' ? rest.concat(picked) : picked.concat(rest); D.refresh(); D.changed(); };
D.alignEls = (k) => {
  D.record();
  for (const id of D.sel) { const e = D.elById(id); if (k === 'left') e.x = 0; if (k === 'right') e.x = D.P.w - e.w; if (k === 'hcenter') e.x = Math.round((D.P.w - e.w) / 2); if (k === 'top') e.y = 0; if (k === 'bottom') e.y = D.P.h - e.h; if (k === 'vcenter') e.y = Math.round((D.P.h - e.h) / 2); }
  D.refresh(); D.changed();
};
D.posDialog = async () => {
  const e = D.elById(D.sel[0]); if (!e) return;
  const v = await H.form('Position and size', [
    { id: 'x', label: 'Left', type: 'number', value: e.x, half: true }, { id: 'y', label: 'Top', type: 'number', value: e.y, half: true },
    { id: 'w', label: 'Width', type: 'number', value: e.w, half: true }, { id: 'h', label: 'Height', type: 'number', value: e.h, half: true },
    { id: 'rot', label: 'Rotation (degrees)', type: 'number', value: e.rot || 0, half: true },
    { type: 'note', label: `The slide is ${D.P.w} × ${D.P.h}.` },
  ], 'Apply');
  if (!v) return;
  D.record(); Object.assign(e, { x: v.x | 0, y: v.y | 0, w: Math.max(1, v.w | 0), h: Math.max(1, v.h | 0) }); if (v.rot) e.rot = v.rot; else delete e.rot; D.refresh(); D.changed();
};

// ------------------------------------------------------------------ presenting
// Full screen. Next: →, ↓, Page Down, Space, Enter, N, click, A/RB/d-pad right on a controller (and clickers,
// which send Page Down/Up). Back: ←, ↑, Page Up, Backspace, P, B/LB. Esc ends. B or . blacks the screen, W or ,
// whites it; a number then Enter jumps there; S switches to the presenter view (notes, next slide, timer).
D.present = (from = D.cur, presenter = false) => {
  if (D.editing) D.endEdit();
  saveNotes();
  const pr = $('#present');
  D.show = { i: Math.max(0, Math.min(from, D.P.slides.length - 1)), presenter, blank: null, typed: '', t0: Date.now() };
  pr.hidden = false; pr.tabIndex = 0;
  H.invoke('fullscreen', { on: true });
  D.showDraw();
  pr.focus();
  pr.onkeydown = presentKey;
  pr.onclick = (e) => { if (e.target.closest('button')) return; if (!D.show.presenter) D.showStep(1); };
  pr.oncontextmenu = (e) => { e.preventDefault(); D.showStep(-1); };
  pr.onwheel = (e) => { D.showStep(e.deltaY > 0 ? 1 : -1); };
  D.timer = setInterval(() => { const t = $('#prTimer'); if (t) { const s = Math.floor((Date.now() - D.show.t0) / 1000); t.textContent = `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`; } }, 1000);
  addEventListener('resize', D.showDraw);
};
D.endShow = () => {
  if (!D.show) return;
  const i = D.show.i;
  D.show = null; clearInterval(D.timer); removeEventListener('resize', D.showDraw);
  const pr = $('#present'); pr.hidden = true; pr.innerHTML = '';
  H.invoke('fullscreen', { on: false });
  D.go(i); D.focus();
};
D.showStep = (d) => {
  const s = D.show; if (!s) return;
  if (s.blank) { s.blank = null; D.showDraw(); return; }
  const n = s.i + d;
  if (n >= D.P.slides.length) { if (s.end) { D.endShow(); return; } s.end = true; D.showDraw(); return; }
  s.end = false;
  s.i = Math.max(0, n); D.showDraw();
};
D.showDraw = () => {
  const s = D.show; if (!s) return;
  const pr = $('#present'); pr.innerHTML = '';
  const W = innerWidth, Hh = innerHeight;
  const place = (sl, w, h) => { const k = Math.min(w / D.P.w, h / D.P.h); const box = el('div', { class: 'pr-slide', style: { width: D.P.w * k + 'px', height: D.P.h * k + 'px' } }); const r = D.render(sl, 'show'); r.style.transform = `scale(${k})`; r.style.transformOrigin = '0 0'; box.append(r); return box; };
  if (s.blank) { pr.style.background = s.blank; return; }
  pr.style.background = '#000';
  if (s.end) { pr.append(el('div', { class: 'pr-end', text: 'End of the presentation. Press any key to leave.' })); return; }
  if (!s.presenter) {
    const box = place(D.P.slides[s.i], W, Hh); box.classList.add('fade'); pr.append(el('div', { class: 'pr-center' }, box));
    return;
  }
  const next = D.P.slides[s.i + 1];
  const left = el('div', { class: 'pv-cur' }, place(D.P.slides[s.i], W * 0.62, Hh * 0.72));
  const right = el('div', { class: 'pv-side' },
    el('div', { class: 'pv-lab', text: next ? 'Next' : 'Last slide' }), next ? place(next, W * 0.3, Hh * 0.3) : el('div'),
    el('div', { class: 'pv-lab', text: 'Notes' }), el('div', { class: 'pv-notes', text: D.P.slides[s.i].notes || '(no notes)' }));
  const bar = el('div', { class: 'pv-bar' }, el('span', { id: 'prTimer', text: '0:00' }), el('span', { text: `Slide ${s.i + 1} of ${D.P.slides.length}` }),
    el('button', { class: 'btn', text: '‹ Back', onclick: () => D.showStep(-1) }), el('button', { class: 'btn primary', text: 'Next ›', onclick: () => D.showStep(1) }),
    el('button', { class: 'btn', text: 'Audience view (S)', onclick: () => { s.presenter = false; D.showDraw(); } }), el('button', { class: 'btn', text: 'End (Esc)', onclick: () => D.endShow() }));
  pr.append(el('div', { class: 'pv' }, left, right, bar));
};
function presentKey(e) {
  const s = D.show; if (!s) return;
  const k = e.key;
  e.preventDefault();
  if (s.end && k !== 'ArrowLeft' && k !== 'PageUp' && k !== 'Backspace') { D.endShow(); return; }
  if (/^[0-9]$/.test(k)) { s.typed += k; return; }
  if (k === 'Enter' && s.typed) { const n = parseInt(s.typed, 10) - 1; s.typed = ''; if (n >= 0 && n < D.P.slides.length) { s.i = n; s.end = false; D.showDraw(); } return; }
  s.typed = '';
  if (['ArrowRight', 'ArrowDown', 'PageDown', ' ', 'Enter', 'n', 'N'].includes(k)) D.showStep(1);
  else if (['ArrowLeft', 'ArrowUp', 'PageUp', 'Backspace', 'p', 'P'].includes(k)) D.showStep(-1);
  else if (k === 'Home') { s.i = 0; s.end = false; D.showDraw(); }
  else if (k === 'End') { s.i = D.P.slides.length - 1; s.end = false; D.showDraw(); }
  else if (k === 'Escape') D.endShow();
  else if (k === 'b' || k === 'B' || k === '.') { s.blank = s.blank ? null : '#000'; D.showDraw(); }
  else if (k === 'w' || k === 'W' || k === ',') { s.blank = s.blank ? null : '#fff'; D.showDraw(); }
  else if (k === 's' || k === 'S') { s.presenter = !s.presenter; D.showDraw(); }
}
// The controller: presenting (A, RB, d-pad → next; B, LB, ← back; Start or View ends; Y presenter view), and
// in the editor the d-pad moves between slides.
D.pad = (b) => {
  if (D.show) {
    if (['A', 'RB', 'Right', 'Down', 'RT'].includes(b)) D.showStep(1);
    else if (['B', 'LB', 'Left', 'Up', 'LT'].includes(b)) D.showStep(-1);
    else if (b === 'Start' || b === 'Back') D.endShow();
    else if (b === 'Y') { D.show.presenter = !D.show.presenter; D.showDraw(); }
    else if (b === 'X') { D.show.blank = D.show.blank ? null : '#000'; D.showDraw(); }
    return true;
  }
  if ((H._dialogs && H._dialogs.length) || $$('.menu').length || !$('#start').hidden || D.editing) return false;
  if (b === 'Up' || b === 'Left' || b === 'LB') { D.go(D.cur - 1); return true; }
  if (b === 'Down' || b === 'Right' || b === 'RB') { D.go(D.cur + 1); return true; }
  if (b === 'A' || b === 'Start') { if (b === 'Start' && !$('#start').hidden) return false; D.present(D.cur); return true; }
  return false;
};

// ------------------------------------------------------------------ menus
D.menus = () => [
  { label: 'File', items: H.fileMenu() },
  { label: 'Edit', items: [
    { label: 'Undo', key: 'Ctrl+Z', run: () => D.undo() }, { label: 'Redo', key: 'Ctrl+Y / Ctrl+Shift+Z', run: () => D.redo() }, { sep: true },
    { label: 'Cut', key: 'Ctrl+X', run: () => { if (D.editing) document.execCommand('cut'); else { D.copyEls(); D.deleteSel(); } } },
    { label: 'Copy', key: 'Ctrl+C', run: () => { if (D.editing) document.execCommand('copy'); else D.copyEls(); } },
    { label: 'Paste', key: 'Ctrl+V', run: () => { if (!D.editing) D.pasteEls(); } },
    { label: 'Duplicate', key: 'Ctrl+D', run: () => D.duplicate() },
    { label: 'Select all', key: 'Ctrl+A', run: () => { if (D.editing) document.execCommand('selectAll'); else { D.sel = slide().els.map((e) => e.id); D.drawSel(); } } },
    { label: 'Delete', run: () => D.deleteSel() },
  ] },
  { label: 'View', items: [
    { label: 'Present from the start', key: 'F5', run: () => D.present(0) }, { label: 'Present from this slide', key: 'Shift+F5', run: () => D.present(D.cur) },
    { label: 'Presenter view', key: 'Alt+F5', run: () => D.present(D.cur, true) }, { sep: true },
    { label: 'Light window', checked: () => H.settings.theme === 'light', run: () => { H.settings.theme = H.settings.theme === 'light' ? 'dark' : 'light'; H.applyTheme(); H.saveSettings(); } },
  ] },
  { label: 'Insert', items: [
    { label: 'New slide', key: 'Ctrl+M', run: () => D.addSlide('content') }, { label: 'Slide with layout', sub: Object.entries(LAYOUTS).map(([k, l]) => ({ label: l.name, run: () => D.addSlide(k) })) },
    { label: 'Duplicate slide', key: 'Ctrl+Shift+D', run: () => D.dupSlide(D.cur) }, { sep: true },
    { label: 'Text box', run: () => D.addText() }, { label: 'Picture…', run: () => D.addImage() }, { label: 'Shape', sub: Object.entries(SHAPES).map(([k, l]) => ({ label: l, run: () => D.addShape(k) })) },
  ] },
  { label: 'Slide', items: [
    { label: 'Layout', sub: Object.entries(LAYOUTS).map(([k, l]) => ({ label: l.name, run: () => D.applyLayout(D.cur, k) })) },
    { label: 'Theme', sub: themeItems() }, { label: 'Background…', run: () => D.bgDialog() }, { sep: true },
    { label: 'Move slide up', key: 'Ctrl+Shift+ArrowUp', run: () => D.moveSlide(D.cur, D.cur - 1) }, { label: 'Move slide down', key: 'Ctrl+Shift+ArrowDown', run: () => D.moveSlide(D.cur, D.cur + 1) },
    { label: 'Delete slide', run: () => D.delSlide(D.cur) },
  ] },
  { label: 'Arrange', items: elMenu().slice(5) },
  { label: 'Help', items: H.helpMenu([{ label: 'Presenting: keys and controller', run: () => H.alert('Presenting', 'Next: → ↓ Page Down Space Enter, a click, or A / RB / d-pad on a controller. Back: ← ↑ Page Up, or B / LB. Number then Enter: go to that slide. B or . : black screen; W or , : white. S or Y: presenter view with your notes, the next slide and a timer. Esc or Start: stop.') }, { label: 'What files keep what', run: () => D.formatHelp() }, { sep: true }]) },
];
D.extraKeys = () => ({ 'Ctrl+Shift+Z': { run: () => D.redo() }, 'Ctrl+PageDown': { run: () => D.go(D.cur + 1) }, 'Ctrl+PageUp': { run: () => D.go(D.cur - 1) } });
D.formatHelp = () => H.dialog({ title: 'What each file type keeps', wide: true, body: `<div class="fmt">
  <p><b>.hslides (Hearth Slides)</b>: everything.</p>
  <p><b>.pptx (PowerPoint)</b>, saving: slides, text boxes (fonts, sizes, colours, bold/italic/underline, bullets and numbering, alignment), pictures, shapes (with their text, fill and outline), backgrounds, slide size and speaker notes. The theme's colours and fonts are written onto each slide. Not saved: rotation of text, gradients.</p>
  <p><b>.pptx</b>, opening (best effort): text with its formatting, positions, pictures, simple shapes, tables (as text), backgrounds, speaker notes. Not read: charts, SmartArt, animations, transitions, videos, master-slide decorations.</p></div>` });

// ------------------------------------------------------------------ new, open, save
D.templates = Object.entries(THEMES).slice(0, 6).map(([k, t]) => ({ id: k, name: k === 'paper' ? 'Blank presentation' : t.name, desc: k === 'paper' ? 'White, simple' : 'Theme', thumb: `<div class="mslide" style="background:${t.bg2 ? `linear-gradient(135deg,${t.bg},${t.bg2})` : t.bg}"><b style="background:${t.title}"></b><i style="background:${t.text}"></i><u style="background:${t.accent}"></u></div>` }));
D.newDoc = (tpl) => {
  D.P = P0(); D.P.theme = THEMES[tpl] ? tpl : 'paper';
  D.P.slides = [D.newSlide('title')];
  D.cur = 0; D.sel = []; D.editing = null; U.stack = []; U.redo = [];
  D.refresh(); D.focus();
};
D.load = async (bytes, ext) => {
  let notes = [];
  if (ext === 'hslides') {
    const o = JSON.parse(H.bytesToText(bytes));
    if (!o || !Array.isArray(o.slides)) throw new Error('This isn\'t a Hearth Slides file.');
    D.P = Object.assign(P0(), o);
    for (const s of D.P.slides) { s.id = s.id || uid(); s.els = (s.els || []).map((e) => ({ ...e, id: e.id || uid(), html: e.html != null ? H.clean(e.html) : e.html })); }
  } else if (ext === 'pptx') {
    await H.need('vendor/jszip.min.js', 'formats/pptx.js');
    const r = await SlidesPptx.read(bytes);
    D.P = r.P; notes = r.notes || [];
    for (const s of D.P.slides) for (const e of s.els) if (e.html) e.html = H.clean(e.html);
  } else if (ext === 'odp' || ext === 'ppt') {
    throw new Error(`Hearth Slides can't open .${ext} files yet. Open it in LibreOffice and save it as .pptx.`);
  }
  if (!D.P.slides.length) D.P.slides.push(D.newSlide('title'));
  D.cur = 0; D.sel = []; D.editing = null; U.stack = []; U.redo = [];
  D.refresh();
  return notes;
};
D.save = async (ext) => {
  if (D.editing) D.endEdit();
  saveNotes();
  if (ext === 'hslides') return H.textToBytes(JSON.stringify({ format: 'hearth-slides', version: 1, ...D.P }));
  await H.need('vendor/pptxgen.bundle.js', 'vendor/jszip.min.js', 'formats/pptx.js');
  return SlidesPptx.write(D.P, THEMES[D.P.theme] || THEMES.paper);
};
D.snapshot = () => JSON.stringify(D.P);
D.restore = (s) => { D.P = Object.assign(P0(), JSON.parse(s)); D.cur = 0; D.sel = []; D.refresh(); };
// Print layout and PDF: a page per slide, 10 × 5.625 in (or 10 × 7.5 for 4:3).
D.printLayout = async (box) => {
  const w = 254, h = 254 * D.P.h / D.P.w;
  const k = (w / 25.4 * 96) / D.P.w;
  for (const s of D.P.slides) {
    const pg = el('div', { class: 'ppage', style: { width: w + 'mm', height: h + 'mm' } });
    const r = D.render(s, 'print'); r.style.transform = `scale(${k})`; r.style.transformOrigin = '0 0';
    pg.append(r); box.append(pg);
  }
  return { w, h };
};
D.THEMES = THEMES; D.isEmpty = isEmpty;
H.register(D);
})();
