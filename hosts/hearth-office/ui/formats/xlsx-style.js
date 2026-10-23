// Hearth Sheets: cell formatting in .xlsx files. SheetJS Community Edition reads and writes values, formulas,
// number formats, merges and sizes but not fonts, fills or borders; this reads them from xl/styles.xml and the
// sheets' s="…" attributes, and writes them back into the file SheetJS made (a new styles.xml, every cell's
// style index, formatted empty cells, and frozen panes).
'use strict';
(() => {
const NS = 'http://schemas.openxmlformats.org/spreadsheetml/2006/main';
const RNS = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships';
const xml = (s) => new DOMParser().parseFromString(s, 'application/xml');
const kids = (e, n) => (e ? Array.from(e.children).filter((c) => !n || c.localName === n) : []);
const kid = (e, n) => kids(e, n)[0] || null;
const norm = (p) => { const o = []; for (const x of p.split('/')) { if (x === '..') o.pop(); else if (x && x !== '.') o.push(x); } return o.join('/'); };
// The legacy indexed colours (Excel's 56-colour palette, after the 8 basics).
const IDX = ['000000', 'FFFFFF', 'FF0000', '00FF00', '0000FF', 'FFFF00', 'FF00FF', '00FFFF', '000000', 'FFFFFF', 'FF0000', '00FF00', '0000FF', 'FFFF00', 'FF00FF', '00FFFF', '800000', '008000', '000080', '808000', '800080', '008080', 'C0C0C0', '808080', '9999FF', '993366', 'FFFFCC', 'CCFFFF', '660066', 'FF8080', '0066CC', 'CCCCFF', '000080', 'FF00FF', 'FFFF00', '00FFFF', '800080', '800000', '008080', '0000FF', '00CCFF', 'CCFFFF', 'CCFFCC', 'FFFF99', '99CCFF', 'FF99CC', 'CC99FF', 'FFCC99', '3366FF', '33CCCC', '99CC00', 'FFCC00', 'FF9900', 'FF6600', '666699', '969696', '003366', '339966', '003300', '333300', '993300', '993366', '333399', '333333'];
const tint = (hex, t) => {
  if (!t) return hex;
  const c = [0, 2, 4].map((i) => parseInt(hex.substr(i, 2), 16));
  const o = c.map((v) => Math.round(t < 0 ? v * (1 + t) : v + (255 - v) * t));
  return o.map((v) => Math.max(0, Math.min(255, v)).toString(16).padStart(2, '0')).join('').toUpperCase();
};
const colorOf = (e, theme) => {
  if (!e) return null;
  let hex = null;
  if (e.getAttribute('rgb')) hex = e.getAttribute('rgb').slice(-6).toUpperCase();
  else if (e.getAttribute('theme') != null) hex = theme[+e.getAttribute('theme')] || null;
  else if (e.getAttribute('indexed') != null) { const i = +e.getAttribute('indexed'); if (i === 64 || i === 65) return null; hex = IDX[i] || null; }
  if (!hex || !/^[0-9A-F]{6}$/.test(hex)) return null;
  return '#' + tint(hex, parseFloat(e.getAttribute('tint') || '0')).toLowerCase();
};

async function read(bytes) {
  const zip = await JSZip.loadAsync(bytes);
  const s = async (p) => { const f = zip.file(p); return f ? f.async('string') : null; };
  const wb = xml(await s('xl/workbook.xml'));
  const rels = {};
  const rx = await s('xl/_rels/workbook.xml.rels');
  if (rx) for (const r of xml(rx).getElementsByTagName('Relationship')) rels[r.getAttribute('Id')] = r.getAttribute('Target');
  // Theme colours in Excel's order (light 1 and dark 1 swapped, as Excel does).
  let theme = [];
  const tx = await s('xl/theme/theme1.xml');
  if (tx) {
    const cs = xml(tx).getElementsByTagNameNS('*', 'clrScheme')[0];
    const pick = (n) => { const e = cs && kids(cs).find((c) => c.localName === n); const v = e && e.firstElementChild; if (!v) return null; const hex = ((v.localName === 'sysClr' ? v.getAttribute('lastClr') : v.getAttribute('val')) || '').toUpperCase(); return /^[0-9A-F]{6}$/.test(hex) ? hex : null; };
    theme = ['lt1', 'dk1', 'lt2', 'dk2', 'accent1', 'accent2', 'accent3', 'accent4', 'accent5', 'accent6', 'hlink', 'folHlink'].map(pick);
    if (theme[0] === null) theme[0] = 'FFFFFF'; if (theme[1] === null) theme[1] = '000000';
  }
  const st = xml((await s('xl/styles.xml')) || '<styleSheet/>');
  const root = st.documentElement;
  const numFmts = {}; for (const n of kids(kid(root, 'numFmts'), 'numFmt')) numFmts[n.getAttribute('numFmtId')] = n.getAttribute('formatCode');
  const fonts = kids(kid(root, 'fonts'), 'font').map((f, i) => {
    const o = {};
    const b = kid(f, 'b'); if (b && b.getAttribute('val') !== '0' && b.getAttribute('val') !== 'false') o.b = true;
    const it = kid(f, 'i'); if (it && it.getAttribute('val') !== '0' && it.getAttribute('val') !== 'false') o.i = true;
    const u = kid(f, 'u'); if (u && u.getAttribute('val') !== 'none') o.u = true;
    const k = kid(f, 'strike'); if (k && k.getAttribute('val') !== '0') o.s = true;
    const c = colorOf(kid(f, 'color'), theme); if (c && c !== '#000000') o.color = c;
    const sz = kid(f, 'sz'); if (sz) o.size = parseFloat(sz.getAttribute('val'));
    const nm = kid(f, 'name'); if (nm) o.font = nm.getAttribute('val');
    o._i = i;
    return o;
  });
  const base = fonts[0] || {};
  const fills = kids(kid(root, 'fills'), 'fill').map((f) => {
    const p = kid(f, 'patternFill'); if (!p || !p.getAttribute('patternType') || p.getAttribute('patternType') === 'none' || p.getAttribute('patternType') === 'gray125') return null;
    return colorOf(kid(p, 'fgColor'), theme) || colorOf(kid(p, 'bgColor'), theme);
  });
  const borders = kids(kid(root, 'borders'), 'border').map((b) => {
    const o = {};
    for (const [side, k] of [['top', 't'], ['right', 'r'], ['bottom', 'b'], ['left', 'l']]) {
      const e = kid(b, side); const sty = e && e.getAttribute('style');
      if (sty && sty !== 'none') o[k] = /thick|double/.test(sty) ? 'thick' : /medium/.test(sty) ? 'medium' : 'thin';
    }
    return Object.keys(o).length ? o : null;
  });
  const xfs = kids(kid(root, 'cellXfs'), 'xf').map((x) => {
    const f = {};
    const font = fonts[+x.getAttribute('fontId') || 0] || {};
    if (font.b) f.b = true; if (font.i) f.i = true; if (font.u) f.u = true; if (font.s) f.s = true; if (font.color) f.color = font.color;
    if (font.size && font.size !== (base.size || 11)) f.size = font.size;
    if (font.font && font.font !== base.font) f.font = font.font;
    const fill = fills[+x.getAttribute('fillId') || 0]; if (fill) f.bg = fill;
    const bd = borders[+x.getAttribute('borderId') || 0]; if (bd) f.bd = bd;
    const al = kid(x, 'alignment');
    if (al) {
      const h = al.getAttribute('horizontal'); if (h === 'left' || h === 'center' || h === 'right') f.align = h; else if (h === 'centerContinuous') f.align = 'center';
      const v = al.getAttribute('vertical'); if (v === 'center') f.valign = 'middle'; else if (v === 'top') f.valign = 'top';
      if (al.getAttribute('wrapText') === '1' || al.getAttribute('wrapText') === 'true') f.wrap = true;
    }
    const nid = x.getAttribute('numFmtId');
    if (nid && nid !== '0' && numFmts[nid]) f.nf = numFmts[nid];
    return f;
  });
  const out = { sheets: {} };
  for (const sh of wb.getElementsByTagNameNS('*', 'sheet')) {
    const name = sh.getAttribute('name');
    const rid = sh.getAttributeNS(RNS, 'id') || sh.getAttribute('r:id');
    const target = rels[rid]; if (!target) continue;
    const path = target.startsWith('/') ? norm(target) : norm('xl/' + target);
    const sx = await s(path); if (!sx) continue;
    const d = xml(sx);
    const fmt = {};
    for (const c of d.getElementsByTagNameNS('*', 'c')) {
      const si = c.getAttribute('s'); if (!si || si === '0') continue;
      const f = xfs[+si]; if (!f || !Object.keys(f).length) continue;
      const m = /^([A-Z]+)(\d+)$/.exec(c.getAttribute('r') || ''); if (!m) continue;
      const col = m[1].split('').reduce((a, ch) => a * 26 + ch.charCodeAt(0) - 64, 0) - 1;
      const cf = { ...f };
      delete cf.nf; // number formats come from SheetJS (it knows the built-in ones too)
      if (Object.keys(cf).length) fmt[(+m[2] - 1) + ',' + col] = cf;
    }
    let freeze = null;
    const pane = d.getElementsByTagNameNS('*', 'pane')[0];
    if (pane && /frozen/.test(pane.getAttribute('state') || '')) freeze = { r: Math.round(+(pane.getAttribute('ySplit') || 0)), c: Math.round(+(pane.getAttribute('xSplit') || 0)) };
    out.sheets[name] = { fmt, freeze };
  }
  return out;
}

const escA = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
const BUILTIN = { General: 0, '0': 1, '0.00': 2, '#,##0': 3, '#,##0.00': 4, '0%': 9, '0.00%': 10, '0.00E+00': 11, '@': 49, 'mm-dd-yy': 14, 'h:mm AM/PM': 18, 'h:mm': 20, 'h:mm:ss': 21 };
async function write(bytes, sheets) {
  const zip = await JSZip.loadAsync(bytes);
  // Every look in the workbook gets one entry in styles.xml.
  const fonts = ['<font><sz val="11"/><color theme="1"/><name val="Calibri"/><family val="2"/><scheme val="minor"/></font>'];
  const fills = ['<fill><patternFill patternType="none"/></fill>', '<fill><patternFill patternType="gray125"/></fill>'];
  const borders = ['<border><left/><right/><top/><bottom/><diagonal/></border>'];
  const numFmts = [];
  const xfs = ['<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>'];
  const fontIdx = new Map(), fillIdx = new Map(), borderIdx = new Map(), nfIdx = new Map(), xfIdx = new Map();
  const rgb = (c) => 'FF' + (H.hex(c) || '000000');
  const styleOf = (f) => {
    if (!f) return 0;
    const k = JSON.stringify(f); if (xfIdx.has(k)) return xfIdx.get(k);
    let fontId = 0, fillId = 0, borderId = 0, numFmtId = 0;
    if (f.b || f.i || f.u || f.s || f.color || f.size || f.font) {
      const fx = `<font>${f.b ? '<b/>' : ''}${f.i ? '<i/>' : ''}${f.s ? '<strike/>' : ''}${f.u ? '<u/>' : ''}<sz val="${f.size || 11}"/>${f.color ? `<color rgb="${rgb(f.color)}"/>` : '<color theme="1"/>'}<name val="${escA(f.font || 'Calibri')}"/><family val="2"/></font>`;
      if (!fontIdx.has(fx)) { fontIdx.set(fx, fonts.length); fonts.push(fx); }
      fontId = fontIdx.get(fx);
    }
    if (f.bg) {
      const fx = `<fill><patternFill patternType="solid"><fgColor rgb="${rgb(f.bg)}"/><bgColor indexed="64"/></patternFill></fill>`;
      if (!fillIdx.has(fx)) { fillIdx.set(fx, fills.length); fills.push(fx); }
      fillId = fillIdx.get(fx);
    }
    if (f.bd) {
      const side = (n, k) => f.bd[k] ? `<${n} style="${f.bd[k] === 'thick' ? 'thick' : f.bd[k] === 'medium' ? 'medium' : 'thin'}"><color indexed="64"/></${n}>` : `<${n}/>`;
      const bx = `<border>${side('left', 'l')}${side('right', 'r')}${side('top', 't')}${side('bottom', 'b')}<diagonal/></border>`;
      if (!borderIdx.has(bx)) { borderIdx.set(bx, borders.length); borders.push(bx); }
      borderId = borderIdx.get(bx);
    }
    if (f.nf) {
      if (BUILTIN[f.nf] != null) numFmtId = BUILTIN[f.nf];
      else { if (!nfIdx.has(f.nf)) { nfIdx.set(f.nf, 164 + numFmts.length); numFmts.push(`<numFmt numFmtId="${164 + numFmts.length}" formatCode="${escA(f.nf)}"/>`); } numFmtId = nfIdx.get(f.nf); }
    }
    const al = (f.align || f.valign || f.wrap) ? `<alignment${f.align ? ` horizontal="${f.align}"` : ''}${f.valign ? ` vertical="${f.valign === 'middle' ? 'center' : 'top'}"` : ''}${f.wrap ? ' wrapText="1"' : ''}/>` : '';
    const x = `<xf numFmtId="${numFmtId}" fontId="${fontId}" fillId="${fillId}" borderId="${borderId}" xfId="0"${numFmtId ? ' applyNumberFormat="1"' : ''}${fontId ? ' applyFont="1"' : ''}${fillId ? ' applyFill="1"' : ''}${borderId ? ' applyBorder="1"' : ''}${al ? ' applyAlignment="1">' + al + '</xf>' : '/>'}`;
    xfIdx.set(k, xfs.length); xfs.push(x);
    return xfIdx.get(k);
  };
  // Which file is which sheet (SheetJS writes them in order, but read the workbook to be sure).
  const wb = xml(await zip.file('xl/workbook.xml').async('string'));
  const rels = {}; for (const r of xml(await zip.file('xl/_rels/workbook.xml.rels').async('string')).getElementsByTagName('Relationship')) rels[r.getAttribute('Id')] = r.getAttribute('Target');
  const paths = {};
  for (const sh of wb.getElementsByTagNameNS('*', 'sheet')) { const t = rels[sh.getAttributeNS(RNS, 'id') || sh.getAttribute('r:id')]; if (t) paths[sh.getAttribute('name')] = t.startsWith('/') ? norm(t) : norm('xl/' + t); }
  for (const s of sheets) {
    const path = paths[s.name]; if (!path || !zip.file(path)) continue;
    const d = xml(await zip.file(path).async('string'));
    const root = d.documentElement;
    const data = kid(root, 'sheetData');
    const rows = new Map(); for (const r of kids(data, 'row')) rows.set(+r.getAttribute('r'), r);
    const cellEl = (row, ref) => kids(row, 'c').find((c) => c.getAttribute('r') === ref);
    const colOf = (ref) => ref.replace(/\d+/g, '').split('').reduce((a, ch) => a * 26 + ch.charCodeAt(0) - 64, 0) - 1;
    const name = (r, c) => { let x = '', n = c + 1; while (n > 0) { const m = (n - 1) % 26; x = String.fromCharCode(65 + m) + x; n = Math.floor((n - 1) / 26); } return x + (r + 1); };
    // Every cell SheetJS wrote: our style (formatting + number format).
    const done = new Set();
    for (const row of rows.values()) for (const c of kids(row, 'c')) {
      const ref = c.getAttribute('r'); const r = +ref.replace(/\D+/g, '') - 1, col = colOf(ref);
      const f = s.fmt[r + ',' + col];
      const idx = styleOf(f && Object.keys(f).length ? f : null);
      if (idx) c.setAttribute('s', String(idx)); else c.removeAttribute('s');
      done.add(r + ',' + col);
    }
    // Formatted empty cells (a coloured header with nothing in it yet, borders around a blank box).
    const extra = Object.keys(s.fmt).filter((k) => !done.has(k)).map((k) => k.split(',').map(Number)).sort((a, b) => a[0] - b[0] || a[1] - b[1]);
    for (const [r, col] of extra) {
      const f = s.fmt[r + ',' + col]; if (!f || !(f.bg || f.bd)) continue; // only what shows without a value
      let row = rows.get(r + 1);
      if (!row) {
        row = d.createElementNS(NS, 'row'); row.setAttribute('r', String(r + 1));
        const after = Array.from(rows.keys()).filter((x) => x > r + 1).sort((a, b) => a - b)[0];
        if (after) data.insertBefore(row, rows.get(after)); else data.appendChild(row);
        rows.set(r + 1, row);
      }
      const ref = name(r, col);
      if (cellEl(row, ref)) continue;
      const c = d.createElementNS(NS, 'c'); c.setAttribute('r', ref); c.setAttribute('s', String(styleOf(f)));
      const next = kids(row, 'c').find((x) => colOf(x.getAttribute('r')) > col);
      row.insertBefore(c, next || null);
      row.removeAttribute('spans');
    }
    // Frozen rows and columns.
    if (s.freeze && (s.freeze.r || s.freeze.c)) {
      let views = kid(root, 'sheetViews');
      if (!views) { views = d.createElementNS(NS, 'sheetViews'); const before = kid(root, 'sheetFormatPr') || kid(root, 'cols') || data; root.insertBefore(views, before); }
      let view = kid(views, 'sheetView'); if (!view) { view = d.createElementNS(NS, 'sheetView'); view.setAttribute('workbookViewId', '0'); views.appendChild(view); }
      for (const p of kids(view, 'pane')) p.remove();
      for (const p of kids(view, 'selection')) p.remove();
      const pane = d.createElementNS(NS, 'pane');
      if (s.freeze.c) pane.setAttribute('xSplit', String(s.freeze.c));
      if (s.freeze.r) pane.setAttribute('ySplit', String(s.freeze.r));
      pane.setAttribute('topLeftCell', name(s.freeze.r, s.freeze.c));
      pane.setAttribute('activePane', s.freeze.r && s.freeze.c ? 'bottomRight' : s.freeze.r ? 'bottomLeft' : 'topRight');
      pane.setAttribute('state', 'frozen');
      view.insertBefore(pane, view.firstChild);
    }
    zip.file(path, '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n' + new XMLSerializer().serializeToString(d.documentElement));
  }
  const styles = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="${NS}">${numFmts.length ? `<numFmts count="${numFmts.length}">${numFmts.join('')}</numFmts>` : ''}<fonts count="${fonts.length}">${fonts.join('')}</fonts><fills count="${fills.length}">${fills.join('')}</fills><borders count="${borders.length}">${borders.join('')}</borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="${xfs.length}">${xfs.join('')}</cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles><dxfs count="0"/><tableStyles count="0" defaultTableStyle="TableStyleMedium9" defaultPivotStyle="PivotStyleMedium4"/></styleSheet>`;
  zip.file('xl/styles.xml', styles);
  return zip.generateAsync({ type: 'uint8array', compression: 'DEFLATE', mimeType: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet' });
}

window.SheetsStyle = { read, write };
})();
