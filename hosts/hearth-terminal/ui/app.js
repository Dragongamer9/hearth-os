// Hearth Terminal's window: tabs and splits of xterm.js terminals, each one a shell in a pseudo-terminal on the
// Rust side (src/pty.rs). Works with a keyboard, a mouse, touch, and a controller (the controller arrives as
// keys and a pointer: D-pad = arrows, B = Esc, Y = on-screen keyboard).
'use strict';

const T = window.__TAURI__;
const invoke = (cmd, args) => T.core.invoke(cmd, args);
const $ = id => document.getElementById(id);

// ------------------------------------------------------------------ themes
// Terminal colors and the window around them. Ember is Hearth's palette (Theme.qml); its ANSI colors are
// warm but still tell red from orange from yellow.
const THEMES = {
  ember: {
    name: 'Ember',
    ui: { bg: '#0f0b09', chrome: '#17110e', panel: '#1c1512', panel2: '#261d18', panel3: '#33271f', line: '#3b2e26', text: '#f4ece3', muted: '#a69484', accent: '#ff8a3d', accentInk: '#2a1206' },
    term: {
      background: '#0f0b09', foreground: '#f4ece3', cursor: '#ff8a3d', cursorAccent: '#0f0b09',
      selectionBackground: 'rgba(255,138,61,0.32)', selectionInactiveBackground: 'rgba(255,138,61,0.16)',
      black: '#2a201b', red: '#ff5d5d', green: '#9bd36a', yellow: '#ffd166', blue: '#6cb6ff', magenta: '#e58fd8', cyan: '#5fd3c6', white: '#e7dccf',
      brightBlack: '#7a685a', brightRed: '#ff8a7a', brightGreen: '#b8e68c', brightYellow: '#ffe29a', brightBlue: '#9ccfff', brightMagenta: '#f2b3e8', brightCyan: '#8be8dd', brightWhite: '#fffaf3',
    },
  },
  night: {
    name: 'Night',
    ui: { bg: '#0c1016', chrome: '#121821', panel: '#161d27', panel2: '#1d2531', panel3: '#273140', line: '#2a3443', text: '#e9eef4', muted: '#8d99a8', accent: '#3fd0b8', accentInk: '#06231e' },
    term: {
      background: '#0c1016', foreground: '#e9eef4', cursor: '#3fd0b8', cursorAccent: '#0c1016',
      selectionBackground: 'rgba(63,208,184,0.30)', selectionInactiveBackground: 'rgba(63,208,184,0.15)',
      black: '#1d2531', red: '#ff6b6b', green: '#8bd17c', yellow: '#ffb547', blue: '#63a7ff', magenta: '#d08cf0', cyan: '#3fd0b8', white: '#d3dae3',
      brightBlack: '#5d6978', brightRed: '#ff8f8f', brightGreen: '#a9e69c', brightYellow: '#ffd08a', brightBlue: '#93c3ff', brightMagenta: '#e3b2f7', brightCyan: '#7fe6d4', brightWhite: '#ffffff',
    },
  },
  daylight: {
    name: 'Daylight',
    light: true,
    ui: { bg: '#f7f1ea', chrome: '#efe6dc', panel: '#fbf7f2', panel2: '#e8ddd1', panel3: '#ddcfc0', line: '#d6c8b9', text: '#2a1d15', muted: '#76655a', accent: '#d9601a', accentInk: '#ffffff' },
    term: {
      background: '#f7f1ea', foreground: '#2a1d15', cursor: '#d9601a', cursorAccent: '#f7f1ea',
      selectionBackground: 'rgba(217,96,26,0.25)', selectionInactiveBackground: 'rgba(217,96,26,0.12)',
      black: '#2a1d15', red: '#c4382c', green: '#3b8a2e', yellow: '#a86b00', blue: '#2463b5', magenta: '#a2449a', cyan: '#13867c', white: '#d9cfc4',
      brightBlack: '#7a6a5d', brightRed: '#e0503f', brightGreen: '#4fa53f', brightYellow: '#c58300', brightBlue: '#3a7bd5', brightMagenta: '#bf5cb6', brightCyan: '#1fa596', brightWhite: '#ffffff',
    },
  },
};

const FONT = '"JetBrains Mono", "Cascadia Mono", "Cascadia Code", Consolas, "DejaVu Sans Mono", "Noto Sans Mono", "Liberation Mono", "Symbols Nerd Font Mono", monospace';
const DEFAULTS = { fontSize: 15, theme: 'ember', shell: '', opacity: 1, cursorStyle: 'block', cursorBlink: true, quickKeys: false };

let S = { ...DEFAULTS };          // settings
let INFO = {};                    // what init() said: platform, shells, launch
let seeThroughWindow = false;     // the window was opened see-through (opacity below 100% at start)
const tabs = [];
let current = null;               // the tab in front

// ------------------------------------------------------------------ small helpers
function toast(text, ms = 2200) {
  const t = $('toast');
  t.textContent = text;
  t.classList.add('show');
  clearTimeout(toast.timer);
  toast.timer = setTimeout(() => t.classList.remove('show'), ms);
}

function hexToRgba(hex, a) {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${a})`;
}

function theme() { return THEMES[S.theme] || THEMES.ember; }

/// The terminal colors with the background made see-through when Opacity asks for it.
function termTheme() {
  const th = theme();
  const o = seeThroughWindow ? S.opacity : 1;
  return o < 0.999 ? { ...th.term, background: hexToRgba(th.term.background, o) } : th.term;
}

function applyChrome() {
  const th = theme(), r = document.documentElement;
  for (const [k, v] of Object.entries(th.ui)) r.style.setProperty('--' + k, v);
  r.classList.toggle('light', !!th.light);
  const o = seeThroughWindow ? S.opacity : 1;
  r.style.setProperty('--bgpage', o < 0.999 ? 'transparent' : th.ui.bg);
  r.style.setProperty('--bgterm', o < 0.999 ? 'transparent' : th.term.background);
  r.style.setProperty('--chrome', o < 0.999 ? hexToRgba(th.ui.chrome, Math.min(1, o + 0.12)) : th.ui.chrome);
}

let saveTimer = 0;
function saveSettings() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => invoke('settings_save', { value: S }).catch(e => toast("Settings weren't saved: " + e)), 300);
}

/// A path typed into the shell, quoted so spaces and odd characters survive.
function quotePath(p) {
  if (INFO.platform === 'windows') return /[\s&()^%!;,'"]/.test(p) ? `"${p}"` : p;
  return /^[\w@%+=:,./-]+$/.test(p) ? p : `'${p.replace(/'/g, `'\\''`)}'`;
}

// ------------------------------------------------------------------ panes (one terminal each)
let paneSeq = 0;

function loadRenderer(term) {
  // The graphics card draws the text (WebGL); if it can't, a canvas; failing that, xterm's own DOM drawing.
  try {
    const gl = new WebglAddon.WebglAddon();
    gl.onContextLoss(() => { gl.dispose(); loadCanvas(term); });
    term.loadAddon(gl);
    return 'webgl';
  } catch (e) { return loadCanvas(term); }
}
function loadCanvas(term) {
  try { term.loadAddon(new CanvasAddon.CanvasAddon()); return 'canvas'; } catch (e) { return 'dom'; }
}

function makeTerminal() {
  const term = new Terminal({
    fontFamily: FONT,
    fontSize: S.fontSize,
    lineHeight: 1.12,
    cursorStyle: S.cursorStyle,
    cursorBlink: S.cursorBlink,
    allowTransparency: seeThroughWindow,
    allowProposedApi: true,
    theme: termTheme(),
    scrollback: 10000,
    smoothScrollDuration: 0,
    macOptionIsMeta: true,
    rightClickSelectsWord: false,
    drawBoldTextInBrightColors: true,
    minimumContrastRatio: 1,
  });
  return term;
}

/// A new terminal in `tab`, running the shell (or `command`) in `cwd`.
async function addPane(tab, { shell, cwd, command } = {}) {
  const el = document.createElement('div');
  el.className = 'pane';
  tab.body.appendChild(el);
  const term = makeTerminal();
  const fit = new FitAddon.FitAddon();
  const search = new SearchAddon.SearchAddon();
  term.loadAddon(fit);
  term.loadAddon(search);
  term.loadAddon(new WebLinksAddon.WebLinksAddon((ev, uri) => {
    invoke('open_url', { url: uri }).catch(e => toast(String(e)));
  }));
  const uni = new Unicode11Addon.Unicode11Addon();
  term.loadAddon(uni);
  term.unicode.activeVersion = '11';

  const pane = { n: ++paneSeq, tab, el, term, fit, search, id: 0, title: '', cwd: cwd || '', shell, command, ended: false };
  tab.panes.push(pane);
  layoutTab(tab);
  term.open(el);
  pane.renderer = loadRenderer(term);
  try { fit.fit(); } catch (e) {}

  // The folder the shell is in, when it says so: OSC 7 (file://host/path; bash, zsh, fish setups) or
  // OSC 9;9 ("C:\path"; the PowerShell and Command Prompt prompts set up by src/shells.rs).
  term.parser.registerOscHandler(7, data => {
    try { const u = new URL(data); if (u.protocol === 'file:') pane.cwd = decodeURIComponent(u.pathname).replace(/^\/([A-Za-z]:)/, '$1'); } catch (e) {}
    return true;
  });
  term.parser.registerOscHandler(9, data => {
    if (!data.startsWith('9;')) return false;
    pane.cwd = data.slice(2).replace(/^"(.*)"$/, '$1');
    return true;
  });
  // OSC 52: a program (vim, tmux over ssh) copying to the clipboard. Reading the clipboard this way isn't allowed.
  term.parser.registerOscHandler(52, data => {
    const i = data.indexOf(';');
    const b64 = i < 0 ? '' : data.slice(i + 1);
    if (!b64 || b64 === '?' || b64.length > 4 * 1024 * 1024) return true;
    try {
      const bin = atob(b64);
      const text = new TextDecoder().decode(Uint8Array.from(bin, c => c.charCodeAt(0)));
      invoke('clipboard_write', { text }).then(() => toast('Copied')).catch(() => {});
    } catch (e) {}
    return true;
  });

  term.onTitleChange(t => { pane.title = t; renderTabs(); });
  term.onData(d => typed(pane, d));
  term.onBinary(d => send(pane, d));
  term.onResize(({ cols, rows }) => { if (pane.id) invoke('pty_resize', { id: pane.id, cols, rows }).catch(() => {}); });
  term.onSelectionChange(() => { if (term.hasSelection()) lastSelection = pane; });
  term.textarea?.addEventListener('focus', () => setActivePane(pane));
  term.attachCustomKeyEventHandler(e => keyFromTerminal(e, pane));
  el.addEventListener('mousedown', () => setActivePane(pane));

  await startShell(pane);
  return pane;
}

async function startShell(pane) {
  const { term } = pane;
  const ch = new T.core.Channel();
  ch.onmessage = m => {
    if (pane.closed) return;
    if (m.t === 'data') term.write(m.d);
    else if (m.t === 'exit') ended(pane, m.code);
  };
  try {
    const id = await invoke('pty_spawn', {
      shell: pane.shell || S.shell || null,
      cwd: pane.cwd || null,
      command: pane.command || null,
      cols: term.cols, rows: term.rows, events: ch,
    });
    // Closed while the shell was starting: it mustn't run on unseen.
    if (pane.closed) { invoke('pty_close', { id }).catch(() => {}); return; }
    pane.id = id;
  } catch (e) {
    if (pane.closed) return;
    term.write(`\x1b[31mCouldn't start ${pane.command ? pane.command.join(' ') : 'the shell'}:\x1b[0m ${e}\r\n\r\nPress Enter to close.`);
    pane.ended = true;
    pane.failedToStart = true;
    pane.id = 0;
    renderTabs();
  }
}

function send(pane, data) {
  if (pane.id && !pane.ended) invoke('pty_write', { id: pane.id, data }).catch(() => {});
}

/// Keys typed into a terminal: the quick keys' Ctrl and Alt apply to the next key, and an ended shell waits
/// for Enter (close) or R (start again).
function typed(pane, data) {
  if (pane.ended) {
    if (data === '\r') closePane(pane);
    else if (/^[rR]$/.test(data) && !pane.failedToStart) restart(pane);
    return;
  }
  if ((latch.ctrl || latch.alt) && data.length === 1) {
    let d = data;
    if (latch.ctrl) d = ctrlOf(d);
    if (latch.alt) d = '\x1b' + d;
    clearLatch();
    send(pane, d);
    return;
  }
  send(pane, data);
}

function ctrlOf(c) {
  const code = c.toUpperCase().charCodeAt(0);
  if (code >= 64 && code <= 95) return String.fromCharCode(code - 64);   // @ A..Z [ \ ] ^ _
  if (c === ' ' || c === '2') return '\x00';
  if (c === '?' || c === '8') return '\x7f';
  return c;
}

function ended(pane, code) {
  // The shell is gone, but its terminal stays open until it's closed (on Windows, its console host too).
  if (pane.id) invoke('pty_close', { id: pane.id }).catch(() => {});
  pane.id = 0;
  // A shell that ends normally closes its tab, as you'd expect from typing `exit`. A command (-e) or a shell
  // that failed stays, so you can read what happened.
  if (!pane.command && code === 0) { closePane(pane); return; }
  pane.ended = true;
  const how = code === 0 || code == null ? 'Finished' : `Ended with code ${code}`;
  pane.term.write(`\r\n\x1b[2m[${how}. Press Enter to close${pane.command ? '' : ', or R to start again'}.]\x1b[0m`);
  renderTabs();
}

async function restart(pane) {
  pane.ended = false;
  pane.term.reset();
  await startShell(pane);
  renderTabs();
}

function closePane(pane) {
  const tab = pane.tab;
  pane.closed = true;
  if (pane.id) invoke('pty_close', { id: pane.id }).catch(() => {});
  pane.id = 0;
  pane.term.dispose();
  pane.el.remove();
  tab.panes = tab.panes.filter(p => p !== pane);
  if (!tab.panes.length) { closeTab(tab); return; }
  tab.active = tab.panes[0];
  layoutTab(tab);
  fitTab(tab);
  focusTerminal();
}

/// The folder a new tab or split should open in: where the current shell is.
async function currentCwd() {
  const p = current?.active;
  if (!p) return INFO.launch?.cwd || '';
  if (p.id && INFO.platform === 'linux') {
    try { const c = await invoke('pty_cwd', { id: p.id }); if (c) return c; } catch (e) {}
  }
  return p.cwd || '';
}

// ------------------------------------------------------------------ tabs
async function newTab(opts = {}) {
  const body = document.createElement('div');
  body.className = 'tabbody';
  $('panes').appendChild(body);
  const tab = { body, panes: [], active: null };
  tabs.push(tab);
  showTab(tab);
  const p = await addPane(tab, opts);
  tab.active = p;
  renderTabs();
  focusTerminal();
  return tab;
}

function closeTab(tab) {
  for (const p of [...tab.panes]) {
    p.closed = true;
    if (p.id) invoke('pty_close', { id: p.id }).catch(() => {});
    p.term.dispose();
  }
  tab.body.remove();
  const i = tabs.indexOf(tab);
  if (i >= 0) tabs.splice(i, 1);
  if (!tabs.length) { invoke('quit'); return; }
  if (current === tab) showTab(tabs[Math.min(i, tabs.length - 1)]);
  renderTabs();
  focusTerminal();
}

function showTab(tab) {
  current = tab;
  for (const t of tabs) t.body.hidden = t !== tab;
  renderTabs();
  requestAnimationFrame(() => fitTab(tab));
}

function cycleTab(step) {
  if (tabs.length < 2) return;
  const i = tabs.indexOf(current);
  showTab(tabs[(i + step + tabs.length) % tabs.length]);
  focusTerminal();
}

function tabTitle(tab) {
  const p = tab.active || tab.panes[0];
  if (!p) return 'Terminal';
  if (p.title) return p.title;
  if (p.command) return p.command[0].split(/[\\/]/).pop();
  const sh = INFO.shells.find(s => s.id === (p.shell || S.shell)) || INFO.shells[0];
  return sh ? sh.name.replace(/ \(your shell\)$/, '') : 'Terminal';
}

function renderTabs() {
  const box = $('tabs');
  box.textContent = '';
  tabs.forEach((tab, i) => {
    const b = document.createElement('div');
    b.className = 'tab' + (tab === current ? ' active' : '') + (tab.panes.some(p => p.ended) ? ' ended' : '');
    b.setAttribute('role', 'tab');
    const t = document.createElement('button');
    t.className = 'tt';
    t.dataset.f = '';
    t.textContent = tabTitle(tab);
    t.title = t.textContent;
    t.addEventListener('click', () => { showTab(tab); focusTerminal(); });
    t.addEventListener('auxclick', e => { if (e.button === 1) closeTab(tab); });
    const x = document.createElement('button');
    x.className = 'tx';
    x.dataset.f = '';
    x.title = 'Close tab (Ctrl+Shift+W)';
    x.setAttribute('aria-label', 'Close tab');
    x.innerHTML = '<svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></svg>';
    x.addEventListener('click', e => { e.stopPropagation(); closeTab(tab); });
    b.append(t, x);
    box.appendChild(b);
    if (tab === current) requestAnimationFrame(() => b.scrollIntoView({ block: 'nearest', inline: 'nearest' }));
  });
  $('btnSplit').classList.toggle('on', !!current && current.panes.length > 1);
  const title = current ? tabTitle(current) : 'Terminal';
  document.title = title;
  try { T.window.getCurrentWindow().setTitle(title === 'Terminal' ? 'Terminal' : `${title} – Terminal`); } catch (e) {}
}

// ------------------------------------------------------------------ splits
function layoutTab(tab) {
  const split = tab.panes.length > 1;
  tab.body.classList.toggle('split', split);
  // Side by side in a wide window, one above the other in a tall one.
  const r = $('panes').getBoundingClientRect();
  tab.body.classList.toggle('stack', split && r.height > r.width * 0.8);
  for (const p of tab.panes) p.el.classList.toggle('active', p === tab.active);
}

async function toggleSplit() {
  if (!current) return;
  if (current.panes.length > 1) { closePane(current.active || current.panes[1]); return; }
  const cwd = await currentCwd();
  const p = await addPane(current, { cwd, shell: current.active?.shell });
  setActivePane(p);
  fitTab(current);
  focusTerminal();
  renderTabs();
}

function setActivePane(pane) {
  const tab = pane.tab;
  tab.active = pane;
  for (const p of tab.panes) p.el.classList.toggle('active', p === pane);
  if (tab === current) renderTabs();
}

function cyclePane(step) {
  if (!current || current.panes.length < 2) return;
  const i = current.panes.indexOf(current.active);
  setActivePane(current.panes[(i + step + current.panes.length) % current.panes.length]);
  focusTerminal();
}

function fitTab(tab) {
  if (!tab || tab.body.hidden) return;
  layoutTab(tab);
  for (const p of tab.panes) { try { p.fit.fit(); } catch (e) {} }
}

function focusTerminal() {
  closeMenu();
  const p = current?.active;
  if (p && !p.term.element?.isConnected) return;
  requestAnimationFrame(() => p?.term.focus());
}

// ------------------------------------------------------------------ copy, paste, find, zoom
let lastSelection = null;

async function copy() {
  const p = current?.active;
  const text = p?.term.getSelection() || lastSelection?.term.getSelection();
  if (!text) { toast('Select some text first'); return false; }
  try { await invoke('clipboard_write', { text }); toast('Copied'); } catch (e) { toast("Couldn't copy: " + e); }
  return true;
}

async function paste(text) {
  const p = current?.active;
  if (!p) return;
  if (text == null) {
    try { text = await invoke('clipboard_read'); } catch (e) { text = ''; }
  }
  if (!text) { toast('Nothing to paste'); return; }
  // paste() keeps bracketed paste (programs know it was pasted, not typed) and turns new lines into Enter.
  if (p.ended) return;
  p.term.paste(text);
  focusTerminal();
}

function zoom(step) {
  S.fontSize = step === 0 ? DEFAULTS.fontSize : Math.max(8, Math.min(40, S.fontSize + step));
  applySettings();
  saveSettings();
  toast(`Text size ${S.fontSize}`, 1200);
}

const findOpts = () => ({
  decorations: {
    matchBackground: hexToRgba(theme().ui.accent, 0.28), matchBorder: 'transparent', matchOverviewRuler: theme().ui.accent,
    activeMatchBackground: hexToRgba(theme().ui.accent, 0.7), activeMatchBorder: theme().ui.accent, activeMatchColorOverviewRuler: theme().ui.accent,
  },
});

function openFind() {
  $('findbar').hidden = false;
  const sel = current?.active?.term.getSelection();
  if (sel && !sel.includes('\n')) $('findInput').value = sel;
  $('findInput').focus();
  $('findInput').select();
  requestAnimationFrame(() => fitTab(current));
}
function closeFind() {
  $('findbar').hidden = true;
  for (const t of tabs) for (const p of t.panes) p.search.clearDecorations();
  $('findCount').textContent = '';
  requestAnimationFrame(() => fitTab(current));
  focusTerminal();
}
function find(back) {
  const p = current?.active;
  const q = $('findInput').value;
  if (!p || !q) { $('findCount').textContent = ''; return; }
  if (!p.searchHooked) {
    p.searchHooked = true;
    p.search.onDidChangeResults(r => {
      if (current?.active !== p) return;
      $('findCount').textContent = !r || r.resultCount === 0 ? 'No results' : r.resultIndex >= 0 ? `${r.resultIndex + 1} of ${r.resultCount}` : `${r.resultCount} found`;
    });
  }
  const ok = back ? p.search.findPrevious(q, findOpts()) : p.search.findNext(q, findOpts());
  if (!ok) $('findCount').textContent = 'No results';
}

// ------------------------------------------------------------------ settings
function applySettings() {
  applyChrome();
  for (const t of tabs) for (const p of t.panes) {
    const o = p.term.options;
    o.fontSize = S.fontSize;
    o.cursorStyle = S.cursorStyle;
    o.cursorBlink = S.cursorBlink;
    o.theme = termTheme();
  }
  $('quick').hidden = !S.quickKeys;
  $('btnKeys').classList.toggle('on', S.quickKeys);
  renderSettings();
  requestAnimationFrame(() => fitTab(current));
}

function choiceButtons(box, items, selected, pick) {
  box.textContent = '';
  for (const [value, label] of items) {
    const b = document.createElement('button');
    b.dataset.f = '';
    b.textContent = label;
    b.className = value === selected ? 'sel' : '';
    b.addEventListener('click', () => pick(value));
    box.appendChild(b);
  }
}

function renderSettings() {
  $('vFontSize').textContent = S.fontSize;
  $('vOpacity').textContent = Math.round(S.opacity * 100) + '%';
  $('opacityHint').hidden = !(S.opacity < 0.999 && !seeThroughWindow);
  const set = (k, v) => { S[k] = v; applySettings(); saveSettings(); };
  const box = k => document.querySelector(`.choices[data-setting="${k}"]`);
  choiceButtons(box('theme'), Object.entries(THEMES).map(([k, t]) => [k, t.name]), S.theme, v => set('theme', v));
  const shells = INFO.shells || [];
  const chosen = shells.some(s => s.id === S.shell) ? S.shell : (shells[0]?.id || '');
  choiceButtons(box('shell'), shells.map(s => [s.id, s.name]), chosen, v => set('shell', v));
  choiceButtons(box('cursorStyle'), [['block', 'Block'], ['bar', 'Line'], ['underline', 'Underline']], S.cursorStyle, v => set('cursorStyle', v));
  choiceButtons(box('cursorBlink'), [[true, 'Blinking'], [false, 'Steady']], S.cursorBlink, v => set('cursorBlink', v));
  choiceButtons(box('quickKeys'), [[true, 'Show'], [false, 'Hide']], S.quickKeys, v => set('quickKeys', v));
}

function openSettings() {
  closeMenu();
  renderSettings();
  $('settings').hidden = false;
  requestAnimationFrame(() => $('settings').querySelector('[data-f]:not(#settingsClose)')?.focus());
}
function closeSettings() { $('settings').hidden = true; focusTerminal(); }

document.querySelectorAll('[data-step]').forEach(b => b.addEventListener('click', () => {
  const [k, d] = b.dataset.step.split(':');
  if (k === 'fontSize') S.fontSize = Math.max(8, Math.min(40, S.fontSize + Number(d)));
  if (k === 'opacity') S.opacity = Math.round(Math.max(0.3, Math.min(1, S.opacity + Number(d))) * 10) / 10;
  applySettings();
  saveSettings();
}));

// ------------------------------------------------------------------ menus (right-click, shells)
function closeMenu() { $('menu').hidden = true; }

/// Shows a menu of [label, action, keys, disabled] rows ('-' is a divider) at x, y.
function openMenu(items, x, y) {
  const m = $('menu');
  m.textContent = '';
  for (const it of items) {
    if (it === '-') { m.appendChild(document.createElement('hr')); continue; }
    const [label, act, keys, disabled] = it;
    const b = document.createElement('button');
    b.dataset.f = '';
    b.setAttribute('role', 'menuitem');
    b.disabled = !!disabled;
    b.innerHTML = '<span></span><span class="k"></span>';
    b.firstChild.textContent = label;
    b.lastChild.textContent = keys || '';
    b.addEventListener('click', () => { closeMenu(); act(); });
    m.appendChild(b);
  }
  m.hidden = false;
  const r = m.getBoundingClientRect();
  m.style.left = Math.max(6, Math.min(x, innerWidth - r.width - 6)) + 'px';
  m.style.top = Math.max(6, Math.min(y, innerHeight - r.height - 6)) + 'px';
  m.querySelector('button:not([disabled])')?.focus();
}

function contextMenu(x, y) {
  const p = current?.active;
  const sel = !!p?.term.hasSelection();
  openMenu([
    ['Copy', copy, 'Ctrl+Shift+C', !sel],
    ['Paste', () => paste(), 'Ctrl+Shift+V'],
    ['Select all', () => p?.term.selectAll(), ''],
    '-',
    ['Find', openFind, 'Ctrl+Shift+F'],
    ['Clear', () => { p?.term.clear(); focusTerminal(); }, ''],
    '-',
    ['New tab', () => newTabHere(), 'Ctrl+Shift+T'],
    [current?.panes.length > 1 ? 'Close split' : 'Split', toggleSplit, 'Ctrl+Shift+D'],
    ['Settings', openSettings, 'Ctrl+,'],
  ], x, y);
}

function shellMenu() {
  const r = $('btnShells').getBoundingClientRect();
  openMenu(INFO.shells.map(s => [s.name, () => newTabHere(s.id), '']), r.left, r.bottom + 4);
}

async function newTabHere(shell) { newTab({ cwd: await currentCwd(), shell }); }

// ------------------------------------------------------------------ the on-screen keyboard and quick keys
let oskOpen = false;
async function toggleOsk() {
  try {
    await invoke('osk', { open: !oskOpen });
    oskOpen = !oskOpen;
    $('btnOsk').classList.toggle('on', oskOpen);
  } catch (e) { toast(String(e), 3500); }
  focusTerminal();
}

const latch = { ctrl: false, alt: false };
function clearLatch() {
  latch.ctrl = latch.alt = false;
  document.querySelectorAll('#quick .latch').forEach(b => b.classList.remove('latched'));
}

function quickKey(key) {
  const p = current?.active;
  if (!p) return;
  if (key === 'ctrl' || key === 'alt') {
    latch[key] = !latch[key];
    document.querySelector(`#quick [data-key="${key}"]`).classList.toggle('latched', latch[key]);
    return;
  }
  if (key === 'paste') { paste(); return; }
  const arrows = { up: 'A', down: 'B', right: 'C', left: 'D' };
  let d;
  if (arrows[key]) {
    const mod = 1 + (latch.alt ? 2 : 0) + (latch.ctrl ? 4 : 0);
    // Programs like vim switch the arrows to "application" mode; the arrows must follow.
    d = mod > 1 ? `\x1b[1;${mod}${arrows[key]}` : (p.term.modes.applicationCursorKeysMode ? '\x1bO' : '\x1b[') + arrows[key];
    clearLatch();
    send(p, d);
    return;
  }
  d = key === 'esc' ? '\x1b' : key === 'tab' ? '\t' : key;
  typed(p, d);
}

// Quick keys mustn't take the keyboard focus away from the terminal (that would close the on-screen keyboard).
document.querySelectorAll('#quick button').forEach(b => {
  b.addEventListener('pointerdown', e => e.preventDefault());
  b.addEventListener('click', () => { quickKey(b.dataset.key); if (!b.matches(':focus-visible')) focusTerminal(); });
});

// ------------------------------------------------------------------ keys
/// The app's shortcuts. Returns true when `e` was one (and does it on keydown).
function shortcut(e) {
  const k = e.key, ctrl = e.ctrlKey || e.metaKey, shift = e.shiftKey, alt = e.altKey;
  const win = INFO.platform === 'windows';
  const p = current?.active;
  const map = [
    [ctrl && shift && /^t$/i.test(k), () => newTabHere()],
    [ctrl && shift && /^w$/i.test(k), () => p ? closePane(p) : null],
    [ctrl && !alt && k === 'Tab', () => cycleTab(shift ? -1 : 1)],
    [ctrl && !shift && k === 'PageDown', () => cycleTab(1)],
    [ctrl && !shift && k === 'PageUp', () => cycleTab(-1)],
    [ctrl && shift && /^c$/i.test(k), () => copy()],
    [ctrl && shift && /^v$/i.test(k), () => paste()],
    [shift && !ctrl && k === 'Insert', () => paste()],
    [ctrl && !shift && k === 'Insert', () => copy()],
    [ctrl && shift && /^f$/i.test(k), () => openFind()],
    [ctrl && shift && /^d$/i.test(k), () => toggleSplit()],
    [ctrl && shift && k === 'ArrowRight', () => cyclePane(1)],
    [ctrl && shift && k === 'ArrowLeft', () => cyclePane(-1)],
    [ctrl && !alt && (k === '=' || k === '+'), () => zoom(1)],
    [ctrl && !alt && (k === '-' || k === '_'), () => zoom(-1)],
    [ctrl && !alt && !shift && k === '0', () => zoom(0)],
    [ctrl && !shift && k === ',', () => openSettings()],
    [!ctrl && !alt && k === 'F6', () => focusChrome()],
    // Windows Terminal's habits on Windows: Ctrl+V pastes, Ctrl+C copies when something is selected.
    [win && ctrl && !shift && !alt && /^v$/i.test(k), () => paste()],
    [win && ctrl && !shift && !alt && /^c$/i.test(k) && !!p?.term.hasSelection(), () => { copy(); p.term.clearSelection(); }],
  ];
  for (const [hit, act] of map) {
    if (hit) { if (e.type === 'keydown') act(); return true; }
  }
  return false;
}

function keyFromTerminal(e, pane) {
  if (shortcut(e)) { e.preventDefault(); return false; }
  return true;
}

// Keys outside the terminals: shortcuts, Esc to go back, and the arrows moving between buttons (D-pad).
document.addEventListener('keydown', e => {
  const inTerm = e.target?.closest?.('.xterm');
  if (inTerm) return;   // keyFromTerminal handles those
  const inInput = e.target?.tagName === 'INPUT';
  if (e.target === $('findInput')) {
    if (e.key === 'Enter') { e.preventDefault(); find(e.shiftKey); return; }
    if (e.key === 'Escape') { e.preventDefault(); closeFind(); return; }
  }
  if (shortcut(e)) { e.preventDefault(); return; }
  if (e.key === 'Escape') {
    e.preventDefault();
    if (!$('menu').hidden) { closeMenu(); focusTerminal(); }
    else if (!$('settings').hidden) closeSettings();
    else if (!$('findbar').hidden) closeFind();
    else focusTerminal();
    return;
  }
  const dir = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' }[e.key];
  if (dir && !(inInput && (dir === 'left' || dir === 'right'))) {
    e.preventDefault();
    moveFocus(dir);
    return;
  }
  if (e.key === 'ContextMenu') { e.preventDefault(); const r = document.activeElement.getBoundingClientRect(); contextMenu(r.left, r.bottom); }
});

$('findInput').addEventListener('input', () => find(false));

// ------------------------------------------------------------------ moving between buttons (keyboard, D-pad)
/// The buttons you can move between right now: inside the open menu or settings, else the bars.
function focusables() {
  const layer = !$('menu').hidden ? $('menu') : !$('settings').hidden ? $('settings') : document.getElementById('app');
  return [...layer.querySelectorAll('[data-f]')].filter(el => !el.disabled && el.offsetParent !== null && !el.closest('[hidden]'));
}

function focusChrome() {
  const active = document.querySelector('.tab.active .tt') || focusables()[0];
  active?.focus();
}

/// The nearest button in a direction, by where things are on screen.
function moveFocus(dir) {
  const all = focusables();
  const cur = document.activeElement;
  if (!all.includes(cur)) { all[0]?.focus(); return; }
  const a = cur.getBoundingClientRect();
  const ax = a.left + a.width / 2, ay = a.top + a.height / 2;
  let best = null, bestScore = Infinity;
  for (const el of all) {
    if (el === cur) continue;
    const b = el.getBoundingClientRect();
    const bx = b.left + b.width / 2, by = b.top + b.height / 2;
    const dx = bx - ax, dy = by - ay;
    const along = dir === 'left' ? -dx : dir === 'right' ? dx : dir === 'up' ? -dy : dy;
    const across = dir === 'left' || dir === 'right' ? Math.abs(dy) : Math.abs(dx);
    if (along <= 1) continue;
    const score = along + across * 2.5;
    if (score < bestScore) { bestScore = score; best = el; }
  }
  // The terminal sits between the top bars and the quick keys: moving across it lands in it.
  const acrossTerminal = best && ((dir === 'down' && cur.closest('#bar, #findbar') && best.closest('#quick'))
    || (dir === 'up' && cur.closest('#quick') && best.closest('#bar, #findbar')));
  if (acrossTerminal) focusTerminal();
  else if (best) { best.focus(); best.scrollIntoView({ block: 'nearest', inline: 'nearest' }); }
  // Down from the bars: back into the terminal.
  else if (dir === 'down' && $('menu').hidden && $('settings').hidden) focusTerminal();
}

// ------------------------------------------------------------------ buttons and the mouse
$('btnNew').addEventListener('click', () => newTabHere());
$('btnShells').addEventListener('click', shellMenu);
$('btnSplit').addEventListener('click', toggleSplit);
$('btnFind').addEventListener('click', () => $('findbar').hidden ? openFind() : closeFind());
$('btnKeys').addEventListener('click', () => { S.quickKeys = !S.quickKeys; applySettings(); saveSettings(); focusTerminal(); });
$('btnOsk').addEventListener('pointerdown', e => e.preventDefault());
$('btnOsk').addEventListener('click', toggleOsk);
$('btnSettings').addEventListener('click', () => $('settings').hidden ? openSettings() : closeSettings());
$('settingsClose').addEventListener('click', closeSettings);
$('findPrev').addEventListener('click', () => find(true));
$('findNext').addEventListener('click', () => find(false));
$('findClose').addEventListener('click', closeFind);

document.addEventListener('contextmenu', e => {
  e.preventDefault();
  if (e.target.closest('#panes')) contextMenu(e.clientX, e.clientY);
});
document.addEventListener('mousedown', e => {
  if (!$('menu').hidden && !e.target.closest('#menu')) closeMenu();
  if (!$('settings').hidden && !e.target.closest('#settings') && !e.target.closest('#btnSettings')) closeSettings();
});
// Middle-click pastes on Linux, as in other Linux terminals (the selection was copied when made).
document.addEventListener('auxclick', e => {
  if (e.button === 1 && INFO.platform === 'linux' && e.target.closest('#panes')) {
    const text = lastSelection?.term.getSelection();
    if (text) paste(text);
  }
});

new ResizeObserver(() => requestAnimationFrame(() => fitTab(current))).observe($('panes'));

// Files dropped on the window: their paths typed into the terminal, quoted.
function hookDrop() {
  const ev = T.event;
  ev.listen('tauri://drag-enter', () => { $('drop').hidden = false; });
  ev.listen('tauri://drag-leave', () => { $('drop').hidden = true; });
  ev.listen('tauri://drag-drop', e => {
    $('drop').hidden = true;
    const paths = e.payload?.paths || [];
    const p = current?.active;
    if (p && paths.length) { send(p, paths.map(quotePath).join(' ') + ' '); focusTerminal(); }
  });
}

// ------------------------------------------------------------------ start
async function start() {
  INFO = await invoke('init');
  S = { ...DEFAULTS, ...(INFO.settings || {}) };
  if (!THEMES[S.theme]) S.theme = DEFAULTS.theme;
  S.opacity = Math.max(0.3, Math.min(1, Number(S.opacity) || 1));
  seeThroughWindow = S.opacity < 0.999;
  $('version').textContent = `Hearth Terminal ${INFO.version}`;
  applySettings();
  hookDrop();
  // The terminal measures its letters once: wait for the font so the grid is right from the start.
  try { await Promise.race([document.fonts.load(`${S.fontSize}px "JetBrains Mono"`), new Promise(r => setTimeout(r, 1500))]); } catch (e) {}
  const l = INFO.launch || {};
  await newTab({ cwd: l.cwd || '', command: l.command || null });
  window.hearthReady = true;
}

start().catch(e => {
  document.body.insertAdjacentHTML('beforeend', `<pre style="position:fixed;inset:60px 20px auto;color:#ff5d5d;white-space:pre-wrap">Terminal couldn't start: ${String(e).replace(/</g, '&lt;')}</pre>`);
});
