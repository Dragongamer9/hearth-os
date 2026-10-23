// Hearth Text's window: tabs of files, each one a CodeMirror editor (editor.js), with find and replace, go to line,
// encodings and line endings, read-only views of big or protected files, and Edit as administrator on Linux. The
// Rust side (src/main.rs) reads and writes the files. Works with a keyboard, a mouse, touch and a controller (the
// controller arrives as keys: D-pad = arrows, A = Enter, B = Esc; F10 or the Menu key opens the menu).
import * as E from './editor.js';

const T = window.__TAURI__;
const invoke = (cmd, args) => T.core.invoke(cmd, args);
const $ = id => document.getElementById(id);

// ------------------------------------------------------------------ themes
const THEMES = {
  ember: { name: 'Ember', vars: {} },
  night: {
    name: 'Night',
    vars: {
      bg: '#0c1016', chrome: '#121821', panel: '#161d27', panel2: '#1d2531', panel3: '#273140', line: '#2a3443', text: '#e9eef4', muted: '#8d99a8',
      accent: '#3fd0b8', accentInk: '#06231e', edbg: '#0e131a', actline: 'rgba(63,208,184,.06)', sel: 'rgba(63,208,184,.28)', selmatch: 'rgba(63,208,184,.14)',
      match: 'rgba(255,181,71,.25)', 'match-now': 'rgba(63,208,184,.5)',
      'syn-kw': '#c792ea', 'syn-str': '#a5d86e', 'syn-esc': '#89ddff', 'syn-num': '#f78c6c', 'syn-cm': '#5d6978', 'syn-fn': '#82aaff', 'syn-type': '#ffcb6b',
      'syn-prop': '#80cbc4', 'syn-tag': '#f07178', 'syn-head': '#82aaff', 'syn-link': '#89ddff', 'syn-meta': '#c3a6ff', 'syn-err': '#ff6b6b',
      'syn-ins': '#a5d86e', 'syn-del': '#ff6b6b', 'syn-def': '#e9eef4', 'syn-op': '#a6b3c2',
    },
  },
  daylight: {
    name: 'Daylight', light: true,
    vars: {
      bg: '#f7f1ea', chrome: '#efe6dc', panel: '#fbf7f2', panel2: '#e8ddd1', panel3: '#ddcfc0', line: '#d6c8b9', text: '#2a1d15', muted: '#76655a',
      accent: '#d9601a', accentInk: '#ffffff', warn: '#c58300', edbg: '#fffcf8', actline: 'rgba(217,96,26,.06)', sel: 'rgba(217,96,26,.22)', selmatch: 'rgba(217,96,26,.12)',
      match: 'rgba(197,131,0,.25)', 'match-now': 'rgba(217,96,26,.4)',
      'syn-kw': '#b4450f', 'syn-str': '#3b7a2a', 'syn-esc': '#13867c', 'syn-num': '#9a5b00', 'syn-cm': '#8f7f72', 'syn-fn': '#2463b5', 'syn-type': '#8e3a87',
      'syn-prop': '#7a4b16', 'syn-tag': '#c4382c', 'syn-head': '#b4450f', 'syn-link': '#2463b5', 'syn-meta': '#6b4fbf', 'syn-err': '#c4382c',
      'syn-ins': '#3b7a2a', 'syn-del': '#c4382c', 'syn-def': '#2a1d15', 'syn-op': '#5c4b40',
    },
  },
};
const DEFAULTS = { fontSize: 15, theme: 'ember', wrap: false, lineNumbers: true, recent: [] };
const ENCODINGS = [['utf-8', 'UTF-8'], ['utf-16le', 'UTF-16 LE'], ['utf-16be', 'UTF-16 BE'], ['latin-1', 'Latin-1 (ISO-8859-1)']];
const EOLS = [['lf', 'LF (Linux, macOS)'], ['crlf', 'CRLF (Windows)'], ['cr', 'CR (old Mac)']];
const BIG_TEXT = 2 * 1024 * 1024;   // above this many characters: no colors or bracket matching (stays quick)

let S = { ...DEFAULTS };
let INFO = { platform: 'linux', admin: false, eol: 'lf', limits: {} };
const tabs = [];
let current = null;
let tabSeq = 0;

// ------------------------------------------------------------------ small helpers
function toast(text, ms = 2600) {
  const el = $('toast');
  el.textContent = text;
  el.classList.add('show');
  clearTimeout(toast.timer);
  toast.timer = setTimeout(() => el.classList.remove('show'), ms);
}
const esc = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
const baseName = p => (p || '').split(/[\\/]/).filter(Boolean).pop() || p;
const dirName = p => { const i = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\')); return i > 0 ? p.slice(0, i) : (i === 0 ? '/' : ''); };
function tidyPath(p) {
  const home = INFO.home;
  if (home && p && (p === home || p.startsWith(home + '/') || p.startsWith(home + '\\'))) return '~' + p.slice(home.length);
  return p;
}
function fmtSize(n) {
  if (n < 1024) return n + ' bytes';
  const u = ['KB', 'MB', 'GB', 'TB'];
  let i = -1;
  do { n /= 1024; i++; } while (n >= 1024 && i < u.length - 1);
  return (n >= 10 ? n.toFixed(0) : n.toFixed(1)) + ' ' + u[i];
}
let saveTimer = 0;
function saveSettings() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => invoke('settings_save', { value: S }).catch(e => toast("Settings weren't saved: " + e)), 300);
}
function setTitle(text) {
  document.title = text;
  try { T.window.getCurrentWindow().setTitle(text); } catch (_) { /* not in the app (tests) */ }
}

function applyTheme() {
  const th = THEMES[S.theme] || THEMES.ember, r = document.documentElement;
  for (const k of new Set([...Object.keys(THEMES.night.vars), ...Object.keys(THEMES.daylight.vars)])) r.style.removeProperty('--' + k);
  for (const [k, v] of Object.entries(th.vars)) r.style.setProperty('--' + k, v);
  r.classList.toggle('light', !!th.light);
  r.style.setProperty('--fs', S.fontSize + 'px');
  for (const t of tabs) t.ed.view.requestMeasure();
}

// ------------------------------------------------------------------ tabs
function untitledName() {
  const used = new Set(tabs.filter(t => !t.path).map(t => t.name));
  for (let i = 1; ; i++) { const n = i === 1 ? 'Untitled' : 'Untitled ' + i; if (!used.has(n)) return n; }
}

/// A new tab. `f`: what open_file returned (or nothing for an empty one), plus opts {readonly, line}.
function makeTab(f, opts = {}) {
  const el = document.createElement('div');
  el.className = 'ed';
  el.hidden = true;
  $('editors').appendChild(el);
  const text = f ? f.text : '';
  const big = text.length > BIG_TEXT;
  const tab = {
    id: ++tabSeq, el, path: f ? f.path : null, name: f ? f.name : untitledName(),
    encoding: f ? f.encoding : 'utf-8', bom: f ? f.bom : false, eol: f ? f.eol : INFO.eol, mixedEol: f ? f.mixed_eol : false,
    mtime: f ? f.mtime : 0, size: f ? f.size : 0, writable: f ? f.writable : true, binary: !!(f && f.binary), lossy: !!(f && f.lossy),
    tooBig: !!(f && f.too_big), part: f ? f.part : null, shown: f ? f.shown : 0, admin: false, askedView: !!opts.readonly,
    big, metaDirty: false, dirty: false, changedOnDisk: false, gone: false,
    indent: E.detectIndent(text), lang: f ? E.detectLanguage(f.name, text.slice(0, 200).split('\n')[0]) : 'text',
  };
  tab.readonly = isReadOnly(tab);
  tab.ed = E.createEditor(el, {
    doc: text, lang: tab.lang, readOnly: tab.readonly, wrap: S.wrap, lineNumbers: S.lineNumbers, big, indent: tab.indent,
    onChange: () => { markDirty(tab); if (findOpen()) refreshFindCount(); },
    onCursor: () => { if (tab === current) updateStatus(); },
  });
  tab.saved = tab.ed.view.state.doc;
  tabs.push(tab);
  return tab;
}

function isReadOnly(t) {
  if (t.admin) return false;
  return t.binary || t.tooBig || !!t.part || t.askedView || (!!t.path && !t.writable);
}

/// Whether the tab has changes that aren't saved, worked out now (markDirty updates the dot a moment later).
function isDirty(t) {
  t.dirty = t.metaDirty || !t.ed.view.state.doc.eq(t.saved);
  return t.dirty;
}

function markDirty(tab) {
  clearTimeout(tab.dirtyTimer);
  tab.dirtyTimer = setTimeout(() => {
    const d = tab.metaDirty || !tab.ed.view.state.doc.eq(tab.saved);
    if (d !== tab.dirty) { tab.dirty = d; renderTabs(); updateChrome(); }
    if (tab === current) $('start').hidden = !showStart();
  }, 120);
  if (tab === current) $('start').hidden = true;
}

function activate(tab, focus = true) {
  if (!tab) return;
  if (current && current !== tab) {
    current.el.hidden = true;
    if (findOpen()) E.stopFind(current.ed.view);
    rememberLine(current);
  }
  current = tab;
  tab.el.hidden = false;
  tab.ed.view.requestMeasure();
  renderTabs();
  updateChrome();
  if (findOpen()) runFind();
  if (focus && $('shade').hidden && $('menu').hidden) tab.ed.focus();
  const t = $('tabs').querySelector(`[data-id="${tab.id}"]`);
  if (t) t.scrollIntoView({ block: 'nearest', inline: 'nearest' });
}

function renderTabs() {
  const box = $('tabs');
  box.innerHTML = '';
  for (const t of tabs) {
    const b = document.createElement('div');
    b.className = 'tab' + (t === current ? ' active' : '') + (t.dirty ? ' dirty' : '') + (t.readonly ? ' ro' : '');
    b.dataset.id = t.id;
    b.setAttribute('role', 'tab');
    b.title = (t.path ? tidyPath(t.path) : t.name) + (t.dirty ? ' (not saved)' : '') + (t.readonly ? ' (read-only)' : '');
    b.innerHTML = `<button class="tt" data-f>${esc(t.name)}</button><span class="dot" aria-label="Not saved"></span>`
      + `<button class="tx" data-f aria-label="Close ${esc(t.name)}" title="Close (Ctrl+W)"><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></svg></button>`;
    b.querySelector('.tt').onclick = () => activate(t);
    b.querySelector('.tt').onauxclick = e => { if (e.button === 1) closeTab(t); };
    b.querySelector('.tx').onclick = e => { e.stopPropagation(); closeTab(t); };
    box.appendChild(b);
  }
}

const showStart = () => !!current && !current.path && !current.dirty && current.ed.view.state.doc.length === 0;

// ------------------------------------------------------------------ the bars around the editor
function updateChrome() {
  const t = current;
  if (!t) return;
  setTitle((t.dirty ? '• ' : '') + t.name + (t.readonly ? ' (read-only)' : '') + ' — Text');
  $('btnSave').disabled = t.readonly && !!t.path;
  updateStatus();
  updateBanner();
  $('start').hidden = !showStart();
  if (showStart()) renderRecent();
}

function updateStatus() {
  const t = current;
  if (!t) return;
  const c = E.cursorInfo(t.ed.view);
  $('stPos').textContent = `Ln ${c.line}, Col ${c.col}` + (c.selected ? ` (${c.selected} selected)` : '') + (c.cursors > 1 ? ` · ${c.cursors} cursors` : '');
  $('stLang').textContent = E.languageName(t.lang) + (t.big ? ' (no colors: big file)' : '');
  $('stIndent').textContent = t.indent.tabs ? `Tabs: ${t.indent.width}` : `Spaces: ${t.indent.width}`;
  $('stEnc').textContent = (ENCODINGS.find(e => e[0] === t.encoding) || [0, t.encoding])[1].replace(/ \(.*\)/, '') + (t.bom ? ' with BOM' : '');
  $('stEol').textContent = t.eol.toUpperCase() + (t.mixedEol ? ' (mixed)' : '');
  $('stZoom').textContent = Math.round(S.fontSize / DEFAULTS.fontSize * 100) + '%';
  $('stRO').hidden = !t.readonly;
  $('stAdmin').hidden = !t.admin;
  $('btnUndo').disabled = !E.canUndo(t.ed.view);
  $('btnRedo').disabled = !E.canRedo(t.ed.view);
}

/// The note above the editor: why it's read-only and what you can do, a file changed on disk, odd bytes...
function updateBanner() {
  const t = current, bn = $('banner'), buttons = [];
  let text = '', kind = 'info';
  const btn = (label, run, primary) => buttons.push({ label, run, primary });
  if (t.gone) {
    kind = 'warn';
    text = `“${t.name}” isn't on the disk any more. Save it to keep it.`;
    btn('Save', () => save(t), true);
  } else if (t.changedOnDisk) {
    kind = 'warn';
    text = `“${t.name}” was changed by another program.` + (t.dirty ? ' Reloading loses your changes here.' : '');
    btn('Reload', () => reload(t), true);
    btn('Keep mine', () => { t.changedOnDisk = false; updateChrome(); });
  } else if (t.admin) {
    kind = 'warn';
    text = 'Editing as administrator. Saving asks for an administrator\'s password. Changing system files can stop your computer from starting, so be careful.';
  } else if (t.part) {
    text = `This file is ${fmtSize(t.size)}: only its ${t.part === 'end' ? 'last' : 'first'} ${fmtSize(t.shown)} ${t.part === 'end' ? 'are' : 'are'} shown, read-only.`;
    btn(t.part === 'end' ? 'Show the start' : 'Show the end', () => reload(t, { mode: t.part === 'end' ? 'auto' : 'end' }), true);
  } else if (t.binary) {
    text = 'This file isn\'t text, so it\'s shown read-only (bytes that aren\'t letters show as dots).';
  } else if (t.tooBig) {
    text = `This file is ${fmtSize(t.size)}, too big to edit here, so it's shown read-only.`;
  } else if (t.path && !t.writable) {
    text = 'You don\'t have permission to change this file, so it\'s shown read-only.';
    if (INFO.admin) btn('Edit as administrator', () => editAsAdmin(t), true);
    btn('Save a copy…', () => save(t, { as: true }));
  } else if (t.askedView) {
    text = 'Opened read-only.';
    btn('Edit', () => { t.askedView = false; setReadOnly(t); }, true);
  } else if (t.lossy) {
    kind = 'warn';
    text = `Some bytes in this file aren't valid ${t.encoding.toUpperCase()}; saving would change them. Reopen it with another encoding?`;
    btn('Reopen as Latin-1', () => reload(t, { encoding: 'latin-1' }), true);
  } else if (t.mixedEol) {
    text = `Lines in this file end in different ways. Saving makes them all ${t.eol.toUpperCase()}.`;
    btn('OK', () => { t.mixedEol = false; updateChrome(); });
  }
  bn.hidden = !text;
  bn.className = kind;
  $('bannerText').textContent = text;
  const box = $('bannerButtons');
  box.innerHTML = '';
  for (const b of buttons) {
    const e = document.createElement('button');
    e.className = 'pbtn' + (b.primary ? ' primary' : '');
    e.dataset.f = '';
    e.textContent = b.label;
    e.onclick = b.run;
    box.appendChild(e);
  }
}

function setReadOnly(t) {
  t.readonly = isReadOnly(t);
  t.ed.setReadOnly(t.readonly);
  renderTabs();
  updateChrome();
  t.ed.focus();
}

async function editAsAdmin(t) {
  const ok = await dialog({
    title: `Edit “${t.name}” as administrator?`,
    text: 'You can change it here, and saving asks for an administrator\'s password.\n\nSystem files control how your computer starts and runs: a mistake can stop it from starting. Only change what you understand.',
    buttons: [{ label: 'Cancel', value: false }, { label: 'Edit as administrator', value: true, primary: true }],
  });
  if (!ok) return;
  t.admin = true;
  setReadOnly(t);
}

// ------------------------------------------------------------------ opening
/// Opens a file in a tab (or goes to its tab if it's open). opts: readonly, line, mode, encoding.
async function openPath(path, opts = {}) {
  const have = tabs.find(t => t.path === path);
  if (have && !opts.mode && !opts.encoding) {
    activate(have);
    if (opts.line) E.gotoLine(have.ed.view, opts.line);
    return have;
  }
  let f;
  try { f = await invoke('open_file', { path, mode: opts.mode || 'auto', encoding: opts.encoding || null }); } catch (e) {
    // `hearth-text new.txt` for a file that isn't there yet: an empty tab that saves as it.
    if (!opts.create || !/isn't there/.test(String(e))) { toast(String(e), 4500); return null; }
    let st = { writable: true };
    try { st = await invoke('stat_file', { path }); } catch (_) { }
    f = { path, name: baseName(path), text: '', encoding: 'utf-8', bom: false, eol: INFO.eol, mixed_eol: false, mtime: 0, size: 0, writable: st.writable, binary: false, lossy: false, too_big: false, part: null, shown: 0 };
  }
  const reuse = current && !current.path && !current.dirty && current.ed.view.state.doc.length === 0 ? current : null;
  const tab = makeTab(f, opts);
  if (reuse) dropTab(reuse);
  activate(tab);
  const recent = (S.recent || []).find(r => r.path === f.path);
  const line = opts.line || (recent && !f.part ? recent.line : 0);
  if (line > 1) E.gotoLine(tab.ed.view, line);
  remember(f.path);
  if (f.binary) toast('This file isn\'t text: it\'s shown read-only');
  return tab;
}

/// Reads the tab's file again (asks first if there are changes). opts: mode, encoding.
async function reload(t, opts = {}) {
  if (!t.path) return;
  if (isDirty(t) && !opts.force) {
    const go = await dialog({ title: `Reload “${t.name}”?`, text: 'Your changes here are lost.', buttons: [{ label: 'Cancel', value: false }, { label: 'Reload', value: true, danger: true }] });
    if (!go) return;
  }
  let f;
  try { f = await invoke('open_file', { path: t.path, mode: opts.mode || 'auto', encoding: opts.encoding || null }); } catch (e) { return toast(String(e), 4500); }
  const line = E.cursorInfo(t.ed.view).line;
  Object.assign(t, {
    encoding: f.encoding, bom: f.bom, eol: f.eol, mixedEol: f.mixed_eol, mtime: f.mtime, size: f.size, writable: f.writable, binary: f.binary, lossy: f.lossy,
    tooBig: f.too_big, part: f.part, shown: f.shown, changedOnDisk: false, gone: false, metaDirty: false,
  });
  t.ed.setText(f.text);
  t.saved = t.ed.view.state.doc;
  t.dirty = false;
  t.readonly = isReadOnly(t);
  t.ed.setReadOnly(t.readonly);
  if (!f.part) E.gotoLine(t.ed.view, line);
  renderTabs();
  updateChrome();
}

async function openDialog() {
  const dir = current && current.path ? dirName(current.path) : null;
  let picked = [];
  try { picked = await invoke('pick_open', { dir }); } catch (e) { return toast(String(e)); }
  for (const p of picked) await openPath(p);
}

function remember(path) {
  const list = (S.recent || []).filter(r => r.path !== path);
  const old = (S.recent || []).find(r => r.path === path);
  list.unshift({ path, line: old ? old.line : 1 });
  S.recent = list.slice(0, 15);
  saveSettings();
}
function rememberLine(t) {
  if (!t || !t.path || t.part) return;
  const r = (S.recent || []).find(x => x.path === t.path);
  if (r) { r.line = E.cursorInfo(t.ed.view).line; saveSettings(); }
}

function renderRecent() {
  const box = $('recentList');
  const list = (S.recent || []).slice(0, 8);
  $('recentHead').hidden = !list.length;
  box.innerHTML = '';
  for (const r of list) {
    const b = document.createElement('button');
    b.dataset.f = '';
    b.innerHTML = `<span class="rn">${esc(baseName(r.path))}</span><span class="rp">${esc(tidyPath(dirName(r.path)))}</span>`;
    b.onclick = () => openPath(r.path);
    box.appendChild(b);
  }
}

// ------------------------------------------------------------------ saving and closing
/// Saves the tab (opts.as: Save as…). Returns whether it was saved.
async function save(t, opts = {}) {
  if (!t) return false;
  let path = t.path;
  if (!path || opts.as || (t.readonly && !t.admin && !t.gone)) {
    if (t.readonly && !opts.as && path && !t.gone) {
      if (t.binary || t.part || t.tooBig) { toast('This file is shown read-only and can\'t be saved from here'); return false; }
    }
    try { path = await invoke('pick_save', { path: t.path, name: t.path ? null : t.name + '.txt' }); } catch (e) { toast(String(e)); return false; }
    if (!path) return false;
  }
  const admin = t.admin && path === t.path;
  let st;
  try {
    st = await invoke('save_file', { path, text: t.ed.text(), encoding: t.encoding, bom: t.bom, eol: t.eol, admin });
  } catch (e) {
    const msg = String(e);
    if (/permission/i.test(msg) && !admin) {
      const choice = await dialog({
        title: `Couldn't save “${baseName(path)}”`, text: msg + '.',
        buttons: [{ label: 'Cancel', value: '' }, ...(INFO.admin ? [{ label: 'Save as administrator', value: 'admin' }] : []), { label: 'Save as…', value: 'as', primary: true }],
      });
      if (choice === 'as') return save(t, { as: true });
      if (choice === 'admin') { t.admin = true; setReadOnly(t); return save(t); }
      return false;
    }
    await dialog({ title: `Couldn't save “${baseName(path)}”`, text: msg, buttons: [{ label: 'OK', value: true, primary: true }] });
    return false;
  }
  const renamed = path !== t.path;
  Object.assign(t, { path, name: baseName(path), mtime: st.mtime, size: st.size, writable: st.writable || t.admin, gone: false, changedOnDisk: false, metaDirty: false, mixedEol: false, askedView: false });
  if (renamed) {
    t.admin = false;
    t.writable = st.writable;
    const lang = E.detectLanguage(t.name, t.ed.view.state.doc.line(1).text);
    if (lang !== t.lang) { t.lang = lang; t.ed.setLanguage(lang); }
  }
  t.saved = t.ed.view.state.doc;
  t.dirty = false;
  t.readonly = isReadOnly(t);
  t.ed.setReadOnly(t.readonly);
  remember(path);
  renderTabs();
  updateChrome();
  toast(`Saved “${t.name}”`);
  return true;
}

async function saveAll() {
  for (const t of tabs.filter(isDirty)) { activate(t); if (!await save(t)) return false; }
  return true;
}

/// Asks what to do with unsaved changes: 'save' | 'discard' | 'cancel'.
function askUnsaved(t) {
  return dialog({
    title: `Save the changes to “${t.name}”?`,
    text: 'If you don\'t, they\'re lost.',
    buttons: [{ label: 'Cancel', value: 'cancel' }, { label: 'Don\'t save', value: 'discard', danger: true }, { label: 'Save', value: 'save', primary: true }],
    cancel: 'cancel',
  });
}

async function closeTab(t) {
  if (!t) return;
  if (isDirty(t)) {
    activate(t);
    const a = await askUnsaved(t);
    if (a === 'cancel') return;
    if (a === 'save' && !await save(t)) return;
  }
  rememberLine(t);
  const i = tabs.indexOf(t);
  dropTab(t);
  if (!tabs.length) makeTab(null);
  activate(tabs[Math.min(i, tabs.length - 1)] || tabs[0]);
}

function dropTab(t) {
  const i = tabs.indexOf(t);
  if (i < 0) return;
  tabs.splice(i, 1);
  t.ed.destroy();
  t.el.remove();
  if (current === t) current = null;
}

async function closeWindow() {
  const dirty = tabs.filter(isDirty);
  if (dirty.length === 1) {
    activate(dirty[0]);
    const a = await askUnsaved(dirty[0]);
    if (a === 'cancel') return;
    if (a === 'save' && !await save(dirty[0])) return;
  } else if (dirty.length > 1) {
    const a = await dialog({
      title: `${dirty.length} files have changes that aren't saved`,
      text: dirty.map(t => '• ' + t.name).join('\n'),
      buttons: [{ label: 'Cancel', value: 'cancel' }, { label: 'Close without saving', value: 'discard', danger: true }, { label: 'Save all', value: 'save', primary: true }],
      cancel: 'cancel',
    });
    if (a === 'cancel') return;
    if (a === 'save' && !await saveAll()) return;
  }
  for (const t of tabs) rememberLine(t);
  clearTimeout(saveTimer);
  try { await invoke('settings_save', { value: S }); } catch (_) { }
  invoke('quit');
}

// ------------------------------------------------------------------ files changed by other programs
async function checkDisk() {
  for (const t of tabs) {
    if (!t.path || t.part) continue;
    let st;
    try { st = await invoke('stat_file', { path: t.path }); } catch (_) { continue; }
    if (!st.exists) { if (!t.gone && t.mtime) { t.gone = true; t.metaDirty = true; t.dirty = true; renderTabs(); } continue; }
    if (t.gone) { t.gone = false; }
    if (st.mtime !== t.mtime) {
      if (!t.dirty) { t.mtime = st.mtime; await reload(t, { force: true }); toast(`“${t.name}” was changed by another program and has been reloaded`); }
      else t.changedOnDisk = true;
    }
    if (st.writable !== t.writable && !t.admin) { t.writable = st.writable; t.readonly = isReadOnly(t); t.ed.setReadOnly(t.readonly); }
  }
  renderTabs();
  updateChrome();
}

// ------------------------------------------------------------------ dialogs (keyboard and controller friendly)
/// A question with buttons. o: title, text, buttons [{label, value, primary, danger}], input {value, placeholder},
/// cancel (the value Esc gives). Resolves to the chosen value (with an input: {value, text}).
function dialog(o) {
  return new Promise(resolve => {
    closeMenu();
    const shade = $('shade'), box = $('dlgButtons'), inp = $('dlgInput');
    const back = document.activeElement;
    $('dlgTitle').textContent = o.title || '';
    $('dlgText').textContent = o.text || '';
    $('dlgText').hidden = !o.text;
    inp.hidden = !o.input;
    box.innerHTML = '';
    const cancel = 'cancel' in o ? o.cancel : (o.buttons[0] || {}).value;
    const done = v => {
      shade.hidden = true;
      document.removeEventListener('keydown', key, true);
      if (current) current.ed.focus(); else if (back) back.focus();
      resolve(o.input ? { value: v, text: inp.value } : v);
    };
    const btns = o.buttons.map(b => {
      const e = document.createElement('button');
      e.className = 'pbtn' + (b.primary ? ' primary' : '') + (b.danger ? ' danger' : '');
      e.dataset.f = '';
      e.textContent = b.label;
      e.onclick = () => done(b.value);
      box.appendChild(e);
      return e;
    });
    const key = e => {
      if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); return done(cancel); }
      if (document.activeElement === inp) {
        if (e.key === 'Enter') { e.preventDefault(); e.stopPropagation(); const p = o.buttons.find(b => b.primary) || o.buttons[o.buttons.length - 1]; return done(p.value); }
        if (e.key === 'ArrowDown') { e.preventDefault(); btns[btns.length - 1].focus(); }
        return;
      }
      const i = btns.indexOf(document.activeElement);
      if (e.key === 'ArrowLeft' || e.key === 'ArrowUp' && !o.input) { e.preventDefault(); e.stopPropagation(); btns[Math.max(0, i - 1)].focus(); }
      else if (e.key === 'ArrowRight' || e.key === 'ArrowDown') { e.preventDefault(); e.stopPropagation(); btns[Math.min(btns.length - 1, i + 1)].focus(); }
      else if (e.key === 'ArrowUp' && o.input) { e.preventDefault(); inp.focus(); }
      else if (e.key === 'Tab') {
        e.preventDefault(); e.stopPropagation();
        const all = (o.input ? [inp] : []).concat(btns);
        const j = all.indexOf(document.activeElement);
        all[(j + (e.shiftKey ? all.length - 1 : 1)) % all.length].focus();
      } else if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault(); e.stopPropagation();
        (btns[i] || btns.find((_, k) => o.buttons[k].primary) || btns[btns.length - 1]).click();
      } else if (!e.ctrlKey && !e.altKey) { e.stopPropagation(); }
    };
    document.addEventListener('keydown', key, true);
    shade.hidden = false;
    shade.onclick = e => { if (e.target === shade) done(cancel); };
    if (o.input) {
      inp.value = o.input.value || '';
      inp.placeholder = o.input.placeholder || '';
      inp.focus();
      inp.select();
    } else {
      // The primary button has the focus, except when it would throw something away.
      const p = o.buttons.findIndex(b => b.primary);
      btns[p >= 0 ? p : btns.length - 1].focus();
    }
  });
}

// ------------------------------------------------------------------ menus
let menuStack = [];
/// A menu of items [{label, key, run, on, disabled, sub, sep, head}] next to `anchor` (or in the middle).
function openMenu(items, anchor, title) {
  const m = $('menu');
  m.innerHTML = '';
  if (title) { const h = document.createElement('div'); h.className = 'mh'; h.textContent = title; m.appendChild(h); }
  const buttons = [];
  for (const it of items) {
    if (it.sep) { m.appendChild(document.createElement('hr')); continue; }
    const b = document.createElement('button');
    b.setAttribute('role', 'menuitem');
    b.dataset.f = '';
    if (it.on !== undefined) b.className = it.on ? 'on' : 'off';
    b.innerHTML = `<span class="l">${esc(it.label)}${it.sub ? '  ›' : ''}</span>${it.key ? `<span class="k">${esc(it.key)}</span>` : ''}`;
    b.disabled = !!it.disabled;
    b.onclick = () => {
      if (it.sub) { menuStack.push({ items, anchor, title }); return openMenu(it.sub(), anchor, it.label); }
      closeMenu();
      it.run && it.run();
    };
    m.appendChild(b);
    buttons.push(b);
  }
  m.hidden = false;
  const r = anchor ? anchor.getBoundingClientRect() : null;
  const w = m.offsetWidth, h = m.offsetHeight;
  let x = r ? r.right - w : (innerWidth - w) / 2, y = r ? (r.bottom + 4 + h > innerHeight ? r.top - h - 4 : r.bottom + 4) : (innerHeight - h) / 2;
  m.style.left = Math.max(8, Math.min(innerWidth - w - 8, x)) + 'px';
  m.style.top = Math.max(8, Math.min(innerHeight - h - 8, y)) + 'px';
  const first = buttons.find(b => !b.disabled);
  if (first) first.focus();
}
function closeMenu(refocus) {
  if ($('menu').hidden) return;
  $('menu').hidden = true;
  menuStack = [];
  if (refocus && current) current.ed.focus();
}
function menuBack() {
  const prev = menuStack.pop();
  if (prev) openMenu(prev.items, prev.anchor, prev.title); else closeMenu(true);
}
$('menu').addEventListener('keydown', e => {
  const items = [...$('menu').querySelectorAll('button:not([disabled])')];
  const i = items.indexOf(document.activeElement);
  if (e.key === 'ArrowDown') items[(i + 1) % items.length].focus();
  else if (e.key === 'ArrowUp') items[(i - 1 + items.length) % items.length].focus();
  else if (e.key === 'Home') items[0].focus();
  else if (e.key === 'End') items[items.length - 1].focus();
  else if (e.key === 'Escape' || e.key === 'ArrowLeft' || e.key === 'Backspace') menuBack();
  else if (e.key === 'ArrowRight' && document.activeElement.textContent.includes('›')) document.activeElement.click();
  else return;
  e.preventDefault();
  e.stopPropagation();
});
document.addEventListener('pointerdown', e => { if (!$('menu').hidden && !$('menu').contains(e.target)) closeMenu(); }, true);

function mainMenu(anchor) {
  const t = current;
  openMenu([
    { label: 'New', key: 'Ctrl+N', run: newFile },
    { label: 'Open…', key: 'Ctrl+O', run: openDialog },
    { label: 'Open recent', sub: recentMenu, disabled: !(S.recent || []).length },
    { sep: true },
    { label: 'Save', key: 'Ctrl+S', run: () => save(t), disabled: t.readonly && !!t.path },
    { label: 'Save as…', key: 'Ctrl+Shift+S', run: () => save(t, { as: true }), disabled: t.binary || !!t.part },
    { label: 'Save all', run: saveAll, disabled: !tabs.some(x => x.dirty) },
    { label: 'Reload from disk', run: () => reload(t), disabled: !t.path },
    ...(t.path && !t.writable && !t.admin && INFO.admin && !t.binary && !t.part && !t.tooBig ? [{ label: 'Edit as administrator…', run: () => editAsAdmin(t) }] : []),
    { label: 'Print…', key: 'Ctrl+P', run: printIt },
    { label: 'Close tab', key: 'Ctrl+W', run: () => closeTab(t) },
    { sep: true },
    { label: 'Undo', key: 'Ctrl+Z', run: () => E.doUndo(t.ed.view), disabled: !E.canUndo(t.ed.view) },
    { label: 'Redo', key: 'Ctrl+Y', run: () => E.doRedo(t.ed.view), disabled: !E.canRedo(t.ed.view) },
    { label: 'Select all', key: 'Ctrl+A', run: () => { E.doSelectAll(t.ed.view); t.ed.focus(); } },
    { label: 'Find', key: 'Ctrl+F', run: () => openFind(false) },
    { label: 'Find and replace', key: 'Ctrl+H', run: () => openFind(true), disabled: t.readonly },
    { label: 'Go to line…', key: 'Ctrl+G', run: gotoDialog },
    { sep: true },
    { label: 'Word wrap', key: 'Alt+Z', on: S.wrap, run: toggleWrap },
    { label: 'Line numbers', on: S.lineNumbers, run: toggleNumbers },
    { label: 'Highlighting', sub: langMenu },
    { label: 'Encoding', sub: encodingMenu },
    { label: 'Line endings', sub: eolMenu, disabled: t.readonly },
    { label: 'Indentation', sub: indentMenu },
    { label: 'Theme', sub: themeMenu },
    { label: 'Text size', sub: zoomMenu },
    { sep: true },
    { label: 'Keyboard shortcuts', run: showKeys },
    { label: 'Quit', key: 'Ctrl+Q', run: closeWindow },
  ], anchor);
}
const recentMenu = () => (S.recent || []).map(r => ({ label: tidyPath(r.path), run: () => openPath(r.path) }))
  .concat([{ sep: true }, { label: 'Clear the list', run: () => { S.recent = []; saveSettings(); updateChrome(); } }]);
const langMenu = () => E.LANGUAGES.map(l => ({ label: l.name, on: current.lang === l.id, run: () => { current.lang = l.id; current.ed.setLanguage(l.id); updateStatus(); } }));
const themeMenu = () => Object.entries(THEMES).map(([k, th]) => ({ label: th.name, on: S.theme === k, run: () => { S.theme = k; applyTheme(); saveSettings(); } }));
const zoomMenu = () => [
  { label: 'Bigger', key: 'Ctrl++', run: () => zoom(1) },
  { label: 'Smaller', key: 'Ctrl+−', run: () => zoom(-1) },
  { label: 'Normal size', key: 'Ctrl+0', run: () => zoom(0) },
];
const indentMenu = () => [
  { label: 'Spaces', on: !current.indent.tabs, run: () => setIndent({ tabs: false }) },
  { label: 'Tabs', on: current.indent.tabs, run: () => setIndent({ tabs: true }) },
  { sep: true },
  ...[2, 4, 8].map(w => ({ label: `Width ${w}`, on: current.indent.width === w, run: () => setIndent({ width: w }) })),
];
function setIndent(ch) { current.indent = { ...current.indent, ...ch }; current.ed.setIndent(current.indent); updateStatus(); }
function encodingMenu() {
  const t = current;
  const out = [];
  for (const [id, name] of ENCODINGS) out.push({ label: 'Save as ' + name, on: t.encoding === id && (!t.bom || id.startsWith('utf-16')), disabled: t.readonly, run: () => setEncoding(id, id.startsWith('utf-16')) });
  out.push({ label: 'Save as UTF-8 with BOM', on: t.encoding === 'utf-8' && t.bom, disabled: t.readonly, run: () => setEncoding('utf-8', true) });
  if (t.path && !t.part) {
    out.push({ sep: true });
    for (const [id, name] of ENCODINGS) out.push({ label: 'Reopen as ' + name, run: () => reload(t, { encoding: id }) });
  }
  return out;
}
function setEncoding(enc, bom) {
  const t = current;
  if (t.encoding === enc && t.bom === bom) return;
  t.encoding = enc;
  t.bom = bom;
  t.lossy = false;
  t.metaDirty = true;
  markDirty(t);
  updateChrome();
  toast(`Saving will use ${enc.toUpperCase()}${bom ? ' with a byte-order mark' : ''}`);
}
const eolMenu = () => EOLS.map(([id, name]) => ({ label: name, on: current.eol === id, run: () => {
  const t = current;
  if (t.eol === id && !t.mixedEol) return;
  t.eol = id;
  t.mixedEol = false;
  t.metaDirty = true;
  markDirty(t);
  updateChrome();
} }));

function showKeys() {
  dialog({
    title: 'Keyboard shortcuts',
    text: [
      'Ctrl+N / O / S — new, open, save', 'Ctrl+Shift+S — save as', 'Ctrl+W — close tab · Ctrl+Tab — next tab',
      'Ctrl+Z / Ctrl+Y — undo, redo', 'Ctrl+F / Ctrl+H — find, replace · F3 — next match', 'Ctrl+G — go to line',
      'Alt+Z — word wrap · Ctrl + / − / 0 — text size', 'Ctrl+D — select the next match too (several cursors)',
      'Ctrl+/ — comment the line · Alt+↑/↓ — move the line', 'F10 or the Menu key — this app\'s menu',
      'F6 — the buttons (arrows to move, Esc to go back to the text)',
    ].join('\n'),
    buttons: [{ label: 'Close', value: true, primary: true }],
  });
}

// ------------------------------------------------------------------ view settings
function toggleWrap() { S.wrap = !S.wrap; for (const t of tabs) t.ed.setWrap(S.wrap); saveSettings(); toast(S.wrap ? 'Word wrap on' : 'Word wrap off', 1400); }
function toggleNumbers() { S.lineNumbers = !S.lineNumbers; for (const t of tabs) t.ed.setLineNumbers(S.lineNumbers); saveSettings(); }
function zoom(d) {
  S.fontSize = d === 0 ? DEFAULTS.fontSize : Math.max(8, Math.min(40, S.fontSize + d));
  applyTheme();
  updateStatus();
  saveSettings();
}

function newFile() { activate(makeTab(null)); }

async function gotoDialog() {
  const t = current, c = E.cursorInfo(t.ed.view);
  const r = await dialog({
    title: 'Go to line', text: `Line 1 to ${c.lines}. "120:8" goes to column 8 too.`,
    input: { value: String(c.line), placeholder: 'Line number' },
    buttons: [{ label: 'Cancel', value: false }, { label: 'Go', value: true, primary: true }],
  });
  if (!r.value) return;
  const m = /^\s*(\d+)(?:\s*[:,]\s*(\d+))?\s*$/.exec(r.text);
  if (!m) return toast('That isn\'t a line number');
  E.gotoLine(t.ed.view, +m[1], m[2] ? +m[2] : 1);
}

function printIt() {
  const t = current;
  $('printout').textContent = t.ed.text();
  setTimeout(() => window.print(), 30);
}

// ------------------------------------------------------------------ find and replace
const F = { caseSensitive: false, regexp: false, wholeWord: false };
const findOpen = () => !$('findbar').hidden;
function openFind(replace) {
  const t = current;
  $('findbar').hidden = false;
  $('replaceRow').hidden = !replace || t.readonly;
  const sel = t.ed.view.state.sliceDoc(t.ed.view.state.selection.main.from, t.ed.view.state.selection.main.to);
  if (sel && !sel.includes('\n') && sel.length < 200) $('findInput').value = sel;
  $('findInput').focus();
  $('findInput').select();
  runFind();
}
function closeFind() {
  $('findbar').hidden = true;
  if (current) { E.stopFind(current.ed.view); current.ed.focus(); }
}
function runFind() {
  if (!current) return;
  const ok = E.setFind(current.ed.view, { search: $('findInput').value, replace: $('replaceInput').value, ...F });
  $('findInput').classList.toggle('bad', !ok);
  refreshFindCount(ok);
}
function refreshFindCount(ok = true) {
  const q = $('findInput').value;
  if (!q) { $('findCount').textContent = ''; return; }
  if (!ok) { $('findCount').textContent = 'Not a valid pattern'; return; }
  const c = E.countMatches(current.ed.view);
  $('findCount').textContent = c.total ? (c.current ? `${c.current} of ${c.total}` : `${c.total} found`) + (c.total >= 9999 ? '+' : '') : 'Not found';
}
function findStep(back) {
  if (!$('findInput').value) return;
  runFind();
  (back ? E.findPrevMatch : E.findNextMatch)(current.ed.view);
  refreshFindCount();
}
$('findInput').addEventListener('input', () => { runFind(); if ($('findInput').value) { E.findNextMatch(current.ed.view); refreshFindCount(); } });
$('replaceInput').addEventListener('input', runFind);
$('findInput').addEventListener('keydown', e => {
  if (e.key === 'Enter') { e.preventDefault(); findStep(e.shiftKey); }
  else if (e.key === 'Escape') { e.preventDefault(); closeFind(); }
  else if (e.key === 'ArrowDown' && !$('replaceRow').hidden) { e.preventDefault(); $('replaceInput').focus(); }
});
$('replaceInput').addEventListener('keydown', e => {
  if (e.key === 'Enter' && e.ctrlKey && e.altKey) { e.preventDefault(); doReplaceAll(); }
  else if (e.key === 'Enter') { e.preventDefault(); doReplace(); }
  else if (e.key === 'Escape') { e.preventDefault(); closeFind(); }
  else if (e.key === 'ArrowUp') { e.preventDefault(); $('findInput').focus(); }
});
function toggleOpt(k, el) { F[k] = !F[k]; el.setAttribute('aria-pressed', String(F[k])); runFind(); }
$('optCase').onclick = () => toggleOpt('caseSensitive', $('optCase'));
$('optWord').onclick = () => toggleOpt('wholeWord', $('optWord'));
$('optRegex').onclick = () => toggleOpt('regexp', $('optRegex'));
$('findNext').onclick = () => findStep(false);
$('findPrev').onclick = () => findStep(true);
$('findClose').onclick = closeFind;
function doReplace() { if (current.readonly) return; runFind(); E.replaceOne(current.ed.view); refreshFindCount(); }
function doReplaceAll() {
  if (current.readonly) return;
  runFind();
  const n = E.countMatches(current.ed.view).total;
  E.replaceEvery(current.ed.view);
  refreshFindCount();
  toast(n ? `Replaced ${n}${n >= 9999 ? '+' : ''}` : 'Nothing to replace', 1800);
}
$('replaceOne').onclick = doReplace;
$('replaceAll').onclick = doReplaceAll;

// ------------------------------------------------------------------ buttons
$('btnNew').onclick = newFile;
$('btnOpen').onclick = openDialog;
$('startOpen').onclick = openDialog;
$('btnSave').onclick = () => save(current);
$('btnUndo').onclick = () => { E.doUndo(current.ed.view); current.ed.focus(); };
$('btnRedo').onclick = () => { E.doRedo(current.ed.view); current.ed.focus(); };
$('btnFind').onclick = () => (findOpen() ? closeFind() : openFind(!current.readonly));
$('btnMenu').onclick = () => mainMenu($('btnMenu'));
$('stPos').onclick = gotoDialog;
$('stLang').onclick = () => openMenu(langMenu(), $('stLang'), 'Highlighting');
$('stIndent').onclick = () => openMenu(indentMenu(), $('stIndent'), 'Indentation');
$('stEnc').onclick = () => openMenu(encodingMenu(), $('stEnc'), 'Encoding');
$('stEol').onclick = () => { if (!current.readonly) openMenu(eolMenu(), $('stEol'), 'Line endings'); };
$('stZoom').onclick = () => openMenu(zoomMenu(), $('stZoom'), 'Text size');

// ------------------------------------------------------------------ keys
// The buttons can be reached without a mouse: F6 goes to the top bar, arrows move along it, Down/Esc go back.
const zones = () => [$('bar'), $('findbar'), $('banner'), $('start'), $('status')].filter(z => !z.hidden);
function focusables(z) { return [...z.querySelectorAll('[data-f]')].filter(b => !b.disabled && b.offsetParent !== null); }
function zoneKey(e) {
  const a = document.activeElement;
  const z = zones().find(z => z.contains(a));
  if (!z || a.tagName === 'INPUT') return false;
  const list = focusables(z), i = list.indexOf(a);
  if (e.key === 'ArrowRight' && i >= 0) { list[Math.min(list.length - 1, i + 1)].focus(); return true; }
  if (e.key === 'ArrowLeft' && i >= 0) { list[Math.max(0, i - 1)].focus(); return true; }
  if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
    const zs = zones(), j = zs.indexOf(z), next = zs[j + (e.key === 'ArrowDown' ? 1 : -1)];
    if (z.id === 'start' && i >= 0 && list[i + (e.key === 'ArrowDown' ? 1 : -1)]) { list[i + (e.key === 'ArrowDown' ? 1 : -1)].focus(); return true; }
    if (next && next.id !== 'status' || (next && e.key === 'ArrowUp')) { const f = focusables(next)[0]; if (f) { f.focus(); return true; } }
    current.ed.focus();
    return true;
  }
  if (e.key === 'Escape') { current.ed.focus(); return true; }
  return false;
}

document.addEventListener('keydown', e => {
  if (!$('shade').hidden) return;                        // the dialog has its own keys
  const k = e.key, ctrl = e.ctrlKey || e.metaKey, lower = k.length === 1 ? k.toLowerCase() : k;
  let done = true;
  if (ctrl && !e.shiftKey && lower === 'n') newFile();
  else if (ctrl && lower === 'o') openDialog();
  else if (ctrl && e.shiftKey && lower === 's') save(current, { as: true });
  else if (ctrl && lower === 's') save(current);
  else if (ctrl && lower === 'w') closeTab(current);
  else if (ctrl && lower === 'q') closeWindow();
  else if (ctrl && lower === 'p') printIt();
  else if (ctrl && !e.shiftKey && lower === 'f') openFind(false);
  else if (ctrl && lower === 'h') { if (!current.readonly) openFind(true); }
  else if (ctrl && (lower === 'g' || lower === 'l')) gotoDialog();
  else if (ctrl && (k === '=' || k === '+')) zoom(1);
  else if (ctrl && (k === '-' || k === '_')) zoom(-1);
  else if (ctrl && k === '0') zoom(0);
  else if (ctrl && (k === 'Tab' || k === 'PageDown' || k === 'PageUp')) {
    const back = (k === 'Tab' && e.shiftKey) || k === 'PageUp';
    const i = tabs.indexOf(current);
    activate(tabs[(i + (back ? tabs.length - 1 : 1)) % tabs.length]);
  } else if (e.altKey && lower === 'z') toggleWrap();
  else if (e.altKey && findOpen() && lower === 'c') $('optCase').click();
  else if (e.altKey && findOpen() && lower === 'w') $('optWord').click();
  else if (e.altKey && findOpen() && lower === 'r') $('optRegex').click();
  else if (k === 'F3') findStep(e.shiftKey);
  else if (k === 'F10' || k === 'ContextMenu' && !current.el.contains(document.activeElement)) mainMenu($('btnMenu'));
  else if (k === 'F6') { const f = focusables($('bar')); (f.find(b => b.closest('.tab.active')) || f[0]).focus(); }
  else if (k === 'Escape' && findOpen()) closeFind();
  else if (!$('menu').hidden) done = false;
  else done = zoneKey(e);
  if (done) { e.preventDefault(); e.stopPropagation(); }
}, true);

// ------------------------------------------------------------------ start
async function start() {
  if (!T) { document.body.textContent = 'Hearth Text needs its app window.'; return; }
  try { INFO = await invoke('init'); } catch (e) { toast('Couldn\'t start: ' + e, 8000); }
  S = { ...DEFAULTS, ...(INFO.settings || {}) };
  if (!THEMES[S.theme]) S.theme = DEFAULTS.theme;
  if (!Array.isArray(S.recent)) S.recent = [];
  applyTheme();
  const l = INFO.launch || { files: [] };
  makeTab(null);
  activate(tabs[0]);
  let first = true;
  for (const f of l.files || []) {
    await openPath(f, { readonly: l.readonly, line: first ? l.line : 0, create: true });
    first = false;
  }
  if (l.new && l.files && l.files.length) newFile();
  T.event.listen('open-files', async ev => {
    const p = ev.payload || {};
    let firstFile = true;
    for (const f of p.files || []) { await openPath(f, { readonly: p.readonly, line: firstFile ? p.line : 0, create: true }); firstFile = false; }
    if (p.new || !(p.files || []).length) newFile();
  });
  T.event.listen('close-requested', () => closeWindow());
  // Files dropped on the window open in tabs.
  T.event.listen('tauri://drag-drop', async ev => { for (const p of (ev.payload && ev.payload.paths) || []) await openPath(p); });
  window.addEventListener('focus', () => checkDisk());
  setInterval(() => { if (document.hasFocus()) checkDisk(); }, 4000);
  window.__hearthText = { tabs, openPath, save, closeTab, openFind, mainMenu, current: () => current, dialog, E };
}
start();
