// Hearth Write ↔ .rtf (Rich Text Format). Write's own reader and writer.
// Kept: fonts, sizes, colours, highlight, bold/italic/underline/strikethrough, super/subscript, alignment, spacing,
// indents, headings (by style name or outline level), lists (as bulleted/numbered items), tables, PNG/JPEG pictures,
// links, page breaks, page size and margins, header and footer text, page numbers.
// Not kept: other picture types (WMF/EMF), footnotes, comments, fields other than links and page numbers.
'use strict';
window.WriteFormats = window.WriteFormats || {};
(() => {
const SKIP = new Set(['fonttbl', 'colortbl', 'stylesheet', 'info', 'listtable', 'listoverridetable', 'themedata', 'colorschememapping', 'datastore', 'latentstyles', 'rsidtbl', 'generator', 'xmlnstbl', 'mmathPr', 'pgdsctbl', 'pn', 'nonshppict', 'footnote', 'annotation', 'atnid', 'atnauthor', 'bkmkstart', 'bkmkend', 'fldinst', 'object', 'objdata', 'filetbl', 'revtbl', 'userprops', 'docvar', 'wgrffmtfilter', 'pntext', 'listtext', 'headerl', 'headerr', 'headerf', 'footerl', 'footerr', 'footerf', 'ftnsep', 'ftnsepc', 'aftnsep', 'aftnsepc', 'xe', 'tc', 'template', 'background', 'sp', 'shpinst', 'blipuid', 'passwordhash']);
const DEST = new Set(['fonttbl', 'colortbl', 'stylesheet', 'info', 'pict', 'header', 'footer', 'field', 'fldinst', 'fldrslt', 'shppict', 'pntext', 'listtext', ...SKIP]);
const CP1252 = { 0x80: 0x20ac, 0x82: 0x201a, 0x83: 0x192, 0x84: 0x201e, 0x85: 0x2026, 0x86: 0x2020, 0x87: 0x2021, 0x88: 0x2c6, 0x89: 0x2030, 0x8a: 0x160, 0x8b: 0x2039, 0x8c: 0x152, 0x8e: 0x17d, 0x91: 0x2018, 0x92: 0x2019, 0x93: 0x201c, 0x94: 0x201d, 0x95: 0x2022, 0x96: 0x2013, 0x97: 0x2014, 0x98: 0x2dc, 0x99: 0x2122, 0x9a: 0x161, 0x9b: 0x203a, 0x9c: 0x153, 0x9e: 0x17e, 0x9f: 0x178 };
const ansi = (b) => String.fromCharCode(CP1252[b] || b);

function tokenize(s) {
  const out = []; let i = 0; const n = s.length;
  while (i < n) {
    const c = s[i];
    if (c === '{') { out.push({ t: '{' }); i++; }
    else if (c === '}') { out.push({ t: '}' }); i++; }
    else if (c === '\\') {
      const d = s[i + 1];
      if (/[a-zA-Z]/.test(d)) {
        let j = i + 1; while (j < n && /[a-zA-Z]/.test(s[j])) j++;
        const word = s.slice(i + 1, j);
        let k = j; if (s[k] === '-') k++; while (k < n && /[0-9]/.test(s[k])) k++;
        const num = k > j ? parseInt(s.slice(j, k), 10) : null;
        if (s[k] === ' ') k++;
        out.push({ t: 'w', w: word, n: num }); i = k;
      } else if (d === "'") { out.push({ t: 'x', v: parseInt(s.substr(i + 2, 2), 16) }); i += 4; }
      else if (d === '\r' || d === '\n') { out.push({ t: 'w', w: 'par', n: null }); i += 2; }
      else { out.push({ t: 's', v: d }); i += 2; }
    } else if (c === '\r' || c === '\n') i++;
    else { let j = i; while (j < n && !'{}\\\r\n'.includes(s[j])) j++; out.push({ t: 'text', v: s.slice(i, j) }); i = j; }
  }
  return out;
}

function read(src) {
  const toks = tokenize(src);
  const fonts = {}, colors = [], styles = {};
  const root = document.createElement('div');
  const page = {};
  let st = { b: 0, i: 0, u: 0, s: 0, fs: 24, f: null, cf: 0, hl: 0, va: '', uc: 1, dest: null, skip: false };
  const stack = [];
  let para = null, pfmt = { align: '', li: 0, fi: 0, sb: null, sa: null, sl: null, intbl: false, outline: null, style: null, pn: null, list: null };
  const resetP = () => { pfmt = { align: '', li: 0, fi: 0, sb: null, sa: null, sl: null, intbl: false, outline: null, style: null, pn: null, list: null }; };
  let table = null, row = null, cell = null, cellBuf = [];
  let curList = null; // {el, ordered}
  let field = null; // {inst, rslt}
  let ucSkip = 0;
  let hdr = null; // collecting header/footer text: {kind, text, page}
  let pict = null;
  let fontEntry = null, colorBuf = null, styleBuf = null;
  const target = () => (field && field.inRslt ? field.a : null);
  const newPara = () => { para = document.createElement('p'); return para; };
  const fmtKey = () => JSON.stringify([st.b, st.i, st.u, st.s, st.fs, st.f, st.cf, st.hl, st.va]);
  let lastKey = null, lastSpan = null;
  const emit = (text) => {
    if (!text) return;
    if (st.dest === 'fldinst' && field) { field.inst += text; return; }
    if (hdr) { if (!(st.dest === 'fldrslt' && field && /\b(PAGE|NUMPAGES)\b/.test(field.inst))) hdr.text += text; return; }
    if (st.skip) return;
    if (st.dest === 'pntext') { pfmt.pn = (pfmt.pn || '') + text; return; }
    if (st.dest === 'listtext') { pfmt.pn = (pfmt.pn || '') + text; return; }
    if (!para) newPara();
    const into = target() || para;
    const key = fmtKey() + (into === para ? 'p' : 'a');
    if (key === lastKey && lastSpan && lastSpan.parentNode === into) { lastSpan.textContent += text; return; }
    let node = document.createTextNode(text);
    let outer = node;
    const w = (tag) => { const e = document.createElement(tag); e.append(outer); outer = e; };
    if (st.va === 'super') w('sup'); else if (st.va === 'sub') w('sub');
    if (st.s) w('s'); if (st.u) w('u'); if (st.i) w('i'); if (st.b) w('b');
    const css = [];
    if (st.f != null && fonts[st.f] && fonts[st.f] !== fonts[0]) css.push('font-family:' + H.apps.write.fontCss(fonts[st.f]));
    if (st.fs && st.fs !== baseFs) css.push(`font-size:${st.fs / 2}pt`);
    if (st.cf && colors[st.cf] && colors[st.cf] !== '#000000') css.push('color:' + colors[st.cf]);
    if (st.hl && colors[st.hl]) css.push('background-color:' + colors[st.hl]);
    if (css.length) { const sp = document.createElement('span'); sp.setAttribute('style', css.join(';')); sp.append(outer); outer = sp; }
    into.append(outer);
    lastKey = key; lastSpan = node.parentNode === null ? null : node;
    // Keep appending into the same text node for runs with the same look.
    lastSpan = node;
  };
  let baseFs = 24;
  const finishPara = (forceEmpty) => {
    if (!para && !forceEmpty) return;
    const p = para || document.createElement('p');
    para = null; lastKey = null;
    if (!p.firstChild) p.append(document.createElement('br'));
    // Paragraph style: heading by style name or outline level.
    let el = p;
    const sname = pfmt.style != null ? (styles[pfmt.style] || '').toLowerCase() : '';
    let m = /^heading\s*(\d)/.exec(sname);
    const lvl = m ? +m[1] : (pfmt.outline != null ? pfmt.outline + 1 : 0);
    if (lvl >= 1 && lvl <= 6) { el = document.createElement('h' + lvl); el.append(...p.childNodes); }
    else if (sname === 'title') p.className = 'title';
    else if (sname === 'subtitle') p.className = 'subtitle';
    // Headings and titles get their look from Write's styles: the bold, size and colour written around them go.
    if (el !== p || p.className) {
      el.querySelectorAll('b').forEach((b) => b.replaceWith(...b.childNodes));
      el.querySelectorAll('span').forEach((sp) => { sp.style.fontSize = ''; sp.style.color = ''; if (!sp.getAttribute('style')) sp.replaceWith(...sp.childNodes); });
    }
    const s = el.style;
    if (pfmt.align) s.textAlign = pfmt.align;
    if (pfmt.sl && pfmt.sl > 0 && pfmt.slmult) s.lineHeight = String(Math.round(pfmt.sl / 240 * 100) / 100);
    if (pfmt.sb != null) s.marginTop = pfmt.sb / 20 + 'pt';
    if (pfmt.sa != null) s.marginBottom = pfmt.sa / 20 + 'pt';
    // A list item: from the bullet or number text in front of it (\pntext / \listtext), or the \pn settings.
    const pn = (pfmt.pn || '').replace(/\t/g, '').trim();
    const isItem = pfmt.list || pn;
    if (isItem && !pfmt.intbl) {
      const ordered = pfmt.list === 'ol' || (!pfmt.list && /^[0-9a-zA-Z]+[.)]$/.test(pn));
      if (!curList || curList.ordered !== ordered) {
        curList = { el: document.createElement(ordered ? 'ol' : 'ul'), ordered };
        if (ordered) { const mm = /^(\d+)/.exec(pn); if (mm && +mm[1] > 1) curList.el.setAttribute('start', mm[1]); }
        root.append(curList.el);
      }
      const li = document.createElement('li');
      li.append(...el.childNodes);
      if (s.textAlign) li.style.textAlign = s.textAlign;
      curList.el.append(li);
      return;
    }
    curList = null;
    if (pfmt.li > 0 && !pfmt.intbl) s.marginLeft = Math.round(pfmt.li / 1440 * 25.4) + 'mm';
    if (pfmt.fi && !pfmt.intbl) s.textIndent = Math.round(pfmt.fi / 1440 * 25.4) + 'mm';
    if (pfmt.intbl) { cellBuf.push(el); return; }
    root.append(el);
  };
  for (let k = 0; k < toks.length; k++) {
    const t = toks[k];
    if (t.t === '{') { stack.push({ ...st }); continue; }
    if (t.t === '}') {
      const was = st;
      if (was.dest === 'fonttbl' && fontEntry) { if (fontEntry.name.trim()) fonts[fontEntry.n] = fontEntry.name.trim(); fontEntry = null; }
      if (was.dest === 'pict' && pict && stack.length && stack[stack.length - 1].dest !== 'pict') {
        const hex = pict.hex.replace(/[^0-9a-fA-F]/g, '');
        if (pict.type && hex.length > 8) {
          const bytes = new Uint8Array(hex.length >> 1);
          for (let i = 0; i < bytes.length; i++) bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
          const img = document.createElement('img');
          img.src = H.dataUrl(bytes, pict.type);
          const wTw = pict.wgoal ? pict.wgoal * (pict.scalex || 100) / 100 : null;
          if (wTw) img.style.width = Math.round(wTw / 15) + 'px';
          if (!st.skip || true) { if (!para) newPara(); para.append(img); lastKey = null; }
        }
        pict = null;
      }
      if ((was.dest === 'header' || was.dest === 'footer') && hdr && stack.length && stack[stack.length - 1].dest !== was.dest) {
        const text = hdr.text.replace(/\bPage\b|\bof\b/gi, hdr.page ? '' : '$&').replace(/\s+/g, ' ').trim();
        page[was.dest] = text;
        if (hdr.page) page.numbers = was.dest === 'header' ? 'header-right' : (hdr.align === 'right' ? 'footer-right' : 'footer-center');
        hdr = null;
      }
      if (was.dest === 'field' && field && stack.length && stack[stack.length - 1].dest !== 'field') {
        if (field.a && field.a.childNodes.length) { if (!para) newPara(); para.append(field.a); lastKey = null; }
        field = null;
      }
      if (was.dest === 'fldinst' && field) {
        const m = /HYPERLINK\s+"([^"]+)"/i.exec(field.inst);
        if (m && /^(https?:|mailto:)/i.test(m[1])) { field.a = document.createElement('a'); field.a.setAttribute('href', m[1]); }
        else if (/\b(PAGE|NUMPAGES)\b/.test(field.inst) && hdr) hdr.page = true;
      }
      if (was.dest === 'fldrslt' && field) field.inRslt = false;
      if (was.dest === 'colortbl' && colorBuf) colorBuf = null;
      if (was.dest === 'stylesheet' && styleBuf) { if (styleBuf.n != null) styles[styleBuf.n] = styleBuf.name.replace(/;$/, '').trim(); styleBuf = null; }
      st = stack.pop() || st;
      continue;
    }
    if (t.t === 'x') {
      if (ucSkip > 0) { ucSkip--; continue; }
      if (st.dest === 'pict' && pict) continue;
      if (st.dest === 'fonttbl') { if (fontEntry) fontEntry.name += ansi(t.v); continue; }
      emit(ansi(t.v)); continue;
    }
    if (t.t === 's') {
      if (t.v === '*') { st.skip = true; st.dest = st.dest || 'unknown'; continue; }
      if (t.v === '~') emit('\u00a0'); else if (t.v === '_') emit('\u2011'); else if (t.v === '-') {} else if ('\\{}'.includes(t.v)) emit(t.v);
      continue;
    }
    if (t.t === 'text') {
      let v = t.v;
      if (ucSkip > 0) { const cut = Math.min(ucSkip, v.length); v = v.slice(cut); ucSkip -= cut; }
      if (!v) continue;
      if (st.dest === 'pict' && pict) { pict.hex += v; continue; }
      if (st.dest === 'fonttbl') { if (fontEntry) { fontEntry.name += v; if (fontEntry.name.includes(';')) { fonts[fontEntry.n] = fontEntry.name.split(';')[0].trim(); fontEntry = null; } } continue; }
      if (st.dest === 'colortbl') { if (v.includes(';')) { for (const ch of v) if (ch === ';') { colors.push(colorBuf && colorBuf.set ? '#' + [colorBuf.r, colorBuf.g, colorBuf.b].map((x) => (x || 0).toString(16).padStart(2, '0')).join('') : null); colorBuf = { set: false }; } } continue; }
      if (st.dest === 'stylesheet') { if (styleBuf) styleBuf.name += v; continue; }
      emit(v); continue;
    }
    // A control word.
    const w = t.w, n = t.n;
    if (ucSkip > 0 && w !== 'u') ucSkip = 0;
    if (DEST.has(w) || w === 'pict') {
      st.dest = w;
      if (SKIP.has(w) && w !== 'fonttbl' && w !== 'colortbl' && w !== 'stylesheet' && w !== 'fldinst' && w !== 'pntext' && w !== 'listtext') st.skip = true;
      if (w === 'fonttbl' || w === 'colortbl' || w === 'stylesheet') { st.skip = true; if (w === 'colortbl') colorBuf = { set: false }; }
      if (w === 'pntext' || w === 'listtext') { st.skip = false; pfmt.pn = ''; }
      if (w === 'pict') { pict = { hex: '', type: null }; st.skip = false; }
      if (w === 'shppict') st.skip = false;
      if (w === 'header' || w === 'footer') { hdr = { kind: w, text: '', page: false, align: '' }; st.skip = false; }
      if (w === 'field') { field = { inst: '', a: null, inRslt: false }; st.skip = false; }
      if (w === 'fldinst') { st.skip = false; }
      if (w === 'fldrslt' && field) { field.inRslt = true; st.skip = false; }
      continue;
    }
    if (st.dest === 'fonttbl') {
      if (w === 'f') fontEntry = { n, name: '' };
      continue;
    }
    if (st.dest === 'colortbl') { if (!colorBuf) colorBuf = { set: false }; if (w === 'red') { colorBuf.r = n; colorBuf.set = true; } if (w === 'green') { colorBuf.g = n; colorBuf.set = true; } if (w === 'blue') { colorBuf.b = n; colorBuf.set = true; } continue; }
    if (st.dest === 'stylesheet') { if (w === 's' || w === 'cs' || w === 'ds' || w === 'ts') styleBuf = { n: w === 's' ? n : null, name: '' }; continue; }
    if (st.dest === 'pict' && pict) {
      if (w === 'pngblip') pict.type = 'image/png'; else if (w === 'jpegblip') pict.type = 'image/jpeg';
      else if (w === 'picwgoal') pict.wgoal = n; else if (w === 'pichgoal') pict.hgoal = n; else if (w === 'picscalex') pict.scalex = n;
      else if (w === 'bin') { /* binary pictures aren't supported */ }
      continue;
    }
    switch (w) {
      case 'rtf': case 'ansi': case 'deff': break;
      case 'uc': st.uc = n ?? 1; break;
      case 'u': { let c = n; if (c < 0) c += 65536; emit(String.fromCharCode(c)); ucSkip = st.uc; break; }
      case 'par': if (hdr) { hdr.text += ' '; break; } if (!st.skip) finishPara(true); break;
      case 'line': if (st.dest === 'pntext' || st.dest === 'listtext') break; if (hdr) { hdr.text += ' '; break; } if (!para) newPara(); para.append(document.createElement('br')); lastKey = null; break;
      case 'tab': if (st.dest === 'pntext' || st.dest === 'listtext') break; if (hdr) { hdr.text += ' '; break; } if (!st.skip) { if (!para) newPara(); const sp = document.createElement('span'); sp.className = 'tab'; sp.textContent = '\t'; para.append(sp); lastKey = null; } break;
      case 'page': if (!st.skip) { finishPara(false); curList = null; const hr = document.createElement('hr'); hr.className = 'page-break'; root.append(hr); } break;
      case 'emdash': emit('\u2014'); break; case 'endash': emit('\u2013'); break; case 'bullet': emit('\u2022'); break;
      case 'lquote': emit('\u2018'); break; case 'rquote': emit('\u2019'); break; case 'ldblquote': emit('\u201c'); break; case 'rdblquote': emit('\u201d'); break;
      case 'plain': Object.assign(st, { b: 0, i: 0, u: 0, s: 0, fs: baseFs, f: 0, cf: 0, hl: 0, va: '' }); break;
      case 'b': st.b = n === 0 ? 0 : 1; break; case 'i': st.i = n === 0 ? 0 : 1; break;
      case 'ul': st.u = n === 0 ? 0 : 1; break; case 'ulnone': st.u = 0; break; case 'uldb': case 'ulw': case 'uld': st.u = 1; break;
      case 'strike': case 'striked': st.s = n === 0 ? 0 : 1; break;
      case 'fs': st.fs = n || 24; break; case 'f': st.f = n; break;
      case 'cf': st.cf = n || 0; break; case 'highlight': case 'cb': case 'chcbpat': st.hl = n || 0; break;
      case 'super': st.va = 'super'; break; case 'sub': st.va = 'sub'; break; case 'nosupersub': st.va = ''; break;
      case 'pard': resetP(); break;
      case 'ql': pfmt.align = ''; if (hdr) hdr.align = 'left'; break; case 'qc': pfmt.align = 'center'; if (hdr) hdr.align = 'center'; break;
      case 'qr': pfmt.align = 'right'; if (hdr) hdr.align = 'right'; break; case 'qj': pfmt.align = 'justify'; break;
      case 'li': pfmt.li = n || 0; break; case 'fi': pfmt.fi = n || 0; break;
      case 'sb': pfmt.sb = n; break; case 'sa': pfmt.sa = n; break;
      case 'sl': pfmt.sl = n; break; case 'slmult': pfmt.slmult = n; break;
      case 's': pfmt.style = n; break;
      case 'outlinelevel': pfmt.outline = n; break;
      case 'pnlvlblt': pfmt.list = 'ul'; break; case 'pnlvlbody': pfmt.list = pfmt.list || 'ol'; break; case 'pndec': pfmt.list = 'ol'; break;
      case 'ls': pfmt.list = pfmt.list || null; pfmt.ls = n; break;
      case 'intbl': pfmt.intbl = true; break;
      case 'trowd': if (!table) { finishPara(false); curList = null; table = document.createElement('table'); table.className = 't'; table.append(document.createElement('tbody')); root.append(table); } row = document.createElement('tr'); cellBuf = []; break;
      case 'cell': {
        finishPara(false);
        if (!row) { row = document.createElement('tr'); }
        if (!table) { table = document.createElement('table'); table.className = 't'; table.append(document.createElement('tbody')); root.append(table); }
        const td = document.createElement('td');
        if (cellBuf.length === 1 && cellBuf[0].tagName === 'P' && !cellBuf[0].getAttribute('style')) td.append(...cellBuf[0].childNodes); else td.append(...cellBuf);
        if (!td.firstChild || (td.childNodes.length === 1 && td.firstChild.nodeName === 'BR')) { td.innerHTML = '<br>'; }
        row.append(td); cellBuf = [];
        break;
      }
      case 'row': if (row && table) { table.querySelector('tbody').append(row); row = null; } break;
      case 'paperw': page.pw = n; break; case 'paperh': page.ph = n; break;
      case 'margl': page.ml = n; break; case 'margr': page.mr = n; break; case 'margt': page.mt = n; break; case 'margb': page.mb = n; break;
      case 'landscape': page.land = true; break;
      case 'chpgn': if (hdr) hdr.page = true; break;
      default: break;
    }
    // Paragraphs outside a table end the table.
    if (w === 'pard' && table && !lookAheadIntbl(toks, k)) { table = null; }
  }
  finishPara(false);
  const out = { html: root.innerHTML, page: {} };
  if (page.pw && page.ph) {
    let w = page.pw / 1440 * 25.4, h = page.ph / 1440 * 25.4;
    const land = page.land || w > h; const pw = Math.min(w, h), ph = Math.max(w, h);
    out.page.orient = land ? 'landscape' : 'portrait';
    const name = Object.entries(H.apps.write.SIZES).find(([, d]) => Math.abs(d[0] - pw) < 2 && Math.abs(d[1] - ph) < 2);
    if (name) out.page.size = name[0]; else { out.page.size = 'Custom'; out.page.w = pw; out.page.h = ph; }
  }
  const mm = (v, d) => (v != null ? Math.round(v / 1440 * 25.4 * 10) / 10 : d);
  if (page.ml != null || page.mt != null) out.page.margins = { t: mm(page.mt, 25.4), r: mm(page.mr, 31.75), b: mm(page.mb, 25.4), l: mm(page.ml, 31.75) };
  if (page.header) out.page.header = page.header;
  if (page.footer) out.page.footer = page.footer;
  if (page.numbers) out.page.numbers = page.numbers;
  if (typeof fonts[0] === 'string' && fonts[0]) out.page.font = fonts[0];
  return out;
}
// Whether the paragraph that starts at token k is inside a table (\intbl before the next \par).
function lookAheadIntbl(toks, k) {
  for (let i = k + 1; i < toks.length && i < k + 60; i++) {
    const t = toks[i];
    if (t.t === 'w' && t.w === 'intbl') return true;
    if (t.t === 'w' && (t.w === 'par' || t.w === 'cell' || t.w === 'row')) return false;
    if (t.t === 'text') return false;
  }
  return false;
}

// ------------------------------------------------------------------ writing
async function write(root, page) {
  const W = H.apps.write;
  const fonts = [page.font || 'Calibri'], colors = [];
  const fi = (name) => { if (!name) return 0; let i = fonts.indexOf(name); if (i < 0) { fonts.push(name); i = fonts.length - 1; } return i; };
  const ci = (hex) => { if (!hex) return 0; let i = colors.indexOf(hex); if (i < 0) { colors.push(hex); i = colors.length - 1; } return i + 1; };
  const baseFs = Math.round((page.fontSize || 11) * 2);
  const enc = (t) => {
    let s = '';
    for (const ch of t) {
      const c = ch.codePointAt(0);
      if (ch === '\\' || ch === '{' || ch === '}') s += '\\' + ch;
      else if (ch === '\t') s += '\\tab ';
      else if (ch === '\u00a0') s += '\\~';
      else if (c < 128) s += ch;
      else if (c < 0x10000) s += `\\u${c > 32767 ? c - 65536 : c}?`;
      else { const hi = Math.floor((c - 0x10000) / 0x400) + 0xd800, lo = (c - 0x10000) % 0x400 + 0xdc00; s += `\\u${hi - 65536}?\\u${lo - 65536}?`; }
    }
    return s;
  };
  const pt = (v) => { if (!v) return null; const n = parseFloat(v); if (/pt$/.test(v)) return n; if (/px$/.test(v)) return n * 0.75; if (/mm$/.test(v)) return n / 25.4 * 72; return n; };
  const fmtOf = (e, f) => {
    f = { ...f }; const t = e.tagName;
    if (t === 'B' || t === 'STRONG') f.b = 1; if (t === 'I' || t === 'EM') f.i = 1; if (t === 'U') f.u = 1; if (t === 'S' || t === 'STRIKE' || t === 'DEL') f.s = 1;
    if (t === 'SUP') f.va = 'super'; if (t === 'SUB') f.va = 'sub'; if (t === 'CODE') f.font = 'Courier New';
    const s = e.style;
    if (s) {
      if (s.fontWeight === 'bold' || parseInt(s.fontWeight, 10) >= 600) f.b = 1;
      if (s.fontStyle === 'italic') f.i = 1;
      if (/underline/.test(s.textDecorationLine || s.textDecoration || '')) f.u = 1;
      if (/line-through/.test(s.textDecorationLine || s.textDecoration || '')) f.s = 1;
      if (s.color) f.color = H.hex(s.color);
      if (s.backgroundColor) { const h = H.hex(s.backgroundColor); if (h && h !== 'FFFFFF') f.bg = h; }
      if (s.fontSize) { const p = pt(s.fontSize); if (p) f.fs = Math.round(p * 2); }
      if (s.fontFamily) f.font = W.firstFont(s.fontFamily);
    }
    return f;
  };
  const runCodes = (f) => {
    let c = '';
    if (f.b) c += '\\b'; if (f.i) c += '\\i'; if (f.u) c += '\\ul'; if (f.s) c += '\\strike';
    if (f.va === 'super') c += '\\super'; if (f.va === 'sub') c += '\\sub';
    if (f.color) c += '\\cf' + ci(f.color);
    if (f.bg) c += '\\highlight' + ci(f.bg) + '\\chcbpat' + ci(f.bg);
    if (f.fs) c += '\\fs' + f.fs;
    if (f.font) c += '\\f' + fi(f.font);
    return c;
  };
  const inline = async (nodes, f) => {
    let out = '';
    for (const n of nodes) {
      if (n.nodeType === 3) { const t = n.data.replace(/\u200b/g, '').replace(/\n/g, ' '); if (t) { const c = runCodes(f); out += c ? `{${c} ${enc(t)}}` : enc(t); } continue; }
      if (n.nodeType !== 1) continue;
      const t = n.tagName;
      if (t === 'BR') out += '\\line ';
      else if (t === 'SPAN' && n.classList.contains('tab')) out += '\\tab ';
      else if (t === 'IMG') out += await pictRtf(n);
      else if (t === 'A' && /^(https?:|mailto:)/i.test(n.getAttribute('href') || '')) out += `{\\field{\\*\\fldinst HYPERLINK "${n.getAttribute('href').replace(/["\\{}]/g, '')}"}{\\fldrslt {\\ul\\cf${ci('0563C1')} ${await inline(n.childNodes, f)}}}}`;
      else if (/^(UL|OL|TABLE|P|DIV|H[1-6])$/.test(t)) continue;
      else out += await inline(n.childNodes, fmtOf(n, f));
    }
    return out;
  };
  const pictRtf = async (img) => {
    const src = img.getAttribute('src') || '';
    let d = H.dataUrlBytes(src); if (!d) return '';
    let nat = await H.imageSize(src);
    let blip = d.mime === 'image/png' ? 'pngblip' : d.mime === 'image/jpeg' ? 'jpegblip' : null;
    if (!blip) { const p = await H.toPng(src); if (!p) return ''; d = H.dataUrlBytes(p.url); blip = 'pngblip'; nat = { w: p.w, h: p.h }; }
    let wpx = parseFloat(img.style.width) || nat.w || 200;
    const maxW = (W.pageMm()[0] - page.margins.l - page.margins.r) / 25.4 * 96; if (wpx > maxW) wpx = maxW;
    const hpx = nat.w ? wpx * nat.h / nat.w : wpx * 0.75;
    let hex = ''; for (let i = 0; i < d.bytes.length; i++) { hex += d.bytes[i].toString(16).padStart(2, '0'); if (i % 64 === 63) hex += '\n'; }
    return `{\\pict\\${blip}\\picw${nat.w || Math.round(wpx)}\\pich${nat.h || Math.round(hpx)}\\picwgoal${Math.round(wpx * 15)}\\pichgoal${Math.round(hpx * 15)}\n${hex}}`;
  };
  const parCodes = (e, extra = '') => {
    const s = e.style || {};
    let c = '\\pard\\plain' + extra;
    if (s.textAlign === 'center') c += '\\qc'; else if (s.textAlign === 'right') c += '\\qr'; else if (s.textAlign === 'justify') c += '\\qj';
    if (s.lineHeight && !isNaN(parseFloat(s.lineHeight)) && !/px|pt|%/.test(s.lineHeight)) c += `\\sl${Math.round(parseFloat(s.lineHeight) * 240)}\\slmult1`;
    if (s.marginTop) { const v = pt(s.marginTop); if (v != null) c += '\\sb' + Math.round(v * 20); }
    if (s.marginBottom) { const v = pt(s.marginBottom); if (v != null) c += '\\sa' + Math.round(v * 20); }
    if (s.marginLeft) { const v = pt(s.marginLeft); if (v) c += '\\li' + Math.round(v * 20); }
    if (s.textIndent) { const v = pt(s.textIndent); if (v) c += '\\fi' + Math.round(v * 20); }
    return c + `\\f0\\fs${baseFs} `;
  };
  const HS = [0, 36, 30, 26, 24, 22, 22];
  const blocks = async (nodes, inTable) => {
    let out = '', loose = [];
    const flush = async () => { if (loose.length) { out += `\\pard\\plain\\f0\\fs${baseFs} ${await inline(loose, {})}\\par\n`; loose = []; } };
    const intbl = inTable ? '\\intbl' : '';
    for (const n of nodes) {
      if (n.nodeType === 3) { if (n.data.trim()) loose.push(n); continue; }
      if (n.nodeType !== 1) continue;
      const t = n.tagName;
      if (!/^(P|DIV|H[1-6]|UL|OL|TABLE|BLOCKQUOTE|PRE|HR|FIGURE)$/.test(t)) { loose.push(n); continue; }
      await flush();
      if (/^H[1-6]$/.test(t)) out += parCodes(n, `${intbl}\\s${t[1]}\\outlinelevel${+t[1] - 1}\\sb240\\sa80`) + `{\\b\\fs${HS[t[1]]}\\cf${ci('2F5496')} ${await inline(n.childNodes, {})}}\\par\n`;
      else if (t === 'P' || t === 'DIV' || t === 'FIGURE' || t === 'BLOCKQUOTE') {
        if (n.querySelector(':scope > p, :scope > div, :scope > table, :scope > ul, :scope > ol')) { out += await blocks(n.childNodes, inTable); continue; }
        const title = n.classList.contains('title'), sub = n.classList.contains('subtitle');
        const pre = title ? `{\\fs52 ` : sub ? `{\\fs30\\cf${ci('595959')} ` : t === 'BLOCKQUOTE' ? '{\\i ' : '';
        out += parCodes(n, intbl + (title ? '\\s15' : sub ? '\\s16' : t === 'BLOCKQUOTE' ? '\\li720\\ri720' : '')) + pre + await inline(n.childNodes, {}) + (pre ? '}' : '') + '\\par\n';
      } else if (t === 'PRE') out += `\\pard\\plain${intbl}\\f${fi('Courier New')}\\fs20 ${n.textContent.split('\n').map(enc).join('\\line ')}\\par\n`;
      else if (t === 'HR') out += n.classList.contains('page-break') ? '\\page\n' : `\\pard\\plain${intbl}\\brdrb\\brdrs\\brdrw10\\brsp20 \\par\n`;
      else if (t === 'UL' || t === 'OL') out += await list(n, 0, intbl);
      else if (t === 'TABLE' && !inTable) out += await tableRtf(n);
      else if (t === 'TABLE') out += await blocks([document.createTextNode(n.innerText)], inTable);
    }
    await flush();
    return out;
  };
  const list = async (l, level, intbl) => {
    let out = '', i = parseInt(l.getAttribute('start') || '1', 10);
    for (const li of l.children) {
      if (li.tagName === 'UL' || li.tagName === 'OL') { out += await list(li, level + 1, intbl); continue; }
      if (li.tagName !== 'LI') continue;
      const inl = [], nested = [];
      for (const c of li.childNodes) { if (c.nodeType === 1 && (c.tagName === 'UL' || c.tagName === 'OL')) nested.push(c); else if (c.nodeType === 1 && c.tagName === 'P') inl.push(...c.childNodes); else inl.push(c); }
      const ind = 360 * (level + 1);
      const label = l.tagName === 'OL' ? `${i++}.` : '\\u8226?';
      const pn = l.tagName === 'OL' ? `{\\*\\pn\\pnlvlbody\\pndec\\pnstart${i - 1}\\pnindent360{\\pntxta .}}` : `{\\*\\pn\\pnlvlblt\\pnf${fi('Symbol')}\\pnindent360{\\pntxtb\\'B7}}`;
      out += parCodes(li, `${intbl}\\li${ind + 360}\\fi-360`) + `{\\pntext ${label}\\tab}${pn}` + await inline(inl, {}) + '\\par\n';
      for (const s of nested) out += await list(s, level + 1, intbl);
    }
    return out;
  };
  const tableRtf = async (t) => {
    const rows = Array.from(t.querySelectorAll(':scope > tbody > tr, :scope > thead > tr, :scope > tr'));
    let ncols = 0; for (const r of rows) { let c = 0; for (const x of r.children) c += x.colSpan || 1; ncols = Math.max(ncols, c); }
    ncols = Math.max(1, ncols);
    const total = Math.round((W.pageMm()[0] - page.margins.l - page.margins.r) / 25.4 * 1440);
    const colW = total / ncols;
    const brd = t.classList.contains('nob') ? '' : '\\clbrdrt\\brdrs\\brdrw10\\clbrdrl\\brdrs\\brdrw10\\clbrdrb\\brdrs\\brdrw10\\clbrdrr\\brdrs\\brdrw10';
    let out = '';
    for (const r of rows) {
      let defs = '\\trowd\\trgaph108\\trleft0', x = 0, cells = '';
      for (const c of Array.from(r.children).filter((e) => /^(TD|TH)$/.test(e.tagName))) {
        x += colW * (c.colSpan || 1);
        const bg = H.hex(c.style.backgroundColor);
        defs += brd + (bg ? '\\clcbpat' + ci(bg) : '') + '\\cellx' + Math.round(x);
        const inner = c.querySelector(':scope > p, :scope > ul, :scope > ol, :scope > h1, :scope > h2, :scope > h3') ? (await blocks(c.childNodes, true)).replace(/\\par\n$/, '') : `\\pard\\plain\\intbl\\f0\\fs${baseFs} ${c.tagName === 'TH' ? '{\\b ' : ''}${await inline(c.childNodes, {})}${c.tagName === 'TH' ? '}' : ''}`;
        cells += inner + '\\cell\n';
      }
      out += defs + '\n' + cells + '\\row\n';
    }
    return out + '\\pard\\plain\n';
  };
  const body = await blocks(root.childNodes, false);
  const [wmm, hmm] = W.pageMm();
  const tw = (mm) => Math.round(mm / 25.4 * 1440);
  const pn = '{\\field{\\*\\fldinst PAGE}{\\fldrslt 1}}', np = '{\\field{\\*\\fldinst NUMPAGES}{\\fldrslt 1}}';
  let hf = '';
  if (page.header || page.numbers === 'header-right') hf += `{\\header\\pard\\plain${page.numbers === 'header-right' && !page.header ? '\\qr' : ''}\\f0\\fs${baseFs - 2} ${enc(page.header || '')}${page.numbers === 'header-right' ? `${page.header ? '\\tab ' : ''}Page ${pn} of ${np}` : ''}\\par}`;
  if (page.footer || /^footer/.test(page.numbers)) hf += `{\\footer\\pard\\plain${page.numbers === 'footer-center' ? '\\qc' : page.numbers === 'footer-right' ? '\\qr' : ''}\\f0\\fs${baseFs - 2} ${enc(page.footer || '')}${/^footer/.test(page.numbers) ? `${page.footer ? ' ' : ''}Page ${pn} of ${np}` : ''}\\par}`;
  fi('Symbol');
  const fontTbl = '{\\fonttbl' + fonts.map((f, i) => `{\\f${i}\\f${/mono|courier/i.test(f) ? 'modern' : /times|serif|georgia|cambria|garamond/i.test(f) ? 'roman' : f === 'Symbol' ? 'tech' : 'swiss'}\\fcharset${f === 'Symbol' ? 2 : 0} ${enc(f)};}`).join('') + '}';
  const colorTbl = '{\\colortbl ;' + colors.map((h) => `\\red${parseInt(h.slice(0, 2), 16)}\\green${parseInt(h.slice(2, 4), 16)}\\blue${parseInt(h.slice(4, 6), 16)};`).join('') + '}';
  const styleTbl = `{\\stylesheet{\\s0 Normal;}{\\s1\\outlinelevel0 heading 1;}{\\s2\\outlinelevel1 heading 2;}{\\s3\\outlinelevel2 heading 3;}{\\s4\\outlinelevel3 heading 4;}{\\s5\\outlinelevel4 heading 5;}{\\s6\\outlinelevel5 heading 6;}{\\s15 Title;}{\\s16 Subtitle;}}`;
  const pw = tw(page.orient === 'landscape' ? hmm : wmm), ph = tw(page.orient === 'landscape' ? wmm : hmm);
  return `{\\rtf1\\ansi\\ansicpg1252\\deff0\\uc1${fontTbl}${colorTbl}${styleTbl}{\\*\\generator Hearth Write;}\n` +
    `\\paperw${page.orient === 'landscape' ? ph : pw}\\paperh${page.orient === 'landscape' ? pw : ph}${page.orient === 'landscape' ? '\\landscape' : ''}\\margl${tw(page.margins.l)}\\margr${tw(page.margins.r)}\\margt${tw(page.margins.t)}\\margb${tw(page.margins.b)}\\sa${Math.round((page.spaceAfter ?? 8) * 20)}\n${hf}\n${body}}`;
}

WriteFormats.rtf = { read, write };
})();
