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
const { DisplayModes } = require('./display');
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
let library, bridge, updates, stream, guide;

/** Restart Nimbus. As an AppImage, start the AppImage file again (it may have just been replaced by an update). */
function relaunch() {
  if (process.env.APPIMAGE) app.relaunch({ execPath: process.env.APPIMAGE, args: process.argv.slice(1) });
  else app.relaunch();
  app.exit(0);
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
      : library && library.artFile(parts[0], parts[1]);
    return file ? net.fetch(pathToFileURL(file).toString()) : new Response('Not found', { status: 404 });
  });
}

// ------------------------------------------------------------------ showing and hiding Nimbus around games

/** A game is in front: get out of its way (gamescope shows whichever window is on top). */
function stepAside() { if (win && !WINDOWED) win.hide(); }

/** Back in front, e.g. when a game ends or the Guide button is pressed. */
function comeBack() {
  if (!win || win.isDestroyed()) return;
  if (win.isMinimized()) win.restore();
  win.show();
  win.focus();
}

function resumeGame() {
  if (!library.runningId) return;
  if (stream.isOpen) { stepAside(); stream.focus(); } else stepAside();
}

// ------------------------------------------------------------------ messages from the UI

function onMessage(_e, m) {
  if (!m || typeof m !== 'object') return;
  switch (m.type) {
    case 'ready': post({ type: 'event', name: 'padHost', data: { connected: 0 } }); break; // the UI reads pads itself
    case 'call': bridge.handle(m); break;
    case 'log': log('ui: ' + m.text); break;
    case 'power': power(m.action); break;
    case 'resume': resumeGame(); break; // Quick Access closed while a game is running
    case 'setting': break;
  }
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
      if (IN_SESSION) {
        const next = path.join(process.env.XDG_RUNTIME_DIR || '/tmp', 'nimbus-next');
        try { if (action === 'desktop') fs.writeFileSync(next, 'desktop\n'); else fs.rmSync(next, { force: true }); } catch (e) { log('mode switch: ' + e.message); }
        leaving = true; app.exit(0);
      }
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
    w.on('closed', () => finish(null));
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
  } finally { if (wasVisible) comeBack(); }
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
      if (IN_SESSION) {
        try { fs.writeFileSync(path.join(process.env.XDG_RUNTIME_DIR || '/tmp', 'nimbus-next'), 'game\n'); } catch {}
        app.exit(0);
      } else { relaunch(); }
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
      }),
      pickGameFile: async () => {
        const r = await dialog.showOpenDialog(win, {
          title: 'Add a game to Hearth', properties: ['openFile'],
          filters: [{ name: 'Games', extensions: ['exe', 'desktop', 'AppImage', 'appimage', 'sh', 'x86_64', '*'] }],
        });
        return r.canceled ? null : r.filePaths[0];
      },
      pickImage: async kind => {
        const r = await dialog.showOpenDialog(win, {
          title: kind === 'avatar' ? 'Choose a profile picture' : 'Choose a background picture', properties: ['openFile'],
          filters: [{ name: 'Pictures', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif'] }],
        });
        return r.canceled ? null : r.filePaths[0];
      },
      // Xbox signs in inside the cloud-gaming window; resolves when it's closed.
      signInWindow,
      pickFolder: async current => {
        const r = await dialog.showOpenDialog(win, { title: 'Install folder for Epic and GOG games', defaultPath: current,
          properties: ['openDirectory', 'createDirectory'] });
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
    },
  });

  library.on('started', () => stepAside());
  library.on('exited', () => comeBack());
  stream.on('closed', () => library.markExited());
  ipcMain.on('stream:exit', () => stream.close());
  ipcMain.on('nimbus:msg', onMessage);

  // Guide button over a game: bring Nimbus back with Quick Access open.
  guide = new GuideButton();
  guide.holdView = !!library.store.data.holdView;
  guide.on('press', how => {
    log(`overlay: ${how} pressed${library.runningId ? '' : ' (no game running)'}`);
    if (!library.runningId || (win && win.isVisible() && win.isFocused())) return; // the UI handles it itself then
    comeBack();
    post({ type: 'event', name: 'pad', data: { b: 'guide' } });
  });
  // Opened just for Settings (from Desktop Mode): the desktop's own controller support handles Guide.
  if (!START_PAGE) guide.start();
  // Keyboard backup, like Windows: works when the game runs under X11/Xwayland (most do, and all in gamescope).
  try { globalShortcut.register('Control+Alt+Q', () => guide.fire('hotkey')); } catch (e) { log('Ctrl+Alt+Q: ' + e.message); }

  createWindow();
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
app.on('will-quit', () => { guide && guide.stop(); bridge && bridge.dispose(); });
