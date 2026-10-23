// Hearth Sheets: the spreadsheet. Formulas are worked out by HyperFormula (GPL-3.0); the grid is drawn on a
// canvas (fast with big sheets); formatting, sizes, merges, freeze panes and filters are Sheets' own (S.meta).
// Files: .xlsx/.xls/.ods through SheetJS (formats/xlsx-style.js adds .xlsx cell formatting both ways) and CSV here.
// Undo is Sheets' own: each change keeps what the cells (and their formatting) were before it.
'use strict';
(() => {
const { el, esc, $, $$ } = H;
const HW = 50, HH = 26, DW = 100, DH = 24;
const FONT = '"Segoe UI", "Noto Sans", Carlito, "Liberation Sans", Arial, sans-serif';
const S = {
  id: 'sheets', name: 'Hearth Sheets', untitled: 'Untitled spreadsheet', tagline: 'Budgets, lists and numbers',
  credits: 'HyperFormula (GPL-3.0), SheetJS Community Edition (Apache-2.0) and JSZip (MIT)',
  formats: [{ ext: 'xlsx', label: 'Excel workbook' }, { ext: 'ods', label: 'OpenDocument spreadsheet' }, { ext: 'csv', label: 'CSV (this sheet)' }, { ext: 'tsv', label: 'Tab-separated (this sheet)' }],
  openExts: ['xlsx', 'xlsm', 'xls', 'ods', 'csv', 'tsv'],
  canWrite: (ext) => ['xlsx', 'ods', 'csv', 'tsv'].includes(ext),
  lossy: (ext) => ({
    csv: 'CSV keeps only the values of this sheet: no formulas, formatting or other sheets.',
    tsv: 'Tab-separated text keeps only the values of this sheet: no formulas, formatting or other sheets.',
    ods: 'Sheets saves .ods with values, formulas, number formats, merged cells and column widths, but not fonts, colours or borders. Use .xlsx to keep those.',
  }[ext] || null),
  sheets: [], cur: 0, sel: { r: 0, c: 0, r2: 0, c2: 0 }, editing: null, zoomV: 100,
};
const colName = (c) => { let s = ''; c++; while (c > 0) { const m = (c - 1) % 26; s = String.fromCharCode(65 + m) + s; c = Math.floor((c - 1) / 26); } return s; };
const colIndex = (s) => { let n = 0; for (const ch of s.toUpperCase()) n = n * 26 + ch.charCodeAt(0) - 64; return n - 1; };
const addr = (r, c) => colName(c) + (r + 1);
const parseAddr = (a) => { const m = /^\$?([A-Za-z]{1,3})\$?(\d+)$/.exec(a.trim()); return m ? { r: +m[2] - 1, c: colIndex(m[1]) } : null; };
S.colName = colName; S.addr = addr;
const key = (r, c) => r + ',' + c;
const norm = (s) => ({ r: Math.min(s.r, s.r2), c: Math.min(s.c, s.c2), r2: Math.max(s.r, s.r2), c2: Math.max(s.c, s.c2) });
const newMeta = () => ({ colW: {}, rowH: {}, fmt: {}, merges: [], freeze: { r: 0, c: 0 }, filter: null, hiddenRows: {}, rows: 200, cols: 26 });

// ------------------------------------------------------------------ the formula engine
S.makeEngine = () => {
  const HF = window.HyperFormula;
  S.hf = HF.buildEmpty({
    licenseKey: 'gpl-v3', undoLimit: 0, maxRows: 1048576, maxColumns: 16384,
    dateFormats: ['MM/DD/YYYY', 'MM/DD/YY', 'YYYY-MM-DD', 'YYYY/MM/DD', 'DD.MM.YYYY'], timeFormats: ['hh:mm', 'hh:mm:ss.sss'],
    currencySymbol: ['$', '€', '£', '¥'], functionArgSeparator: ',', decimalSeparator: '.', thousandSeparator: '',
    nullDate: { year: 1899, month: 12, day: 30 }, smartRounding: true, useColumnIndex: false,
  });
  // Excel writes TRUE and FALSE without brackets.
  try { S.hf.addNamedExpression('TRUE', '=TRUE()'); S.hf.addNamedExpression('FALSE', '=FALSE()'); } catch {}
};
S.sh = () => S.sheets[S.cur];
S.sid = () => S.sh().id;
S.meta = () => S.sh().meta;
S.val = (r, c, sid = S.sid()) => S.hf.getCellValue({ sheet: sid, row: r, col: c });
S.ser = (r, c, sid = S.sid()) => S.hf.getCellSerialized({ sheet: sid, row: r, col: c });
S.fmtAt = (r, c, m = S.meta()) => m.fmt[key(r, c)] || null;
S.used = (sid = S.sid()) => { const d = S.hf.getSheetDimensions(sid); return { rows: d.height, cols: d.width }; };

// What a cell shows: its value through its number format.
const ERRS = { DIV_BY_ZERO: '#DIV/0!', NA: '#N/A', NAME: '#NAME?', NUM: '#NUM!', REF: '#REF!', VALUE: '#VALUE!', CYCLE: '#CYCLE!', ERROR: '#ERROR!', SPILL: '#SPILL!', LIC: '#LIC!' };
S.display = (v, f) => {
  if (v === null || v === undefined || v === '') return '';
  if (typeof v === 'object' && v.type) return v.value || ERRS[v.type] || '#ERROR!';
  if (typeof v === 'boolean') return v ? 'TRUE' : 'FALSE';
  if (typeof v === 'number') {
    const nf = f && f.nf;
    if (nf && nf !== 'General' && window.XLSX) { try { return XLSX.SSF.format(nf, v); } catch { /* bad format: as a number */ } }
    if (Number.isInteger(v) && Math.abs(v) < 1e15) return String(v);
    const a = Math.abs(v);
    if (a !== 0 && (a < 1e-9 || a >= 1e15)) return v.toExponential(5).replace(/\.?0+e/, 'e');
    return String(Math.round(v * 1e10) / 1e10);
  }
  return String(v);
};
S.text = (r, c, sid) => { const sh = sid == null ? S.sh() : S.sheets.find((s) => s.id === sid); return S.display(S.val(r, c, sh.id), S.fmtAt(r, c, sh.meta)); };

// ------------------------------------------------------------------ undo
// Each step keeps what changed: cell contents (serialized) for a range or every sheet, and formatting (meta).
const U = { stack: [], redo: [] };
const cloneMeta = (m) => JSON.parse(JSON.stringify(m));
const grab = (sid, r, c, r2, c2) => { const out = []; for (let i = r; i <= r2; i++) { const row = []; for (let j = c; j <= c2; j++) row.push(S.ser(i, j, sid) ?? null); out.push(row); } return out; };
// Runs a change that touches cells in `range` (or the whole workbook when range is 'all') and maybe the meta.
S.change = (range, fn, opts = {}) => {
  const sid = S.sid(), sh = S.sh();
  let before;
  if (range === 'all') before = { all: S.hf.getAllSheetsSerialized(), metas: S.sheets.map((s) => ({ id: s.id, name: s.name, meta: cloneMeta(s.meta) })) };
  else if (range) { const n = norm(range); before = { sid, n, cells: grab(sid, n.r, n.c, n.r2, n.c2) }; }
  const metaBefore = opts.meta || range === 'all' ? cloneMeta(sh.meta) : null;
  if (opts.suspend) S.hf.suspendEvaluation();
  try { fn(); } finally { if (opts.suspend) S.hf.resumeEvaluation(); }
  let after;
  if (range === 'all') after = { all: S.hf.getAllSheetsSerialized(), metas: S.sheets.map((s) => ({ id: s.id, name: s.name, meta: cloneMeta(s.meta) })) };
  else if (range) after = { sid, n: before.n, cells: grab(sid, before.n.r, before.n.c, before.n.r2, before.n.c2) };
  U.stack.push({ before, after, sheet: sh, metaBefore, metaAfter: metaBefore ? cloneMeta(sh.meta) : null, sel: { ...S.sel }, all: range === 'all' });
  if (U.stack.length > 200) U.stack.shift();
  U.redo = [];
  S.dirty();
};
const putCells = (st) => { S.hf.setCellContents({ sheet: st.sid, row: st.n.r, col: st.n.c }, st.cells); };
const putAll = (st) => {
  // Back to every sheet as it was (used for row/column inserts, sorts, sheet changes).
  const names = Object.keys(st.all);
  for (const s of S.sheets.slice()) if (!names.includes(s.name)) { try { S.hf.removeSheet(s.id); } catch {} }
  const list = [];
  for (const m of st.metas) {
    let id = S.hf.getSheetId(m.name);
    if (id === undefined) { S.hf.addSheet(m.name); id = S.hf.getSheetId(m.name); }
    S.hf.setSheetContent(id, st.all[m.name] || []);
    list.push({ id, name: m.name, meta: cloneMeta(m.meta) });
  }
  S.sheets = list;
};
const apply = (e, dir) => {
  const st = dir === 'undo' ? e.before : e.after;
  try { S.hf.suspendEvaluation(); if (e.all) putAll(st); else if (st) putCells(st); } finally { S.hf.resumeEvaluation(); }
  const m = dir === 'undo' ? e.metaBefore : e.metaAfter;
  if (m && !e.all) e.sheet.meta = cloneMeta(m);
  const i = S.sheets.findIndex((s) => s.id === e.sheet.id || s.name === e.sheet.name);
  if (i >= 0) S.cur = i; else S.cur = Math.min(S.cur, S.sheets.length - 1);
  S.sel = { ...e.sel };
  S.tabs(); S.dirty();
};
S.undo = () => { const e = U.stack.pop(); if (!e) return H.toast('Nothing to undo'); apply(e, 'undo'); U.redo.push(e); };
S.redo = () => { const e = U.redo.pop(); if (!e) return H.toast('Nothing to redo'); apply(e, 'redo'); U.stack.push(e); };
S.metaChange = (fn) => S.change(null, fn, { meta: true });
S.dirty = () => { H.changed(); S.layout(); S.draw(); S.updateBar(); };

// ------------------------------------------------------------------ the window
S.mount = async (main, toolbar, status) => {
  await H.need('vendor/hyperformula.full.min.js', 'vendor/xlsx.full.min.js');
  S.makeEngine();
  main.innerHTML = `
    <div class="sheet-wrap">
      <div class="fxbar"><input id="nameBox" class="app-key" spellcheck="false" title="Cell or range: type one to go there"><span class="fxlab">fx</span><input id="fx" class="app-key" spellcheck="false" placeholder="Value or formula (start with =)"></div>
      <div id="sgrid" class="sgrid" tabindex="-1">
        <canvas id="scv"></canvas>
        <div id="sscroll"><div id="sspace"></div></div>
        <textarea id="cellEd" class="app-key" spellcheck="false" hidden></textarea>
        <div id="acList" class="aclist" hidden></div>
        <textarea id="kb" class="app-key" aria-label="Spreadsheet"></textarea>
      </div>
      <div class="tabbar"><button class="ibtn f" id="addSheet" title="Add a sheet"><svg viewBox="0 0 24 24"><path d="M12 5v14M5 12h14"/></svg></button><div id="tabs"></div></div>
    </div>`;
  S.cv = $('#scv'); S.ctx = S.cv.getContext('2d'); S.scroll = $('#sscroll'); S.space = $('#sspace'); S.kb = $('#kb'); S.ed = $('#cellEd'); S.fx = $('#fx');
  buildToolbar(toolbar);
  status.querySelector('#statusLeft').innerHTML = '<span id="sSel"></span>';
  status.querySelector('#statusRight').innerHTML = '<span id="sCalc"></span><button id="zOut" title="Zoom out">−</button><button id="zVal">100%</button><button id="zIn" title="Zoom in">+</button>';
  $('#zOut').onclick = () => S.zoom(S.zoomV - 10); $('#zIn').onclick = () => S.zoom(S.zoomV + 10); $('#zVal').onclick = () => S.zoom(100);
  $('#addSheet').onclick = () => S.addSheet();
  wire();
  new ResizeObserver(() => { S.layout(); S.draw(); }).observe($('#sgrid'));
};
S.scroller = () => S.scroll;
S.focus = () => { if (!S.editing) S.kb.focus({ preventScroll: true }); };

function buildToolbar(tb) {
  const I = H.ICON;
  const fonts = Array.from(new Set(['Calibri', 'Arial', 'Liberation Sans', 'Carlito', 'Times New Roman', 'Courier New', 'Georgia', 'Verdana', 'Noto Sans', ...H.fonts])).sort();
  S.fontSel = H.tsel([['', 'Default font'], ...fonts.map((f) => [f, f])], 'Font', (v) => { S.setFmt({ font: v || null }); S.focus(); }, { cls: 'font' });
  S.sizeSel = H.tsel([8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 36].map((s) => [s, s]), 'Font size', (v) => { S.setFmt({ size: +v }); S.focus(); }, { cls: 'size' });
  S.nfSel = H.tsel([['General', 'General'], ['#,##0.00', 'Number'], ['"$"#,##0.00', 'Currency'], ['0.00%', 'Percent'], ['0.00E+00', 'Scientific'], ['yyyy-mm-dd', 'Date'], ['dddd, mmmm d, yyyy', 'Long date'], ['h:mm AM/PM', 'Time'], ['yyyy-mm-dd h:mm', 'Date and time'], ['@', 'Text'], ['custom', 'Custom…']], 'Number format', (v) => { if (v === 'custom') S.customFormat(); else S.setFmt({ nf: v === 'General' ? null : v }); S.focus(); }, { cls: 'style' });
  tb.append(
    H.tbtn(I.undo, 'Undo (Ctrl+Z)', () => S.undo()), H.tbtn(I.redo, 'Redo (Ctrl+Y)', () => S.redo()), H.tsep(),
    S.nfSel, H.tbtn(I.currency, 'Currency', () => S.setFmt({ nf: '"$"#,##0.00' })), H.tbtn(I.percent, 'Percent', () => S.setFmt({ nf: '0%' })),
    H.tbtn(I.decDown, 'Fewer decimals', () => S.decimals(-1)), H.tbtn(I.decUp, 'More decimals', () => S.decimals(1)), H.tsep(),
    S.fontSel, S.sizeSel, H.tsep(),
    H.tbtn(I.bold, 'Bold (Ctrl+B)', () => S.toggle('b'), { cmd: 'b' }), H.tbtn(I.italic, 'Italic (Ctrl+I)', () => S.toggle('i'), { cmd: 'i' }),
    H.tbtn(I.underline, 'Underline (Ctrl+U)', () => S.toggle('u'), { cmd: 'u' }), H.tbtn(I.strike, 'Strikethrough', () => S.toggle('s'), { cmd: 's' }),
    H.colorBtn(I.color, 'Text colour', '#c00000', (c) => S.setFmt({ color: c })), H.colorBtn(I.fill, 'Fill colour', '#ffff00', (c) => S.setFmt({ bg: c }), { none: 'No fill' }), H.tsep(),
    H.tbtn(I.borders, 'Borders', (e, b) => { const r = b.getBoundingClientRect(); H.showMenu(borderItems(), r.left, r.bottom + 4); }),
    H.tbtn(I.left, 'Align left', () => S.setFmt({ align: 'left' }), { cmd: 'al-left' }), H.tbtn(I.center, 'Centre', () => S.setFmt({ align: 'center' }), { cmd: 'al-center' }), H.tbtn(I.right, 'Align right', () => S.setFmt({ align: 'right' }), { cmd: 'al-right' }),
    H.tbtn(I.wrap, 'Wrap text', () => S.toggle('wrap'), { cmd: 'wrap' }), H.tbtn(I.merge, 'Merge cells', () => S.merge()), H.tsep(),
    H.tbtn(I.sum, 'AutoSum (Alt+=)', () => S.autoSum()), H.tbtn(I.sortAZ, 'Sort', (e, b) => { const r = b.getBoundingClientRect(); H.showMenu(sortItems(), r.left, r.bottom + 4); }),
    H.tbtn(I.filter, 'Filter (Ctrl+Shift+L)', () => S.toggleFilter(), { cmd: 'filter' }), H.tbtn(I.freeze, 'Freeze', (e, b) => { const r = b.getBoundingClientRect(); H.showMenu(freezeItems(), r.left, r.bottom + 4); }),
    H.tbtn(I.chart, 'Chart', () => S.chartDialog()),
  );
}
const borderItems = () => [
  ['all', 'All borders'], ['outer', 'Outside borders'], ['thick', 'Thick outside border'], ['bottom', 'Bottom border'], ['top', 'Top border'], ['left', 'Left border'], ['right', 'Right border'], ['none', 'No borders'],
].map(([k, l]) => ({ label: l, run: () => S.borders(k) }));
const sortItems = () => [{ label: 'Sort A → Z (by this column)', run: () => S.sortQuick(1) }, { label: 'Sort Z → A (by this column)', run: () => S.sortQuick(-1) }, { label: 'Sort range…', run: () => S.sortDialog() }];
const freezeItems = () => [
  { label: 'No frozen rows or columns', run: () => S.freeze(0, 0) }, { label: 'Freeze top row', run: () => S.freeze(1, S.meta().freeze.c) }, { label: 'Freeze first column', run: () => S.freeze(S.meta().freeze.r, 1) },
  { label: 'Freeze up to the selected cell', run: () => S.freeze(S.sel.r, S.sel.c) },
];

// ------------------------------------------------------------------ geometry
S.layout = () => {
  const m = S.meta(); if (!m) return;
  const u = S.used();
  m.rows = Math.max(m.rows || 200, u.rows + 60, S.sel.r2 + 30, S.sel.r + 30);
  m.cols = Math.max(m.cols || 26, u.cols + 6, S.sel.c2 + 4, S.sel.c + 4);
  const z = S.zoomV / 100;
  const xs = [0]; for (let c = 0; c < m.cols; c++) xs.push(xs[c] + Math.round((m.colW[c] ?? DW) * z));
  const ys = [0]; for (let r = 0; r < m.rows; r++) ys.push(ys[r] + (m.hiddenRows[r] ? 0 : Math.round((m.rowH[r] ?? DH) * z)));
  S.xs = xs; S.ys = ys; S.hw = Math.round(HW * z); S.hh = Math.round(HH * z);
  S.space.style.width = (S.hw + xs[m.cols] + 200) + 'px';
  S.space.style.height = (S.hh + ys[m.rows] + 200) + 'px';
};
const bs = (arr, v) => { let lo = 0, hi = arr.length - 2; while (lo < hi) { const mid = (lo + hi + 1) >> 1; if (arr[mid] <= v) lo = mid; else hi = mid - 1; } return lo; };
// Where a cell is on the canvas (with frozen rows/columns staying put).
S.cellRect = (r, c) => {
  const m = S.meta(), sl = S.scroll.scrollLeft, st = S.scroll.scrollTop;
  const fc = m.freeze.c, fr = m.freeze.r;
  const x = S.hw + S.xs[c] - (c >= fc ? sl : 0), y = S.hh + S.ys[r] - (r >= fr ? st : 0);
  return { x, y, w: S.xs[c + 1] - S.xs[c], h: S.ys[r + 1] - S.ys[r] };
};
S.mergeAt = (r, c, m = S.meta()) => m.merges.find((g) => r >= g.r && r <= g.r2 && c >= g.c && c <= g.c2) || null;
S.rangeRect = (n) => {
  const a = S.cellRect(n.r, n.c), b = S.cellRect(n.r2, n.c2);
  return { x: a.x, y: a.y, w: b.x + b.w - a.x, h: b.y + b.h - a.y };
};
// The cell under a point on the canvas.
S.hit = (x, y) => {
  const m = S.meta(), sl = S.scroll.scrollLeft, st = S.scroll.scrollTop;
  const fxw = S.xs[m.freeze.c], fyh = S.ys[m.freeze.r];
  const gx = x - S.hw, gy = y - S.hh;
  const cx = gx < fxw ? gx : gx + sl, cy = gy < fyh ? gy : gy + st;
  return { r: Math.max(0, Math.min(m.rows - 1, bs(S.ys, Math.max(0, cy)))), c: Math.max(0, Math.min(m.cols - 1, bs(S.xs, Math.max(0, cx)))), hx: gx < 0, hy: gy < 0 };
};

// ------------------------------------------------------------------ drawing
let drawQueued = false;
S.draw = () => { if (drawQueued) return; drawQueued = true; requestAnimationFrame(() => { drawQueued = false; S.paint(); }); };
S.paint = () => {
  const m = S.meta(); if (!m || !S.xs) return;
  const g = $('#sgrid'); const W = g.clientWidth, Hh = g.clientHeight;
  const dpr = window.devicePixelRatio || 1;
  if (S.cv.width !== Math.round(W * dpr) || S.cv.height !== Math.round(Hh * dpr)) { S.cv.width = Math.round(W * dpr); S.cv.height = Math.round(Hh * dpr); S.cv.style.width = W + 'px'; S.cv.style.height = Hh + 'px'; }
  const ctx = S.ctx; ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  const light = !document.documentElement.classList.contains('light') ? false : true;
  const C = { bg: '#ffffff', grid: '#e2e2e2', head: light ? '#f3eee9' : '#211915', headText: light ? '#6b5a4c' : '#a69484', headSel: light ? '#ffd9bf' : '#4a2c18', headLine: light ? '#ddd2c8' : '#3b2e26', sel: 'rgba(255,138,61,0.14)', selLine: '#ff8a3d' };
  ctx.fillStyle = C.bg; ctx.fillRect(0, 0, W, Hh);
  const sl = S.scroll.scrollLeft, st = S.scroll.scrollTop, fr = m.freeze.r, fc = m.freeze.c;
  const fxw = S.xs[fc], fyh = S.ys[fr];
  // Visible rows and columns in the scrolling part.
  const c0 = bs(S.xs, fxw + sl), c1 = Math.min(m.cols - 1, bs(S.xs, sl + W - S.hw));
  const r0 = bs(S.ys, fyh + st), r1 = Math.min(m.rows - 1, bs(S.ys, st + Hh - S.hh));
  const colsVis = []; for (let c = 0; c < fc; c++) colsVis.push(c); for (let c = Math.max(fc, c0); c <= c1; c++) colsVis.push(c);
  const rowsVis = []; for (let r = 0; r < fr; r++) if (!m.hiddenRows[r]) rowsVis.push(r); for (let r = Math.max(fr, r0); r <= r1; r++) if (!m.hiddenRows[r]) rowsVis.push(r);
  const z = S.zoomV / 100;
  // Four areas: frozen corner, frozen rows, frozen columns, the rest. Each clipped to its own rectangle.
  const areas = [
    { rs: rowsVis.filter((r) => r >= fr), cs: colsVis.filter((c) => c >= fc), x: S.hw + fxw, y: S.hh + fyh },
    { rs: rowsVis.filter((r) => r < fr), cs: colsVis.filter((c) => c >= fc), x: S.hw + fxw, y: S.hh },
    { rs: rowsVis.filter((r) => r >= fr), cs: colsVis.filter((c) => c < fc), x: S.hw, y: S.hh + fyh },
    { rs: rowsVis.filter((r) => r < fr), cs: colsVis.filter((c) => c < fc), x: S.hw, y: S.hh },
  ];
  const sid = S.sid();
  for (const a of areas) {
    if (!a.rs.length || !a.cs.length) continue;
    ctx.save(); ctx.beginPath(); ctx.rect(a.x, a.y, W - a.x, Hh - a.y); ctx.clip();
    // Gridlines.
    ctx.strokeStyle = C.grid; ctx.lineWidth = 1; ctx.beginPath();
    for (const c of a.cs) { const R = S.cellRect(0, c); const x = Math.round(R.x + R.w) - 0.5; ctx.moveTo(x, a.y); ctx.lineTo(x, Hh); }
    for (const r of a.rs) { const R = S.cellRect(r, 0); const y = Math.round(R.y + R.h) - 0.5; ctx.moveTo(a.x, y); ctx.lineTo(W, y); }
    ctx.stroke();
    // Fills, then text, then borders.
    const merged = new Set();
    const cellsToText = [];
    for (const r of a.rs) for (const c of a.cs) {
      const mg = S.mergeAt(r, c, m);
      if (mg) { const k = key(mg.r, mg.c); if (merged.has(k)) continue; merged.add(k); const R = S.rangeRect(mg); const f = S.fmtAt(mg.r, mg.c, m); ctx.fillStyle = (f && f.bg) || C.bg; ctx.fillRect(R.x, R.y, R.w - 1, R.h - 1); cellsToText.push([mg.r, mg.c, R, f]); continue; }
      const f = S.fmtAt(r, c, m); const R = S.cellRect(r, c);
      if (f && f.bg) { ctx.fillStyle = f.bg; ctx.fillRect(R.x, R.y, R.w - 1, R.h - 1); }
      cellsToText.push([r, c, R, f]);
    }
    for (const [r, c, R, f] of cellsToText) {
      const v = S.val(r, c, sid);
      if (v === null || v === '') continue;
      const t = S.display(v, f);
      const isNum = typeof v === 'number', isErr = typeof v === 'object';
      const size = ((f && f.size) || 11) * 1.333 * z;
      ctx.font = `${f && f.i ? 'italic ' : ''}${f && f.b ? '700 ' : ''}${size.toFixed(1)}px ${f && f.font ? `"${f.font}", ` : ''}${FONT}`;
      ctx.fillStyle = (f && f.color) || (isErr ? '#c00000' : '#1b1b1b');
      let align = (f && f.align) || (isNum ? 'right' : typeof v === 'boolean' || isErr ? 'center' : 'left');
      const pad = 4 * z;
      ctx.save();
      // Left-aligned text runs over empty cells to its right.
      let w = R.w;
      if (align === 'left' && !(f && f.wrap) && !isNum) { const tw = ctx.measureText(t).width + pad * 2; let cc = c + 1; while (w < tw && cc < m.cols && (S.val(r, cc, sid) === null || S.val(r, cc, sid) === '') && !S.mergeAt(r, cc, m)) { w += S.xs[cc + 1] - S.xs[cc]; cc++; } }
      ctx.beginPath(); ctx.rect(R.x, R.y, w - 1, R.h - 1); ctx.clip();
      ctx.textBaseline = 'middle';
      if (f && f.wrap) {
        const lines = wrapText(ctx, t, R.w - pad * 2); const lh = size * 1.2;
        let y = R.y + (f.valign === 'top' ? pad + lh / 2 : f.valign === 'middle' ? R.h / 2 - (lines.length - 1) * lh / 2 : R.h - pad - (lines.length - 1) * lh - lh / 2 + 2);
        for (const ln of lines) { drawLine(ctx, ln, R, align, pad, y, f); y += lh; }
      } else {
        const y = f && f.valign === 'top' ? R.y + size * 0.7 + pad / 2 : f && f.valign === 'middle' ? R.y + R.h / 2 : R.y + R.h - size * 0.62 - 1;
        let shown = t;
        if (isNum && ctx.measureText(t).width > R.w - pad) shown = '#'.repeat(Math.max(1, Math.floor((R.w - pad) / ctx.measureText('#').width)));
        drawLine(ctx, shown, { ...R, w }, align, pad, Math.min(y, R.y + R.h - size * 0.5), f);
      }
      ctx.restore();
    }
    // Borders.
    for (const r of a.rs) for (const c of a.cs) {
      const f = S.fmtAt(r, c, m); if (!f || !f.bd) continue;
      const mg = S.mergeAt(r, c, m); const R = mg ? S.rangeRect(mg) : S.cellRect(r, c);
      ctx.strokeStyle = '#000';
      for (const side of ['t', 'r', 'b', 'l']) {
        const s = f.bd[side]; if (!s) continue;
        ctx.lineWidth = s === 'thick' ? 2.5 : s === 'medium' ? 1.8 : 1;
        ctx.beginPath();
        const o = ctx.lineWidth % 2 ? 0.5 : 0;
        if (side === 't') { ctx.moveTo(R.x - 1, Math.round(R.y - 1) + o); ctx.lineTo(R.x + R.w, Math.round(R.y - 1) + o); }
        if (side === 'b') { ctx.moveTo(R.x - 1, Math.round(R.y + R.h - 1) + o); ctx.lineTo(R.x + R.w, Math.round(R.y + R.h - 1) + o); }
        if (side === 'l') { ctx.moveTo(Math.round(R.x - 1) + o, R.y - 1); ctx.lineTo(Math.round(R.x - 1) + o, R.y + R.h); }
        if (side === 'r') { ctx.moveTo(Math.round(R.x + R.w - 1) + o, R.y - 1); ctx.lineTo(Math.round(R.x + R.w - 1) + o, R.y + R.h); }
        ctx.stroke();
      }
    }
    // Filter buttons.
    if (m.filter) {
      const fl = m.filter;
      if (a.rs.includes(fl.r)) for (const c of a.cs) if (c >= fl.c && c <= fl.c2) {
        const R = S.cellRect(fl.r, c); const s = Math.min(18 * z, R.h - 4);
        const bx = R.x + R.w - s - 3, by = R.y + (R.h - s) / 2;
        const active = fl.crit && fl.crit[c] && fl.crit[c].length;
        ctx.fillStyle = active ? '#ff8a3d' : '#f1ece7'; ctx.strokeStyle = '#b9a99b'; ctx.lineWidth = 1;
        ctx.fillRect(bx, by, s, s); ctx.strokeRect(bx + 0.5, by + 0.5, s - 1, s - 1);
        ctx.fillStyle = active ? '#fff' : '#5a4a3d'; ctx.beginPath(); ctx.moveTo(bx + s * 0.28, by + s * 0.4); ctx.lineTo(bx + s * 0.72, by + s * 0.4); ctx.lineTo(bx + s * 0.5, by + s * 0.66); ctx.fill();
      }
    }
    ctx.restore();
  }
  // Frozen dividers.
  ctx.strokeStyle = '#b8a898'; ctx.lineWidth = 2;
  if (fr) { ctx.beginPath(); ctx.moveTo(0, S.hh + fyh); ctx.lineTo(W, S.hh + fyh); ctx.stroke(); }
  if (fc) { ctx.beginPath(); ctx.moveTo(S.hw + fxw, 0); ctx.lineTo(S.hw + fxw, Hh); ctx.stroke(); }
  // Selection.
  const n = norm(S.sel);
  ctx.save(); ctx.beginPath(); ctx.rect(S.hw, S.hh, W, Hh); ctx.clip();
  const sr = S.rangeRect(n);
  const single = n.r === n.r2 && n.c === n.c2;
  if (!single) { ctx.fillStyle = C.sel; ctx.fillRect(sr.x, sr.y, sr.w, sr.h); }
  if (S.clip && S.clip.sid === S.sid()) { const cr = S.rangeRect(S.clip.n); ctx.setLineDash([5, 3]); ctx.strokeStyle = '#ff8a3d'; ctx.lineWidth = 2; ctx.strokeRect(cr.x, cr.y, cr.w - 1, cr.h - 1); ctx.setLineDash([]); }
  ctx.strokeStyle = C.selLine; ctx.lineWidth = 2;
  ctx.strokeRect(sr.x - 0.5, sr.y - 0.5, sr.w, sr.h);
  if (S.fillTo) { const fr2 = S.rangeRect(norm(S.fillTo)); ctx.setLineDash([4, 3]); ctx.strokeStyle = '#7a6a5d'; ctx.lineWidth = 1.5; ctx.strokeRect(fr2.x - 0.5, fr2.y - 0.5, fr2.w, fr2.h); ctx.setLineDash([]); }
  // The fill handle.
  ctx.fillStyle = C.selLine; ctx.fillRect(sr.x + sr.w - 4, sr.y + sr.h - 4, 7, 7);
  ctx.strokeStyle = '#fff'; ctx.lineWidth = 1; ctx.strokeRect(sr.x + sr.w - 4.5, sr.y + sr.h - 4.5, 8, 8);
  S.handle = { x: sr.x + sr.w, y: sr.y + sr.h };
  // The point-mode reference while typing a formula.
  if (S.point) { const pr = S.rangeRect(norm(S.point)); ctx.setLineDash([4, 2]); ctx.strokeStyle = '#2f7bff'; ctx.lineWidth = 2; ctx.strokeRect(pr.x, pr.y, pr.w - 1, pr.h - 1); ctx.setLineDash([]); }
  ctx.restore();
  // Headers.
  ctx.font = `${(12 * z).toFixed(1)}px ${FONT}`; ctx.textBaseline = 'middle'; ctx.textAlign = 'center';
  ctx.fillStyle = C.head; ctx.fillRect(0, 0, W, S.hh); ctx.fillRect(0, 0, S.hw, Hh);
  const headArea = (cs, x0) => {
    ctx.save(); ctx.beginPath(); ctx.rect(x0, 0, W - x0, S.hh); ctx.clip();
    for (const c of cs) {
      const R = S.cellRect(0, c);
      if (c >= n.c && c <= n.c2) { ctx.fillStyle = C.headSel; ctx.fillRect(R.x, 0, R.w, S.hh); }
      ctx.fillStyle = C.headText; ctx.fillText(colName(c), R.x + R.w / 2, S.hh / 2 + 1);
      ctx.fillStyle = C.headLine; ctx.fillRect(Math.round(R.x + R.w) - 1, 4, 1, S.hh - 8);
    }
    ctx.restore();
  };
  headArea(colsVis.filter((c) => c >= fc), S.hw + fxw); headArea(colsVis.filter((c) => c < fc), S.hw);
  const rowArea = (rs, y0) => {
    ctx.save(); ctx.beginPath(); ctx.rect(0, y0, S.hw, Hh - y0); ctx.clip();
    for (const r of rs) {
      const R = S.cellRect(r, 0);
      if (r >= n.r && r <= n.r2) { ctx.fillStyle = C.headSel; ctx.fillRect(0, R.y, S.hw, R.h); }
      ctx.fillStyle = C.headText; ctx.fillText(String(r + 1), S.hw / 2, R.y + R.h / 2 + 1);
      ctx.fillStyle = C.headLine; ctx.fillRect(4, Math.round(R.y + R.h) - 1, S.hw - 8, 1);
    }
    ctx.restore();
  };
  rowArea(rowsVis.filter((r) => r >= fr), S.hh + fyh); rowArea(rowsVis.filter((r) => r < fr), S.hh);
  ctx.fillStyle = C.head; ctx.fillRect(0, 0, S.hw, S.hh);
  ctx.fillStyle = C.headLine; ctx.fillRect(0, S.hh - 1, W, 1); ctx.fillRect(S.hw - 1, 0, 1, Hh);
  ctx.textAlign = 'left';
  if (S.editing) S.placeEditor();
};
function drawLine(ctx, t, R, align, pad, y, f) {
  const tw = ctx.measureText(t).width;
  const x = align === 'right' ? R.x + R.w - pad - tw : align === 'center' ? R.x + (R.w - tw) / 2 : R.x + pad;
  ctx.fillText(t, x, y);
  if (f && (f.u || f.s)) { ctx.fillRect(x, f.u ? y + 7 : y, tw, 1); }
}
function wrapText(ctx, t, w) {
  const out = [];
  for (const para of String(t).split('\n')) {
    let line = '';
    for (const word of para.split(/(\s+)/)) {
      const test = line + word;
      if (ctx.measureText(test).width > w && line.trim()) { out.push(line.trimEnd()); line = word.trimStart(); }
      else line = test;
    }
    out.push(line);
  }
  return out;
}

// ------------------------------------------------------------------ selection, keyboard, mouse
S.select = (r, c, r2 = r, c2 = c, keepAnchor) => {
  const m = S.meta();
  r = Math.max(0, r); c = Math.max(0, c); r2 = Math.max(0, r2); c2 = Math.max(0, c2);
  S.sel = keepAnchor ? { ...S.sel, r2, c2 } : { r, c, r2, c2 };
  // A merged cell selects whole.
  const n = norm(S.sel);
  for (const g of m.merges) if (!(g.r > n.r2 || g.r2 < n.r || g.c > n.c2 || g.c2 < n.c)) { if (S.sel.r2 >= S.sel.r) S.sel.r2 = Math.max(S.sel.r2, g.r2); else S.sel.r2 = Math.min(S.sel.r2, g.r); if (S.sel.c2 >= S.sel.c) S.sel.c2 = Math.max(S.sel.c2, g.c2); else S.sel.c2 = Math.min(S.sel.c2, g.c); }
  if (S.sel.r2 + 20 > m.rows || S.sel.c2 + 4 > m.cols) S.layout();
  S.ensureVisible(S.sel.r2, S.sel.c2);
  S.updateBar(); S.draw();
};
S.ensureVisible = (r, c) => {
  const m = S.meta(), g = $('#sgrid');
  if (!S.xs[c + 1] && S.xs[c + 1] !== 0) S.layout();
  const fxw = S.xs[m.freeze.c], fyh = S.ys[m.freeze.r];
  const vw = g.clientWidth - S.hw - fxw, vh = g.clientHeight - S.hh - fyh;
  if (c >= m.freeze.c) { const x0 = S.xs[c] - fxw, x1 = S.xs[c + 1] - fxw; if (x0 < S.scroll.scrollLeft) S.scroll.scrollLeft = x0; else if (x1 > S.scroll.scrollLeft + vw - 14) S.scroll.scrollLeft = x1 - vw + 14; }
  if (r >= m.freeze.r) { const y0 = S.ys[r] - fyh, y1 = S.ys[r + 1] - fyh; if (y0 < S.scroll.scrollTop) S.scroll.scrollTop = y0; else if (y1 > S.scroll.scrollTop + vh - 14) S.scroll.scrollTop = y1 - vh + 14; }
};
// Ctrl+arrow: to the edge of the data.
const jump = (r, c, dr, dc) => {
  const m = S.meta(); const has = (rr, cc) => { const v = S.val(rr, cc); return v !== null && v !== ''; };
  const inb = (rr, cc) => rr >= 0 && cc >= 0 && rr < Math.max(m.rows, 1) && cc < Math.max(m.cols, 1);
  let rr = r + dr, cc = c + dc;
  if (!inb(rr, cc)) return { r, c };
  if (has(r, c) && has(rr, cc)) { while (inb(rr + dr, cc + dc) && has(rr + dr, cc + dc)) { rr += dr; cc += dc; } return { r: rr, c: cc }; }
  while (inb(rr, cc) && !has(rr, cc)) { if (!inb(rr + dr, cc + dc)) return { r: rr, c: cc }; rr += dr; cc += dc; }
  return { r: rr, c: cc };
};
S.updateBar = () => {
  if (!S.meta()) return;
  const n = norm(S.sel);
  $('#nameBox').value = n.r === n.r2 && n.c === n.c2 ? addr(S.sel.r, S.sel.c) : `${addr(n.r, n.c)}:${addr(n.r2, n.c2)}`;
  if (!S.editing) { const sv = S.ser(S.sel.r, S.sel.c); S.fx.value = sv == null ? '' : typeof sv === 'string' && sv.startsWith("'") ? sv : String(sv); }
  // Sum, average and count of what's selected.
  let sum = 0, cnt = 0, num = 0;
  if ((n.r2 - n.r + 1) * (n.c2 - n.c + 1) <= 200000 && !(n.r === n.r2 && n.c === n.c2)) {
    const u = S.used();
    for (let r = n.r; r <= Math.min(n.r2, u.rows - 1); r++) for (let c = n.c; c <= Math.min(n.c2, u.cols - 1); c++) { const v = S.val(r, c); if (v !== null && v !== '') { cnt++; if (typeof v === 'number') { sum += v; num++; } } }
  }
  $('#sCalc').textContent = num ? `Sum ${S.display(sum, S.fmtAt(n.r, n.c))}   Average ${S.display(Math.round(sum / num * 1e6) / 1e6, S.fmtAt(n.r, n.c))}   Count ${cnt}` : cnt ? `Count ${cnt}` : '';
  $('#sSel').textContent = S.sh().name;
  // Toolbar state.
  const f = S.fmtAt(S.sel.r, S.sel.c) || {};
  for (const b of $$('#toolbar [data-cmd]')) { const c = b.dataset.cmd; b.classList.toggle('on', c.startsWith('al-') ? (f.align || '') === c.slice(3) : c === 'filter' ? !!S.meta().filter : !!f[c]); }
  S.fontSel.value = f.font || ''; S.sizeSel.value = String(f.size || 11);
  const nf = f.nf || 'General'; if (![...S.nfSel.options].some((o) => o.value === nf)) S.nfSel.append(el('option', { value: nf, text: nf })); S.nfSel.value = nf;
};

function wire() {
  const sc = S.scroll;
  sc.addEventListener('scroll', () => { S.draw(); if (S.editing) S.placeEditor(); });
  let drag = null;
  sc.addEventListener('mousedown', (e) => {
    if (e.button !== 0) return;
    const rect = sc.getBoundingClientRect(); const x = e.clientX - rect.left, y = e.clientY - rect.top;
    if (x > sc.clientWidth || y > sc.clientHeight) return; // the scrollbars
    e.preventDefault();
    const h = S.hit(x, y);
    // Resizing a column or row from its header edge.
    if (h.hy && !h.hx) { const edge = colEdge(x); if (edge != null) { startResize('col', edge, e.clientX); return; } }
    if (h.hx && !h.hy) { const edge = rowEdge(y); if (edge != null) { startResize('row', edge, e.clientY); return; } }
    // Point mode: clicking a cell while typing a formula puts its address in the formula.
    if (S.editing && S.pointable()) { S.pointTo(h.r, h.c, false); drag = { point: true, r: h.r, c: h.c }; return; }
    if (S.editing) S.commit();
    // The fill handle.
    if (S.handle && Math.abs(x - S.handle.x) < 7 && Math.abs(y - S.handle.y) < 7) { drag = { fill: true }; S.fillTo = { ...norm(S.sel) }; return; }
    // Filter buttons.
    const m = S.meta();
    if (m.filter && h.r === m.filter.r && h.c >= m.filter.c && h.c <= m.filter.c2 && !h.hx && !h.hy) { const R = S.cellRect(h.r, h.c); if (x > R.x + R.w - 24) { S.filterMenu(h.c, e.clientX, e.clientY); return; } }
    if (h.hx && h.hy) { S.select(0, 0, m.rows - 1, m.cols - 1); return; }
    if (h.hy) { if (e.shiftKey) S.select(S.sel.r, S.sel.c, m.rows - 1, h.c, true); else { S.sel = { r: 0, c: h.c, r2: m.rows - 1, c2: h.c }; S.updateBar(); S.draw(); } drag = { col: h.c }; S.focus(); return; }
    if (h.hx) { if (e.shiftKey) S.select(S.sel.r, S.sel.c, h.r, m.cols - 1, true); else { S.sel = { r: h.r, c: 0, r2: h.r, c2: m.cols - 1 }; S.updateBar(); S.draw(); } drag = { row: h.r }; S.focus(); return; }
    if (e.shiftKey) S.select(S.sel.r, S.sel.c, h.r, h.c, true); else S.select(h.r, h.c);
    drag = { cell: true };
    S.focus();
  });
  window.addEventListener('mousemove', (e) => {
    if (!drag) { hoverCursor(e); return; }
    const rect = sc.getBoundingClientRect(); const h = S.hit(e.clientX - rect.left, e.clientY - rect.top);
    if (drag.point) { S.pointTo(drag.r, drag.c, false, h.r, h.c); return; }
    if (drag.fill) {
      const n = norm(S.sel);
      const dr = h.r > n.r2 ? h.r - n.r2 : h.r < n.r ? h.r - n.r : 0, dc = h.c > n.c2 ? h.c - n.c2 : h.c < n.c ? h.c - n.c : 0;
      if (Math.abs(dr) >= Math.abs(dc)) S.fillTo = dr >= 0 ? { r: n.r, c: n.c, r2: Math.max(n.r2, h.r), c2: n.c2 } : { r: h.r, c: n.c, r2: n.r2, c2: n.c2 };
      else S.fillTo = dc >= 0 ? { r: n.r, c: n.c, r2: n.r2, c2: Math.max(n.c2, h.c) } : { r: n.r, c: h.c, r2: n.r2, c2: n.c2 };
      S.draw(); return;
    }
    const m = S.meta();
    if (drag.col != null) { S.sel = { r: 0, c: drag.col, r2: m.rows - 1, c2: h.c }; S.updateBar(); S.draw(); return; }
    if (drag.row != null) { S.sel = { r: drag.row, c: 0, r2: h.r, c2: m.cols - 1 }; S.updateBar(); S.draw(); return; }
    if (drag.cell) S.select(S.sel.r, S.sel.c, h.r, h.c, true);
  });
  window.addEventListener('mouseup', () => {
    if (drag && drag.fill && S.fillTo) { const t = S.fillTo; S.fillTo = null; S.fill(norm(S.sel), norm(t)); }
    drag = null;
  });
  sc.addEventListener('dblclick', (e) => {
    const rect = sc.getBoundingClientRect(); const x = e.clientX - rect.left, y = e.clientY - rect.top;
    const h = S.hit(x, y);
    if (h.hy && !h.hx) { const edge = colEdge(x); if (edge != null) { S.autoFit(edge); return; } }
    if (h.hx || h.hy) return;
    S.select(h.r, h.c); S.startEdit(null, true);
  });
  sc.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    const rect = sc.getBoundingClientRect(); const h = S.hit(e.clientX - rect.left, e.clientY - rect.top);
    const n = norm(S.sel);
    if (!(h.r >= n.r && h.r <= n.r2 && h.c >= n.c && h.c <= n.c2) && !h.hx && !h.hy) S.select(h.r, h.c);
    S.contextMenu(e.clientX, e.clientY, h);
  });
  sc.addEventListener('wheel', (e) => { if (e.ctrlKey) { e.preventDefault(); S.zoom(S.zoomV + (e.deltaY < 0 ? 10 : -10)); } }, { passive: false });
  // Keyboard (the hidden #kb box has the focus while no cell is being edited).
  S.kb.addEventListener('keydown', onKey);
  S.kb.addEventListener('paste', (e) => { e.preventDefault(); S.paste(e.clipboardData); });
  S.kb.addEventListener('copy', (e) => { e.preventDefault(); S.copy(e.clipboardData, false); });
  S.kb.addEventListener('cut', (e) => { e.preventDefault(); S.copy(e.clipboardData, true); });
  S.kb.addEventListener('input', () => { const v = S.kb.value; S.kb.value = ''; if (v && !S.editing) S.startEdit(v); });
  S.ed.addEventListener('keydown', onEditKey);
  S.ed.addEventListener('input', () => { S.fx.value = S.ed.value; S.point = null; S.autocomplete(); S.sizeEditor(); });
  S.fx.addEventListener('focus', () => { if (!S.editing) S.startEdit(null, true, true); });
  S.fx.addEventListener('input', () => { if (S.editing) { S.ed.value = S.fx.value; S.autocomplete(); } });
  S.fx.addEventListener('keydown', (e) => { if (e.key === 'Enter') { e.preventDefault(); S.commit(); S.move(1, 0); S.focus(); } else if (e.key === 'Escape') { e.preventDefault(); S.cancel(); S.focus(); } else if (e.key === 'Tab') { e.preventDefault(); S.commit(); S.move(0, 1); S.focus(); } });
  $('#nameBox').addEventListener('keydown', (e) => {
    if (e.key !== 'Enter') return;
    e.preventDefault();
    const v = e.target.value.trim(); const parts = v.split(':');
    const a = parseAddr(parts[0]), b = parts[1] ? parseAddr(parts[1]) : a;
    if (a && b) { S.select(a.r, a.c, b.r, b.c); } else H.toast('Type a cell like B4 or a range like A1:C10');
    S.focus();
  });
  $('#nameBox').addEventListener('focus', (e) => e.target.select());
}
function colEdge(x) { const m = S.meta(); for (let c = 0; c < m.cols; c++) { const R = S.cellRect(0, c); if (Math.abs(x - (R.x + R.w)) < 4) return c; if (R.x > x + 4) break; } return null; }
function rowEdge(y) { const m = S.meta(); const h = S.hit(S.hw + 1, y); for (const r of [h.r - 1, h.r, h.r + 1]) { if (r < 0 || r >= m.rows) continue; const R = S.cellRect(r, 0); if (Math.abs(y - (R.y + R.h)) < 4) return r; } return null; }
function hoverCursor(e) {
  const rect = S.scroll.getBoundingClientRect(); const x = e.clientX - rect.left, y = e.clientY - rect.top;
  if (x < 0 || y < 0 || x > S.scroll.clientWidth || y > S.scroll.clientHeight) return;
  let cur = 'cell';
  if (y < S.hh && x > S.hw && colEdge(x) != null) cur = 'col-resize';
  else if (x < S.hw && y > S.hh && rowEdge(y) != null) cur = 'row-resize';
  else if (S.handle && Math.abs(x - S.handle.x) < 7 && Math.abs(y - S.handle.y) < 7) cur = 'crosshair';
  else if (x < S.hw || y < S.hh) cur = 'default';
  S.scroll.style.cursor = cur;
}
function startResize(kind, i, start) {
  const m = S.meta(); const z = S.zoomV / 100;
  const orig = kind === 'col' ? (m.colW[i] ?? DW) : (m.rowH[i] ?? DH);
  const before = cloneMeta(m);
  const mv = (e) => { const d = ((kind === 'col' ? e.clientX : e.clientY) - start) / z; const v = Math.max(kind === 'col' ? 12 : 8, Math.round(orig + d)); if (kind === 'col') m.colW[i] = v; else m.rowH[i] = v; S.layout(); S.draw(); };
  const up = () => { removeEventListener('mousemove', mv); removeEventListener('mouseup', up); const after = cloneMeta(m); U.stack.push({ sheet: S.sh(), metaBefore: before, metaAfter: after, sel: { ...S.sel } }); U.redo = []; H.changed(); };
  addEventListener('mousemove', mv); addEventListener('mouseup', up);
}
S.autoFit = (c) => {
  const u = S.used(); const ctx = S.ctx; let w = 30;
  for (let r = 0; r < Math.min(u.rows, 5000); r++) { const t = S.text(r, c); if (!t) continue; const f = S.fmtAt(r, c) || {}; ctx.font = `${f.b ? '700 ' : ''}${((f.size || 11) * 1.333).toFixed(1)}px ${FONT}`; w = Math.max(w, ctx.measureText(t).width + 12); }
  S.metaChange(() => { S.meta().colW[c] = Math.min(600, Math.ceil(w)); });
};

function onKey(e) {
  const k = H.keyOf(e);
  const n = S.sel, m = S.meta();
  const nav = { ArrowUp: [-1, 0], ArrowDown: [1, 0], ArrowLeft: [0, -1], ArrowRight: [0, 1] };
  const base = k.replace(/^(Ctrl\+)?(Shift\+)?/, '');
  if (nav[base]) {
    e.preventDefault();
    const [dr, dc] = nav[base];
    const ext = e.shiftKey, ctrl = e.ctrlKey || e.metaKey;
    const from = ext ? { r: n.r2, c: n.c2 } : { r: n.r, c: n.c };
    let to = ctrl ? jump(from.r, from.c, dr, dc) : { r: from.r + dr, c: from.c + dc };
    // Skip rows a filter hides.
    while (!ctrl && dr && m.hiddenRows[to.r] && to.r > 0 && to.r < m.rows - 1) to.r += dr;
    // Step over a merged cell in one go.
    const mg = !ext && S.mergeAt(n.r, n.c);
    if (mg && !ctrl) { if (dr > 0) to.r = mg.r2 + 1; if (dc > 0) to.c = mg.c2 + 1; }
    if (ext) S.select(n.r, n.c, to.r, to.c, true); else S.select(to.r, to.c);
    return;
  }
  if (k === 'Enter' || k === 'Shift+Enter') { e.preventDefault(); const nn = norm(n); void nn; S.move(k === 'Enter' ? 1 : -1, 0); return; }
  if (k === 'Tab' || k === 'Shift+Tab') { e.preventDefault(); const nn = norm(n); void nn; S.move(0, k === 'Tab' ? 1 : -1); return; }
  if (k === 'F2') { e.preventDefault(); S.startEdit(null, true); return; }
  if (k === 'Delete' || k === 'Backspace') { e.preventDefault(); if (k === 'Backspace') { S.startEdit(''); return; } S.clear('contents'); return; }
  if (k === 'Home') { e.preventDefault(); S.select(n.r, 0); return; }
  if (k === 'Ctrl+Home') { e.preventDefault(); S.select(0, 0); return; }
  if (k === 'Ctrl+End') { e.preventDefault(); const u = S.used(); S.select(Math.max(0, u.rows - 1), Math.max(0, u.cols - 1)); return; }
  if (k === 'PageDown' || k === 'PageUp') { e.preventDefault(); const rows = Math.floor(($('#sgrid').clientHeight - S.hh) / (DH * S.zoomV / 100)) - 1; S.select(Math.max(0, n.r + (k === 'PageDown' ? rows : -rows)), n.c); return; }
  if (k === 'Ctrl+Space') { e.preventDefault(); S.sel = { r: 0, c: n.c, r2: m.rows - 1, c2: n.c2 }; S.draw(); S.updateBar(); return; }
  if (k === 'Shift+Space') { e.preventDefault(); S.sel = { r: n.r, c: 0, r2: n.r2, c2: m.cols - 1 }; S.draw(); S.updateBar(); return; }
  if (k === 'Escape') { if (S.clip) { S.clip = null; S.draw(); } return; }
}
S.move = (dr, dc) => {
  const m = S.meta(); let r = S.sel.r + dr, c = S.sel.c + dc;
  const mg = S.mergeAt(S.sel.r, S.sel.c); if (mg) { if (dr > 0) r = mg.r2 + 1; if (dc > 0) c = mg.c2 + 1; }
  while (dr && m.hiddenRows[r] && r > 0) r += dr;
  S.select(Math.max(0, r), Math.max(0, c));
};

// ------------------------------------------------------------------ editing a cell
S.startEdit = (initial, keep, inFx) => {
  if (S.editing) return;
  const { r, c } = S.sel;
  const sv = S.ser(r, c);
  const cur = sv == null ? '' : String(sv);
  S.editing = { r, c, mode: keep ? 'edit' : 'enter', orig: cur };
  S.ed.hidden = false;
  S.ed.value = initial != null ? initial : cur;
  S.fx.value = S.ed.value;
  const f = S.fmtAt(r, c) || {};
  Object.assign(S.ed.style, { font: `${f.i ? 'italic ' : ''}${f.b ? '700 ' : ''}${((f.size || 11) * 1.333 * S.zoomV / 100).toFixed(1)}px ${f.font ? `"${f.font}", ` : ''}${FONT}`, color: f.color || '#1b1b1b', background: f.bg || '#fff', textAlign: f.align || 'left' });
  S.placeEditor();
  if (!inFx) { S.ed.focus(); const L = S.ed.value.length; S.ed.setSelectionRange(L, L); }
  S.autocomplete();
};
S.placeEditor = () => {
  if (!S.editing) return;
  const mg = S.mergeAt(S.editing.r, S.editing.c);
  const R = mg ? S.rangeRect(mg) : S.cellRect(S.editing.r, S.editing.c);
  Object.assign(S.ed.style, { left: R.x - 1 + 'px', top: R.y - 1 + 'px', minWidth: R.w + 1 + 'px', minHeight: R.h + 1 + 'px', height: R.h + 1 + 'px' });
  S.sizeEditor();
};
S.sizeEditor = () => { S.ed.style.width = '0px'; S.ed.style.width = Math.min(Math.max(S.ed.scrollWidth + 8, parseFloat(S.ed.style.minWidth) || 0), 640) + 'px'; S.ed.style.height = 'auto'; S.ed.style.height = Math.max(S.ed.scrollHeight, parseFloat(S.ed.style.minHeight) || 0) + 'px'; };
S.cancel = () => { if (!S.editing) return; S.editing = null; S.point = null; S.ed.hidden = true; $('#acList').hidden = true; S.updateBar(); S.draw(); S.focus(); };
// Typed text into a cell: numbers with thousands separators, percentages, dates and formulas are understood.
S.parseInput = (t) => {
  if (t === '') return { v: null };
  if (t.startsWith('=') && t.length > 1) return { v: fixFormula(t) };
  const m = /^(-?)(\d{1,3}(?:,\d{3})+)(\.\d+)?$/.exec(t.trim());
  if (m) return { v: Number(m[1] + m[2].replace(/,/g, '') + (m[3] || '')), nf: m[3] ? '#,##0.' + '0'.repeat(m[3].length - 1) : '#,##0' };
  return { v: t };
};
// Formulas typed the Excel way: function names in capitals, and unclosed brackets closed.
const fixFormula = (t) => {
  let s = t, out = '', inStr = false, depth = 0;
  for (const ch of s) { if (ch === '"') inStr = !inStr; if (!inStr) { if (ch === '(') depth++; if (ch === ')') depth--; } out += ch; }
  if (inStr) out += '"';
  while (depth-- > 0) out += ')';
  return out.replace(/("[^"]*")|([A-Za-z_][A-Za-z0-9_.]*)(?=\()/g, (mm, str, fn) => str || fn.toUpperCase());
};
S.commit = () => {
  if (!S.editing) return;
  const { r, c } = S.editing;
  const text = S.ed.value;
  S.editing = null; S.point = null; S.ed.hidden = true; $('#acList').hidden = true;
  if (text === (S.ser(r, c) == null ? '' : String(S.ser(r, c)))) { S.draw(); S.updateBar(); return; }
  const p = S.parseInput(text);
  S.change({ r, c, r2: r, c2: c }, () => {
    S.hf.setCellContents({ sheet: S.sid(), row: r, col: c }, [[p.v]]);
    autoFormat(r, c, p.nf);
  }, { meta: true });
};
// A new date, percentage or currency gets a matching number format.
function autoFormat(r, c, nf) {
  const m = S.meta(); const k = key(r, c);
  const f = m.fmt[k] || {};
  if (nf && !f.nf) { m.fmt[k] = { ...f, nf }; return; }
  if (f.nf) return;
  const t = S.hf.getCellValueDetailedType({ sheet: S.sid(), row: r, col: c });
  const raw = String(S.ser(r, c) ?? '');
  const map = { NUMBER_DATE: 'yyyy-mm-dd', NUMBER_DATETIME: 'yyyy-mm-dd h:mm', NUMBER_TIME: 'h:mm', NUMBER_CURRENCY: '"$"#,##0.00', NUMBER_PERCENT: /\.\d/.test(raw) ? '0.00%' : '0%' };
  if (map[t] && !raw.startsWith('=')) m.fmt[k] = { ...f, nf: map[t] };
  else if (map[t] && raw.startsWith('=') && (t === 'NUMBER_DATE' || t === 'NUMBER_CURRENCY')) m.fmt[k] = { ...f, nf: map[t] };
}
function onEditKey(e) {
  const k = H.keyOf(e);
  const ac = $('#acList');
  if (!ac.hidden && (k === 'Tab' || k === 'Enter') && ac.querySelector('.on')) { e.preventDefault(); S.acceptAc(); return; }
  if (!ac.hidden && (k === 'ArrowDown' || k === 'ArrowUp')) { e.preventDefault(); const items = $$('.ac', ac); let i = items.findIndex((x) => x.classList.contains('on')); items.forEach((x) => x.classList.remove('on')); i = (i + (k === 'ArrowDown' ? 1 : -1) + items.length) % items.length; items[i].classList.add('on'); return; }
  if (k === 'Enter' || k === 'Shift+Enter') { e.preventDefault(); S.commit(); S.move(k === 'Enter' ? 1 : -1, 0); S.focus(); return; }
  if (k === 'Alt+Enter') { e.preventDefault(); const s = S.ed.selectionStart; S.ed.setRangeText('\n', s, S.ed.selectionEnd, 'end'); S.sizeEditor(); return; }
  if (k === 'Tab' || k === 'Shift+Tab') { e.preventDefault(); S.commit(); S.move(0, k === 'Tab' ? 1 : -1); S.focus(); return; }
  if (k === 'Escape') { e.preventDefault(); if (!ac.hidden) { ac.hidden = true; return; } S.cancel(); return; }
  if (k === 'F4') { e.preventDefault(); toggleAbsolute(); return; }
  const nav = { ArrowUp: [-1, 0], ArrowDown: [1, 0], ArrowLeft: [0, -1], ArrowRight: [0, 1] };
  const base = k.replace(/^Shift\+/, '');
  if (nav[base] && !e.ctrlKey) {
    if (S.pointable() || S.point) { e.preventDefault(); const [dr, dc] = nav[base]; const p = S.point ? (e.shiftKey ? { r: S.point.r2, c: S.point.c2 } : { r: S.point.r, c: S.point.c }) : { r: S.editing.r, c: S.editing.c }; if (e.shiftKey && S.point) S.pointTo(S.point.r, S.point.c, true, p.r + dr, p.c + dc); else S.pointTo(Math.max(0, p.r + dr), Math.max(0, p.c + dc), true); return; }
    if (S.editing.mode === 'enter') { e.preventDefault(); S.commit(); S.move(...nav[base]); S.focus(); }
  }
}
// Point mode: in a formula, right after an operator or "(", arrows and clicks pick cells.
S.pointable = () => {
  if (!S.editing) return false;
  const v = S.ed.value, p = S.ed.selectionStart;
  if (!v.startsWith('=') || p !== S.ed.selectionEnd) return false;
  if (S.point) return true;
  return /[=(,+\-*/^&<>:;]\s*$/.test(v.slice(0, p));
};
S.pointTo = (r, c, kb, r2 = r, c2 = c) => {
  const v = S.ed.value;
  const refText = (r === r2 && c === c2) ? addr(r, c) : `${addr(Math.min(r, r2), Math.min(c, c2))}:${addr(Math.max(r, r2), Math.max(c, c2))}`;
  let start = S.ed.selectionStart;
  if (S.point && S.point.start != null) start = S.point.start;
  const end = S.point && S.point.end != null ? S.point.end : S.ed.selectionEnd;
  S.ed.value = v.slice(0, start) + refText + v.slice(end);
  S.point = { r, c, r2, c2, start, end: start + refText.length };
  S.ed.setSelectionRange(S.point.end, S.point.end);
  S.fx.value = S.ed.value;
  S.ed.focus(); S.sizeEditor(); S.draw();
  if (kb) S.ensureVisible(r2, c2);
};
function toggleAbsolute() {
  const v = S.ed.value, p = S.ed.selectionStart;
  const re = /\$?[A-Z]{1,3}\$?\d+/gi; let m;
  while ((m = re.exec(v))) {
    if (p >= m.index && p <= m.index + m[0].length) {
      const t = m[0]; const a = /^(\$?)([A-Z]+)(\$?)(\d+)$/i.exec(t);
      const state = (a[1] ? 2 : 0) + (a[3] ? 1 : 0);
      const next = [[1, 1], [0, 1], [1, 0], [0, 0]][state === 0 ? 0 : state === 3 ? 1 : state === 1 ? 2 : 3];
      const nt = (next[0] ? '$' : '') + a[2] + (next[1] ? '$' : '') + a[4];
      S.ed.value = v.slice(0, m.index) + nt + v.slice(m.index + t.length);
      S.ed.setSelectionRange(m.index + nt.length, m.index + nt.length); S.fx.value = S.ed.value;
      return;
    }
  }
}
// Function names as you type them.
S.autocomplete = () => {
  const ac = $('#acList');
  if (!S.editing) { ac.hidden = true; return; }
  const v = S.ed.value.slice(0, S.ed.selectionStart);
  const m = /^=.*?([A-Za-z][A-Za-z0-9.]*)$/.exec(v);
  if (!m || /\d$/.test(m[1]) && /[A-Za-z]+\d+$/.test(m[1])) { ac.hidden = true; return; }
  const word = m[1].toUpperCase();
  S.fnames = S.fnames || S.hf.getRegisteredFunctionNames().sort();
  const list = S.fnames.filter((f) => f.startsWith(word)).slice(0, 8);
  if (!list.length || (list.length === 1 && list[0] === word)) { ac.hidden = true; return; }
  ac.innerHTML = ''; list.forEach((f, i) => ac.append(el('div', { class: 'ac' + (i === 0 ? ' on' : ''), text: f, onmousedown: (e) => { e.preventDefault(); ac.querySelectorAll('.ac').forEach((x) => x.classList.remove('on')); e.target.classList.add('on'); S.acceptAc(); } })));
  ac.dataset.word = m[1];
  const R = S.ed.getBoundingClientRect(), G = $('#sgrid').getBoundingClientRect();
  ac.style.left = (R.left - G.left) + 'px'; ac.style.top = (R.bottom - G.top + 2) + 'px';
  ac.hidden = false;
};
S.acceptAc = () => {
  const ac = $('#acList'); const on = ac.querySelector('.on'); if (!on) return;
  const p = S.ed.selectionStart, w = ac.dataset.word.length;
  S.ed.setRangeText(on.textContent + '(', p - w, p, 'end');
  S.fx.value = S.ed.value; ac.hidden = true; S.ed.focus();
};

// ------------------------------------------------------------------ formatting
const cellsIn = (n, cap = 200000) => {
  // Whole rows or columns: only as far as the data (and a little more).
  const u = S.used(); const m = S.meta();
  const r2 = n.r2 >= m.rows - 1 ? Math.max(n.r, Math.min(n.r2, Math.max(u.rows, 100) - 1)) : n.r2;
  const c2 = n.c2 >= m.cols - 1 ? Math.max(n.c, Math.min(n.c2, Math.max(u.cols, 26) - 1)) : n.c2;
  const out = []; for (let r = n.r; r <= r2 && out.length < cap; r++) for (let c = n.c; c <= c2; c++) out.push([r, c]);
  return out;
};
S.setFmt = (props) => {
  const n = norm(S.sel);
  S.metaChange(() => {
    const m = S.meta();
    for (const [r, c] of cellsIn(n)) {
      const k = key(r, c); const f = { ...(m.fmt[k] || {}) };
      for (const [p, v] of Object.entries(props)) { if (v === null || v === undefined || v === false) delete f[p]; else f[p] = v; }
      if (Object.keys(f).length) m.fmt[k] = f; else delete m.fmt[k];
    }
  });
};
S.toggle = (p) => { const f = S.fmtAt(S.sel.r, S.sel.c) || {}; S.setFmt({ [p]: f[p] ? null : true }); };
S.decimals = (d) => {
  const f = S.fmtAt(S.sel.r, S.sel.c) || {}; let nf = f.nf;
  const v = S.val(S.sel.r, S.sel.c);
  if (!nf || nf === 'General') { const s = typeof v === 'number' ? String(v) : ''; const dec = s.includes('.') ? s.split('.')[1].length : 0; nf = dec ? '0.' + '0'.repeat(dec) : '0'; }
  const m = /(0+)?(\.(0+))?(%|E\+00)?/.exec(nf.replace(/"[^"]*"/g, ''));
  let cur = m && m[3] ? m[3].length : 0; cur = Math.max(0, cur + d);
  const next = nf.replace(/0(\.0+)?(?=[^0]*$|%|E)/, cur ? '0.' + '0'.repeat(cur) : '0');
  S.setFmt({ nf: next });
};
S.customFormat = async () => {
  const f = S.fmtAt(S.sel.r, S.sel.c) || {};
  const v = await H.prompt('Number format', 'Excel-style format code (e.g. #,##0.00 or dd/mm/yyyy or 0.0%)', f.nf || 'General');
  if (v != null) S.setFmt({ nf: v === 'General' ? null : v });
};
S.borders = (kind) => {
  const n = norm(S.sel);
  S.metaChange(() => {
    const m = S.meta();
    for (const [r, c] of cellsIn(n)) {
      const k = key(r, c); const f = { ...(m.fmt[k] || {}) }; const bd = { ...(f.bd || {}) };
      const w = kind === 'thick' ? 'thick' : 'thin';
      if (kind === 'none') { delete f.bd; if (Object.keys(f).length) m.fmt[k] = f; else delete m.fmt[k]; continue; }
      if (kind === 'all') { bd.t = bd.r = bd.b = bd.l = 'thin'; }
      if ((kind === 'outer' || kind === 'thick' || kind === 'top') && r === n.r) bd.t = w;
      if ((kind === 'outer' || kind === 'thick' || kind === 'bottom') && r === n.r2) bd.b = w;
      if ((kind === 'outer' || kind === 'thick' || kind === 'left') && c === n.c) bd.l = w;
      if ((kind === 'outer' || kind === 'thick' || kind === 'right') && c === n.c2) bd.r = w;
      if (Object.keys(bd).length) { f.bd = bd; m.fmt[k] = f; }
    }
  });
};
S.merge = () => {
  const n = norm(S.sel); const m = S.meta();
  const existing = m.merges.filter((g) => !(g.r > n.r2 || g.r2 < n.r || g.c > n.c2 || g.c2 < n.c));
  if (existing.length) { S.metaChange(() => { m.merges = m.merges.filter((g) => !existing.includes(g)); }); return; }
  if (n.r === n.r2 && n.c === n.c2) return;
  // Only the top-left value is kept (as in other spreadsheets).
  let others = 0; for (const [r, c] of cellsIn(n)) if ((r !== n.r || c !== n.c) && S.val(r, c) !== null && S.val(r, c) !== '') others++;
  const go = () => S.change(n, () => {
    for (const [r, c] of cellsIn(n)) if (r !== n.r || c !== n.c) S.hf.setCellContents({ sheet: S.sid(), row: r, col: c }, [[null]]);
    m.merges.push({ ...n });
    const k = key(n.r, n.c); m.fmt[k] = { ...(m.fmt[k] || {}), align: (m.fmt[k] || {}).align || 'center' };
  }, { meta: true });
  if (others) H.confirm('Merge cells?', 'Merging keeps only the top-left value; the others are cleared.', 'Merge').then((ok) => ok && go()); else go();
};
S.freeze = (r, c) => { S.metaChange(() => { S.meta().freeze = { r: Math.max(0, r), c: Math.max(0, c) }; }); S.scroll.scrollTop = 0; S.scroll.scrollLeft = 0; };
S.zoom = (v) => { S.zoomV = Math.max(50, Math.min(200, Math.round(v / 10) * 10)); $('#zVal').textContent = S.zoomV + '%'; S.layout(); S.draw(); };

// ------------------------------------------------------------------ clear, copy, paste, fill
S.clear = (what) => {
  const n = norm(S.sel); const cells = cellsIn(n);
  const u = S.used(); const r2 = Math.min(n.r2, Math.max(u.rows - 1, n.r)), c2 = Math.min(n.c2, Math.max(u.cols - 1, n.c));
  S.change({ r: n.r, c: n.c, r2, c2 }, () => {
    if (what !== 'formats') { const blank = []; for (let r = n.r; r <= r2; r++) { const row = []; for (let c = n.c; c <= c2; c++) row.push(null); blank.push(row); } S.hf.setCellContents({ sheet: S.sid(), row: n.r, col: n.c }, blank); }
    if (what !== 'contents') { const m = S.meta(); for (const [r, c] of cells) delete m.fmt[key(r, c)]; }
  }, { meta: true, suspend: true });
};
const tsvOf = (n) => { const lines = []; for (let r = n.r; r <= n.r2; r++) { const row = []; for (let c = n.c; c <= n.c2; c++) { let t = S.text(r, c); if (/[\t\n"]/.test(t)) t = '"' + t.replace(/"/g, '""') + '"'; row.push(t); } lines.push(row.join('\t')); } return lines.join('\n'); };
S.copy = (cd, cut) => {
  const n = norm(S.sel); const u = S.used();
  const nn = { r: n.r, c: n.c, r2: Math.min(n.r2, Math.max(n.r, u.rows - 1)), c2: Math.min(n.c2, Math.max(n.c, u.cols - 1)) };
  const text = tsvOf(nn);
  const fmts = []; for (let r = nn.r; r <= nn.r2; r++) { const row = []; for (let c = nn.c; c <= nn.c2; c++) row.push(S.fmtAt(r, c)); fmts.push(row); }
  S.clip = { sid: S.sid(), n: nn, text, fmts, cut };
  if (cut) S.hf.cut({ start: { sheet: S.sid(), row: nn.r, col: nn.c }, end: { sheet: S.sid(), row: nn.r2, col: nn.c2 } });
  else S.hf.copy({ start: { sheet: S.sid(), row: nn.r, col: nn.c }, end: { sheet: S.sid(), row: nn.r2, col: nn.c2 } });
  if (cd) cd.setData('text/plain', text); else { try { navigator.clipboard.writeText(text); } catch {} }
  S.draw();
};
S.paste = async (cd) => {
  let text = cd ? cd.getData('text/plain') : null;
  if (text == null) { try { text = await navigator.clipboard.readText(); } catch { H.toast('Press Ctrl+V to paste'); return; } }
  const { r, c } = S.sel;
  const clip = S.clip;
  if (clip && (text === clip.text || !text)) {
    // From this window: formulas move with their references, formats come along.
    const h = clip.n.r2 - clip.n.r, w = clip.n.c2 - clip.n.c;
    const target = { r, c, r2: r + h, c2: c + w };
    S.change(clip.cut ? 'all' : target, () => {
      S.hf.paste({ sheet: S.sid(), row: r, col: c });
      const m = S.meta();
      for (let i = 0; i <= h; i++) for (let j = 0; j <= w; j++) { const f = clip.fmts[i][j]; if (f) m.fmt[key(r + i, c + j)] = { ...f }; else delete m.fmt[key(r + i, c + j)]; }
      if (clip.cut && clip.sid === S.sid()) for (let i = 0; i <= h; i++) for (let j = 0; j <= w; j++) { const rr = clip.n.r + i, cc = clip.n.c + j; if (rr < r || rr > r + h || cc < c || cc > c + w) delete m.fmt[key(rr, cc)]; }
    }, { meta: true });
    if (clip.cut) S.clip = null;
    S.select(r, c, r + h, c + w);
    return;
  }
  // From elsewhere: tab-separated rows (what other spreadsheets and tables copy).
  const rows = parseDelimited(text.replace(/\r?\n$/, ''), text.includes('\t') ? '\t' : (text.includes('\n') || !text.includes(',') ? '\t' : '\t'));
  if (!rows.length) return;
  const w = Math.max(...rows.map((x) => x.length));
  S.change({ r, c, r2: r + rows.length - 1, c2: c + w - 1 }, () => {
    const data = rows.map((row) => Array.from({ length: w }, (_, j) => { const v = row[j] ?? ''; return v === '' ? null : S.parseInput(v).v; }));
    S.hf.setCellContents({ sheet: S.sid(), row: r, col: c }, data);
  });
  S.select(r, c, r + rows.length - 1, c + w - 1);
};
// Fill: drag the corner. Numbers with a step continue the series, dates count up, "Item 1" counts up, lists
// of days and months carry on, formulas copy with their references moved, everything else repeats.
const LISTS = [['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'], ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'],
  ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'], ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'], ['Q1', 'Q2', 'Q3', 'Q4']];
S.fill = (src, dst) => {
  if (dst.r === src.r && dst.r2 === src.r2 && dst.c === src.c && dst.c2 === src.c2) return;
  const vertical = dst.c === src.c && dst.c2 === src.c2;
  const sid = S.sid();
  S.change(dst, () => {
    const m = S.meta();
    const lines = vertical ? range(src.c, src.c2) : range(src.r, src.r2);
    for (const L of lines) {
      const cells = vertical ? range(src.r, src.r2).map((r) => [r, L]) : range(src.c, src.c2).map((c) => [L, c]);
      const targets = vertical ? (dst.r2 > src.r2 ? range(src.r2 + 1, dst.r2) : range(dst.r, src.r - 1).reverse()).map((r) => [r, L]) : (dst.c2 > src.c2 ? range(src.c2 + 1, dst.c2) : range(dst.c, src.c - 1).reverse()).map((c) => [L, c]);
      const forward = vertical ? dst.r2 > src.r2 : dst.c2 > src.c2;
      const sers = cells.map(([r, c]) => S.ser(r, c, sid));
      const vals = cells.map(([r, c]) => S.val(r, c, sid));
      const allNum = sers.every((s) => typeof s === 'number');
      const isDate = (f) => f && f.nf && /[dmy]/i.test(f.nf.replace(/"[^"]*"/g, '')) && !/^\[?h/.test(f.nf);
      const fmts = cells.map(([r, c]) => S.fmtAt(r, c));
      let gen = null;
      if (allNum && (sers.length > 1 || isDate(fmts[0]))) {
        const step = sers.length > 1 ? (sers[sers.length - 1] - sers[0]) / (sers.length - 1) : 1;
        const startV = forward ? sers[sers.length - 1] : sers[0];
        gen = (i) => Math.round((startV + (forward ? 1 : -1) * step * (i + 1)) * 1e10) / 1e10;
      } else if (sers.length === 1 && typeof sers[0] === 'string' && !sers[0].startsWith('=')) {
        const s = sers[0];
        const li = LISTS.find((l) => l.some((x) => x.toLowerCase() === s.toLowerCase()));
        const nm = /^(.*?)(\d+)(\D*)$/.exec(s);
        if (li) { const i0 = li.findIndex((x) => x.toLowerCase() === s.toLowerCase()); gen = (i) => li[(((i0 + (forward ? 1 : -1) * (i + 1)) % li.length) + li.length) % li.length]; }
        else if (nm) gen = (i) => nm[1] + Math.max(0, +nm[2] + (forward ? 1 : -1) * (i + 1)) + nm[3];
      }
      targets.forEach(([r, c], i) => {
        const srcIdx = forward ? i % cells.length : cells.length - 1 - (i % cells.length);
        const [sr, sc] = cells[srcIdx];
        if (gen) S.hf.setCellContents({ sheet: sid, row: r, col: c }, [[gen(i)]]);
        else if (typeof sers[srcIdx] === 'string' && sers[srcIdx].startsWith('=')) { S.hf.copy({ start: { sheet: sid, row: sr, col: sc }, end: { sheet: sid, row: sr, col: sc } }); S.hf.paste({ sheet: sid, row: r, col: c }); }
        else S.hf.setCellContents({ sheet: sid, row: r, col: c }, [[sers[srcIdx] ?? null]]);
        const f = S.fmtAt(sr, sc, m); if (f) m.fmt[key(r, c)] = { ...f }; else delete m.fmt[key(r, c)];
      });
      void vals;
    }
  }, { meta: true });
  S.sel = { r: dst.r, c: dst.c, r2: dst.r2, c2: dst.c2 }; S.updateBar(); S.draw();
};
const range = (a, b) => { const o = []; for (let i = a; i <= b; i++) o.push(i); return o; };
S.autoSum = () => {
  const { r, c } = S.sel;
  // The numbers right above (or to the left of) the cell.
  let r0 = r - 1; while (r0 >= 0 && typeof S.val(r0, c) === 'number') r0--;
  let f;
  if (r0 < r - 1) f = `=SUM(${addr(r0 + 1, c)}:${addr(r - 1, c)})`;
  else { let c0 = c - 1; while (c0 >= 0 && typeof S.val(r, c0) === 'number') c0--; f = c0 < c - 1 ? `=SUM(${addr(r, c0 + 1)}:${addr(r, c - 1)})` : '=SUM()'; }
  S.startEdit(f, true);
  if (f === '=SUM()') S.ed.setSelectionRange(5, 5);
};

// ------------------------------------------------------------------ rows, columns, sheets
const shiftMeta = (m, kind, at, n) => {
  // Moves formatting, sizes, merges and the filter when rows/columns are added (n > 0) or removed (n < 0).
  const isRow = kind === 'row';
  const mv = (i) => (i >= at ? (n < 0 && i < at - n ? null : i + n) : i);
  const fmt = {};
  for (const [k, f] of Object.entries(m.fmt)) { const [r, c] = k.split(',').map(Number); const nr = isRow ? mv(r) : r, nc = isRow ? c : mv(c); if (nr != null && nc != null) fmt[key(nr, nc)] = f; }
  m.fmt = fmt;
  const sizes = isRow ? m.rowH : m.colW; const ns = {};
  for (const [k, v] of Object.entries(sizes)) { const t = mv(+k); if (t != null) ns[t] = v; }
  if (isRow) m.rowH = ns; else m.colW = ns;
  m.merges = m.merges.map((g) => { const a = isRow ? 'r' : 'c', b = isRow ? 'r2' : 'c2'; const s = mv(g[a]), e = mv(g[b]); if (s == null || e == null) return null; return { ...g, [a]: s, [b]: e }; }).filter((g) => g && (g.r2 > g.r || g.c2 > g.c));
  if (m.filter) { const f = m.filter; if (isRow) { const r = mv(f.r), r2 = mv(f.r2); if (r == null) m.filter = null; else { f.r = r; f.r2 = r2 ?? f.r2 + n; } } else { const c = mv(f.c), c2 = mv(f.c2); if (c == null || c2 == null) m.filter = null; else { f.c = c; f.c2 = c2; } } }
  if (isRow) m.hiddenRows = {};
};
S.insert = (kind, before) => {
  const n = norm(S.sel); const count = kind === 'row' ? n.r2 - n.r + 1 : n.c2 - n.c + 1;
  const at = kind === 'row' ? (before ? n.r : n.r2 + 1) : (before ? n.c : n.c2 + 1);
  S.change('all', () => {
    if (kind === 'row') S.hf.addRows(S.sid(), [at, Math.min(count, 1000)]); else S.hf.addColumns(S.sid(), [at, Math.min(count, 200)]);
    shiftMeta(S.meta(), kind, at, Math.min(count, kind === 'row' ? 1000 : 200));
    S.applyFilter();
  });
};
S.remove = (kind) => {
  const n = norm(S.sel); const u = S.used();
  const at = kind === 'row' ? n.r : n.c;
  const count = kind === 'row' ? Math.min(n.r2, Math.max(u.rows, n.r + 1) - 1) - n.r + 1 : Math.min(n.c2, Math.max(u.cols, n.c + 1) - 1) - n.c + 1;
  if (count <= 0) return;
  S.change('all', () => {
    if (kind === 'row') S.hf.removeRows(S.sid(), [at, count]); else S.hf.removeColumns(S.sid(), [at, count]);
    shiftMeta(S.meta(), kind, at, -count);
    S.applyFilter();
  });
  S.select(n.r, n.c);
};
S.tabs = () => {
  const box = $('#tabs'); if (!box) return;
  box.innerHTML = '';
  S.sheets.forEach((s, i) => {
    const t = el('button', { class: 'stab f' + (i === S.cur ? ' on' : ''), text: s.name, title: 'Double-click to rename; right-click for more' });
    t.onclick = () => S.goSheet(i);
    t.ondblclick = () => S.renameSheet(i);
    t.oncontextmenu = (e) => { e.preventDefault(); S.goSheet(i); H.showMenu([
      { label: 'Rename…', run: () => S.renameSheet(i) }, { label: 'Duplicate', run: () => S.dupSheet(i) },
      { label: 'Move left', disabled: () => i === 0, run: () => S.moveSheet(i, -1) }, { label: 'Move right', disabled: () => i === S.sheets.length - 1, run: () => S.moveSheet(i, 1) },
      { sep: true }, { label: 'Delete sheet', disabled: () => S.sheets.length < 2, run: () => S.delSheet(i) }], e.clientX, e.clientY); };
    box.append(t);
  });
};
S.goSheet = (i) => { if (S.editing) S.commit(); S.cur = i; S.sel = S.sheets[i].sel || { r: 0, c: 0, r2: 0, c2: 0 }; S.scroll.scrollTop = 0; S.scroll.scrollLeft = 0; S.tabs(); S.layout(); S.draw(); S.updateBar(); S.focus(); };
S.uniqueName = (base) => { let n = base, i = 2; while (S.hf.getSheetId(n) !== undefined) n = `${base} ${i++}`; return n; };
S.addSheet = () => {
  S.change('all', () => {
    const name = S.uniqueName('Sheet' + (S.sheets.length + 1));
    S.hf.addSheet(name); S.sheets.push({ id: S.hf.getSheetId(name), name, meta: newMeta() });
  });
  S.goSheet(S.sheets.length - 1);
};
S.renameSheet = async (i) => {
  const s = S.sheets[i];
  const v = await H.prompt('Rename sheet', 'Sheet name', s.name);
  if (!v || v === s.name) return;
  if (/[\\/?*[\]:']/.test(v) || v.length > 31) return H.alert('Can\'t use that name', 'Sheet names can be up to 31 characters and can\'t have \\ / ? * [ ] : or \'.');
  if (S.hf.getSheetId(v) !== undefined) return H.alert('That name is taken', 'Another sheet already has that name.');
  S.change('all', () => { S.hf.renameSheet(s.id, v); s.name = v; });
  S.tabs();
};
S.dupSheet = (i) => {
  const s = S.sheets[i];
  S.change('all', () => {
    const name = S.uniqueName(s.name + ' copy');
    S.hf.addSheet(name); const id = S.hf.getSheetId(name);
    S.hf.setSheetContent(id, S.hf.getSheetSerialized(s.id));
    S.sheets.splice(i + 1, 0, { id, name, meta: cloneMeta(s.meta) });
  });
  S.goSheet(i + 1);
};
S.moveSheet = (i, d) => { S.change('all', () => { const [s] = S.sheets.splice(i, 1); S.sheets.splice(i + d, 0, s); }); S.goSheet(i + d); };
S.delSheet = async (i) => {
  if (S.sheets.length < 2) return;
  if (!(await H.confirm('Delete this sheet?', `"${S.sheets[i].name}" and everything on it will be deleted. You can undo this.`, 'Delete', true))) return;
  S.change('all', () => { S.hf.removeSheet(S.sheets[i].id); S.sheets.splice(i, 1); });
  S.goSheet(Math.max(0, i - 1));
};

// ------------------------------------------------------------------ sort and filter
// The block of data around the selection (to the first empty row and column), when one cell is selected.
S.region = () => {
  const n = norm(S.sel);
  if (n.r !== n.r2 || n.c !== n.c2) { const u = S.used(); return { r: n.r, c: n.c, r2: Math.min(n.r2, u.rows - 1), c2: Math.min(n.c2, u.cols - 1) }; }
  const has = (r, c) => r >= 0 && c >= 0 && S.val(r, c) !== null && S.val(r, c) !== '';
  let g = { r: n.r, c: n.c, r2: n.r, c2: n.c }, grew = true, guard = 0;
  while (grew && guard++ < 500) {
    grew = false;
    for (let c = g.c; c <= g.c2; c++) { if (has(g.r - 1, c)) { g.r--; grew = true; break; } }
    for (let c = g.c; c <= g.c2; c++) { if (has(g.r2 + 1, c)) { g.r2++; grew = true; break; } }
    for (let r = g.r; r <= g.r2; r++) { if (has(r, g.c - 1)) { g.c--; grew = true; break; } }
    for (let r = g.r; r <= g.r2; r++) { if (has(r, g.c2 + 1)) { g.c2++; grew = true; break; } }
  }
  return g;
};
const looksLikeHeader = (g) => { if (g.r2 <= g.r) return false; let text = 0, numBelow = 0; for (let c = g.c; c <= g.c2; c++) { if (typeof S.val(g.r, c) === 'string') text++; if (typeof S.val(g.r + 1, c) === 'number') numBelow++; } return text === g.c2 - g.c + 1 && (numBelow > 0 || g.c2 === g.c ? true : true); };
const cmp = (a, b) => {
  const ea = a === null || a === '', eb = b === null || b === '';
  if (ea || eb) return ea && eb ? 0 : ea ? 1 : -1;
  const ta = typeof a === 'number' ? 0 : typeof a === 'string' ? 1 : 2, tb = typeof b === 'number' ? 0 : typeof b === 'string' ? 1 : 2;
  if (ta !== tb) return ta - tb;
  if (ta === 0) return a - b;
  return String(a).localeCompare(String(b), undefined, { numeric: true, sensitivity: 'base' });
};
S.sortQuick = (dir) => { const g = S.region(); S.sort(g, [{ c: S.sel.c, dir }], looksLikeHeader(g)); };
S.sortDialog = async () => {
  const g = S.region();
  const hdr = looksLikeHeader(g);
  const cols = range(g.c, g.c2).map((c) => [c, hdr ? `${colName(c)}: ${S.text(g.r, c)}` : `Column ${colName(c)}`]);
  const v = await H.form(`Sort ${addr(g.r, g.c)}:${addr(g.r2, g.c2)}`, [
    { id: 'header', label: 'The first row is a header (it stays at the top)', type: 'check', value: hdr },
    { id: 'c1', label: 'Sort by', type: 'select', value: S.sel.c, options: cols, half: true }, { id: 'd1', label: 'Order', type: 'select', value: '1', options: [['1', 'A → Z, smallest first'], ['-1', 'Z → A, largest first']], half: true },
    { id: 'c2', label: 'Then by', type: 'select', value: '', options: [['', '(nothing)'], ...cols], half: true }, { id: 'd2', label: 'Order', type: 'select', value: '1', options: [['1', 'A → Z'], ['-1', 'Z → A']], half: true },
  ], 'Sort');
  if (!v) return;
  const keys = [{ c: +v.c1, dir: +v.d1 }]; if (v.c2 !== '') keys.push({ c: +v.c2, dir: +v.d2 });
  S.sort(g, keys, v.header);
};
// Sorting moves whole rows. Formulas move with their row (references to their own row follow), through a
// scratch sheet so HyperFormula moves the references the way copy and paste does.
S.sort = (g, keys, header) => {
  const r0 = header ? g.r + 1 : g.r;
  if (r0 >= g.r2 + (header ? 1 : 0) && r0 > g.r2) return;
  const sid = S.sid(), m = S.meta();
  const rows = range(r0, g.r2).map((r) => ({ r, vals: keys.map((k) => S.val(r, k.c, sid)) }));
  rows.sort((a, b) => { for (let i = 0; i < keys.length; i++) { const x = cmp(a.vals[i], b.vals[i]); if (x) return keys[i].dir * x * ((a.vals[i] === null || a.vals[i] === '' || b.vals[i] === null || b.vals[i] === '') ? keys[i].dir : 1); } return a.r - b.r; });
  S.change('all', () => {
    const tmp = S.uniqueName('__hearth_sort');
    S.hf.addSheet(tmp); const tid = S.hf.getSheetId(tmp);
    const w = g.c2 - g.c;
    rows.forEach((row, i) => { S.hf.copy({ start: { sheet: sid, row: row.r, col: g.c }, end: { sheet: sid, row: row.r, col: g.c2 } }); S.hf.paste({ sheet: tid, row: i, col: g.c }); });
    S.hf.copy({ start: { sheet: tid, row: 0, col: g.c }, end: { sheet: tid, row: rows.length - 1, col: g.c + w } });
    S.hf.paste({ sheet: sid, row: r0, col: g.c });
    S.hf.removeSheet(tid);
    const fm = {}; rows.forEach((row, i) => { for (let c = g.c; c <= g.c2; c++) { const f = m.fmt[key(row.r, c)]; if (f) fm[key(r0 + i, c)] = f; } });
    for (let r = r0; r <= g.r2; r++) for (let c = g.c; c <= g.c2; c++) delete m.fmt[key(r, c)];
    Object.assign(m.fmt, fm);
  });
  H.toast(`Sorted ${rows.length} rows`);
};
S.toggleFilter = () => {
  const m = S.meta();
  if (m.filter) { S.metaChange(() => { m.filter = null; m.hiddenRows = {}; }); return; }
  const g = S.region();
  if (g.r2 <= g.r) { H.toast('Select a table with a header row first'); return; }
  S.metaChange(() => { m.filter = { r: g.r, c: g.c, r2: g.r2, c2: g.c2, crit: {} }; S.applyFilter(); });
};
S.applyFilter = () => {
  const m = S.meta(); m.hiddenRows = {};
  const f = m.filter; if (!f) return;
  const u = S.used(); f.r2 = Math.max(f.r2, Math.min(u.rows - 1, f.r2 + 1000));
  for (let r = f.r + 1; r <= f.r2; r++) for (const [c, hide] of Object.entries(f.crit || {})) if (hide && hide.length && hide.includes(S.text(r, +c))) { m.hiddenRows[r] = true; break; }
};
S.filterMenu = (c, x, y) => {
  const m = S.meta(), f = m.filter;
  const vals = new Map(); for (let r = f.r + 1; r <= f.r2; r++) { const t = S.text(r, c); vals.set(t, (vals.get(t) || 0) + 1); }
  const hidden = new Set((f.crit && f.crit[c]) || []);
  const list = Array.from(vals.keys()).sort((a, b) => cmp(a === '' ? null : isNaN(+a) ? a : +a, b === '' ? null : isNaN(+b) ? b : +b));
  H.closeMenus();
  const box = el('div', { class: 'menu filterbox' });
  const sortA = el('button', { class: 'mi wide', text: 'Sort A → Z', onclick: () => { H.closeMenus(); S.sort({ r: f.r, c: f.c, r2: f.r2, c2: f.c2 }, [{ c, dir: 1 }], true); } });
  const sortZ = el('button', { class: 'mi wide', text: 'Sort Z → A', onclick: () => { H.closeMenus(); S.sort({ r: f.r, c: f.c, r2: f.r2, c2: f.c2 }, [{ c, dir: -1 }], true); } });
  const search = el('input', { class: 'fsearch', placeholder: 'Search' });
  const items = el('div', { class: 'fitems' });
  const all = el('label', { class: 'chk' }, el('input', { type: 'checkbox', checked: hidden.size === 0 }), el('span', { text: '(Select all)' }));
  items.append(all);
  const boxes = list.map((v) => { const cb = el('input', { type: 'checkbox', checked: !hidden.has(v) }); const l = el('label', { class: 'chk' }, cb, el('span', { text: v === '' ? '(Blanks)' : v })); l.dataset.v = v; items.append(l); return [v, cb, l]; });
  all.querySelector('input').onchange = (e) => boxes.forEach(([, cb, l]) => { if (!l.hidden) cb.checked = e.target.checked; });
  search.oninput = () => { const q = search.value.toLowerCase(); boxes.forEach(([v, , l]) => { l.hidden = !!q && !v.toLowerCase().includes(q); }); };
  const ok = el('button', { class: 'btn primary', text: 'OK', onclick: () => { const hide = boxes.filter(([, cb]) => !cb.checked).map(([v]) => v); H.closeMenus(); S.metaChange(() => { f.crit = { ...(f.crit || {}), [c]: hide }; S.applyFilter(); }); } });
  const clear = el('button', { class: 'btn', text: 'Clear', onclick: () => { H.closeMenus(); S.metaChange(() => { f.crit = { ...(f.crit || {}), [c]: [] }; S.applyFilter(); }); } });
  box.append(sortA, sortZ, el('div', { class: 'msep' }), search, items, el('div', { class: 'frow' }, clear, ok));
  $('#layer').append(box);
  box.style.left = Math.min(x, innerWidth - 260) + 'px'; box.style.top = Math.min(y + 6, innerHeight - 380) + 'px';
  search.focus();
};

// ------------------------------------------------------------------ charts (a picture of the selected data)
S.chartDialog = async () => {
  const g = S.region();
  if (g.r2 <= g.r) return H.toast('Select the data for the chart first (labels in the first column, numbers next to them)');
  const v = await H.form('Chart', [{ id: 'type', label: 'Type', type: 'select', value: 'bar', options: [['bar', 'Bar chart'], ['line', 'Line chart'], ['pie', 'Pie chart']] }, { id: 'title', label: 'Title', value: looksLikeHeader(g) ? S.text(g.r, g.c + 1) : 'Chart' }], 'Show');
  if (!v) return;
  const hdr = looksLikeHeader(g);
  const labels = range(hdr ? g.r + 1 : g.r, g.r2).map((r) => S.text(r, g.c));
  const series = range(g.c + 1, g.c2).map((c) => ({ name: hdr ? S.text(g.r, c) : colName(c), data: range(hdr ? g.r + 1 : g.r, g.r2).map((r) => { const x = S.val(r, c); return typeof x === 'number' ? x : 0; }) }));
  if (!series.length) return H.toast('Put labels in the first column and numbers in the next ones');
  const svg = chartSvg(v.type, v.title, labels, series);
  const box = el('div', { class: 'chartbox', html: svg });
  const r2 = await H.dialog({ title: v.title || 'Chart', body: box, wide: true, buttons: [{ label: 'Close', value: null }, { label: 'Save as picture…', value: 'save', primary: true }] });
  if (r2 === 'save') {
    const p = await H.invoke('pick_save', { name: (v.title || 'chart') + '.svg', label: 'SVG picture', exts: ['svg'], dir: H.dirOf(H.doc.path) });
    if (p) { await H.invoke('write_file', { path: p, data: H.bytesToB64(H.textToBytes(svg)) }); H.toast('Saved the chart'); }
  }
};
function chartSvg(type, title, labels, series) {
  const W = 680, Hh = 400, pad = { l: 60, r: 20, t: 46, b: 70 };
  const colors = ['#2f5496', '#ff8a3d', '#3ccf7a', '#c00000', '#7030a0', '#00b0f0', '#ffc000', '#8a7a6c'];
  let s = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${W} ${Hh}" width="${W}" height="${Hh}" font-family="sans-serif" font-size="12"><rect width="${W}" height="${Hh}" fill="#fff"/><text x="${W / 2}" y="26" text-anchor="middle" font-size="16" font-weight="700" fill="#222">${esc(title)}</text>`;
  if (type === 'pie') {
    const data = series[0].data, total = data.reduce((a, b) => a + Math.max(0, b), 0) || 1;
    let a0 = -Math.PI / 2; const cx = 240, cy = 220, rr = 150;
    data.forEach((d, i) => {
      const a1 = a0 + Math.max(0, d) / total * Math.PI * 2;
      const x0 = cx + rr * Math.cos(a0), y0 = cy + rr * Math.sin(a0), x1 = cx + rr * Math.cos(a1), y1 = cy + rr * Math.sin(a1);
      s += `<path d="M${cx},${cy} L${x0.toFixed(1)},${y0.toFixed(1)} A${rr},${rr} 0 ${a1 - a0 > Math.PI ? 1 : 0} 1 ${x1.toFixed(1)},${y1.toFixed(1)} Z" fill="${colors[i % colors.length]}" stroke="#fff" stroke-width="2"/>`;
      s += `<rect x="440" y="${70 + i * 22}" width="14" height="14" fill="${colors[i % colors.length]}"/><text x="460" y="${82 + i * 22}" fill="#333">${esc(labels[i])} (${Math.round(Math.max(0, d) / total * 100)}%)</text>`;
      a0 = a1;
    });
    return s + '</svg>';
  }
  const all = series.flatMap((x) => x.data); const max = Math.max(0, ...all), min = Math.min(0, ...all);
  const span = max - min || 1; const pw = W - pad.l - pad.r, ph = Hh - pad.t - pad.b;
  const y = (v) => pad.t + ph - (v - min) / span * ph;
  for (let i = 0; i <= 5; i++) { const v = min + span * i / 5; s += `<line x1="${pad.l}" x2="${W - pad.r}" y1="${y(v).toFixed(1)}" y2="${y(v).toFixed(1)}" stroke="#e5e5e5"/><text x="${pad.l - 6}" y="${(y(v) + 4).toFixed(1)}" text-anchor="end" fill="#666">${Math.round(v * 100) / 100}</text>`; }
  const n = labels.length, bw = pw / n;
  labels.forEach((l, i) => { s += `<text x="${(pad.l + bw * i + bw / 2).toFixed(1)}" y="${Hh - pad.b + 18}" text-anchor="middle" fill="#444">${esc(String(l).slice(0, 14))}</text>`; });
  series.forEach((se, k) => {
    const col = colors[k % colors.length];
    if (type === 'line') s += `<polyline fill="none" stroke="${col}" stroke-width="2.5" points="${se.data.map((v, i) => `${(pad.l + bw * i + bw / 2).toFixed(1)},${y(v).toFixed(1)}`).join(' ')}"/>` + se.data.map((v, i) => `<circle cx="${(pad.l + bw * i + bw / 2).toFixed(1)}" cy="${y(v).toFixed(1)}" r="3.5" fill="${col}"/>`).join('');
    else { const gw = bw * 0.8 / series.length; se.data.forEach((v, i) => { const x = pad.l + bw * i + bw * 0.1 + gw * k; s += `<rect x="${x.toFixed(1)}" y="${Math.min(y(v), y(0)).toFixed(1)}" width="${(gw - 2).toFixed(1)}" height="${Math.abs(y(v) - y(0)).toFixed(1)}" fill="${col}"/>`; }); }
    s += `<rect x="${pad.l + k * 140}" y="${Hh - 26}" width="12" height="12" fill="${col}"/><text x="${pad.l + k * 140 + 18}" y="${Hh - 16}" fill="#333">${esc(se.name)}</text>`;
  });
  return s + '</svg>';
}

// ------------------------------------------------------------------ menus
S.contextMenu = (x, y, h) => {
  const col = h && h.hy && !h.hx, row = h && h.hx && !h.hy;
  const structure = col ? [
    { label: 'Insert column left', run: () => S.insert('col', true) }, { label: 'Insert column right', run: () => S.insert('col', false) },
    { label: 'Delete columns', run: () => S.remove('col') }, { label: 'Column width…', run: () => S.sizeDialog('col') }, { label: 'Fit column width', run: () => S.autoFit(h.c) },
  ] : row ? [
    { label: 'Insert row above', run: () => S.insert('row', true) }, { label: 'Insert row below', run: () => S.insert('row', false) },
    { label: 'Delete rows', run: () => S.remove('row') }, { label: 'Row height…', run: () => S.sizeDialog('row') },
  ] : [
    { label: 'Insert row above', run: () => S.insert('row', true) }, { label: 'Insert column left', run: () => S.insert('col', true) },
    { label: 'Delete rows', run: () => S.remove('row') }, { label: 'Delete columns', run: () => S.remove('col') },
  ];
  H.showMenu([
    { label: 'Cut', key: 'Ctrl+X', run: () => S.copy(null, true) }, { label: 'Copy', key: 'Ctrl+C', run: () => S.copy(null, false) }, { label: 'Paste', key: 'Ctrl+V', run: () => S.paste(null) }, { sep: true },
    ...structure, { sep: true },
    { label: 'Clear contents', key: 'Delete', run: () => S.clear('contents') }, { label: 'Clear formatting', run: () => S.clear('formats') }, { sep: true },
    { label: 'Sort A → Z', run: () => S.sortQuick(1) }, { label: 'Sort Z → A', run: () => S.sortQuick(-1) }, { label: 'Filter', run: () => S.toggleFilter() }, { sep: true },
    { label: 'Format cells…', key: 'Ctrl+1', run: () => S.formatDialog() },
  ], x, y);
};
S.sizeDialog = async (kind) => {
  const n = norm(S.sel); const m = S.meta();
  const cur = kind === 'col' ? (m.colW[n.c] ?? DW) : (m.rowH[n.r] ?? DH);
  const v = await H.prompt(kind === 'col' ? 'Column width' : 'Row height', kind === 'col' ? 'Width in pixels' : 'Height in pixels', String(cur), { type: 'number' });
  if (!v) return;
  S.metaChange(() => { if (kind === 'col') for (let c = n.c; c <= Math.min(n.c2, n.c + 500); c++) m.colW[c] = Math.max(8, +v); else for (let r = n.r; r <= Math.min(n.r2, n.r + 5000); r++) m.rowH[r] = Math.max(6, +v); });
};
S.formatDialog = async () => {
  const f = S.fmtAt(S.sel.r, S.sel.c) || {};
  const v = await H.form('Format cells', [
    { id: 'nf', label: 'Number format (Excel code)', value: f.nf || 'General', half: true },
    { id: 'align', label: 'Horizontal alignment', type: 'select', value: f.align || '', options: [['', 'Automatic'], ['left', 'Left'], ['center', 'Centre'], ['right', 'Right']], half: true },
    { id: 'valign', label: 'Vertical alignment', type: 'select', value: f.valign || '', options: [['', 'Bottom'], ['middle', 'Middle'], ['top', 'Top']], half: true },
    { id: 'size', label: 'Font size', type: 'number', value: f.size || 11, min: 6, max: 96, half: true },
    { id: 'b', label: 'Bold', type: 'check', value: f.b }, { id: 'i', label: 'Italic', type: 'check', value: f.i }, { id: 'u', label: 'Underline', type: 'check', value: f.u },
    { id: 'wrap', label: 'Wrap text', type: 'check', value: f.wrap },
    { id: 'color', label: 'Text colour', type: 'color', value: f.color || '#000000', half: true }, { id: 'bg', label: 'Fill colour', type: 'color', value: f.bg || '#ffffff', half: true },
  ], 'Apply');
  if (!v) return;
  S.setFmt({ nf: v.nf === 'General' || !v.nf ? null : v.nf, align: v.align || null, valign: v.valign || null, size: v.size && v.size !== 11 ? v.size : null, b: v.b, i: v.i, u: v.u, wrap: v.wrap, color: v.color === '#000000' ? null : v.color, bg: v.bg === '#ffffff' ? null : v.bg });
};
S.findDialog = async () => {
  const v = await H.form('Find and replace', [{ id: 'q', label: 'Find', value: S._lastFind || '' }, { id: 'rep', label: 'Replace with (leave empty to just find)', value: '' }, { id: 'all', label: 'Every sheet', type: 'check', value: false }], 'Find');
  if (!v || !v.q) return;
  S._lastFind = v.q;
  const q = v.q.toLowerCase();
  const sheets = v.all ? S.sheets.map((_, i) => i) : [S.cur];
  const hits = [];
  for (const i of sheets) { const s = S.sheets[i]; const u = S.used(s.id); for (let r = 0; r < u.rows; r++) for (let c = 0; c < u.cols; c++) { const t = String(S.ser(r, c, s.id) ?? ''); if (t.toLowerCase().includes(q)) hits.push([i, r, c, t]); } }
  if (!hits.length) return H.toast('Not found');
  if (v.rep !== '') {
    const re = new RegExp(v.q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'gi');
    S.change('all', () => { for (const [i, r, c, t] of hits) S.hf.setCellContents({ sheet: S.sheets[i].id, row: r, col: c }, [[S.parseInput(t.replace(re, v.rep)).v]]); });
    return H.toast(`Replaced in ${hits.length} cell${hits.length === 1 ? '' : 's'}`);
  }
  const after = hits.find(([i, r, c]) => i > S.cur || (i === S.cur && (r > S.sel.r || (r === S.sel.r && c > S.sel.c)))) || hits[0];
  if (after[0] !== S.cur) S.goSheet(after[0]);
  S.select(after[1], after[2]);
  H.toast(`${hits.length} found`);
};
S.menus = () => [
  { label: 'File', items: H.fileMenu([{ label: 'Import CSV into a new sheet…', run: () => S.importCsv() }]) },
  { label: 'Edit', items: [
    { label: 'Undo', key: 'Ctrl+Z', run: () => S.undo() }, { label: 'Redo', key: 'Ctrl+Y / Ctrl+Shift+Z', run: () => S.redo() }, { sep: true },
    { label: 'Cut', key: 'Ctrl+X', run: () => S.copy(null, true) }, { label: 'Copy', key: 'Ctrl+C', run: () => S.copy(null, false) }, { label: 'Paste', key: 'Ctrl+V', run: () => S.paste(null) },
    { label: 'Select all', key: 'Ctrl+A', run: () => S.select(0, 0, S.meta().rows - 1, S.meta().cols - 1) }, { sep: true },
    { label: 'Fill down', key: 'Ctrl+D', run: () => { const n = norm(S.sel); if (n.r2 > n.r) S.fill({ ...n, r2: n.r }, n); } },
    { label: 'Fill right', key: 'Ctrl+R', run: () => { const n = norm(S.sel); if (n.c2 > n.c) S.fill({ ...n, c2: n.c }, n); } },
    { label: 'Clear contents', run: () => S.clear('contents') }, { label: 'Clear formatting', run: () => S.clear('formats') }, { label: 'Clear all', run: () => S.clear('all') }, { sep: true },
    { label: 'Find and replace…', key: 'Ctrl+F / Ctrl+H', run: () => S.findDialog() },
  ] },
  { label: 'View', items: [
    { label: 'Freeze', sub: freezeItems() }, { sep: true },
    { label: 'Zoom in', key: 'Ctrl+=', run: () => S.zoom(S.zoomV + 10) }, { label: 'Zoom out', key: 'Ctrl+-', run: () => S.zoom(S.zoomV - 10) }, { label: 'Actual size', key: 'Ctrl+0', run: () => S.zoom(100) }, { sep: true },
    { label: 'Light window', checked: () => H.settings.theme === 'light', run: () => { H.settings.theme = H.settings.theme === 'light' ? 'dark' : 'light'; H.applyTheme(); H.saveSettings(); S.draw(); } },
  ] },
  { label: 'Insert', items: [
    { label: 'Row above', run: () => S.insert('row', true) }, { label: 'Row below', run: () => S.insert('row', false) },
    { label: 'Column left', run: () => S.insert('col', true) }, { label: 'Column right', run: () => S.insert('col', false) }, { sep: true },
    { label: 'Sheet', key: 'Shift+F11', run: () => S.addSheet() }, { label: 'Chart…', run: () => S.chartDialog() }, { sep: true },
    { label: 'Function', sub: ['SUM', 'AVERAGE', 'COUNT', 'COUNTA', 'MIN', 'MAX', 'IF', 'VLOOKUP', 'XLOOKUP', 'ROUND', 'SUMIF', 'COUNTIF', 'CONCATENATE', 'TODAY', 'NOW', 'TEXT', 'LEFT', 'RIGHT', 'LEN', 'TRIM', 'UPPER', 'DATE', 'YEAR', 'MONTH', 'DAY'].map((f) => ({ label: f, run: () => { S.startEdit('=' + f + '(', true); } })) },
    { label: 'Today\'s date', key: 'Ctrl+;', run: () => { const d = new Date(); S.startEdit(`${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`); S.commit(); } },
  ] },
  { label: 'Format', items: [
    { label: 'Format cells…', key: 'Ctrl+1', run: () => S.formatDialog() }, { sep: true },
    { label: 'Bold', key: 'Ctrl+B', run: () => S.toggle('b') }, { label: 'Italic', key: 'Ctrl+I', run: () => S.toggle('i') }, { label: 'Underline', key: 'Ctrl+U', run: () => S.toggle('u') },
    { label: 'Number format', sub: [['General', 'General'], ['0', 'Whole number'], ['#,##0.00', 'Number'], ['"$"#,##0.00', 'Currency ($)'], ['"€"#,##0.00', 'Currency (€)'], ['"£"#,##0.00', 'Currency (£)'], ['0%', 'Percent'], ['0.00%', 'Percent, 2 decimals'], ['yyyy-mm-dd', 'Date (2024-01-31)'], ['m/d/yyyy', 'Date (1/31/2024)'], ['d/m/yyyy', 'Date (31/1/2024)'], ['dddd, mmmm d, yyyy', 'Long date'], ['h:mm AM/PM', 'Time'], ['@', 'Text']].map(([v, l]) => ({ label: l, run: () => S.setFmt({ nf: v === 'General' ? null : v }) })).concat([{ label: 'Custom…', run: () => S.customFormat() }]) },
    { label: 'Borders', sub: borderItems() }, { label: 'Merge or unmerge cells', run: () => S.merge() }, { label: 'Wrap text', run: () => S.toggle('wrap') }, { sep: true },
    { label: 'Column width…', run: () => S.sizeDialog('col') }, { label: 'Row height…', run: () => S.sizeDialog('row') },
    { label: 'Fit column width', run: () => { const n = norm(S.sel); for (let c = n.c; c <= Math.min(n.c2, n.c + 50); c++) S.autoFit(c); } },
  ] },
  { label: 'Data', items: [
    { label: 'Sort A → Z', run: () => S.sortQuick(1) }, { label: 'Sort Z → A', run: () => S.sortQuick(-1) }, { label: 'Sort range…', run: () => S.sortDialog() }, { sep: true },
    { label: 'Filter', key: 'Ctrl+Shift+L', checked: () => !!S.meta().filter, run: () => S.toggleFilter() },
    { label: 'Remove duplicates…', run: () => S.dedupe() },
  ] },
  { label: 'Help', items: H.helpMenu([{ label: 'Functions Sheets knows', run: () => H.dialog({ title: `${S.hf.getRegisteredFunctionNames().length} functions`, wide: true, body: el('p', { class: 'fnlist', text: S.hf.getRegisteredFunctionNames().sort().join(', ') }) }) }, { label: 'What files keep what', run: () => S.formatHelp() }, { sep: true }]) },
];
S.extraKeys = () => ({ 'Ctrl+Shift+Z': { run: () => S.redo() }, 'Alt+=': { run: () => S.autoSum() }, 'Ctrl+H': { run: () => S.findDialog() }, 'Ctrl+PageDown': { run: () => S.goSheet((S.cur + 1) % S.sheets.length) }, 'Ctrl+PageUp': { run: () => S.goSheet((S.cur - 1 + S.sheets.length) % S.sheets.length) } });
S.formatHelp = () => H.dialog({ title: 'What each file type keeps', wide: true, body: `<div class="fmt">
  <p><b>.xlsx (Excel)</b>: every sheet, values, formulas (with their last results, so other programs show them straight away), number formats, fonts, bold/italic/underline/strikethrough, text and fill colours, borders, alignment, wrap, merged cells, column widths, row heights, frozen rows and columns, and the filter range. Not kept: charts, pictures, conditional formatting, data validation, comments, pivot tables and macros in files Sheets opens.</p>
  <p><b>.ods (OpenDocument)</b>: sheets, values, formulas, number formats, merged cells and column widths. Fonts, colours and borders aren't saved to .ods yet.</p>
  <p><b>.xls (Excel 97–2003)</b>: opened (values, formulas, number formats); saved as .xlsx.</p>
  <p><b>.csv / .tsv</b>: the values of the sheet you're on, as shown (formulas become their results).</p></div>` });
S.dedupe = async () => {
  const g = S.region(); const hdr = looksLikeHeader(g);
  if (!(await H.confirm('Remove duplicate rows?', `Rows in ${addr(g.r, g.c)}:${addr(g.r2, g.c2)} that are exactly the same as one above them will be removed${hdr ? ' (the header row stays)' : ''}.`, 'Remove'))) return;
  const seen = new Set(); const keep = [];
  for (let r = hdr ? g.r + 1 : g.r; r <= g.r2; r++) { const k = range(g.c, g.c2).map((c) => S.text(r, c)).join('\u0001'); if (!seen.has(k)) { seen.add(k); keep.push(r); } }
  const removed = (g.r2 - (hdr ? g.r + 1 : g.r) + 1) - keep.length;
  if (!removed) return H.toast('No duplicates');
  S.change(g, () => {
    const data = keep.map((r) => range(g.c, g.c2).map((c) => S.ser(r, c) ?? null));
    const start = hdr ? g.r + 1 : g.r;
    while (data.length < g.r2 - start + 1) data.push(range(g.c, g.c2).map(() => null));
    S.hf.setCellContents({ sheet: S.sid(), row: start, col: g.c }, data);
  });
  H.toast(`Removed ${removed} duplicate row${removed === 1 ? '' : 's'}`);
};

// ------------------------------------------------------------------ the controller
S.pad = (b) => {
  if ((H._dialogs && H._dialogs.length) || $$('.menu').length || !$('#start').hidden || S.editing) return false;
  const d = { Up: [-1, 0], Down: [1, 0], Left: [0, -1], Right: [0, 1] }[b];
  if (d) { S.move(...d); return true; }
  if (b === 'LB' || b === 'RB') { S.goSheet((S.cur + (b === 'RB' ? 1 : -1) + S.sheets.length) % S.sheets.length); return true; }
  if (b === 'A') { S.startEdit(null, true); return true; }
  return false;
};

// ------------------------------------------------------------------ new, open, save
S.templates = [
  { id: 'blank', name: 'Blank spreadsheet', thumb: '<div class="mgrid"></div>' },
  { id: 'budget', name: 'Monthly budget', desc: 'Income, spending, what\'s left', thumb: '<div class="mgrid t1"></div>' },
  { id: 'todo', name: 'To-do list', desc: 'Tasks, who, when, done', thumb: '<div class="mgrid t2"></div>' },
  { id: 'invoice', name: 'Invoice', desc: 'Items, quantities, totals', thumb: '<div class="mgrid t3"></div>' },
];
const TPL = {
  budget: { name: 'Budget', data: [['Monthly budget'], [], ['Income', 'Amount'], ['Salary', 2500], ['Other', 150], ['Total income', '=SUM(B4:B5)'], [], ['Spending', 'Amount'], ['Rent', 900], ['Food', 400], ['Transport', 120], ['Bills', 180], ['Fun', 150], ['Total spending', '=SUM(B9:B13)'], [], ['Left over', '=B6-B14']],
    fmt: { '0,0': { b: true, size: 16 }, '2,0': { b: true, bg: '#d9ead3' }, '2,1': { b: true, bg: '#d9ead3' }, '7,0': { b: true, bg: '#f4cccc' }, '7,1': { b: true, bg: '#f4cccc' }, '5,0': { b: true }, '13,0': { b: true }, '15,0': { b: true, size: 13 }, '15,1': { b: true, size: 13, nf: '"$"#,##0.00' } }, nfCol: { 1: '"$"#,##0.00' }, colW: { 0: 160, 1: 120 } },
  todo: { name: 'To do', data: [['Task', 'Who', 'Due', 'Done'], ['Plan the week', 'Me', '2026-11-02', 'no'], ['', '', '', '']], fmt: { '0,0': { b: true, bg: '#cfe2f3' }, '0,1': { b: true, bg: '#cfe2f3' }, '0,2': { b: true, bg: '#cfe2f3' }, '0,3': { b: true, bg: '#cfe2f3' } }, colW: { 0: 260, 1: 120, 2: 110, 3: 70 }, freeze: { r: 1, c: 0 } },
  invoice: { name: 'Invoice', data: [['INVOICE'], ['Number', 'INV-001'], ['Date', '=TODAY()'], [], ['Item', 'Qty', 'Price', 'Total'], ['Item one', 2, 25, '=B6*C6'], ['Item two', 1, 40, '=B7*C7'], ['', '', '', '=IF(B8="","",B8*C8)'], [], ['', '', 'Subtotal', '=SUM(D6:D8)'], ['', '', 'Tax (10%)', '=ROUND(D10*0.1,2)'], ['', '', 'Total', '=D10+D11']],
    fmt: { '0,0': { b: true, size: 18 }, '2,1': { nf: 'yyyy-mm-dd', align: 'left' }, '4,0': { b: true, bg: '#fce5cd' }, '4,1': { b: true, bg: '#fce5cd' }, '4,2': { b: true, bg: '#fce5cd' }, '4,3': { b: true, bg: '#fce5cd' }, '11,2': { b: true }, '11,3': { b: true, nf: '"$"#,##0.00', bd: { t: 'thin', b: 'thick' } } }, nfRange: [[5, 2, 11, 3, '"$"#,##0.00']], colW: { 0: 220, 2: 110, 3: 110 } },
};
S.reset = () => {
  if (!S.hf) S.makeEngine();
  for (const n of S.hf.getSheetNames()) S.hf.removeSheet(S.hf.getSheetId(n));
  S.sheets = []; S.cur = 0; U.stack = []; U.redo = []; S.clip = null;
};
S.addLoaded = (name, data, meta) => {
  const nm = S.uniqueName(name || 'Sheet1');
  S.hf.addSheet(nm); const id = S.hf.getSheetId(nm);
  if (data && data.length) S.hf.setSheetContent(id, data);
  S.sheets.push({ id, name: nm, meta: Object.assign(newMeta(), meta || {}) });
  return S.sheets[S.sheets.length - 1];
};
S.newDoc = (tpl) => {
  S.reset();
  const t = TPL[tpl];
  if (!t) S.addLoaded('Sheet1', null);
  else {
    const sh = S.addLoaded(t.name, t.data.map((r) => r.slice()), { colW: { ...(t.colW || {}) }, freeze: t.freeze || { r: 0, c: 0 } });
    sh.meta.fmt = JSON.parse(JSON.stringify(t.fmt || {}));
    if (t.nfCol) for (const [c, nf] of Object.entries(t.nfCol)) for (let r = 0; r < t.data.length; r++) if (typeof t.data[r][c] === 'number' || String(t.data[r][c] || '').startsWith('=')) sh.meta.fmt[key(r, c)] = { ...(sh.meta.fmt[key(r, c)] || {}), nf };
    if (t.nfRange) for (const [r1, c1, r2, c2, nf] of t.nfRange) for (let r = r1; r <= r2; r++) for (let c = c1; c <= c2; c++) sh.meta.fmt[key(r, c)] = { ...(sh.meta.fmt[key(r, c)] || {}), nf };
  }
  S.cur = 0; S.sel = { r: 0, c: 0, r2: 0, c2: 0 };
  S.tabs(); S.layout(); S.draw(); S.updateBar(); S.focus();
};
// CSV and TSV: quotes, doubled quotes, line breaks inside quotes; the separator is guessed (comma, semicolon or tab).
const parseDelimited = (text, sep) => {
  const rows = []; let row = [], cell = '', q = false, i = 0;
  while (i < text.length) {
    const ch = text[i];
    if (q) { if (ch === '"') { if (text[i + 1] === '"') { cell += '"'; i += 2; continue; } q = false; i++; continue; } cell += ch; i++; continue; }
    if (ch === '"' && cell === '') { q = true; i++; continue; }
    if (ch === sep) { row.push(cell); cell = ''; i++; continue; }
    if (ch === '\r') { i++; continue; }
    if (ch === '\n') { row.push(cell); rows.push(row); row = []; cell = ''; i++; continue; }
    cell += ch; i++;
  }
  if (cell !== '' || row.length) { row.push(cell); rows.push(row); }
  return rows;
};
const guessSep = (t) => { const line = t.split('\n').slice(0, 5).join('\n'); const c = { ',': 0, ';': 0, '\t': 0 }; let q = false; for (const ch of line) { if (ch === '"') q = !q; else if (!q && ch in c) c[ch]++; } return Object.entries(c).sort((a, b) => b[1] - a[1])[0][1] ? Object.entries(c).sort((a, b) => b[1] - a[1])[0][0] : ','; };
const csvData = (text, sep) => {
  const rows = parseDelimited(text.replace(/^﻿/, ''), sep || guessSep(text));
  return rows.map((r) => r.map((v) => { if (v === '') return null; const p = S.parseInput(v); return p.v; }));
};
S.importCsv = async () => {
  const p = await H.invoke('pick_open', { kind: 'csv' }); if (!p) return;
  const r = await H.invoke('read_file', { path: p });
  const text = H.bytesToText(H.b64ToBytes(r.data));
  S.change('all', () => { S.addLoaded(H.stem(p), csvData(text, H.extOf(p) === 'tsv' ? '\t' : null)); });
  S.goSheet(S.sheets.length - 1);
};
// A string from a file that HyperFormula would read as a number, date or formula stays text.
const asText = (s) => (/^[A-Za-zÀ-￿]/.test(s) && !/^(true|false)$/i.test(s) ? s : "'" + s);
S.load = async (bytes, ext) => {
  S.reset();
  const notes = [];
  if (ext === 'csv' || ext === 'tsv') {
    const text = H.bytesToText(bytes);
    S.addLoaded(H.stem(H.doc.name && H.doc.path ? H.doc.name : 'Sheet1') || 'Sheet1', csvData(text, ext === 'tsv' ? '\t' : null));
    // Fit the columns to what's in them.
    S.cur = 0; S.layout();
    const u = S.used(); for (let c = 0; c < Math.min(u.cols, 40); c++) { let w = 60; for (let r = 0; r < Math.min(u.rows, 300); r++) w = Math.max(w, S.text(r, c).length * 7.5 + 14); S.meta().colW[c] = Math.min(360, Math.round(w)); }
  } else {
    const wb = XLSX.read(bytes, { type: 'array', cellFormula: true, cellNF: true, cellStyles: true, cellDates: false, sheetStubs: false });
    let styles = null;
    if (ext === 'xlsx' || ext === 'xlsm') { try { await H.need('vendor/jszip.min.js', 'formats/xlsx-style.js'); styles = await SheetsStyle.read(bytes); } catch (e) { console.warn(e); H.invoke('log', { message: 'xlsx styles: ' + (e && e.stack || e) }); notes.push('Some cell formatting couldn\'t be read.'); } }
    for (const name of wb.SheetNames) {
      const ws = wb.Sheets[name];
      if (!ws || !ws['!ref']) { S.addLoaded(name, []); continue; }
      const rg = XLSX.utils.decode_range(ws['!ref']);
      const rows = Math.min(rg.e.r, 200000), cols = Math.min(rg.e.c, 2000);
      const data = []; const meta = newMeta();
      for (let r = 0; r <= rows; r++) {
        const row = [];
        for (let c = 0; c <= cols; c++) {
          const cell = ws[XLSX.utils.encode_cell({ r, c })];
          if (!cell) { row.push(null); continue; }
          let v = null;
          if (cell.f) v = '=' + cell.f.replace(/_xlfn\.|_xlws\./g, '');
          else if (cell.t === 'n') v = cell.v;
          else if (cell.t === 'b') v = !!cell.v;
          else if (cell.t === 's' || cell.t === 'str') v = cell.v === '' ? null : asText(String(cell.v));
          else if (cell.t === 'e') v = cell.w || '#N/A';
          else if (cell.t === 'd') v = cell.v instanceof Date ? (cell.v - new Date(Date.UTC(1899, 11, 30))) / 864e5 : null;
          row.push(v);
          if (cell.z && cell.z !== 'General' && !(typeof cell.z === 'number')) meta.fmt[key(r, c)] = { nf: String(cell.z) };
        }
        data.push(row);
      }
      for (const mg of ws['!merges'] || []) meta.merges.push({ r: mg.s.r, c: mg.s.c, r2: mg.e.r, c2: mg.e.c });
      (ws['!cols'] || []).forEach((cl, c) => { if (cl && (cl.wpx || cl.wch || cl.width)) meta.colW[c] = Math.round(cl.width ? cl.width * 7 + 5 : cl.wch ? cl.wch * 7 + 5 : cl.wpx); });
      (ws['!rows'] || []).forEach((rw, r) => { if (rw && (rw.hpx || rw.hpt)) meta.rowH[r] = Math.round(rw.hpx || rw.hpt * 4 / 3); });
      if (ws['!autofilter'] && ws['!autofilter'].ref) { const fr = XLSX.utils.decode_range(ws['!autofilter'].ref); meta.filter = { r: fr.s.r, c: fr.s.c, r2: fr.e.r, c2: fr.e.c, crit: {} }; }
      const st = styles && styles.sheets[name];
      if (st) {
        for (const [k, f] of Object.entries(st.fmt)) meta.fmt[k] = Object.assign({}, meta.fmt[k] || {}, f);
        if (st.freeze) meta.freeze = st.freeze;
      }
      meta.rows = Math.max(200, rows + 60); meta.cols = Math.max(26, cols + 6);
      S.addLoaded(name, data, meta);
    }
    if (!S.sheets.length) S.addLoaded('Sheet1', []);
    if (wb.Workbook && wb.Workbook.Names && wb.Workbook.Names.length) {
      for (const n of wb.Workbook.Names) { try { if (n.Name && n.Ref && !n.Name.startsWith('_xlnm')) S.hf.addNamedExpression(n.Name, '=' + n.Ref); } catch {} }
    }
    // Formulas that use functions HyperFormula doesn't have show #NAME?.
    let unknown = 0; for (const s of S.sheets) { const u = S.used(s.id); for (let r = 0; r < Math.min(u.rows, 5000); r++) for (let c = 0; c < u.cols; c++) { const v = S.val(r, c, s.id); if (v && typeof v === 'object' && v.type === 'NAME') unknown++; } }
    if (unknown) notes.push(`${unknown} formula${unknown === 1 ? ' uses' : 's use'} a function Sheets doesn't know (they show #NAME?).`);
  }
  S.cur = 0; S.sel = { r: 0, c: 0, r2: 0, c2: 0 };
  S.scroll.scrollTop = 0; S.scroll.scrollLeft = 0;
  for (const s of S.sheets) { const cur = S.cur; S.cur = S.sheets.indexOf(s); S.applyFilter(); S.cur = cur; }
  S.tabs(); S.layout(); S.draw(); S.updateBar(); S.focus();
  return notes;
};
const ERRCODE = { '#NULL!': 0x00, '#DIV/0!': 0x07, '#VALUE!': 0x0f, '#REF!': 0x17, '#NAME?': 0x1d, '#NUM!': 0x24, '#N/A': 0x2a };
S.toWorkbook = () => {
  const wb = XLSX.utils.book_new();
  for (const s of S.sheets) {
    const ws = {}; const u = S.used(s.id); const m = s.meta;
    let maxR = 0, maxC = 0;
    for (let r = 0; r < u.rows; r++) for (let c = 0; c < u.cols; c++) {
      const ser = S.ser(r, c, s.id); if (ser === null || ser === undefined || ser === '') continue;
      const v = S.val(r, c, s.id); const f = m.fmt[key(r, c)];
      const cell = {};
      if (typeof ser === 'string' && ser.startsWith('=')) cell.f = ser.slice(1).replace(/\bTRUE\(\)/g, 'TRUE').replace(/\bFALSE\(\)/g, 'FALSE');
      if (v && typeof v === 'object') { const code = v.value || ERRS[v.type]; cell.t = 'e'; cell.v = ERRCODE[code] ?? 0x0f; cell.w = code; }
      else if (typeof v === 'number') { cell.t = 'n'; cell.v = v; }
      else if (typeof v === 'boolean') { cell.t = 'b'; cell.v = v; }
      else { cell.t = cell.f ? 'str' : 's'; cell.v = v == null ? '' : String(v); }
      if (f && f.nf) cell.z = f.nf;
      ws[XLSX.utils.encode_cell({ r, c })] = cell;
      maxR = Math.max(maxR, r); maxC = Math.max(maxC, c);
    }
    for (const g of m.merges) { maxR = Math.max(maxR, g.r2); maxC = Math.max(maxC, g.c2); }
    for (const k of Object.keys(m.fmt)) { const [r, c] = k.split(',').map(Number); maxR = Math.max(maxR, r); maxC = Math.max(maxC, c); }
    ws['!ref'] = XLSX.utils.encode_range({ s: { r: 0, c: 0 }, e: { r: maxR, c: maxC } });
    if (m.merges.length) ws['!merges'] = m.merges.map((g) => ({ s: { r: g.r, c: g.c }, e: { r: g.r2, c: g.c2 } }));
    const cols = []; for (const [c, w] of Object.entries(m.colW)) cols[+c] = { width: Math.max(1, Math.round((w - 5) / 7 * 100) / 100) };
    for (let i = 0; i < cols.length; i++) if (!cols[i]) cols[i] = {};
    if (cols.length) ws['!cols'] = cols;
    const rows = []; for (const [r, h] of Object.entries(m.rowH)) rows[+r] = { hpx: h, hpt: h * 0.75 };
    for (const r of Object.keys(m.hiddenRows || {})) rows[+r] = { ...(rows[+r] || {}), hidden: true };
    for (let i = 0; i < rows.length; i++) if (!rows[i]) rows[i] = {};
    if (rows.length) ws['!rows'] = rows;
    if (m.filter) ws['!autofilter'] = { ref: XLSX.utils.encode_range({ s: { r: m.filter.r, c: m.filter.c }, e: { r: m.filter.r2, c: m.filter.c2 } }) };
    XLSX.utils.book_append_sheet(wb, ws, s.name);
  }
  return wb;
};
S.save = async (ext) => {
  if (S.editing) S.commit();
  if (ext === 'csv' || ext === 'tsv') {
    const sep = ext === 'tsv' ? '\t' : ',';
    const u = S.used(); const lines = [];
    for (let r = 0; r < u.rows; r++) {
      const row = [];
      for (let c = 0; c < u.cols; c++) {
        const v = S.val(r, c); const f = S.fmtAt(r, c);
        let t = typeof v === 'number' && !(f && f.nf && /[dmyhs]/i.test(f.nf.replace(/"[^"]*"/g, ''))) ? String(v) : S.display(v, f);
        if (t.includes(sep) || /["\n\r]/.test(t) || /^\s|\s$/.test(t)) t = '"' + t.replace(/"/g, '""') + '"';
        row.push(t);
      }
      lines.push(row.join(sep));
    }
    return H.textToBytes(lines.join('\r\n') + '\r\n');
  }
  const wb = S.toWorkbook();
  const out = XLSX.write(wb, { bookType: ext === 'ods' ? 'ods' : 'xlsx', type: 'array', compression: true });
  let bytes = new Uint8Array(out);
  if (ext === 'xlsx') {
    await H.need('vendor/jszip.min.js', 'formats/xlsx-style.js');
    bytes = await SheetsStyle.write(bytes, S.sheets.map((s) => ({ name: s.name, fmt: s.meta.fmt, freeze: s.meta.freeze })));
  }
  return bytes;
};
S.snapshot = () => JSON.stringify({ sheets: S.sheets.map((s) => ({ name: s.name, data: S.hf.getSheetSerialized(s.id), meta: s.meta })), cur: S.cur });
S.restore = (txt) => {
  const o = JSON.parse(txt);
  S.reset();
  for (const s of o.sheets) S.addLoaded(s.name, s.data, s.meta);
  S.cur = Math.min(o.cur || 0, S.sheets.length - 1);
  S.tabs(); S.layout(); S.draw(); S.updateBar(); S.focus();
};
// Print layout: the used range as tables, split into pages of rows (landscape A4/Letter), every sheet.
S.printLayout = async (box) => {
  const letter = /^en-(US|CA)/.test(navigator.language || '');
  const [w, h] = letter ? [279.4, 215.9] : [297, 210];
  for (const s of S.sheets) {
    const u = S.used(s.id); if (!u.rows) continue;
    const m = s.meta;
    const maxW = (w - 20) / 25.4 * 96;
    // Columns that fit across a page.
    const colGroups = []; let cur = [], acc = 0;
    for (let c = 0; c < u.cols; c++) { const cw = m.colW[c] ?? DW; if (acc + cw > maxW && cur.length) { colGroups.push(cur); cur = []; acc = 0; } cur.push(c); acc += cw; }
    if (cur.length) colGroups.push(cur);
    const rowsPerPage = Math.floor(((h - 24) / 25.4 * 96) / 22);
    for (const cols of colGroups) for (let r0 = 0; r0 < u.rows; r0 += rowsPerPage) {
      const pg = el('div', { class: 'ppage sheetpage', style: { width: w + 'mm', height: h + 'mm', padding: '10mm' } });
      pg.append(el('div', { class: 'sph', text: `${s.name}` }));
      const t = el('table', { class: 'ptable' });
      const cg = el('colgroup'); cols.forEach((c) => cg.append(el('col', { style: { width: (m.colW[c] ?? DW) + 'px' } }))); t.append(cg);
      for (let r = r0; r < Math.min(u.rows, r0 + rowsPerPage); r++) {
        if (m.hiddenRows[r]) continue;
        const tr = el('tr');
        for (const c of cols) {
          const f = m.fmt[key(r, c)] || {}; const v = S.val(r, c, s.id);
          const td = el('td', { text: S.display(v, f) });
          const st = td.style;
          if (f.b) st.fontWeight = '700'; if (f.i) st.fontStyle = 'italic'; if (f.u) st.textDecoration = 'underline'; if (f.color) st.color = f.color; if (f.bg) st.background = f.bg;
          st.textAlign = f.align || (typeof v === 'number' ? 'right' : 'left'); if (f.size) st.fontSize = f.size + 'pt';
          if (f.bd) for (const [sd, prop] of [['t', 'borderTop'], ['r', 'borderRight'], ['b', 'borderBottom'], ['l', 'borderLeft']]) if (f.bd[sd]) st[prop] = (f.bd[sd] === 'thick' ? '2px' : '1px') + ' solid #000';
          if (f.wrap) st.whiteSpace = 'normal';
          tr.append(td);
        }
        t.append(tr);
      }
      pg.append(t);
      box.append(pg);
    }
  }
  if (!box.children.length) box.append(el('div', { class: 'ppage', style: { width: w + 'mm', height: h + 'mm' } }));
  return { w, h };
};

S.parseDelimited = parseDelimited; S.colIndex = colIndex; S.parseAddr = parseAddr;
H.register(S);
})();
