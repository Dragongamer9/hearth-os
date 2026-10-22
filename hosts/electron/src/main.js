'use strict';
// Hearth's window, drawn by Chromium. Everything Hearth does is the Hearth core's (the same program as the usual
// Hearth: hearth-core and Game Mode), started windowless as `nimbus --core-stdio`; this app only shows the screens and
// does what the core asks of its windows. The channel is described in hosts/hearth-rs/src/winhost.rs; why there are
// two engines, in docs/RENDERERS.md. Started by the `nimbus` program itself when Chromium is the engine to use
// (on Nvidia graphics, or when chosen in Settings › Display › Drawing engine), so the session and its watchdog
// don't change.

const { app, BrowserWindow, dialog, ipcMain, protocol, globalShortcut, screen } = require('electron');
const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');
const ch = require('./channel');

const ARGS = process.argv.slice(1);
const WINDOWED = ARGS.includes('--windowed');
const IN_SESSION = process.env.NIMBUS_SESSION === '1';
const CORE = process.env.HEARTH_CORE_BIN || '/usr/lib/nimbus/nimbus';
const RES = app.isPackaged ? process.resourcesPath : path.join(__dirname, '..');
const UI = app.isPackaged ? path.join(RES, 'ui') : path.join(__dirname, '..', '..', '..', 'ui');
const JS = app.isPackaged ? path.join(RES, 'js') : path.join(__dirname, '..', '..', 'hearth-rs', 'src', 'js');
const MAIN_TITLE = 'Hearth';
const OVERLAY_TITLE = 'Hearth Overlay';
const EDGE_UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36 Edg/138.0.0.0';

function log(t) { process.stderr.write('[hearth-electron] ' + t + '\n'); }
const script = name => { try { return fs.readFileSync(path.join(JS, name), 'utf8'); } catch { return ''; } };

// Chromium kept drawing at full speed in gamescope (it thought windows under the overlay were hidden).
app.commandLine.appendSwitch('disable-renderer-backgrounding');
app.commandLine.appendSwitch('disable-backgrounding-occluded-windows');
app.commandLine.appendSwitch('disable-background-timer-throttling');

protocol.registerSchemesAsPrivileged([
  { scheme: 'nimbus', privileges: { standard: true, secure: true, supportFetchAPI: true, corsEnabled: true } },
  { scheme: 'nimbus-art', privileges: { standard: true, secure: true, supportFetchAPI: true, corsEnabled: true, bypassCSP: true } },
]);

// ------------------------------------------------------------------ the core
let core = null;
let leaving = false;
const wins = {};                 // label → BrowserWindow (main, overlay, signin, stream)
const artWait = new Map();       // id → resolve
let artId = 1;

function send(obj) { if (core && core.stdin.writable) core.stdin.write(ch.line(obj)); }

function startCore() {
  core = spawn(CORE, ch.coreArgs(ARGS), { stdio: ['pipe', 'pipe', 'inherit'], env: { ...process.env, HEARTH_RENDERER_RUNNING: 'chromium' } });
  // Hearth's core missing or not startable: say so and leave (the session's watchdog takes over), rather than an
  // error box over an empty window. Writing to a core that just died (EPIPE) mustn't do that either.
  core.on('error', e => { log('core: ' + e.message); if (!leaving) { leaving = true; app.exit(1); } });
  core.stdin.on('error', e => log('core input: ' + e.message));
  core.stdout.setEncoding('utf8');
  core.stdout.on('data', ch.lineReader(l => { const m = ch.parse(l); if (m) fromCore(m); }));
  core.on('exit', code => {
    log('core exited (' + code + ')');
    if (!leaving) { leaving = true; app.exit(code == null ? 1 : code); }
  });
}

function fromCore(m) {
  switch (m.t) {
    case 'msg':
      for (const label of ch.targets(m, Object.keys(wins))) {
        const w = wins[label];
        if (w && !w.isDestroyed()) w.webContents.send('nimbus:msg', m.msg);
      }
      return;
    case 'do': return act(m);
    case 'ask': return ask(m);
    case 'art': { const r = artWait.get(m.id); if (r) { artWait.delete(m.id); r(m); } return; }
  }
}

// ------------------------------------------------------------------ what the core asks of the windows
function act(m) {
  const w = m.win && wins[m.win] && !wins[m.win].isDestroyed() ? wins[m.win] : null;
  switch (m.op) {
    // The overlay is shown without taking the keyboard from the game (some games stall when they lose it).
    case 'show': if (w) { if (m.win === 'overlay') w.showInactive(); else w.show(); } break;
    case 'hide': if (w) w.hide(); break;
    case 'focus': if (w) w.focus(); break;
    case 'unminimize': if (w && w.isMinimized()) w.restore(); break;
    case 'eval': if (w) w.webContents.executeJavaScript(String(m.js || '')).catch(() => {}); break;
    case 'destroy': if (w) w.destroy(); break;
    case 'exit': leaving = true; app.exit(Number(m.code) || 0); break;
    case 'restart': restart(); break;
    case 'zoom': { const p = Number(m.pct); zoomPct = p > 0 && p <= 500 ? p : 100; applyZoom(); break; }
    case 'overlay.create': makeOverlay(); break;
    case 'hotkey':
      // Ctrl+Alt+Q, the keyboard's Guide button.
      try { globalShortcut.register('Control+Alt+Q', () => send({ t: 'hotkey' })); } catch (e) { log('Ctrl+Alt+Q: ' + e.message); }
      break;
    case 'stream.open': openStream(m); break;
    case 'stream.close': if (wins.stream && !wins.stream.isDestroyed()) wins.stream.destroy(); break;
    case 'signin.open': openSignIn(m); break;
    case 'signin.close': closeSignIn(); break;
  }
}

async function ask(m) {
  let value = null;
  try {
    const parent = wins.main && !wins.main.isDestroyed() ? wins.main : undefined;
    if (m.op === 'pickFile') {
      const r = await dialog.showOpenDialog(parent, { title: m.title || '', properties: ['openFile'] });
      value = r.canceled ? null : (r.filePaths[0] || null);
    } else if (m.op === 'pickFolder') {
      const r = await dialog.showOpenDialog(parent, { title: m.title || '', defaultPath: m.start || undefined, properties: ['openDirectory'] });
      value = r.canceled ? null : (r.filePaths[0] || null);
    }
  } catch (e) { log('picker: ' + e.message); }
  send({ t: 'answer', id: m.id, value });
}

/** Start Hearth again from the start: the `nimbus` program decides the engine again (Settings › Drawing engine). */
function restart() {
  leaving = true;
  try {
    const args = ARGS.filter(a => a !== '--core-stdio');
    spawn(CORE, args, { detached: true, stdio: 'ignore' }).unref();
  } catch (e) { log('restart: ' + e.message); }
  app.exit(0);
}

// ------------------------------------------------------------------ windows
/** The core keeps track of each window's state (shown, focused, its title) for Game Mode. */
function track(label, w) {
  wins[label] = w;
  const report = () => {
    if (w.isDestroyed()) return;
    send({ t: 'win', win: label, exists: true, visible: w.isVisible(), focused: w.isFocused(), title: w.getTitle() });
  };
  for (const e of ['show', 'hide', 'focus', 'blur', 'minimize', 'restore', 'page-title-updated', 'ready-to-show']) w.on(e, () => setImmediate(report));
  w.on('closed', () => { if (wins[label] === w) delete wins[label]; send({ t: 'win', win: label, exists: false, visible: false, focused: false, title: '' }); });
  report();
}

function pageWindow(label, opts, query) {
  const w = new BrowserWindow({
    backgroundColor: '#0c1016', autoHideMenuBar: true, ...opts,
    webPreferences: { preload: path.join(__dirname, 'preload.js'), contextIsolation: true, sandbox: true, nodeIntegration: false, backgroundThrottling: false },
  });
  // A page keeps its title: Game Mode finds Hearth's windows by it.
  w.on('page-title-updated', e => e.preventDefault());
  w.webContents.on('will-navigate', (e, url) => { if (!url.startsWith('nimbus://')) e.preventDefault(); });
  w.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
  w.webContents.on('render-process-gone', (_e, d) => { log(label + ': page gone (' + d.reason + '), reloading'); if (!w.isDestroyed()) w.loadURL('nimbus://app/index.html' + (query || '')).catch(() => {}); });
  track(label, w);
  w.loadURL('nimbus://app/index.html' + (query || '')).catch(e => log(label + ': ' + e.message));
  return w;
}

// Settings › Display › size (the core sends it at start, before the page is there, and on every change).
let zoomPct = 100;
function applyZoom() { if (wins.main && !wins.main.isDestroyed()) wins.main.webContents.setZoomFactor(zoomPct / 100); }

function makeMain() {
  const w = pageWindow('main', { title: MAIN_TITLE, show: false, fullscreen: !WINDOWED, frame: WINDOWED, width: 1280, height: 800 });
  w.once('ready-to-show', () => w.show());
  w.webContents.on('did-finish-load', applyZoom);
  // As the session there's nothing behind Hearth, so Alt+F4 must not close it by accident.
  w.on('close', e => { if (!leaving && IN_SESSION && !WINDOWED) e.preventDefault(); });
  w.on('closed', () => { if (!leaving) { leaving = true; app.exit(0); } });
}

function makeOverlay() {
  if (wins.overlay && !wins.overlay.isDestroyed()) return;
  const b = screen.getPrimaryDisplay().bounds;
  pageWindow('overlay', { title: OVERLAY_TITLE, show: false, frame: false, transparent: true, backgroundColor: '#00000000', hasShadow: false,
    x: b.x, y: b.y, width: b.width, height: b.height, alwaysOnTop: true, skipTaskbar: true, resizable: false, focusable: true }, '?overlay=full');
}

// A store's sign-in page: the core decides from each address whether it's done (winhost.rs signin_outcome).
// A window the core closed itself is marked, so its 'closed' (which can come after the next one has opened) isn't
// taken for you closing the next sign-in.
function closeSignIn() { const w = wins.signin; if (w && !w.isDestroyed()) { w.hearthClosedByCore = true; w.destroy(); } }
function openSignIn(m) {
  closeSignIn();
  const w = new BrowserWindow({
    title: m.title || 'Sign in', backgroundColor: '#0c1016', fullscreen: !!m.fullscreen, width: 900, height: 760, autoHideMenuBar: true,
    // A fresh browser profile each time: nothing from one sign-in is left for the next.
    webPreferences: { partition: 'signin-' + Date.now(), contextIsolation: true, sandbox: true },
  });
  w.on('page-title-updated', e => e.preventDefault());
  const inject = (m.pad ? script('signin-pad.js') : '') + '\n' + (m.script || '');
  const nav = url => send({ t: 'signin.nav', url });
  // hearth-signin://done or cancel: the core's answer, not a page (Chromium would show an error page for it).
  const own = url => String(url).startsWith('hearth-signin:');
  w.webContents.on('will-navigate', (e, url) => { if (own(url)) e.preventDefault(); nav(url); });
  w.webContents.on('will-redirect', (e, url) => { if (own(url)) e.preventDefault(); nav(url); });
  w.webContents.on('did-navigate', (_e, url) => nav(url));
  w.webContents.on('did-navigate-in-page', (_e, url) => nav(url));
  w.webContents.on('dom-ready', () => { if (inject.trim()) w.webContents.executeJavaScript(inject).catch(() => {}); });
  w.webContents.setWindowOpenHandler(({ url }) => { nav(url); if (!own(url)) w.loadURL(url).catch(() => {}); return { action: 'deny' }; });
  w.on('closed', () => { if (!w.hearthClosedByCore) send({ t: 'signin.closed' }); });
  track('signin', w);
  w.loadURL(String(m.url || '')).catch(e => log('sign-in page: ' + e.message));
}

// Xbox Cloud Gaming, full screen, with its own sign-in kept between games.
function openStream(m) {
  const old = wins.stream && !wins.stream.isDestroyed() ? wins.stream : null;
  if (old) { old.loadURL(m.url).catch(e => log('stream: ' + e.message)); old.focus(); return; }
  const w = new BrowserWindow({
    title: m.title || 'Xbox Cloud Gaming', backgroundColor: '#000000', fullscreen: !m.windowed, width: 1280, height: 800, autoHideMenuBar: true,
    webPreferences: { partition: 'persist:xcloud', contextIsolation: true, sandbox: true, backgroundThrottling: false },
  });
  w.webContents.setUserAgent(EDGE_UA);
  w.on('page-title-updated', e => e.preventDefault());
  const inject = script('stream-exit.js') + '\n' + script('stream-signin.js') + (m.windowed ? '' : '\n' + script('signin-pad.js'));
  w.webContents.on('dom-ready', () => w.webContents.executeJavaScript(inject).catch(() => {}));
  w.webContents.on('will-navigate', (e, url) => { if (url.startsWith('hearth-signin://stream-exit')) { e.preventDefault(); send({ t: 'stream.exit' }); } });
  w.webContents.setWindowOpenHandler(({ url }) => { if (url.startsWith('hearth-signin://stream-exit')) send({ t: 'stream.exit' }); return { action: 'deny' }; });
  w.on('closed', () => send({ t: 'stream.closed' }));
  track('stream', w);
  w.loadURL(m.url).catch(e => log('stream: ' + e.message));
}

// ------------------------------------------------------------------ pages, pictures
function serveFiles() {
  // Hearth's page and its files.
  protocol.handle('nimbus', req => {
    const u = new URL(req.url);
    let rel;
    try { rel = decodeURIComponent(u.pathname).replace(/^\/+/, '') || 'index.html'; } catch { return new Response('', { status: 404 }); }
    const file = path.normalize(path.join(UI, rel));
    // Inside the ui folder only (a bare startsWith also let ../ui-something/ through).
    if (!file.startsWith(path.join(UI, path.sep))) return new Response('', { status: 404 });
    return fs.promises.readFile(file).then(b => new Response(b, { headers: { 'content-type': mime(file) } }))
      .catch(() => new Response('', { status: 404 }));
  });
  // Game art: the core says which file a nimbus-art:// address is (pictures only), this app reads it.
  protocol.handle('nimbus-art', req => new Promise(resolve => {
    const id = artId++;
    const timer = setTimeout(() => { artWait.delete(id); resolve(new Response('', { status: 404 })); }, 10000);
    artWait.set(id, r => {
      clearTimeout(timer);
      if (!r.path) return resolve(new Response('', { status: 404 }));
      fs.promises.readFile(r.path).then(b => resolve(new Response(b, { headers: { 'content-type': r.mime || 'image/png', 'cache-control': 'max-age=86400', 'access-control-allow-origin': '*' } })))
        .catch(() => resolve(new Response('', { status: 404 })));
    });
    send({ t: 'art', id, uri: req.url });
  }));
}

function mime(f) {
  const ext = path.extname(f).toLowerCase();
  return ({ '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.png': 'image/png',
    '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.svg': 'image/svg+xml', '.webp': 'image/webp', '.woff2': 'font/woff2', '.ttf': 'font/ttf',
    '.mp3': 'audio/mpeg', '.ogg': 'audio/ogg', '.wav': 'audio/wav' })[ext] || 'application/octet-stream';
}

// ------------------------------------------------------------------ start
ipcMain.on('nimbus:msg', (e, msg) => {
  const label = Object.keys(wins).find(k => wins[k] && !wins[k].isDestroyed() && wins[k].webContents === e.sender) || 'main';
  send({ t: 'page', win: label, msg });
});

app.whenReady().then(() => {
  serveFiles();
  startCore();
  makeMain();
});
app.on('window-all-closed', () => { /* the main window's 'closed' decides */ });
app.on('will-quit', () => { try { globalShortcut.unregisterAll(); } catch { /* fine */ } if (core) try { core.stdin.end(); } catch { /* gone */ } });
