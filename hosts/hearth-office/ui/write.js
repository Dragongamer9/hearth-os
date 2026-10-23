// Hearth Write: the word processor. The document is HTML in a contenteditable "sheet of paper" (#ed), with page
// settings beside it (W.page: size, margins, header, footer, page numbers, default font). Files are read into that
// HTML and written from it: .docx (formats/docx.js), .odt (formats/odt.js), .rtf (formats/rtf.js), and .txt, .md
// and .html here. Undo is Write's own (snapshots), so every change, typed or made by a button, undoes the same way.
'use strict';
(() => {
const { el, esc, $, $$ } = H;
const MM = 96 / 25.4; // px per mm
const SIZES = { A4: [210, 297], Letter: [215.9, 279.4], Legal: [215.9, 355.6], A5: [148, 210], Executive: [184.15, 266.7] };
const DEFAULT_FONTS = ['Calibri', 'Carlito', 'Arial', 'Liberation Sans', 'Helvetica', 'Times New Roman', 'Liberation Serif', 'Georgia', 'Cambria', 'Caladea',
  'Garamond', 'Verdana', 'Tahoma', 'Trebuchet MS', 'Segoe UI', 'Noto Sans', 'Noto Serif', 'DejaVu Sans', 'DejaVu Serif', 'Courier New', 'Liberation Mono', 'Comic Sans MS'];
const SERIF = /times|serif|georgia|cambria|caladea|garamond|book|roman|palat/i, MONO = /mono|courier|consol|code/i;
// A font name as CSS, with look-alikes after it (Calibri → Carlito, Arial → Liberation Sans...).
const fontCss = (name) => {
  if (!name) return '';
  const n = name.replace(/["']/g, '');
  const alias = { calibri: 'Carlito', cambria: 'Caladea', arial: 'Liberation Sans', 'times new roman': 'Liberation Serif', 'courier new': 'Liberation Mono', helvetica: 'Liberation Sans' }[n.toLowerCase()];
  const generic = MONO.test(n) ? 'monospace' : SERIF.test(n) ? 'serif' : 'sans-serif';
  return `"${n}"${alias ? `, "${alias}"` : ''}, ${generic}`;
};
const firstFont = (css) => (css || '').split(',')[0].trim().replace(/^["']|["']$/g, '');

const W = {
  id: 'write', name: 'Hearth Write', untitled: 'Untitled document', tagline: 'Letters, reports and everyday documents',
  credits: 'docx (MIT), mammoth (BSD-2), JSZip (MIT), DOMPurify (Apache-2.0), marked and turndown (MIT), Typo.js (BSD-3) with the Hunspell en_US dictionary (SCOWL)',
  formats: [
    { ext: 'docx', label: 'Word document' }, { ext: 'odt', label: 'OpenDocument text' }, { ext: 'rtf', label: 'Rich text' },
    { ext: 'html', label: 'Web page' }, { ext: 'md', label: 'Markdown' }, { ext: 'txt', label: 'Plain text' },
  ],
  openExts: ['docx', 'odt', 'rtf', 'txt', 'md', 'markdown', 'html', 'htm', 'doc'],
  canWrite: (ext) => ['docx', 'odt', 'rtf', 'html', 'htm', 'md', 'markdown', 'txt'].includes(ext),
  lossy: (ext) => ({
    txt: 'Plain text keeps only the words: formatting, pictures, tables and page settings are left out.',
    md: 'Markdown keeps headings, bold, italic, lists, links, tables and pictures, but not fonts, sizes, colours, alignment or page settings.',
    markdown: 'Markdown keeps headings, bold, italic, lists, links, tables and pictures, but not fonts, sizes, colours, alignment or page settings.',
  }[ext] || null),
};
W.defaultPage = () => ({ size: 'Letter', orient: 'portrait', margins: { t: 25.4, r: 25.4, b: 25.4, l: 25.4 }, header: '', footer: '', numbers: 'none', font: 'Calibri', fontSize: 11, lineHeight: 1.15, spaceAfter: 8 });
W.page = W.defaultPage();
W.pageMm = () => { const [w, h] = SIZES[W.page.size] || (W.page.w && W.page.h ? [W.page.w, W.page.h] : SIZES.A4); return W.page.orient === 'landscape' ? [h, w] : [w, h]; };

// ------------------------------------------------------------------ the window
W.mount = async (main, toolbar, status) => {
  main.innerHTML = `
    <div id="wfind" class="findbar" hidden>
      <input id="fFind" class="app-key" placeholder="Find" spellcheck="false">
      <input id="fRepl" class="app-key" placeholder="Replace with" spellcheck="false">
      <label class="chk inline"><input type="checkbox" id="fCase"><span>Match case</span></label>
      <span id="fCount" class="note"></span>
      <button class="btn f" id="fPrev" title="Previous (Shift+Enter)">‹</button><button class="btn f" id="fNext" title="Next (Enter)">›</button>
      <button class="btn f" id="fOne">Replace</button><button class="btn f" id="fAll">Replace all</button>
      <button class="ibtn f" id="fClose" title="Close (Escape)"><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></svg></button>
    </div>
    <div id="desk" class="desk">
      <div id="zoomer">
        <div id="paper" class="paper">
          <div id="hdrView" class="hf hdr" title="Header (Format › Page setup)"></div>
          <div id="ed" class="doc" contenteditable="true" spellcheck="false"></div>
          <div id="ftrView" class="hf ftr" title="Footer (Format › Page setup)"></div>
          <div id="guides"></div>
        </div>
      </div>
    </div>
    <div id="preview" class="preview" hidden></div>
    <div id="imgbar" class="floatbar" hidden></div>
    <div id="measure" aria-hidden="true"></div>`;
  W.ed = $('#ed'); W.paper = $('#paper'); W.desk = $('#desk');
  document.execCommand('defaultParagraphSeparator', false, 'p');
  buildToolbar(toolbar);
  status.querySelector('#statusLeft').innerHTML = '<button id="wCount" title="Word count">0 words</button><span id="wPages"></span><span id="wSpell"></span>';
  status.querySelector('#statusRight').innerHTML = '<button id="wPreview" title="Print preview">Print layout</button><button id="zOut" title="Zoom out">−</button><input id="zoom" type="range" min="50" max="200" step="10" value="100" title="Zoom"><button id="zIn" title="Zoom in">+</button><button id="zVal">100%</button>';
  $('#wCount').onclick = () => W.wordCountDialog();
  $('#wPreview').onclick = () => W.togglePreview();
  $('#zoom').oninput = (e) => W.zoom(+e.target.value);
  $('#zOut').onclick = () => W.zoom(W.zoomV - 10); $('#zIn').onclick = () => W.zoom(W.zoomV + 10); $('#zVal').onclick = () => W.zoom(100);
  W.zoomV = H.settings.writeZoom || 100;
  wireEditor();
  wireFind();
  // The big libraries load in the background, so opening a .docx later is quick.
  setTimeout(() => H.need('vendor/jszip.min.js'), 400);
  setTimeout(() => W.spell.start(), 1500);
};
W.scroller = () => W.desk;
W.focus = () => { W.ed.focus(); };

function buildToolbar(tb) {
  const I = H.ICON;
  const fonts = Array.from(new Set([...DEFAULT_FONTS.filter((f) => !H.fonts.length || H.fonts.includes(f) || ['Calibri', 'Arial', 'Times New Roman', 'Courier New', 'Cambria', 'Georgia', 'Verdana'].includes(f)), ...H.fonts])).sort((a, b) => a.localeCompare(b));
  W.fontSel = H.tsel(fonts.map((f) => [f, f]), 'Font', (v) => { W.applyFont(v); W.focus(); }, { cls: 'font' });
  W.sizeSel = H.tsel([8, 9, 10, 10.5, 11, 12, 14, 16, 18, 20, 24, 28, 32, 36, 48, 60, 72].map((s) => [s, s]), 'Font size', (v) => { W.applySize(+v); W.focus(); }, { cls: 'size' });
  W.styleSel = H.tsel([['p', 'Normal text'], ['title', 'Title'], ['subtitle', 'Subtitle'], ['h1', 'Heading 1'], ['h2', 'Heading 2'], ['h3', 'Heading 3'], ['h4', 'Heading 4'], ['blockquote', 'Quote'], ['pre', 'Code']], 'Paragraph style', (v) => { W.setBlock(v); W.focus(); }, { cls: 'style' });
  tb.append(
    H.tbtn(I.undo, 'Undo (Ctrl+Z)', () => W.undo()), H.tbtn(I.redo, 'Redo (Ctrl+Y)', () => W.redo()), H.tsep(),
    W.styleSel, W.fontSel, W.sizeSel, H.tsep(),
    H.tbtn(I.bold, 'Bold (Ctrl+B)', () => W.exec('bold'), { cmd: 'bold' }), H.tbtn(I.italic, 'Italic (Ctrl+I)', () => W.exec('italic'), { cmd: 'italic' }),
    H.tbtn(I.underline, 'Underline (Ctrl+U)', () => W.exec('underline'), { cmd: 'underline' }), H.tbtn(I.strike, 'Strikethrough', () => W.exec('strikeThrough'), { cmd: 'strikeThrough' }),
    H.colorBtn(I.color, 'Text colour', '#c00000', (c) => W.applyColor(c || '#000000')),
    H.colorBtn(I.highlight, 'Highlight', '#ffff00', (c) => W.applyHighlight(c), { none: 'No highlight' }), H.tsep(),
    H.tbtn(I.left, 'Align left (Ctrl+L)', () => W.align('left'), { cmd: 'justifyLeft' }), H.tbtn(I.center, 'Centre (Ctrl+E)', () => W.align('center'), { cmd: 'justifyCenter' }),
    H.tbtn(I.right, 'Align right (Ctrl+R)', () => W.align('right'), { cmd: 'justifyRight' }), H.tbtn(I.justify, 'Justify (Ctrl+J)', () => W.align('justify'), { cmd: 'justifyFull' }),
    H.tbtn('<svg viewBox="0 0 24 24"><path d="M10 6h10M10 12h10M10 18h10M5 4v16M3 6l2-2 2 2M3 18l2 2 2-2"/></svg>', 'Line spacing', (e, b) => { const r = b.getBoundingClientRect(); H.showMenu(lineSpacingItems(), r.left, r.bottom + 4); }),
    H.tsep(),
    H.tbtn(I.bullets, 'Bulleted list (Ctrl+Shift+8)', () => W.exec('insertUnorderedList'), { cmd: 'insertUnorderedList' }), H.tbtn(I.numbers, 'Numbered list (Ctrl+Shift+7)', () => W.exec('insertOrderedList'), { cmd: 'insertOrderedList' }),
    H.tbtn(I.outdent, 'Decrease indent', () => W.exec('outdent')), H.tbtn(I.indent, 'Increase indent', () => W.exec('indent')), H.tsep(),
    H.tbtn(I.link, 'Link (Ctrl+K)', () => W.linkDialog()), H.tbtn(I.image, 'Picture', () => W.insertPicture()), H.tbtn(I.table, 'Table', (e, b) => W.tablePicker(b)),
    H.tbtn(I.clear, 'Clear formatting (Ctrl+Space)', () => W.clearFormat()), H.tsep(),
    H.tbtn(I.find, 'Find and replace (Ctrl+H)', () => W.find(true)), H.tbtn(I.spell, 'Spelling (F7)', () => W.spell.dialog()),
  );
}
const lineSpacingItems = () => [1, 1.15, 1.5, 2, 2.5, 3].map((v) => ({ label: String(v), checked: () => W.curLine === v, run: () => W.lineSpacing(v) }))
  .concat([{ sep: true }, { label: 'Paragraph settings…', run: () => W.paragraphDialog() }]);

// ------------------------------------------------------------------ undo (snapshots of the HTML and where the caret was)
const U = { stack: [], redo: [], lastT: 0, lastKind: '' };
const textPos = (root, node, off) => {
  // Characters (and pictures, counted as one) before (node, off).
  let n = 0;
  const w = document.createTreeWalker(root, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT);
  if (node.nodeType === 1) { if (off < node.childNodes.length) { node = node.childNodes[off]; off = 0; } else { let x = node; while (x && !x.nextSibling && x !== root) x = x.parentNode; if (!x || x === root) return Infinity; node = x.nextSibling; off = 0; } }
  for (let c = w.nextNode(); c; c = w.nextNode()) {
    if (c === node) return n + (c.nodeType === 3 ? off : 0);
    if (c.nodeType === 3) n += c.length; else if (c.tagName === 'IMG' || c.tagName === 'BR') n += 1; else if (/^(P|H\d|LI|TD|TH|BLOCKQUOTE|PRE|DIV)$/.test(c.tagName)) n += 1;
  }
  return n;
};
const nodeAt = (root, pos) => {
  let n = 0;
  const w = document.createTreeWalker(root, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT);
  let last = null;
  for (let c = w.nextNode(); c; c = w.nextNode()) {
    if (c.nodeType === 3) { if (n + c.length >= pos) return [c, pos - n]; n += c.length; last = c; }
    else {
      const blk = /^(P|H\d|LI|TD|TH|BLOCKQUOTE|PRE|DIV)$/.test(c.tagName);
      if ((c.tagName === 'IMG' || c.tagName === 'BR' || blk) && n >= pos) { return [c.parentNode, Array.prototype.indexOf.call(c.parentNode.childNodes, c)]; }
      if (c.tagName === 'IMG' || c.tagName === 'BR' || blk) n += 1;
    }
  }
  return last ? [last, last.length] : [root, root.childNodes.length];
};
W.getSel = () => {
  const s = getSelection();
  if (!s.rangeCount || !W.ed.contains(s.anchorNode)) return null;
  const r = s.getRangeAt(0);
  return { a: textPos(W.ed, r.startContainer, r.startOffset), b: textPos(W.ed, r.endContainer, r.endOffset) };
};
W.setSel = (p) => {
  if (!p) return;
  try {
    const [n1, o1] = nodeAt(W.ed, p.a), [n2, o2] = nodeAt(W.ed, p.b);
    const r = document.createRange(); r.setStart(n1, o1); r.setEnd(n2, o2);
    const s = getSelection(); s.removeAllRanges(); s.addRange(r);
  } catch {}
};
const snap = () => ({ html: W.ed.innerHTML, sel: W.getSel(), page: JSON.stringify(W.page) });
W.record = (kind = 'cmd') => {
  const now = Date.now();
  if (kind === 'typing' && U.lastKind === 'typing' && now - U.lastT < 1500) { U.lastT = now; return; }
  U.stack.push(snap()); if (U.stack.length > 300) U.stack.shift();
  U.redo = []; U.lastT = now; U.lastKind = kind;
};
const restore = (s) => { W.ed.innerHTML = s.html; W.page = JSON.parse(s.page); W.applyPage(); W.focus(); W.setSel(s.sel); W.afterChange(); };
W.undo = () => { if (!U.stack.length) return H.toast('Nothing to undo'); U.redo.push(snap()); restore(U.stack.pop()); U.lastKind = ''; };
W.redo = () => { if (!U.redo.length) return H.toast('Nothing to redo'); U.stack.push(snap()); restore(U.redo.pop()); U.lastKind = ''; };

// After any change: dirty, counts, spelling, page guides.
W.afterChange = () => { H.changed(); W.countSoon(); W.spell.soon(); W.guidesSoon(); };

// ------------------------------------------------------------------ editing commands
W.exec = (cmd, val) => {
  W.focus();
  const before = W.ed.innerHTML;
  W.record();
  document.execCommand('styleWithCSS', false, false);
  document.execCommand(cmd, false, val);
  W.normalizeFonts();
  if (W.ed.innerHTML === before) { U.stack.pop(); W.updateState(); return; } // nothing changed (e.g. bold for what's typed next)
  W.afterChange(); W.updateState();
};
// execCommand writes <font> tags; Write keeps spans with styles (they save to every format the same way).
W.normalizeFonts = () => {
  const fonts = $$('font', W.ed);
  if (!fonts.length) return;
  const sel = W.getSel();
  for (const f of fonts) {
    const s = document.createElement('span');
    if (f.getAttribute('face')) s.style.fontFamily = fontCss(f.getAttribute('face'));
    if (f.getAttribute('color')) s.style.color = f.getAttribute('color');
    if (f.getAttribute('size') === '7' && W._pendingSize) s.style.fontSize = W._pendingSize + 'pt';
    if (f.style.cssText) s.style.cssText += f.style.cssText;
    while (f.firstChild) s.append(f.firstChild);
    f.replaceWith(s);
    // Sizes set inside: the new size wins.
    if (s.style.fontSize) $$('span', s).forEach((x) => { x.style.fontSize = ''; if (!x.getAttribute('style')) x.replaceWith(...x.childNodes); });
  }
  W._pendingSize = null;
  W.setSel(sel);
};
W.applyFont = (name) => { W.exec('fontName', name); };
W.applySize = (pt) => {
  const s = getSelection();
  if (!s.rangeCount) return;
  if (s.isCollapsed) { // nothing selected: the next typing gets the size
    W.record();
    const span = el('span', { style: { fontSize: pt + 'pt' } }, '​');
    const r = s.getRangeAt(0); r.insertNode(span);
    const r2 = document.createRange(); r2.setStart(span.firstChild, 1); r2.collapse(true); s.removeAllRanges(); s.addRange(r2);
    W.afterChange(); return;
  }
  W._pendingSize = pt;
  W.exec('fontSize', '7');
};
W.applyColor = (c) => W.exec('foreColor', c);
W.applyHighlight = (c) => {
  W.focus(); W.record();
  document.execCommand('styleWithCSS', false, true);
  document.execCommand('hiliteColor', false, c || 'transparent');
  document.execCommand('styleWithCSS', false, false);
  if (!c) $$('[style*="background"]', W.ed).forEach((e) => { if (/transparent|rgba\(0, 0, 0, 0\)/.test(e.style.backgroundColor)) e.style.backgroundColor = ''; });
  W.afterChange();
};
W.clearFormat = () => {
  W.exec('removeFormat');
  // removeFormat leaves spans with sizes and fonts behind: those go too, inside the selection.
  const s = getSelection(); if (!s.rangeCount) return;
  const r = s.getRangeAt(0);
  $$('span', W.ed).forEach((sp) => { if (r.intersectsNode(sp)) { sp.style.fontSize = ''; sp.style.fontFamily = ''; sp.style.color = ''; sp.style.backgroundColor = ''; if (!sp.getAttribute('style')) sp.replaceWith(...sp.childNodes); } });
};
const BLOCKS = /^(P|H[1-6]|LI|BLOCKQUOTE|PRE|DIV|TD|TH)$/;
W.blockOf = (n) => { while (n && n !== W.ed) { if (n.nodeType === 1 && BLOCKS.test(n.tagName)) return n; n = n.parentNode; } return null; };
// The paragraphs (and list items) the selection touches.
W.selBlocks = () => {
  const s = getSelection(); if (!s.rangeCount) return [];
  const r = s.getRangeAt(0);
  const out = $$('p,h1,h2,h3,h4,h5,h6,li,blockquote,pre', W.ed).filter((b) => r.intersectsNode(b) && !b.querySelector('p,li,h1,h2,h3,h4,h5,h6'));
  if (!out.length) { const b = W.blockOf(r.startContainer); if (b) out.push(b); }
  return out;
};
W.setBlock = (v) => {
  W.focus();
  const tag = v === 'title' || v === 'subtitle' ? 'p' : v;
  W.record();
  document.execCommand('formatBlock', false, '<' + tag + '>');
  for (const b of W.selBlocks()) { b.classList.remove('title', 'subtitle'); if (v === 'title' || v === 'subtitle') b.classList.add(v); if (!b.className) b.removeAttribute('class'); }
  W.afterChange(); W.updateState();
};
W.align = (a) => {
  W.focus(); W.record();
  for (const b of W.selBlocks()) b.style.textAlign = a === 'left' ? '' : a;
  W.afterChange(); W.updateState();
};
W.lineSpacing = (v) => {
  W.focus(); W.record();
  for (const b of W.selBlocks()) b.style.lineHeight = String(v);
  W.afterChange(); W.updateState();
};
W.paragraphDialog = async () => {
  const b = W.selBlocks()[0]; if (!b) return;
  const cs = getComputedStyle(b);
  const px2pt = (v) => Math.round(parseFloat(v) * 0.75 * 10) / 10;
  const v = await H.form('Paragraph', [
    { id: 'line', label: 'Line spacing', type: 'number', step: 0.05, min: 0.8, max: 4, value: parseFloat(b.style.lineHeight) || W.page.lineHeight, half: true },
    { id: 'align', label: 'Alignment', type: 'select', value: b.style.textAlign || 'left', options: [['left', 'Left'], ['center', 'Centre'], ['right', 'Right'], ['justify', 'Justified']], half: true },
    { id: 'before', label: 'Space before (pt)', type: 'number', min: 0, step: 1, value: px2pt(cs.marginTop), half: true },
    { id: 'after', label: 'Space after (pt)', type: 'number', min: 0, step: 1, value: px2pt(cs.marginBottom), half: true },
    { id: 'indent', label: 'Indent left (mm)', type: 'number', min: 0, step: 1, value: Math.round(parseFloat(cs.marginLeft) / MM), half: true },
    { id: 'first', label: 'First line indent (mm)', type: 'number', step: 1, value: Math.round(parseFloat(cs.textIndent) / MM), half: true },
  ], 'Apply');
  if (!v) return;
  W.record();
  for (const x of W.selBlocks()) {
    x.style.lineHeight = String(v.line);
    x.style.textAlign = v.align === 'left' ? '' : v.align;
    x.style.marginTop = v.before + 'pt'; x.style.marginBottom = v.after + 'pt';
    x.style.marginLeft = v.indent ? v.indent + 'mm' : '';
    x.style.textIndent = v.first ? v.first + 'mm' : '';
  }
  W.afterChange();
};
W.insertHTML = (html) => { W.focus(); W.record(); document.execCommand('insertHTML', false, html); W.normalizeFonts(); W.afterChange(); };
W.pageBreak = () => W.insertHTML('<hr class="page-break"><p><br></p>');

// Links: Ctrl+K adds or changes one; Ctrl+click opens it (web and mail links only).
W.linkDialog = async () => {
  const s = getSelection();
  const a = s.anchorNode && (s.anchorNode.nodeType === 1 ? s.anchorNode : s.anchorNode.parentElement).closest('a');
  const sel = W.getSel();
  const text = s.toString();
  const v = await H.form(a ? 'Edit link' : 'Add a link', [
    { id: 'text', label: 'Text to show', value: a ? a.textContent : text },
    { id: 'url', label: 'Web address', value: a ? a.getAttribute('href') : (/^https?:\/\//.test(text) ? text : 'https://'), placeholder: 'https://…' },
  ], a ? 'Save' : 'Add link');
  if (!v) return;
  let url = v.url.trim();
  if (url && !/^(https?:|mailto:|#)/i.test(url)) url = (url.includes('@') && !url.includes('/') ? 'mailto:' : 'https://') + url;
  W.focus(); W.setSel(sel); W.record();
  if (a) { if (!url) a.replaceWith(...a.childNodes); else { a.setAttribute('href', url); if (v.text && v.text !== a.textContent) a.textContent = v.text; } }
  else if (url) {
    if (!text || v.text !== text) document.execCommand('insertHTML', false, `<a href="${esc(url)}">${esc(v.text || url)}</a>`);
    else document.execCommand('createLink', false, url);
  }
  W.afterChange();
};
W.insertPicture = async () => {
  const p = await H.invoke('pick_open', { kind: 'image' });
  if (!p) return;
  const r = await H.invoke('read_file', { path: p });
  const ext = H.extOf(p);
  let url = `data:${H.mimeOf(ext)};base64,${r.data}`;
  if (['bmp', 'webp', 'tif', 'tiff'].includes(ext)) { const png = await H.toPng(url); if (png) url = png.url; }
  const { w } = await H.imageSize(url);
  const max = (W.pageMm()[0] - W.page.margins.l - W.page.margins.r) * MM;
  W.insertHTML(`<img src="${url}" alt="${esc(H.stem(p))}" style="width:${Math.round(Math.min(w || max, max))}px">`);
};

// Tables: a grid to pick the size, then right-click a cell for rows and columns.
W.tablePicker = (btn) => {
  const r = btn.getBoundingClientRect();
  H.closeMenus();
  const m = el('div', { class: 'menu tgrid' });
  const lab = el('div', { class: 'mhead', text: 'Insert table' });
  const grid = el('div', { class: 'tg' });
  for (let i = 0; i < 8; i++) for (let j = 0; j < 10; j++) {
    const c = el('button', { class: 'tc', 'data-r': i + 1, 'data-c': j + 1 });
    c.onmouseenter = () => { $$('.tc', grid).forEach((x) => x.classList.toggle('on', +x.dataset.r <= i + 1 && +x.dataset.c <= j + 1)); lab.textContent = `${i + 1} × ${j + 1} table`; };
    c.onclick = () => { H.closeMenus(); W.insertTable(i + 1, j + 1); };
    grid.append(c);
  }
  m.append(lab, grid, el('button', { class: 'mi wide', text: 'More rows or columns…', onclick: async () => { H.closeMenus(); const v = await H.form('Insert table', [{ id: 'r', label: 'Rows', type: 'number', value: 3, min: 1, max: 500, half: true }, { id: 'c', label: 'Columns', type: 'number', value: 3, min: 1, max: 40, half: true }], 'Insert'); if (v) W.insertTable(v.r | 0, v.c | 0); } }));
  m.addEventListener('mousedown', (e) => e.preventDefault());
  $('#layer').append(m); m.style.left = r.left + 'px'; m.style.top = r.bottom + 4 + 'px';
};
W.insertTable = (rows, cols) => {
  const row = '<tr>' + '<td><br></td>'.repeat(cols) + '</tr>';
  W.insertHTML(`<table class="t"><tbody>${row.repeat(rows)}</tbody></table><p><br></p>`);
};
const cellOf = (n) => n && (n.nodeType === 1 ? n : n.parentElement).closest('td,th');
W.tableOp = (op) => {
  const s = getSelection(); const td = cellOf(s.anchorNode); if (!td || !W.ed.contains(td)) return;
  const tr = td.parentElement, table = td.closest('table');
  const ci = Array.prototype.indexOf.call(tr.children, td);
  W.record();
  const blankRow = () => { const r = document.createElement('tr'); for (let i = 0; i < tr.children.length; i++) r.append(el('td', { html: '<br>' })); return r; };
  if (op === 'rowAbove') tr.before(blankRow());
  else if (op === 'rowBelow') tr.after(blankRow());
  else if (op === 'colLeft' || op === 'colRight') $$('tr', table).forEach((r) => { const c = r.children[Math.min(ci, r.children.length - 1)]; const n = el(c && c.tagName === 'TH' ? 'th' : 'td', { html: '<br>' }); if (!c) r.append(n); else if (op === 'colLeft') c.before(n); else c.after(n); });
  else if (op === 'delRow') { if ($$('tr', table).length === 1) table.remove(); else tr.remove(); }
  else if (op === 'delCol') { if (tr.children.length === 1) table.remove(); else $$('tr', table).forEach((r) => r.children[ci] && r.children[ci].remove()); }
  else if (op === 'delTable') table.remove();
  else if (op === 'header') { const first = table.querySelector('tr'); $$('td,th', first).forEach((c) => { const n = el(c.tagName === 'TH' ? 'td' : 'th', { html: c.innerHTML }); c.replaceWith(n); }); }
  else if (op === 'borders') table.classList.toggle('nob');
  W.afterChange();
};

// ------------------------------------------------------------------ the editor's events
function wireEditor() {
  const ed = W.ed;
  ed.addEventListener('beforeinput', (e) => {
    if (e.inputType === 'historyUndo') { e.preventDefault(); W.undo(); return; }
    if (e.inputType === 'historyRedo') { e.preventDefault(); W.redo(); return; }
    W.record(e.inputType === 'insertText' || e.inputType === 'deleteContentBackward' ? 'typing' : 'edit');
  });
  ed.addEventListener('input', () => { W.afterChange(); if (!ed.firstElementChild) ed.innerHTML = '<p><br></p>'; });
  document.addEventListener('selectionchange', () => { if (document.activeElement === ed) W.updateStateSoon(); });
  ed.addEventListener('keydown', (e) => {
    // Tab in a table moves between cells (and adds a row at the end); elsewhere it indents lists or types a tab.
    if (e.key === 'Tab' && !e.ctrlKey && !e.altKey) {
      const td = cellOf(getSelection().anchorNode);
      e.preventDefault();
      if (td) {
        const cells = $$('td,th', td.closest('table'));
        let i = cells.indexOf(td) + (e.shiftKey ? -1 : 1);
        if (i >= cells.length) { W.tableOp('rowBelow'); i = cells.length; }
        const t = $$('td,th', td.closest('table'))[Math.max(0, i)];
        if (t) { const r = document.createRange(); r.selectNodeContents(t); getSelection().removeAllRanges(); getSelection().addRange(r); }
      } else if (W.blockOf(getSelection().anchorNode)?.tagName === 'LI') W.exec(e.shiftKey ? 'outdent' : 'indent');
      else W.insertHTML('<span class="tab">\t</span>');
    }
  });
  ed.addEventListener('paste', (e) => {
    const cd = e.clipboardData; if (!cd) return;
    const file = Array.from(cd.files || []).find((f) => f.type.startsWith('image/'));
    if (file) {
      e.preventDefault();
      const fr = new FileReader(); fr.onload = () => W.insertHTML(`<img src="${fr.result}" style="max-width:100%">`); fr.readAsDataURL(file);
      return;
    }
    const html = cd.getData('text/html');
    e.preventDefault();
    if (html) W.insertHTML(cleanPasted(html));
    else { const t = cd.getData('text/plain'); W.insertHTML(textToHtml(t, true)); }
  });
  ed.addEventListener('drop', (e) => {
    const f = e.dataTransfer && Array.from(e.dataTransfer.files || []).find((x) => x.type.startsWith('image/'));
    if (f) { e.preventDefault(); const fr = new FileReader(); fr.onload = () => W.insertHTML(`<img src="${fr.result}" style="max-width:100%">`); fr.readAsDataURL(f); }
  });
  ed.addEventListener('click', (e) => {
    const a = e.target.closest('a');
    if (a && (e.ctrlKey || e.metaKey)) { e.preventDefault(); H.invoke('open_url', { url: a.href }).catch((x) => H.toast(String(x))); return; }
    if (e.target.tagName === 'IMG') W.selectImage(e.target); else W.selectImage(null);
  });
  ed.addEventListener('mouseover', (e) => { const a = e.target.closest('a'); if (a) a.title = `${a.getAttribute('href')} — Ctrl+click to open`; });
  ed.addEventListener('contextmenu', (e) => { e.preventDefault(); W.contextMenu(e); });
  W.desk.addEventListener('mousedown', (e) => { if (e.target === W.desk || e.target.id === 'zoomer') { e.preventDefault(); W.focus(); } });
  W.desk.addEventListener('scroll', () => W.positionImgbar());
}
// Pasted HTML: cleaned (no scripts), and stripped of the page styles office suites and web pages put around it.
function cleanPasted(html) {
  let h = H.clean(html.replace(/<!--[\s\S]*?-->/g, ''));
  const box = el('div', { html: h });
  $$('*', box).forEach((n) => {
    n.removeAttribute('class');
    // Keep the formatting people mean (bold, colours, sizes, alignment); drop layout (positions, widths, margins).
    const st = n.style; if (!st) return;
    const keep = {}; for (const p of ['font-weight', 'font-style', 'text-decoration', 'color', 'background-color', 'font-size', 'font-family', 'text-align', 'vertical-align']) if (st.getPropertyValue(p)) keep[p] = st.getPropertyValue(p);
    n.removeAttribute('style');
    for (const [k, v] of Object.entries(keep)) if (!(k === 'color' && /^(rgb\(0, 0, 0\)|#000000|black)$/.test(v)) && !(k === 'background-color' && /transparent|rgba\(0, 0, 0, 0\)|white|#fff/.test(v))) n.style.setProperty(k, v);
  });
  return box.innerHTML;
}
const textToHtml = (t, inline) => {
  const lines = String(t).replace(/\r\n?/g, '\n').split('\n');
  if (inline && lines.length === 1) return esc(lines[0]);
  return lines.map((l) => `<p>${l ? esc(l).replace(/\t/g, '<span class="tab">\t</span>') : '<br>'}</p>`).join('');
};

// The toolbar follows the caret: style, font, size, bold..., alignment.
W.updateStateSoon = H.debounce(() => W.updateState(), 60);
W.updateState = () => {
  const s = getSelection(); if (!s.rangeCount || !W.ed.contains(s.anchorNode)) return;
  for (const b of $$('#toolbar [data-cmd]')) { let on = false; try { on = document.queryCommandState(b.dataset.cmd); } catch {} b.classList.toggle('on', on); }
  const n = s.anchorNode.nodeType === 1 ? s.anchorNode : s.anchorNode.parentElement;
  const cs = getComputedStyle(n);
  const f = firstFont(cs.fontFamily);
  if (f && W.fontSel.value !== f) { if (![...W.fontSel.options].some((o) => o.value === f)) W.fontSel.append(el('option', { value: f, text: f })); W.fontSel.value = f; }
  const pt = Math.round(parseFloat(cs.fontSize) * 0.75 * 2) / 2;
  if (![...W.sizeSel.options].some((o) => +o.value === pt)) W.sizeSel.append(el('option', { value: pt, text: pt }));
  W.sizeSel.value = String(pt);
  const b = W.blockOf(n);
  let st = b ? b.tagName.toLowerCase() : 'p';
  if (b && b.classList.contains('title')) st = 'title'; else if (b && b.classList.contains('subtitle')) st = 'subtitle';
  if (st === 'li' || st === 'td' || st === 'th' || st === 'div') st = 'p';
  W.styleSel.value = st;
  W.curLine = b ? parseFloat(b.style.lineHeight) || W.page.lineHeight : W.page.lineHeight;
};

// Right-click: spelling suggestions, cut/copy/paste, links, tables.
W.contextMenu = async (e) => {
  const items = [];
  const word = W.spell.wordAt(e.clientX, e.clientY);
  if (word && W.spell.isWrong(word.text)) {
    const sug = await W.spell.suggest(word.text);
    items.push({ heading: `“${word.text}”` });
    if (!sug.length) items.push({ label: 'No suggestions', disabled: () => true });
    for (const s of sug.slice(0, 6)) items.push({ label: s, style: 'font-weight:600', run: () => { W.record(); const r = word.range; r.deleteContents(); r.insertNode(document.createTextNode(s)); W.afterChange(); } });
    items.push({ label: 'Add to dictionary', run: () => W.spell.add(word.text) }, { label: 'Ignore', run: () => W.spell.ignore(word.text) }, { sep: true });
  }
  const sel = getSelection();
  items.push(
    { label: 'Cut', key: 'Ctrl+X', disabled: () => sel.isCollapsed, run: () => { W.record(); document.execCommand('cut'); W.afterChange(); } },
    { label: 'Copy', key: 'Ctrl+C', disabled: () => sel.isCollapsed, run: () => document.execCommand('copy') },
    { label: 'Paste', key: 'Ctrl+V', run: () => W.pasteFromMenu() },
    { label: 'Paste as plain text', key: 'Ctrl+Shift+V', run: () => W.pasteFromMenu(true) },
    { sep: true },
  );
  const a = e.target.closest('a');
  if (a) items.push({ label: 'Open link', run: () => H.invoke('open_url', { url: a.href }) }, { label: 'Edit link…', run: () => W.linkDialog() }, { label: 'Remove link', run: () => { W.record(); a.replaceWith(...a.childNodes); W.afterChange(); } }, { sep: true });
  if (cellOf(e.target)) items.push({ label: 'Table', sub: tableItems() }, { sep: true });
  if (e.target.tagName === 'IMG') items.push({ label: 'Picture size…', run: () => W.imageDialog(e.target) }, { sep: true });
  items.push({ label: 'Paragraph…', run: () => W.paragraphDialog() }, { label: 'Link…', key: 'Ctrl+K', run: () => W.linkDialog() });
  H.showMenu(items, e.clientX, e.clientY);
};
W.pasteFromMenu = async (plain) => {
  try {
    if (!plain && navigator.clipboard.read) {
      const items = await navigator.clipboard.read();
      for (const it of items) if (it.types.includes('text/html')) { W.insertHTML(cleanPasted(await (await it.getType('text/html')).text())); return; }
    }
    const t = await navigator.clipboard.readText();
    W.insertHTML(textToHtml(t, true));
  } catch { H.toast('Press Ctrl+V to paste'); }
};
const tableItems = () => [
  { label: 'Insert row above', run: () => W.tableOp('rowAbove') }, { label: 'Insert row below', run: () => W.tableOp('rowBelow') },
  { label: 'Insert column left', run: () => W.tableOp('colLeft') }, { label: 'Insert column right', run: () => W.tableOp('colRight') }, { sep: true },
  { label: 'Delete row', run: () => W.tableOp('delRow') }, { label: 'Delete column', run: () => W.tableOp('delCol') }, { label: 'Delete table', run: () => W.tableOp('delTable') }, { sep: true },
  { label: 'Header row on/off', run: () => W.tableOp('header') }, { label: 'Borders on/off', run: () => W.tableOp('borders') },
];

// Pictures: click one for its bar (sizes, wrapping, delete); drag the corner to resize.
W.selectImage = (img) => {
  $$('img.sel', W.ed).forEach((i) => i.classList.remove('sel'));
  W.selImg = img;
  const bar = $('#imgbar');
  if (!img) { bar.hidden = true; return; }
  img.classList.add('sel');
  bar.innerHTML = '';
  const b = (t, title, fn) => el('button', { class: 'btn f', text: t, title, onclick: () => { W.record(); fn(); W.afterChange(); W.positionImgbar(); } });
  const setW = (f) => { const max = W.ed.clientWidth; img.style.width = Math.round(max * f) + 'px'; img.style.height = ''; };
  bar.append(b('25%', 'A quarter of the page width', () => setW(.25)), b('50%', 'Half', () => setW(.5)), b('100%', 'Full width', () => setW(1)),
    b('Inline', 'In line with the text', () => { img.style.float = ''; img.style.margin = ''; img.style.display = ''; }),
    b('Left', 'Text wraps on the right', () => { img.style.float = 'left'; img.style.margin = '0 12px 6px 0'; img.style.display = ''; }),
    b('Right', 'Text wraps on the left', () => { img.style.float = 'right'; img.style.margin = '0 0 6px 12px'; img.style.display = ''; }),
    b('Centre', 'On its own line, centred', () => { img.style.float = ''; img.style.display = 'block'; img.style.margin = '6px auto'; }),
    el('button', { class: 'btn f', text: 'Size…', onclick: () => W.imageDialog(img) }),
    b('Delete', 'Remove the picture', () => { img.remove(); W.selectImage(null); }));
  const grip = el('div', { class: 'grip', title: 'Drag to resize' });
  grip.addEventListener('mousedown', (e) => {
    e.preventDefault(); W.record();
    const x0 = e.clientX, w0 = img.getBoundingClientRect().width / (W.zoomV / 100);
    const mv = (ev) => { img.style.width = Math.max(16, Math.round(w0 + (ev.clientX - x0) / (W.zoomV / 100))) + 'px'; img.style.height = ''; W.positionImgbar(); };
    const up = () => { removeEventListener('mousemove', mv); removeEventListener('mouseup', up); W.afterChange(); };
    addEventListener('mousemove', mv); addEventListener('mouseup', up);
  });
  bar.append(grip);
  bar.hidden = false;
  W.positionImgbar();
};
W.positionImgbar = () => {
  const img = W.selImg, bar = $('#imgbar');
  if (!img || !document.contains(img)) { bar.hidden = true; return; }
  const r = img.getBoundingClientRect(), m = $('#main').getBoundingClientRect();
  bar.style.left = Math.max(8, Math.min(r.left - m.left, m.width - bar.offsetWidth - 8)) + 'px';
  bar.style.top = Math.max(8, r.top - m.top - bar.offsetHeight - 8) + 'px';
  const g = bar.querySelector('.grip');
  g.style.left = (r.right - m.left - bar.offsetLeft - 7) + 'px'; g.style.top = (r.bottom - m.top - bar.offsetTop - 7) + 'px';
};
W.imageDialog = async (img) => {
  const v = await H.form('Picture', [
    { id: 'w', label: 'Width (px)', type: 'number', min: 8, value: Math.round(img.getBoundingClientRect().width / (W.zoomV / 100)), half: true },
    { id: 'alt', label: 'Description (for screen readers)', value: img.alt || '' },
  ], 'Apply');
  if (!v) return;
  W.record(); img.style.width = v.w + 'px'; img.style.height = ''; img.alt = v.alt; W.afterChange();
};

// ------------------------------------------------------------------ find and replace
function wireFind() {
  const f = $('#fFind'), r = $('#fRepl');
  const run = H.debounce(() => W.findRun(), 120);
  f.addEventListener('input', run); $('#fCase').addEventListener('change', run);
  f.addEventListener('keydown', (e) => { if (e.key === 'Enter') { e.preventDefault(); W.findStep(e.shiftKey ? -1 : 1); } if (e.key === 'Escape') W.find(false); });
  r.addEventListener('keydown', (e) => { if (e.key === 'Enter') { e.preventDefault(); W.replaceOne(); } if (e.key === 'Escape') W.find(false); });
  $('#fNext').onclick = () => W.findStep(1); $('#fPrev').onclick = () => W.findStep(-1);
  $('#fOne').onclick = () => W.replaceOne(); $('#fAll').onclick = () => W.replaceAll(); $('#fClose').onclick = () => W.find(false);
}
W.find = (show, replace) => {
  const bar = $('#wfind');
  if (show === false) { bar.hidden = true; W.matches = []; if (window.CSS && CSS.highlights) CSS.highlights.delete('find'); W.focus(); return; }
  bar.hidden = false;
  bar.classList.toggle('noreplace', !replace && replace !== undefined ? true : false);
  const t = getSelection().toString();
  if (t && t.length < 80 && !t.includes('\n')) $('#fFind').value = t;
  $('#fFind').focus(); $('#fFind').select();
  W.findRun();
};
// Every match as a Range (matches can cross formatting: "he<b>ll</b>o").
W.findAll = (needle, matchCase) => {
  const out = []; if (!needle) return out;
  const nodes = []; let text = '';
  const w = document.createTreeWalker(W.ed, NodeFilter.SHOW_TEXT);
  for (let n = w.nextNode(); n; n = w.nextNode()) { nodes.push([n, text.length]); text += n.data; }
  const hay = matchCase ? text : text.toLowerCase(), nd = matchCase ? needle : needle.toLowerCase();
  const at = (pos) => { let lo = 0, hi = nodes.length - 1; while (lo < hi) { const mid = (lo + hi + 1) >> 1; if (nodes[mid][1] <= pos) lo = mid; else hi = mid - 1; } return [nodes[lo][0], pos - nodes[lo][1]]; };
  for (let i = hay.indexOf(nd); i >= 0 && out.length < 10000; i = hay.indexOf(nd, i + nd.length)) {
    const r = document.createRange(); const [a, ao] = at(i); const [b, bo] = at(i + nd.length - 1);
    r.setStart(a, ao); r.setEnd(b, bo + 1); out.push(r);
  }
  return out;
};
W.findRun = () => {
  W.matches = W.findAll($('#fFind').value, $('#fCase').checked);
  W.matchI = -1;
  if (window.CSS && CSS.highlights && window.Highlight) { if (W.matches.length) CSS.highlights.set('find', new Highlight(...W.matches)); else CSS.highlights.delete('find'); }
  $('#fCount').textContent = $('#fFind').value ? (W.matches.length ? `${W.matches.length} found` : 'Not found') : '';
};
W.findStep = (d) => {
  if (!W.matches || !W.matches.length) { W.findRun(); if (!W.matches.length) return; }
  W.matchI = (W.matchI + d + W.matches.length) % W.matches.length;
  const r = W.matches[W.matchI];
  const s = getSelection(); s.removeAllRanges(); s.addRange(r);
  const rect = r.getBoundingClientRect(), dr = W.desk.getBoundingClientRect();
  if (rect.top < dr.top + 40 || rect.bottom > dr.bottom - 40) W.desk.scrollTop += rect.top - dr.top - dr.height / 3;
  $('#fCount').textContent = `${W.matchI + 1} of ${W.matches.length}`;
};
W.replaceOne = () => {
  if (!W.matches || W.matchI < 0) { W.findStep(1); return; }
  W.record();
  const r = W.matches[W.matchI]; r.deleteContents(); r.insertNode(document.createTextNode($('#fRepl').value));
  W.ed.normalize(); W.afterChange();
  const i = W.matchI; W.findRun(); W.matchI = i - 1; W.findStep(1);
};
W.replaceAll = () => {
  const ms = W.findAll($('#fFind').value, $('#fCase').checked);
  if (!ms.length) return;
  W.record();
  for (let i = ms.length - 1; i >= 0; i--) { ms[i].deleteContents(); ms[i].insertNode(document.createTextNode($('#fRepl').value)); }
  W.ed.normalize(); W.afterChange(); W.findRun();
  H.toast(`Replaced ${ms.length}`);
};

// ------------------------------------------------------------------ counts, zoom, page setup
W.text = () => W.ed.innerText || '';
W.counts = () => {
  const t = W.text();
  const words = (t.match(/[\p{L}\p{N}][\p{L}\p{N}'’\-]*/gu) || []).length;
  return { words, chars: t.replace(/\n/g, '').length, charsNoSpace: t.replace(/\s/g, '').length, paras: $$('p,h1,h2,h3,h4,h5,h6,li,blockquote,pre', W.ed).filter((p) => p.textContent.trim()).length, pages: W.pageCount || 1 };
};
W.countSoon = H.debounce(() => { const c = W.counts(); $('#wCount').textContent = `${c.words.toLocaleString()} word${c.words === 1 ? '' : 's'}`; }, 250);
W.wordCountDialog = () => {
  const c = W.counts();
  const s = getSelection().toString();
  const sw = s ? (s.match(/[\p{L}\p{N}][\p{L}\p{N}'’\-]*/gu) || []).length : null;
  H.dialog({ title: 'Word count', body: `<div class="kv"><span>Pages</span><b>${c.pages}</b><span>Words</span><b>${c.words.toLocaleString()}</b>${sw !== null ? `<span>Words selected</span><b>${sw.toLocaleString()}</b>` : ''}<span>Characters (no spaces)</span><b>${c.charsNoSpace.toLocaleString()}</b><span>Characters (with spaces)</span><b>${c.chars.toLocaleString()}</b><span>Paragraphs</span><b>${c.paras.toLocaleString()}</b></div>` });
};
W.zoom = (v) => {
  W.zoomV = Math.max(50, Math.min(200, Math.round(v / 10) * 10));
  $('#zoomer').style.zoom = W.zoomV / 100;
  $('#zoom').value = W.zoomV; $('#zVal').textContent = W.zoomV + '%';
  H.settings.writeZoom = W.zoomV; H.saveSettingsSoon && H.saveSettingsSoon();
  W.positionImgbar();
};
H.saveSettingsSoon = H.debounce(() => H.saveSettings(), 800);
W.applyPage = () => {
  const p = W.page, [w, h] = W.pageMm();
  Object.assign(W.paper.style, { width: w + 'mm', minHeight: h + 'mm', padding: `${p.margins.t}mm ${p.margins.r}mm ${p.margins.b}mm ${p.margins.l}mm` });
  Object.assign(W.ed.style, { fontFamily: fontCss(p.font), fontSize: p.fontSize + 'pt', lineHeight: String(p.lineHeight) });
  W.ed.style.setProperty('--after', (p.spaceAfter ?? 8) + 'pt');
  const num = (where) => p.numbers === where ? ' <span class="pn">Page 1</span>' : '';
  $('#hdrView').innerHTML = (p.header ? esc(p.header) : '') + num('header-right');
  $('#ftrView').innerHTML = (p.footer ? esc(p.footer) : '') + num('footer-center') + num('footer-right');
  $('#hdrView').style.top = Math.min(12.7, p.margins.t / 2) + 'mm'; $('#ftrView').style.bottom = Math.min(12.7, p.margins.b / 2) + 'mm';
  $('#hdrView').style.left = $('#ftrView').style.left = p.margins.l + 'mm'; $('#hdrView').style.right = $('#ftrView').style.right = p.margins.r + 'mm';
  $('#ftrView').classList.toggle('center', p.numbers === 'footer-center' && !p.footer);
  W.guidesSoon();
};
W.pageSetup = async () => {
  const p = W.page;
  const v = await H.form('Page setup', [
    { id: 'size', label: 'Paper size', type: 'select', value: p.size, options: Object.keys(SIZES).map((k) => [k, `${k} (${SIZES[k][0]} × ${SIZES[k][1]} mm)`]).concat(SIZES[p.size] ? [] : [[p.size, `This document's (${Math.round(p.w)} × ${Math.round(p.h)} mm)`]]), half: true },
    { id: 'orient', label: 'Orientation', type: 'select', value: p.orient, options: [['portrait', 'Portrait'], ['landscape', 'Landscape']], half: true },
    { type: 'heading', label: 'Margins (mm)' },
    { id: 't', label: 'Top', type: 'number', min: 0, step: 1, value: Math.round(p.margins.t * 10) / 10, half: true }, { id: 'b', label: 'Bottom', type: 'number', min: 0, step: 1, value: Math.round(p.margins.b * 10) / 10, half: true },
    { id: 'l', label: 'Left', type: 'number', min: 0, step: 1, value: Math.round(p.margins.l * 10) / 10, half: true }, { id: 'r', label: 'Right', type: 'number', min: 0, step: 1, value: Math.round(p.margins.r * 10) / 10, half: true },
    { type: 'heading', label: 'Header, footer and page numbers' },
    { id: 'header', label: 'Header (on every page)', value: p.header }, { id: 'footer', label: 'Footer (on every page)', value: p.footer },
    { id: 'numbers', label: 'Page numbers', type: 'select', value: p.numbers, options: [['none', 'None'], ['footer-center', 'Bottom, centre'], ['footer-right', 'Bottom, right'], ['header-right', 'Top, right']] },
    { type: 'heading', label: 'Default text' },
    { id: 'font', label: 'Font', type: 'select', value: p.font, options: Array.from(new Set([p.font, ...DEFAULT_FONTS, ...H.fonts])).map((f) => [f, f]), half: true },
    { id: 'fontSize', label: 'Size (pt)', type: 'number', min: 6, max: 72, step: 0.5, value: p.fontSize, half: true },
    { id: 'lineHeight', label: 'Line spacing', type: 'number', min: 0.8, max: 4, step: 0.05, value: p.lineHeight, half: true },
    { id: 'spaceAfter', label: 'Space after paragraphs (pt)', type: 'number', min: 0, max: 72, step: 1, value: p.spaceAfter ?? 8, half: true },
  ], 'Apply', { wide: true });
  if (!v) return;
  W.record();
  W.page = { ...p, size: v.size, orient: v.orient, margins: { t: v.t || 0, r: v.r || 0, b: v.b || 0, l: v.l || 0 }, header: v.header, footer: v.footer, numbers: v.numbers, font: v.font, fontSize: v.fontSize || 11, lineHeight: v.lineHeight || 1.15, spaceAfter: isNaN(v.spaceAfter) ? 8 : v.spaceAfter };
  W.applyPage(); W.afterChange();
};

// ------------------------------------------------------------------ pages: print layout, preview, guides
// The document is laid out page by page in a hidden measuring box: paragraphs are split between lines, lists
// between items, tables between rows. The same pages are printed, exported as PDF and shown as guides.
W.paginate = (target) => {
  const p = W.page, [wmm, hmm] = W.pageMm();
  const meas = $('#measure');
  meas.innerHTML = '';
  meas.style.width = wmm + 'mm';
  const contentH = (hmm - p.margins.t - p.margins.b) * MM;
  const pages = [];
  let body;
  const newPage = () => {
    const pg = el('div', { class: 'ppage' }); Object.assign(pg.style, { width: wmm + 'mm', height: hmm + 'mm', padding: `${p.margins.t}mm ${p.margins.r}mm ${p.margins.b}mm ${p.margins.l}mm` });
    body = el('div', { class: 'doc pbody' }); Object.assign(body.style, { fontFamily: W.ed.style.fontFamily, fontSize: W.ed.style.fontSize, lineHeight: W.ed.style.lineHeight, height: contentH + 'px' });
    body.style.setProperty('--after', W.ed.style.getPropertyValue('--after'));
    pg.append(body); meas.append(pg); pages.push({ pg, body, starts: [] });
  };
  newPage();
  const queue = Array.from(W.ed.childNodes).map((n, i) => { const c = n.cloneNode(true); c._src = i; return c; });
  const overflow = () => body.scrollHeight > contentH + 1;
  let guard = 0;
  while (queue.length && guard++ < 20000) {
    const blk = queue.shift();
    if (blk.nodeType === 3 && !blk.data.trim()) continue;
    if (blk.nodeType === 1 && blk.matches('hr.page-break')) { if (body.childNodes.length) newPage(); continue; }
    body.append(blk);
    if (blk._src != null) pages[pages.length - 1].starts.push({ src: blk._src, off: blk._off || 0 });
    if (!overflow()) continue;
    const room = contentH - (blk.offsetTop - body.offsetTop);
    const rest = blk.nodeType === 1 ? splitBlock(blk, room) : null;
    if (rest) { rest._src = blk._src; queue.unshift(rest); newPage(); continue; }
    if (body.childNodes.length > 1) { blk.remove(); pages[pages.length - 1].starts.pop(); newPage(); queue.unshift(blk); continue; }
    // A single thing taller than a page: pictures shrink to fit, anything else is cut at the page's end.
    $$('img', blk).forEach((i) => { i.style.maxHeight = contentH * 0.95 + 'px'; i.style.width = 'auto'; });
    if (blk.tagName === 'IMG') { blk.style.maxHeight = contentH * 0.95 + 'px'; blk.style.width = 'auto'; }
    if (queue.length) newPage();
  }
  // Headers, footers, page numbers.
  const n = pages.length;
  pages.forEach((pg, i) => {
    const num = `Page ${i + 1} of ${n}`;
    const top = Math.min(12.7, p.margins.t / 2), bottom = Math.min(12.7, p.margins.b / 2);
    if (p.header || p.numbers === 'header-right') pg.pg.append(el('div', { class: 'phf', style: { top: top + 'mm', left: p.margins.l + 'mm', right: p.margins.r + 'mm', fontFamily: W.ed.style.fontFamily } }, el('span', { text: p.header }), p.numbers === 'header-right' ? el('span', { text: num }) : null));
    if (p.footer || p.numbers === 'footer-center' || p.numbers === 'footer-right') {
      const f = el('div', { class: 'phf foot' + (p.numbers === 'footer-center' ? ' center' : ''), style: { bottom: bottom + 'mm', left: p.margins.l + 'mm', right: p.margins.r + 'mm', fontFamily: W.ed.style.fontFamily } }, el('span', { text: p.footer }), p.numbers !== 'none' && p.numbers !== 'header-right' ? el('span', { text: num }) : null);
      pg.pg.append(f);
    }
  });
  W.pageCount = n;
  if (target) { target.innerHTML = ''; pages.forEach((x) => target.append(x.pg)); }
  return pages;
};
// Splits a block that runs past the page's end: returns the part for the next page (or null if it can't).
function splitBlock(blk, room) {
  const tag = blk.tagName;
  if (room < 18) return null;
  if (tag === 'TABLE') {
    const rows = $$(':scope > tbody > tr, :scope > tr', blk);
    const top = blk.getBoundingClientRect().top;
    let k = rows.findIndex((r) => r.getBoundingClientRect().bottom - top > room);
    if (k <= 0) return null;
    const rest = blk.cloneNode(false); const tb = document.createElement('tbody'); rest.append(tb);
    // The header row repeats on the next page.
    if (rows[0].querySelector('th')) tb.append(rows[0].cloneNode(true));
    rows.slice(k).forEach((r) => tb.append(r));
    return rest;
  }
  if (tag === 'UL' || tag === 'OL') {
    const items = Array.from(blk.children);
    const top = blk.getBoundingClientRect().top;
    let k = items.findIndex((li) => li.getBoundingClientRect().bottom - top > room);
    if (k < 0) return null;
    // Split inside the item when it's the first one, or a long one.
    if (k === 0 || items[k].getBoundingClientRect().height > room / 2) {
      const inner = splitText(items[k], room - (items[k].getBoundingClientRect().top - top));
      if (inner) {
        const rest = blk.cloneNode(false); rest.append(inner, ...items.slice(k + 1));
        inner.style.listStyleType = 'none';
        if (tag === 'OL') rest.setAttribute('start', (parseInt(blk.getAttribute('start') || '1', 10)) + k);
        return rest;
      }
      if (k === 0) return null;
    }
    const rest = blk.cloneNode(false); items.slice(k).forEach((li) => rest.append(li));
    if (tag === 'OL') rest.setAttribute('start', (parseInt(blk.getAttribute('start') || '1', 10)) + k);
    return rest;
  }
  if (/^(P|H[1-6]|BLOCKQUOTE|PRE|DIV|LI)$/.test(tag)) return splitText(blk, room);
  return null;
}
// Splits a paragraph between lines: everything after the last line that fits moves to a copy of it.
function splitText(blk, room) {
  const top = blk.getBoundingClientRect().top;
  const limit = top + room;
  const w = document.createTreeWalker(blk, NodeFilter.SHOW_TEXT);
  let cut = null;
  for (let n = w.nextNode(); n; n = w.nextNode()) {
    if (!n.data.length) continue;
    const r = document.createRange(); r.selectNodeContents(n);
    const rects = r.getClientRects(); if (!rects.length) continue;
    if (rects[rects.length - 1].bottom <= limit) continue;
    // Binary search the first character whose line goes past the limit.
    let lo = 0, hi = n.data.length - 1;
    while (lo < hi) {
      const mid = (lo + hi) >> 1; r.setStart(n, mid); r.setEnd(n, mid + 1);
      const rr = r.getClientRects(); const b = rr.length ? rr[rr.length - 1].bottom : 0;
      if (b > limit) hi = mid; else lo = mid + 1;
    }
    // Back to the start of that word.
    let k = lo; while (k > 0 && !/\s/.test(n.data[k - 1])) k--;
    if (k === 0 && n === w.root.firstChild) { cut = null; break; }
    cut = [n, k];
    break;
  }
  if (!cut) return null;
  const r = document.createRange(); r.setStart(cut[0], cut[1]); r.setEndAfter(blk.lastChild);
  const frag = r.extractContents();
  if (!blk.textContent.trim() && !blk.querySelector('img')) { blk.append(frag); return null; }
  const rest = blk.cloneNode(false); rest.append(frag);
  rest.style.textIndent = '0'; rest.style.marginTop = '0'; blk.style.marginBottom = '0';
  return rest;
}
W.printLayout = async (box) => { W.paginate(box); const [w, h] = W.pageMm(); return { w, h }; };
W.togglePreview = () => {
  const pv = $('#preview');
  if (!pv.hidden) { pv.hidden = true; W.desk.hidden = false; $('#wPreview').classList.remove('on'); W.focus(); return; }
  W.paginate(pv);
  pv.prepend(el('div', { class: 'pvbar' }, el('b', { text: `Print layout · ${W.pageCount} page${W.pageCount === 1 ? '' : 's'}` }), el('span', { class: 'sp' }), el('button', { class: 'btn f', text: 'Print…', onclick: () => H.print() }), el('button', { class: 'btn f', text: 'Export PDF…', onclick: () => H.exportPdf() }), el('button', { class: 'btn primary f', text: 'Back to editing', onclick: () => W.togglePreview() })));
  pv.hidden = false; W.desk.hidden = true; $('#wPreview').classList.add('on');
};
// Dashed lines on the paper where each new page starts (worked out from the print layout).
W.guidesSoon = H.debounce(() => W.guides(), 1200);
W.guides = () => {
  const g = $('#guides'); if (!g) return;
  if (W.desk.hidden || !W.ed.childNodes.length) return;
  let pages;
  try { pages = W.paginate(null); } catch (e) { return; }
  g.innerHTML = '';
  $('#wPages').textContent = `${pages.length} page${pages.length === 1 ? '' : 's'}`;
  const z = W.zoomV / 100, pr = W.paper.getBoundingClientRect();
  pages.slice(1).forEach((pg, i) => {
    const s = pg.starts[0]; if (!s) return;
    const srcNode = W.ed.childNodes[s.src]; if (!srcNode || srcNode.nodeType !== 1) return;
    // The page starts at the top of that block, or (when a paragraph was split) at the first line moved over.
    let y = srcNode.getBoundingClientRect().top;
    const first = pg.body.firstChild;
    if (first && first._src === s.src && pages[i].body.lastChild && pages[i].body.lastChild._src === s.src) {
      const moved = (first.textContent || '').slice(0, 30);
      if (moved) { const ms = W.findAllIn(srcNode, moved); if (ms) y = ms.getBoundingClientRect().top; }
    }
    const top = (y - pr.top) / z - 4;
    g.append(el('div', { class: 'guide', style: { top: top + 'px' } }, el('span', { text: `Page ${i + 2}` })));
  });
};
W.findAllIn = (root, needle) => {
  const w = document.createTreeWalker(root, NodeFilter.SHOW_TEXT); let text = ''; const nodes = [];
  for (let n = w.nextNode(); n; n = w.nextNode()) { nodes.push([n, text.length]); text += n.data; }
  const i = text.lastIndexOf(needle); if (i < 0) return null;
  const at = nodes.filter(([, o]) => o <= i).pop(); if (!at) return null;
  const r = document.createRange(); r.setStart(at[0], Math.min(i - at[1], at[0].length)); r.setEnd(at[0], Math.min(i - at[1] + 1, at[0].length));
  return r;
};

// ------------------------------------------------------------------ menus
W.menus = () => [
  { label: 'File', items: H.fileMenu([{ label: 'Page setup…', run: () => W.pageSetup() }]) },
  { label: 'Edit', items: [
    { label: 'Undo', key: 'Ctrl+Z', run: () => W.undo() }, { label: 'Redo', key: 'Ctrl+Y / Ctrl+Shift+Z', run: () => W.redo() }, { sep: true },
    { label: 'Cut', key: 'Ctrl+X', run: () => { W.record(); document.execCommand('cut'); W.afterChange(); } }, { label: 'Copy', key: 'Ctrl+C', run: () => document.execCommand('copy') },
    { label: 'Paste', run: () => W.pasteFromMenu() }, { label: 'Paste as plain text', key: 'Ctrl+Shift+V', run: () => W.pasteFromMenu(true) },
    { label: 'Select all', key: 'Ctrl+A', run: () => { W.focus(); document.execCommand('selectAll'); } }, { sep: true },
    { label: 'Find…', key: 'Ctrl+F', run: () => W.find(true, false) }, { label: 'Find and replace…', key: 'Ctrl+H', run: () => W.find(true, true) },
    { label: 'Find next', key: 'F3', run: () => W.findStep(1) },
  ] },
  { label: 'View', items: [
    { label: 'Print layout', key: 'Ctrl+Alt+P', checked: () => !$('#preview').hidden, run: () => W.togglePreview() }, { sep: true },
    { label: 'Zoom in', key: 'Ctrl+=', run: () => W.zoom(W.zoomV + 10) }, { label: 'Zoom out', key: 'Ctrl+-', run: () => W.zoom(W.zoomV - 10) }, { label: 'Actual size', key: 'Ctrl+0', run: () => W.zoom(100) },
    { label: 'Fit page width', run: () => W.zoom(Math.floor((W.desk.clientWidth - 60) / (W.pageMm()[0] * MM) * 100)) }, { sep: true },
    { label: 'Light window', checked: () => H.settings.theme === 'light', run: () => { H.settings.theme = H.settings.theme === 'light' ? 'dark' : 'light'; H.applyTheme(); H.saveSettings(); } },
    { label: 'Mark spelling mistakes', checked: () => H.settings.spell !== false, run: () => { H.settings.spell = H.settings.spell === false; H.saveSettings(); W.spell.soon(); } },
  ] },
  { label: 'Insert', items: [
    { label: 'Picture…', run: () => W.insertPicture() }, { label: 'Table…', run: () => W.tablePicker($('#toolbar [title="Table"]')) }, { label: 'Link…', key: 'Ctrl+K', run: () => W.linkDialog() }, { sep: true },
    { label: 'Page break', key: 'Ctrl+Enter', run: () => W.pageBreak() }, { label: 'Horizontal line', run: () => W.insertHTML('<hr><p><br></p>') },
    { label: 'Date', run: () => W.insertHTML(esc(new Date().toLocaleDateString(undefined, { year: 'numeric', month: 'long', day: 'numeric' }))) },
    { label: 'Special character', sub: ['©', '®', '™', '€', '£', '¥', '°', '±', '×', '÷', '—', '–', '…', '•', '§', '¶', '→', '←', '✓', '½', '¼', '¾'].map((c) => ({ label: c, run: () => W.insertHTML(c) })) },
  ] },
  { label: 'Format', items: [
    { label: 'Bold', key: 'Ctrl+B', run: () => W.exec('bold') }, { label: 'Italic', key: 'Ctrl+I', run: () => W.exec('italic') }, { label: 'Underline', key: 'Ctrl+U', run: () => W.exec('underline') },
    { label: 'Strikethrough', key: 'Alt+Shift+5', run: () => W.exec('strikeThrough') }, { label: 'Superscript', key: 'Ctrl+.', run: () => W.exec('superscript') }, { label: 'Subscript', key: 'Ctrl+,', run: () => W.exec('subscript') }, { sep: true },
    { label: 'Paragraph style', sub: [['p', 'Normal text', 'Ctrl+Alt+0'], ['title', 'Title'], ['subtitle', 'Subtitle'], ['h1', 'Heading 1', 'Ctrl+Alt+1'], ['h2', 'Heading 2', 'Ctrl+Alt+2'], ['h3', 'Heading 3', 'Ctrl+Alt+3'], ['h4', 'Heading 4'], ['blockquote', 'Quote'], ['pre', 'Code']].map(([v, l, k]) => ({ label: l, key: k, run: () => W.setBlock(v) })) },
    { label: 'Align', sub: [{ label: 'Left', key: 'Ctrl+L', run: () => W.align('left') }, { label: 'Centre', key: 'Ctrl+E', run: () => W.align('center') }, { label: 'Right', key: 'Ctrl+R', run: () => W.align('right') }, { label: 'Justified', key: 'Ctrl+J', run: () => W.align('justify') }] },
    { label: 'Line spacing', sub: lineSpacingItems },
    { label: 'Bulleted list', key: 'Ctrl+Shift+8', run: () => W.exec('insertUnorderedList') }, { label: 'Numbered list', key: 'Ctrl+Shift+7', run: () => W.exec('insertOrderedList') },
    { label: 'Bigger text', key: 'Ctrl+]', run: () => W.applySize((+W.sizeSel.value || 11) + 1) }, { label: 'Smaller text', key: 'Ctrl+[', run: () => W.applySize(Math.max(1, (+W.sizeSel.value || 11) - 1)) },
    { label: 'Paragraph…', run: () => W.paragraphDialog() }, { label: 'Clear formatting', key: 'Ctrl+Space', run: () => W.clearFormat() }, { sep: true },
    { label: 'Table', sub: tableItems() }, { label: 'Page setup…', run: () => W.pageSetup() },
  ] },
  { label: 'Tools', items: [
    { label: 'Spelling…', key: 'F7', run: () => W.spell.dialog() }, { label: 'Word count', key: 'Ctrl+Shift+C', run: () => W.wordCountDialog() },
    { label: 'Your dictionary…', run: () => W.spell.manage() },
  ] },
  { label: 'Help', items: H.helpMenu([{ label: 'What files keep what', run: () => W.formatHelp() }, { sep: true }]) },
];
W.extraKeys = () => ({ 'Ctrl+Shift+Z': { run: () => W.redo() }, Escape: { run: () => { if (!$('#wfind').hidden) W.find(false); else if (!$('#preview').hidden) W.togglePreview(); else W.selectImage(null); } } });
W.formatHelp = () => H.dialog({
  title: 'What each file type keeps', wide: true, body: `<div class="fmt">
  <p><b>.docx (Word)</b>: text, fonts, sizes, colours, highlight, bold/italic/underline/strikethrough, headings and styles, lists, alignment, line spacing and paragraph spacing, tables, pictures, links, page size and margins, header, footer and page numbers. Word features Write doesn't have (comments, tracked changes, text boxes, footnotes, columns, fields other than page numbers) are dropped when Write opens the file.</p>
  <p><b>.odt (OpenDocument)</b>: the same as .docx, for LibreOffice.</p>
  <p><b>.rtf</b>: text formatting, alignment, lists (as bullets and numbers), tables, pictures (PNG and JPEG), page size and margins, header and footer.</p>
  <p><b>.html</b>: everything Write has (it's Write's own format underneath).</p>
  <p><b>.md (Markdown)</b>: headings, bold, italic, strikethrough, lists, links, tables, pictures and code. No fonts, colours, sizes or alignment.</p>
  <p><b>.txt</b>: just the words.</p></div>` });

// ------------------------------------------------------------------ new, open, save
W.templates = [
  { id: 'blank', name: 'Blank document', thumb: '<div class="mini"></div>' },
  { id: 'letter', name: 'Letter', desc: 'Addresses, date and a sign-off', thumb: '<div class="mini"><i style="width:40%;margin-left:auto"></i><i style="width:30%;margin-left:auto"></i><br><i style="width:45%"></i><i style="width:35%"></i><br><i></i><i></i><i style="width:70%"></i><br><i style="width:25%"></i></div>' },
  { id: 'report', name: 'Report', desc: 'Title, headings and a table', thumb: '<div class="mini"><b style="width:60%"></b><i style="width:40%"></i><br><u></u><i></i><i></i><u></u><i></i><s></s></div>' },
  { id: 'notes', name: 'Meeting notes', desc: 'Attendees, agenda, actions', thumb: '<div class="mini"><b style="width:50%"></b><u style="width:30%"></u><i></i><u style="width:30%"></u><i></i><i></i><u style="width:30%"></u><i style="width:60%"></i></div>' },
  { id: 'resume', name: 'Résumé', desc: 'Experience, education, skills', thumb: '<div class="mini"><b style="width:55%;margin:auto"></b><i style="width:70%;margin:2px auto"></i><u></u><i></i><i></i><u></u><i></i><u></u><i style="width:60%"></i></div>' },
];
const TEMPLATES = {
  blank: '<p><br></p>',
  letter: `<p style="text-align:right">Your name<br>Street and number<br>Town, postcode</p><p style="text-align:right">${new Date().toLocaleDateString(undefined, { year: 'numeric', month: 'long', day: 'numeric' })}</p><p>Their name<br>Company<br>Street and number<br>Town, postcode</p><p>Dear …,</p><p>Write your letter here.</p><p><br></p><p>Kind regards,</p><p><br></p><p>Your name</p>`,
  report: '<p class="title">Report title</p><p class="subtitle">A short line about what this is</p><h1>Summary</h1><p>The main points in a few sentences.</p><h1>Details</h1><h2>First part</h2><p>Write here.</p><table class="t"><tbody><tr><th>Item</th><th>Value</th><th>Notes</th></tr><tr><td>One</td><td>10</td><td><br></td></tr><tr><td>Two</td><td>20</td><td><br></td></tr></tbody></table><h2>Second part</h2><p>Write here.</p><h1>Next steps</h1><ul><li>First</li><li>Second</li></ul>',
  notes: `<p class="title">Meeting notes</p><p><b>Date:</b> ${new Date().toLocaleDateString()}<br><b>Attendees:</b> </p><h2>Agenda</h2><ol><li><br></li></ol><h2>Notes</h2><p><br></p><h2>Actions</h2><table class="t"><tbody><tr><th>What</th><th>Who</th><th>When</th></tr><tr><td><br></td><td><br></td><td><br></td></tr></tbody></table>`,
  resume: '<p class="title" style="text-align:center">Your Name</p><p style="text-align:center">email@example.com · phone · town</p><h2>Experience</h2><p><b>Job title</b>, Company — 2022 to now</p><ul><li>What you did and what came of it</li></ul><h2>Education</h2><p><b>Course</b>, School — year</p><h2>Skills</h2><p>Skill, skill, skill</p>',
};
W.newDoc = (tpl) => {
  W.page = W.defaultPage();
  if (H.settings.writePage) W.page.size = H.settings.writePage;
  else if (/^en-(US|CA)|^es-(MX|US)|^fil/.test(navigator.language || '')) W.page.size = 'Letter';
  else W.page.size = 'A4';
  W.ed.innerHTML = TEMPLATES[tpl || 'blank'] || TEMPLATES.blank;
  U.stack = []; U.redo = [];
  W.applyPage(); W.zoom(W.zoomV);
  W.countSoon(); W.spell.soon();
  const r = document.createRange(); r.selectNodeContents(W.ed.firstElementChild || W.ed); r.collapse(true);
  getSelection().removeAllRanges(); getSelection().addRange(r);
  W.focus();
};
// Puts a document in the editor: html (cleaned here) and page settings.
W.setDoc = (html, page) => {
  W.page = Object.assign(W.defaultPage(), page || {});
  W.page.margins = Object.assign(W.defaultPage().margins, (page && page.margins) || {});
  W.ed.innerHTML = H.clean(html) || '<p><br></p>';
  // Loose text and inline elements at the top level go into paragraphs.
  let p = null;
  for (const n of Array.from(W.ed.childNodes)) {
    const inline = n.nodeType === 3 || (n.nodeType === 1 && !/^(P|H[1-6]|UL|OL|TABLE|BLOCKQUOTE|PRE|HR|DIV|FIGURE)$/.test(n.tagName));
    if (inline) { if (n.nodeType === 3 && !n.data.trim() && !p) { n.remove(); continue; } if (!p) { p = document.createElement('p'); n.before(p); } p.append(n); }
    else p = null;
  }
  $$('div', W.ed).forEach((d) => { if (d.parentNode === W.ed && !d.querySelector('p,div,table,ul,ol,h1,h2,h3')) { const q = document.createElement('p'); q.style.cssText = d.style.cssText; q.append(...d.childNodes); d.replaceWith(q); } });
  $$('table', W.ed).forEach((t) => { if (!t.classList.contains('t')) t.classList.add('t'); });
  if (!W.ed.firstChild) W.ed.innerHTML = '<p><br></p>';
  U.stack = []; U.redo = [];
  W.applyPage(); W.zoom(W.zoomV); W.countSoon(); W.spell.soon();
  W.desk.scrollTop = 0;
};
W.load = async (bytes, ext) => {
  const notes = [];
  if (ext === 'docx') {
    await H.need('vendor/jszip.min.js', 'formats/docx.js');
    const r = await WriteFormats.docx.read(bytes);
    W.setDoc(r.html, r.page); if (r.notes) notes.push(...r.notes);
  } else if (ext === 'odt') {
    await H.need('vendor/jszip.min.js', 'formats/odt.js');
    const r = await WriteFormats.odt.read(bytes); W.setDoc(r.html, r.page);
  } else if (ext === 'rtf') {
    await H.need('formats/rtf.js');
    const r = WriteFormats.rtf.read(H.bytesToText(bytes)); W.setDoc(r.html, r.page);
  } else if (ext === 'md' || ext === 'markdown') {
    await H.need('vendor/marked.umd.js');
    W.setDoc(marked.parse(H.bytesToText(bytes), { gfm: true, breaks: false }));
  } else if (ext === 'html' || ext === 'htm') {
    const doc = new DOMParser().parseFromString(H.bytesToText(bytes), 'text/html');
    let page = null;
    const meta = doc.querySelector('meta[name="hearth-page"]');
    if (meta) { try { page = JSON.parse(meta.getAttribute('content')); } catch {} }
    W.setDoc(doc.body ? doc.body.innerHTML : '', page);
  } else if (ext === 'doc') {
    throw new Error('Old Word documents (.doc, from Word 97–2003) can\'t be opened. Open it in Word or LibreOffice and save it as .docx.');
  } else {
    W.setDoc(textToHtml(H.bytesToText(bytes)));
    W.page.font = 'Liberation Mono'; W.applyPage();
  }
  return notes;
};
// The document as clean HTML for the writers (no editing marks).
W.exportRoot = () => {
  const root = W.ed.cloneNode(true);
  $$('img.sel', root).forEach((i) => i.classList.remove('sel'));
  $$('span', root).forEach((s) => { if (s.textContent === '​' && !s.children.length) s.remove(); });
  root.normalize();
  return root;
};
W.save = async (ext) => {
  const root = W.exportRoot();
  if (ext === 'docx') { await H.need('vendor/jszip.min.js', 'vendor/docx.iife.js', 'formats/docx.js'); return WriteFormats.docx.write(root, W.page); }
  if (ext === 'odt') { await H.need('vendor/jszip.min.js', 'formats/odt.js'); return WriteFormats.odt.write(root, W.page); }
  if (ext === 'rtf') { await H.need('formats/rtf.js'); return H.textToBytes(await WriteFormats.rtf.write(root, W.page)); }
  if (ext === 'html' || ext === 'htm') return H.textToBytes(W.toHtmlFile(root));
  if (ext === 'md' || ext === 'markdown') { await H.need('vendor/turndown.browser.umd.js'); return H.textToBytes(W.toMarkdown(root)); }
  return H.textToBytes(W.toText(root));
};
W.toHtmlFile = (root) => {
  const p = W.page, [w, h] = W.pageMm();
  return `<!doctype html>
<html><head><meta charset="utf-8"><meta name="generator" content="Hearth Write">
<meta name="hearth-page" content="${esc(JSON.stringify(p))}">
<title>${esc(H.stem(H.doc.name || 'Document'))}</title>
<style>
@page { size: ${w}mm ${h}mm; margin: ${p.margins.t}mm ${p.margins.r}mm ${p.margins.b}mm ${p.margins.l}mm; }
body { font-family: ${fontCss(p.font)}; font-size: ${p.fontSize}pt; line-height: ${p.lineHeight}; max-width: ${w - p.margins.l - p.margins.r}mm; margin: 2em auto; color: #000; }
p { margin: 0 0 ${p.spaceAfter ?? 8}pt; } table { border-collapse: collapse; } td, th { border: 1px solid #999; padding: 4px 6px; vertical-align: top; }
.title { font-size: 26pt; } .subtitle { font-size: 15pt; color: #555; } hr.page-break { page-break-after: always; border: 0; } img { max-width: 100%; }
</style></head>
<body>
${root.innerHTML}
</body></html>
`;
};
W.toText = (root) => {
  const out = [];
  const walk = (n, prefix) => {
    for (const c of n.childNodes) {
      if (c.nodeType === 3) { if (c.data.trim()) out.push(prefix + c.data.trim()); continue; }
      if (c.nodeType !== 1) continue;
      const t = c.tagName;
      if (t === 'UL' || t === 'OL') { let i = parseInt(c.getAttribute('start') || '1', 10); for (const li of c.children) { out.push(prefix + (t === 'OL' ? `${i++}. ` : '• ') + inlineText(li)); $$(':scope > ul, :scope > ol', li).forEach((sub) => walk({ childNodes: [sub] }, prefix + '    ')); } }
      else if (t === 'TABLE') { for (const tr of $$('tr', c)) out.push(prefix + $$('td,th', tr).map((x) => x.innerText.replace(/\s+/g, ' ').trim()).join('\t')); out.push(''); }
      else if (t === 'HR') out.push(c.classList.contains('page-break') ? '\f' : '');
      else out.push(prefix + inlineText(c));
    }
  };
  const inlineText = (e) => { const c = e.cloneNode(true); $$('ul,ol', c).forEach((x) => x.remove()); $$('br', c).forEach((b) => b.replaceWith('\n')); return c.textContent.replace(/​/g, ''); };
  walk(root, '');
  return out.join('\n').replace(/\n{3,}/g, '\n\n') + '\n';
};
W.toMarkdown = (root) => {
  const td = new TurndownService({ headingStyle: 'atx', bulletListMarker: '-', codeBlockStyle: 'fenced', emDelimiter: '*' });
  td.addRule('strike', { filter: ['s', 'strike', 'del'], replacement: (c) => `~~${c}~~` });
  td.addRule('underline', { filter: ['u'], replacement: (c) => c });
  td.addRule('title', { filter: (n) => n.nodeName === 'P' && n.classList.contains('title'), replacement: (c) => `\n\n# ${c}\n\n` });
  td.addRule('pagebreak', { filter: (n) => n.nodeName === 'HR' && n.classList.contains('page-break'), replacement: () => '\n\n---\n\n' });
  td.addRule('table', {
    filter: 'table', replacement: (_, node) => {
      const rows = $$('tr', node).map((tr) => $$('td,th', tr).map((c) => td.turndown(c.innerHTML).replace(/\n+/g, ' ').replace(/\|/g, '\\|').trim()));
      if (!rows.length) return '';
      const n = Math.max(...rows.map((r) => r.length));
      const line = (r) => '| ' + Array.from({ length: n }, (_, i) => r[i] || '').join(' | ') + ' |';
      return '\n\n' + [line(rows[0]), '|' + ' --- |'.repeat(n), ...rows.slice(1).map(line)].join('\n') + '\n\n';
    },
  });
  td.addRule('span-tab', { filter: (n) => n.nodeName === 'SPAN' && n.classList.contains('tab'), replacement: () => '    ' });
  return td.turndown(root.innerHTML).replace(/​/g, '') + '\n';
};
W.snapshot = () => JSON.stringify({ html: W.exportRoot().innerHTML, page: W.page });
W.restore = (s) => { const o = JSON.parse(s); W.setDoc(o.html, o.page); };

// ------------------------------------------------------------------ spelling
// Typo.js with the Hunspell en_US dictionary, in a worker (loading the dictionary takes a moment). Mistakes are
// underlined with the CSS Highlight API where the web view has it; right-click and Tools › Spelling work anyway.
W.spell = (() => {
  const S = { cache: new Map(), worker: null, ready: false, pending: new Map(), seq: 0 };
  const user = () => new Set((H.settings.dictionary || []).map((w) => w.toLowerCase()));
  S.ignored = new Set();
  S.start = () => {
    if (S.worker) return;
    try {
      S.worker = new Worker('spell-worker.js');
      S.worker.onmessage = (e) => {
        const m = e.data;
        if (m.type === 'ready') { S.ready = true; S.soon(); $('#wSpell') && ($('#wSpell').textContent = ''); return; }
        if (m.type === 'error') { $('#wSpell').textContent = 'Spelling unavailable'; H.invoke('log', { message: 'spell: ' + m.error }); return; }
        const p = S.pending.get(m.id); if (p) { S.pending.delete(m.id); p(m.result); }
      };
    } catch (e) { H.invoke('log', { message: 'spell worker: ' + e }); }
  };
  const ask = (type, data) => new Promise((res) => { if (!S.worker || !S.ready) return res(null); const id = ++S.seq; S.pending.set(id, res); S.worker.postMessage({ id, type, data }); });
  const WORD = /[A-Za-zÀ-ÖØ-öø-ÿ]+(?:['’][A-Za-z]+)*/g;
  const skip = (w) => w.length < 2 || /^[A-Z]{2,}$/.test(w) || user().has(w.toLowerCase()) || S.ignored.has(w);
  S.isWrong = (w) => { w = w.replace(/’/g, "'"); return !skip(w) && S.cache.get(w) === false; };
  S.soon = H.debounce(() => S.run(), 700);
  S.run = async () => {
    const hl = window.CSS && CSS.highlights && window.Highlight;
    if (H.settings.spell === false || !S.ready) { if (hl) CSS.highlights.delete('spell'); return; }
    const nodes = []; const w = document.createTreeWalker(W.ed, NodeFilter.SHOW_TEXT);
    for (let n = w.nextNode(); n && nodes.length < 20000; n = w.nextNode()) if (!n.parentElement.closest('pre,code,a')) nodes.push(n);
    const unknown = new Set();
    for (const n of nodes) for (const m of n.data.matchAll(WORD)) { const x = m[0].replace(/’/g, "'"); if (!skip(x) && !S.cache.has(x)) unknown.add(x); }
    if (unknown.size) { const res = await ask('check', Array.from(unknown)); if (res) for (const [k, v] of Object.entries(res)) S.cache.set(k, v); }
    if (!hl) return;
    const ranges = [];
    for (const n of nodes) for (const m of n.data.matchAll(WORD)) if (S.isWrong(m[0])) { const r = new Range(); r.setStart(n, m.index); r.setEnd(n, m.index + m[0].length); ranges.push(r); }
    if (ranges.length) CSS.highlights.set('spell', new Highlight(...ranges)); else CSS.highlights.delete('spell');
  };
  S.wordAt = (x, y) => {
    let r = document.caretRangeFromPoint ? document.caretRangeFromPoint(x, y) : null;
    if (!r && document.caretPositionFromPoint) { const p = document.caretPositionFromPoint(x, y); if (p) { r = document.createRange(); r.setStart(p.offsetNode, p.offset); } }
    if (!r || r.startContainer.nodeType !== 3) return null;
    const n = r.startContainer, t = n.data; let a = r.startOffset, b = a;
    const isW = (c) => /[A-Za-zÀ-ÖØ-öø-ÿ'’]/.test(c || '');
    while (a > 0 && isW(t[a - 1])) a--; while (b < t.length && isW(t[b])) b++;
    const text = t.slice(a, b).replace(/^['’]+|['’]+$/g, '');
    if (!text) return null;
    const range = document.createRange(); range.setStart(n, a); range.setEnd(n, a + t.slice(a, b).length);
    return { text, range };
  };
  S.suggest = async (w) => (await ask('suggest', w.replace(/’/g, "'"))) || [];
  S.add = (w) => { H.settings.dictionary = Array.from(new Set([...(H.settings.dictionary || []), w])); H.saveSettings(); S.run(); };
  S.ignore = (w) => { S.ignored.add(w); S.run(); };
  S.manage = async () => {
    const v = await H.form('Your dictionary', [{ id: 'words', label: 'Words Write knows as spelt right (one a line)', type: 'textarea', rows: 10, value: (H.settings.dictionary || []).join('\n') }], 'Save');
    if (v) { H.settings.dictionary = v.words.split(/\n/).map((x) => x.trim()).filter(Boolean); H.saveSettings(); S.run(); }
  };
  // Tools › Spelling: goes through the mistakes one by one.
  S.dialog = async () => {
    if (!S.ready) { H.toast('The dictionary is still loading…'); S.start(); return; }
    await S.run();
    const list = [];
    const w = document.createTreeWalker(W.ed, NodeFilter.SHOW_TEXT);
    for (let n = w.nextNode(); n; n = w.nextNode()) for (const m of n.data.matchAll(WORD)) if (S.isWrong(m[0])) { const r = document.createRange(); r.setStart(n, m.index); r.setEnd(n, m.index + m[0].length); list.push({ r, w: m[0] }); }
    if (!list.length) return H.alert('Spelling', 'No spelling mistakes found.');
    let i = 0;
    const box = el('div', { class: 'spellbox' });
    const show = async () => {
      while (i < list.length && !S.isWrong(list[i].w)) i++;
      if (i >= list.length) { box.innerHTML = '<p>Done: that was the last one.</p>'; return; }
      const it = list[i];
      const s = getSelection(); s.removeAllRanges(); s.addRange(it.r);
      const rect = it.r.getBoundingClientRect(), dr = W.desk.getBoundingClientRect();
      if (rect.top < dr.top || rect.bottom > dr.bottom) W.desk.scrollTop += rect.top - dr.top - dr.height / 3;
      const sug = await S.suggest(it.w);
      box.innerHTML = '';
      box.append(el('p', {}, 'Not in the dictionary: ', el('b', { text: it.w }), ` (${i + 1} of ${list.length})`));
      const sl = el('div', { class: 'sugs' });
      (sug.length ? sug : []).slice(0, 8).forEach((x) => sl.append(el('button', { class: 'btn f', 'data-f': true, text: x, onclick: () => { W.record(); it.r.deleteContents(); it.r.insertNode(document.createTextNode(x)); W.afterChange(); i++; show(); } })));
      if (!sug.length) sl.append(el('span', { class: 'note', text: 'No suggestions.' }));
      box.append(sl, el('div', { class: 'row' },
        el('button', { class: 'btn f', 'data-f': true, text: 'Skip', onclick: () => { i++; show(); } }),
        el('button', { class: 'btn f', 'data-f': true, text: 'Ignore all', onclick: () => { S.ignore(it.w); i++; show(); } }),
        el('button', { class: 'btn f', 'data-f': true, text: 'Add to dictionary', onclick: () => { S.add(it.w); i++; show(); } })));
    };
    show();
    await H.dialog({ title: 'Spelling', body: box, buttons: [{ label: 'Close', value: true, primary: true }] });
    S.run();
  };
  return S;
})();

// The controller in Write: d-pad and sticks scroll the document (it's for reading; typing needs a keyboard).
W.pad = (b) => {
  if ((H._dialogs && H._dialogs.length) || $$('.menu').length || !$('#start').hidden) return false;
  if (b === 'LB' || b === 'RB') { W.desk.scrollBy({ top: (b === 'RB' ? 1 : -1) * W.desk.clientHeight * 0.9, behavior: 'smooth' }); return true; }
  if (b === 'Y') { W.togglePreview(); return true; }
  return false;
};

W.SIZES = SIZES; W.fontCss = fontCss; W.firstFont = firstFont; W.MM = MM;
H.register(W);
})();
