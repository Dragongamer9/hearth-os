'use strict';
// Nimbus for Linux: hosts the shared web UI (../../ui) in a fullscreen Electron window.
// Inside the "Nimbus" login session it runs in gamescope (see session/); started from a desktop
// with --windowed it runs in a normal window for development.

const { app, BrowserWindow, ipcMain, protocol, net, dialog, screen, session, Menu } = require('electron');
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
const sessionSetup = require('./session');
const { run, readText, log } = require('./util');

const args = process.argv.slice(1);
const WINDOWED = args.includes('--windowed');
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

// ------------------------------------------------------------------ startup

function createWindow() {
  win = new BrowserWindow({
    title: 'Nimbus',
    backgroundColor: '#0c1016',
    show: false,
    fullscreen: !WINDOWED, width: 1280, height: 800,
    autoHideMenuBar: true,
    webPreferences: { preload: path.join(__dirname, 'preload.js'), contextIsolation: true, sandbox: true, nodeIntegration: false },
  });
  win.once('ready-to-show', () => win.show());
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
      } else { app.relaunch(); app.exit(0); }
    },
  });

  bridge = new Bridge({
    post, library, updates,
    platform: {
      displayModes,
      info: () => ({
        platform: 'linux', name: osName(), host: `Electron ${process.versions.electron}`, version: app.getVersion(),
        session: IN_SESSION, user: os.userInfo().username,
      }),
      pickGameFile: async () => {
        const r = await dialog.showOpenDialog(win, {
          title: 'Add a game to Nimbus', properties: ['openFile'],
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
      openXboxSignIn: () => new Promise(resolve => {
        stream.open('https://play.xbox.com/', 'Sign in to Xbox');
        stream.once('closed', resolve);
      }),
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
        else { app.relaunch(); app.exit(0); }
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
  guide.on('press', () => {
    if (!library.runningId || (win && win.isVisible() && win.isFocused())) return; // the UI handles it itself then
    comeBack();
    post({ type: 'event', name: 'pad', data: { b: 'guide' } });
  });
  guide.start();

  createWindow();
  library.startBackgroundServices().catch(e => log('background services FAILED ' + e.message));
  setTimeout(() => { if (library.store.data.autoUpdateCheck) updates.check(); }, 20000);
  setInterval(() => { if (library.store.data.autoUpdateCheck) updates.check(); }, 6 * 60 * 60 * 1000);
});

app.on('second-instance', () => comeBack());
app.on('window-all-closed', () => app.quit());
app.on('will-quit', () => { guide && guide.stop(); bridge && bridge.dispose(); });
