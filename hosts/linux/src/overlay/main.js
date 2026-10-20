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

const { app, BrowserWindow, ipcMain, protocol, net, screen, Menu } = require('electron');
const path = require('node:path');
const readline = require('node:readline');
const { pathToFileURL } = require('node:url');
const { OverlayProtocol } = require('./protocol');
const { HearthLink, socketPath } = require('./link');
const { artFile } = require('./art');

// stdout is HHD's (stat:/grab: lines): everything else goes to stderr, which HHD writes to its log.
console.log = (...a) => console.error(...a);
console.info = console.log;
process.stdout.on('error', () => {});

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
    if (msg.what === 'opacity') proto.peek(!msg.value);   // Take a screenshot: a picture of just the game
    if (!win || win.isDestroyed()) return;
    if (msg.what === 'reload') load(PAGE);
  },
});

function serveUi(req) {
  const rel = decodeURIComponent(new URL(req.url).pathname);
  const file = path.normalize(path.join(UI_DIR, rel));
  if (!file.startsWith(UI_DIR + path.sep)) return new Response('Not found', { status: 404 });
  return net.fetch(pathToFileURL(file).toString());
}
function serveArt(req) {
  const file = artFile(req.url);
  return file ? net.fetch(pathToFileURL(file).toString()) : new Response('Not found', { status: 404 });
}

function start() {
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
  // On screen (see-through) right away: HHD looks for the window within 10 seconds of starting us.
  load(BLANK).then(() => { if (win && !win.isDestroyed()) win.show(); });

  // The page: its messages go to Hearth; a call made while Hearth isn't there fails at once instead of waiting.
  ipcMain.on('nimbus:msg', (e, msg) => {
    if (!win || e.sender !== win.webContents || !msg || typeof msg !== 'object') return;
    if (msg.type === 'ready') toPage({ type: 'event', name: 'padHost', data: { connected: 1 } });
    if (msg.type === 'overlay' && msg.action === 'close' && !link.connected) { proto.disconnected(); return; }
    if (!link.send(msg) && msg.type === 'call') toPage({ type: 'result', id: msg.id, ok: false, error: "Hearth isn't running" });
  });

  // Hearth: the page is (re)loaded each time it connects, so it starts from Hearth's current state.
  link.on('connect', () => { console.error('overlay: connected to Hearth'); proto.disconnected(); load(PAGE); });
  link.on('disconnect', () => { console.error('overlay: Hearth went away'); proto.disconnected(); load(BLANK); });
  link.on('message', msg => proto.fromHearth(msg));
  link.start();

  // HHD.
  const rl = readline.createInterface({ input: process.stdin });
  rl.on('line', l => proto.line(l));
  rl.on('close', () => { if (!process.stdin.isTTY) app.quit(); });   // HHD went away
}

app.whenReady().then(start);
app.on('window-all-closed', () => app.quit());
