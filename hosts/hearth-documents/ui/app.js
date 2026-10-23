// Hearth Documents: reads PDFs (pdf.js, vendor/pdfjs), e-books (epub.js, vendor/epub) and comic books (CBZ pages
// from the app). Page thumbnails and the table of contents in the sidebar, continuous scrolling, zoom, find, go to
// page, turning pages, dark pages, print, recent documents and the last place in each. Mouse, touch, keyboard and
// controllers (standard mapping). In a plain browser it shows sample documents (?pdf=URL, ?epub=URL) for checking.
'use strict';

const T = window.__TAURI__;
const invoke = T ? T.core.invoke : demoInvoke;
const MEDIA = T ? T.core.convertFileSrc('', 'media') : '';
const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const human = b => { if (b == null) return ''; const u = ['bytes', 'KB', 'MB', 'GB']; let v = b, i = 0; while (v >= 1000 && i < u.length - 1) { v /= 1000; i++; } return (i ? v.toFixed(v < 10 ? 1 : 0) : v) + ' ' + u[i]; };
const kindOf = p => { const m = /\.(pdf|epub|cbz)$/i.exec(p || ''); return m ? m[1].toLowerCase() : null; };
const baseName = p => String(p || '').split(/[\\/]/).pop();
const sleep = ms => new Promise(r => setTimeout(r, ms));

const S = {
  kind: null, path: null, name: '', state: { recent: [], settings: {} }, settings: {},
  sideOpen: false, sideTab: 'pages', fs: false, pad: false, selftest: false,
  // PDF
  pdf: null, viewer: null, bus: null, links: null, finder: null, thumbsDone: new Set(),
  // EPUB
  book: null, rendition: null, toc: [], loc: null,
  // CBZ
  cbz: null,
};

// ---------------------------------------------------------------- saving where you were
let saveTimer = 0;
function rememberSoon(fields) {
  if (!S.path) return;
  Object.assign(S.pending || (S.pending = {}), fields);
  clearTimeout(saveTimer);
  saveTimer = setTimeout(flushRemember, 700);
}
function flushRemember() {
  clearTimeout(saveTimer);
  if (!S.path || !S.pending) return Promise.resolve();
  const f = S.pending; S.pending = null;
  return invoke('remember', { path: S.path, fields: f }).catch(() => {});
}
window.addEventListener('beforeunload', flushRemember);
function setting(k, v) { S.settings[k] = v; invoke('settings', { value: { [k]: v } }).catch(() => {}); }
const entry = path => (S.state.recent || []).find(e => e.path === path) || {};

// ---------------------------------------------------------------- opening
async function openDoc(path) {
  const kind = kindOf(path);
  if (!kind) { toast('Documents opens PDF, EPUB and CBZ files'); return; }
  await closeDoc();
  S.kind = kind; S.path = path; S.name = baseName(path);
  $('#app').classList.remove('home');
  $('#app').dataset.kind = kind;
  for (const b of $$('[data-for]')) b.hidden = !b.dataset.for.split(' ').includes(kind);
  $('#viewerContainer').hidden = kind !== 'pdf';
  $('#epubWrap').hidden = kind !== 'epub';
  $('#cbzWrap').hidden = kind !== 'cbz';
  $('[data-stab="pages"]').hidden = kind === 'epub';
  $('[data-stab="outline"]').hidden = kind === 'cbz';
  S.sideTab = kind === 'epub' ? 'outline' : kind === 'cbz' ? 'pages' : (S.settings.sideTab || 'pages');
  setSide(S.settings.side ?? (kind === 'pdf'), false);
  $('#name').textContent = S.name;
  $('#sub').textContent = '';
  setMsg('');
  loading('Opening ' + S.name + '…');
  invoke('set_title', { title: `${S.name} – Documents` }).catch(() => {});
  try {
    if (kind === 'pdf') await openPdf(path);
    else if (kind === 'epub') await openEpub(path);
    else await openCbz(path);
    rememberSoon({ kind });
  } catch (e) {
    console.error(e);
    setMsg('Documents can\'t open this file', String(e && e.message || e));
  }
  loading(false);
}

async function closeDoc() {
  await flushRemember();
  if (S.pdf) { try { S.viewer.setDocument(null); S.links.setDocument(null); } catch (_) {} try { await S.pdf.destroy(); } catch (_) {} S.pdf = null; }
  if (S.book) { try { S.book.destroy(); } catch (_) {} S.book = null; S.rendition = null; $('#epubArea').innerHTML = ''; }
  S.cbz = null; $('#cbzPages').innerHTML = '';
  $('#sidePages').innerHTML = ''; $('#sideOutline').innerHTML = ''; $('#sideResults').innerHTML = '';
  $('[data-stab="results"]').hidden = true;
  S.thumbsDone = new Set();
  $('#findbar').hidden = true;
  S.path = null;
}

async function readBytes(path) {
  const r = await invoke('read_file', { path });
  return r instanceof ArrayBuffer ? r : new Uint8Array(r).buffer;
}

function loading(text) { $('#loading').hidden = !text; if (text) $('#loadingText').textContent = text; }
function setMsg(title, text) { const m = $('#msg'); if (!title) { m.hidden = true; return; } m.hidden = false; m.innerHTML = `<div><b>${esc(title)}</b>${esc(text || '')}</div>`; }

// ---------------------------------------------------------------- PDF (pdf.js)
let pdfjsLib = null, pdfjsViewer = null;
async function loadPdfJs() {
  if (pdfjsLib) return;
  pdfjsLib = await import('./vendor/pdfjs/pdf.min.mjs');
  globalThis.pdfjsLib = pdfjsLib; // pdf_viewer.mjs reads it from there
  pdfjsLib.GlobalWorkerOptions.workerSrc = new URL('vendor/pdfjs/pdf.worker.min.mjs', location.href).href;
  pdfjsViewer = await import('./vendor/pdfjs/pdf_viewer.mjs');
}

async function setupPdfViewer() {
  if (S.viewer) return;
  const container = $('#viewerContainer');
  S.bus = new pdfjsViewer.EventBus();
  S.links = new pdfjsViewer.PDFLinkService({ eventBus: S.bus });
  S.finder = new pdfjsViewer.PDFFindController({ eventBus: S.bus, linkService: S.links });
  S.viewer = new pdfjsViewer.PDFViewer({ container, viewer: $('#viewer'), eventBus: S.bus, linkService: S.links, findController: S.finder, textLayerMode: 1, removePageBorders: true });
  S.links.setViewer(S.viewer);
  S.bus.on('pagechanging', e => { updatePage(); markThumb(e.pageNumber); rememberSoon({ page: e.pageNumber, pages: S.pdf ? S.pdf.numPages : 0 }); });
  S.bus.on('scalechanging', () => { updateZoom(); rememberSoon({ zoom: S.viewer.currentScaleValue }); });
  S.bus.on('rotationchanging', e => rememberSoon({ rotation: e.pagesRotation }));
  S.bus.on('updatefindmatchescount', e => showFindCount(e.matchesCount));
  S.bus.on('updatefindcontrolstate', e => { if (e.matchesCount) showFindCount(e.matchesCount, e.state); });
}

async function openPdf(path) {
  await loadPdfJs();
  await setupPdfViewer();
  const data = await readBytes(path);
  const task = pdfjsLib.getDocument({
    data, cMapUrl: 'vendor/pdfjs/cmaps/', cMapPacked: true, standardFontDataUrl: 'vendor/pdfjs/standard_fonts/', isEvalSupported: false,
  });
  task.onPassword = async (updatePassword, reason) => {
    const pw = await askText(reason === 2 ? 'That password isn\'t right. Try again' : 'This PDF is locked', 'Password', true);
    if (pw == null) { task.destroy(); return; }
    updatePassword(pw);
  };
  const pdf = await task.promise;
  S.pdf = pdf;
  const saved = entry(path);
  const ready = new Promise(r => S.bus.on('pagesinit', r, { once: true }));
  S.viewer.setDocument(pdf);
  S.links.setDocument(pdf, null);
  await ready;
  S.viewer.currentScaleValue = saved.zoom || S.settings.zoom || 'page-width';
  if (saved.rotation) S.viewer.pagesRotation = saved.rotation;
  if (saved.page && saved.page <= pdf.numPages) S.viewer.currentPageNumber = saved.page;
  if (saved.page > 1) toast(`Back to page ${saved.page}`);
  updatePage(); updateZoom();
  const meta = await pdf.getMetadata().catch(() => null);
  const title = meta && meta.info && meta.info.Title;
  $('#sub').textContent = [title && title !== S.name ? title : '', `${pdf.numPages} pages`].filter(Boolean).join('  ·  ');
  buildPdfThumbs();
  buildPdfOutline();
  $('#viewerContainer').focus();
}

function updatePage() {
  if (S.kind === 'pdf' && S.pdf) {
    const n = S.viewer.currentPageNumber, t = S.pdf.numPages;
    $('#pageLabel').textContent = `${n} / ${t}`;
    $('#progress').textContent = `Page ${n} of ${t}`;
  } else if (S.kind === 'cbz' && S.cbz) {
    $('#pageLabel').textContent = `${S.cbz.page} / ${S.cbz.pages.length}`;
    $('#progress').textContent = `Page ${S.cbz.page} of ${S.cbz.pages.length}`;
  }
}
function updateZoom() {
  if (S.kind === 'pdf' && S.viewer) {
    const v = S.viewer.currentScaleValue;
    $('#zoomLabel').textContent = v === 'page-width' ? 'Width' : v === 'page-fit' ? 'Page' : Math.round(S.viewer.currentScale * 100) + '%';
  } else if (S.kind === 'cbz' && S.cbz) {
    $('#zoomLabel').textContent = S.cbz.zoom === 'width' ? 'Width' : S.cbz.zoom === 'page' ? 'Page' : Math.round(S.cbz.zoom * 100) + '%';
  }
}

// Page thumbnails: drawn when they scroll into the sidebar, one at a time.
let thumbQueue = Promise.resolve(), thumbObserver = null;
function buildPdfThumbs() {
  const box = $('#sidePages');
  const n = S.pdf.numPages;
  let h = '';
  for (let i = 1; i <= n; i++) h += `<button class="thumb" data-f data-page="${i}"><span class="tw" style="aspect-ratio:0.707"></span>${i}</button>`;
  box.innerHTML = h;
  if (thumbObserver) thumbObserver.disconnect();
  const pdf = S.pdf;
  thumbObserver = new IntersectionObserver(es => {
    for (const e of es) {
      if (!e.isIntersecting) continue;
      const i = +e.target.dataset.page;
      if (S.thumbsDone.has(i)) continue;
      S.thumbsDone.add(i);
      thumbQueue = thumbQueue.then(async () => {
        if (S.pdf !== pdf) return;
        const page = await pdf.getPage(i);
        const vp1 = page.getViewport({ scale: 1, rotation: (page.rotate + (S.viewer.pagesRotation || 0)) % 360 });
        const scale = 300 / vp1.width;
        const vp = page.getViewport({ scale, rotation: vp1.rotation });
        const cv = document.createElement('canvas');
        cv.width = Math.ceil(vp.width); cv.height = Math.ceil(vp.height);
        await page.render({ canvasContext: cv.getContext('2d'), viewport: vp }).promise;
        const tw = $('.tw', e.target); tw.style.aspectRatio = ''; tw.innerHTML = ''; tw.appendChild(cv);
      }).catch(() => {});
    }
  }, { root: box, rootMargin: '400px 0px' });
  $$('.thumb', box).forEach(t => thumbObserver.observe(t));
  markThumb(S.viewer.currentPageNumber);
}
function markThumb(n) {
  const box = $('#sidePages');
  $$('.thumb.cur', box).forEach(t => t.classList.remove('cur'));
  const t = $(`.thumb[data-page="${n}"]`, box);
  if (t) { t.classList.add('cur'); if (S.sideOpen && !box.contains(document.activeElement)) t.scrollIntoView({ block: 'nearest' }); }
}

async function buildPdfOutline() {
  const outline = await S.pdf.getOutline().catch(() => null);
  const box = $('#sideOutline');
  if (!outline || !outline.length) { box.innerHTML = '<p class="empty">This document has no table of contents.</p>'; if (S.sideTab === 'outline') setSideTab('pages'); return; }
  S.outline = outline;
  let k = 0;
  const flat = [];
  const tree = items => items.map(it => {
    const id = k++; flat[id] = it;
    const kids = it.items && it.items.length;
    return `<div class="oitem">${kids ? `<button class="otog" data-otog="${id}" tabindex="-1"><svg viewBox="0 0 24 24"><path d="m9 6 6 6-6 6"/></svg></button>` : '<span style="width:30px;flex:none"></span>'}<button class="olink" data-f data-o="${id}">${esc(it.title)}</button></div>${kids ? `<div class="okids" data-okids="${id}" hidden>${tree(it.items)}</div>` : ''}`;
  }).join('');
  box.innerHTML = tree(outline);
  S.outlineFlat = flat;
}
function goOutline(id) {
  const it = S.outlineFlat[id];
  if (!it) return;
  if (it.url) { openUrl(it.url); return; }
  if (it.dest) S.links.goToDestination(it.dest);
  if (window.innerWidth < 760) setSide(false);
}

// Links inside the PDF: web links open in the browser; nothing navigates this window.
document.addEventListener('click', e => {
  const a = e.target.closest('a[href]');
  if (!a) return;
  const href = a.getAttribute('href');
  if (/^(https?:|mailto:)/i.test(href)) { e.preventDefault(); e.stopPropagation(); openUrl(href); }
  else if (!href.startsWith('#')) e.preventDefault();
}, true);
function openUrl(u) { invoke('open_url', { url: u }).then(() => toast('Opening the link…')).catch(e => toast(String(e))); }

// ---------------------------------------------------------------- EPUB (epub.js)
const THEMES = {
  light: { bg: '#fbf7f0', fg: '#2b211b', link: '#c2551b' },
  sepia: { bg: '#f1e4c9', fg: '#4a3626', link: '#9c4a16' },
  dark: { bg: '#141010', fg: '#e8dccd', link: '#ff8a3d' },
};
async function openEpub(path) {
  if (!window.ePub) throw new Error('The e-book reader (epub.js) is missing');
  const data = await readBytes(path);
  const book = window.ePub(data);
  S.book = book;
  await book.ready;
  const area = $('#epubArea');
  area.innerHTML = '';
  const r = book.renderTo(area, { width: '100%', height: '100%', flow: 'paginated', spread: 'auto', minSpreadWidth: 1000, allowScriptedContent: false });
  S.rendition = r;
  for (const [name, t] of Object.entries(THEMES)) {
    r.themes.register(name, {
      'html, body': { background: `${t.bg} !important`, color: `${t.fg} !important` },
      'body': { 'font-family': '"Noto Serif", Georgia, "Times New Roman", serif !important', 'line-height': '1.6 !important' },
      'a, a:link, a:visited': { color: `${t.link} !important` },
      '::selection': { background: 'rgba(255,138,61,.35)' },
      '.hearth-hit': { background: 'rgba(255,209,102,.6)' },
    });
  }
  applyTheme(S.settings.theme || 'light', false);
  r.themes.fontSize((S.settings.fontSize || 100) + '%');
  r.on('relocated', loc => {
    S.loc = loc;
    const cfi = loc.start.cfi;
    let pct = book.locations.length() ? Math.round(book.locations.percentageFromCfi(cfi) * 100) : null;
    const chapter = chapterOf(loc.start.href);
    $('#pageLabel').textContent = pct != null ? `${pct}%` : 'Page';
    $('#progress').textContent = [chapter, loc.start.displayed ? `page ${loc.start.displayed.page} of ${loc.start.displayed.total} in this chapter` : ''].filter(Boolean).join(' · ');
    rememberSoon({ cfi, progress: pct != null ? pct / 100 : undefined });
    markToc(loc.start.href);
  });
  // Keys and swipes inside the book's pages come here.
  r.on('keydown', e => onKey(e));
  let tx = null;
  r.on('touchstart', e => { tx = e.changedTouches[0].screenX; });
  r.on('touchend', e => { if (tx == null) return; const dx = e.changedTouches[0].screenX - tx; tx = null; if (Math.abs(dx) > 60) dx < 0 ? r.next() : r.prev(); });
  r.on('click', () => { if (!$('#menu').hidden) closeMenu(); });
  const saved = entry(path);
  await r.display(saved.cfi || undefined);
  if (saved.cfi) toast('Back where you were');
  const nav = await book.loaded.navigation;
  S.toc = nav.toc || [];
  buildToc();
  const meta = await book.loaded.metadata.catch(() => ({}));
  $('#sub').textContent = [meta.title && meta.title !== S.name ? meta.title : '', meta.creator].filter(Boolean).join('  ·  ');
  // Percentages need the book's locations: worked out in the background.
  book.locations.generate(1200).then(() => { if (S.book === book && S.loc) r.emit('relocated', S.loc); }).catch(() => {});
}
function applyTheme(name, save = true) {
  const t = THEMES[name] || THEMES.light;
  S.theme = name;
  if (S.rendition) S.rendition.themes.select(name);
  const w = $('#epubWrap');
  w.style.setProperty('--epub-bg', t.bg); w.style.setProperty('--epub-fg', t.fg);
  if (save) { setting('theme', name); toast({ light: 'Light', sepia: 'Sepia', dark: 'Dark' }[name]); }
}
function fontSize(d) {
  const v = Math.max(60, Math.min(250, (S.settings.fontSize || 100) + d));
  setting('fontSize', v);
  if (S.rendition) S.rendition.themes.fontSize(v + '%');
  toast(`Text size ${v}%`);
}
function chapterOf(href) {
  const clean = h => String(h || '').split('#')[0];
  let found = '';
  const walk = items => { for (const it of items) { if (clean(it.href) === clean(href) || clean(href).endsWith(clean(it.href))) found = it.label.trim(); if (it.subitems) walk(it.subitems); } };
  walk(S.toc || []);
  return found;
}
function buildToc() {
  const box = $('#sideOutline');
  if (!S.toc.length) { box.innerHTML = '<p class="empty">This book has no table of contents.</p>'; return; }
  let k = 0;
  const flat = [];
  const tree = items => items.map(it => {
    const id = k++; flat[id] = it;
    const kids = it.subitems && it.subitems.length;
    return `<div class="oitem">${kids ? `<button class="otog open" data-otog="${id}" tabindex="-1"><svg viewBox="0 0 24 24"><path d="m9 6 6 6-6 6"/></svg></button>` : '<span style="width:30px;flex:none"></span>'}<button class="olink" data-f data-toc="${id}" data-href="${esc(it.href)}">${esc(it.label.trim())}</button></div>${kids ? `<div class="okids" data-okids="${id}">${tree(it.subitems)}</div>` : ''}`;
  }).join('');
  box.innerHTML = tree(S.toc);
  S.tocFlat = flat;
}
function markToc(href) {
  const clean = String(href || '').split('#')[0];
  $$('#sideOutline .olink.cur').forEach(b => b.classList.remove('cur'));
  const b = $$('#sideOutline [data-href]').find(x => x.dataset.href.split('#')[0] === clean || clean.endsWith(x.dataset.href.split('#')[0]));
  if (b) b.classList.add('cur');
}

// Find in a book: every chapter is searched; the results are listed in the sidebar.
async function epubSearch(q) {
  const book = S.book;
  if (!book || !q) return [];
  const results = [];
  for (const item of book.spine.spineItems) {
    try {
      await item.load(book.load.bind(book));
      const found = item.find(q) || [];
      for (const f of found) results.push({ ...f, chapter: chapterOf(item.href) });
    } catch (_) {} finally { try { item.unload(); } catch (_) {} }
    if (S.book !== book) return [];
  }
  return results;
}

// ---------------------------------------------------------------- CBZ (pages from the app)
async function openCbz(path) {
  const pages = await invoke('cbz_pages', { path });
  S.cbz = { pages, page: 1, zoom: S.settings.cbzZoom || 'page' };
  const saved = entry(path);
  const box = $('#cbzPages');
  box.innerHTML = pages.map((p, i) => `<img data-i="${i}" alt="Page ${i + 1}" loading="${i < 3 ? 'eager' : 'lazy'}" decoding="async" src="${cbzUrl(path, i)}" style="width:600px;height:900px">`).join('');
  const side = $('#sidePages');
  side.innerHTML = pages.map((p, i) => `<button class="thumb" data-f data-page="${i + 1}"><span class="tw"><img loading="lazy" decoding="async" src="${cbzUrl(path, i)}" alt=""></span>${i + 1}</button>`).join('');
  for (const im of $$('img', box)) im.addEventListener('load', () => { im.dataset.w = im.naturalWidth; im.dataset.h = im.naturalHeight; cbzLayout(); }, { once: true });
  cbzLayout();
  $('#sub').textContent = `${pages.length} pages`;
  if (saved.page > 1) { await sleep(50); cbzGo(saved.page, false); toast(`Back to page ${saved.page}`); }
  updatePage(); updateZoom();
  $('#cbzWrap').focus();
}
function cbzUrl(path, i) { return T ? `${MEDIA}cbz/${encodeURIComponent(path)}?i=${i}` : (window.DOCS_DEMO && window.DOCS_DEMO.cbz ? window.DOCS_DEMO.cbz(i) : ''); }
function cbzLayout() {
  if (!S.cbz) return;
  const wrap = $('#cbzWrap'), W = wrap.clientWidth - 28, H = wrap.clientHeight - 28;
  for (const im of $$('#cbzPages img')) {
    const w = +im.dataset.w || 600, h = +im.dataset.h || 900;
    const s = S.cbz.zoom === 'width' ? W / w : S.cbz.zoom === 'page' ? Math.min(W / w, H / h) : S.cbz.zoom;
    im.style.width = Math.round(w * s) + 'px'; im.style.height = Math.round(h * s) + 'px';
  }
}
function cbzGo(n, smooth = true) {
  if (!S.cbz) return;
  n = Math.max(1, Math.min(S.cbz.pages.length, n));
  const im = $(`#cbzPages img[data-i="${n - 1}"]`);
  if (im) $('#cbzWrap').scrollTo({ top: im.offsetTop - 14, behavior: smooth ? 'smooth' : 'auto' });
}
$('#cbzWrap').addEventListener('scroll', () => {
  if (!S.cbz) return;
  const wrap = $('#cbzWrap'), mid = wrap.scrollTop + wrap.clientHeight / 3;
  let n = 1;
  for (const im of $$('#cbzPages img')) { if (im.offsetTop <= mid) n = +im.dataset.i + 1; else break; }
  if (n !== S.cbz.page) { S.cbz.page = n; updatePage(); markThumb(n); rememberSoon({ page: n, pages: S.cbz.pages.length }); }
}, { passive: true });
window.addEventListener('resize', () => cbzLayout());

// ---------------------------------------------------------------- moving around (all kinds)
function pageStep(d) {
  if (S.kind === 'pdf' && S.viewer) d > 0 ? S.viewer.nextPage() : S.viewer.previousPage();
  else if (S.kind === 'epub' && S.rendition) d > 0 ? S.rendition.next() : S.rendition.prev();
  else if (S.kind === 'cbz' && S.cbz) cbzGo(S.cbz.page + d);
}
function goPage(n) {
  if (S.kind === 'pdf' && S.pdf) S.viewer.currentPageNumber = Math.max(1, Math.min(S.pdf.numPages, n));
  else if (S.kind === 'cbz') cbzGo(n);
}
function scrollBy(dy) {
  const el = S.kind === 'pdf' ? $('#viewerContainer') : S.kind === 'cbz' ? $('#cbzWrap') : null;
  if (el) el.scrollBy({ top: dy });
}
function zoom(step) {
  if (S.kind === 'pdf' && S.viewer) {
    if (step === 'cycle') {
      const order = ['page-width', 'page-fit', '1'];
      const now = String(S.viewer.currentScaleValue);
      S.viewer.currentScaleValue = order[(order.indexOf(now) + 1) % order.length] || 'page-width';
    } else {
      const s = Math.max(0.25, Math.min(8, S.viewer.currentScale * (step > 0 ? 1.15 : 1 / 1.15)));
      S.viewer.currentScale = s;
    }
    setting('zoom', String(S.viewer.currentScaleValue));
  } else if (S.kind === 'cbz' && S.cbz) {
    const page = S.cbz.page;
    if (step === 'cycle') { const order = ['page', 'width', 1]; S.cbz.zoom = order[(order.indexOf(S.cbz.zoom) + 1) % order.length]; }
    else { const cur = $('#cbzPages img'); const nowS = cur ? cur.clientWidth / (+cur.dataset.w || 600) : 1; S.cbz.zoom = Math.max(0.2, Math.min(5, nowS * (step > 0 ? 1.15 : 1 / 1.15))); }
    setting('cbzZoom', typeof S.cbz.zoom === 'string' ? S.cbz.zoom : 'page');
    cbzLayout(); cbzGo(page, false); updateZoom();
  } else if (S.kind === 'epub') fontSize(step > 0 ? 10 : -10);
}
// Ctrl+wheel (and touchpad pinch) zooms.
document.addEventListener('wheel', e => { if (e.ctrlKey && (S.kind === 'pdf' || S.kind === 'cbz')) { e.preventDefault(); zoom(e.deltaY < 0 ? 1 : -1); } }, { passive: false });
// Two-finger pinch on touch screens (PDF and comics).
(function pinch() {
  const pts = new Map(); let start = null;
  const el = () => S.kind === 'pdf' ? $('#viewerContainer') : $('#cbzWrap');
  for (const id of ['viewerContainer', 'cbzWrap']) {
    const box = document.getElementById(id);
    box.addEventListener('touchstart', e => { if (e.touches.length === 2) { const [a, b] = e.touches; start = { d: Math.hypot(a.clientX - b.clientX, a.clientY - b.clientY), s: S.kind === 'pdf' && S.viewer ? S.viewer.currentScale : 1 }; } }, { passive: true });
    box.addEventListener('touchmove', e => {
      if (!start || e.touches.length !== 2) return;
      const [a, b] = e.touches, d = Math.hypot(a.clientX - b.clientX, a.clientY - b.clientY);
      if (S.kind === 'pdf' && S.viewer) S.viewer.currentScale = Math.max(0.25, Math.min(8, start.s * d / start.d));
      e.preventDefault();
    }, { passive: false });
    box.addEventListener('touchend', () => { start = null; });
  }
  void pts; void el;
})();

function rotate() { if (S.viewer) { S.viewer.pagesRotation = (S.viewer.pagesRotation + 90) % 360; S.thumbsDone = new Set(); buildPdfThumbs(); } }
function toggleDark() {
  const on = !document.body.classList.contains('darkPages');
  document.body.classList.toggle('darkPages', on);
  $('#darkBtn').classList.toggle('on', on);
  setting('darkPages', on);
}

// ---------------------------------------------------------------- sidebar
function setSide(open, save = true) {
  S.sideOpen = open;
  $('#app').classList.toggle('sideOpen', open);
  $('#sideBtn').classList.toggle('on', open);
  setSideTab(S.sideTab);
  if (save) setting('side', open);
  setTimeout(() => { if (S.viewer && S.kind === 'pdf') { const v = S.viewer.currentScaleValue; S.viewer.currentScaleValue = v; } cbzLayout(); if (S.rendition) try { S.rendition.resize(); } catch (_) {} }, 260);
}
function setSideTab(t) {
  S.sideTab = t;
  for (const b of $$('.stab')) b.classList.toggle('on', b.dataset.stab === t);
  $('#sidePages').hidden = t !== 'pages';
  $('#sideOutline').hidden = t !== 'outline';
  $('#sideResults').hidden = t !== 'results';
  if (t !== 'results' && S.kind === 'pdf') setting('sideTab', t);
}
$('#side').addEventListener('click', e => {
  const st = e.target.closest('[data-stab]'); if (st) { setSideTab(st.dataset.stab); return; }
  const tog = e.target.closest('[data-otog]');
  if (tog) { const k = $(`[data-okids="${tog.dataset.otog}"]`); k.hidden = !k.hidden; tog.classList.toggle('open', !k.hidden); return; }
  const o = e.target.closest('[data-o]'); if (o) { goOutline(+o.dataset.o); return; }
  const tc = e.target.closest('[data-toc]'); if (tc) { S.rendition.display(S.tocFlat[+tc.dataset.toc].href); if (innerWidth < 760) setSide(false); return; }
  const th = e.target.closest('[data-page]'); if (th) { goPage(+th.dataset.page); return; }
  const r = e.target.closest('[data-cfi]'); if (r) { S.rendition.display(r.dataset.cfi); return; }
});

// ---------------------------------------------------------------- find
function openFind() {
  if (S.kind !== 'pdf' && S.kind !== 'epub') return;
  $('#findbar').hidden = false;
  const i = $('#findIn');
  const sel = String(window.getSelection() || '').trim();
  if (sel && sel.length < 80) i.value = sel;
  i.focus(); i.select();
}
function closeFind() {
  $('#findbar').hidden = true;
  if (S.kind === 'pdf' && S.bus) S.bus.dispatch('find', { source: null, type: '', query: '', caseSensitive: false, entireWord: false, highlightAll: false, findPrevious: false, matchDiacritics: false });
  if (S.kind === 'epub' && S.rendition && S.hits) { for (const h of S.hits) try { S.rendition.annotations.remove(h.cfi, 'highlight'); } catch (_) {} S.hits = null; }
  docFocus();
}
let lastQuery = '';
async function findGo(prev = false) {
  const q = $('#findIn').value.trim();
  if (!q) { $('#findCount').textContent = ''; return; }
  if (S.kind === 'pdf') {
    S.bus.dispatch('find', { source: null, type: q === lastQuery ? 'again' : '', query: q, caseSensitive: false, entireWord: false, highlightAll: true, findPrevious: prev, matchDiacritics: false });
    lastQuery = q;
  } else if (S.kind === 'epub') {
    if (q !== lastQuery || !S.hits) {
      lastQuery = q;
      $('#findCount').textContent = '…';
      if (S.hits) for (const h of S.hits) try { S.rendition.annotations.remove(h.cfi, 'highlight'); } catch (_) {}
      S.hits = await epubSearch(q);
      S.hitAt = -1;
      for (const h of S.hits) try { S.rendition.annotations.highlight(h.cfi, {}, null, 'hearth-hit', { fill: '#ffd166', 'fill-opacity': '0.5' }); } catch (_) {}
      const box = $('#sideResults');
      box.innerHTML = S.hits.length ? S.hits.map((h, k) => `<button class="result" data-f data-cfi="${esc(h.cfi)}" data-hit="${k}">${h.chapter ? `<div style="color:var(--faint);font-size:.85em">${esc(h.chapter)}</div>` : ''}${esc(h.excerpt).replace(new RegExp(q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'ig'), m => `<b>${m}</b>`)}</button>`).join('') : '<p class="empty">Nothing found.</p>';
      $('[data-stab="results"]').hidden = false;
      setSideTab('results');
      if (!S.sideOpen) setSide(true, false);
    }
    if (!S.hits.length) { $('#findCount').textContent = 'None'; return; }
    S.hitAt = (S.hitAt + (prev ? -1 : 1) + S.hits.length) % S.hits.length;
    $('#findCount').textContent = `${S.hitAt + 1} of ${S.hits.length}`;
    S.rendition.display(S.hits[S.hitAt].cfi);
  }
}
function showFindCount(m, state) {
  if (!m) return;
  $('#findCount').textContent = m.total ? `${m.current || 0} of ${m.total}` : (state === 1 ? 'None' : '…');
  S.lastMatches = m;
}
$('#findIn').addEventListener('keydown', e => {
  e.stopPropagation();
  if (e.key === 'Enter') { e.preventDefault(); findGo(e.shiftKey); }
  else if (e.key === 'Escape') { e.preventDefault(); closeFind(); }
  else if (e.key === 'F3') { e.preventDefault(); findGo(e.shiftKey); }
});
$('#findIn').addEventListener('input', () => { if (S.kind === 'pdf') { clearTimeout(S.findT); S.findT = setTimeout(() => findGo(false), 250); } });

// ---------------------------------------------------------------- go to page
async function gotoDialog() {
  if (S.kind === 'epub') {
    if (!S.book || !S.book.locations.length()) { toast('Still working out the pages of this book…'); return; }
    const v = await askText('Go to a place in the book', 'Percent (0 to 100)');
    const n = parseFloat(v);
    if (!isNaN(n)) S.rendition.display(S.book.locations.cfiFromPercentage(Math.max(0, Math.min(100, n)) / 100));
    return;
  }
  const total = S.kind === 'pdf' ? (S.pdf ? S.pdf.numPages : 0) : (S.cbz ? S.cbz.pages.length : 0);
  if (!total) return;
  const v = await askText(`Go to page (1 to ${total})`, 'Page number');
  const n = parseInt(v, 10);
  if (n >= 1) goPage(n);
}

// ---------------------------------------------------------------- printing
async function printDoc() {
  if (S.kind === 'epub') { toast('Printing e-books isn\'t available yet'); return; }
  const box = $('#print');
  box.innerHTML = '';
  let cancelled = false;
  const v = document.createElement('div');
  v.className = 'veil';
  v.innerHTML = `<div class="dlg"><h2>Getting the pages ready to print</h2><p id="prText">Page 0</p><div class="prog"><i id="prBar"></i></div><div class="row"><button class="btn" data-f id="prCancel">Cancel</button></div></div>`;
  v._back = () => { cancelled = true; };
  $('#layer').appendChild(v);
  $('#prCancel').onclick = () => { cancelled = true; };
  $('#prCancel').focus();
  try {
    if (S.kind === 'pdf') {
      const n = S.pdf.numPages;
      for (let i = 1; i <= n && !cancelled; i++) {
        $('#prText').textContent = `Page ${i} of ${n}`; $('#prBar').style.width = (i / n * 100) + '%';
        const page = await S.pdf.getPage(i);
        const vp = page.getViewport({ scale: 150 / 72, rotation: (page.rotate + S.viewer.pagesRotation) % 360 });
        const cv = document.createElement('canvas');
        cv.width = Math.ceil(vp.width); cv.height = Math.ceil(vp.height);
        const g = cv.getContext('2d'); g.fillStyle = '#fff'; g.fillRect(0, 0, cv.width, cv.height);
        await page.render({ canvasContext: g, viewport: vp, intent: 'print' }).promise;
        const blob = await new Promise(r => cv.toBlob(r, 'image/jpeg', 0.92));
        const im = new Image(); im.src = URL.createObjectURL(blob); box.appendChild(im);
      }
    } else if (S.kind === 'cbz') {
      S.cbz.pages.forEach((p, i) => { const im = new Image(); im.src = cbzUrl(S.path, i); box.appendChild(im); });
      await Promise.all($$('img', box).map(im => im.decode().catch(() => {})));
    }
    v.remove();
    if (cancelled) { box.innerHTML = ''; return; }
    await sleep(100);
    T ? await invoke('print') : window.print();
  } catch (e) { v.remove(); toast('Couldn\'t print: ' + e); }
}
window.addEventListener('afterprint', () => { for (const im of $$('#print img')) if (im.src.startsWith('blob:')) URL.revokeObjectURL(im.src); $('#print').innerHTML = ''; });

// ---------------------------------------------------------------- home: recent documents
async function showHome() {
  await closeDoc();
  S.kind = null;
  $('#app').classList.add('home');
  $('#app').dataset.kind = '';
  $('#name').textContent = 'Documents';
  $('#sub').textContent = '';
  $('#progress').textContent = '';
  invoke('set_title', { title: 'Documents' }).catch(() => {});
  try { S.state = await invoke('recent'); } catch (_) {}
  renderRecent();
  const first = $('#recent .rdoc:not(.gone)') || $('[data-act="open"]');
  if (first) first.focus();
}
function renderRecent() {
  const list = (S.state.recent || []).filter(e => kindOf(e.path));
  $('#recentTitle').hidden = !list.length;
  $('#recent').innerHTML = list.length ? list.map(e => {
    const k = kindOf(e.path);
    const pct = e.progress != null ? e.progress : (e.page && e.pages ? e.page / e.pages : null);
    const where = k === 'epub' ? (pct != null ? `${Math.round(pct * 100)}% read` : '') : e.page ? `Page ${e.page}${e.pages ? ` of ${e.pages}` : ''}` : '';
    const folder = String(e.path).replace(/[\\/][^\\/]*$/, '');
    return `<button class="rdoc${e.there === false ? ' gone' : ''}" data-f data-recent="${esc(e.path)}" title="${esc(e.path)}"><span class="ri">${k.toUpperCase()}</span>
      <span class="rn">${esc(e.name || baseName(e.path))}</span><span class="rm">${e.there === false ? 'Not there any more' : esc([where, folder].filter(Boolean).join('  ·  '))}</span>
      ${pct != null ? `<span class="bar"><i style="width:${Math.round(pct * 100)}%"></i></span>` : ''}</button>`;
  }).join('') : '<p class="empty">Documents you open show up here, and open where you left off.</p>';
}
$('#recent').addEventListener('click', e => {
  const b = e.target.closest('[data-recent]');
  if (!b) return;
  if (b.classList.contains('gone')) { invoke('forget', { path: b.dataset.recent }).then(showHome); toast('Taken off the list'); return; }
  openDoc(b.dataset.recent);
});

// ---------------------------------------------------------------- menus, dialogs, toasts
let menuReturn = null;
function openMenu(anchor, items) {
  const m = $('#menu');
  menuReturn = document.activeElement;
  m.innerHTML = items.map((it, k) => it.sep ? '<div class="msep"></div>' : it.title ? `<div class="mlabel">${esc(it.title)}</div>`
    : `<button class="mi${it.on ? ' on' : ''}" data-f data-mi="${k}" ${it.disabled ? 'disabled style="opacity:.45"' : ''}>${esc(it.label)}${it.key && !it.on ? `<span class="k">${esc(it.key)}</span>` : ''}</button>`).join('');
  m.hidden = false;
  const r = anchor.getBoundingClientRect(), mw = m.offsetWidth, mh = m.offsetHeight;
  m.style.left = Math.max(10, Math.min(innerWidth - mw - 10, r.right - mw)) + 'px';
  m.style.top = Math.max(10, Math.min(innerHeight - mh - 10, r.bottom + 6)) + 'px';
  m.onclick = e => { const b = e.target.closest('[data-mi]'); if (!b || b.disabled) return; closeMenu(); items[+b.dataset.mi].run(); };
  const first = $('.mi:not([disabled])', m);
  if (first) { first.focus(); if (S.pad) first.classList.add('kf'); }
}
function closeMenu() { const m = $('#menu'); if (m.hidden) return false; m.hidden = true; if (menuReturn && document.contains(menuReturn)) menuReturn.focus(); return true; }
document.addEventListener('pointerdown', e => { if (!$('#menu').hidden && !e.target.closest('#menu')) closeMenu(); }, true);

function moreMenu() {
  const doc = !!S.kind;
  const items = [
    { label: 'Open a document…', key: 'Ctrl+O', run: pickDoc },
    { label: 'Recent documents', run: showHome, disabled: !doc },
  ];
  if (doc) {
    items.push({ sep: true });
    if (S.kind !== 'epub') items.push({ label: 'Print…', key: 'Ctrl+P', run: printDoc });
    items.push({ label: S.kind === 'epub' ? 'Go to a place…' : 'Go to page…', key: 'Ctrl+G', run: gotoDialog });
    items.push({ label: 'Show in its folder', run: () => invoke('show_in_folder', { path: S.path }).catch(e => toast(String(e))) });
    items.push({ label: 'About this document', run: aboutDoc });
    if (S.kind === 'epub') {
      items.push({ sep: true }, { title: 'Page colour' });
      for (const [k, l] of [['light', 'Light'], ['sepia', 'Sepia'], ['dark', 'Dark']]) items.push({ label: l, on: S.theme === k, run: () => applyTheme(k) });
      items.push({ title: 'Layout' }, { label: 'Two pages side by side when wide', on: S.settings.spread !== 'none', run: () => { setting('spread', S.settings.spread === 'none' ? 'auto' : 'none'); S.rendition.spread(S.settings.spread); } });
    }
  }
  items.push({ sep: true }, { label: 'Keyboard and controller', run: helpDialog });
  openMenu($('#moreBtn'), items);
}
async function aboutDoc() {
  const meta = await invoke('file_meta', { path: S.path }).catch(() => ({}));
  const rows = [['File', S.name], ['Size', human(meta.size)], ['Changed', meta.modified ? new Date(meta.modified).toLocaleString() : ''], ['Folder', meta.folder || '']];
  if (S.kind === 'pdf' && S.pdf) {
    const m = await S.pdf.getMetadata().catch(() => null);
    const i = (m && m.info) || {};
    rows.unshift(['Title', i.Title || ''], ['Author', i.Author || ''], ['Pages', String(S.pdf.numPages)], ['Made with', [i.Creator, i.Producer].filter(Boolean).join(', ')], ['PDF version', i.PDFFormatVersion || '']);
  } else if (S.kind === 'epub' && S.book) {
    const m = await S.book.loaded.metadata.catch(() => ({}));
    rows.unshift(['Title', m.title || ''], ['Author', m.creator || ''], ['Publisher', m.publisher || ''], ['Language', m.language || '']);
  }
  dialog(`<h2>About this document</h2><dl class="kv">${rows.filter(r => r[1]).map(([k, v]) => `<dt>${esc(k)}</dt><dd>${esc(v)}</dd>`).join('')}</dl>`, [{ label: 'Close', kind: 'primary', value: 1 }]);
}
function helpDialog() {
  dialog(`<h2>Keyboard and controller</h2><dl class="kv">
    <dt>← → , PgUp PgDn, D-pad</dt><dd>Previous / next page (LB / RB too)</dd>
    <dt>↑ ↓, sticks</dt><dd>Scroll</dd>
    <dt>+ − 0, LT / RT</dt><dd>Zoom (text size in e-books); 0 cycles fit width / fit page / 100%</dd>
    <dt>Ctrl+F, F3</dt><dd>Find</dd><dt>Ctrl+G</dt><dd>Go to page</dd>
    <dt>O or X</dt><dd>Pages and contents</dd><dt>R</dt><dd>Turn pages</dd><dt>D</dt><dd>Dark pages (dark e-book)</dd><dt>L, S</dt><dd>Light, sepia e-book</dd>
    <dt>F11 or View</dt><dd>Full screen</dd><dt>Ctrl+P, Ctrl+O</dt><dd>Print, open</dd>
    <dt>A</dt><dd>Use the tools at the top</dd><dt>B or Esc</dt><dd>Back</dd></dl>`, [{ label: 'Close', kind: 'primary', value: 1 }]);
}

function dialog(html, buttons, onOpen) {
  return new Promise(resolve => {
    const ret = document.activeElement;
    const v = document.createElement('div');
    v.className = 'veil';
    v.innerHTML = `<div class="dlg" role="dialog">${html}<div class="row">${buttons.map((b, k) => `<button class="btn ${b.kind || ''}" data-f data-b="${k}">${esc(b.label)}</button>`).join('')}</div></div>`;
    const done = val => { v.remove(); if (ret && document.contains(ret)) ret.focus(); resolve(val); };
    v._back = () => done(null);
    v._done = done;
    v.addEventListener('click', e => { const b = e.target.closest('[data-b]'); if (b) done(buttons[+b.dataset.b].value); else if (e.target === v) done(null); });
    $('#layer').appendChild(v);
    if (onOpen) onOpen(v, done);
    else { const f = $('[data-b]:last-child', v); if (f) f.focus(); }
  });
}
function askText(title, label, password = false) {
  let value = '';
  return dialog(`<h2>${esc(title)}</h2><p><input class="inp" style="width:100%" id="askIn" ${password ? 'type="password"' : ''} placeholder="${esc(label)}" autocomplete="off"></p>`,
    [{ label: 'Cancel', value: null }, { label: 'OK', kind: 'primary', value: 'ok' }], (v, done) => {
      const i = $('#askIn', v); i.focus();
      i.addEventListener('input', () => { value = i.value; });
      i.addEventListener('keydown', e => { e.stopPropagation(); if (e.key === 'Enter') done('ok'); if (e.key === 'Escape') done(null); });
    }).then(r => r === 'ok' ? value : null);
}
let toastTimer = 0;
function toast(text, ms = 2600) {
  const t = $('#toast');
  t.textContent = text; t.hidden = false;
  clearTimeout(toastTimer); toastTimer = setTimeout(() => { t.hidden = true; }, ms);
}

async function pickDoc() {
  try { const p = await invoke('pick', { from: S.path }); if (p) openDoc(p); } catch (e) { toast(String(e)); }
}
async function setFullscreen(on) {
  try { S.fs = T ? await invoke('fullscreen', { on }) : on; } catch (_) { S.fs = on; }
  $('#app').classList.toggle('fs', S.fs);
  idle();
  setTimeout(() => { if (S.viewer && S.kind === 'pdf') { const v = S.viewer.currentScaleValue; S.viewer.currentScaleValue = v; } cbzLayout(); if (S.rendition) try { S.rendition.resize(); } catch (_) {} }, 200);
}
let idleT = 0;
function idle() { $('#app').classList.remove('idle'); clearTimeout(idleT); if (S.fs) idleT = setTimeout(() => { if ($('#menu').hidden && !$('#layer').firstChild && $('#findbar').hidden && !document.activeElement?.closest('#top')) $('#app').classList.add('idle'); }, 2500); }
document.addEventListener('mousemove', idle);

const actions = {
  sidebar: () => setSide(!S.sideOpen), find: openFind, findNext: () => findGo(false), findPrev: () => findGo(true), findClose: closeFind,
  goto: gotoDialog, zoomIn: () => zoom(1), zoomOut: () => zoom(-1), zoomCycle: () => zoom('cycle'),
  smaller: () => fontSize(-10), bigger: () => fontSize(10), theme: () => applyTheme({ light: 'sepia', sepia: 'dark', dark: 'light' }[S.theme] || 'light'),
  rotate, dark: toggleDark, fullscreen: () => setFullscreen(!S.fs), more: moreMenu, open: pickDoc,
  prevPage: () => pageStep(-1), nextPage: () => pageStep(1),
};
document.addEventListener('click', e => {
  const b = e.target.closest('[data-act]');
  if (!b || b.disabled) return;
  const f = actions[b.dataset.act];
  if (f) { e.preventDefault(); f(); }
});

// ---------------------------------------------------------------- moving focus with arrows / D-pad
function topLayer() { const v = $('#layer .veil:last-child'); if (v) return v; if (!$('#menu').hidden) return $('#menu'); return $('#app'); }
function focusables(root) { return $$('[data-f]', root).filter(el => !el.disabled && !el.hidden && el.offsetParent !== null); }
function move(dir, root = topLayer()) {
  const list = focusables(root), a = document.activeElement;
  if (!list.length) return false;
  if (!a || !list.includes(a)) { list[0].focus(); list[0].classList.add('kf'); return true; }
  const r = a.getBoundingClientRect(), cx = r.left + r.width / 2, cy = r.top + r.height / 2;
  let best = null, bs = Infinity;
  for (const el of list) {
    if (el === a) continue;
    const q = el.getBoundingClientRect(), dx = q.left + q.width / 2 - cx, dy = q.top + q.height / 2 - cy;
    const ok = dir === 'left' ? dx < -4 : dir === 'right' ? dx > 4 : dir === 'up' ? dy < -4 : dy > 4;
    if (!ok) continue;
    const h = dir === 'left' || dir === 'right';
    const sc = (h ? Math.abs(dx) : Math.abs(dy)) + (h ? Math.abs(dy) : Math.abs(dx)) * 2.2;
    if (sc < bs) { bs = sc; best = el; }
  }
  if (best) { a.classList.remove('kf'); best.focus(); best.classList.add('kf'); best.scrollIntoView({ block: 'nearest', inline: 'nearest' }); return true; }
  return false;
}
document.addEventListener('focusout', e => e.target.classList?.remove('kf'));
const inUi = () => topLayer() !== $('#app') || !!document.activeElement?.closest('#top,#side,#home,#findbar');
function docFocus() { const el = S.kind === 'pdf' ? $('#viewerContainer') : S.kind === 'cbz' ? $('#cbzWrap') : null; if (el) el.focus(); else if (document.activeElement) document.activeElement.blur(); }

function back() {
  const v = $('#layer .veil:last-child');
  if (v) { v._back(); return; }
  if (closeMenu()) return;
  if (!$('#findbar').hidden) { closeFind(); return; }
  if (document.activeElement && document.activeElement.closest('#top,#side')) { docFocus(); return; }
  if (S.fs) { setFullscreen(false); return; }
  if (S.kind && S.sideOpen && innerWidth < 760) { setSide(false); return; }
}

// ---------------------------------------------------------------- keyboard
function onKey(e) {
  S.pad = false;
  const k = e.key, ctrl = e.ctrlKey || e.metaKey;
  const typing = e.target && /^(INPUT|TEXTAREA)$/.test(e.target.tagName);
  if (k === 'Escape') { e.preventDefault && e.preventDefault(); back(); return; }
  if (typing) return;
  if (ctrl) {
    const c = { f: openFind, g: gotoDialog, o: pickDoc, p: printDoc, '=': () => zoom(1), '+': () => zoom(1), '-': () => zoom(-1), '0': () => zoom('cycle'), w: () => invoke('quit').catch(() => {}), q: () => invoke('quit').catch(() => {}) }[k.toLowerCase()];
    if (c) { e.preventDefault && e.preventDefault(); c(); }
    return;
  }
  const dirs = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
  if (dirs[k] && inUi()) { e.preventDefault && e.preventDefault(); move(dirs[k]); return; }
  if (topLayer() !== $('#app') || !S.kind) return;
  const map = {
    ArrowLeft: () => pageStep(-1), ArrowRight: () => pageStep(1), PageUp: S.kind === 'epub' ? () => pageStep(-1) : null, PageDown: S.kind === 'epub' ? () => pageStep(1) : null,
    ArrowUp: S.kind === 'epub' ? () => pageStep(-1) : null, ArrowDown: S.kind === 'epub' ? () => pageStep(1) : null,
    ' ': S.kind === 'epub' ? () => pageStep(e.shiftKey ? -1 : 1) : null,
    '+': () => zoom(1), '=': () => zoom(1), '-': () => zoom(-1), '0': () => zoom('cycle'),
    o: actions.sidebar, r: S.kind === 'pdf' ? rotate : null, d: S.kind === 'epub' ? () => applyTheme('dark') : toggleDark,
    s: S.kind === 'epub' ? () => applyTheme('sepia') : null, l: S.kind === 'epub' ? () => applyTheme('light') : null,
    f: actions.fullscreen, F11: actions.fullscreen, F3: () => { openFind(); findGo(e.shiftKey); },
    Home: S.kind === 'epub' ? () => S.rendition.display(0) : S.kind === 'cbz' ? () => cbzGo(1) : null,
    End: S.kind === 'cbz' ? () => cbzGo(S.cbz.pages.length) : null,
  };
  const f = map[k];
  if (f) { e.preventDefault && e.preventDefault(); f(); }
}
document.addEventListener('keydown', onKey);

// ---------------------------------------------------------------- controllers (standard mapping)
const held = {};
let lastPadMove = 0;
const BTN = { a: 0, b: 1, x: 2, y: 3, lb: 4, rb: 5, view: 8, start: 9, up: 12, down: 13, left: 14, right: 15 };
function padFrame() {
  const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : [];
  if (pads.length) {
    const now = performance.now(), st = {}, ax = { lx: 0, ly: 0, ry: 0, lt: 0, rt: 0 };
    const dz = v => Math.abs(v) < 0.2 ? 0 : v;
    for (const g of pads) {
      for (const [n, i] of Object.entries(BTN)) st[n] ||= !!g.buttons[i]?.pressed;
      ax.lx += dz(g.axes[0] || 0); ax.ly += dz(g.axes[1] || 0); ax.ry += dz(g.axes[3] || 0);
      ax.lt = Math.max(ax.lt, g.buttons[6]?.value || 0); ax.rt = Math.max(ax.rt, g.buttons[7]?.value || 0);
    }
    const ui = inUi() || !S.kind;
    if (ui) { st.up ||= ax.ly < -0.6; st.down ||= ax.ly > 0.6; st.left ||= ax.lx < -0.6; st.right ||= ax.lx > 0.6; }
    for (const [k, on] of Object.entries(st)) {
      if (!on) { held[k] = 0; continue; }
      const first = !held[k];
      const dir = ['up', 'down', 'left', 'right', 'lb', 'rb'].includes(k);
      const repeat = held[k] && dir && now - held[k] > 420 && now - lastPadMove > 140;
      if (first) held[k] = now;
      if (!(first || repeat)) continue;
      lastPadMove = now; S.pad = true; idle();
      padButton(k, ui);
    }
    if (!ui) {
      const sy = ax.ly + ax.ry;
      if (sy && S.kind !== 'epub') scrollBy(sy * 22);
      const z = ax.rt - ax.lt;
      if (Math.abs(z) > 0.3 && now - (S.lastZoom || 0) > 220) { S.lastZoom = now; zoom(z > 0 ? 1 : -1); }
    }
  }
  requestAnimationFrame(padFrame);
}
function padButton(k, ui) {
  if (k === 'b') { back(); return; }
  const dir = { up: 'up', down: 'down', left: 'left', right: 'right' }[k];
  if (ui) {
    if (dir) { if (!move(dir) && k === 'down' && topLayer() === $('#app') && S.kind) docFocus(); return; }
    if (k === 'a') { const a = document.activeElement; if (a && a !== document.body && a.click) a.click(); return; }
  }
  if (topLayer() !== $('#app')) return;
  switch (k) {
    case 'left': case 'lb': pageStep(-1); break;
    case 'right': case 'rb': pageStep(1); break;
    case 'up': if (S.kind === 'epub') pageStep(-1); else scrollBy(-120); break;
    case 'down': if (S.kind === 'epub') pageStep(1); else scrollBy(120); break;
    case 'a': { const f = $('#tools [data-f]:not([hidden])'); if (f) { f.focus(); f.classList.add('kf'); } break; }
    case 'x': if (S.kind) { setSide(!S.sideOpen); if (S.sideOpen) { const t = $('#side .thumb.cur, #side .olink.cur') || focusables($('#side'))[0]; if (t) { t.focus(); t.classList.add('kf'); } } } break;
    case 'y': moreMenu(); break;
    case 'view': actions.fullscreen(); break;
    case 'start': if (S.kind === 'epub') applyTheme({ light: 'sepia', sepia: 'dark', dark: 'light' }[S.theme] || 'light'); else if (S.kind) toggleDark(); break;
  }
}
requestAnimationFrame(padFrame);
window.addEventListener('gamepadconnected', () => toast('Controller connected'));

// Files dropped on the window open here.
if (T && T.event) T.event.listen('tauri://drag-drop', e => { const p = e.payload && e.payload.paths && e.payload.paths.find(kindOf); if (p) openDoc(p); }).catch(() => {});

// ---------------------------------------------------------------- CI's check (HEARTH_DOCUMENTS_SELFTEST=1)
async function selftest() {
  const log = l => invoke('log', { line: 'SELFTEST ' + l });
  try {
    if (S.kind === 'pdf') {
      if (!S.pdf) throw new Error('no PDF');
      const outline = await S.pdf.getOutline();
      S.bus.dispatch('find', { source: null, type: '', query: 'fireside', caseSensitive: false, entireWord: false, highlightAll: true, findPrevious: false, matchDiacritics: false });
      for (let i = 0; i < 40 && !(S.lastMatches && S.lastMatches.total); i++) await sleep(250);
      await sleep(800);
      const text = $$('.textLayer span').length;
      await log(`pdf: ${S.pdf.numPages} pages, contents ${outline ? outline.length : 0} entries, "fireside" found ${S.lastMatches ? S.lastMatches.total : 0} times, text layer ${text} pieces, worker ${S.pdf._transport && S.pdf._transport.loadingTask && S.pdf._transport.loadingTask._worker && S.pdf._transport.loadingTask._worker._webWorker ? 'yes' : 'no (main thread)'}`);
      S.viewer.currentPageNumber = 5; await sleep(600);
      await flushRemember();
      if (!(S.pdf.numPages > 1 && S.lastMatches && S.lastMatches.total > 0)) throw new Error('find or pages failed');
    } else if (S.kind === 'epub') {
      if (!S.rendition) throw new Error('no book');
      await sleep(1500);
      const before = S.loc && S.loc.start.cfi;
      await S.rendition.next(); await sleep(800);
      const after = S.loc && S.loc.start.cfi;
      const hits = await epubSearch('lantern');
      const frame = $('#epubArea iframe');
      const words = frame && frame.contentDocument ? frame.contentDocument.body.innerText.length : 0;
      await log(`epub: contents ${S.toc.length} entries, text ${words} characters shown, turned the page ${before !== after ? 'yes' : 'no'}, "lantern" found ${hits.length} times`);
      await flushRemember();
      if (!S.toc.length || !words || !hits.length) throw new Error('e-book check failed');
    } else if (S.kind === 'cbz') {
      await sleep(1500);
      const ims = $$('#cbzPages img');
      const ok = ims.filter(i => i.naturalWidth > 0).length;
      await log(`cbz: ${S.cbz.pages.length} pages, ${ok} pictures drawn so far (${ims[0] ? ims[0].naturalWidth + 'x' + ims[0].naturalHeight : 'none'})`);
      if (!ok) throw new Error('no comic pages drawn');
    } else throw new Error('nothing opened: ' + ($('#msg').innerText || ''));
    await log('ok');
  } catch (e) { await log('FAILED ' + (e && e.message || e)); }
  await invoke('quit');
}

// ---------------------------------------------------------------- start
(async function start() {
  let init = {};
  try { init = await invoke('init'); } catch (_) {}
  S.state = init.state || { recent: [], settings: {} };
  S.settings = S.state.settings || {};
  S.selftest = !!init.selftest;
  if (S.settings.darkPages) { document.body.classList.add('darkPages'); $('#darkBtn').classList.add('on'); }
  if (init.target) await openDoc(init.target);
  else await showHome();
  if (S.selftest) selftest();
})();

// ---------------------------------------------------------------- without the app (a plain browser): sample documents
function demoInvoke(cmd, args = {}) {
  const D = window.DOCS_DEMO || {};
  const q = new URLSearchParams(location.search);
  const ok = v => Promise.resolve(v);
  switch (cmd) {
    case 'init': return ok({ target: q.get('open') || D.target || null, platform: 'linux', state: D.state || { recent: [], settings: {} } });
    case 'recent': return ok(D.state || { recent: [], settings: {} });
    case 'read_file': return fetch(D.files && D.files[args.path] || args.path).then(r => { if (!r.ok) throw new Error('not found'); return r.arrayBuffer(); });
    case 'cbz_pages': return ok(D.cbzPages || []);
    case 'file_meta': return ok({ size: 123456, modified: Date.now(), folder: '/home/you/Documents' });
    case 'remember': case 'forget': case 'settings': case 'set_title': case 'print': case 'show_in_folder': return ok(null);
    case 'log': console.log(args.line); return ok(null);
    case 'fullscreen': return ok(!!args.on);
    case 'pick': return ok(null);
    default: return Promise.reject('Not in the preview');
  }
}
