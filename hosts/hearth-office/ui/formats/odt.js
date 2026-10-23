// Hearth Write ↔ .odt (OpenDocument text, LibreOffice's format), with JSZip and the XML inside.
// Kept both ways: paragraphs and headings, title/subtitle/quote/code styles, fonts, sizes, colours, highlight,
// bold/italic/underline/strikethrough, super/subscript, alignment, line and paragraph spacing, indents, lists,
// tables (merged cells, cell colours), pictures, links, page size and margins, header, footer, page numbers.
// Not kept: frames and text boxes, sections, footnotes, tracked changes, comments, fields other than page numbers.
'use strict';
window.WriteFormats = window.WriteFormats || {};
(() => {
const NS = {
  office: 'urn:oasis:names:tc:opendocument:xmlns:office:1.0', style: 'urn:oasis:names:tc:opendocument:xmlns:style:1.0',
  text: 'urn:oasis:names:tc:opendocument:xmlns:text:1.0', table: 'urn:oasis:names:tc:opendocument:xmlns:table:1.0',
  draw: 'urn:oasis:names:tc:opendocument:xmlns:drawing:1.0', fo: 'urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0',
  xlink: 'http://www.w3.org/1999/xlink', svg: 'urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0',
  meta: 'urn:oasis:names:tc:opendocument:xmlns:meta:1.0', dc: 'http://purl.org/dc/elements/1.1/', loext: 'urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0',
};
const at = (e, ns, name) => (e ? e.getAttributeNS(NS[ns], name) : null);
const kids = (e) => (e ? Array.from(e.children) : []);
const first = (e, name) => kids(e).find((c) => c.localName === name) || null;
const all = (d, ns, name) => Array.from(d.getElementsByTagNameNS(NS[ns], name));
const len = (v) => { // a length to mm
  if (!v) return null; const n = parseFloat(v);
  if (/mm$/.test(v)) return n; if (/cm$/.test(v)) return n * 10; if (/in$/.test(v)) return n * 25.4; if (/pt$/.test(v)) return n / 72 * 25.4; if (/px$/.test(v)) return n / 96 * 25.4; if (/pc$/.test(v)) return n / 6 * 25.4;
  return n;
};
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

// ------------------------------------------------------------------ reading
async function read(bytes) {
  const zip = await JSZip.loadAsync(bytes);
  const s = async (p) => { const f = zip.file(p); return f ? f.async('string') : null; };
  const content = await s('content.xml');
  if (!content) throw new Error('This isn\'t an OpenDocument text file (no content.xml inside).');
  const cx = new DOMParser().parseFromString(content, 'application/xml');
  const sx = new DOMParser().parseFromString((await s('styles.xml')) || '<x/>', 'application/xml');
  const fonts = {};
  for (const d of [cx, sx]) for (const f of all(d, 'style', 'font-face')) fonts[at(f, 'style', 'name')] = (at(f, 'svg', 'font-family') || at(f, 'style', 'name')).replace(/^'|'$/g, '');
  const styles = {}, lists = {};
  const addStyles = (d) => {
    for (const st of all(d, 'style', 'style')) {
      const name = at(st, 'style', 'name');
      const tp = first(st, 'text-properties'), pp = first(st, 'paragraph-properties'), cp = first(st, 'table-cell-properties');
      const o = { family: at(st, 'style', 'family'), parent: at(st, 'style', 'parent-style-name'), display: at(st, 'style', 'display-name') || name, t: {}, p: {} };
      if (tp) {
        if (at(tp, 'fo', 'font-weight') === 'bold' || parseInt(at(tp, 'fo', 'font-weight'), 10) >= 600) o.t.b = true;
        if (at(tp, 'fo', 'font-weight') === 'normal') o.t.b = false;
        if (at(tp, 'fo', 'font-style') === 'italic') o.t.i = true;
        if (at(tp, 'fo', 'font-style') === 'normal') o.t.i = false;
        const u = at(tp, 'style', 'text-underline-style'); if (u) o.t.u = u !== 'none';
        const l = at(tp, 'style', 'text-line-through-style'); if (l) o.t.s = l !== 'none';
        const c = at(tp, 'fo', 'color'); if (c) o.t.color = c;
        const sz = at(tp, 'fo', 'font-size'); if (sz && /pt$/.test(sz)) o.t.sz = parseFloat(sz);
        const fn = at(tp, 'style', 'font-name') || at(tp, 'fo', 'font-family'); if (fn) o.t.font = fonts[fn] || fn.replace(/^'|'$/g, '');
        const bg = at(tp, 'fo', 'background-color'); if (bg && bg !== 'transparent') o.t.bg = bg;
        const pos = at(tp, 'style', 'text-position'); if (pos) o.t.va = /^super|^\d/.test(pos) && !/^-/.test(pos) ? 'sup' : /^sub|^-/.test(pos) ? 'sub' : null;
      }
      if (pp) {
        const a = at(pp, 'fo', 'text-align'); if (a) o.p.align = a === 'center' ? 'center' : (a === 'end' || a === 'right') ? 'right' : a === 'justify' ? 'justify' : 'left';
        const lh = at(pp, 'fo', 'line-height'); if (lh && /%$/.test(lh)) o.p.line = parseFloat(lh) / 100;
        const mt = at(pp, 'fo', 'margin-top'); if (mt) o.p.before = len(mt);
        const mb = at(pp, 'fo', 'margin-bottom'); if (mb) o.p.after = len(mb);
        const ml = at(pp, 'fo', 'margin-left'); if (ml) o.p.left = len(ml);
        const ti = at(pp, 'fo', 'text-indent'); if (ti) o.p.indent = len(ti);
        const bg = at(pp, 'fo', 'background-color'); if (bg && bg !== 'transparent') o.p.bg = bg;
        if (at(pp, 'fo', 'break-before') === 'page') o.p.breakBefore = true;
      }
      if (cp) { const bg = at(cp, 'fo', 'background-color'); if (bg && bg !== 'transparent') o.cellBg = bg; const b = at(cp, 'fo', 'border'); if (b) o.border = b; }
      styles[name] = o;
    }
    for (const ls of all(d, 'text', 'list-style')) {
      const lv = {};
      for (const l of kids(ls)) {
        const level = parseInt(at(l, 'text', 'level') || '1', 10);
        lv[level] = l.localName === 'list-level-style-number' ? { ordered: at(l, 'style', 'num-format') !== '' , type: at(l, 'style', 'num-format') || '1', start: parseInt(at(l, 'text', 'start-value') || '1', 10) } : { ordered: false };
      }
      lists[at(ls, 'style', 'name')] = lv;
    }
  };
  addStyles(sx); addStyles(cx);
  const resolve = (name, key) => { const out = {}; const chain = []; let n = name; let guard = 0; while (n && styles[n] && guard++ < 20) { chain.unshift(styles[n]); n = styles[n].parent; } for (const c of chain) Object.assign(out, c[key]); return out; };
  const styleNameOf = (name) => { let n = name, guard = 0; while (n && styles[n] && guard++ < 20) { const d = (styles[n].display || '').toLowerCase().replace(/_20_/g, ' '); if (/^(heading( \d)?|title|subtitle|quotations|quote|preformatted text|source text)$/.test(d)) return d; n = styles[n].parent; } return ''; };
  const defT = resolve('Standard', 't');
  const ctx = { zip, styles, lists, resolve, styleNameOf, imgs: {}, defT };
  const body = all(cx, 'office', 'text')[0];
  const root = document.createElement('div');
  await blocks(kids(body), root, ctx, null, 0);
  // Page setup, header and footer.
  const page = {};
  const dft = all(sx, 'style', 'default-style').find((d) => at(d, 'style', 'family') === 'paragraph');
  const dtp = dft && first(dft, 'text-properties');
  const stdT = resolve('Standard', 't');
  if (stdT.font) page.font = stdT.font; else if (dtp && at(dtp, 'style', 'font-name')) page.font = fonts[at(dtp, 'style', 'font-name')] || at(dtp, 'style', 'font-name');
  if (stdT.sz) page.fontSize = stdT.sz; else if (dtp && at(dtp, 'fo', 'font-size')) page.fontSize = parseFloat(at(dtp, 'fo', 'font-size'));
  const mp = all(sx, 'style', 'master-page')[0];
  if (mp) {
    const pl = all(sx, 'style', 'page-layout').find((x) => at(x, 'style', 'name') === at(mp, 'style', 'page-layout-name'));
    const plp = pl && first(pl, 'page-layout-properties');
    if (plp) {
      const w = len(at(plp, 'fo', 'page-width')), h = len(at(plp, 'fo', 'page-height'));
      if (w && h) {
        page.orient = w > h ? 'landscape' : 'portrait';
        const pw = Math.min(w, h), ph = Math.max(w, h);
        const sizes = H.apps.write.SIZES;
        const name = Object.entries(sizes).find(([, d]) => Math.abs(d[0] - pw) < 2 && Math.abs(d[1] - ph) < 2);
        if (name) page.size = name[0]; else { page.size = 'Custom'; page.w = pw; page.h = ph; }
      }
      page.margins = { t: len(at(plp, 'fo', 'margin-top')) ?? 20, r: len(at(plp, 'fo', 'margin-right')) ?? 20, b: len(at(plp, 'fo', 'margin-bottom')) ?? 20, l: len(at(plp, 'fo', 'margin-left')) ?? 20 };
      // The header and footer sit inside the page margins in OpenDocument; Write measures margins to the text.
      if (first(mp, 'header')) page.margins.t += 10;
      if (first(mp, 'footer')) page.margins.b += 10;
    }
    for (const kind of ['header', 'footer']) {
      const hf = first(mp, kind); if (!hf) continue;
      const texts = [];
      for (const p of kids(hf)) {
        if (p.getElementsByTagNameNS(NS.text, 'page-number').length) {
          const al = resolve(at(p, 'text', 'style-name'), 'p').align;
          page.numbers = kind === 'header' ? 'header-right' : al === 'right' ? 'footer-right' : 'footer-center';
          const c = p.cloneNode(true);
          for (const x of Array.from(c.getElementsByTagNameNS(NS.text, 'page-number')).concat(Array.from(c.getElementsByTagNameNS(NS.text, 'page-count')))) x.remove();
          const t = c.textContent.replace(/\bPage\b|\bof\b/gi, '').replace(/\s+/g, ' ').trim();
          if (t) texts.push(t);
        } else if (p.textContent.trim()) texts.push(p.textContent.trim());
      }
      page[kind] = texts.join('  ');
    }
  }
  return { html: root.innerHTML, page };
}

async function blocks(nodes, out, ctx) {
  for (const n of nodes) {
    const ln = n.localName;
    if (ln === 'p' || ln === 'h') out.append(await para(n, ctx, ln === 'h' ? parseInt(at(n, 'text', 'outline-level') || '1', 10) : 0));
    else if (ln === 'list') out.append(await list(n, ctx, 1, at(n, 'text', 'style-name')));
    else if (ln === 'table') out.append(await table(n, ctx));
    else if (ln === 'section' || ln === 'index-body' || ln === 'table-of-content' || ln === 'alphabetical-index' || ln === 'illustration-index') await blocks(kids(n), out, ctx);
    else if (ln === 'soft-page-break' || ln === 'sequence-decls' || ln === 'variable-decls' || ln === 'tracked-changes' || ln === 'forms' || ln === 'user-field-decls' || ln === 'index-title-template' || ln === 'table-of-content-source') continue;
  }
}
async function para(n, ctx, level) {
  const sn = at(n, 'text', 'style-name');
  const kind = ctx.styleNameOf(sn);
  const pp = ctx.resolve(sn, 'p');
  let tag = 'p', cls = '';
  if (level || /^heading/.test(kind)) { const m = /(\d)/.exec(kind); tag = 'h' + Math.min(6, level || (m ? +m[1] : 1)); }
  else if (kind === 'title') cls = 'title';
  else if (kind === 'subtitle') cls = 'subtitle';
  else if (kind === 'quotations' || kind === 'quote') tag = 'blockquote';
  else if (kind === 'preformatted text' || kind === 'source text') tag = 'pre';
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  const own = (ctx.styles[sn] && ctx.styles[sn].p) || {};
  if (pp.align && pp.align !== 'left') e.style.textAlign = pp.align;
  if (own.line) e.style.lineHeight = String(own.line);
  if (own.before != null) e.style.marginTop = Math.round(own.before / 25.4 * 72 * 10) / 10 + 'pt';
  if (own.after != null) e.style.marginBottom = Math.round(own.after / 25.4 * 72 * 10) / 10 + 'pt';
  if (own.left > 0.5) e.style.marginLeft = Math.round(own.left) + 'mm';
  if (own.indent) e.style.textIndent = Math.round(own.indent) + 'mm';
  if (own.bg) e.style.backgroundColor = own.bg;
  const base = (tag[0] === 'h' && tag !== 'hr') || cls ? {} : ctx.resolve(sn, 't');
  await inline(kids(n).length || n.childNodes.length ? Array.from(n.childNodes) : [], e, ctx, base, sn);
  if (!e.textContent.length && !e.querySelector('img')) e.append(document.createElement('br'));
  return e;
}
async function inline(nodes, out, ctx, base, pstyle) {
  for (const c of nodes) {
    if (c.nodeType === 3) { out.append(wrap(document.createTextNode(c.data), base, ctx)); continue; }
    if (c.nodeType !== 1) continue;
    const ln = c.localName;
    if (ln === 'span') {
      const sn = at(c, 'text', 'style-name');
      const f = Object.assign({}, base, ctx.resolve(sn, 't'));
      await inline(Array.from(c.childNodes), out, ctx, f, pstyle);
    } else if (ln === 's') out.append(wrap(document.createTextNode(' '.repeat(Math.max(1, parseInt(at(c, 'text', 'c') || '1', 10)) - 1) + ' '), base, ctx));
    else if (ln === 'tab') { const s = document.createElement('span'); s.className = 'tab'; s.textContent = '\t'; out.append(s); }
    else if (ln === 'line-break') out.append(document.createElement('br'));
    else if (ln === 'a') {
      const href = at(c, 'xlink', 'href') || '';
      if (/^(https?:|mailto:|#)/i.test(href)) { const a = document.createElement('a'); a.setAttribute('href', href); await inline(Array.from(c.childNodes), a, ctx, base, pstyle); out.append(a); }
      else await inline(Array.from(c.childNodes), out, ctx, base, pstyle);
    } else if (ln === 'frame') {
      const img = first(c, 'image');
      if (img) {
        const href = at(img, 'xlink', 'href');
        const f = href && ctx.zip.file(href.replace(/^\.\//, ''));
        if (f) {
          const ext = (href.split('.').pop() || '').toLowerCase();
          if (!['emf', 'wmf', 'svm'].includes(ext)) {
            const el = document.createElement('img');
            el.src = ctx.imgs[href] || (ctx.imgs[href] = `data:${H.mimeOf(ext)};base64,${await f.async('base64')}`);
            const w = len(at(c, 'svg', 'width')); if (w) el.style.width = Math.round(w / 25.4 * 96) + 'px';
            const title = first(c, 'title') || first(c, 'desc'); if (title) el.alt = title.textContent;
            const anchor = at(c, 'text', 'anchor-type');
            if (anchor === 'paragraph' || anchor === 'char') { el.style.display = 'block'; el.style.margin = '6px 0'; }
            out.append(el);
          }
        }
      } else { const tb = first(c, 'text-box'); if (tb) await blocks(kids(tb), out, ctx); }
    } else if (ln === 'note' || ln === 'annotation' || ln === 'bookmark' || ln === 'bookmark-start' || ln === 'bookmark-end' || ln === 'soft-page-break' || ln === 'change-start' || ln === 'change-end' || ln === 'change') continue;
    else if (ln === 'page-number') out.append(document.createTextNode(c.textContent || '1'));
    else await inline(Array.from(c.childNodes), out, ctx, base, pstyle); // fields and others: their shown text
  }
}
function wrap(node, f, ctx) {
  if (!f || !Object.keys(f).length) return node;
  let out = node;
  const w = (tag) => { const e = document.createElement(tag); e.append(out); out = e; };
  if (f.va === 'sup') w('sup'); else if (f.va === 'sub') w('sub');
  if (f.s) w('s'); if (f.u) w('u'); if (f.i) w('i'); if (f.b) w('b');
  const st = [];
  if (f.font && f.font !== ctx.defT.font) st.push('font-family:' + H.apps.write.fontCss(f.font));
  if (f.sz && f.sz !== ctx.defT.sz) st.push(`font-size:${f.sz}pt`);
  if (f.color && !/^#0{6}$/.test(f.color)) st.push('color:' + f.color);
  if (f.bg) st.push('background-color:' + f.bg);
  if (st.length) { const s = document.createElement('span'); s.setAttribute('style', st.join(';')); s.append(out); out = s; }
  return out;
}
async function list(n, ctx, level, styleName) {
  const ls = (ctx.lists[styleName] || {})[level] || { ordered: false };
  const l = document.createElement(ls.ordered ? 'ol' : 'ul');
  if (ls.ordered && ['a', 'A', 'i', 'I'].includes(ls.type)) l.setAttribute('type', ls.type);
  if (ls.ordered && ls.start > 1) l.setAttribute('start', ls.start);
  for (const item of kids(n)) {
    if (item.localName !== 'list-item' && item.localName !== 'list-header') continue;
    const li = document.createElement('li');
    for (const c of kids(item)) {
      if (c.localName === 'p' || c.localName === 'h') {
        const p = await para(c, ctx, 0);
        if (li.childNodes.length) li.append(document.createElement('br'));
        if (p.style.textAlign) li.style.textAlign = p.style.textAlign;
        while (p.firstChild) li.append(p.firstChild);
        if (li.lastChild && li.lastChild.nodeName === 'BR' && li.childNodes.length > 1) li.lastChild.remove();
      } else if (c.localName === 'list') li.append(await list(c, ctx, level + 1, at(c, 'text', 'style-name') || styleName));
    }
    if (!li.firstChild) li.append(document.createElement('br'));
    l.append(li);
  }
  return l;
}
async function table(n, ctx) {
  const t = document.createElement('table'); t.className = 't';
  const body = document.createElement('tbody'); t.append(body);
  const rows = [];
  const walk = (e, header) => { for (const c of kids(e)) { if (c.localName === 'table-row') rows.push([c, header]); else if (c.localName === 'table-header-rows') walk(c, true); else if (c.localName === 'table-rows' || c.localName === 'table-row-group') walk(c, header); } };
  walk(n, false);
  let anyBorder = false;
  for (const [r, header] of rows) {
    const tr = document.createElement('tr');
    for (const c of kids(r)) {
      if (c.localName === 'covered-table-cell') continue;
      if (c.localName !== 'table-cell') continue;
      const rep = Math.min(50, parseInt(at(c, 'table', 'number-columns-repeated') || '1', 10));
      for (let i = 0; i < rep; i++) {
        const td = document.createElement(header ? 'th' : 'td');
        const cs = parseInt(at(c, 'table', 'number-columns-spanned') || '1', 10), rs = parseInt(at(c, 'table', 'number-rows-spanned') || '1', 10);
        if (cs > 1) td.colSpan = cs; if (rs > 1) td.rowSpan = rs;
        const st = ctx.styles[at(c, 'table', 'style-name')];
        if (st && st.cellBg) td.style.backgroundColor = st.cellBg;
        if (st && st.border && st.border !== 'none') anyBorder = true;
        if (!st || !st.border) { const anyB = st && Object.keys(st).some((k) => /border/.test(k)); if (!anyB) anyBorder = anyBorder || false; }
        await blocks(kids(c), td, ctx);
        if (td.children.length === 1 && td.firstElementChild.tagName === 'P' && !td.firstElementChild.getAttribute('style')) { const p = td.firstElementChild; p.replaceWith(...p.childNodes); }
        if (!td.firstChild) td.append(document.createElement('br'));
        tr.append(td);
      }
    }
    body.append(tr);
  }
  void anyBorder;
  return t;
}

// ------------------------------------------------------------------ writing
async function write(root, page) {
  const W = H.apps.write;
  const [wmm, hmm] = W.pageMm();
  const T = new Map(), P = new Map(), C = new Map(), pics = [];
  const fontsUsed = new Set([page.font]);
  const tKey = (f) => JSON.stringify(f);
  const tStyle = (f) => {
    if (!Object.keys(f).length) return null;
    const k = tKey(f); if (!T.has(k)) T.set(k, { name: 'T' + (T.size + 1), f }); if (f.font) fontsUsed.add(f.font);
    return T.get(k).name;
  };
  const pStyle = (p) => { const k = JSON.stringify(p); if (!P.has(k)) P.set(k, { name: 'P' + (P.size + 1), p }); return P.get(k).name; };
  const cStyle = (c) => { const k = JSON.stringify(c); if (!C.has(k)) C.set(k, { name: 'Ce' + (C.size + 1), c }); return C.get(k).name; };
  const pt = (v) => { if (!v) return null; const n = parseFloat(v); if (/pt$/.test(v)) return n; if (/px$/.test(v)) return n * 0.75; if (/mm$/.test(v)) return n / 25.4 * 72; return n; };
  const fmtOf = (e, f) => {
    f = { ...f };
    const t = e.tagName;
    if (t === 'B' || t === 'STRONG') f.b = true; if (t === 'I' || t === 'EM') f.i = true; if (t === 'U') f.u = true; if (t === 'S' || t === 'STRIKE' || t === 'DEL') f.s = true;
    if (t === 'SUP') f.va = 'sup'; if (t === 'SUB') f.va = 'sub'; if (t === 'CODE') f.font = 'Liberation Mono';
    const s = e.style;
    if (s) {
      if (s.fontWeight === 'bold' || parseInt(s.fontWeight, 10) >= 600) f.b = true;
      if (s.fontStyle === 'italic') f.i = true;
      if (/underline/.test(s.textDecorationLine || s.textDecoration || '')) f.u = true;
      if (/line-through/.test(s.textDecorationLine || s.textDecoration || '')) f.s = true;
      if (s.color) { const h = H.hex(s.color); if (h) f.color = '#' + h.toLowerCase(); }
      if (s.backgroundColor) { const h = H.hex(s.backgroundColor); if (h && h !== 'FFFFFF') f.bg = '#' + h.toLowerCase(); }
      if (s.fontSize) { const p = pt(s.fontSize); if (p) f.sz = Math.round(p * 2) / 2; }
      if (s.fontFamily) f.font = W.firstFont(s.fontFamily);
    }
    return f;
  };
  const parts = []; let picN = 0;
  const inlineXml = async (nodes, f) => {
    let out = '';
    for (const n of nodes) {
      if (n.nodeType === 3) {
        const t = n.data.replace(/​/g, '').replace(/\n/g, ' ');
        if (!t) continue;
        let x = esc(t).replace(/ {2,}/g, (m) => ' ' + `<text:s text:c="${m.length - 1}"/>`).replace(/\t/g, '<text:tab/>');
        const st = tStyle(f);
        out += st ? `<text:span text:style-name="${st}">${x}</text:span>` : x;
        continue;
      }
      if (n.nodeType !== 1) continue;
      const t = n.tagName;
      if (t === 'BR') out += '<text:line-break/>';
      else if (t === 'SPAN' && n.classList.contains('tab')) out += '<text:tab/>';
      else if (t === 'IMG') {
        const src = n.getAttribute('src') || '';
        let d = H.dataUrlBytes(src); if (!d) continue;
        let ext = { 'image/png': 'png', 'image/jpeg': 'jpg', 'image/gif': 'gif', 'image/svg+xml': 'svg' }[d.mime];
        let nat = await H.imageSize(src);
        if (!ext) { const png = await H.toPng(src); if (!png) continue; d = H.dataUrlBytes(png.url); ext = 'png'; nat = { w: png.w, h: png.h }; }
        const name = `Pictures/image${++picN}.${ext}`;
        pics.push({ name, bytes: d.bytes, mime: d.mime === 'image/svg+xml' ? 'image/svg+xml' : (ext === 'jpg' ? 'image/jpeg' : 'image/' + ext) });
        let wpx = parseFloat(n.style.width) || nat.w || 200; const maxW = (wmm - page.margins.l - page.margins.r) / 25.4 * 96; if (wpx > maxW) wpx = maxW;
        const hpx = nat.w ? wpx * nat.h / nat.w : wpx * 0.75;
        const block = n.style.display === 'block';
        out += `<draw:frame draw:name="Image${picN}" text:anchor-type="${block ? 'paragraph' : 'as-char'}" svg:width="${(wpx / 96 * 2.54).toFixed(3)}cm" svg:height="${(hpx / 96 * 2.54).toFixed(3)}cm" draw:z-index="${picN}"><draw:image xlink:href="${name}" xlink:type="simple" xlink:show="embed" xlink:actuate="onLoad"/>${n.alt ? `<svg:title>${esc(n.alt)}</svg:title>` : ''}</draw:frame>`;
      } else if (t === 'A' && n.getAttribute('href')) out += `<text:a xlink:type="simple" xlink:href="${esc(n.getAttribute('href'))}">${await inlineXml(n.childNodes, f)}</text:a>`;
      else if (/^(UL|OL|TABLE|P|DIV|H[1-6])$/.test(t)) continue;
      else out += await inlineXml(n.childNodes, fmtOf(n, f));
    }
    return out;
  };
  const paraProps = (e, parent) => {
    const s = e.style || {};
    const p = { parent };
    if (s.textAlign && s.textAlign !== 'left' && s.textAlign !== 'start') p.align = s.textAlign === 'right' ? 'end' : s.textAlign;
    if (s.lineHeight && !isNaN(parseFloat(s.lineHeight)) && !/px|pt|%/.test(s.lineHeight)) p.line = Math.round(parseFloat(s.lineHeight) * 100) + '%';
    if (s.marginTop) { const v = pt(s.marginTop); if (v != null) p.mt = v.toFixed(1) + 'pt'; }
    if (s.marginBottom) { const v = pt(s.marginBottom); if (v != null) p.mb = v.toFixed(1) + 'pt'; }
    if (s.marginLeft) { const v = pt(s.marginLeft); if (v) p.ml = v.toFixed(1) + 'pt'; }
    if (s.textIndent) { const v = pt(s.textIndent); if (v) p.ti = v.toFixed(1) + 'pt'; }
    if (s.backgroundColor) { const h = H.hex(s.backgroundColor); if (h) p.bg = '#' + h.toLowerCase(); }
    return p;
  };
  const pAttr = (e, parent) => { const p = paraProps(e, parent); return Object.keys(p).length > 1 ? pStyle(p) : parent; };
  let tableN = 0;
  const blocksXml = async (nodes) => {
    let out = '', loose = [];
    const flush = async () => { if (loose.length) { out += `<text:p text:style-name="Standard">${await inlineXml(loose, {})}</text:p>`; loose = []; } };
    for (const n of nodes) {
      if (n.nodeType === 3) { if (n.data.trim()) loose.push(n); continue; }
      if (n.nodeType !== 1) continue;
      const t = n.tagName;
      if (!/^(P|DIV|H[1-6]|UL|OL|TABLE|BLOCKQUOTE|PRE|HR|FIGURE)$/.test(t)) { loose.push(n); continue; }
      await flush();
      if (/^H[1-6]$/.test(t)) out += `<text:h text:style-name="${pAttr(n, 'Heading_20_' + t[1])}" text:outline-level="${t[1]}">${await inlineXml(n.childNodes, {})}</text:h>`;
      else if (t === 'P' || t === 'DIV' || t === 'FIGURE') {
        if (n.querySelector(':scope > p, :scope > div, :scope > table, :scope > ul, :scope > ol')) { out += await blocksXml(n.childNodes); continue; }
        const parent = n.classList.contains('title') ? 'Title' : n.classList.contains('subtitle') ? 'Subtitle' : 'Standard';
        out += `<text:p text:style-name="${pAttr(n, parent)}">${await inlineXml(n.childNodes, {})}</text:p>`;
      } else if (t === 'BLOCKQUOTE') out += `<text:p text:style-name="${pAttr(n, 'Quotations')}">${await inlineXml(n.childNodes, {})}</text:p>`;
      else if (t === 'PRE') for (const line of n.textContent.split('\n')) out += `<text:p text:style-name="Preformatted_20_Text">${esc(line).replace(/ {2,}/g, (m) => ' ' + `<text:s text:c="${m.length - 1}"/>`)}</text:p>`;
      else if (t === 'HR') out += n.classList.contains('page-break') ? '<text:p text:style-name="PageBreak"/>' : '<text:p text:style-name="Rule"/>';
      else if (t === 'UL' || t === 'OL') out += await listXml(n, 1);
      else if (t === 'TABLE') out += await tableXml(n);
    }
    await flush();
    return out;
  };
  const listStyleOf = (l) => { if (l.tagName === 'UL') return 'LB'; const ty = l.getAttribute('type'); return { a: 'La', A: 'LA', i: 'Li', I: 'LI' }[ty] || 'LN'; };
  const listXml = async (l, level) => {
    let out = `<text:list${level === 1 ? ` text:style-name="${listStyleOf(l)}"` : ''}>`;
    const start = parseInt(l.getAttribute('start') || '1', 10);
    let first = true;
    for (const li of l.children) {
      if (li.tagName === 'UL' || li.tagName === 'OL') { out += `<text:list-item>${await listXml(li, level + 1)}</text:list-item>`; continue; }
      if (li.tagName !== 'LI') continue;
      const inline = [], nested = [];
      for (const c of li.childNodes) { if (c.nodeType === 1 && (c.tagName === 'UL' || c.tagName === 'OL')) nested.push(c); else if (c.nodeType === 1 && c.tagName === 'P') inline.push(...c.childNodes); else inline.push(c); }
      out += `<text:list-item${first && start > 1 ? ` text:start-value="${start}"` : ''}><text:p text:style-name="${pAttr(li, 'List_20_Paragraph')}">${await inlineXml(inline, {})}</text:p>`;
      for (const s of nested) out += await listXml(s, level + 1);
      out += '</text:list-item>';
      first = false;
    }
    return out + '</text:list>';
  };
  const tableXml = async (t) => {
    const rows = Array.from(t.querySelectorAll(':scope > tbody > tr, :scope > thead > tr, :scope > tr'));
    let ncols = 0;
    for (const r of rows) { let c = 0; for (const cell of r.children) c += cell.colSpan || 1; ncols = Math.max(ncols, c); }
    ncols = Math.max(1, ncols);
    const name = `Table${++tableN}`;
    const border = t.classList.contains('nob') ? 'none' : '0.5pt solid #999999';
    let out = `<table:table table:name="${name}" table:style-name="Tbl"><table:table-column table:number-columns-repeated="${ncols}"/>`;
    const covered = {}; // "r,c" → true (rows spanned from above)
    rows.forEach((r, ri) => {
      out += '<table:table-row>';
      let ci = 0;
      const cells = Array.from(r.children).filter((c) => /^(TD|TH)$/.test(c.tagName));
      const parts2 = [];
      for (const c of cells) {
        while (covered[ri + ',' + ci]) { parts2.push('<table:covered-table-cell/>'); ci++; }
        const cs = c.colSpan || 1, rs = c.rowSpan || 1;
        for (let a = 0; a < rs; a++) for (let b = 0; b < cs; b++) if (a || b) covered[(ri + a) + ',' + (ci + b)] = true;
        const bg = H.hex(c.style.backgroundColor);
        parts2.push({ c, cs, rs, st: cStyle({ border, bg: bg ? '#' + bg.toLowerCase() : null }) });
        for (let b = 1; b < cs; b++) parts2.push('<table:covered-table-cell/>');
        ci += cs;
      }
      while (ci < ncols) { if (covered[ri + ',' + ci]) parts2.push('<table:covered-table-cell/>'); else parts2.push({ c: null, cs: 1, rs: 1, st: cStyle({ border, bg: null }) }); ci++; }
      out += '\u0000' + parts2.length; // placeholder count, filled below
      r._parts = parts2;
      out += '</table:table-row>';
    });
    // Cell contents need awaits: fill the placeholders in order.
    let i = 0; const fills = [];
    for (const r of rows) {
      let s = '';
      for (const p of r._parts) {
        if (typeof p === 'string') { s += p; continue; }
        let inner = '';
        if (p.c) {
          if (p.c.querySelector(':scope > p, :scope > ul, :scope > ol, :scope > table, :scope > h1, :scope > h2, :scope > h3')) inner = await blocksXml(p.c.childNodes);
          else inner = `<text:p text:style-name="${p.c.tagName === 'TH' ? 'Table_20_Heading' : pAttr(p.c, 'Table_20_Contents')}">${await inlineXml(p.c.childNodes, {})}</text:p>`;
        } else inner = '<text:p text:style-name="Table_20_Contents"/>';
        s += `<table:table-cell table:style-name="${p.st}" office:value-type="string"${p.cs > 1 ? ` table:number-columns-spanned="${p.cs}"` : ''}${p.rs > 1 ? ` table:number-rows-spanned="${p.rs}"` : ''}>${inner}</table:table-cell>`;
      }
      fills.push(s); delete r._parts; i++;
    }
    let k = 0;
    out = out.replace(/\u0000\d+/g, () => fills[k++]);
    return out + '</table:table>';
  };
  const bodyXml = await blocksXml(root.childNodes);
  const contentW = (wmm - page.margins.l - page.margins.r).toFixed(1);
  const ff = (f) => `<style:font-face style:name="${esc(f)}" svg:font-family="'${esc(f)}'"/>`;
  const textProps = (f) => {
    let a = '';
    if (f.b) a += ' fo:font-weight="bold" style:font-weight-asian="bold" style:font-weight-complex="bold"';
    if (f.i) a += ' fo:font-style="italic" style:font-style-asian="italic" style:font-style-complex="italic"';
    if (f.u) a += ' style:text-underline-style="solid" style:text-underline-width="auto" style:text-underline-color="font-color"';
    if (f.s) a += ' style:text-line-through-style="solid" style:text-line-through-type="single"';
    if (f.color) a += ` fo:color="${f.color}"`;
    if (f.bg) a += ` fo:background-color="${f.bg}"`;
    if (f.sz) a += ` fo:font-size="${f.sz}pt" style:font-size-asian="${f.sz}pt" style:font-size-complex="${f.sz}pt"`;
    if (f.font) a += ` style:font-name="${esc(f.font)}"`;
    if (f.va) a += ` style:text-position="${f.va === 'sup' ? 'super' : 'sub'} 58%"`;
    return `<style:text-properties${a}/>`;
  };
  const autoStyles = [];
  for (const { name, f } of T.values()) autoStyles.push(`<style:style style:name="${name}" style:family="text">${textProps(f)}</style:style>`);
  for (const { name, p } of P.values()) {
    let a = '';
    if (p.align) a += ` fo:text-align="${p.align}"`; if (p.line) a += ` fo:line-height="${p.line}"`;
    if (p.mt) a += ` fo:margin-top="${p.mt}"`; if (p.mb) a += ` fo:margin-bottom="${p.mb}"`; if (p.ml) a += ` fo:margin-left="${p.ml}"`; if (p.ti) a += ` fo:text-indent="${p.ti}"`;
    if (p.bg) a += ` fo:background-color="${p.bg}"`;
    autoStyles.push(`<style:style style:name="${name}" style:family="paragraph" style:parent-style-name="${p.parent}"><style:paragraph-properties${a}/></style:style>`);
  }
  for (const { name, c } of C.values()) autoStyles.push(`<style:style style:name="${name}" style:family="table-cell"><style:table-cell-properties fo:padding="0.1cm" fo:border="${c.border}"${c.bg ? ` fo:background-color="${c.bg}"` : ''}/></style:style>`);
  autoStyles.push(`<style:style style:name="Tbl" style:family="table"><style:table-properties style:width="${contentW}mm" table:align="margins"/></style:style>`);
  autoStyles.push('<style:style style:name="PageBreak" style:family="paragraph" style:parent-style-name="Standard"><style:paragraph-properties fo:break-before="page"/></style:style>');
  autoStyles.push('<style:style style:name="Rule" style:family="paragraph" style:parent-style-name="Standard"><style:paragraph-properties fo:border-bottom="0.5pt solid #808080" fo:padding="0.05cm"/></style:style>');
  const lvl = (kind, fmt) => Array.from({ length: 10 }, (_, i) => kind === 'b'
    ? `<text:list-level-style-bullet text:level="${i + 1}" text:bullet-char="${['•', '◦', '▪'][i % 3]}"><style:list-level-properties text:list-level-position-and-space-mode="label-alignment"><style:list-level-label-alignment text:label-followed-by="listtab" fo:text-indent="-0.635cm" fo:margin-left="${(1.27 * (i + 1)).toFixed(2)}cm"/></style:list-level-properties></text:list-level-style-bullet>`
    : `<text:list-level-style-number text:level="${i + 1}" style:num-suffix="." style:num-format="${i === 0 ? fmt : ['1', 'a', 'i'][i % 3]}"><style:list-level-properties text:list-level-position-and-space-mode="label-alignment"><style:list-level-label-alignment text:label-followed-by="listtab" fo:text-indent="-0.635cm" fo:margin-left="${(1.27 * (i + 1)).toFixed(2)}cm"/></style:list-level-properties></text:list-level-style-number>`).join('');
  autoStyles.push(`<text:list-style style:name="LB">${lvl('b')}</text:list-style>`, `<text:list-style style:name="LN">${lvl('n', '1')}</text:list-style>`,
    `<text:list-style style:name="La">${lvl('n', 'a')}</text:list-style>`, `<text:list-style style:name="LA">${lvl('n', 'A')}</text:list-style>`, `<text:list-style style:name="Li">${lvl('n', 'i')}</text:list-style>`, `<text:list-style style:name="LI">${lvl('n', 'I')}</text:list-style>`);
  const nsDecl = Object.entries(NS).map(([k, v]) => `xmlns:${k}="${v}"`).join(' ');
  const content = `<?xml version="1.0" encoding="UTF-8"?>
<office:document-content ${nsDecl} office:version="1.3"><office:font-face-decls>${Array.from(fontsUsed).map(ff).join('')}</office:font-face-decls><office:automatic-styles>${autoStyles.join('')}</office:automatic-styles><office:body><office:text>${bodyXml}</office:text></office:body></office:document-content>`;
  const hs = (lv, size, italic) => `<style:style style:name="Heading_20_${lv}" style:display-name="Heading ${lv}" style:family="paragraph" style:parent-style-name="Heading" style:next-style-name="Standard" style:default-outline-level="${lv}"><style:paragraph-properties fo:margin-top="12pt" fo:margin-bottom="4pt" fo:keep-with-next="always"/><style:text-properties fo:font-size="${size}pt" fo:font-weight="bold" fo:color="#2f5496"${italic ? ' fo:font-style="italic"' : ''}/></style:style>`;
  const pnXml = '<text:page-number text:select-page="current">1</text:page-number>';
  const numP = (align) => `<text:p text:style-name="${align === 'center' ? 'HFCenter' : 'HFRight'}">Page ${pnXml} of <text:page-count>1</text:page-count></text:p>`;
  let header = '', footer = '';
  if (page.header || page.numbers === 'header-right') header = `<style:header>${page.header ? `<text:p text:style-name="Header">${esc(page.header)}</text:p>` : ''}${page.numbers === 'header-right' ? numP('right') : ''}</style:header>`;
  if (page.footer || /^footer/.test(page.numbers)) footer = `<style:footer>${page.footer ? `<text:p text:style-name="Footer">${esc(page.footer)}</text:p>` : ''}${/^footer/.test(page.numbers) ? numP(page.numbers === 'footer-center' ? 'center' : 'right') : ''}</style:footer>`;
  const pw = page.orient === 'landscape' ? hmm : wmm, ph = page.orient === 'landscape' ? wmm : hmm;
  const W2 = page.orient === 'landscape' ? ph : pw, H2 = page.orient === 'landscape' ? pw : ph;
  const styles = `<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles ${nsDecl} office:version="1.3"><office:font-face-decls>${Array.from(fontsUsed).map(ff).join('')}<style:font-face style:name="Liberation Mono" svg:font-family="'Liberation Mono'"/></office:font-face-decls>
<office:styles>
<style:default-style style:family="paragraph"><style:paragraph-properties fo:margin-top="0pt" fo:margin-bottom="${page.spaceAfter ?? 8}pt" fo:line-height="${Math.round(page.lineHeight * 100)}%"/><style:text-properties style:font-name="${esc(page.font)}" fo:font-size="${page.fontSize}pt" fo:language="en" fo:country="US"/></style:default-style>
<style:style style:name="Standard" style:family="paragraph" style:class="text"><style:text-properties style:font-name="${esc(page.font)}" fo:font-size="${page.fontSize}pt"/></style:style>
<style:style style:name="Heading" style:family="paragraph" style:parent-style-name="Standard" style:class="text"/>
${hs(1, 18)}${hs(2, 15)}${hs(3, 13)}${hs(4, 12, true)}${hs(5, 11)}${hs(6, 11, true)}
<style:style style:name="Title" style:family="paragraph" style:parent-style-name="Heading" style:class="chapter"><style:text-properties fo:font-size="26pt"/></style:style>
<style:style style:name="Subtitle" style:family="paragraph" style:parent-style-name="Heading" style:class="chapter"><style:text-properties fo:font-size="15pt" fo:color="#595959"/></style:style>
<style:style style:name="Quotations" style:family="paragraph" style:parent-style-name="Standard" style:class="html"><style:paragraph-properties fo:margin-left="1cm" fo:margin-right="1cm"/><style:text-properties fo:font-style="italic" fo:color="#404040"/></style:style>
<style:style style:name="Preformatted_20_Text" style:display-name="Preformatted Text" style:family="paragraph" style:parent-style-name="Standard" style:class="html"><style:paragraph-properties fo:margin-top="0pt" fo:margin-bottom="0pt"/><style:text-properties style:font-name="Liberation Mono" fo:font-size="10pt"/></style:style>
<style:style style:name="List_20_Paragraph" style:display-name="List Paragraph" style:family="paragraph" style:parent-style-name="Standard" style:class="list"><style:paragraph-properties fo:margin-bottom="2pt"/></style:style>
<style:style style:name="Table_20_Contents" style:display-name="Table Contents" style:family="paragraph" style:parent-style-name="Standard" style:class="extra"><style:paragraph-properties fo:margin-bottom="0pt"/></style:style>
<style:style style:name="Table_20_Heading" style:display-name="Table Heading" style:family="paragraph" style:parent-style-name="Table_20_Contents" style:class="extra"><style:text-properties fo:font-weight="bold"/></style:style>
<style:style style:name="Header" style:family="paragraph" style:parent-style-name="Standard" style:class="extra"><style:paragraph-properties fo:margin-bottom="0pt"/></style:style>
<style:style style:name="Footer" style:family="paragraph" style:parent-style-name="Standard" style:class="extra"><style:paragraph-properties fo:margin-bottom="0pt"/></style:style>
<style:style style:name="HFCenter" style:family="paragraph" style:parent-style-name="Footer"><style:paragraph-properties fo:text-align="center"/></style:style>
<style:style style:name="HFRight" style:family="paragraph" style:parent-style-name="Footer"><style:paragraph-properties fo:text-align="end"/></style:style>
</office:styles>
<office:automatic-styles><style:page-layout style:name="pm1"><style:page-layout-properties fo:page-width="${W2.toFixed(1)}mm" fo:page-height="${H2.toFixed(1)}mm" style:print-orientation="${page.orient}" fo:margin-top="${Math.max(0, page.margins.t - (header ? 10 : 0)).toFixed(1)}mm" fo:margin-bottom="${Math.max(0, page.margins.b - (footer ? 10 : 0)).toFixed(1)}mm" fo:margin-left="${page.margins.l.toFixed(1)}mm" fo:margin-right="${page.margins.r.toFixed(1)}mm"/>${header ? '<style:header-style><style:header-footer-properties fo:min-height="0mm" fo:margin-bottom="5mm"/></style:header-style>' : ''}${footer ? '<style:footer-style><style:header-footer-properties fo:min-height="0mm" fo:margin-top="5mm"/></style:footer-style>' : ''}</style:page-layout></office:automatic-styles>
<office:master-styles><style:master-page style:name="Standard" style:page-layout-name="pm1">${header}${footer}</style:master-page></office:master-styles></office:document-styles>`;
  const meta = `<?xml version="1.0" encoding="UTF-8"?><office:document-meta ${nsDecl} office:version="1.3"><office:meta><meta:generator>Hearth Write</meta:generator><dc:title>${esc(H.stem(H.doc.name || ''))}</dc:title><meta:creation-date>${new Date().toISOString().slice(0, 19)}</meta:creation-date></office:meta></office:document-meta>`;
  const manifest = `<?xml version="1.0" encoding="UTF-8"?><manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.3"><manifest:file-entry manifest:full-path="/" manifest:version="1.3" manifest:media-type="application/vnd.oasis.opendocument.text"/><manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/><manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/><manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>${pics.map((p) => `<manifest:file-entry manifest:full-path="${p.name}" manifest:media-type="${p.mime}"/>`).join('')}</manifest:manifest>`;
  const zip = new JSZip();
  zip.file('mimetype', 'application/vnd.oasis.opendocument.text', { compression: 'STORE' });
  zip.file('content.xml', content); zip.file('styles.xml', styles); zip.file('meta.xml', meta);
  zip.file('META-INF/manifest.xml', manifest);
  for (const p of pics) zip.file(p.name, p.bytes);
  void parts;
  return zip.generateAsync({ type: 'uint8array', compression: 'DEFLATE', mimeType: 'application/vnd.oasis.opendocument.text' });
}

WriteFormats.odt = { read, write };
})();
