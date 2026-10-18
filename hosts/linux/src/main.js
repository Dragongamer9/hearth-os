'use strict';
// Nimbus for Linux: hosts the shared web UI (../../ui) in a fullscreen Electron window.
// Inside the "Nimbus" login session it runs in gamescope (see session/); started from a desktop
// with --windowed it runs in a normal window for development.

const { app, BrowserWindow, ipcMain, protocol, net, dialog, screen, session, Menu, safeStorage, globalShortcut } = require('electron');
const path = require('node:path');
const fs = require('node:fs');
const os = require('node:os');
const { pathToFileURL } = require('node:url');
const { Library } = require('./library');
const { Bridge } = require('./bridge');
const { Updates } = require('./updates');
const { StreamWindow } = require('./stream');
const { GuideButton } = require('./guide');
const { ButtonPrefs, Buttons, Controller } = require('./buttons');
const { Browser } = require('./browser');
const { DisplayModes } = require('./display');
const steam = require('./library/steam');
const { secure } = require('./stores/common');
const sessionSetup = require('./session');
const { run, readText, log } = require('./util');

const args = process.argv.slice(1);
const WINDOWED = args.includes('--windowed');
// --settings: open on Settings ("Hearth Settings" in Desktop Mode); with --windowed, a normal window.
const START_PAGE = args.includes('--settings') ? 'settings' : null;
const IN_SESSION = process.env.NIMBUS_SESSION === '1' || args.includes('--session');
const UPDATE_EXIT_CODE = 75; // nimbus-watchdog starts the new version right away

const UI_DIR = app.isPackaged ? path.join(process.resourcesPath, 'ui') : path.join(__dirname, '..', '..', '..', 'ui');
const SESSION_DIR = app.isPackaged ? path.join(process.resourcesPath, 'session') : path.join(__dirname, '..', 'session');
const DESKTOP_DIR = app.isPackaged ? path.join(process.resourcesPath, 'desktop') : path.join(__dirname, '..', 'desktop');
const APP_ROOT = path.join(__dirname, '..');

if (!app.requestSingleInstanceLock()) { app.exit(0); }

protocol.registerSchemesAsPrivileged([
  { scheme: 'nimbus', privileges: { standard: true, secure: true, supportFetchAPI: true } },
  { scheme: 'nimbus-art', privileges: { standard: true, secure: true, supportFetchAPI: true } },
]);

let win = null;
let leaving = false;
let library, bridge, updates, stream, guide, buttons;
const buttonPrefs = new ButtonPrefs();
const controller = new Controller(path.join(DESKTOP_DIR, 'shell', 'hearth-controller'));

/** Restart Nimbus. As an AppImage, start the AppImage file again (it may have just been replaced by an update). */
function relaunch() {
  if (process.env.APPIMAGE) app.relaunch({ execPath: process.env.APPIMAGE, args: process.argv.slice(1) });
  else app.relaunch();
  app.exit(0);
}

/** Settings › System › Save logs: Hearth's logs in one file in your home folder, for bug reports. */
async function exportLogs() {
  const stamp = new Date().toISOString().slice(0, 16).replace(/[:T]/g, '-');
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'hearth-logs-'));
  const data = path.join(process.env.XDG_DATA_HOME || path.join(os.homedir(), '.local', 'share'), 'nimbus');
  const runDir = process.env.XDG_RUNTIME_DIR || '/tmp';
  const copy = (from, name) => { try { fs.copyFileSync(from, path.join(dir, name)); } catch {} };
  for (const f of ['bridge.log', 'session.log']) copy(path.join(data, f), f);
  copy(path.join(runDir, 'nimbus-shell.log'), 'desktop-shell.log');
  copy('/etc/os-release', 'os-release.txt');
  const info = { version: app.getVersion(), electron: process.versions.electron, ...require('./sysinfo').info() };
  fs.writeFileSync(path.join(dir, 'about.json'), JSON.stringify(info, null, 2));
  await run('sh', ['-c', 'bootc status 2>/dev/null > "$0/bootc-status.txt"; journalctl --user -b -n 3000 --no-pager > "$0/journal-user.txt" 2>/dev/null; true', dir]).catch(() => {});
  const out = path.join(os.homedir(), `Hearth-logs-${stamp}.tar.gz`);
  await run('tar', ['-czf', out, '-C', dir, '.']);
  fs.rmSync(dir, { recursive: true, force: true });
  return { path: out };
}

function post(msg) { if (win && !win.isDestroyed()) win.webContents.send('nimbus:msg', msg); }

function osName() {
  const m = /^PRETTY_NAME="?([^"\n]+)"?/m.exec(readText('/etc/os-release') || '');
  return m ? m[1] : 'Linux';
}

// ------------------------------------------------------------------ files for the UI

function serveFile(root, rel) {
  const file = path.normalize(path.join(root, rel));
  if (!file.startsWith(root + path.sep)) return new Response('Not found', { status: 404 }); // no ../ escapes
  return net.fetch(pathToFileURL(file).toString());
}

function registerProtocols() {
  // nimbus://app/index.html → the shared UI folder
  protocol.handle('nimbus', req => serveFile(UI_DIR, decodeURIComponent(new URL(req.url).pathname)));
  // nimbus-art://art/{id}/{kind} → a game's art in its store's own folders
  // nimbus-art://profile/{avatar|background|steam} → profile pictures
  protocol.handle('nimbus-art', req => {
    const u = new URL(req.url);
    const parts = u.pathname.split('/').filter(Boolean).map(decodeURIComponent);
    const file = u.hostname === 'profile' ? bridge && bridge.profile.file(parts[0])
      : u.hostname === 'cache' ? require('./art-cache').artCache().file(parts[0])   // downloaded web art
      : library && library.artFile(parts[0], parts[1]);
    return file ? net.fetch(pathToFileURL(file).toString()) : new Response('Not found', { status: 404 });
  });
}

// ------------------------------------------------------------------ showing and hiding Nimbus around games

/** A game is in front: get out of its way (gamescope shows whichever window is on top). */
function stepAside() { if (win && !WINDOWED) win.hide(); }

const gamescope = require('./gamescope');
const inGamescope = () => IN_SESSION && !WINDOWED && gamescope.available();
/** Puts the running game in front (gamescope): its window, or its Steam app id. */
function focusRunningGame() {
  if (!inGamescope() || !library.running) return;
  const id = String(library.running.id || '');
  const steamAppId = id.startsWith('steam:') ? parseInt(id.slice(6), 10) || 0 : 0;
  gamescope.focusGame({ pids: () => library.gamePids(), steamAppId, exclude: [process.pid] }).catch(() => {});
}

/** Puts another Hearth window (a sign-in page, a file picker) in front in gamescope. */
function toFront(w) {
  if (!w || w.isDestroyed()) return;
  if (!w.isVisible()) w.show();
  w.focus();
  if (inGamescope()) gamescope.focusWindow(xid(w)).catch(() => {});
}
/** File pickers are separate windows gamescope wouldn't show while Hearth's is pinned in front. */
async function withDialog(fn) {
  if (inGamescope()) await gamescope.clearFocus().catch(() => {});
  try { return await fn(); } finally { comeBack(); }
}

/** When something was last deliberately put in front (Hearth, an app from the overlay, the game). Closing
 *  the overlay gives the screen back to what was there before only if nothing was chosen since: otherwise
 *  that hand-back fired a moment after "Back to Hearth" or "Switch" and pulled the old app back in front. */
let focusChosenAt = 0;
const chooseFocus = () => { focusChosenAt = Date.now(); };

/** Back in front, e.g. when a game ends or the Guide button is pressed. */
function comeBack() {
  if (!win || win.isDestroyed()) return;
  chooseFocus();
  // Belt and braces: the overlay never stays up over Hearth.
  if (overlay && !overlay.isDestroyed() && !overlayOn && overlay.isVisible()) parkOverlay();
  if (win.isMinimized()) win.restore();
  win.show();
  win.focus();
  // gamescope shows one app at a time: tell it Hearth is the one now.
  if (inGamescope()) gamescope.focusWindow(xid(win)).catch(() => {});
}

/**
 * Hearth stays in front with its "starting" screen while the game (or its launcher) loads in the background,
 * and puts it in front once its window is really up, instead of showing a black screen or a half-drawn window
 * meanwhile. A Steam window that may need you (a cloud-save question) is shown right away. "Show it now" on
 * the starting screen (resumeGame) skips the wait.
 */
let launchWait = null;
async function showGameWhenReady(id) {
  const wait = launchWait = { id, cancelled: false };
  const sid = String(id || '');
  const steamAppId = sid.startsWith('steam:') ? parseInt(sid.slice(6), 10) || 0 : 0;
  log(`launch: waiting for ${id} to open its window`);
  // Windows games through Proton can take minutes the first time (umu sets up Proton and the game's prefix):
  // keep waiting, and after a while let the starting screen say so and offer "Show it now".
  const slow = setTimeout(() => { if (launchWait === wait) post({ type: 'event', name: 'game.slow', data: { id, firstRun: !steamAppId } }); }, 40000);
  const r = await gamescope.waitForWindow({ pids: () => library.gamePids(), steamAppId, exclude: [process.pid], timeoutMs: 10 * 60000,
    cancelled: () => wait.cancelled || library.runningId !== id }).catch(() => null);
  clearTimeout(slow);
  if (wait.cancelled || library.runningId !== id) return;
  launchWait = null;
  if (r && r.win) {
    log(`launch: ${id} is ready (window ${r.win}), bringing it in front`);
    chooseFocus(); stepAside();
    await gamescope.focusWindow(r.win).catch(() => {});
    post({ type: 'event', name: 'game.ready', data: { id } });
  } else if (r && r.dialog) {
    log(`launch: Steam is asking something ("${r.title}"), showing it`);
    chooseFocus(); stepAside();
    await gamescope.focusWindow(r.dialog).catch(() => {});
    post({ type: 'event', name: 'game.ready', data: { id } });
    focusRunningGame();   // and the game once it opens
  } else {
    log(`launch: ${id} didn't open a window in 10 minutes; showing whatever it has`);
    chooseFocus(); stepAside(); focusRunningGame();
    post({ type: 'event', name: 'game.ready', data: { id } });
  }
}

function resumeGame() {
  if (!library.runningId) return;
  if (launchWait) launchWait.cancelled = true;
  chooseFocus();
  if (stream.isOpen) { stepAside(); stream.focus(); if (inGamescope()) gamescope.focusWindow(xid(stream.win || win)).catch(() => {}); return; }
  stepAside();
  focusRunningGame();
}

// ------------------------------------------------------------------ messages from the UI

function onMessage(e, m) {
  if (!m || typeof m !== 'object') return;
  if (overlay && e && e.sender === overlay.webContents) {
    if (m.type === 'call') overlayBridge.handle(m);
    else if (m.type === 'overlay' && m.action === 'close') hideOverlay();
    else if (m.type === 'overlay' && m.action === 'alive') overlayAlive = Date.now();
    // "Back to Hearth" and "Back to game" in the overlay (these were ignored, so the overlay stayed open)
    else if (m.type === 'overlay' && m.action === 'home') { chooseFocus(); hideOverlay(); comeBack(); }
    else if (m.type === 'resume') { chooseFocus(); hideOverlay(); resumeGame(); }
    else if (m.type === 'log') log('overlay: ' + m.text);
    else if (m.type === 'power') { hideOverlay(); power(m.action); }
    return;
  }
  switch (m.type) {
    case 'ready': post({ type: 'event', name: 'padHost', data: { connected: 0 } }); break; // the UI reads pads itself
    case 'call': bridge.handle(m); break;
    case 'log': log('ui: ' + m.text); break;
    case 'power': power(m.action); break;
    case 'resume': resumeGame(); break; // Quick Access closed while a game is running
    case 'setting': break;
  }
}

/**
 * Ends Game Mode and tells the session what comes next ('desktop', 'game', or null to sign out).
 * Steam is closed first, while Game Mode's display still exists: left running, it kept gamescope from
 * closing (the switch froze on a black screen) and was stuck on a display that no longer existed. Steam gets
 * a few seconds to close by itself; the session cleans up anything still running after that.
 */
let leavingGameMode = false;
async function leaveGameMode(next) {
  if (leavingGameMode) return;
  leavingGameMode = true; leaving = true;
  const file = path.join(process.env.XDG_RUNTIME_DIR || '/tmp', 'nimbus-next');
  try { if (next) fs.writeFileSync(file, next + '\n'); else fs.rmSync(file, { force: true }); } catch (e) { log('mode switch: ' + e.message); }
  log(`leaving Game Mode (next: ${next || 'sign out'})`);
  const steamUp = () => run('pgrep', ['-u', String(process.getuid()), '-x', 'steam'], { timeout: 3000 }).then(() => true, () => false);
  try {
    if (await steamUp()) {
      log('closing Steam before switching');
      run('steam', ['-shutdown'], { timeout: 5000 }).catch(() => {});
      for (let t = Date.now(); Date.now() - t < 7000 && await steamUp();) await new Promise(r => setTimeout(r, 300));
    }
  } catch {}
  app.exit(0);
}

function power(action) {
  const sys = verb => run('systemctl', [verb]).catch(e => post({ type: 'toast', text: e.message }));
  switch (action) {
    case 'sleep': sys('suspend'); break;
    case 'restart': sys('reboot'); break;
    case 'shutdown': sys('poweroff'); break;
    case 'signout':
    case 'desktop':
      // In the Nimbus session, Nimbus asks the session for the next mode and quits: "desktop" switches to
      // Desktop Mode, sign out (no next mode) ends the session. Outside it, just get out of the way.
      if (IN_SESSION) leaveGameMode(action === 'desktop' ? 'desktop' : null);
      else if (action === 'signout' && process.env.XDG_SESSION_ID) run('loginctl', ['terminate-session', process.env.XDG_SESSION_ID]).catch(() => {});
      else win.minimize();
      break;
  }
}

// ------------------------------------------------------------------ store sign-in

/**
 * A full-screen window showing a store's own sign-in page. You type your password into the store's page;
 * Nimbus only watches for the page the store sends you to afterwards, which carries a one-time code.
 * onNavigate(url): a code, '' (matched but no code: cancelled), or null (keep going).
 * onLoaded(url, readText, navigate): async, for stores that put the code in the page itself (Epic).
 * Esc or B on a controller cancels. Resolves with the code, or null.
 */
function signInWindow({ title, url, onNavigate, onLoaded }) {
  return new Promise(resolve => {
    let done = false;
    const w = new BrowserWindow({
      title, backgroundColor: '#0c1016', fullscreen: !WINDOWED, width: 1100, height: 820, autoHideMenuBar: true,
      // A fresh browser profile each time: nothing from one sign-in is left for the next.
      webPreferences: { partition: 'signin-' + Date.now(), contextIsolation: true, sandbox: true,
        preload: path.join(__dirname, 'signin-preload.js') },
    });
    const finish = v => { if (done) return; done = true; resolve(v || null); if (!w.isDestroyed()) w.close(); };
    const check = (e, u) => {
      if (!onNavigate) return;
      const r = onNavigate(u);
      if (r === null || r === undefined) return;
      if (e) e.preventDefault();
      finish(r);
    };
    const wc = w.webContents;
    const wcId = wc.id;
    signInContents.add(wcId);
    w.on('closed', () => signInContents.delete(wcId));
    wc.on('will-redirect', (e, u) => check(e, u));
    wc.on('will-navigate', (e, u) => check(e, u));
    wc.on('did-navigate', (_e, u) => check(null, u));
    wc.on('did-finish-load', async () => {
      if (!onLoaded || done) return;
      try {
        const r = await onLoaded(wc.getURL(), () => wc.executeJavaScript('document.body ? document.body.innerText : ""'), u => w.loadURL(u));
        if (r) finish(r);
      } catch (e) { log('sign-in: ' + e.message); }
    });
    wc.setWindowOpenHandler(({ url: u }) => { w.loadURL(u); return { action: 'deny' }; });
    wc.on('before-input-event', (_e, i) => { if (i.type === 'keyDown' && i.key === 'Escape') finish(null); });
    w.on('closed', () => { finish(null); comeBack(); });
    // gamescope keeps showing whichever window Hearth last put in front (Hearth's own): put this one there,
    // or the sign-in page opens out of sight and looks like it never loads.
    w.once('ready-to-show', () => toFront(w));
    w.on('show', () => toFront(w));
    setTimeout(() => toFront(w), 800);
    w.loadURL(url);
  });
}
// Controller input from a sign-in window's preload (pointer, keyboard), turned into real input events so every
// site accepts it. Only sign-in windows may ask for this.
const signInContents = new Set();
const signInSender = e => (signInContents.has(e.sender.id) ? e.sender : null);
ipcMain.on('signin:cancel', e => { const w = BrowserWindow.fromWebContents(e.sender); if (w && signInSender(e)) w.close(); });
ipcMain.on('signin:click', (e, p) => {
  const wc = signInSender(e); if (!wc || !p) return;
  const x = Math.round(+p.x), y = Math.round(+p.y);
  wc.sendInputEvent({ type: 'mouseMove', x, y });
  wc.sendInputEvent({ type: 'mouseDown', x, y, button: 'left', clickCount: 1 });
  wc.sendInputEvent({ type: 'mouseUp', x, y, button: 'left', clickCount: 1 });
});
ipcMain.on('signin:wheel', (e, p) => {
  const wc = signInSender(e); if (!wc || !p) return;
  wc.sendInputEvent({ type: 'mouseWheel', x: Math.round(+p.x), y: Math.round(+p.y), deltaX: 0, deltaY: Math.round(+p.dy), canScroll: true });
});
ipcMain.on('signin:type', (e, text) => {
  const wc = signInSender(e); if (!wc || typeof text !== 'string') return;
  for (const ch of text.slice(0, 64)) wc.sendInputEvent({ type: 'char', keyCode: ch });
});
ipcMain.on('signin:key', (e, key) => {
  const wc = signInSender(e); if (!wc || !['Backspace', 'Tab', 'Return'].includes(key)) return;
  wc.sendInputEvent({ type: 'keyDown', keyCode: key });
  if (key === 'Return') wc.sendInputEvent({ type: 'char', keyCode: '\r' });
  wc.sendInputEvent({ type: 'keyUp', keyCode: key });
});

// ------------------------------------------------------------------ in-game overlay (gamescope)
// In Game Mode, Quick Access opens on top of the running game instead of switching back to Hearth: a
// transparent window marked as gamescope's overlay (STEAM_OVERLAY), which gamescope draws over the game,
// and gives input to while it's open (STEAM_INPUT_FOCUS). It loads the same page with ?overlay=full.
// Elsewhere (a desktop, or without xprop) the Guide button brings Hearth itself back, as before.
let overlay = null, overlayBridge = null, overlayOn = false;

function xid(w) { const h = w.getNativeWindowHandle(); return h.length >= 8 ? Number(h.readBigUInt64LE(0)) : h.readUInt32LE(0); }
function xprop(args) { return run('xprop', args, { timeout: 4000 }); }
const setWinAtom = (w, name, v) => xprop(['-id', String(xid(w)), '-f', name, '32c', '-set', name, String(v)]);
const setRootAtom = (name, v) => xprop(['-root', '-f', name, '32c', '-set', name, String(v)]);

function overlayAvailable() {
  return IN_SESSION && !WINDOWED && require('./util').has('xprop') && !!process.env.DISPLAY;
}

function createOverlay(platform) {
  if (!overlayAvailable()) return;
  const b = screen.getPrimaryDisplay().bounds;
  overlay = new BrowserWindow({
    show: false, frame: false, transparent: true, backgroundColor: '#00000000', hasShadow: false,
    x: b.x, y: b.y, width: b.width, height: b.height, alwaysOnTop: true, skipTaskbar: true, resizable: false,
    // Never paused while hidden: Chromium thought a window gamescope draws on top was still hidden and
    // stopped its timers and drawing, so the overlay froze after a few uses.
    webPreferences: { preload: path.join(__dirname, 'preload.js'), contextIsolation: true, sandbox: true, nodeIntegration: false, backgroundThrottling: false },
  });
  // A frozen or crashed page: start it again (and get it out of the way).
  overlay.webContents.on('render-process-gone', (_e, d) => { log('overlay: page gone (' + d.reason + '), reloading'); hideOverlay(); overlay.loadURL('nimbus://app/index.html?overlay=full'); });
  overlay.webContents.on('unresponsive', () => { log('overlay: page unresponsive, reloading'); hideOverlay(); overlay.webContents.forcefullyCrashRenderer(); });
  overlay.webContents.on('will-navigate', (e, url) => { if (!url.startsWith('nimbus://')) e.preventDefault(); });
  overlay.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
  overlay.on('close', e => { if (!leaving) { e.preventDefault(); hideOverlay(); } });
  overlay.on('closed', () => { overlay = null; });
  overlayBridge = new Bridge({ post: m => { if (overlay && !overlay.isDestroyed()) overlay.webContents.send('nimbus:msg', m); }, library, updates, platform });
  setWinAtom(overlay, 'STEAM_OVERLAY', 1).catch(e => { log('overlay: xprop failed, using the old way: ' + e.message); overlay.destroy(); overlay = null; });
  overlay.loadURL('nimbus://app/index.html?overlay=full');
  // Last line of defence: an overlay that's up while nothing asked for it gets put away.
  setInterval(() => {
    if (overlay && !overlay.isDestroyed() && !overlayOn && !osdOverlay && overlay.isVisible()) { log('overlay: up without being open, putting it away'); parkOverlay(); }
  }, 2000);
}

/** Out of sight for sure: unmapped, no longer marked as an overlay for gamescope, moved off the screen.
 *  (Just hiding it wasn't always enough: gamescope sometimes kept showing an overlay window that had been
 *  hidden, and the overlay looked stuck open over Hearth or the next app.) */
function parkOverlay() {
  if (!overlay || overlay.isDestroyed()) return;
  overlay.hide();
  setWinAtom(overlay, 'STEAM_INPUT_FOCUS', 0).catch(() => {});
  setWinAtom(overlay, 'STEAM_OVERLAY', 0).catch(() => {});
  const b = screen.getPrimaryDisplay().bounds;
  overlay.setBounds({ x: b.x - b.width - 100, y: b.y, width: b.width, height: b.height });
}
function unparkOverlay() {
  const b = screen.getPrimaryDisplay().bounds;
  overlay.setBounds(b);
  setWinAtom(overlay, 'STEAM_OVERLAY', 1).catch(() => {});
}

let overlayAlive = 0, overlayWatch = null, beforeOverlay = 0;
function showOverlay() {
  if (!overlay) return false;
  overlayOn = true;
  unparkOverlay();
  // What was on screen, to give it back the controller when the overlay closes.
  gamescope.cardinals('GAMESCOPE_FOCUSED_WINDOW').then(f => { beforeOverlay = f[0] || 0; }).catch(() => {});
  // The page says it's alive every second while open; silence for 5 s means it froze: reload it, close it.
  overlayAlive = Date.now();
  clearInterval(overlayWatch);
  overlayWatch = setInterval(() => {
    if (!overlayOn) return clearInterval(overlayWatch);
    if (Date.now() - overlayAlive > 5000) {
      log('overlay: page stopped answering, closing and reloading it');
      clearInterval(overlayWatch);
      hideOverlay();
      overlay.webContents.reloadIgnoringCache();
    }
  }, 1000);
  // Unmapped and mapped again, so gamescope sees it as the newest overlay (Steam has overlay windows too).
  if (overlay.isVisible()) overlay.hide();
  overlay.showInactive();
  overlay.focus();
  setWinAtom(overlay, 'STEAM_OVERLAY', 1).catch(() => {});
  setWinAtom(overlay, 'STEAM_INPUT_FOCUS', 1).catch(() => {});
  // The controller is read here (evdev) and passed on: a window gamescope draws on top doesn't reliably get
  // it through the browser's Gamepad API, which left the overlay open with no way to close it.
  const sendMsg = m => overlay.webContents.send('nimbus:msg', m);
  sendMsg({ type: 'event', name: 'padHost', data: { connected: guide && guide.open.size ? 1 : 0 } });
  sendMsg({ type: 'event', name: 'overlay.shown', data: {} });
  return true;
}
function hideOverlay() {
  if (!overlay) return;
  if (!overlayOn && !overlay.isVisible()) return;
  overlayOn = false;
  osdOverlay = false;
  stopRepeat();
  overlay.webContents.send('nimbus:msg', { type: 'event', name: 'overlay.hidden', data: {} });
  parkOverlay();
  clearInterval(overlayWatch);
  // Belt and braces: if it's somehow still up a moment later, put it away again.
  setTimeout(() => { if (!overlayOn && overlay && !overlay.isDestroyed() && overlay.isVisible()) parkOverlay(); }, 600);
  // Give the controller back to what was on screen (gamescope can leave it with nothing in front), unless
  // something else was chosen in the meantime (Back to Hearth, Switch to an app, Back to game).
  const hidAt = Date.now();
  if (inGamescope() && !(win && win.isVisible() && win.isFocused())) {
    setTimeout(() => {
      if (overlayOn || focusChosenAt >= hidAt) return;
      if (beforeOverlay && beforeOverlay !== xid(overlay)) gamescope.focusWindow(beforeOverlay).catch(() => {});
      else if (library.runningId) focusRunningGame();
    }, 350);   // long enough for a Switch / Back to Hearth that follows the close
  }
}

// Controller presses for the overlay, with repeat on held directions like the UI's own.
let repeatTimer = null, lastPad = { b: '', t: 0 };
function stopRepeat() { clearTimeout(repeatTimer); repeatTimer = null; }
let bHold = null;
function overlayPad(b, down) {
  if (!overlayOn || !overlay) return;
  // Holding B for a second always closes it, even if the page has stopped answering.
  if (b === 'b') { clearTimeout(bHold); if (down) bHold = setTimeout(() => { log('overlay: B held, closing'); hideOverlay(); }, 1000); }
  if (!down) { stopRepeat(); return; }
  const now = Date.now();
  if (b === lastPad.b && now - lastPad.t < 40) return;      // the same press from two devices
  lastPad = { b, t: now };
  if (b === 'guide') return;                                 // Guide closes it (handled with 'press')
  const send = () => overlay && !overlay.isDestroyed() && overlay.webContents.send('nimbus:msg', { type: 'event', name: 'pad', data: { b } });
  send();
  stopRepeat();
  if (['up', 'down', 'left', 'right'].includes(b)) {
    const again = () => { send(); repeatTimer = setTimeout(again, 85); };
    repeatTimer = setTimeout(again, 340);
  }
}

/** Is Hearth's own window the one on screen? (gamescope knows; elsewhere, window focus.) */
async function hearthOnScreen() {
  if (!win || !win.isVisible()) return false;
  if (!inGamescope()) return win.isFocused();
  const f = (await gamescope.cardinals('GAMESCOPE_FOCUSED_WINDOW').catch(() => []))[0];
  return f ? f === xid(win) : win.isFocused();
}

// ------------------------------------------------------------------ power and volume buttons (Game Mode)
// hearth-buttons reads them from the kernel; what the power button does is in Settings › Power.
let osdTimer = null, osdOverlay = false;
const hearthInFront = () => win && win.isVisible() && win.isFocused() && !overlayOn;

/** The volume bar: in Hearth when it's in front, else over the game (the overlay, without taking input). */
function osd(data) {
  if (hearthInFront() || !overlay || !library.runningId) return post({ type: 'event', name: 'osd', data });
  if (!overlayOn && !osdOverlay) { osdOverlay = true; unparkOverlay(); overlay.showInactive(); }
  overlay.webContents.send('nimbus:msg', { type: 'event', name: 'osd', data });
  clearTimeout(osdTimer);
  osdTimer = setTimeout(() => { if (osdOverlay && !overlayOn) parkOverlay(); osdOverlay = false; }, 1800);
}

function powerMenu() {
  if (library.runningId && !hearthInFront()) {
    if (overlayOn || showOverlay()) { osdOverlay = false; overlay.webContents.send('nimbus:msg', { type: 'event', name: 'power.menu', data: {} }); return; }
    comeBack();
  }
  post({ type: 'event', name: 'power.menu', data: {} });
}

function startButtons() {
  buttons = new Buttons(path.join(DESKTOP_DIR, 'shell', 'hearth-buttons'));
  const max = () => buttonPrefs.maxVolume();
  buttons.on('volume', d => bridge.audio.step(d, max()).then(v => osd({ kind: 'vol', ...v, max: max() })).catch(e => log('volume: ' + e.message)));
  buttons.on('mute', () => bridge.audio.toggleMute().then(v => osd({ kind: 'vol', ...v, max: max() })).catch(e => log('mute: ' + e.message)));
  buttons.on('power', kind => {
    const what = buttonPrefs.get()[kind === 'long' ? 'hold' : 'press'];
    log(`power button: ${kind} press → ${what}`);
    if (what === 'sleep') power('sleep');
    else if (what === 'shutdown') power('shutdown');
    else if (what === 'menu') powerMenu();
  });
  buttons.start();
}

/** Frame-rate limit for games in Game Mode (gamescope's own limiter); 0 = no limit. */
async function setFpsLimit(fps) {
  const n = Math.max(0, Math.min(240, Math.round(+fps || 0)));
  if (!IN_SESSION || !require('./util').has('xprop')) throw new Error('The frame-rate limit works in Game Mode.');
  await setRootAtom('GAMESCOPE_FPS_LIMIT', n);
  library.store.update(d => { d.fpsLimit = n; });
  return n;
}

// ------------------------------------------------------------------ screenshots

/**
 * Saves a screenshot of the game to Pictures/Screenshots/Nimbus. Nimbus steps aside first so the picture
 * shows the game, not Quick Access. Uses whichever tool this desktop has.
 */
async function screenshot() {
  const pics = (await run('xdg-user-dir', ['PICTURES']).catch(() => '')).trim() || path.join(os.homedir(), 'Pictures');
  const dir = path.join(pics, 'Screenshots', 'Hearth');
  fs.mkdirSync(dir, { recursive: true });
  const file = path.join(dir, `Screenshot ${new Date().toISOString().slice(0, 19).replace('T', ' ').replace(/:/g, '-')}.png`);
  const wasVisible = win && win.isVisible();
  const overlayWasOn = overlayOn;
  if (overlayWasOn) { overlay.hide(); await new Promise(r => setTimeout(r, 450)); }
  if (wasVisible) { win.hide(); await new Promise(r => setTimeout(r, 450)); }
  const tries = [
    ['grim', [file]],                                   // wlroots desktops (Desktop Mode)
    ['spectacle', ['-b', '-n', '-f', '-o', file]],      // KDE
    ['gnome-screenshot', ['-f', file]],                 // GNOME
    ['gamescopectl', ['screenshot', file]],             // gamescope (newer versions)
  ];
  let ok = false;
  try {
    for (const [cmd, args] of tries) {
      if (!require('./util').has(cmd)) continue;
      try { await run(cmd, args, { timeout: 10000 }); } catch { continue; }
      if (fs.existsSync(file)) { ok = true; break; }
    }
  } finally { if (wasVisible) comeBack(); if (overlayWasOn && overlay) overlay.showInactive(); }
  if (!ok) throw new Error('No screenshot tool found. Install grim (Desktop Mode), Spectacle (KDE) or GNOME Screenshot.');
  return { file };
}

// ------------------------------------------------------------------ startup

function createWindow() {
  win = new BrowserWindow({
    title: START_PAGE === 'settings' ? 'Hearth Settings' : 'Hearth',
    backgroundColor: '#0c1016',
    show: false,
    fullscreen: !WINDOWED, width: 1280, height: 800,
    icon: path.join(__dirname, 'hearth-icon.png'),
    autoHideMenuBar: true,
    webPreferences: { preload: path.join(__dirname, 'preload.js'), contextIsolation: true, sandbox: true, nodeIntegration: false },
  });
  win.once('ready-to-show', () => win.show());
  // Desktop Mode couldn't start and sent us back here: say why (see desktop/labwc/autostart).
  win.webContents.once('did-finish-load', () => setTimeout(() => {
    const f = path.join(process.env.XDG_RUNTIME_DIR || '/tmp', 'nimbus-desktop-failed');
    let text = '';
    try { text = fs.readFileSync(f, 'utf8'); fs.rmSync(f, { force: true }); } catch { return; }
    log('desktop mode failed: ' + text);
    post({ type: 'event', name: 'desktopFailed', data: { text: text.trim().slice(0, 1500) } });
  }, 2500));
  win.webContents.on('did-finish-load', () => win.webContents.setZoomFactor((library.store.data.uiScale || 100) / 100));
  // Nimbus only ever shows its own pages.
  win.webContents.on('will-navigate', (e, url) => { if (!url.startsWith('nimbus://')) e.preventDefault(); });
  win.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
  if (!app.isPackaged) {
    win.webContents.on('before-input-event', (_e, i) => { if (i.type === 'keyDown' && i.key === 'F12') win.webContents.toggleDevTools(); });
  }
  // As the session there is nothing behind Nimbus, so Alt+F4 must not close it by accident.
  win.on('close', e => { if (IN_SESSION && !leaving) e.preventDefault(); });
  win.on('closed', () => { win = null; });
  win.loadURL('nimbus://app/index.html');
}

// Windows that gamescope draws without telling Chromium they're visible must keep running.
app.commandLine.appendSwitch('disable-renderer-backgrounding');
app.commandLine.appendSwitch('disable-backgrounding-occluded-windows');
app.commandLine.appendSwitch('disable-background-timer-throttling');

app.whenReady().then(() => {
  Menu.setApplicationMenu(null);
  secure.impl = safeStorage; // the desktop keyring encrypts the Xbox sign-in
  registerProtocols();

  stream = new StreamWindow({ BrowserWindow, session, windowed: WINDOWED });
  library = new Library({ stream });
  updates = new Updates({ version: app.getVersion(), autoCheck: () => library.store.data.autoUpdateCheck });

  // Interface zoom: the Scale setting in Game Mode (and on X11), remembered between runs.
  const getZoom = () => library.store.data.uiScale || 100;
  const setZoom = pct => { library.store.update(d => { d.uiScale = pct; }); if (win) win.webContents.setZoomFactor(pct / 100); };
  const displayModes = new DisplayModes({
    screen: () => {
      const d = screen.getPrimaryDisplay();
      return { w: Math.round(d.size.width * d.scaleFactor), h: Math.round(d.size.height * d.scaleFactor), rate: d.displayFrequency || 60 };
    },
    getZoom, setZoom,
    // New gamescope options take effect when Game Mode starts again: ask the session to restart it.
    restartGameMode: () => {
      leaving = true;
      if (IN_SESSION) leaveGameMode('game');
      else relaunch();
    },
  });

  bridge = new Bridge({
    post, library, updates,
    platform: {
      displayModes,
      info: () => ({
        platform: 'linux', name: osName(), host: `Electron ${process.versions.electron}`, version: app.getVersion(),
        session: IN_SESSION, user: os.userInfo().username,
        system: sessionSetup.isSystemInstall(),   // part of Hearth (the OS): updates come with the system
        start: START_PAGE,                         // the page to open on
        live: fs.existsSync('/etc/hearth-live'),    // running from the live USB
        features: ['restartApp', 'exportLogs', 'powerButton', 'overamp', ...(controller.available ? ['remap'] : [])],
      }),
      restartApp: () => { setTimeout(relaunch, 300); return null; },
      canFpsLimit: () => IN_SESSION && require('./util').has('xprop'),
      // The overlay's open apps (Game Mode): list, switch to, close.
      apps: {
        supported: () => inGamescope(),
        // Hearth is always first (hidden while a game plays, so gamescope doesn't list it), then every window
        // that's open, then Steam if it's running in the background without a window.
        list: async () => {
          const game = new Set(library.gamePids());
          const own = [xid(win), overlay ? xid(overlay) : 0].filter(Boolean);
          const list = (await gamescope.apps({ exclude: [process.pid, ...own] })).map(a => ({ ...a, hearth: false, game: game.has(a.pid) }));
          const hearthFront = win && win.isVisible() && !list.some(a => a.focused);
          const out = [{ id: xid(win), pid: process.pid, title: 'Hearth', app: 'hearth', hearth: true, game: false, focused: hearthFront }, ...list];
          if (steam.isRunning() && !list.some(a => /steam/i.test(a.app || '') || /^Steam$/i.test(a.title))) {
            out.push({ id: 0, pid: 0, title: 'Steam', app: 'steam', steam: true, game: false, focused: false });
          }
          return out;
        },
        focus: async (id, pid) => {
          chooseFocus();   // before closing the overlay, so it doesn't hand the screen back to the old app
          hideOverlay();
          if (pid === process.pid) return comeBack();
          if (!id && !pid) { stepAside(); return steam.openUri('steam://open/main'); }   // Steam without a window
          stepAside();
          await gamescope.focusWindow(id);
          chooseFocus();
          setTimeout(() => { if (overlayOn) hideOverlay(); }, 400);
        },
        close: async pid => {
          if (!pid || pid === process.pid) throw new Error("Hearth can't close itself here.");
          if (library.gamePids().includes(pid)) return library.closeRunning();
          try { process.kill(pid, 'SIGTERM'); } catch (e) { throw new Error("Couldn't close it: " + e.message); }
          return true;
        },
      },
      setFpsLimit,
      exportLogs,
      pickGameFile: async () => {
        const r = await withDialog(() => dialog.showOpenDialog(win, {
          title: 'Add a game to Hearth', properties: ['openFile'],
          filters: [{ name: 'Games', extensions: ['exe', 'desktop', 'AppImage', 'appimage', 'sh', 'x86_64', '*'] }],
        }));
        return r.canceled ? null : r.filePaths[0];
      },
      pickImage: async kind => {
        const r = await withDialog(() => dialog.showOpenDialog(win, {
          title: kind === 'avatar' ? 'Choose a profile picture' : 'Choose a background picture', properties: ['openFile'],
          filters: [{ name: 'Pictures', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif'] }],
        }));
        return r.canceled ? null : r.filePaths[0];
      },
      // Xbox signs in inside the cloud-gaming window; resolves when it's closed.
      signInWindow,
      pickFolder: async current => {
        const r = await withDialog(() => dialog.showOpenDialog(win, { title: 'Install folder for Epic and GOG games', defaultPath: current,
          properties: ['openDirectory', 'createDirectory'] }));
        return r.canceled ? null : r.filePaths[0];
      },
      screenshot,
      // Bringing Nimbus up over a game (the Linux "overlay"): Guide button, optional held View, Ctrl+Alt+Q.
      overlay: {
        settings: () => ({ holdView: !!library.store.data.holdView, gameBarButton: false, hotkey: 'Ctrl+Alt+Q',
          guide: guide ? guide.status() : { devices: [], problems: [], lastPress: 0 } }),
        setHoldView: on => { library.store.update(d => { d.holdView = !!on; }); if (guide) guide.holdView = !!on; },
      },
      screenInfo: () => {
        const d = screen.getPrimaryDisplay();
        const w = Math.round(d.size.width * d.scaleFactor), h = Math.round(d.size.height * d.scaleFactor);
        return { resolution: `${w} × ${h}`, scale: Math.round(d.scaleFactor * 100) + '%' };
      },
      session: {
        status: () => sessionSetup.status(),
        register: () => sessionSetup.register(SESSION_DIR, DESKTOP_DIR, APP_ROOT),
        unregister: () => sessionSetup.unregister(SESSION_DIR, DESKTOP_DIR),
      },
      installUpdate: () => {
        try { updates.install(); } catch (e) { post({ type: 'event', name: 'update', data: { ...updates.status(), error: e.message } }); return; }
        leaving = true;
        if (IN_SESSION) app.exit(UPDATE_EXIT_CODE);
        else relaunch();
      },
      returnToDesktop: () => power('desktop'),
      buttonPrefs, controller, browser: new Browser(),
      // Steam's own window (its downloads page): out of Hearth's way so gamescope shows it; Guide comes back.
      showSteam: () => { stepAside(); if (inGamescope()) gamescope.clearFocus().catch(() => {}); },
      get buttons() { return buttons; },
    },
  });

  // Offline: know it (launches skip waiting on the network), and when it comes back, finish downloading art.
  const { setOnline } = require('./util');
  let wasOnline = net.isOnline();
  setOnline(wasOnline);
  setInterval(() => {
    const on = net.isOnline();
    if (on === wasOnline) return;
    wasOnline = on; setOnline(on);
    log(`network: ${on ? 'online' : 'offline'}`);
    post({ type: 'event', name: 'net', data: { online: on } });
    if (on) require('./art-cache').artCache().retry();
  }, 15000);
  // Art finished downloading: the UI reloads the library quietly to use the on-device copies.
  require('./art-cache').artCache().on('changed', () => post({ type: 'event', name: 'library.art', data: {} }));

  library.on('started', id => {
    // Xbox Cloud Gaming plays in Hearth's own stream window: that's the one to put in front.
    if (stream.isOpen) {
      stepAside(); post({ type: 'event', name: 'game.ready', data: { id } });
      if (inGamescope()) setTimeout(() => { if (stream.isOpen && stream.win && !stream.win.isDestroyed()) gamescope.focusWindow(xid(stream.win)).catch(() => {}); }, 900);
      return;
    }
    if (!inGamescope()) { stepAside(); focusRunningGame(); post({ type: 'event', name: 'game.ready', data: { id } }); return; }
    showGameWhenReady(id);
  });
  library.on('exited', () => { hideOverlay(); if (inGamescope()) gamescope.clearFocus().catch(() => {}); comeBack(); });
  stream.on('closed', () => library.markExited());
  ipcMain.on('stream:exit', () => stream.close());
  ipcMain.on('nimbus:msg', onMessage);

  // Guide button over a game: bring Nimbus back with Quick Access open.
  guide = new GuideButton();
  guide.holdView = !!library.store.data.holdView;
  // Over anything that isn't Hearth (a game, Steam, any app), Guide opens the overlay; again closes it.
  guide.on('press', async how => {
    if (overlayOn) { log(`overlay: ${how} pressed, closing`); return hideOverlay(); }
    if (await hearthOnScreen()) return;           // Hearth's own UI handles Guide itself
    log(`overlay: ${how} pressed over ${library.runningId ? 'a game' : 'another app'}`);
    if (showOverlay()) return;
    comeBack();
    post({ type: 'event', name: 'pad', data: { b: 'guide' } });
  });
  guide.on('button', overlayPad);
  // Opened just for Settings (from Desktop Mode): the desktop's own controller support handles Guide,
  // and the desktop handles the power and volume buttons.
  if (!START_PAGE) guide.start();
  if (IN_SESSION && !WINDOWED && !START_PAGE) startButtons();
  // Keyboard backup, like Windows: works when the game runs under X11/Xwayland (most do, and all in gamescope).
  try { globalShortcut.register('Control+Alt+Q', () => guide.fire('hotkey')); } catch (e) { log('Ctrl+Alt+Q: ' + e.message); }

  createWindow();
  createOverlay(bridge.platform);
  if (IN_SESSION && library.store.data.fpsLimit) setFpsLimit(library.store.data.fpsLimit).catch(() => {});
  library.startBackgroundServices().catch(e => log('background services FAILED ' + e.message));
  setTimeout(() => { if (library.store.data.autoUpdateCheck) updates.check(); }, 20000);
  setInterval(() => { if (library.store.data.autoUpdateCheck) updates.check(); }, 6 * 60 * 60 * 1000);
});

// Started again (e.g. Settings from the Desktop Mode taskbar while Hearth is open): show this one.
app.on('second-instance', (_e, argv) => {
  comeBack();
  if (argv.includes('--settings')) post({ type: 'event', name: 'open', data: { view: 'settings' } });
});
app.on('window-all-closed', () => app.quit());
app.on('will-quit', () => { guide && guide.stop(); buttons && buttons.stop(); bridge && bridge.dispose(); });
