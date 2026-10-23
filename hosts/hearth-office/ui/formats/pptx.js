// Hearth Slides ↔ .pptx. Writing with PptxGenJS. Reading is Slides' own, best effort, with JSZip: slide size,
// text boxes and placeholders (positions taken from the layout/master when the slide doesn't say), text
// formatting and bullets, pictures, simple shapes, groups, tables (as text), backgrounds and speaker notes.
// Not read: charts, SmartArt, video/audio, animations, transitions, master-slide decorations.
'use strict';
(() => {
const A = 'http://schemas.openxmlformats.org/drawingml/2006/main';
const RNS = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships';
const xml = (s) => (s ? new DOMParser().parseFromString(s, 'application/xml') : null);
const kids = (e, n) => (e ? Array.from(e.children).filter((c) => !n || c.localName === n) : []);
const kid = (e, n) => kids(e, n)[0] || null;
const path = (e, ...names) => { let c = e; for (const n of names) { c = kid(c, n); if (!c) return null; } return c; };
const desc = (e, n) => (e ? Array.from(e.getElementsByTagNameNS('*', n)) : []);
const norm = (p) => { const o = []; for (const x of p.split('/')) { if (x === '..') o.pop(); else if (x && x !== '.') o.push(x); } return o.join('/'); };
const esc = H.esc;

async function rels(zip, file) {
  const dir = file.replace(/[^/]*$/, '');
  const rp = dir + '_rels/' + file.split('/').pop() + '.rels';
  const f = zip.file(rp); const out = {};
  if (!f) return out;
  for (const r of xml(await f.async('string')).getElementsByTagName('Relationship')) {
    const t = r.getAttribute('Target') || '';
    out[r.getAttribute('Id')] = { type: (r.getAttribute('Type') || '').split('/').pop(), target: r.getAttribute('TargetMode') === 'External' ? t : (t.startsWith('/') ? norm(t) : norm(dir + t)), external: r.getAttribute('TargetMode') === 'External' };
  }
  return out;
}

async function read(bytes) {
  const zip = await JSZip.loadAsync(bytes);
  const str = async (p) => { const f = zip.file(p); return f ? f.async('string') : null; };
  const pres = xml(await str('ppt/presentation.xml'));
  if (!pres) throw new Error('This isn\'t a PowerPoint file (no ppt/presentation.xml inside).');
  const prels = await rels(zip, 'ppt/presentation.xml');
  const sz = desc(pres, 'sldSz')[0];
  const cx = sz ? +sz.getAttribute('cx') : 12192000, cy = sz ? +sz.getAttribute('cy') : 6858000;
  const W = 960, Hh = Math.round(960 * cy / cx);
  const k = W / cx; // EMU → slide units
  const notes = new Set();
  // Theme colours.
  const themeRel = Object.values(prels).find((r) => r.type === 'theme');
  let scheme = {};
  const tx = themeRel && xml(await str(themeRel.target));
  if (tx) { const cs = desc(tx, 'clrScheme')[0]; for (const c of kids(cs)) { const v = c.firstElementChild; if (v) scheme[c.localName] = (v.localName === 'sysClr' ? v.getAttribute('lastClr') : v.getAttribute('val')) || null; } }
  const fontScheme = {}; if (tx) { const maj = desc(tx, 'majorFont')[0], min = desc(tx, 'minorFont')[0]; if (maj) fontScheme.major = path(maj, 'latin') && path(maj, 'latin').getAttribute('typeface'); if (min) fontScheme.minor = path(min, 'latin') && path(min, 'latin').getAttribute('typeface'); }
  const P = { w: W, h: Hh, theme: 'paper', slides: [] };
  const ids = desc(pres, 'sldId');
  const cache = {};
  const loadPart = async (p) => { if (cache[p]) return cache[p]; const d = xml(await str(p)); const r = await rels(zip, p); return (cache[p] = { d, r, p }); };
  for (const sid of ids) {
    const rel = prels[sid.getAttributeNS(RNS, 'id') || sid.getAttribute('r:id')];
    if (!rel) continue;
    const sl = await loadPart(rel.target);
    const layoutRel = Object.values(sl.r).find((r) => r.type === 'slideLayout');
    const lay = layoutRel ? await loadPart(layoutRel.target) : null;
    const masterRel = lay && Object.values(lay.r).find((r) => r.type === 'slideMaster');
    const mas = masterRel ? await loadPart(masterRel.target) : null;
    // Colour map (bg1 → lt1 ...), from the master.
    const cmap = {}; const cm = mas && desc(mas.d, 'clrMap')[0];
    if (cm) for (const at of cm.attributes) cmap[at.name] = at.value;
    const color = (fillEl) => {
      if (!fillEl) return null;
      const c = fillEl.firstElementChild; if (!c) return null;
      let hex = null;
      if (c.localName === 'srgbClr') hex = c.getAttribute('val');
      else if (c.localName === 'schemeClr') { let n = c.getAttribute('val'); n = cmap[n] || ({ bg1: 'lt1', tx1: 'dk1', bg2: 'lt2', tx2: 'dk2' }[n]) || n; hex = scheme[n]; }
      else if (c.localName === 'sysClr') hex = c.getAttribute('lastClr');
      else if (c.localName === 'prstClr') hex = { black: '000000', white: 'FFFFFF', red: 'FF0000', blue: '0000FF', green: '00FF00', yellow: 'FFFF00' }[c.getAttribute('val')] || null;
      if (!hex) return null;
      const lm = kid(c, 'lumMod'), lo = kid(c, 'lumOff');
      if (lm || lo) { const f = lm ? +lm.getAttribute('val') / 100000 : 1, o = lo ? +lo.getAttribute('val') / 100000 : 0; hex = [0, 2, 4].map((i) => Math.round(Math.min(255, parseInt(hex.substr(i, 2), 16) * f + 255 * o)).toString(16).padStart(2, '0')).join(''); }
      return '#' + hex.toLowerCase();
    };
    // Default text sizes and colours by placeholder kind, from the master's text styles.
    const txStyles = mas && desc(mas.d, 'txStyles')[0];
    const lvl1 = (name) => { const s = txStyles && kid(txStyles, name); const l = s && kid(s, 'lvl1pPr'); const d = l && kid(l, 'defRPr'); return { sz: d && d.getAttribute('sz') ? +d.getAttribute('sz') / 100 : null, color: d ? color(kid(d, 'solidFill')) : null, bullet: !!(l && (kid(l, 'buChar') || kid(l, 'buAutoNum'))) }; };
    const defs = { title: lvl1('titleStyle'), body: lvl1('bodyStyle'), other: lvl1('otherStyle') };
    const phOf = (sp) => { const ph = desc(kid(sp, 'nvSpPr') || kid(sp, 'nvPicPr') || sp, 'ph')[0]; return ph ? { type: ph.getAttribute('type') || 'body', idx: ph.getAttribute('idx') } : null; };
    const findPh = (part, ph) => { if (!part || !ph) return null; const sps = desc(part.d, 'sp'); return sps.find((s) => { const p = phOf(s); return p && p.idx != null && p.idx === ph.idx && ph.idx != null; }) || sps.find((s) => { const p = phOf(s); return p && (p.type === ph.type || (ph.type === 'ctrTitle' && p.type === 'title') || (ph.type === 'subTitle' && p.type === 'body')); }); };
    const xfrmOf = (sp) => { const x = desc(kid(sp, 'spPr') || kid(sp, 'grpSpPr') || sp, 'xfrm')[0]; if (!x) return null; const off = kid(x, 'off'), ext = kid(x, 'ext'); if (!off || !ext) return null; return { x: +off.getAttribute('x'), y: +off.getAttribute('y'), w: +ext.getAttribute('cx'), h: +ext.getAttribute('cy'), rot: +(x.getAttribute('rot') || 0) / 60000 }; };
    const bgOf = (part) => { if (!part) return null; const bg = desc(part.d, 'bg')[0]; if (!bg) return null; const pr = kid(bg, 'bgPr'); if (pr) { const sf = kid(pr, 'solidFill'); if (sf) return color(sf); const gf = kid(pr, 'gradFill'); if (gf) { const g = desc(gf, 'gs')[0]; if (g) return color(g); } const bf = kid(pr, 'blipFill'); if (bf) return { blip: bf, part }; } const ref = kid(bg, 'bgRef'); if (ref) return color(ref); return null; };
    const s = { id: 'p' + H.uid(), layout: 'blank', els: [], notes: '' };
    let bg = bgOf(sl) || bgOf(lay) || bgOf(mas);
    if (bg && typeof bg === 'object') { const blip = desc(bg.blip, 'blip')[0]; const r = blip && bg.part.r[blip.getAttributeNS(RNS, 'embed') || blip.getAttribute('r:embed')]; if (r) { const f = zip.file(r.target); if (f) s.bgImage = `data:${H.mimeOf(r.target.split('.').pop().toLowerCase())};base64,${await f.async('base64')}`; } bg = null; }
    s.bg = bg || '#ffffff';
    const tree = desc(sl.d, 'spTree')[0];
    // Shapes, pictures, groups and tables, in drawing order.
    const walk = async (nodes, map) => {
      for (const n of nodes) {
        const ln = n.localName;
        if (ln === 'grpSp') {
          const x = xfrmOf(n); const ch = desc(kid(n, 'grpSpPr'), 'chOff')[0], ce = desc(kid(n, 'grpSpPr'), 'chExt')[0];
          let m2 = map;
          if (x && ch && ce && +ce.getAttribute('cx') && +ce.getAttribute('cy')) { const sx = x.w / +ce.getAttribute('cx'), sy = x.h / +ce.getAttribute('cy'), ox = +ch.getAttribute('x'), oy = +ch.getAttribute('y'); m2 = (r) => map({ x: x.x + (r.x - ox) * sx, y: x.y + (r.y - oy) * sy, w: r.w * sx, h: r.h * sy, rot: r.rot }); }
          await walk(kids(n), m2);
        } else if (ln === 'sp' || ln === 'cxnSp') {
          const ph = phOf(n);
          if (ph && ['dt', 'ftr', 'sldNum', 'hdr'].includes(ph.type)) continue;
          let xf = xfrmOf(n);
          const lph = ph ? findPh(lay, ph) : null, mph = ph ? findPh(mas, ph) : null;
          if (!xf) xf = (lph && xfrmOf(lph)) || (mph && xfrmOf(mph));
          if (!xf) continue;
          const r = map(xf);
          const spPr = kid(n, 'spPr');
          const geom = desc(spPr, 'prstGeom')[0]; const prst = geom ? geom.getAttribute('prst') : (desc(spPr, 'custGeom')[0] ? 'custom' : null);
          const fillEl = kid(spPr, 'solidFill'); const noFill = kid(spPr, 'noFill');
          const ln2 = kid(spPr, 'ln'); const lineColor = ln2 && !kid(ln2, 'noFill') ? color(kid(ln2, 'solidFill')) : null; const lineW = ln2 && ln2.getAttribute('w') ? Math.max(1, Math.round(+ln2.getAttribute('w') / 12700 * 96 / 72 * k * 9525)) : 2;
          // Style references (shapes drawn with the theme's accent colour).
          const style = kid(n, 'style'); const styleFill = style ? color(kid(style, 'fillRef')) : null; const styleLine = style ? color(kid(style, 'lnRef')) : null;
          const styleFont = style ? color(kid(style, 'fontRef')) : null;
          const kind = ph ? (ph.type === 'title' || ph.type === 'ctrTitle' ? 'title' : ph.type === 'subTitle' ? 'subtitle' : 'body') : null;
          const d0 = kind === 'title' ? defs.title : kind ? defs.body : defs.other;
          // Text: the slide's own, else nothing (placeholder prompts from the layout aren't content).
          const body = kid(n, 'txBody');
          const t = body ? textOf(body, color, d0, kind === 'body' && d0.bullet, lph, mph, fontScheme) : null;
          const shapeKind = { rect: 'rect', roundRect: 'round', ellipse: 'ellipse', triangle: 'triangle', rightArrow: 'arrow', line: 'line', straightConnector1: 'line', star5: 'star' }[prst];
          const isShape = !ph && prst && prst !== 'rect' ? true : !ph && (fillEl || (style && !noFill && prst)) && !kid(kid(n, 'nvSpPr') || n, 'cNvSpPr')?.getAttribute('txBox');
          const base = { id: 'e' + H.uid(), x: Math.round(r.x * k), y: Math.round(r.y * k), w: Math.max(1, Math.round(r.w * k)), h: Math.max(1, Math.round(r.h * k)) };
          if (r.rot) base.rot = Math.round(r.rot);
          if (ln === 'cxnSp' || shapeKind === 'line') { s.els.push({ ...base, type: 'shape', shape: 'line', line: lineColor || styleLine || '#000000', lineW, h: Math.max(1, base.h), flip: !!desc(spPr, 'xfrm')[0]?.getAttribute('flipV') }); continue; }
          if (isShape && !noFill) {
            const e = { ...base, type: 'shape', shape: shapeKind || 'rect', fill: color(fillEl) || styleFill || '#4472c4' };
            if (lineColor) { e.line = lineColor; e.lineW = lineW; }
            if (t) { Object.assign(e, t); if (styleFont) e.color = styleFont; } else { e.html = ''; e.size = 18; e.align = 'center'; }
            if (prst && !shapeKind && prst !== 'custom') notes.add('Some shapes are drawn as rectangles.');
            s.els.push(e); continue;
          }
          if (!t || D_isEmpty(t.html)) { if (fillEl && !noFill) s.els.push({ ...base, type: 'shape', shape: 'rect', fill: color(fillEl), html: '' }); continue; }
          const e = { ...base, type: 'text', role: kind || undefined, ...t };
          if (fillEl && !noFill) e.fill = color(fillEl);
          if (lineColor) { e.line = lineColor; e.lineW = lineW; }
          s.els.push(e);
        } else if (ln === 'pic') {
          let xf = xfrmOf(n);
          if (!xf) { const ph = phOf(n); const lph = ph && findPh(lay, ph); xf = lph && xfrmOf(lph); }
          if (!xf) continue;
          const r = map(xf);
          const blip = desc(n, 'blip')[0]; const id = blip && (blip.getAttributeNS(RNS, 'embed') || blip.getAttribute('r:embed'));
          const rr = id && sl.r[id]; if (!rr || rr.external) continue;
          const ext = rr.target.split('.').pop().toLowerCase();
          if (['emf', 'wmf'].includes(ext)) { notes.add('Some pictures (EMF/WMF) can\'t be shown.'); continue; }
          const f = zip.file(rr.target); if (!f) continue;
          s.els.push({ id: 'e' + H.uid(), type: 'image', src: `data:${H.mimeOf(ext)};base64,${await f.async('base64')}`, x: Math.round(r.x * k), y: Math.round(r.y * k), w: Math.round(r.w * k), h: Math.round(r.h * k), alt: (desc(n, 'cNvPr')[0] || { getAttribute: () => '' }).getAttribute('descr') || '' });
        } else if (ln === 'graphicFrame') {
          const xf0 = desc(kid(n, 'xfrm') || n, 'off')[0] ? { x: +desc(n, 'off')[0].getAttribute('x'), y: +desc(n, 'off')[0].getAttribute('y'), w: +desc(n, 'ext')[0].getAttribute('cx'), h: +desc(n, 'ext')[0].getAttribute('cy') } : null;
          const tbl = desc(n, 'tbl')[0];
          if (tbl && xf0) {
            const r = map(xf0);
            const rows = desc(tbl, 'tr').map((tr) => '<tr>' + kids(tr, 'tc').filter((tc) => !tc.getAttribute('hMerge') && !tc.getAttribute('vMerge')).map((tc) => `<td${tc.getAttribute('gridSpan') ? ` colspan="${tc.getAttribute('gridSpan')}"` : ''}>${esc(desc(tc, 'p').map((p) => desc(p, 't').map((x) => x.textContent).join('')).join(' '))}</td>`).join('') + '</tr>').join('');
            s.els.push({ id: 'e' + H.uid(), type: 'text', x: Math.round(r.x * k), y: Math.round(r.y * k), w: Math.round(r.w * k), h: Math.round(r.h * k), size: 16, align: 'left', valign: 'top', html: `<table class="stbl">${rows}</table>` });
          } else if (desc(n, 'chart').length) notes.add('Charts aren\'t shown.');
          else if (desc(n, 'relIds').length) notes.add('SmartArt isn\'t shown.');
        } else if (ln === 'AlternateContent') { const ch = kid(n, 'Choice') || kid(n, 'Fallback'); if (ch) await walk(kids(ch), map); }
      }
    };
    await walk(kids(tree), (r) => r);
    // Speaker notes.
    const nr = Object.values(sl.r).find((r) => r.type === 'notesSlide');
    if (nr) { const nd = xml(await str(nr.target)); if (nd) { const sp = desc(nd, 'sp').find((x) => { const ph = desc(x, 'ph')[0]; return ph && (ph.getAttribute('type') || 'body') === 'body'; }); if (sp) s.notes = desc(sp, 'p').map((p) => desc(p, 't').map((t) => t.textContent).join('')).join('\n').trim(); } }
    P.slides.push(s);
  }
  // A dark deck reads better with a dark theme around new slides.
  return { P, notes: Array.from(notes) };
}
const D_isEmpty = (html) => !html || !html.replace(/<(?!img)[^>]*>/g, '').replace(/&nbsp;|\s/g, '');
// A text body as HTML with the box's main size, alignment and colour.
function textOf(body, color, d0, defBullets, lph, mph, fontScheme) {
  const bp = kid(body, 'bodyPr');
  const anchor = bp && bp.getAttribute('anchor');
  const lstStyle = (sp) => { const b = sp && kid(sp, 'txBody'); const l = b && kid(b, 'lstStyle'); const p = l && kid(l, 'lvl1pPr'); const d = p && kid(p, 'defRPr'); return { sz: d && d.getAttribute('sz') ? +d.getAttribute('sz') / 100 : null, color: d ? color(kid(d, 'solidFill')) : null, algn: p && p.getAttribute('algn') }; };
  const L = lstStyle(lph), M = lstStyle(mph);
  const baseSz = L.sz || M.sz || d0.sz || 18;
  const norm = desc(bp, 'normAutofit')[0]; const scale = norm && norm.getAttribute('fontScale') ? +norm.getAttribute('fontScale') / 100000 : 1;
  const sizes = [];
  let html = '', listOpen = null, firstAlign = null;
  const closeList = () => { if (listOpen) { html += `</${listOpen}>`; listOpen = null; } };
  for (const p of kids(body, 'p')) {
    const pPr = kid(p, 'pPr');
    const algn = (pPr && pPr.getAttribute('algn')) || L.algn || M.algn;
    const align = { ctr: 'center', r: 'right', just: 'justify' }[algn] || 'left';
    if (firstAlign === null) firstAlign = align;
    const bu = !pPr ? undefined : kid(pPr, 'buChar') ? 'ul' : kid(pPr, 'buAutoNum') ? 'ol' : kid(pPr, 'buNone') ? null : undefined;
    const list = bu === undefined ? (defBullets ? 'ul' : null) : bu;
    let inner = '';
    for (const r of kids(p)) {
      if (r.localName === 'br') { inner += '<br>'; continue; }
      if (r.localName !== 'r' && r.localName !== 'fld') continue;
      const rp = kid(r, 'rPr');
      let t = esc(desc(r, 't').map((x) => x.textContent).join(''));
      if (!t) continue;
      const css = [];
      if (rp && rp.getAttribute('sz')) { const sz = +rp.getAttribute('sz') / 100 * scale; sizes.push(sz); if (Math.abs(sz - baseSz * scale) > 0.5) css.push(`font-size:${(sz * 96 / 72 * 960 / 960).toFixed(1)}px`); }
      const c = rp && color(kid(rp, 'solidFill')); if (c) css.push(`color:${c}`);
      const lat = rp && kid(rp, 'latin'); if (lat && lat.getAttribute('typeface') && !/^\+/.test(lat.getAttribute('typeface'))) css.push(`font-family:"${lat.getAttribute('typeface')}"`);
      if (css.length) t = `<span style="${css.join(';')}">${t}</span>`;
      if (rp && rp.getAttribute('b') === '1') t = `<b>${t}</b>`;
      if (rp && rp.getAttribute('i') === '1') t = `<i>${t}</i>`;
      if (rp && rp.getAttribute('u') && rp.getAttribute('u') !== 'none') t = `<u>${t}</u>`;
      if (rp && rp.getAttribute('strike') && rp.getAttribute('strike') !== 'noStrike') t = `<s>${t}</s>`;
      const link = rp && kid(rp, 'hlinkClick'); void link;
      inner += t;
    }
    const style = align !== firstAlign ? ` style="text-align:${align}"` : '';
    if (list) { if (listOpen !== list) { closeList(); html += `<${list}>`; listOpen = list; } html += `<li${style}>${inner || '<br>'}</li>`; }
    else { closeList(); html += `<div${style}>${inner || '<br>'}</div>`; }
  }
  closeList();
  // px at 960 wide: points × 96/72 × (960 / 960).
  const size = Math.round(baseSz * scale * 96 / 72);
  const out = { html: html.replace(/(<div><br><\/div>)+$/, ''), size, align: firstAlign || 'left', valign: anchor === 'ctr' ? 'middle' : anchor === 'b' ? 'bottom' : 'top' };
  const col = L.color || M.color || d0.color; if (col) out.color = col;
  void fontScheme; void sizes;
  return out;
}

// ------------------------------------------------------------------ writing
async function write(P, T) {
  const pptx = new PptxGenJS();
  const wIn = 10, hIn = 10 * P.h / P.w;
  pptx.defineLayout({ name: 'HEARTH', width: wIn, height: hIn });
  pptx.layout = 'HEARTH';
  pptx.author = 'Hearth Slides'; pptx.title = H.stem(H.doc.name || 'Presentation');
  const inch = (v) => +(v / 96).toFixed(3); // slide units are px at 96 per inch (960 = 10 in)
  const hex = (c) => (c ? H.hex(c) : null);
  const firstFont = (css) => (css || '').split(',')[0].trim().replace(/^["']|["']$/g, '');
  for (const s of P.slides) {
    const sl = pptx.addSlide();
    if (s.bgImage) { const d = s.bgImage.replace(/^data:/, ''); sl.background = { data: d }; }
    else sl.background = { color: hex(s.bg || T.bg) || 'FFFFFF' };
    for (const e of s.els) {
      const pos = { x: inch(e.x), y: inch(e.y), w: inch(Math.max(1, e.w)), h: inch(Math.max(1, e.h)) };
      if (e.rot) pos.rotate = e.rot;
      if (e.type === 'image') {
        let src = e.src || '';
        if (!/^data:image\/(png|jpe?g|gif)/.test(src)) { const png = await H.toPng(src); if (!png) continue; src = png.url; }
        sl.addImage({ data: src.replace(/^data:/, ''), ...pos, altText: e.alt || undefined });
        continue;
      }
      if (e.type === 'shape' && e.shape === 'line') {
        sl.addShape(pptx.ShapeType.line, { ...pos, h: inch(Math.max(0, e.h)), line: { color: hex(e.line) || '000000', width: (e.lineW || 3) * 0.75 }, flipV: !!e.flip });
        continue;
      }
      const roleColor = e.color || (e.role === 'title' ? T.title : e.type === 'shape' ? '#ffffff' : T.text);
      const fontFace = e.font || firstFont(e.role === 'title' ? T.titleFont : T.font);
      const runs = htmlRuns(e.html || '', { color: hex(roleColor), size: (e.size || 24) * 0.75, bold: e.bold || e.role === 'title', font: fontFace });
      const opts = {
        ...pos, fontFace, fontSize: +((e.size || 24) * 0.75).toFixed(1), color: hex(roleColor) || '000000',
        bold: !!(e.bold || e.role === 'title'), align: e.align || 'left', valign: e.valign === 'middle' ? 'middle' : e.valign === 'bottom' ? 'bottom' : 'top',
        margin: 4, fit: 'none', wrap: true,
      };
      if (e.type === 'shape') {
        opts.shape = pptx.ShapeType[{ rect: 'rect', round: 'roundRect', ellipse: 'ellipse', triangle: 'triangle', arrow: 'rightArrow', star: 'star5' }[e.shape] || 'rect'];
        const f = e.fill === undefined ? T.accent : e.fill;
        if (f) opts.fill = { color: hex(f) }; else opts.fill = { type: 'none' };
        if (e.line) opts.line = { color: hex(e.line), width: (e.lineW || 2) * 0.75 };
        sl.addText(runs.length ? runs : [{ text: '', options: {} }], opts);
        continue;
      }
      if (e.fill) opts.fill = { color: hex(e.fill) };
      if (e.line) opts.line = { color: hex(e.line), width: (e.lineW || 2) * 0.75 };
      // A table in a text box becomes a real table.
      if (/<table/i.test(e.html || '')) {
        const box = document.createElement('div'); box.innerHTML = e.html;
        const rows = Array.from(box.querySelectorAll('tr')).map((tr) => Array.from(tr.children).map((td) => ({ text: td.textContent, options: { colspan: td.colSpan > 1 ? td.colSpan : undefined } })));
        if (rows.length) { sl.addTable(rows, { x: pos.x, y: pos.y, w: pos.w, fontSize: opts.fontSize, color: opts.color, fontFace, border: { type: 'solid', pt: 0.75, color: '999999' } }); continue; }
      }
      if (!runs.length || window.H.apps.slides.isEmpty(e.html)) continue;
      sl.addText(runs, opts);
    }
    if (s.notes) sl.addNotes(s.notes);
  }
  const out = await pptx.write({ outputType: 'uint8array' });
  return out instanceof Uint8Array ? out : new Uint8Array(out);
}
// HTML of a text box → PptxGenJS text runs (paragraphs end with breakLine; list items get bullets).
function htmlRuns(html, base) {
  const box = document.createElement('div'); box.innerHTML = html;
  const runs = [];
  const paras = [];
  const walkBlocks = (n, list, level) => {
    let inline = [];
    const flush = (el) => { if (inline.length || el) { paras.push({ nodes: inline, list, level, align: el && el.style ? el.style.textAlign : '' }); inline = []; } };
    for (const c of Array.from(n.childNodes)) {
      if (c.nodeType === 1 && (c.tagName === 'UL' || c.tagName === 'OL')) { flush(); for (const li of c.children) { if (li.tagName === 'LI') { const sub = Array.from(li.childNodes).filter((x) => !(x.nodeType === 1 && /^(UL|OL)$/.test(x.tagName))); paras.push({ nodes: sub, list: c.tagName === 'OL' ? 'ol' : 'ul', level, align: li.style.textAlign }); for (const s of li.children) if (/^(UL|OL)$/.test(s.tagName)) walkBlocks({ childNodes: [s] }, null, level + 1); } } }
      else if (c.nodeType === 1 && /^(DIV|P|H[1-6]|BLOCKQUOTE)$/.test(c.tagName)) { flush(); paras.push({ nodes: Array.from(c.childNodes), list, level, align: c.style.textAlign }); }
      else if (c.nodeType === 1 && c.tagName === 'BR') { inline.push(c); }
      else inline.push(c);
    }
    flush();
  };
  walkBlocks(box, null, 0);
  const fmtOf = (e, f) => {
    f = { ...f }; const t = e.tagName;
    if (t === 'B' || t === 'STRONG') f.bold = true; if (t === 'I' || t === 'EM') f.italic = true; if (t === 'U') f.underline = { style: 'sng' }; if (t === 'S' || t === 'STRIKE') f.strike = 'sngStrike';
    const s = e.style;
    if (s) { if (s.fontWeight === 'bold' || +s.fontWeight >= 600) f.bold = true; if (s.fontStyle === 'italic') f.italic = true; if (/underline/.test(s.textDecoration || s.textDecorationLine || '')) f.underline = { style: 'sng' }; if (s.color) { const h = H.hex(s.color); if (h) f.color = h; } if (s.fontSize && /px$/.test(s.fontSize)) f.fontSize = +(parseFloat(s.fontSize) * 0.75).toFixed(1); if (s.fontFamily) f.fontFace = s.fontFamily.split(',')[0].replace(/["']/g, '').trim(); }
    return f;
  };
  paras.forEach((p, pi) => {
    const pr = [];
    const walk = (nodes, f) => { for (const n of nodes) { if (n.nodeType === 3) { const t = n.data.replace(/​/g, ''); if (t) pr.push({ text: t, options: { ...f } }); } else if (n.nodeType === 1) { if (n.tagName === 'BR') pr.push({ text: '', options: { ...f, breakLine: true } }); else walk(Array.from(n.childNodes), fmtOf(n, f)); } } };
    walk(p.nodes, {});
    if (!pr.length) pr.push({ text: '', options: {} });
    const last = pr[pr.length - 1];
    if (pi < paras.length - 1) last.options.breakLine = true;
    if (p.list) { pr[0].options.bullet = p.list === 'ol' ? { type: 'number' } : true; if (p.level) pr[0].options.indentLevel = p.level; }
    if (p.align) pr[0].options.align = p.align;
    runs.push(...pr);
  });
  void base;
  return runs;
}

window.SlidesPptx = { read, write };
})();
