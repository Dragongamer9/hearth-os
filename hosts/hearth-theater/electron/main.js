'use strict';
// Hearth Theater's windows, drawn by Chromium. Hearth's own Electron build (/usr/lib/hearth-electron, hosts/electron)
// runs this when started with --hearth-theater (by /usr/bin/hearth-theater, which points its output at Theater's
// log); tests run it with a development Electron. Everything Theater keeps and decides is its core's
// (`hearth-theater --core-stdio`, src/main.rs), started here and spoken to one JSON object a line.
//
//   The main window: Theater's home screen (ui/index.html; preload.js gives it the calls it used to have from Tauri).
//   The viewer: the site, full screen, as a view inside the main window. Nothing of Theater's is put into the site's
//     pages. The controller is read by Theater's own page underneath (ui/viewer-pad.js), and the pointer it moves
//     is a real mouse made by the core (uinput, or XTest), so every click is the system's own: frames from other
//     sites, video players, popups and bot checks all take it. A hint bar and messages show in small views on top.
//   The reader: a hidden window that loads a site's page with Theater's cookies and runs src/reader.js in an
//     isolated world (invisible to the page), never on bot-check pages.

const { app, BrowserWindow, WebContentsView, protocol, session, ipcMain } = require('electron');
const { spawn } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');
const rules = require('./rules');

const ARGS = process.argv.slice(1);
const FULL = ARGS.includes('--fullscreen');
const READ_ALL = ARGS.includes('--read-catalogs');
const CORE = process.env.HEARTH_THEATER_CORE || '/usr/bin/hearth-theater';
const ROOT = path.join(__dirname, '..');                 // ui/ and src/ (the same next to this file when packaged)
const UI = path.join(ROOT, 'ui');
const READER_JS = (() => { try { return fs.readFileSync(path.join(ROOT, 'src', 'reader.js'), 'utf8'); } catch { return ''; } })();
const DATA = process.env.HEARTH_THEATER_DIR || path.join(process.env.XDG_DATA_HOME || path.join(os.homedir(), '.local', 'share'), 'hearth-theater');
const READER_WORLD = 1077;

function log(t) { try { process.stderr.write(`${Date.now()} [theater window] ${t}\n`); } catch { /* no log */ } }

// ------------------------------------------------------------------ before Chromium starts
app.setName('Hearth Theater');
// Theater's own browser profile: its cookies and sign-ins, kept between runs (never Hearth's own window's).
app.setPath('userData', path.join(DATA, 'chromium'));
// Drawing keeps going under gamescope's overlays, as Hearth's own Chromium window does.
app.commandLine.appendSwitch('disable-renderer-backgrounding');
app.commandLine.appendSwitch('disable-backgrounding-occluded-windows');
app.commandLine.appendSwitch('disable-background-timer-throttling');
app.userAgentFallback = rules.userAgent(process.versions.chrome);
protocol.registerSchemesAsPrivileged([{ scheme: 'theater', privileges: { standard: true, secure: true, supportFetchAPI: true } }]);
if (!READ_ALL && !app.requestSingleInstanceLock()) { log('Theater is already open; showing it'); app.exit(0); }

process.on('uncaughtException', e => log('problem: ' + (e && e.stack || e)));

// ------------------------------------------------------------------ the core
let core = null, leaving = false, win = null;
const calls = new Map();
let callId = 0;

function send(obj) { if (core && core.stdin.writable) { try { core.stdin.write(JSON.stringify(obj) + '\n'); } catch { /* gone */ } } }

function startCore() {
  const args = rules.coreArgs(ARGS);
  log(`core: starting ${CORE} ${args.join(' ')}`);
  core = spawn(CORE, args, { stdio: ['pipe', 'pipe', 'inherit'] });
  core.on('error', e => { log('core: ' + e.message); quit(1); });
  core.stdin.on('error', e => log('core input: ' + e.message));
  core.stdout.setEncoding('utf8');
  let buf = '';
  core.stdout.on('data', chunk => {
    buf += chunk;
    let i;
    while ((i = buf.indexOf('\n')) >= 0) {
      const l = buf.slice(0, i).trim(); buf = buf.slice(i + 1);
      if (!l.startsWith('{')) continue;
      try { fromCore(JSON.parse(l)); } catch (e) { log('core said something odd: ' + e.message); }
    }
  });
  core.on('exit', code => { log('core: ended (' + code + ')'); quit(code ? 1 : 0); });
}

function quit(code = 0) {
  if (leaving) return;
  leaving = true;
  try { if (core) core.stdin.end(); } catch { /* gone */ }
  app.exit(code);
}

function fromCore(m) {
  switch (m.t) {
    case 'reply': { const r = calls.get(m.id); if (r) { calls.delete(m.id); r(m); } return; }
    case 'event': toPage('theater:event', m.name, m.payload); return;
    case 'read': return startRead(m);
    case 'read-stop': return stopRead(m.id);
    case 'log': log('core: ' + m.text); return;
    case 'do':
      if (m.op === 'viewer') openViewer(m);
      else if (m.op === 'close-viewer') closeViewer();
      else if (m.op === 'quit') quit(0);
      return;
  }
}

function toPage(ch, ...a) { if (win && !win.isDestroyed()) win.webContents.send(ch, ...a); }

ipcMain.handle('theater:call', (e, cmd, args) => {
  if (!win || e.sender !== win.webContents) return { ok: false, error: 'not Theater' };
  return new Promise(res => { const id = ++callId; calls.set(id, res); send({ t: 'call', id, cmd: String(cmd), args: args || {} }); });
});

// ------------------------------------------------------------------ the main window
function serveUi() {
  protocol.handle('theater', req => {
    let rel;
    try { rel = decodeURIComponent(new URL(req.url).pathname).replace(/^\/+/, '') || 'index.html'; } catch { return new Response('', { status: 404 }); }
    const file = path.normalize(path.join(UI, rel));
    if (!file.startsWith(UI + path.sep)) return new Response('', { status: 404 });
    const type = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.svg': 'image/svg+xml', '.png': 'image/png', '.txt': 'text/plain; charset=utf-8' }[path.extname(file)] || 'application/octet-stream';
    return fs.promises.readFile(file).then(b => new Response(b, { headers: { 'content-type': type } })).catch(() => new Response('', { status: 404 }));
  });
}

function makeMain() {
  win = new BrowserWindow({
    title: 'Hearth Theater', width: 1280, height: 800, minWidth: 640, minHeight: 440, fullscreen: FULL, backgroundColor: '#0f0b09', autoHideMenuBar: true, show: false,
    webPreferences: { preload: path.join(__dirname, 'preload.js'), contextIsolation: true, sandbox: true, nodeIntegration: false, backgroundThrottling: false, spellcheck: false },
  });
  win.on('page-title-updated', e => e.preventDefault());
  win.once('ready-to-show', () => { win.show(); log('main window: shown'); });
  win.webContents.on('will-navigate', (e, url) => { if (!url.startsWith('theater://')) e.preventDefault(); });
  win.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
  win.webContents.on('did-finish-load', () => {
    log('main window: home screen loaded');
    // A viewer opened before the page was there (hearth-theater --open): the controller is its.
    if (viewer) toPage('theater:viewer-event', { type: 'open' });
  });
  win.webContents.on('did-fail-load', (_e, code, desc, url) => log(`main window: didn't load ${url} (${code} ${desc})`));
  win.webContents.on('console-message', (e) => { const lvl = e.level ?? e.params?.level; if (lvl === 'error' || lvl === 3) log('home screen: ' + (e.message || e.params?.message || '')); });
  win.webContents.on('render-process-gone', (_e, d) => { log('main window: page gone (' + d.reason + '), loading it again'); if (!win.isDestroyed()) win.webContents.reload(); });
  win.on('resize', layout);
  win.on('closed', () => { win = null; quit(0); });
  win.loadURL('theater://app/index.html' + (FULL ? '#full' : '')).catch(e => log('main window: ' + e.message));
  // Electron's single-instance: Theater opened again shows this one.
  app.on('second-instance', () => { if (win) { if (win.isMinimized()) win.restore(); win.show(); win.focus(); } });
}

// ------------------------------------------------------------------ the viewer
let viewer = null;        // { view, title, oskBand: {top, h} | null, popups: Set<BrowserWindow> }
const hud = { bar: null, toast: null, barT: 0, toastT: 0 };

function webPrefs() {
  return { contextIsolation: true, sandbox: true, nodeIntegration: false, backgroundThrottling: false, autoplayPolicy: 'no-user-gesture-required', spellcheck: false };
}

/** Logging and safety for any page the viewer shows (the viewer itself and its popups). */
function watchPage(wc, what) {
  wc.on('did-start-navigation', d => { if (d.isMainFrame && !d.isSameDocument) log(`${what}: going to ${rules.logUrl(d.url)}`); });
  wc.on('did-finish-load', () => log(`${what}: loaded ${rules.logUrl(wc.getURL())}${rules.isChallenge(wc.getURL(), wc.getTitle()) ? ' (a bot check: nothing of Theater\'s runs on it)' : ''}`));
  wc.on('did-fail-load', (_e, code, desc, url, main) => { if (code !== -3) log(`${what}: ${main ? 'page' : 'frame'} didn't load: ${rules.logUrl(url)} (${code} ${desc})`); });
  wc.on('page-title-updated', (_e, t) => log(`${what}: page title "${String(t).slice(0, 80)}"`));
  wc.on('render-process-gone', (_e, d) => { log(`${what}: page gone (${d.reason}); loading it again`); if (!wc.isDestroyed()) wc.reload(); });
  wc.on('certificate-error', (_e, url, err) => log(`${what}: certificate problem on ${rules.logUrl(url)} (${err})`));
  // Web pages only (no file:, no other programs' links).
  wc.on('will-navigate', (e, url) => { if (!/^(https?|about|blob|data):/i.test(url)) { e.preventDefault(); log(`${what}: refused a link to ${rules.logUrl(url)}`); } });
  wc.on('will-attach-webview', e => e.preventDefault());
  wc.setWindowOpenHandler(({ url, disposition }) => {
    const p = rules.popupPolicy(disposition, url, wc.getURL());
    log(`${what}: asked for a new window (${disposition}) for ${rules.logUrl(url)}: ${p}`);
    if (p === 'here') { wc.loadURL(url).catch(() => {}); return { action: 'deny' }; }
    if (p === 'deny') { hudToast('A page tried to open another tab (left closed)'); return { action: 'deny' }; }
    return { action: 'allow', overrideBrowserWindowOptions: { width: 900, height: 760, autoHideMenuBar: true, backgroundColor: '#ffffff', webPreferences: webPrefs() } };
  });
  wc.on('did-create-window', child => {
    if (!viewer) { child.destroy(); return; }
    viewer.popups.add(child);
    log(`${what}: opened a popup window`);
    watchPage(child.webContents, 'popup');
    child.on('page-title-updated', e => e.preventDefault());
    child.on('closed', () => { if (viewer) viewer.popups.delete(child); log('popup: closed'); });
  });
}

function openViewer(m) {
  if (!win || win.isDestroyed()) return;
  closeViewer(true);
  const view = new WebContentsView({ webPreferences: webPrefs() });
  view.setBackgroundColor('#000000');
  viewer = { view, title: m.title, oskBand: null, popups: new Set(), full: false };
  win.contentView.addChildView(view);
  for (const k of ['bar', 'toast']) {
    if (!hud[k] || hud[k].webContents.isDestroyed()) {
      hud[k] = new WebContentsView({ webPreferences: { contextIsolation: true, sandbox: true, backgroundThrottling: false } });
      hud[k].setBackgroundColor('#00000000');
      hud[k].webContents.loadURL(`theater://app/hud.html#${k}`).catch(() => {});
    }
    win.contentView.addChildView(hud[k]);   // on top of the site
  }
  layout();
  watchPage(view.webContents, 'viewer');
  view.webContents.on('enter-html-full-screen', () => { if (viewer) viewer.full = true; log('viewer: the page went full screen'); });
  view.webContents.on('leave-html-full-screen', () => { if (viewer) viewer.full = false; log('viewer: the page left full screen'); });
  if (process.env.HEARTH_THEATER_TEST_CLICK) testClick(view.webContents);
  view.webContents.loadURL(m.url).catch(e => log('viewer: ' + e.message));
  view.webContents.focus();
  win.setTitle(m.title || 'Hearth Theater');
  toPage('theater:viewer-event', { type: 'open' });
  hudBar(5000);
}

function closeViewer(replacing = false) {
  if (!viewer) return;
  const v = viewer;
  viewer = null;
  for (const p of v.popups) { try { p.destroy(); } catch { /* gone */ } }
  try { win.contentView.removeChildView(v.view); } catch { /* gone */ }
  try { v.view.webContents.close(); } catch { /* gone */ }
  for (const k of ['bar', 'toast']) if (hud[k]) { hud[k].setBounds({ x: 0, y: 0, width: 0, height: 0 }); }
  log('viewer: closed');
  if (replacing || !win || win.isDestroyed()) return;
  win.setTitle('Hearth Theater');
  toPage('theater:viewer-event', { type: 'closed' });
  toPage('theater:event', 'closed', {});
  win.webContents.focus();
}

function layout() {
  if (!win || win.isDestroyed() || !viewer) return;
  const [w, h] = win.getContentSize();
  const band = viewer.oskBand;
  if (!band) viewer.view.setBounds({ x: 0, y: 0, width: w, height: h });
  else if (band.top <= 2) viewer.view.setBounds({ x: 0, y: Math.round(band.h), width: w, height: Math.max(100, h - Math.round(band.h)) });
  else viewer.view.setBounds({ x: 0, y: 0, width: w, height: Math.max(100, Math.round(band.top)) });
  if (hud.bar) hud.bar.setBounds(hud.barT > Date.now() ? { x: 0, y: h - 110, width: w, height: 110 } : { x: 0, y: 0, width: 0, height: 0 });
  if (hud.toast) hud.toast.setBounds(hud.toastT > Date.now() ? { x: 0, y: 0, width: w, height: 80 } : { x: 0, y: 0, width: 0, height: 0 });
}

function hudRun(v, js) {
  const go = () => v.webContents.executeJavaScript(js).catch(() => {});
  if (v.webContents.isLoading()) v.webContents.once('did-finish-load', go); else go();
}

function hudBar(ms) {
  if (!viewer || !hud.bar) return;
  hud.barT = Date.now() + ms;
  hudRun(hud.bar, 'window.show && show()');
  layout();
  setTimeout(layout, ms + 50);
}

function hudToast(text, ms = 1800) {
  if (!viewer || !hud.toast) return;
  hud.toastT = Date.now() + ms;
  hudRun(hud.toast, `window.show && show(${JSON.stringify(String(text))})`);
  layout();
  setTimeout(layout, ms + 50);
}

/** The page the controller acts on: a popup in front, or the viewer. */
function front() {
  if (!viewer) return null;
  for (const p of viewer.popups) if (!p.isDestroyed() && p.isFocused()) return { wc: p.webContents, popup: p };
  return { wc: viewer.view.webContents, popup: null };
}

// Play/pause, skipping and filling the screen act on the biggest video on screen, in whichever frame it is. They run
// only when asked (X, Y, ◀ ▶), leave nothing behind, and never touch bot checks or captchas.
const BIGGEST = `(() => { const vis = v => { const r = v.getBoundingClientRect(); return r.width > 40 && r.height > 30 && r.bottom > 0 && r.right > 0 && r.top < innerHeight && r.left < innerWidth ? r.width * r.height : 0; };
  let best = 0; for (const v of document.querySelectorAll('video')) best = Math.max(best, vis(v)); return best; })()`;
const ACT = what => `(() => { const vis = v => { const r = v.getBoundingClientRect(); return r.width > 40 && r.height > 30 && r.bottom > 0 && r.right > 0 && r.top < innerHeight && r.left < innerWidth ? r.width * r.height : 0; };
  const v = [...document.querySelectorAll('video')].sort((a, b) => vis(b) - vis(a))[0]; if (!v) return 'none'; const w = ${JSON.stringify(what)};
  if (w.do === 'toggle') { if (v.paused) { const p = v.play(); if (p && p.catch) p.catch(() => {}); return 'playing'; } v.pause(); return 'paused'; }
  if (w.do === 'seek') { try { v.currentTime = Math.max(0, Math.min((v.duration || 1e9) - .5, v.currentTime + w.s)); } catch (e) {} return 'seeked'; }
  if (w.do === 'fill') { if (document.fullscreenElement) { document.exitFullscreen().catch(() => {}); return 'unfilled'; } const p = v.requestFullscreen ? v.requestFullscreen() : null; if (p && p.catch) p.catch(() => {}); return 'filled'; }
  return 'none'; })()`;

async function media(what) {
  const f = front();
  if (!f) return;
  const top = f.wc.mainFrame;
  if (rules.isChallenge(f.wc.getURL(), f.wc.getTitle())) { hudToast('This is a bot check: use the pointer'); return; }
  let best = null, area = 0;
  for (const fr of top.framesInSubtree) {
    if (rules.isCaptchaUrl(fr.url) || !/^https?:/.test(fr.url || '')) continue;
    const a = await fr.executeJavaScript(BIGGEST).catch(() => 0);
    if (a > area) { area = a; best = fr; }
  }
  if (!best) { hudToast('No video on screen yet'); return; }
  const r = await best.executeJavaScript(ACT(what), true).catch(e => 'error ' + e.message);
  log(`viewer: ${what.do} → ${r}`);
  if (what.do === 'fill') hudToast(r === 'filled' ? 'Filling the screen (Y to undo)' : 'Back to the page');
}

ipcMain.on('theater:viewer', (e, op, a = {}) => {
  if (!win || e.sender !== win.webContents || !viewer) return;
  const f = front();
  switch (op) {
    case 'close': closeViewer(); break;
    case 'back':
      // A page's own full screen is left the way a keyboard leaves it (Esc).
      if (!f.popup && viewer.full) send({ t: 'ptr', op: 'key', key: 'escape' });
      else if (f.wc.navigationHistory.canGoBack()) f.wc.navigationHistory.goBack();
      else if (f.popup) f.popup.close();
      else hudToast('View: back to Theater');
      break;
    case 'forward': if (f.wc.navigationHistory.canGoForward()) f.wc.navigationHistory.goForward(); break;
    case 'media': media(a).catch(err => log('viewer: ' + err.message)); break;
    case 'hints': hudBar(6000); break;
    case 'toast': hudToast(String(a.text || '')); break;
    // The on-screen keyboard (Theater's own page, docked under the site): the site gives it room, and what's
    // typed goes into the field you clicked, as typing does.
    case 'osk':
      viewer.oskBand = a.on ? { top: Number(a.top) || 0, h: Number(a.h) || 0 } : null;
      layout();
      f.wc.focus();
      log(`viewer: on-screen keyboard ${a.on ? 'open' : 'closed'}`);
      break;
    case 'type': if (typeof a.text === 'string' && a.text) { f.wc.focus(); f.wc.insertText(a.text).catch?.(() => {}); } break;
  }
});

// The pointer: the page reading the controller says how to move and click; the core makes it real.
ipcMain.on('theater:ptr', (e, m) => {
  if (!win || e.sender !== win.webContents || !viewer || !m || typeof m !== 'object') return;
  if (!['move', 'button', 'wheel', 'key'].includes(m.op)) return;
  send({ ...m, t: 'ptr' });
});

/** Tests (apps-create.yml): with the page loaded, put the pointer next to (x, y) of the page, move it there, and
 *  press A the way the controller does. The page itself says whether a real click arrived. */
function testClick(wc) {
  const [x, y] = String(process.env.HEARTH_THEATER_TEST_CLICK).split(',').map(Number);
  wc.once('did-finish-load', () => setTimeout(() => {
    if (!win || win.isDestroyed()) return;
    const b = win.getContentBounds();
    log(`test: pointer to the page's (${x}, ${y}), the screen's (${b.x + x}, ${b.y + y}), then A`);
    send({ t: 'ptr', op: 'warp', x: b.x + x - 30, y: b.y + y - 10 });
    setTimeout(() => send({ t: 'ptr', op: 'move', dx: 30, dy: 10 }), 200);
    setTimeout(() => win.webContents.executeJavaScript('window.__theaterFakePad = { a: true }').catch(() => {}), 600);
    setTimeout(() => win.webContents.executeJavaScript('window.__theaterFakePad = null').catch(() => {}), 800);
  }, 2500));
}

// ------------------------------------------------------------------ the reader (hidden)
let reader = null;   // { id, win, done, challengeT }

function stopRead(id) {
  if (!reader || (id != null && reader.id !== id)) return;
  const r = reader;
  reader = null;
  clearTimeout(r.challengeT);
  try { r.win.destroy(); } catch { /* gone */ }
}

function startRead(m) {
  stopRead();
  const w = new BrowserWindow({ show: false, width: 1280, height: 800, webPreferences: { contextIsolation: true, sandbox: true, nodeIntegration: false, backgroundThrottling: false, autoplayPolicy: 'user-gesture-required', spellcheck: false } });
  w.webContents.setAudioMuted(true);
  const r = reader = { id: m.id, win: w, done: false, challengeT: 0 };
  const done = (msg, text) => { if (r.done) return; r.done = true; clearTimeout(r.challengeT); send({ t: 'read-done', id: m.id, ...msg }); log(`reader: ${text}`); };
  const wc = w.webContents;
  wc.setWindowOpenHandler(() => ({ action: 'deny' }));
  wc.on('will-navigate', (e, url) => { if (!/^https?:/i.test(url)) e.preventDefault(); });
  wc.on('will-attach-webview', e => e.preventDefault());
  const run = async () => {
    if (r.done || w.isDestroyed()) return;
    const url = wc.getURL(), title = wc.getTitle();
    if (rules.isChallenge(url, title)) {
      log(`reader: ${rules.logUrl(url)} is a bot check; waiting for it to pass (nothing of Theater's runs on it)`);
      if (!r.challengeT) r.challengeT = setTimeout(() => done({ error: 'The site showed a bot check. Open it in the viewer once (pass the check there), then choose Refresh now.' }, `gave up on the bot check at ${rules.logUrl(url)}`), 25000);
      return;
    }
    try {
      const code = `new Promise(res => { window.__hearthReaderMode = ${JSON.stringify(m.mode === 'search' ? 'search' : 'catalog')}; window.__hearthReaderSink = res;\n${READER_JS}\n})`;
      const out = await wc.executeJavaScriptInIsolatedWorld(READER_WORLD, [{ code }]);
      if (out && typeof out === 'object') done({ data: JSON.stringify(out) }, `read ${rules.logUrl(url)} (${(out.rows || []).length} rows, ${(out.items || []).length} results${out.error ? ', ' + out.error : ''})`);
    } catch (e) { log(`reader: on ${rules.logUrl(url)}: ${e.message}`); }
  };
  wc.on('did-finish-load', () => { run(); });
  wc.on('did-fail-load', (_e, code, desc, url, main) => { if (main && code !== -3) done({ error: `The site didn't load (${desc})` }, `couldn't load ${rules.logUrl(url)} (${code} ${desc})`); });
  wc.on('render-process-gone', (_e, d) => done({ error: 'The page stopped while it was read' }, `page gone (${d.reason})`));
  log(`reader: reading ${rules.logUrl(m.url)} (${m.mode})`);
  w.loadURL(m.url).catch(() => {});
}

// ------------------------------------------------------------------ start
app.on('web-contents-created', (_e, wc) => wc.on('will-attach-webview', e => e.preventDefault()));
app.on('child-process-gone', (_e, d) => log(`chromium: its ${d.type} process ended (${d.reason}, ${d.exitCode})`));
app.on('window-all-closed', () => { if (!READ_ALL) quit(0); });

app.whenReady().then(() => {
  log(`started: Electron ${process.versions.electron}, Chromium ${process.versions.chrome}${FULL ? ', full screen' : ''}${READ_ALL ? ', reading catalogs' : ''}; profile ${app.getPath('userData')}`);
  const ses = session.defaultSession;
  ses.setPermissionRequestHandler((_wc, p, cb) => cb(rules.allowPermission(p)));
  ses.setPermissionCheckHandler((_wc, p) => rules.allowPermission(p));
  // What draws: the graphics card and how (the first thing to read when Theater shows nothing).
  const gpu = app.getGPUFeatureStatus();
  log(`renderer: Chromium (Hearth's Electron), compositing ${gpu.gpu_compositing}, WebGL ${gpu.webgl}, video decode ${gpu.video_decode}, rasterization ${gpu.rasterization}`);
  app.getGPUInfo('complete').then(i => {
    const dev = (i.gpuDevice || []).find(d => d.active) || (i.gpuDevice || [])[0] || {};
    log(`renderer: GPU vendor 0x${(dev.vendorId || 0).toString(16)} device 0x${(dev.deviceId || 0).toString(16)}${dev.driverVendor ? ' ' + dev.driverVendor : ''}${dev.driverVersion ? ' ' + dev.driverVersion : ''}; GL ${i.auxAttributes?.glRenderer || '?'} (${i.auxAttributes?.glImplementationParts || i.auxAttributes?.glVersion || '?'})`);
  }).catch(e => log('renderer: no GPU details (' + e.message + ')'));
  serveUi();
  if (!READ_ALL) makeMain();
  startCore();
});
