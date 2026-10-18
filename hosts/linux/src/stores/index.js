'use strict';
// The store accounts you sign into from Library › Stores (Epic, GOG, Xbox; Steam signs in through Steam itself),
// their game lists, and the Epic/GOG installs Nimbus runs. Port of the Windows host's Stores/StoreAccounts.cs,
// so Linux works the same way: no Epic launcher, no GOG Galaxy, no Heroic needed.

const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const { EventEmitter } = require('node:events');
const { EpicStore } = require('./epic');
const { GogStore } = require('./gog');
const { XboxStore } = require('./xbox');
const { InstallQueue } = require('./installs');
const tools = require('./tools');
const proton = require('./proton');
const { log } = require('../util');

const STALE_MS = 12 * 3600e3;

class StoreAccounts extends EventEmitter {
  /** @param {import('../library/store').Store} store  Nimbus's library.json */
  constructor(store) {
    super();
    this.store = store;
    this.epic = new EpicStore();
    this.gog = new GogStore();
    this.xbox = new XboxStore();
    this.installs = new InstallQueue();
    this.errors = {};
    this.busy = new Set();
    this.installs.on('finished', (id, error) => this.emit('installFinished', id, error));
    // The download queue is kept in library.json, so a restart (or Hearth closing) doesn't lose it.
    this.installs.on('changed', () => this.saveQueue());
  }

  static get defaultRoot() { return path.join(os.homedir(), 'Games'); }
  get installRoot() { return this.store.data.installRoot || StoreAccounts.defaultRoot; }
  set installRoot(v) { this.store.update(d => { d.installRoot = v; }); }

  of(s) { return { epic: this.epic, gog: this.gog, xbox: this.xbox }[s] || null; }
  signedIn(s) { const st = this.of(s); return !!st && st.signedIn; }

  // ------------------------------------------------------------------ status

  /** What the Stores page shows. installed: counts per store; steam: {name, avatar, owned, missing}. */
  status(installed, steam) {
    const inst = s => installed[s] || 0;
    const base = s => ({ store: s, installed: inst(s), busy: this.busy.has(s), error: this.errors[s] || null });
    const gog = this.gog.profile();
    return [
      { ...base('steam'), signedIn: !!steam.name, name: steam.name, avatar: steam.avatar, owned: steam.owned, fetched: 0,
        toolReady: true, note: steam.missing ? "Steam isn't installed on this device." : steam.note || null },
      { ...base('epic'), signedIn: this.epic.signedIn, name: this.epic.displayName(), avatar: null, owned: this.epic.owned.games.length,
        fetched: this.epic.owned.fetched, toolReady: tools.isReady(tools.TOOLS.legendary), note: null },
      { ...base('gog'), signedIn: this.gog.signedIn, name: gog.name, avatar: gog.avatar, owned: this.gog.owned.games.length,
        fetched: this.gog.owned.fetched, toolReady: tools.isReady(tools.TOOLS.gogdl), note: null },
      { ...base('xbox'), signedIn: this.xbox.signedIn, name: this.xbox.gamertag, avatar: this.xbox.gamerpic, owned: this.xbox.owned.games.length,
        fetched: this.xbox.owned.fetched, toolReady: true, note: null },
    ];
  }

  // ------------------------------------------------------------------ sign in / out / refresh

  async signIn(s, signInWindow) {
    const st = this.of(s);
    if (!st) throw new Error('Unknown store');
    this.errors[s] = null;
    const ok = await st.signIn(signInWindow);
    if (!ok) return false;
    log(`accounts: signed in to ${s}`);
    this.emit('changed');
    this.refresh(s).catch(() => {}); // fetch the game list in the background
    return true;
  }

  async signOut(s) {
    const st = this.of(s);
    if (!st) throw new Error('Unknown store');
    await st.signOut();
    this.errors[s] = null;
    log(`accounts: signed out of ${s}`);
    this.emit('changed');
  }

  /** Downloads the store's game list again. Errors are kept and shown on the Stores page. */
  async refresh(s) {
    if (!this.signedIn(s) || this.busy.has(s)) return;
    this.busy.add(s);
    this.emit('changed');
    try {
      await this.of(s).refreshOwned();
      this.errors[s] = null;
      log(`accounts: ${s} list has ${this.of(s).owned.games.length} games`);
    } catch (e) {
      this.errors[s] = e.message;
      log(`accounts: ${s} refresh FAILED ${e.message}`);
    } finally {
      this.busy.delete(s);
      this.emit('changed');
    }
  }

  /** At startup and on rescans: refresh lists older than 12 hours, in the background. */
  refreshStale() {
    for (const s of ['epic', 'gog', 'xbox']) {
      if (this.signedIn(s) && Date.now() - this.of(s).owned.fetched > STALE_MS) this.refresh(s).catch(() => {});
    }
  }

  // ------------------------------------------------------------------ games for the library

  /** Epic games installed by legendary and GOG games installed by gogdl. */
  installed(s) {
    if (s === 'epic') return this.epic.installed();
    if (s === 'gog') return this.gog.installed(this.store.data.gogInstalls || {});
    return [];
  }

  /** Games on the account that aren't installed (the library shows them with an Install button). */
  notInstalled(s, installedIds) {
    if ((s !== 'epic' && s !== 'gog') || !this.signedIn(s)) return [];
    return this.of(s).owned.games.filter(o => !installedIds.has(`${s}:${o.id}`)).map(o => ({
      id: `${s}:${o.id}`, title: o.title, source: s, added: o.added || 0, storeLastPlayed: o.lastPlayed || 0,
      installed: false, state: 'notInstalled', note: o.note || null,
      installUri: o.note ? null : `nimbus-${s}:${o.id}`, art: { ...o.art },
    }));
  }

  /** Art from the account's list, for games found installed some other way (Heroic). */
  addArt(g) {
    const st = this.of(g.source);
    if (!st || Object.keys(g.art || {}).length) return;
    const o = st.owned.games.find(x => x.id === g.id.slice(g.source.length + 1));
    if (o) g.art = { ...o.art };
  }

  /** Played-on-Xbox info for the cloud catalog, matched by title: { normalizedTitle: lastPlayed }. */
  xboxPlayed() {
    const map = new Map();
    for (const o of this.xbox.signedIn ? this.xbox.owned.games : []) map.set(norm(o.title), o.lastPlayed || 0);
    return map;
  }

  // ------------------------------------------------------------------ installs

  startInstall(g, { paused = false } = {}) {
    const uri = g.installUri || '';
    const root = this.installRoot;
    if (uri.startsWith('nimbus-epic:')) {
      const app = uri.slice('nimbus-epic:'.length);
      if (!this.epic.signedIn) throw new Error('Sign in to Epic Games first (Library › Stores).');
      return this.installs.enqueue({ gameId: g.id, title: g.title, uri, work: (job, signal) => this.epic.install(job, app, root, signal) }, { paused });
    }
    if (uri.startsWith('nimbus-gog:')) {
      const id = uri.slice('nimbus-gog:'.length);
      if (!this.gog.signedIn) throw new Error('Sign in to GOG first (Library › Stores).');
      return this.installs.enqueue({
        gameId: g.id, title: g.title, uri,
        work: async (job, signal) => {
          const dir = await this.gog.install(job, id, root, signal);
          this.store.update(d => { d.gogInstalls = { ...(d.gogInstalls || {}), [id]: dir }; });
        },
      }, { paused });
    }
    throw new Error("Nimbus can't install this game itself.");
  }

  saveQueue() {
    const q = this.installs.active().map(j => ({ gameId: j.gameId, title: j.title, uri: j.uri, paused: !!j.paused,
      bytesDone: j.bytesDone || null, bytesTotal: j.bytesTotal || null, progress: j.progress ?? null }));
    this.store.update(d => { d.installQueue = q; });
  }

  /** After a restart: puts the saved downloads back in the queue (legendary and gogdl carry on from the files
   *  already downloaded). find(id) → the library's game. Paused ones stay paused. */
  restoreQueue(find) {
    const saved = (this.store.data.installQueue || []).slice();
    for (const e of saved) {
      if (this.installs.get(e.gameId)) continue;
      const g = find(e.gameId);
      if (!g || g.installed) continue;
      try {
        const job = this.startInstall({ ...g, installUri: g.installUri || e.uri }, { paused: !!e.paused });
        if (job) { job.bytesDone = e.bytesDone; job.bytesTotal = e.bytesTotal; job.progress = e.progress; }
        log(`install ${e.gameId}: back in the queue after a restart${e.paused ? ' (paused)' : ''}`);
      } catch (err) { log(`install ${e.gameId}: couldn't restore: ${err.message}`); }
    }
  }

  async uninstall(g) {
    if (!g.canUninstall) throw new Error(`Uninstall ${g.title} from its store app.`);
    if (g.source === 'epic') return this.epic.uninstall(g.id.slice(5));
    if (g.source === 'gog') {
      const id = g.id.slice(4);
      const dir = (this.store.data.gogInstalls || {})[id];
      // Only ever delete a folder that is clearly this game's (it holds GOG's goggame-<id>.info).
      if (dir && fs.existsSync(path.join(dir, `goggame-${id}.info`))) await fs.promises.rm(dir, { recursive: true, force: true });
      this.store.update(d => { if (d.gogInstalls) delete d.gogInstalls[id]; });
      proton.removePrefix('gog', id);
      return;
    }
    throw new Error(`Uninstall ${g.title} from its store app.`);
  }

  /** Starts an Epic or GOG game Nimbus installed. Resolves with the child process. */
  launch(g, options = null) {
    if (g.launchVia === 'legendary') return this.epic.launch(g.id.slice(5), options);
    if (g.source === 'gog' && g.windowsExe) return this.gog.launch(g, options);
    throw new Error("Nimbus doesn't know how to start this game.");
  }
}

const norm = s => String(s || '').toLowerCase().replace(/[^a-z0-9]+/g, '');

module.exports = { StoreAccounts, norm };
