'use strict';
// Hearth Browser: a web browser in Hearth's look, built on the Electron that Hearth already ships (so it
// adds almost nothing to Hearth OS). Started with `nimbus --browser [url]` (/usr/bin/hearth-browser).
//
// One window per browser: the top bar (tabs, back/forward/reload, the address bar, bookmarks and the menu)
// is its own page (chrome.html); each tab is a WebContentsView below it. Hearth's own pages (the start
// page, history, bookmarks, downloads, settings) are hearth:// pages (pages.html). Bookmarks, history and
// settings live in the browser's profile folder (store.js); tabs come back at the next start.

const { app, BaseWindow, WebContentsView, ipcMain, protocol, net, session, Menu, clipboard, shell, dialog } = require('electron');
const path = require('node:path');
const fs = require('node:fs');
const os = require('node:os');
const { pathToFileURL } = require('node:url');
const { BrowserStore, toUrl, displayUrl, SEARCH, START_TILES } = require('./store');

// Its own profile, separate from Hearth's (and its own "only one copy running" lock).
app.setName('Hearth Browser');
app.setPath('userData', path.join(app.getPath('appData'), 'hearth-browser'));
app.commandLine.appendSwitch('enable-features', 'VaapiVideoDecoder,VaapiVideoDecodeLinuxGL');

protocol.registerSchemesAsPrivileged([{ scheme: 'hearth', privileges: { standard: true, secure: true, supportFetchAPI: true } }]);

const CHROME_H = 86;                 // the tab strip and toolbar
const HERE = __dirname;
const LOGO = fs.existsSync(path.join(HERE, '..', 'hearth-icon.png')) ? path.join(HERE, '..', 'hearth-icon.png') : undefined;
let store;
const windows = new Set();

const urlsFromArgs = argv => argv.slice(1).filter(a => !a.startsWith('-') && /^(https?:|file:|hearth:|[\w-]+\.[\w.-]+)/i.test(a)).map(a => toUrl(a)).filter(Boolean);

if (!app.requestSingleInstanceLock()) { app.quit(); }
else {
  app.on('second-instance', (_e, argv) => {
    const urls = urlsFromArgs(argv);
    const w = [...windows].pop();
    if (!w) return openWindow(urls);
    if (urls.length) urls.forEach(u => w.newTab(u, true)); else w.newTab(null, true);
    w.win.show(); w.win.focus();
  });
  app.whenReady().then(start);
  app.on('window-all-closed', () => { store && store.flush(); app.quit(); });
}

function start() {
  store = new BrowserStore(path.join(app.getPath('userData'), 'data'));
  Menu.setApplicationMenu(null);
  const ses = session.fromPartition('persist:hearth-browser');
  // Sites see an ordinary Chrome (some refuse Electron's user agent).
  ses.setUserAgent(ses.getUserAgent().replace(/ Electron\/\S+/, '').replace(/ (Nimbus|Hearth[\w ]*)\/\S+/gi, ''));
  ses.setPermissionRequestHandler((wc, permission, cb) => cb(['fullscreen', 'clipboard-sanitized-write', 'notifications', 'media', 'pointerLock', 'mediaKeySystem'].includes(permission)));
  ses.on('will-download', (_e, item) => downloads.add(item));
  // Hearth's own pages
  const pages = scheme => scheme.handle('hearth', req => {
    const u = new URL(req.url);
    const file = /\.(js|css|svg|png)$/.test(u.pathname) ? path.join(HERE, path.basename(u.pathname)) : path.join(HERE, 'pages.html');
    return net.fetch(pathToFileURL(file).toString());
  });
  pages(ses.protocol);
  pages(protocol);
  adblock.set(store.settings.adblock !== false);
  const restore = store.settings.restoreTabs ? store.session : [];
  const urls = urlsFromArgs(process.argv);
  openWindow(urls.length ? (restore.length ? [...restore, ...urls] : urls) : restore);
}

// ---------------------------------------------------------------- ad and tracker blocking
// Ghostery's blocker with its ready-made ads and trackers lists (EasyList, EasyPrivacy and others): blocks the
// requests and hides the empty boxes ads leave. The lists are downloaded once and kept in the profile folder.
const adblock = {
  blocker: null, on: false, blocked: 0,
  async set(on) {
    const ses = session.fromPartition('persist:hearth-browser');
    this.on = on;
    try {
      if (on) {
        if (!this.blocker) {
          const { ElectronBlocker } = require('@ghostery/adblocker-electron');
          const cache = path.join(app.getPath('userData'), 'adblock-engine.bin');
          this.blocker = await ElectronBlocker.fromPrebuiltAdsAndTracking(fetch, { path: cache, read: fs.promises.readFile, write: fs.promises.writeFile });
          this.blocker.on('request-blocked', () => { this.blocked++; });
        }
        if (this.on) this.blocker.enableBlockingInSession(ses);
      } else if (this.blocker) this.blocker.disableBlockingInSession(ses);
    } catch (e) { console.error('ad blocking:', e && e.message); }
  },
};

// ---------------------------------------------------------------- downloads
const downloads = {
  list: [],
  add(item) {
    const dir = app.getPath('downloads');
    let name = item.getFilename() || 'download', file = path.join(dir, name), n = 1;
    while (fs.existsSync(file)) { const ext = path.extname(name); file = path.join(dir, `${path.basename(name, ext)} (${n++})${ext}`); }
    item.setSavePath(file);
    const d = { id: Date.now() + Math.random(), name: path.basename(file), file, url: item.getURL(), total: item.getTotalBytes(), got: 0, state: 'progressing', item };
    this.list.unshift(d);
    if (this.list.length > 50) this.list.length = 50;
    item.on('updated', (_e, state) => { d.got = item.getReceivedBytes(); d.total = item.getTotalBytes(); d.state = state === 'interrupted' ? 'interrupted' : item.isPaused() ? 'paused' : 'progressing'; this.push(); });
    item.once('done', (_e, state) => { d.state = state; d.got = item.getReceivedBytes(); d.item = null; this.push(true); });
    this.push(true);
  },
  public() { return this.list.map(({ item, ...d }) => d); },
  push(force) {
    const now = Date.now();
    if (!force && now - (this.at || 0) < 400) return;
    this.at = now;
    for (const w of windows) w.toChrome('downloads', this.public());
  },
};

// ---------------------------------------------------------------- a browser window
function openWindow(urls = []) {
  const w = new BrowserWin();
  windows.add(w);
  if (urls.length) urls.forEach((u, i) => w.newTab(u, i === urls.length - 1));
  else w.newTab(null, true);
  return w;
}

class BrowserWin {
  constructor() {
    this.tabs = [];          // {id, view, title, favicon, loading, url}
    this.active = null;
    this.fullscreen = false;
    this.expanded = false;   // the top bar's menus open over the page
    this.win = new BaseWindow({ width: 1280, height: 820, minWidth: 560, minHeight: 400, title: 'Hearth Browser', backgroundColor: '#0f0b09', icon: LOGO, show: false });
    this.chrome = new WebContentsView({ webPreferences: { preload: path.join(HERE, 'chrome-preload.js'), contextIsolation: true, sandbox: true } });
    this.chrome.setBackgroundColor('#00000000');
    this.win.contentView.addChildView(this.chrome);
    this.chrome.webContents.loadFile(path.join(HERE, 'chrome.html'));
    this.chrome.webContents.on('before-input-event', (e, input) => this.shortcut(e, input));
    this.chrome.webContents.once('did-finish-load', () => { this.pushAll(); this.win.show(); });
    this.win.on('resize', () => this.layout());
    this.win.on('closed', () => {
      windows.delete(this);
      if (!windows.size) store.saveSession(this.tabs.map(t => t.url).filter(Boolean));
      store.flush();
    });
    this.win.on('close', () => { if (windows.size === 1) store.saveSession(this.tabs.map(t => t.url).filter(Boolean)); });
    this.layout();
  }

  layout() {
    const [w, h] = this.win.getContentSize();
    const top = this.fullscreen ? 0 : CHROME_H;
    this.chrome.setBounds({ x: 0, y: 0, width: w, height: this.fullscreen ? 0 : this.expanded ? h : CHROME_H });
    const t = this.tab();
    if (t) t.view.setBounds({ x: 0, y: top, width: w, height: Math.max(0, h - top) });
  }

  tab(id = this.active) { return this.tabs.find(t => t.id === id); }
  toChrome(name, data) { if (!this.chrome.webContents.isDestroyed()) this.chrome.webContents.send('hb', { name, data }); }

  state(t) {
    const wc = t.view.webContents;
    return { id: t.id, title: t.title || displayUrl(t.url) || 'New tab', url: t.url || '', display: displayUrl(t.url), favicon: t.favicon || null, loading: t.loading,
      back: wc.navigationHistory.canGoBack(), forward: wc.navigationHistory.canGoForward(), bookmarked: store.isBookmarked(t.url), secure: /^https:|^hearth:/.test(t.url || ''),
      zoom: Math.round(wc.getZoomFactor() * 100), audible: wc.isCurrentlyAudible(), muted: wc.isAudioMuted() };
  }
  pushAll() { this.toChrome('tabs', { tabs: this.tabs.map(t => this.state(t)), active: this.active, bookmarks: store.bookmarks.slice(0, 30), downloads: downloads.public() }); }
  pushTab(t) { this.toChrome('tab', this.state(t)); if (t.id === this.active) this.win.setTitle((t.title ? t.title + ' – ' : '') + 'Hearth Browser'); }

  newTab(url, focus = true, after = null) {
    const view = new WebContentsView({ webPreferences: { partition: 'persist:hearth-browser', preload: path.join(HERE, 'page-preload.js'), contextIsolation: true, sandbox: true, spellcheck: true } });
    view.setBackgroundColor('#ffffff');
    const t = { id: Date.now() + Math.floor(Math.random() * 1000), view, title: '', favicon: null, loading: false, url: '' };
    const at = after ? this.tabs.findIndex(x => x.id === after) + 1 : this.tabs.length;
    this.tabs.splice(at > 0 ? at : this.tabs.length, 0, t);
    const wc = view.webContents;
    wc.on('page-title-updated', (_e, title) => { t.title = title; store.retitle(t.url, title); this.pushTab(t); });
    wc.on('page-favicon-updated', (_e, icons) => { t.favicon = icons[0] || null; this.pushTab(t); });
    wc.on('did-start-loading', () => { t.loading = true; this.pushTab(t); });
    wc.on('did-stop-loading', () => { t.loading = false; this.pushTab(t); });
    const navigated = u => { t.url = u; if (!u.startsWith('hearth:')) store.visit(u, t.title); else t.favicon = null; this.pushTab(t); };
    wc.on('did-navigate', (_e, u) => { t.favicon = null; navigated(u); });
    wc.on('did-navigate-in-page', (_e, u, main) => { if (main) navigated(u); });
    wc.on('did-fail-load', (_e, code, desc, u, main) => { if (main && code !== -3) this.toChrome('error', { id: t.id, text: `Couldn't open ${displayUrl(u) || u}: ${desc}` }); });
    wc.on('audio-state-changed', () => this.pushTab(t));
    wc.on('zoom-changed', () => this.pushTab(t));
    wc.on('enter-html-full-screen', () => { this.fullscreen = true; this.win.setFullScreen(true); this.layout(); });
    wc.on('leave-html-full-screen', () => { this.fullscreen = false; this.win.setFullScreen(false); this.layout(); });
    wc.on('found-in-page', (_e, r) => this.toChrome('found', { matches: r.matches, at: r.activeMatchOrdinal }));
    wc.on('before-input-event', (e, input) => this.shortcut(e, input));
    wc.on('context-menu', (_e, p) => this.contextMenu(t, p));
    // Links that open a new window: a new tab next to this one (pop-ups from nowhere are blocked).
    wc.setWindowOpenHandler(({ url, disposition }) => {
      this.newTab(url, disposition !== 'background-tab', t.id);
      return { action: 'deny' };
    });
    wc.loadURL(url || store.settings.home || 'hearth://start');
    if (focus) this.select(t.id); else this.pushAll();
    return t;
  }

  select(id) {
    const t = this.tab(id); if (!t) return;
    const old = this.tab();
    if (old && old !== t) this.win.contentView.removeChildView(old.view);
    this.active = id;
    this.win.contentView.addChildView(t.view, 0);    // under the top bar (its menus open over the page)
    this.layout();
    this.pushAll();
    this.win.setTitle((t.title ? t.title + ' – ' : '') + 'Hearth Browser');
    if (!t.url || t.url.startsWith('hearth://start')) this.toChrome('focusAddress', {}); else t.view.webContents.focus();
  }

  close(id) {
    const i = this.tabs.findIndex(t => t.id === id); if (i < 0) return;
    const [t] = this.tabs.splice(i, 1);
    if (this.active === id) this.win.contentView.removeChildView(t.view);
    t.view.webContents.close();
    if (!this.tabs.length) { this.win.close(); return; }
    if (this.active === id) this.select(this.tabs[Math.min(i, this.tabs.length - 1)].id); else this.pushAll();
  }

  navigate(text) {
    const t = this.tab(); if (!t) return;
    const u = toUrl(text, store.settings.search);
    if (u) { t.view.webContents.loadURL(u); t.view.webContents.focus(); }
  }

  contextMenu(t, p) {
    const wc = t.view.webContents, items = [];
    if (p.linkURL) items.push({ label: 'Open link in new tab', click: () => this.newTab(p.linkURL, false, t.id) }, { label: 'Copy link', click: () => clipboard.writeText(p.linkURL) }, { type: 'separator' });
    if (p.mediaType === 'image' && p.srcURL) items.push({ label: 'Open image in new tab', click: () => this.newTab(p.srcURL, false, t.id) }, { label: 'Copy image', click: () => wc.copyImageAt(p.x, p.y) }, { label: 'Save image', click: () => wc.downloadURL(p.srcURL) }, { type: 'separator' });
    if (p.isEditable) items.push({ role: 'cut' }, { role: 'copy' }, { role: 'paste' }, { type: 'separator' });
    else if (p.selectionText) items.push({ role: 'copy' }, { label: `Search for "${p.selectionText.slice(0, 30)}"`, click: () => this.newTab(toUrl(p.selectionText, store.settings.search), true, t.id) }, { type: 'separator' });
    if (p.dictionarySuggestions && p.dictionarySuggestions.length) items.unshift(...p.dictionarySuggestions.slice(0, 4).map(s => ({ label: s, click: () => wc.replaceMisspelling(s) })), { type: 'separator' });
    items.push({ label: 'Back', enabled: wc.navigationHistory.canGoBack(), click: () => wc.navigationHistory.goBack() }, { label: 'Reload', click: () => wc.reload() });
    Menu.buildFromTemplate(items).popup();
  }

  /** Keyboard shortcuts, the same everywhere in the window. */
  shortcut(e, input) {
    if (input.type !== 'keyDown') return;
    const c = input.control || input.meta, k = input.key.toLowerCase(), t = this.tab(), wc = t && t.view.webContents;
    const done = f => { e.preventDefault(); f(); };
    if (c && k === 't') return done(() => this.newTab(null, true));
    if (c && k === 'w') return done(() => t && this.close(t.id));
    if (c && input.shift && k === 't') return;
    if ((c && k === 'l') || (input.alt && k === 'd') || k === 'f6') return done(() => this.toChrome('focusAddress', {}));
    if ((c && k === 'r') || k === 'f5') return done(() => wc && (input.shift ? wc.reloadIgnoringCache() : wc.reload()));
    if (input.alt && k === 'arrowleft') return done(() => wc && wc.navigationHistory.goBack());
    if (input.alt && k === 'arrowright') return done(() => wc && wc.navigationHistory.goForward());
    if (c && k === 'tab') return done(() => { const i = this.tabs.indexOf(t), n = this.tabs.length; this.select(this.tabs[(i + (input.shift ? n - 1 : 1)) % n].id); });
    if (c && /^[1-9]$/.test(k)) return done(() => { const i = k === '9' ? this.tabs.length - 1 : +k - 1; if (this.tabs[i]) this.select(this.tabs[i].id); });
    if (c && k === 'f') return done(() => this.toChrome('find', {}));
    if (c && k === 'd') return done(() => this.act({ act: 'bookmark' }));
    if (c && k === 'h') return done(() => this.newTab('hearth://history', true, t && t.id));
    if (c && k === 'j') return done(() => this.toChrome('openDownloads', {}));
    if (c && (k === '=' || k === '+')) return done(() => wc && wc.setZoomFactor(Math.min(3, wc.getZoomFactor() + 0.1)));
    if (c && k === '-') return done(() => wc && wc.setZoomFactor(Math.max(0.3, wc.getZoomFactor() - 0.1)));
    if (c && k === '0') return done(() => wc && wc.setZoomFactor(1));
    if (k === 'f11') return done(() => { this.win.setFullScreen(!this.win.isFullScreen()); });
    if (c && input.shift && k === 'n') return done(() => openWindow());
    if (c && k === 'n') return done(() => openWindow());
  }

  /** What the top bar asks for. */
  act(m) {
    const t = this.tab(), wc = t && t.view.webContents;
    switch (m.act) {
      case 'new': return this.newTab(m.url || null, true);
      case 'select': return this.select(m.id);
      case 'close': return this.close(m.id);
      case 'move': { const i = this.tabs.findIndex(x => x.id === m.id), j = Math.max(0, Math.min(this.tabs.length - 1, m.to)); if (i >= 0) { this.tabs.splice(j, 0, ...this.tabs.splice(i, 1)); this.pushAll(); } return; }
      case 'go': return this.navigate(m.text);
      case 'open': if (wc) { wc.loadURL(m.url); wc.focus(); } return;
      case 'back': return wc && wc.navigationHistory.goBack();
      case 'forward': return wc && wc.navigationHistory.goForward();
      case 'reload': return wc && (t.loading ? wc.stop() : wc.reload());
      case 'home': return wc && wc.loadURL(store.settings.home || 'hearth://start');
      case 'bookmark': if (t) { store.toggleBookmark(t.url, t.title); this.pushAll(); } return;
      case 'mute': if (wc) { wc.setAudioMuted(!wc.isAudioMuted()); this.pushTab(t); } return;
      case 'zoom': if (wc) wc.setZoomFactor(m.v === 0 ? 1 : Math.max(0.3, Math.min(3, wc.getZoomFactor() + m.v / 10))); this.pushTab(t); return;
      case 'find': if (wc) { if (m.text) wc.findInPage(m.text, { forward: m.forward !== false, findNext: !!m.next }); else wc.stopFindInPage('clearSelection'); } return;
      case 'expand': this.expanded = !!m.on; this.layout(); return;
      case 'focusPage': return wc && wc.focus();
      case 'internal': return this.newTab('hearth://' + m.page, true, t && t.id);
      case 'newWindow': return openWindow();
      case 'print': return wc && wc.print();
      case 'downloadOpen': { const d = downloads.list.find(x => x.id === m.id); if (d) shell.openPath(d.file); return; }
      case 'downloadShow': { const d = downloads.list.find(x => x.id === m.id); if (d) shell.showItemInFolder(d.file); return; }
      case 'downloadCancel': { const d = downloads.list.find(x => x.id === m.id); if (d && d.item) d.item.cancel(); return; }
      case 'quit': return app.quit();
    }
  }
}

// ---------------------------------------------------------------- messages
const winOf = wc => [...windows].find(w => w.chrome.webContents === wc || w.tabs.some(t => t.view.webContents === wc));
ipcMain.on('hb:act', (e, m) => { const w = winOf(e.sender); if (w && m && typeof m === 'object') w.act(m); });
ipcMain.handle('hb:suggest', (_e, text) => store.suggest(text));
// Hearth's own pages ask for their data (only hearth:// pages get this).
ipcMain.handle('hb:page', (e, m) => {
  if (!e.senderFrame || !String(e.senderFrame.url).startsWith('hearth://')) throw new Error('not allowed');
  switch (m && m.get) {
    case 'start': return { tiles: store.topSites(8).length >= 4 ? store.topSites(8) : START_TILES, bookmarks: store.bookmarks.slice(0, 12), search: SEARCH[store.settings.search].name };
    case 'history': return store.history(m.q || '');
    case 'bookmarks': return store.bookmarks;
    case 'downloads': return downloads.public();
    case 'settings': return { settings: store.settings, engines: Object.entries(SEARCH).map(([id, e]) => ({ id, name: e.name })), version: app.getVersion(), blocked: adblock.blocked };
    case 'set': store.set(m.key, m.value); if (m.key === 'adblock') adblock.set(!!m.value); return store.settings;
    case 'clearHistory': store.clearHistory(); return true;
    case 'clearData': return session.fromPartition('persist:hearth-browser').clearStorageData().then(() => session.fromPartition('persist:hearth-browser').clearCache()).then(() => true);
    case 'removeBookmark': store.removeBookmark(m.url); for (const w of windows) w.pushAll(); return store.bookmarks;
    case 'search': { const w = winOf(e.sender); if (w) w.navigate(m.text); return true; }
    case 'makeDefault': return new Promise(res => require('node:child_process').execFile('xdg-settings', ['set', 'default-web-browser', 'hearth-browser.desktop'], err => res(!err)));
  }
  return null;
});
