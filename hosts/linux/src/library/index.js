'use strict';
// Scans every store, merges Nimbus's own data (favorites, hidden, play time), launches games and watches them.
// Same messages and shapes as the Windows host's LibraryService, so the shared UI can't tell them apart.

const { EventEmitter } = require('node:events');
const { GameOptions, kindOf: optionsKind, protonChoices } = require('./game-options');
const crypto = require('node:crypto');
const steam = require('./steam');
const heroic = require('./heroic');
const xcloud = require('./xcloud');
const manual = require('./manual');
const proc = require('./proc');
const { Store } = require('./store');
const { StoreAccounts, norm } = require('../stores');
const { log } = require('../util');

const POLL_MS = 2000;
const START_TIMEOUT_MS = 3 * 60 * 1000; // Steam may update the game first

class Library extends EventEmitter {
  constructor({ store = new Store(), stream, accounts } = {}) {
    super();
    this.store = store;
    this.options = new GameOptions(store);
    // Epic / GOG / Xbox accounts and the installs Nimbus runs (stores/), same as Windows.
    this.accounts = accounts || new StoreAccounts(store);
    this.accounts.on('installFinished', (id, error) => {
      const g = this.games.get(id);
      this.emit('installDone', { id, title: g ? g.title : id, error });
      if (!error) this.scan().catch(() => {});
    });
    this.stream = stream;            // opens xCloud games in a window; set by main.js
    this.games = new Map();
    this.counts = {};
    this.running = null;             // {id, track, launchedAt, startedAt, child}
    this.timer = null;
    this.scanning = null;
    this.downloadSig = '';
  }

  // ------------------------------------------------------------------ scanning

  async get(rescan) {
    if (rescan) this.accounts.refreshStale();
    if (rescan || this.games.size === 0) await this.scan();
    return {
      games: [...this.games.values()].map(g => this.dto(g)).sort((a, b) => a.title.localeCompare(b.title, undefined, { sensitivity: 'base' })),
      sources: { ...this.store.data.sources },
      counts: this.counts,
      runningId: this.running && this.running.startedAt ? this.running.id : null,
      steamBackground: this.store.data.steamBackground,
    };
  }

  scan() {
    // Concurrent callers share one scan.
    if (!this.scanning) this.scanning = this._scan().finally(() => { this.scanning = null; });
    return this.scanning;
  }

  async _scan() {
    const on = k => this.store.sourceOn(k);
    const jobs = {
      steam: on('steam') ? () => steam.scan() : null,
      // Games Nimbus installed itself (legendary / gogdl), then any Heroic installed.
      epic: on('epic') ? () => [...this.accounts.installed('epic'), ...heroic.scanEpic()] : null,
      gog: on('gog') ? () => [...this.accounts.installed('gog'), ...heroic.scanGog()] : null,
      xbox: on('xbox') ? () => xcloud.scan() : null,
      manual: () => this.manualGames(),
    };
    const all = new Map();
    const counts = {};
    await Promise.all(Object.entries(jobs).map(async ([src, job]) => {
      let list = [];
      try { list = job ? await job() : []; } catch (e) { log(`scan ${src} failed: ${e.message}`); } // one broken store shouldn't break the library
      counts[src] = src === 'xbox' ? list.filter(g => g.id !== 'xbox:cloud').length : list.filter(g => g.installed).length;
      for (const g of list) if (!all.has(g.id)) all.set(g.id, g);
    }));

    // Games on your Epic and GOG accounts that aren't installed: shown with an Install button.
    for (const s of ['epic', 'gog']) {
      if (!on(s)) continue;
      const mine = [...all.values()].filter(g => g.source === s);
      for (const g of mine) this.accounts.addArt(g);
      for (const g of this.accounts.notInstalled(s, new Set(mine.map(g => g.id)))) if (!all.has(g.id)) all.set(g.id, g);
    }
    // Owned-but-not-installed Steam games come from Steam's binary caches (slower, so after the rest).
    if (on('steam')) {
      try {
        const have = new Set([...all.values()].filter(g => g.source === 'steam').map(g => g.id.slice(6)));
        for (const g of steam.scanOwned(have)) if (!all.has(g.id)) all.set(g.id, g);
      } catch (e) { log('steam owned: ' + e.message); }
      counts.steamOwned = [...all.values()].filter(g => g.source === 'steam').length;
    }
    // Cloud games you've played on Xbox (when signed in): when you last played them.
    const played = this.accounts.xboxPlayed();
    if (played.size) for (const g of all.values()) if (g.cloud && played.has(norm(g.title))) g.storeLastPlayed = played.get(norm(g.title));

    this.games = all;
    this.counts = counts;
    this.downloadSig = signature(all.values());
  }

  manualGames() {
    return this.store.data.manual.map(m => ({
      id: m.id, title: m.title, source: 'manual',
      installPath: m.path, added: m.added,
      exe: m.path, args: m.args || null,
      installed: true, state: 'installed', art: {},
    }));
  }

  dto(g) {
    const d = this.store.data;
    const stats = d.stats[g.id] || {};
    // Steam counts its own play time; for other stores Nimbus's numbers are the only ones.
    const art = {};
    for (const [k, v] of Object.entries(g.art || {})) {
      art[k] = /^https?:\/\//.test(v) ? v : `nimbus-art://art/${encodeURIComponent(g.id)}/${k}`;
    }
    return {
      id: g.id, title: g.title, source: g.source,
      fav: d.favorites.includes(g.id), hidden: d.hidden.includes(g.id),
      minutes: Math.max(g.storeMinutes || 0, stats.minutes || 0),
      lastPlayed: Math.max(g.storeLastPlayed || 0, stats.lastPlayed || 0),
      added: g.added || 0, sizeBytes: g.sizeBytes || null,
      art, path: g.installed ? (g.installPath || null) : null,
      canRemove: g.source === 'manual',
      installed: !!g.installed, state: g.state || 'installed', cloud: !!g.cloud,
      progress: g.progress ?? null, bytesDone: g.bytesDone ?? null, bytesTotal: g.bytesTotal ?? null,
      canUninstall: !!g.canUninstall, note: g.note || null,
      installKind: !g.installUri ? null : g.installUri.startsWith('nimbus-') ? 'nimbus' : g.installUri.startsWith('steam:') ? 'steam' : 'store',
      // An install Nimbus is running shows as a download.
      ...this.jobState(g.id),
    };
  }

  jobState(id) {
    const j = this.accounts.installs.get(id);
    return j ? { state: j.state, progress: j.progress, bytesDone: j.bytesDone, bytesTotal: j.bytesTotal, bytesPerSec: j.speed } : {};
  }

  /** Local file behind nimbus-art://art/{id}/{kind}, or null. */
  artFile(id, kind) {
    const g = this.games.get(id);
    const f = g && g.art && g.art[kind];
    return f && !/^https?:\/\//.test(f) ? f : null;
  }

  // ------------------------------------------------------------------ downloads (Steam)

  /** Cheap refresh of Steam's install records: what's downloading, and whether that set changed. */
  async downloads() {
    if (this.store.sourceOn('steam')) {
      const manifests = steam.scan();
      const fresh = new Set(manifests.map(m => m.id));
      for (const old of [...this.games.values()]) {
        // A cancelled Steam download removes its manifest: back to "not installed" (rescan restores it).
        if (old.source === 'steam' && !['installed', 'notInstalled'].includes(old.state) && !fresh.has(old.id)) this.games.delete(old.id);
      }
      for (const m of manifests) this.games.set(m.id, m);
    }
    const sig = this.signature();
    const changed = sig !== this.downloadSig;
    this.downloadSig = sig;
    const active = [...this.games.values()].filter(g => !['installed', 'notInstalled', 'cloud'].includes(g.state)).map(g => this.dto(g));
    for (const j of this.accounts.installs.active()) {
      const g = this.games.get(j.gameId);
      if (g && !active.some(a => a.id === g.id)) active.push(this.dto(g));
    }
    return { active, changed };
  }

  signature() {
    const jobs = this.accounts.installs.active().map(j => `${j.gameId}=${j.state === 'queued' ? 'queued' : 'active'}`);
    return [signature(this.games.values()), ...jobs].join(';');
  }

  async install(id) {
    const g = this.games.get(id);
    if (!g || !g.installUri) {
      throw new Error(g && g.note ? `${g.note}. Nimbus can't install it.` : "Nimbus can't install this game. Install it from its store app.");
    }
    if (g.installUri.startsWith('nimbus-')) { this.accounts.startInstall(g); return; } // Epic / GOG: Nimbus downloads it itself
    if (g.source === 'steam' && this.store.data.steamBackground) await steam.ensureRunningSilent();
    steam.openUri(g.installUri);
  }

  cancelInstall(id) { return this.accounts.installs.cancel(id); }

  async uninstall(id) {
    const g = this.games.get(id);
    if (!g) throw new Error("That game isn't in the library anymore.");
    if (this.running && this.running.id === id) throw new Error(`Exit ${g.title} first.`);
    await this.accounts.uninstall(g);
    await this.scan();
  }

  // ------------------------------------------------------------------ edits

  setFavorite(id, on) { this.store.setIn('favorites', id, on); }
  setHidden(id, on) { this.store.setIn('hidden', id, on); }
  async setSources(src) { this.store.update(d => Object.assign(d.sources, src)); await this.scan(); }
  async setSteamBackground(on) { this.store.update(d => { d.steamBackground = on; }); if (on) await steam.ensureRunningSilent(); }

  async addManual(file) {
    const m = { id: 'manual:' + crypto.randomBytes(6).toString('hex'), title: manual.titleFor(file), path: file, added: Date.now() };
    this.store.update(d => d.manual.push(m));
    await this.scan();
    return this.dto(this.games.get(m.id));
  }

  async removeManual(id) {
    this.store.update(d => {
      d.manual = d.manual.filter(m => m.id !== id);
      d.favorites = d.favorites.filter(x => x !== id);
      d.hidden = d.hidden.filter(x => x !== id);
      delete d.stats[id];
    });
    await this.scan();
  }

  async startBackgroundServices() {
    this.accounts.refreshStale(); // Epic / GOG / Xbox game lists, if they're old
    if (this.store.data.steamBackground && this.store.sourceOn('steam')) await steam.ensureRunningSilent();
  }

  // ------------------------------------------------------------------ launching and watching

  /** Starts the game. Resolves {tracked:false} when Nimbus can't watch it (it then won't show as running). */
  async launch(id) {
    const g = this.games.get(id);
    if (!g) throw new Error("That game isn't in the library anymore. Try rescanning.");
    if (!g.installed && !g.cloud) throw new Error(`${g.title} isn't installed yet.`);
    if (this.running) throw new Error('Another game is already running');

    if (g.cloud) {
      if (!this.stream) throw new Error('Streaming is not available');
      this.running = { id, cloud: true, launchedAt: Date.now() };
      this.stream.open(g.streamUrl, g.title);
      this.markStarted();
      return { tracked: true };
    }

    let track = g.track || null;
    let child = null;
    if (g.source === 'steam') {
      if (this.store.data.steamBackground) await steam.ensureRunningSilent();
      steam.openUri(g.launchUri);
    } else if (g.launchVia === 'legendary' || g.windowsExe) {
      // Installed by Nimbus: started directly with Proton, so Nimbus can watch it.
      child = await this.accounts.launch(g, this.options.get(id));
      track = { ...(g.track || {}), pid: child && child.pid };
    } else if (g.source === 'epic' || g.source === 'gog') {
      heroic.openUri(g.launchUri, g.launchVia);
    } else if (g.source === 'manual') {
      child = manual.launch(g, this.options.get(id));
      // Watch the folder too (games that hand off to another process), but never a folder so broad
      // that unrelated programs would count as "the game is running".
      const dir = require('node:path').dirname(g.exe);
      const broad = ['/', '/usr/bin', '/usr/local/bin', '/bin', '/opt', require('node:os').homedir()].includes(dir);
      track = { pid: child && child.pid, dirs: g.exe.endsWith('.desktop') || broad ? [] : [dir] };
    } else throw new Error("Nimbus doesn't know how to start this game.");

    if (!track) return { tracked: false };
    this.running = { id, track, launchedAt: Date.now(), startedAt: 0, child };
    this.poll();
    return { tracked: true };
  }

  poll() {
    clearTimeout(this.timer);
    const r = this.running;
    if (!r || r.cloud) return;
    const pids = proc.gameProcesses(r.track);
    if (!r.startedAt) {
      if (pids.length) this.markStarted();
      else if (Date.now() - r.launchedAt > START_TIMEOUT_MS) {
        this.running = null;
        log(`game start timed out ${r.id}`);
        this.emit('timeout', r.id);
        return;
      }
    } else if (!pids.length) return this.markExited();
    this.timer = setTimeout(() => this.poll(), POLL_MS);
  }

  markStarted() {
    const r = this.running;
    if (!r || r.startedAt) return;
    r.startedAt = Date.now();
    log(`game started ${r.id}`);
    this.emit('started', r.id);
  }

  /** The game is gone: record play time and tell the UI. */
  markExited() {
    const r = this.running;
    if (!r) return;
    this.running = null;
    clearTimeout(this.timer);
    if (r.startedAt) {
      const minutes = Math.round((Date.now() - r.startedAt) / 60000);
      this.store.update(d => {
        const s = d.stats[r.id] || (d.stats[r.id] = { minutes: 0, lastPlayed: 0 });
        s.minutes += minutes;
        s.lastPlayed = Date.now();
      });
    }
    log(`game exited ${r.id}`);
    const g = this.games.get(r.id);
    this.emit('exited', g ? this.dto(g) : { id: r.id });
  }

  async closeRunning() {
    const r = this.running;
    if (!r) return true;
    if (r.cloud) { this.stream && this.stream.close(); return true; }
    const ok = await proc.stop(proc.gameProcesses(r.track));
    this.poll();
    return ok;
  }

  /** Game settings page: what this game can set, and what it has. */
  gameOptions(id) {
    const g = this.games.get(id);
    if (!g) throw new Error("That game isn't in the library anymore.");
    const kind = optionsKind(g);
    return { id, kind, options: this.options.get(id), protonChoices: kind === 'proton' ? protonChoices() : [],
      gamemode: require('../util').has('gamemoderun'), canResetPrefix: kind === 'proton' && (g.launchVia === 'legendary' || !!g.windowsExe) };
  }
  setGameOptions(id, patch) { this.options.set(id, patch || {}); return this.gameOptions(id); }
  /** Deletes the game's Wine prefix, so Proton sets it up fresh next time (fixes broken setups; local saves in it go too). */
  resetPrefix(id) {
    const g = this.games.get(id);
    if (!g || !(g.launchVia === 'legendary' || g.windowsExe)) throw new Error("This game doesn't have a Hearth Wine prefix.");
    if (this.runningId === id) throw new Error('Close the game first.');
    const proton = require('../stores/proton');
    if (g.launchVia === 'legendary') proton.removePrefix('epic', g.id.slice(5));
    else proton.removePrefix('gog', g.gogId);
    return true;
  }

  /** Process ids of the running game (for bringing its window to the front). */
  gamePids() {
    const r = this.running;
    if (!r || !r.track) return [];
    try { return proc.gameProcesses(r.track).map(p => (typeof p === 'object' ? p.pid : p)); } catch { return []; }
  }

  get runningId() { return this.running && this.running.startedAt ? this.running.id : null; }
  get runningSince() { return this.running && this.running.startedAt ? this.running.startedAt : null; }

  dispose() { clearTimeout(this.timer); }
}

function signature(games) {
  return [...games].filter(g => !['installed', 'notInstalled', 'cloud'].includes(g.state))
    .sort((a, b) => (a.id < b.id ? -1 : 1)).map(g => `${g.id}=${g.state}`).join(';');
}

module.exports = { Library };
