'use strict';
// HEARTH_HHD_OVERLAY: Hearth's Quick Access as Handheld Daemon's overlay app (`nimbus --hhd-overlay`, started by HHD
// through desktop/shell/hearth-overlay, which the HHD editions install as hhd-ui).
//
// HHD starts its overlay app in gamescope and does the hard parts: it finds this window (by its WM_CLASS,
// dev.hhd.hhd-ui), shows it over the game when we say stat:qam and hides it on stat:closed, holds the controllers
// back from the game while we say grab:enable and hands us their presses as action: lines, and turns its QAM button
// into cmd:open_qam / cmd:open_expanded. This app draws Hearth's own Quick Access page (ui/index.html?overlay=full)
// in Chromium, GPU-composited on a see-through window like hhd-ui, which is what makes it smooth; the page's calls
// and events go to the running Hearth app over its overlay socket (link.js), so it can do everything the old
// overlay window could. The protocol itself is in protocol.js.
//
// The page is the one the running Hearth app draws, not this package's copy: Hearth may be newer (or older) than the
// image (an app-path override, an AppImage). When Hearth connects it says where its screens come from (hello):
// served by Hearth itself over the socket (the WebKit app's built-in copy), or a folder (the Electron window app's).
// This package's own copy (UI_DIR) is only for a Hearth too old to say, or a file Hearth can't give.

const { app, BrowserWindow, ipcMain, protocol, net, screen, Menu } = require('electron');
const path = require('node:path');
const fs = require('node:fs');
const readline = require('node:readline');
const { pathToFileURL } = require('node:url');
const { OverlayProtocol } = require('./protocol');
const { HearthLink, socketPath } = require('./link');
const { artFile } = require('./art');

// stdout is HHD's (stat:/grab: lines): everything else goes to stderr, which HHD writes to its log, and to
// $XDG_RUNTIME_DIR/hearth-overlay.log (hearth-overlay starts it), which Send logs includes.
const LOG_FILE = process.env.XDG_RUNTIME_DIR ? path.join(process.env.XDG_RUNTIME_DIR, 'hearth-overlay.log') : null;
const toStderr = console.error.bind(console);
console.error = (...a) => {
  toStderr(...a);
  if (LOG_FILE) { try { fs.appendFileSync(LOG_FILE, a.map(x => typeof x === 'string' ? x : String(x && x.stack || x)).join(' ') + '\n'); } catch {} }
};
console.log = (...a) => console.error(...a);
console.info = console.log;
process.stdout.on('error', () => {});
// Nothing in here may end the app: HHD only starts it again later (or on the next QAM press), and until then Quick
// Access has nowhere to open. A slip is logged (HHD's log) and the app carries on.
process.on('uncaughtException', e => console.error(`overlay: ${e && e.stack || e}`));
process.on('unhandledRejection', e => console.error(`overlay: ${e && e.stack || e}`));

// HHD finds its overlay window by WM_CLASS; Electron names it after the app.
app.setName('dev.hhd.hhd-ui');
app.setPath('userData', path.join(app.getPath('appData'), 'hearth-overlay'));
app.commandLine.appendSwitch('gtk-version', '3');   // as hhd-ui
app.commandLine.appendSwitch('enable-gpu-rasterization');

protocol.registerSchemesAsPrivileged([
  { scheme: 'nimbus', privileges: { standard: true, secure: true, supportFetchAPI: true } },
  { scheme: 'nimbus-art', privileges: { standard: true, secure: true, supportFetchAPI: true } },
]);

const UI_DIR = app.isPackaged ? path.join(process.resourcesPath, 'ui') : path.join(__dirname, '..', '..', '..', '..', 'ui');
const PAGE = 'nimbus://app/index.html?overlay=full';
const BLANK = 'data:text/html,<html style="background:transparent"></html>';
const TITLE = 'Hearth Overlay';   // Hearth leaves its own windows out of Quick Access's open apps by this name
const HELLO_WAIT = 1500;          // how long a connecting Hearth gets to say where its screens are
// HEARTH_OVERLAY_SELFTEST=1 (CI, base-*.yml): draw this app's own Quick Access page, say whether its text shows
// (`overlay-selftest: {...}` on stderr) and end: 0 text shows, 3 no text (no font), 4 the page didn't load.
const SELFTEST = process.env.HEARTH_OVERLAY_SELFTEST === '1';
// Run in the page after it loads: does text draw? Chromium with no font it can use draws nothing at all (Quick Access
// came up without any text on the Nvidia PC, 0.6.5): a canvas with text in the page's sans-serif and a span in the
// page's own font both come out empty then.
const FONT_PROBE = `(() => {
  const r = { ink: 0, width: 0, spanWidth: 0 };
  try {
    const c = document.createElement('canvas'); c.width = 260; c.height = 60;
    const x = c.getContext('2d'); x.font = '28px sans-serif'; x.fillStyle = '#fff'; x.fillText('Hearth Aa 09', 4, 40);
    const d = x.getImageData(0, 0, c.width, c.height).data;
    for (let i = 3; i < d.length; i += 4) if (d[i] > 0) r.ink++;
    r.width = Math.round(x.measureText('Hearth Aa 09').width);
  } catch (e) { r.error = String(e); }
  try {
    const s = document.createElement('span'); s.textContent = 'Quick Access';
    s.style.cssText = 'position:absolute;left:-9999px;top:0;white-space:nowrap;visibility:visible';
    (document.body || document.documentElement).appendChild(s);
    r.spanWidth = Math.round(s.getBoundingClientRect().width); s.remove();
  } catch (e) { r.error = String(e); }
  r.bodyFont = document.body ? getComputedStyle(document.body).fontFamily : '';
  r.quickAccess = !!document.getElementById('qam');
  r.overlayMode = !!(document.body && document.body.classList.contains('overlay'));
  return r;
})()`;
const fontsOk = r => !!r && r.ink > 0 && r.width > 0 && r.spanWidth > 0;
const FILE_WAIT = 5000;           // one file from Hearth

/** Where the page's files come from: {mode:'hearth'} (asked of Hearth), {mode:'dir', dir} or {mode:'own'} (UI_DIR). */
let uiSrc = { mode: 'own' };
let helloTimer = null;
const files = new Map();   // id → {done(reply|null)}: files asked of Hearth
let fileId = 0;

let win = null;
/** Loads a page in the window (a newer load cancelling this one isn't an error). */
const load = url => (win && !win.isDestroyed() ? win.loadURL(url).catch(() => {}) : Promise.resolve());
const write = line => { try { process.stdout.write(line + '\n'); } catch {} };
const toPage = msg => { if (win && !win.isDestroyed()) win.webContents.send('nimbus:msg', msg); };
const link = new HearthLink(socketPath());

const proto = new OverlayProtocol({
  out: write,
  toHearth: msg => link.send(msg),
  toPage,
  connected: () => link.connected,
  onHost: msg => {
    if (msg.what === 'hello') return hello(msg);
    if (msg.what === 'opacity') proto.peek(!msg.value);   // Take a screenshot: a picture of just the game
    if (!win || win.isDestroyed()) return;
    if (msg.what === 'reload') load(PAGE);
  },
});

/** Hearth said where its screens come from: the page is loaded from there. */
function hello(msg) {
  if (helloTimer) { clearTimeout(helloTimer); helloTimer = null; }
  const dir = typeof msg.uiDir === 'string' && path.isAbsolute(msg.uiDir) ? path.normalize(msg.uiDir) : null;
  uiSrc = msg.ui === 'hearth' ? { mode: 'hearth' } : dir ? { mode: 'dir', dir } : { mode: 'own' };
  hearthVersion = String(msg.version || '');
  console.error(`overlay: Hearth ${msg.version || ''} connected; its screens come from ${uiSrc.mode === 'hearth' ? 'Hearth itself' : uiSrc.mode === 'dir' ? uiSrc.dir : 'this app'}`);
  load(PAGE);
}
function fileFrom(dir, rel) {
  const file = path.normalize(path.join(dir, rel));
  if (!file.startsWith(dir + path.sep)) return null;
  return file;
}
function ownFile(rel) {
  const file = fileFrom(UI_DIR, rel);
  return file ? net.fetch(pathToFileURL(file).toString()) : new Response('Not found', { status: 404 });
}
/** One file of Hearth's screens, asked of Hearth over the socket (null: it couldn't give it). */
function askHearth(rel) {
  return new Promise(done => {
    const id = ++fileId;
    const t = setTimeout(() => { files.delete(id); done(null); }, FILE_WAIT);
    files.set(id, { done: r => { clearTimeout(t); files.delete(id); done(r); } });
    if (!link.send({ type: 'ui-file', id, path: rel.replace(/^\/+/, '') })) files.get(id).done(null);
  });
}
function dropFiles() { for (const f of [...files.values()]) f.done(null); }
async function serveUi(req) {
  const rel = decodeURIComponent(new URL(req.url).pathname);
  if (uiSrc.mode === 'dir') {
    const file = fileFrom(uiSrc.dir, rel);
    if (file && fs.existsSync(file)) return net.fetch(pathToFileURL(file).toString());
  } else if (uiSrc.mode === 'hearth') {
    const r = await askHearth(rel);
    if (r && r.ok && typeof r.data === 'string') {
      return new Response(Buffer.from(r.data, 'base64'), { status: 200, headers: { 'content-type': r.mime || 'application/octet-stream', 'cache-control': 'no-store' } });
    }
    console.error(`overlay: Hearth didn't give ${rel}${r && r.error ? ` (${r.error})` : ''}; using this app's copy`);
  }
  return ownFile(rel);
}
function serveArt(req) {
  const file = artFile(req.url);
  return file ? net.fetch(pathToFileURL(file).toString()) : new Response('Not found', { status: 404 });
}

/** Where the page came from, for the log. */
function srcName() { return uiSrc.mode === 'hearth' ? 'Hearth itself' : uiSrc.mode === 'dir' ? uiSrc.dir : `this app (${UI_DIR})`; }
let hearthVersion = '';
/** After Quick Access loads: which page it is and whether its text shows (a line in HHD's log and Send logs). */
async function checkPage() {
  if (!win || win.isDestroyed()) return null;
  const url = win.webContents.getURL();
  if (!url.startsWith('nimbus://')) return null;
  let r = null;
  try { r = await win.webContents.executeJavaScript(FONT_PROBE, true); } catch (e) { r = { error: String(e && e.message || e) }; }
  const what = `Quick Access page from ${srcName()}${hearthVersion ? ` (Hearth ${hearthVersion})` : ''}, overlay app ${app.getVersion()}, Electron ${process.versions.electron}`;
  if (fontsOk(r)) console.error(`overlay: ${what}; text shows (fonts found: sans-serif ${r.width}px wide, page font ${r.bodyFont})`);
  else console.error(`overlay: ${what}; NO TEXT: Chromium found no font it can use (${JSON.stringify(r)}); FONTCONFIG_FILE=${process.env.FONTCONFIG_FILE || 'the system\'s'}`);
  return r;
}

/** CI: this app's own page, its text checked, then the end. */
function selftest() {
  Menu.setApplicationMenu(null);
  protocol.handle('nimbus', serveUi);
  protocol.handle('nimbus-art', serveArt);
  win = new BrowserWindow({ x: 0, y: 0, width: 1280, height: 800, frame: false, transparent: true, backgroundColor: '#00000000', show: true, title: TITLE,
    webPreferences: { preload: path.join(__dirname, '..', 'preload.js'), contextIsolation: true, nodeIntegration: false, sandbox: true, spellcheck: false } });
  const done = (code, r) => { process.stderr.write(`overlay-selftest: ${JSON.stringify({ ok: code === 0, code, fontconfig: process.env.FONTCONFIG_FILE || null, lang: process.env.LANG || null, electron: process.versions.electron, ...(r || {}) })}\n`); app.exit(code); };
  const timer = setTimeout(() => done(4, { error: 'the page did not load in 40 s' }), 40000);
  win.webContents.once('did-finish-load', () => setTimeout(async () => {
    clearTimeout(timer);
    const r = await checkPage();
    done(fontsOk(r) ? 0 : 3, r);
  }, 1500));
  win.webContents.once('did-fail-load', (_e, code, desc) => { clearTimeout(timer); done(4, { error: `${code} ${desc}` }); });
  load(PAGE);
}

function start() {
  if (SELFTEST) return selftest();
  Menu.setApplicationMenu(null);
  protocol.handle('nimbus', serveUi);
  protocol.handle('nimbus-art', serveArt);
  const { width, height } = screen.getPrimaryDisplay().size;   // gamescope's screen
  win = new BrowserWindow({
    x: 0, y: 0, width, height, fullscreen: true, frame: false, resizable: false, skipTaskbar: true,
    transparent: true, backgroundColor: '#00000000', show: false, title: TITLE,
    webPreferences: { preload: path.join(__dirname, '..', 'preload.js'), contextIsolation: true, nodeIntegration: false,
                      sandbox: true, backgroundThrottling: false, spellcheck: false },
  });
  win.setMenu(null);
  win.on('page-title-updated', e => e.preventDefault());
  win.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
  win.webContents.on('will-navigate', (e, url) => { if (!url.startsWith('nimbus://')) e.preventDefault(); });
  win.webContents.on('render-process-gone', (_e, d) => { console.error(`overlay: page ended (${d.reason}), reloading`); proto.disconnected(); setTimeout(() => load(link.connected ? PAGE : BLANK), 500); });
  // Each time Quick Access (not the blank page) loads: which page, and whether its text shows.
  win.webContents.on('did-finish-load', () => { setTimeout(() => { checkPage().catch(() => {}); }, 1000); });
  // On screen (see-through) right away: HHD looks for the window within 10 seconds of starting us.
  load(BLANK).then(() => { if (win && !win.isDestroyed()) win.show(); });

  // The page: its messages go to Hearth; a call made while Hearth isn't there fails at once instead of waiting.
  ipcMain.on('nimbus:msg', (e, msg) => {
    if (!win || e.sender !== win.webContents || !msg || typeof msg !== 'object') return;
    if (msg.type === 'ready') toPage({ type: 'event', name: 'padHost', data: { connected: 1 } });
    if (msg.type === 'overlay' && msg.action === 'close' && !link.connected) { proto.disconnected(); return; }
    if (!link.send(msg) && msg.type === 'call') toPage({ type: 'result', id: msg.id, ok: false, error: "Hearth isn't running" });
  });

  // Hearth: the page is (re)loaded each time it connects, so it starts from Hearth's current state (and Hearth's
  // version of it: once Hearth has said where its screens are, or after HELLO_WAIT for a Hearth that doesn't say).
  // Hearth going away (it restarted) isn't the end: the link keeps trying and the page comes back with it.
  link.on('connect', () => {
    console.error('overlay: connected to Hearth');
    proto.disconnected();
    uiSrc = { mode: 'own' };
    if (helloTimer) clearTimeout(helloTimer);
    helloTimer = setTimeout(() => { helloTimer = null; console.error("overlay: Hearth didn't say where its screens are; using this app's copy"); load(PAGE); }, HELLO_WAIT);
    proto.hearthConnected();
  });
  link.on('disconnect', () => {
    console.error('overlay: Hearth went away; waiting for it to come back');
    if (helloTimer) { clearTimeout(helloTimer); helloTimer = null; }
    dropFiles();
    proto.disconnected();
    load(BLANK);
  });
  link.on('message', msg => {
    if (msg && msg.type === 'ui-file') { const f = files.get(msg.id); if (f) f.done(msg); return; }
    proto.fromHearth(msg);
  });
  link.start();

  // HHD.
  const rl = readline.createInterface({ input: process.stdin });
  rl.on('line', l => proto.line(l));
  rl.on('close', () => { if (!process.stdin.isTTY) app.quit(); });   // HHD went away
}

app.whenReady().then(start);
app.on('window-all-closed', () => app.quit());
