// Hearth Write ↔ .docx. Reading is Write's own (JSZip + the XML inside), so fonts, sizes, colours, alignment,
// spacing, lists, tables, pictures, links, page setup, header, footer and page numbers come through; mammoth.js
// is the fallback when a file is too unusual for it. Writing uses the docx library.
// Not kept: comments, tracked changes (insertions are kept as text, deletions dropped), footnotes, text boxes,
// shapes, charts, columns, sections after the first, fields other than page numbers (their last shown text stays).
'use strict';
window.WriteFormats = window.WriteFormats || {};
(() => {
const WNS = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main';
const RNS = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships';
const kids = (e, name) => e ? Array.from(e.children).filter((c) => !name || c.localName === name) : [];
const kid = (e, name) => e ? Array.from(e.children).find((c) => c.localName === name) || null : null;
const wa = (e, name) => e ? (e.getAttributeNS(WNS, name) ?? e.getAttribute('w:' + name) ?? e.getAttribute(name)) : null;
const ra = (e, name) => e ? (e.getAttributeNS(RNS, name) ?? e.getAttribute('r:' + name)) : null;
const desc = (e, name) => e ? Array.from(e.getElementsByTagNameNS('*', name)) : [];
const on = (e) => { if (!e) return undefined; const v = wa(e, 'val'); return !(v === '0' || v === 'false' || v === 'off' || v === 'none'); };
const esc = H.esc;
const HL = { yellow: 'FFFF00', green: '00FF00', cyan: '00FFFF', magenta: 'FF00FF', blue: '0000FF', red: 'FF0000', darkBlue: '000080', darkCyan: '008080', darkGreen: '008000', darkMagenta: '800080', darkRed: '800000', darkYellow: '808000', darkGray: '808080', lightGray: 'C0C0C0', black: '000000', white: 'FFFFFF' };
const xml = (s) => (s ? new DOMParser().parseFromString(s, 'application/xml') : null);
const norm = (path) => { const out = []; for (const p of path.split('/')) { if (p === '..') out.pop(); else if (p && p !== '.') out.push(p); } return out.join('/'); };

async function readRels(zip, path, dir) {
  const map = {};
  const f = zip.file(path); if (!f) return map;
  const d = xml(await f.async('string'));
  for (const r of d.getElementsByTagName('Relationship')) {
    const t = r.getAttribute('Target') || '';
    const ext = r.getAttribute('TargetMode') === 'External';
    map[r.getAttribute('Id')] = { type: (r.getAttribute('Type') || '').split('/').pop(), target: ext ? t : (t.startsWith('/') ? norm(t) : norm(dir + t)), external: ext };
  }
  return map;
}

// Run properties: {b, i, u, s, color, sz (pt), font, bg, va, caps}
function rpr(e) {
  const o = {};
  if (!e) return o;
  const b = kid(e, 'b'); if (b) o.b = on(b);
  const i = kid(e, 'i'); if (i) o.i = on(i);
  const u = kid(e, 'u'); if (u) o.u = wa(u, 'val') !== 'none' && on(u);
  const s = kid(e, 'strike') || kid(e, 'dstrike'); if (s) o.s = on(s);
  const c = kid(e, 'color'); if (c && wa(c, 'val') && wa(c, 'val') !== 'auto') o.color = wa(c, 'val');
  const sz = kid(e, 'sz'); if (sz && wa(sz, 'val')) o.sz = parseInt(wa(sz, 'val'), 10) / 2;
  const f = kid(e, 'rFonts'); if (f) { const n = wa(f, 'ascii') || wa(f, 'hAnsi') || wa(f, 'cs'); if (n) o.font = n; }
  const hl = kid(e, 'highlight'); if (hl && HL[wa(hl, 'val')]) o.bg = HL[wa(hl, 'val')];
  const shd = kid(e, 'shd'); if (shd && wa(shd, 'fill') && !/^(auto|FFFFFF)$/i.test(wa(shd, 'fill')) && !o.bg) o.bg = wa(shd, 'fill');
  const va = kid(e, 'vertAlign'); if (va) o.va = wa(va, 'val');
  const caps = kid(e, 'caps'); if (caps) o.caps = on(caps);
  const vanish = kid(e, 'vanish'); if (vanish) o.hidden = on(vanish);
  return o;
}
const merge = (...os) => Object.assign({}, ...os.filter(Boolean));

function parseStyles(d) {
  const S = { para: {}, char: {}, defRun: {}, defPara: {}, byName: {} };
  if (!d) return S;
  const dd = desc(d, 'docDefaults')[0];
  if (dd) { S.defRun = rpr(kid(kid(dd, 'rPrDefault'), 'rPr')); S.defPara = ppr(kid(kid(dd, 'pPrDefault'), 'pPr')); }
  const raw = {};
  for (const s of desc(d, 'style')) {
    const id = wa(s, 'styleId'); const type = wa(s, 'type');
    raw[id] = { id, type, name: (wa(kid(s, 'name'), 'val') || id), basedOn: wa(kid(s, 'basedOn'), 'val'), r: rpr(kid(s, 'rPr')), p: ppr(kid(s, 'pPr')), def: wa(s, 'default') === '1' };
  }
  const resolve = (id, seen = new Set()) => {
    const s = raw[id]; if (!s || seen.has(id)) return { r: {}, p: {} };
    seen.add(id);
    const base = s.basedOn ? resolve(s.basedOn, seen) : { r: {}, p: {} };
    return { name: s.name, r: merge(base.r, s.r), p: merge(base.p, s.p) };
  };
  for (const id of Object.keys(raw)) {
    const s = raw[id], res = resolve(id);
    const entry = { id, name: s.name, r: res.r, p: res.p };
    if (s.type === 'character') S.char[id] = entry; else if (s.type === 'paragraph') S.para[id] = entry;
    if (s.type === 'paragraph' && s.def) S.normal = entry;
  }
  return S;
}
// Paragraph properties: {jc, line, before, after, left, first, numId, ilvl, keep}
function ppr(e) {
  const o = {};
  if (!e) return o;
  const jc = wa(kid(e, 'jc'), 'val'); if (jc) o.jc = jc;
  const sp = kid(e, 'spacing');
  if (sp) {
    const line = wa(sp, 'line'), rule = wa(sp, 'lineRule');
    if (line && (!rule || rule === 'auto')) o.line = parseInt(line, 10) / 240;
    if (wa(sp, 'before') != null) o.before = parseInt(wa(sp, 'before'), 10) / 20;
    if (wa(sp, 'after') != null) o.after = parseInt(wa(sp, 'after'), 10) / 20;
  }
  const ind = kid(e, 'ind');
  if (ind) {
    const l = wa(ind, 'left') ?? wa(ind, 'start'); if (l != null) o.left = parseInt(l, 10);
    if (wa(ind, 'firstLine') != null) o.first = parseInt(wa(ind, 'firstLine'), 10);
    if (wa(ind, 'hanging') != null) o.first = -parseInt(wa(ind, 'hanging'), 10);
  }
  const np = kid(e, 'numPr');
  if (np) { o.numId = wa(kid(np, 'numId'), 'val'); o.ilvl = parseInt(wa(kid(np, 'ilvl'), 'val') || '0', 10); }
  const bdr = kid(e, 'pBdr'); if (bdr && kid(bdr, 'bottom')) o.bottomBorder = true;
  const shd = kid(e, 'shd'); if (shd && wa(shd, 'fill') && !/^(auto|FFFFFF)$/i.test(wa(shd, 'fill'))) o.bg = wa(shd, 'fill');
  return o;
}
function parseNumbering(d) {
  const N = { abs: {}, num: {} };
  if (!d) return N;
  for (const a of desc(d, 'abstractNum')) {
    const lv = {};
    for (const l of kids(a, 'lvl')) lv[wa(l, 'ilvl')] = { fmt: wa(kid(l, 'numFmt'), 'val') || 'decimal', start: parseInt(wa(kid(l, 'start'), 'val') || '1', 10), text: wa(kid(l, 'lvlText'), 'val') };
    N.abs[wa(a, 'abstractNumId')] = lv;
  }
  for (const n of desc(d, 'num')) {
    const over = {};
    for (const o of kids(n, 'lvlOverride')) { const s = kid(o, 'startOverride'); if (s) over[wa(o, 'ilvl')] = parseInt(wa(s, 'val'), 10); }
    N.num[wa(n, 'numId')] = { abs: wa(kid(n, 'abstractNumId'), 'val'), over };
  }
  return N;
}
const listFmt = (N, numId, ilvl) => {
  const n = N.num[numId]; if (!n) return { ordered: false };
  const l = (N.abs[n.abs] || {})[ilvl] || { fmt: 'bullet' };
  const type = { lowerLetter: 'a', upperLetter: 'A', lowerRoman: 'i', upperRoman: 'I' }[l.fmt] || '1';
  return { ordered: l.fmt !== 'bullet' && l.fmt !== 'none', type, start: n.over[ilvl] ?? l.start };
};

// ------------------------------------------------------------------ reading
async function read(bytes) {
  try { return await readOwn(bytes); }
  catch (e) {
    console.warn('docx: own reader failed, using mammoth', e);
    H.invoke('log', { message: 'docx reader: ' + (e && e.stack || e) });
    await H.need('vendor/mammoth.browser.min.js');
    const r = await mammoth.convertToHtml({ arrayBuffer: bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) }, { styleMap: ['u => u', 'strike => s'] });
    return { html: r.value, page: null, notes: ['This file was opened in a simpler way: some formatting (fonts, colours, sizes, alignment) may be missing.'] };
  }
}
async function readOwn(bytes) {
  const zip = await JSZip.loadAsync(bytes);
  const str = async (p) => { const f = zip.file(p); return f ? f.async('string') : null; };
  let main = 'word/document.xml';
  const rr = xml(await str('_rels/.rels'));
  if (rr) for (const r of rr.getElementsByTagName('Relationship')) if (/\/officeDocument$/.test(r.getAttribute('Type'))) main = norm(r.getAttribute('Target'));
  const dir = main.replace(/[^/]*$/, '');
  const docXml = await str(main);
  if (!docXml) throw new Error('This isn\'t a Word document (no word/document.xml inside).');
  const doc = xml(docXml);
  if (doc.getElementsByTagName('parsererror').length) throw new Error('The document\'s XML is damaged.');
  const ctx = {
    zip, dir,
    rels: await readRels(zip, dir + '_rels/' + main.split('/').pop() + '.rels', dir),
    styles: parseStyles(xml(await str(dir + 'styles.xml'))),
    numbering: parseNumbering(xml(await str(dir + 'numbering.xml'))),
    imgs: {}, notes: new Set(),
  };
  const st = ctx.styles;
  const normal = st.normal || { r: {}, p: {} };
  ctx.base = merge(st.defRun, normal.r);
  const basePara = merge(st.defPara, normal.p);
  const body = desc(doc, 'body')[0];
  const root = document.createElement('div');
  await blocks(kids(body), root, ctx);
  // Page setup from the last section.
  const page = {};
  if (ctx.base.font) page.font = ctx.base.font;
  if (ctx.base.sz) page.fontSize = ctx.base.sz;
  if (basePara.line) page.lineHeight = Math.round(basePara.line * 100) / 100;
  if (basePara.after != null) page.spaceAfter = basePara.after;
  const sect = kids(body, 'sectPr').pop() || desc(body, 'sectPr').pop();
  if (sect) {
    const sz = kid(sect, 'pgSz');
    if (sz) {
      let w = parseInt(wa(sz, 'w'), 10) / 1440 * 25.4, h = parseInt(wa(sz, 'h'), 10) / 1440 * 25.4;
      const land = wa(sz, 'orient') === 'landscape' || w > h;
      if (land && w < h) [w, h] = [h, w];
      page.orient = land ? 'landscape' : 'portrait';
      const pw = Math.min(w, h), ph = Math.max(w, h);
      const name = Object.entries(W_SIZES()).find(([, d]) => Math.abs(d[0] - pw) < 2 && Math.abs(d[1] - ph) < 2);
      if (name) page.size = name[0]; else { page.size = 'Custom'; page.w = pw; page.h = ph; }
    }
    const m = kid(sect, 'pgMar');
    if (m) page.margins = { t: tw(wa(m, 'top')), r: tw(wa(m, 'right')), b: tw(wa(m, 'bottom')), l: tw(wa(m, 'left')) };
    for (const [kind, refName] of [['header', 'headerReference'], ['footer', 'footerReference']]) {
      const ref = kids(sect, refName).find((x) => (wa(x, 'type') || 'default') === 'default') || kids(sect, refName)[0];
      const rel = ref && ctx.rels[ra(ref, 'id')];
      const hx = rel && xml(await str(rel.target));
      if (!hx) continue;
      const texts = [];
      for (const p of desc(hx, 'p')) {
        const instr = desc(p, 'instrText').map((x) => x.textContent).join(' ') + ' ' + desc(p, 'fldSimple').map((x) => wa(x, 'instr')).join(' ');
        if (/\bPAGE\b/.test(instr)) {
          const jc = wa(kid(kid(p, 'pPr'), 'jc'), 'val');
          page.numbers = kind === 'header' ? 'header-right' : (jc === 'right' || jc === 'end' ? 'footer-right' : 'footer-center');
          const t = textWithoutFields(p).replace(/\bPage\b|\bof\b/gi, '').replace(/\s+/g, ' ').trim();
          if (t) texts.push(t);
          continue;
        }
        const t = desc(p, 't').map((x) => x.textContent).join('');
        if (t.trim()) texts.push(t.trim());
      }
      page[kind] = texts.join('  ');
    }
  }
  return { html: root.innerHTML, page, notes: Array.from(ctx.notes) };
}
const tw = (v) => Math.round((parseInt(v || '0', 10) / 1440 * 25.4) * 10) / 10;
const W_SIZES = () => (H.apps.write && H.apps.write.SIZES) || { A4: [210, 297], Letter: [215.9, 279.4] };
function textWithoutFields(p) {
  let out = '', state = 0;
  for (const r of desc(p, 'r')) {
    const fc = kid(r, 'fldChar');
    if (fc) { const t = wa(fc, 'fldCharType'); state = t === 'begin' ? 1 : t === 'separate' ? 2 : 0; continue; }
    if (state) continue;
    out += desc(r, 't').map((x) => x.textContent).join('');
  }
  return out;
}

// Body-level elements (paragraphs, tables, content controls) into `out`, grouping list paragraphs into lists.
async function blocks(nodes, out, ctx) {
  const stack = []; // [{list, numId, li}]
  const close = (n) => { while (stack.length > n) stack.pop(); };
  for (const n of nodes) {
    const name = n.localName;
    if (name === 'p') {
      const r = await para(n, ctx);
      if (!r) continue;
      if (r.list) {
        const { numId, ilvl } = r.list;
        const fmt = listFmt(ctx.numbering, numId, ilvl);
        close(ilvl + 1);
        if (stack.length === ilvl + 1 && (stack[ilvl].numId !== numId || (stack[ilvl].list.tagName === 'OL') !== fmt.ordered)) close(ilvl);
        while (stack.length < ilvl + 1) {
          const lvl = stack.length;
          const f = lvl === ilvl ? fmt : listFmt(ctx.numbering, numId, lvl);
          const list = document.createElement(f.ordered ? 'ol' : 'ul');
          if (f.ordered && f.type !== '1') list.setAttribute('type', f.type);
          if (f.ordered && f.start > 1) list.setAttribute('start', f.start);
          const parent = lvl === 0 ? out : (stack[lvl - 1].li || stack[lvl - 1].list);
          parent.append(list);
          stack.push({ list, numId, li: null });
        }
        const li = document.createElement('li');
        while (r.el.firstChild) li.append(r.el.firstChild);
        if (r.el.getAttribute('style')) li.setAttribute('style', r.el.getAttribute('style'));
        li.style.marginLeft = ''; li.style.textIndent = '';
        if (!li.getAttribute('style')) li.removeAttribute('style');
        if (!li.firstChild) li.append(document.createElement('br'));
        stack[ilvl].list.append(li);
        stack[ilvl].li = li;
        continue;
      }
      close(0);
      out.append(r.el);
      if (r.pageBreakAfter) { const hr = document.createElement('hr'); hr.className = 'page-break'; out.append(hr); }
    } else if (name === 'tbl') { close(0); out.append(await table(n, ctx)); }
    else if (name === 'sdt') { close(0); await blocks(kids(kid(n, 'sdtContent')), out, ctx); }
    else if (name === 'customXml' || name === 'ins' || name === 'smartTag') { close(0); await blocks(kids(n), out, ctx); }
    else if (name === 'altChunk') ctx.notes.add('Part of this document (an embedded file) couldn\'t be shown.');
  }
}

async function para(p, ctx) {
  const pPr = kid(p, 'pPr');
  const styleId = wa(kid(pPr, 'pStyle'), 'val');
  const style = ctx.styles.para[styleId] || null;
  const direct = ppr(pPr);
  const pp = merge(style && style.p, direct);
  const sname = ((style && style.name) || styleId || '').toLowerCase();
  let tag = 'p', cls = null, heading = false;
  let m;
  if ((m = /^heading\s*(\d)$/.exec(sname)) || (m = /^heading(\d)$/i.exec(styleId || ''))) { tag = 'h' + Math.min(6, +m[1]); heading = true; }
  else if (sname === 'title') { cls = 'title'; heading = true; }
  else if (sname === 'subtitle') { cls = 'subtitle'; heading = true; }
  else if (/quote/.test(sname)) tag = 'blockquote';
  else if (/^(code|html preformatted|source code|preformatted text)/.test(sname)) tag = 'pre';
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  const s = e.style;
  const jc = pp.jc;
  if (jc === 'center') s.textAlign = 'center'; else if (jc === 'right' || jc === 'end') s.textAlign = 'right'; else if (jc === 'both' || jc === 'distribute') s.textAlign = 'justify';
  if (direct.line) s.lineHeight = String(Math.round(direct.line * 100) / 100);
  if (direct.before != null) s.marginTop = direct.before + 'pt';
  if (direct.after != null) s.marginBottom = direct.after + 'pt';
  const list = pp.numId && pp.numId !== '0' && ctx.numbering.num[pp.numId] ? { numId: pp.numId, ilvl: pp.ilvl || 0 } : null;
  if (!list) {
    if (direct.left > 0) s.marginLeft = Math.round(direct.left / 1440 * 25.4) + 'mm';
    if (direct.first) s.textIndent = Math.round(direct.first / 1440 * 25.4) + 'mm';
  }
  if (pp.bg) s.backgroundColor = '#' + pp.bg;
  // Runs. A heading's own look comes from Write's heading style; only direct formatting is kept on its runs.
  const base = heading ? {} : merge(style && style.r);
  ctx.fld = 0;
  const frag = document.createDocumentFragment();
  let pageBreakAfter = false;
  const st = { pending: [], key: null };
  const flush = () => {
    if (!st.pending.length) return;
    const fmt = st.fmt;
    let node = document.createDocumentFragment();
    for (const piece of st.pending) node.append(piece);
    node = wrap(node, fmt, ctx);
    (st.target || frag).append(node);
    st.pending = []; st.key = null;
  };
  const addRun = (piece, fmt, target) => {
    const key = JSON.stringify(fmt) + '|' + (target ? 'a' : '');
    if (key !== st.key || target !== st.target) { flush(); st.key = key; st.fmt = fmt; st.target = target; }
    st.pending.push(piece);
  };
  const inline = async (nodes, target, link) => {
    for (const n of nodes) {
      const nm = n.localName;
      if (nm === 'r') {
        const rPr = kid(n, 'rPr');
        const cs = ctx.styles.char[wa(kid(rPr, 'rStyle'), 'val')];
        const fmt = merge(base, cs && cs.r, rpr(rPr));
        if (fmt.hidden) continue;
        for (const c of kids(n)) {
          const cn = c.localName;
          if (cn === 'fldChar') { const t = wa(c, 'fldCharType'); ctx.fld = t === 'begin' ? 1 : t === 'separate' ? 2 : 0; continue; }
          if (ctx.fld === 1) continue; // field code, not its result
          if (cn === 't') { let t = c.textContent; if (fmt.caps) t = t.toUpperCase(); addRun(document.createTextNode(t), fmt, target); }
          else if (cn === 'tab' || cn === 'ptab') { const sp = document.createElement('span'); sp.className = 'tab'; sp.textContent = '\t'; addRun(sp, fmt, target); }
          else if (cn === 'br' || cn === 'cr') {
            if (wa(c, 'type') === 'page') { pageBreakAfter = true; continue; }
            addRun(document.createElement('br'), fmt, target);
          } else if (cn === 'noBreakHyphen') addRun(document.createTextNode('‑'), fmt, target);
          else if (cn === 'sym') { const code = parseInt(wa(c, 'char') || '0', 16); if (code) addRun(document.createTextNode(String.fromCharCode(code >= 0xf000 ? code - 0xf000 : code)), fmt, target); }
          else if (cn === 'drawing' || cn === 'pict' || cn === 'object') { const img = await picture(c, ctx); if (img) { flush(); (target || frag).append(img); } }
          else if (cn === 'footnoteReference' || cn === 'endnoteReference') ctx.notes.add('Footnotes aren\'t shown.');
          else if (cn === 'AlternateContent') { const ch = kid(c, 'Choice') || kid(c, 'Fallback'); for (const d of desc(ch, 'drawing').concat(desc(ch, 'pict'))) { const img = await picture(d, ctx); if (img) { flush(); (target || frag).append(img); } } }
        }
      } else if (nm === 'hyperlink') {
        const rel = ctx.rels[ra(n, 'id')];
        const href = rel ? rel.target : (wa(n, 'anchor') ? '#' + wa(n, 'anchor') : null);
        if (href && /^(https?:|mailto:|#)/i.test(href)) {
          flush();
          const a = document.createElement('a'); a.setAttribute('href', href);
          await inline(kids(n), a, true); flush();
          (target || frag).append(a);
        } else await inline(kids(n), target, link);
      } else if (nm === 'fldSimple' || nm === 'ins' || nm === 'smartTag' || nm === 'customXml' || nm === 'dir' || nm === 'bdo' || nm === 'moveTo') await inline(kids(n), target, link);
      else if (nm === 'sdt') await inline(kids(kid(n, 'sdtContent')), target, link);
      else if (nm === 'oMath' || nm === 'oMathPara') addRun(document.createTextNode(n.textContent), {}, target);
    }
  };
  await inline(kids(p), null, false);
  flush();
  e.append(frag);
  if (!e.textContent.length && !e.querySelector('img,br')) {
    if (pp.bottomBorder) return { el: document.createElement('hr'), pageBreakAfter };
    if (pageBreakAfter) return { el: Object.assign(document.createElement('hr'), { className: 'page-break' }) };
    e.append(document.createElement('br'));
  }
  if (tag === 'pre') { $$br(e); }
  return { el: e, list, pageBreakAfter };
}
const $$br = (e) => { e.querySelectorAll('br').forEach((b) => b.replaceWith('\n')); };

// A run's formatting as elements around its text. Defaults (the document's font and size) aren't repeated.
function wrap(node, f, ctx) {
  const st = [];
  if (f.font && f.font !== ctx.base.font) st.push(`font-family:${H.apps.write.fontCss(f.font)}`);
  if (f.sz && f.sz !== ctx.base.sz) st.push(`font-size:${f.sz}pt`);
  if (f.color && f.color !== '000000') st.push(`color:#${f.color}`);
  if (f.bg) st.push(`background-color:#${f.bg}`);
  let out = node;
  const w = (tag) => { const e = document.createElement(tag); e.append(out); out = e; };
  if (f.va === 'superscript') w('sup'); else if (f.va === 'subscript') w('sub');
  if (f.s) w('s');
  if (f.u) w('u');
  if (f.i) w('i');
  if (f.b) w('b');
  if (st.length) { const sp = document.createElement('span'); sp.setAttribute('style', st.join(';')); sp.append(out); out = sp; }
  return out;
}

async function picture(d, ctx) {
  const blip = desc(d, 'blip')[0];
  const id = blip ? ra(blip, 'embed') : (desc(d, 'imagedata')[0] ? (ra(desc(d, 'imagedata')[0], 'id')) : null);
  const rel = id && ctx.rels[id];
  if (!rel || rel.external) return null;
  const ext = (rel.target.split('.').pop() || '').toLowerCase();
  if (['emf', 'wmf'].includes(ext)) { ctx.notes.add('Some pictures (EMF/WMF drawings) can\'t be shown.'); return null; }
  let url = ctx.imgs[rel.target];
  if (!url) { const f = ctx.zip.file(rel.target); if (!f) return null; url = ctx.imgs[rel.target] = `data:${H.mimeOf(ext)};base64,${await f.async('base64')}`; }
  const img = document.createElement('img');
  img.src = url;
  const extent = desc(d, 'extent')[0];
  if (extent) { const cx = parseInt(extent.getAttribute('cx'), 10) / 9525; if (cx > 0) img.style.width = Math.round(cx) + 'px'; }
  else { const shape = desc(d, 'shape')[0]; const m = shape && /width:([\d.]+)pt/.exec(shape.getAttribute('style') || ''); if (m) img.style.width = Math.round(parseFloat(m[1]) / 0.75) + 'px'; }
  const docPr = desc(d, 'docPr')[0]; if (docPr && (docPr.getAttribute('descr') || docPr.getAttribute('title'))) img.alt = docPr.getAttribute('descr') || docPr.getAttribute('title');
  const anchor = desc(d, 'anchor')[0];
  if (anchor) {
    const align = desc(kid(anchor, 'positionH'), 'align')[0];
    const a = align ? align.textContent : '';
    if (kid(anchor, 'wrapTopAndBottom') || kid(anchor, 'wrapNone')) { img.style.display = 'block'; img.style.margin = a === 'right' ? '6px 0 6px auto' : a === 'center' ? '6px auto' : '6px 0'; }
    else if (a === 'right') { img.style.float = 'right'; img.style.margin = '0 0 6px 12px'; }
    else { img.style.float = 'left'; img.style.margin = '0 12px 6px 0'; }
  }
  return img;
}

async function table(t, ctx) {
  const tb = document.createElement('table'); tb.className = 't';
  const tPr = kid(t, 'tblPr');
  const borders = kid(tPr, 'tblBorders');
  if (borders && kids(borders).length && kids(borders).every((b) => /^(none|nil)$/.test(wa(b, 'val') || ''))) tb.classList.add('nob');
  if (!borders && !wa(kid(tPr, 'tblStyle'), 'val')) tb.classList.add('nob');
  const grid = kids(kid(t, 'tblGrid'), 'gridCol').map((g) => parseInt(wa(g, 'w') || '0', 10));
  if (grid.length && grid.every((x) => x > 0)) {
    const cg = document.createElement('colgroup');
    const total = grid.reduce((a, b) => a + b, 0);
    for (const g of grid) { const c = document.createElement('col'); c.style.width = (g / total * 100).toFixed(2) + '%'; cg.append(c); }
    tb.append(cg);
  }
  const body = document.createElement('tbody'); tb.append(body);
  const vm = {}; // column → cell that a vertical merge started in
  for (const tr of kids(t, 'tr')) {
    const row = document.createElement('tr');
    const header = !!kid(kid(tr, 'trPr'), 'tblHeader');
    let col = 0;
    for (const tc of kids(tr, 'tc').concat(kids(tr, 'sdt').flatMap((s) => kids(kid(s, 'sdtContent'), 'tc')))) {
      const tcPr = kid(tc, 'tcPr');
      const span = parseInt(wa(kid(tcPr, 'gridSpan'), 'val') || '1', 10);
      const vmerge = kid(tcPr, 'vMerge');
      if (vmerge && wa(vmerge, 'val') !== 'restart') {
        const start = vm[col]; if (start) start.rowSpan = (start.rowSpan || 1) + 1;
        col += span; continue;
      }
      const cell = document.createElement(header ? 'th' : 'td');
      if (span > 1) cell.colSpan = span;
      if (vmerge) vm[col] = cell; else delete vm[col];
      const shd = kid(tcPr, 'shd'); if (shd && wa(shd, 'fill') && !/^(auto|FFFFFF)$/i.test(wa(shd, 'fill'))) cell.style.backgroundColor = '#' + wa(shd, 'fill');
      await blocks(kids(tc), cell, ctx);
      // A cell with one paragraph: just its content.
      if (cell.children.length === 1 && cell.firstElementChild.tagName === 'P' && !cell.firstElementChild.getAttribute('style')) { const p = cell.firstElementChild; p.replaceWith(...p.childNodes); }
      if (!cell.firstChild) cell.append(document.createElement('br'));
      row.append(cell);
      col += span;
    }
    body.append(row);
  }
  return tb;
}

// ------------------------------------------------------------------ writing
async function write(root, page) {
  const D = window.docx;
  const W = H.apps.write;
  const [wmm, hmm] = W.pageMm();
  const twip = (mm) => Math.round(mm / 25.4 * 1440);
  const contentTw = twip(wmm - page.margins.l - page.margins.r);
  const ctx = { listInstance: 0, imgs: new Map() };
  // Pictures first (they need their sizes and, for some types, converting to PNG).
  for (const img of root.querySelectorAll('img')) {
    let src = img.getAttribute('src') || '';
    let d = H.dataUrlBytes(src);
    if (!d) continue;
    let type = { 'image/png': 'png', 'image/jpeg': 'jpg', 'image/gif': 'gif', 'image/bmp': 'bmp' }[d.mime];
    let nat = await H.imageSize(src);
    if (!type) { const png = await H.toPng(src); if (!png) continue; d = H.dataUrlBytes(png.url); type = 'png'; nat = { w: png.w, h: png.h }; }
    let w = parseFloat(img.style.width) || nat.w || 200;
    const maxW = contentTw / 15;
    if (w > maxW) w = maxW;
    const h = nat.w ? w * nat.h / nat.w : (parseFloat(img.style.height) || w * 0.75);
    ctx.imgs.set(img, { data: d.bytes, type, w: Math.round(w), h: Math.round(h), alt: img.alt || '' });
  }
  const children = [];
  blocksOut(root.childNodes, children, ctx, D, page, 0);
  if (!children.length) children.push(new D.Paragraph({}));
  const pn = () => new D.TextRun({ children: ['Page ', D.PageNumber.CURRENT, ' of ', D.PageNumber.TOTAL_PAGES] });
  const header = [], footer = [];
  if (page.header) header.push(new D.Paragraph({ children: [new D.TextRun(page.header)] }));
  if (page.numbers === 'header-right') header.push(new D.Paragraph({ alignment: D.AlignmentType.RIGHT, children: [pn()] }));
  if (page.footer) footer.push(new D.Paragraph({ children: [new D.TextRun(page.footer)] }));
  if (page.numbers === 'footer-center' || page.numbers === 'footer-right') footer.push(new D.Paragraph({ alignment: page.numbers === 'footer-center' ? D.AlignmentType.CENTER : D.AlignmentType.RIGHT, children: [pn()] }));
  const lvls = (fmt) => Array.from({ length: 9 }, (_, i) => ({
    level: i,
    format: fmt === 'bullet' ? D.LevelFormat.BULLET : [D.LevelFormat.DECIMAL, D.LevelFormat.LOWER_LETTER, D.LevelFormat.LOWER_ROMAN][i % 3],
    text: fmt === 'bullet' ? ['•', '◦', '▪'][i % 3] : `%${i + 1}.`,
    alignment: D.AlignmentType.LEFT,
    style: { paragraph: { indent: { left: 720 * (i + 1), hanging: 360 } } },
  }));
  const lvlsType = (f) => Array.from({ length: 9 }, (_, i) => ({ level: i, format: i === 0 ? f : [D.LevelFormat.DECIMAL, D.LevelFormat.LOWER_LETTER, D.LevelFormat.LOWER_ROMAN][i % 3], text: `%${i + 1}.`, alignment: D.AlignmentType.LEFT, style: { paragraph: { indent: { left: 720 * (i + 1), hanging: 360 } } } }));
  const hd = (size, extra = {}) => ({ run: { size: size * 2, bold: true, color: '2F5496', font: page.font, ...extra }, paragraph: { spacing: { before: 240, after: 80 }, keepNext: true } });
  const doc = new D.Document({
    creator: 'Hearth Write', title: H.stem(H.doc.name || ''),
    styles: {
      default: {
        document: { run: { font: page.font, size: Math.round(page.fontSize * 2) }, paragraph: { spacing: { after: Math.round((page.spaceAfter ?? 8) * 20), line: Math.round(page.lineHeight * 240) } } },
        title: { run: { size: 52, font: page.font, color: '1F1F1F' }, paragraph: { spacing: { after: 120 } } },
        heading1: hd(18), heading2: hd(15), heading3: hd(13), heading4: hd(12, { italics: true }), heading5: hd(11), heading6: hd(11, { italics: true }),
        hyperlink: { run: { color: '0563C1', underline: { type: D.UnderlineType.SINGLE } } },
      },
      paragraphStyles: [
        { id: 'Subtitle', name: 'Subtitle', basedOn: 'Normal', next: 'Normal', quickFormat: true, run: { size: 30, color: '595959' }, paragraph: { spacing: { after: 160 } } },
        { id: 'Quote', name: 'Quote', basedOn: 'Normal', next: 'Normal', quickFormat: true, run: { italics: true, color: '404040' }, paragraph: { indent: { left: 720, right: 720 } } },
        { id: 'Code', name: 'Code', basedOn: 'Normal', quickFormat: true, run: { font: 'Courier New', size: 20 }, paragraph: { spacing: { after: 0, line: 240 } } },
      ],
    },
    numbering: { config: [
      { reference: 'hw-bullets', levels: lvls('bullet') }, { reference: 'hw-numbers', levels: lvls('number') },
      { reference: 'hw-lower-alpha', levels: lvlsType(D.LevelFormat.LOWER_LETTER) }, { reference: 'hw-upper-alpha', levels: lvlsType(D.LevelFormat.UPPER_LETTER) },
      { reference: 'hw-lower-roman', levels: lvlsType(D.LevelFormat.LOWER_ROMAN) }, { reference: 'hw-upper-roman', levels: lvlsType(D.LevelFormat.UPPER_ROMAN) },
    ] },
    sections: [{
      properties: { page: {
        size: { width: twip(page.orient === 'landscape' ? hmm : wmm), height: twip(page.orient === 'landscape' ? wmm : hmm), orientation: page.orient === 'landscape' ? D.PageOrientation.LANDSCAPE : D.PageOrientation.PORTRAIT },
        margin: { top: twip(page.margins.t), right: twip(page.margins.r), bottom: twip(page.margins.b), left: twip(page.margins.l), header: twip(Math.min(12.7, page.margins.t / 2)), footer: twip(Math.min(12.7, page.margins.b / 2)) },
      } },
      headers: header.length ? { default: new D.Header({ children: header }) } : undefined,
      footers: footer.length ? { default: new D.Footer({ children: footer }) } : undefined,
      children,
    }],
  });
  const blob = await D.Packer.toBlob(doc);
  return new Uint8Array(await blob.arrayBuffer());
}

const pt = (v) => { if (!v) return null; const n = parseFloat(v); if (isNaN(n)) return null; if (/pt$/.test(v)) return n; if (/px$/.test(v)) return n * 0.75; if (/mm$/.test(v)) return n / 25.4 * 72; if (/cm$/.test(v)) return n / 2.54 * 72; if (/in$/.test(v)) return n * 72; if (/em$/.test(v)) return n * 12; return n; };
const KW_SIZE = { 'xx-small': 7, 'x-small': 7.5, small: 10, medium: 12, large: 13.5, 'x-large': 18, 'xx-large': 24, 'xxx-large': 36, '-webkit-xxx-large': 36 };

// Inline formatting from the elements around a piece of text.
function fmtOf(e, base) {
  const f = { ...base };
  const t = e.tagName;
  if (t === 'B' || t === 'STRONG') f.bold = true;
  if (t === 'I' || t === 'EM') f.italics = true;
  if (t === 'U' || t === 'INS') f.underline = true;
  if (t === 'S' || t === 'STRIKE' || t === 'DEL') f.strike = true;
  if (t === 'SUP') f.superScript = true;
  if (t === 'SUB') f.subScript = true;
  if (t === 'CODE') f.font = 'Courier New';
  if (t === 'MARK') f.shade = 'FFFF00';
  if (t === 'A') f.link = true;
  const s = e.style;
  if (s) {
    if (s.fontWeight && (s.fontWeight === 'bold' || parseInt(s.fontWeight, 10) >= 600)) f.bold = true;
    if (s.fontWeight && (s.fontWeight === 'normal' || parseInt(s.fontWeight, 10) < 600 && /^\d/.test(s.fontWeight))) f.bold = false;
    if (s.fontStyle === 'italic') f.italics = true;
    const td = s.textDecorationLine || s.textDecoration || '';
    if (/underline/.test(td)) f.underline = true;
    if (/line-through/.test(td)) f.strike = true;
    if (s.color) { const h = H.hex(s.color); if (h) f.color = h; }
    if (s.backgroundColor) { const h = H.hex(s.backgroundColor); if (h && h !== 'FFFFFF') f.shade = h; }
    if (s.fontSize) { const p = KW_SIZE[s.fontSize] || pt(s.fontSize); if (p) f.size = Math.round(p * 2); }
    if (s.fontFamily) f.font = H.apps.write.firstFont(s.fontFamily);
    if (s.verticalAlign === 'super') f.superScript = true;
    if (s.verticalAlign === 'sub') f.subScript = true;
  }
  if (t === 'FONT') { if (e.getAttribute('color')) f.color = H.hex(e.getAttribute('color')); if (e.getAttribute('face')) f.font = e.getAttribute('face'); }
  return f;
}
const runOpts = (f, extra = {}) => {
  const o = { ...extra };
  if (f.bold) o.bold = true; if (f.italics) o.italics = true; if (f.underline) o.underline = {}; if (f.strike) o.strike = true;
  if (f.superScript) o.superScript = true; if (f.subScript) o.subScript = true;
  if (f.color) o.color = f.color; if (f.size) o.size = f.size; if (f.font) o.font = f.font;
  if (f.shade) o.shading = { type: window.docx.ShadingType.CLEAR, fill: f.shade, color: 'auto' };
  if (f.link) o.style = 'Hyperlink';
  return o;
};
// The runs (text, breaks, tabs, pictures, links) inside an element.
function runsOut(nodes, f, ctx, D, out) {
  for (const n of nodes) {
    if (n.nodeType === 3) {
      const t = n.data.replace(/​/g, '').replace(/[\n\r]+/g, ' ');
      if (t) out.push(new D.TextRun(runOpts(f, { text: t })));
      continue;
    }
    if (n.nodeType !== 1) continue;
    const t = n.tagName;
    if (t === 'BR') { out.push(new D.TextRun(runOpts(f, { break: 1 }))); continue; }
    if (t === 'IMG') { const im = ctx.imgs.get(n); if (im) out.push(new D.ImageRun({ type: im.type, data: im.data, transformation: { width: im.w, height: im.h }, altText: im.alt ? { title: im.alt, description: im.alt, name: im.alt } : undefined })); continue; }
    if (t === 'SPAN' && n.classList.contains('tab')) { out.push(new D.TextRun(runOpts(f, { children: [new D.Tab()] }))); continue; }
    if (t === 'A' && n.getAttribute('href') && /^(https?:|mailto:)/i.test(n.getAttribute('href'))) {
      const inner = []; runsOut(n.childNodes, fmtOf(n, f), ctx, D, inner);
      const textRuns = inner.filter((r) => r instanceof D.TextRun);
      if (textRuns.length) out.push(new D.ExternalHyperlink({ children: textRuns, link: n.getAttribute('href') }));
      continue;
    }
    if (/^(UL|OL|TABLE|P|DIV|H[1-6]|BLOCKQUOTE|PRE)$/.test(t)) continue; // blocks inside inline content are handled by the caller
    runsOut(n.childNodes, fmtOf(n, f), ctx, D, out);
  }
}
function paraOpts(e, D, extra = {}) {
  const o = { ...extra };
  const s = e.style || {};
  const a = s.textAlign;
  if (a === 'center') o.alignment = D.AlignmentType.CENTER; else if (a === 'right') o.alignment = D.AlignmentType.RIGHT; else if (a === 'justify') o.alignment = D.AlignmentType.JUSTIFIED;
  const sp = {};
  if (s.lineHeight && !isNaN(parseFloat(s.lineHeight)) && !/px|pt|%/.test(s.lineHeight)) sp.line = Math.round(parseFloat(s.lineHeight) * 240);
  if (s.marginTop) { const p = pt(s.marginTop); if (p != null) sp.before = Math.round(p * 20); }
  if (s.marginBottom) { const p = pt(s.marginBottom); if (p != null) sp.after = Math.round(p * 20); }
  if (Object.keys(sp).length) o.spacing = sp;
  const ind = {};
  if (s.marginLeft) { const p = pt(s.marginLeft); if (p) ind.left = Math.round(p * 20); }
  if (s.textIndent) { const p = pt(s.textIndent); if (p > 0) ind.firstLine = Math.round(p * 20); else if (p < 0) ind.hanging = Math.round(-p * 20); }
  if (Object.keys(ind).length) o.indent = ind;
  if (s.backgroundColor) { const h = H.hex(s.backgroundColor); if (h) o.shading = { type: D.ShadingType.CLEAR, fill: h, color: 'auto' }; }
  return o;
}
function blocksOut(nodes, out, ctx, D, page, depth) {
  let loose = [];
  const flushLoose = () => { if (loose.length) { const runs = []; runsOut(loose, {}, ctx, D, runs); if (runs.length) out.push(new D.Paragraph({ children: runs })); loose = []; } };
  for (const n of nodes) {
    if (n.nodeType === 3) { if (n.data.trim()) loose.push(n); continue; }
    if (n.nodeType !== 1) continue;
    const t = n.tagName;
    if (!/^(P|DIV|H[1-6]|UL|OL|TABLE|BLOCKQUOTE|PRE|HR|FIGURE|SECTION|ARTICLE)$/.test(t)) { loose.push(n); continue; }
    flushLoose();
    if (/^H[1-6]$/.test(t)) { const runs = []; runsOut(n.childNodes, {}, ctx, D, runs); out.push(new D.Paragraph(paraOpts(n, D, { heading: D.HeadingLevel['HEADING_' + t[1]], children: runs }))); }
    else if (t === 'P' || t === 'DIV' || t === 'FIGURE' || t === 'SECTION' || t === 'ARTICLE') {
      if (n.querySelector(':scope > p, :scope > div, :scope > table, :scope > ul, :scope > ol')) { blocksOut(n.childNodes, out, ctx, D, page, depth); continue; }
      const runs = []; runsOut(n.childNodes, {}, ctx, D, runs);
      const extra = { children: runs };
      if (n.classList.contains('title')) extra.heading = D.HeadingLevel.TITLE;
      else if (n.classList.contains('subtitle')) extra.style = 'Subtitle';
      out.push(new D.Paragraph(paraOpts(n, D, extra)));
    } else if (t === 'BLOCKQUOTE') {
      if (n.querySelector(':scope > p')) { for (const c of n.children) { const runs = []; runsOut(c.childNodes, {}, ctx, D, runs); out.push(new D.Paragraph(paraOpts(c, D, { style: 'Quote', children: runs }))); } }
      else { const runs = []; runsOut(n.childNodes, {}, ctx, D, runs); out.push(new D.Paragraph(paraOpts(n, D, { style: 'Quote', children: runs }))); }
    } else if (t === 'PRE') {
      const lines = n.textContent.replace(/\r/g, '').split('\n');
      out.push(new D.Paragraph({ style: 'Code', children: lines.map((l, i) => new D.TextRun({ text: l, break: i ? 1 : 0, font: 'Courier New' })) }));
    } else if (t === 'HR') {
      if (n.classList.contains('page-break')) out.push(new D.Paragraph({ children: [new D.PageBreak()] }));
      else out.push(new D.Paragraph({ border: { bottom: { color: 'auto', space: 1, style: D.BorderStyle.SINGLE, size: 6 } } }));
    } else if (t === 'UL' || t === 'OL') listOut(n, out, ctx, D, 0, ++ctx.listInstance);
    else if (t === 'TABLE') out.push(tableOut(n, ctx, D, page));
  }
  flushLoose();
}
function listOut(list, out, ctx, D, level, instance) {
  const type = (list.getAttribute('type') || '').trim();
  const ref = list.tagName === 'UL' ? 'hw-bullets' : ({ a: 'hw-lower-alpha', A: 'hw-upper-alpha', i: 'hw-lower-roman', I: 'hw-upper-roman' }[type] || 'hw-numbers');
  for (const li of list.children) {
    if (li.tagName === 'UL' || li.tagName === 'OL') { listOut(li, out, ctx, D, Math.min(8, level + 1), instance); continue; }
    if (li.tagName !== 'LI') continue;
    const inline = [], nested = [];
    let firstDone = false;
    const flush = () => {
      if (!inline.length && firstDone) return;
      const runs = []; runsOut(inline, {}, ctx, D, runs);
      out.push(new D.Paragraph(paraOpts(li, D, { children: runs, numbering: { reference: ref, level, instance } })));
      inline.length = 0; firstDone = true;
    };
    for (const c of li.childNodes) {
      if (c.nodeType === 1 && (c.tagName === 'UL' || c.tagName === 'OL')) { flush(); listOut(c, out, ctx, D, Math.min(8, level + 1), instance); }
      else if (c.nodeType === 1 && c.tagName === 'P') { if (inline.length || !firstDone) { inline.push(...c.childNodes); flush(); } else { const runs = []; runsOut(c.childNodes, {}, ctx, D, runs); out.push(new D.Paragraph({ children: runs, indent: { left: 720 * (level + 1) } })); } }
      else inline.push(c);
    }
    if (inline.length || !firstDone) flush();
    void nested;
  }
}
function tableOut(t, ctx, D, page) {
  const W = H.apps.write;
  const [wmm] = W.pageMm();
  const contentTw = Math.round((wmm - page.margins.l - page.margins.r) / 25.4 * 1440);
  const rows = Array.from(t.querySelectorAll(':scope > tbody > tr, :scope > thead > tr, :scope > tr, :scope > tfoot > tr'));
  let ncols = 0;
  for (const r of rows) { let c = 0; for (const cell of r.children) c += cell.colSpan || 1; ncols = Math.max(ncols, c); }
  ncols = Math.max(1, ncols);
  const cols = Array.from(t.querySelectorAll(':scope > colgroup > col')).map((c) => parseFloat(c.style.width));
  const widths = cols.length === ncols && cols.every((x) => x > 0) ? cols.map((p) => Math.round(contentTw * p / cols.reduce((a, b) => a + b, 0))) : Array(ncols).fill(Math.floor(contentTw / ncols));
  const none = { style: D.BorderStyle.NONE, size: 0, color: 'FFFFFF' };
  const line = { style: D.BorderStyle.SINGLE, size: 4, color: '999999' };
  const b = t.classList.contains('nob') ? none : line;
  const trs = rows.map((r) => {
    let ci = 0;
    const header = !!r.querySelector('th') && r === rows[0];
    const cells = Array.from(r.children).filter((c) => /^(TD|TH)$/.test(c.tagName)).map((c) => {
      const kids = [];
      if (c.querySelector(':scope > p, :scope > ul, :scope > ol, :scope > table, :scope > h1, :scope > h2, :scope > h3, :scope > div')) blocksOut(c.childNodes, kids, ctx, D, page, 1);
      else { const runs = []; runsOut(c.childNodes, c.tagName === 'TH' ? { bold: true } : {}, ctx, D, runs); kids.push(new D.Paragraph(paraOpts(c, D, { children: runs, spacing: { after: 0 } }))); }
      if (!kids.length) kids.push(new D.Paragraph({}));
      const span = c.colSpan || 1;
      const w = widths.slice(ci, ci + span).reduce((a, x) => a + x, 0) || widths[0];
      ci += span;
      const o = { children: kids, width: { size: w, type: D.WidthType.DXA } };
      if (span > 1) o.columnSpan = span;
      if (c.rowSpan > 1) o.rowSpan = c.rowSpan;
      const bg = H.hex(c.style.backgroundColor); if (bg) o.shading = { type: D.ShadingType.CLEAR, fill: bg, color: 'auto' };
      return new D.TableCell(o);
    });
    return new D.TableRow({ children: cells.length ? cells : [new D.TableCell({ children: [new D.Paragraph({})] })], tableHeader: header || undefined });
  });
  return new D.Table({ rows: trs, width: { size: contentTw, type: D.WidthType.DXA }, columnWidths: widths, borders: { top: b, bottom: b, left: b, right: b, insideHorizontal: b, insideVertical: b } });
}

WriteFormats.docx = { read, write };
})();
