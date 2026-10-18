'use strict';
// Request/response bridge — the same contract as the Windows host's SystemBridge (see docs/bridge.md).
//   UI sends   {type:"call", id, method, args}
//   Host sends {type:"result", id, ok, value | error}   and   {type:"event", name, data} for pushes.
// Platform pieces that need Electron (dialogs, windows, quitting) come in through `platform`.

const { Wifi, Bluetooth, Audio, Display, Power, setAirplane, airplaneOn } = require('./system');
const { Profile, steamAccount } = require('./accounts');
const perf = require('./perf');
const steam = require('./library/steam');
const { DisplayModes } = require('./display');
const { SystemAdmin } = require('./system-admin');
const { Importer } = require('./importer');
const { ButtonPrefs, Controller } = require('./buttons');
const { Browser } = require('./browser');
const sysinfo = require('./sysinfo');
const { log } = require('./util');

class Bridge {
  /**
   * @param {object} o
   * @param {(msg:object)=>void} o.post          sends a message to the UI
   * @param {import('./library').Library} o.library
   * @param {import('./updates').Updates} o.updates
   * @param {object} o.platform  {info, pickGameFile, pickImage, openXboxSignIn, screenInfo, session:{status,register,unregister}, installUpdate, returnToDesktop}
   */
  constructor({ post, library, updates, platform }) {
    this.post = post;
    this.library = library;
    this.updates = updates;
    this.platform = platform;
    this.wifi = new Wifi();
    this.bt = new Bluetooth();
    this.audio = new Audio();
    this.display = new Display();
    this.power = new Power();
    this.profile = new Profile(library.store, library.accounts);
    this.admin = platform.admin || new SystemAdmin();
    this.prefs = platform.buttonPrefs || new ButtonPrefs();
    this.controller = platform.controller || null;
    this.browser = platform.browser || new Browser();
    this.browser.on('changed', s => this.event('browser', s));
    this.admin.on('update', s => this.event('systemUpdate', s));
    // One importer for the app (the overlay has its own bridge): kept on the library.
    this.importer = platform.importer || (library.importer ||= new Importer({ admin: this.admin, library }));
    this.importer.on('changed', s => this.event('import', s));
    this.importer.on('libraryChanged', () => this.event('library.changed', {}));
    // Account changes come in bursts (sign-in, list refresh): tell the UI once they settle.
    library.accounts.on('changed', () => { clearTimeout(this.accTimer); this.accTimer = setTimeout(() => this.event('accounts.changed', {}), 400); });
    library.on('installDone', d => this.event('install.done', d));
    this.displayModes = platform.displayModes || new DisplayModes({
      screen: () => ({ w: 0, h: 0, rate: 60 }), getZoom: () => 100, setZoom() {}, restartGameMode() {},
    });

    this.bt.on('found', d => this.event('bt.found', d));
    this.bt.on('scanDone', () => this.event('bt.scanDone', {}));
    library.on('started', id => this.event('game.started', { id }));
    library.on('timeout', id => this.event('game.timeout', { id }));
    library.on('exited', dto => this.event('game.exited', dto));
    updates.on('changed', s => this.event('update', s));

    // Battery and network change on their own; push them every 30 s like the Windows host.
    this.statusTimer = setInterval(() => this.pushStatus(), 30000);
  }

  event(name, data) { this.post({ type: 'event', name, data }); }
  async pushStatus() { try { this.event('status', await this.status()); } catch { /* best effort */ } }

  async handle(m) {
    const started = Date.now();
    try {
      const value = await this.dispatch(m.method, m.args || {});
      log(`${m.method} ok ${Date.now() - started}ms`);
      this.post({ type: 'result', id: m.id, ok: true, value: value === undefined ? null : value });
    } catch (e) {
      log(`${m.method} FAILED ${Date.now() - started}ms ${e.message}`);
      this.post({ type: 'result', id: m.id, ok: false, error: e.message });
    }
  }

  async dispatch(method, a) {
    const req = k => { if (typeof a[k] !== 'string' || !a[k]) throw new Error(`Missing '${k}'`); return a[k]; };
    const L = this.library;
    switch (method) {
      case 'host.info': return this.platform.info();
      case 'sys.status': return this.status();
      case 'system.info': return sysinfo.info();
      case 'app.restart': return this.platform.restartApp ? this.platform.restartApp() : null;
      case 'app.exportLogs': return this.platform.exportLogs ? this.platform.exportLogs() : null;

      case 'radios.set':
        if (a.kind === 'wifi') await this.wifi.setRadio(!!a.on); else await this.bt.setPower(!!a.on);
        return this.status();
      case 'airplane.set': await setAirplane(!!a.on, this.wifi, this.bt); if (a.on) this.bt.stopScan(); return this.status();

      case 'wifi.scan': return this.wifi.scan();
      case 'wifi.connect': { const r = await this.wifi.connect(req('ssid'), a.password || null); this.pushStatus(); return r; }
      case 'wifi.disconnect': await this.wifi.disconnect(); this.pushStatus(); return null;
      case 'wifi.forget': await this.wifi.forget(req('ssid')); this.pushStatus(); return null;

      case 'bt.paired': return this.bt.paired();
      case 'bt.scanStart':
        if (!(await this.bt.powered())) throw new Error('Bluetooth is off');
        this.bt.startScan(a.seconds || 15); return null;
      case 'bt.scanStop': this.bt.stopScan(); return null;
      case 'bt.pair': return this.bt.pair(req('id'));
      case 'bt.confirmPin': return null; // bluetoothctl's default agent confirms on its own
      case 'bt.unpair': return this.bt.unpair(req('id'));

      case 'audio.get': return this.audio.get();
      case 'audio.setVolume': await this.audio.setVolume(Math.round(+a.v || 0), this.prefs.maxVolume()); return null;

      // Power and volume buttons (Settings › Power › Power button, Settings › Audio › Over-amplification)
      case 'buttons.get': return this.buttonsInfo();
      case 'buttons.set': {
        const before = this.prefs.get();
        const now = this.prefs.set(a);
        if (before.overamp && !now.overamp && this.audio.available) {
          const v = await this.audio.volume().catch(() => null);
          if (v && v.volume > 100) await this.audio.setVolume(100);
        }
        return this.buttonsInfo();
      }

      // Game art kept on the device (Settings › System)
      case 'art.status': return require('./art-cache').artCache().status();

      // Web browser (Aloha), Settings › System
      case 'browser.status': return this.browser.status();
      case 'browser.act': this.browser.act(req('action')); return this.browser.status();

      // Controller remapping through InputPlumber (Settings › Controller)
      case 'controller.status': return this.ctl().status();
      case 'controller.set': return this.ctl().set({ button: a.button, to: a.to, target: a.target });
      case 'controller.preset': return this.ctl().preset(req('name'));
      case 'controller.reset': return this.ctl().reset();
      case 'audio.setMute': await this.audio.setMute(!!a.m); return null;
      case 'audio.setDefault': await this.audio.setDefault(req('id')); return this.audio.get();

      case 'display.get': {
        const [b, modes] = await Promise.all([this.display.brightness().catch(() => null), this.displayModes.get()]);
        return { supported: b != null, brightness: b ?? 0, ...this.platform.screenInfo(), ...modes };
      }
      // Same arguments as Windows: {w, h, hz} and {v} (percent).
      case 'display.setMode': {
        const w = Math.round(+a.w), h = Math.round(+a.h), hz = +a.hz;
        if (!(w > 0 && h > 0 && hz > 0)) throw new Error('Pick a resolution and refresh rate');
        await this.displayModes.setMode(w, h, hz);
        return this.dispatch('display.get', {});
      }
      case 'display.setScale': await this.displayModes.setScale(Math.round(+a.v)); return this.dispatch('display.get', {});
      case 'display.setBrightness': await this.display.setBrightness(Math.round(+a.v || 0)); return null;

      case 'power.get': return this.power.get();
      case 'power.setMode': await this.power.setMode(req('mode')); return this.power.get();

      case 'library.get': return L.get(!!a.rescan);
      case 'library.setFav': L.setFavorite(req('id'), !!a.on); return null;
      case 'library.setHidden': L.setHidden(req('id'), !!a.on); return null;
      case 'library.setSources': {
        const src = {};
        for (const k of ['steam', 'epic', 'gog', 'xbox']) if (k in a) src[k] = a[k] === true;
        await L.setSources(src);
        return L.get(false);
      }
      case 'library.addManual': {
        const file = await this.platform.pickGameFile();
        return file ? L.addManual(file) : null;
      }
      case 'library.remove': await L.removeManual(req('id')); return null;
      case 'library.downloads': return L.downloads();
      case 'library.setSteamBackground': await L.setSteamBackground(!!a.on); return null;
      case 'game.launch': return L.launch(req('id'));
      case 'game.install': await L.install(req('id')); return null;
      case 'game.close': return { ok: await L.closeRunning() };
      // Installs and uninstalls happen in Steam and Heroic on Linux; their games never offer these.
      case 'game.options.get': return L.gameOptions(req('id'));
      case 'game.options.set': return L.setGameOptions(req('id'), a.options || {});
      case 'game.resetPrefix': return { ok: L.resetPrefix(req('id')) };
      case 'game.cancelInstall': return { ok: L.cancelInstall(req('id')) };
      case 'game.pauseInstall': return { ok: L.pauseInstall(req('id')) };
      case 'game.resumeInstall': return { ok: L.resumeInstall(req('id')) };
      case 'game.installFirst': return { ok: L.firstInQueue(req('id')) };
      case 'downloads.clearRecent': L.clearRecent(); return { ok: true };
      case 'steam.downloads': if (this.platform.showSteam) this.platform.showSteam(); steam.openUri('steam://open/downloads'); return { ok: true };
      case 'game.uninstall': await L.uninstall(req('id')); return L.get(false);

      case 'accounts.status': {
        const counts = {};
        for (const g of L.games.values()) if (g.installed) counts[g.source] = (counts[g.source] || 0) + 1;
        const st = steamAccount();
        return {
          accounts: L.accounts.status(counts, { name: st.signedIn ? (st.name || 'Signed in') : null, avatar: st.avatarFile ? 'nimbus-art://profile/steam' : null,
            owned: [...L.games.values()].filter(g => g.source === 'steam').length, missing: st.installed === false, note: st.signedIn ? null : st.note || null }),
          installRoot: L.accounts.installRoot,
        };
      }
      case 'accounts.signIn': {
        const store = req('store');
        if (store === 'steam') { steam.openUri('steam://open/main'); return { ok: true }; }
        return { ok: await L.accounts.signIn(store, this.platform.signInWindow) };
      }
      case 'accounts.signOut': await L.accounts.signOut(req('store')); return null;
      case 'accounts.refresh': await L.accounts.refresh(req('store')); await L.scan(); return null;
      case 'accounts.pickFolder': {
        const dir = await this.platform.pickFolder(L.accounts.installRoot);
        if (dir) L.accounts.installRoot = dir;
        return { installRoot: L.accounts.installRoot };
      }

      case 'perf.stats':
        return { ...(await perf.sample()), battery: this.power.battery(), runningId: L.runningId, runningSince: L.runningSince,
          canFpsLimit: !!(this.platform.canFpsLimit && this.platform.canFpsLimit()), fpsLimit: L.store && L.store.data ? (L.store.data.fpsLimit || 0) : 0,
          ...(await require('./power-control').status().then(s => ({ powerMode: s.mode.supported ? s.mode.mode : null, powerModes: s.mode.available, tdp: s.tdp })).catch(() => ({ powerMode: 'balanced', tdp: { supported: false } }))) };
      case 'perf.setTdp': return require('./power-control').setTdp(+a.watts);
      case 'apps.list': return this.platform.apps && this.platform.apps.supported() ? this.platform.apps.list() : [];
      case 'apps.focus': return this.platform.apps.focus(+a.id, +a.pid);
      case 'apps.close': return { ok: await this.platform.apps.close(+a.pid) };
      case 'perf.setFpsLimit': return { fpsLimit: await this.platform.setFpsLimit(+a.fps || 0) };
      // In Game Mode Quick Access opens in an overlay window over the game; elsewhere Hearth comes back over it.
      case 'overlay.settings': return this.platform.overlay.settings();
      case 'overlay.setHoldView': this.platform.overlay.setHoldView(!!a.on); return this.platform.overlay.settings();
      case 'overlay.setGameBarButton': return this.platform.overlay.settings(); // Windows only
      case 'overlay.screenshot': return this.platform.screenshot();

      case 'profile.get': return this.profile.get();
      case 'profile.set': return this.profile.set(a);
      case 'profile.pickImage': {
        const kind = req('kind');
        const file = await this.platform.pickImage(kind);
        return file ? this.profile.setImage(kind, file) : null;
      }

      // Settings › Sign-in (Linux)
      case 'login.get': return this.admin.login();
      case 'login.setAutologin': return this.admin.setAutologin(!!a.on);
      case 'login.setStartMode': return this.admin.setStartMode(req('mode'));
      case 'login.setHostname': return this.admin.setHostname(req('name'));
      case 'login.password': return this.admin.changePassword(req('old'), req('new'));
      // Settings › Updates › System (Hearth, the OS)
      case 'import.status': return this.importer.status();
      case 'import.scan': return this.importer.scan();
      case 'import.start': return this.importer.start(a.items, a.mode || 'copy');
      case 'import.cancel': return this.importer.cancel();
      case 'takeover.plan': return this.importer.takeoverPlan();
      case 'takeover.run': return this.importer.takeoverRun(a.remove, a.windowsBoot !== false);
      case 'system.update.status': return this.admin.refreshUpdate();
      case 'system.update.check': this.admin.checkUpdate(); return this.admin.updateStatus();
      case 'system.update.install': this.admin.installUpdate(); return this.admin.updateStatus();

      case 'update.status': return this.updates.status();
      case 'update.check': return this.updates.check();
      case 'update.download': this.updates.download(); return this.updates.status();
      case 'update.install': setTimeout(() => this.platform.installUpdate(), 50); return null;
      case 'update.setAuto': L.store.update(d => { d.autoUpdateCheck = !!a.on; }); return this.updates.status();

      // The UI reads controllers itself (Gamepad API); there is no host-side controller mouse on Linux yet.
      case 'pad.status': return { connected: 0, mouseMode: 'off', mouseOn: false };
      case 'pad.setMouseMode': throw new Error('Controller mouse is not available on Linux yet');

      case 'shell.status': return this.platform.session.status();
      case 'shell.register': return this.platform.session.register();
      case 'shell.unregister': return this.platform.session.unregister();
      case 'shell.returnToDesktop': setTimeout(() => this.platform.returnToDesktop(), 50); return null;

      default: throw new Error(`Unknown method: ${method}`);
    }
  }

  buttonsInfo() {
    const b = this.platform.buttons;
    return { ...this.prefs.get(), maxVolume: this.prefs.maxVolume(), watching: b ? b.status() : { devices: [], problems: ['Only in Game Mode'] } };
  }
  ctl() {
    if (!this.controller) throw new Error('Controller remapping needs Hearth, the OS.');
    return this.controller;
  }

  async status() {
    const safe = (p, fallback = null) => Promise.resolve().then(p).catch(() => fallback);
    const [wifiOn, btOn, airplane, audio, brightness, powerMode] = await Promise.all([
      safe(() => this.wifi.radioOn()), safe(() => this.bt.powered()), safe(() => airplaneOn(), false),
      safe(() => (this.audio.available ? this.audio.volume() : null)), safe(() => this.display.brightness()),
      safe(() => this.power.mode(), 'balanced'),
    ]);
    const cur = wifiOn ? await safe(() => this.wifi.current(), { ssid: null, bars: 0 }) : { ssid: null, bars: 0 };
    return {
      wifi: { available: wifiOn != null, on: wifiOn === true, ssid: cur.ssid, bars: cur.bars },
      bt: { available: btOn != null, on: btOn === true },
      airplane: !!airplane,
      battery: this.power.battery(),
      audio: audio ? { ...audio, max: this.prefs.maxVolume() } : audio,
      display: { supported: brightness != null, brightness: brightness ?? 0 },
      powerMode,
    };
  }

  dispose() { clearInterval(this.statusTimer); this.bt.stopScan(); this.library.dispose(); }
}

module.exports = { Bridge };
