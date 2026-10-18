'use strict';
// Xbox Cloud Gaming in a fullscreen Nimbus window. The Xbox sign-in is kept in its own browser profile
// (partition "persist:xcloud") so you only sign in once, and it never sees Nimbus's own pages.
// To leave a stream: hold View + Menu for about 1.5 seconds, press the Guide button, or Ctrl+Shift+Q.

const path = require('node:path');
const { EventEmitter } = require('node:events');

/** Xbox Cloud Gaming serves its best quality to Edge on Windows, so the stream window says it is one. */
function edgeUserAgent(chromeVersion) {
  const major = String(chromeVersion || '138').split('.')[0];
  return `Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${major}.0.0.0 Safari/537.36 Edg/${major}.0.0.0`;
}

const ALLOWED_HOSTS = /(^|\.)(xbox\.com|microsoft\.com|live\.com|microsoftonline\.com|xboxlive\.com|msauth\.net|msftauth\.net|gssv-play-prod\.xboxlive\.com)$/;

class StreamWindow extends EventEmitter {
  constructor({ BrowserWindow, session, windowed }) {
    super();
    this.BrowserWindow = BrowserWindow;
    this.session = session;
    this.windowed = windowed;
    this.win = null;
  }

  open(url, title) {
    if (this.win && !this.win.isDestroyed()) { this.win.loadURL(url); this.win.show(); this.win.focus(); return; }
    // Each profile signs in to Xbox Cloud Gaming on its own.
    let pid = 'main';
    try { pid = new (require('./profiles').Profiles)(require('./util').DATA_DIR).active; } catch {}
    const ses = this.session.fromPartition(pid === 'main' ? 'persist:xcloud' : 'persist:xcloud-' + pid);
    ses.setUserAgent(edgeUserAgent(process.versions.chrome));
    this.win = new this.BrowserWindow({
      title: title || 'Xbox Cloud Gaming',
      backgroundColor: '#000000',
      fullscreen: !this.windowed, width: 1280, height: 800, autoHideMenuBar: true,
      webPreferences: {
        session: ses,
        preload: path.join(__dirname, 'stream-preload.js'),
        contextIsolation: true, sandbox: true, nodeIntegration: false,
      },
    });
    const wc = this.win.webContents;
    // Sign-in and the stream stay in this window; anything else (help links, stores) opens nowhere.
    wc.setWindowOpenHandler(({ url: u }) => {
      try { if (ALLOWED_HOSTS.test(new URL(u).hostname)) wc.loadURL(u); } catch {}
      return { action: 'deny' };
    });
    wc.on('before-input-event', (e, input) => {
      if (input.type === 'keyDown' && input.control && input.shift && input.key.toLowerCase() === 'q') { e.preventDefault(); this.close(); }
    });
    // The page can ask for fullscreen (and keyboard lock); allow it, and nothing else.
    ses.setPermissionRequestHandler((_wc, permission, cb) => cb(['fullscreen', 'keyboardLock', 'pointerLock'].includes(permission)));
    this.win.on('closed', () => { this.win = null; this.emit('closed'); });
    this.win.loadURL(url);
  }

  focus() { if (this.win && !this.win.isDestroyed()) { this.win.show(); this.win.focus(); } }
  get isOpen() { return !!this.win && !this.win.isDestroyed(); }
  close() { if (this.win && !this.win.isDestroyed()) this.win.close(); }
}

module.exports = { StreamWindow, edgeUserAgent };
