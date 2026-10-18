'use strict';
// Settings › Import games: bring games and personal files over from this PC's other drives (Windows,
// SteamOS / Steam Deck, Bazzite and other Linux, second drives, SD cards).
//
// The work on the drives is done by hearth-import (as root through hearth-admin): finding what's there,
// mounting drives read-only, copying. This side keeps the state for the page (scan, progress with speed,
// result), and afterwards registers what was copied: GOG games in Hearth's GOG list, Epic games with
// legendary (right away if you're signed in to Epic, otherwise as soon as you sign in), and Steam is
// restarted so it finds the new games.

const { EventEmitter } = require('node:events');
const { log } = require('./util');
const steam = require('./library/steam');

class Importer extends EventEmitter {
  constructor({ admin, library }) {
    super();
    this.admin = admin;
    this.library = library;
    this.state = { state: 'idle', scan: null, scannedAt: 0, progress: null, speeds: [], result: null, error: null };
    this.proc = null;
    // Epic games waiting for an Epic sign-in to finish importing.
    library.accounts.on('changed', () => this.finishPendingEpic().catch(e => log('import: epic: ' + e.message)));
    // First start after installing (often next to Windows, from the live USB's "Install next to Windows"):
    // look once for games on other drives and offer to bring them over.
    this.offerTimer = setTimeout(() => this.offerOnce().catch(e => log('import: offer: ' + e.message)), 90000);
    if (this.offerTimer.unref) this.offerTimer.unref();
  }

  async offerOnce() {
    if (!this.supported || this.store.data.importOffered) return;
    this.store.update(d => { d.importOffered = Date.now(); });
    await this.scan();
    const drives = (this.state.scan && this.state.scan.drives) || [];
    const games = drives.reduce((n, d) => n + d.steam.length + d.epic.length + d.gog.length + (d.lutris || []).length, 0);
    if (games > 0) {
      const from = drives.filter(d => d.steam.length + d.epic.length + d.gog.length + (d.lutris || []).length).map(d => d.name);
      this.set({ offer: { games, from } });
      log(`import: offering ${games} games from ${from.join(', ')}`);
    }
  }

  get supported() { return this.admin.canAdmin; }
  status() { return { supported: this.supported, pendingEpic: (this.store.data.pendingEpicImports || []).length, ...this.state }; }
  get store() { return this.library.store; }

  set(patch, force = true) {
    Object.assign(this.state, patch);
    const now = Date.now();
    if (!force && now - (this.lastPush || 0) < 500) return;
    this.lastPush = now;
    this.emit('changed', this.status());
  }

  async scan() {
    if (!this.supported) throw new Error('Importing needs Hearth, the OS.');
    if (this.state.state !== 'idle') return this.status();
    this.set({ state: 'scanning', error: null, result: null });
    try {
      const scan = await this.admin.run(['import-scan'], { timeout: 5 * 60000 });
      this.set({ state: 'idle', scan, scannedAt: Date.now() });
    } catch (e) {
      this.set({ state: 'idle', error: e.message });
    }
    return this.status();
  }

  /** keys: items from the scan. mode: 'copy' or 'move' (originals deleted after a good copy). */
  start(keys, mode = 'copy') {
    if (this.state.state !== 'idle') throw new Error('Already busy');
    if (!Array.isArray(keys) || !keys.length) throw new Error('Pick something to import first');
    if (!['copy', 'move'].includes(mode)) throw new Error('Copy or move');
    const root = this.library.accounts.installRoot;
    this.set({ state: 'importing', error: null, result: null, speeds: [], started: Date.now(),
      progress: { overall: 0, total: 0, i: 0, n: keys.length, name: '', done: 0, size: 0, bps: 0 }, mode });
    const onStage = st => {
      if (st.phase === 'start') return this.set({ progress: { ...this.state.progress, total: st.total, n: st.n } });
      if (st.phase === 'done') return this.set({ progress: { ...this.state.progress, overall: st.total } });
      const speeds = st.bps ? [...this.state.speeds.filter(([t]) => Date.now() - t < 5 * 60e3), [Date.now(), st.bps]].slice(-400) : this.state.speeds;
      this.set({ progress: { ...this.state.progress, ...st }, speeds }, false);
    };
    this.admin.run(['import-run'], { input: JSON.stringify({ mode, items: keys, root }) + '\n', onStage,
      onSpawn: p => { this.proc = p; }, timeout: 24 * 3600 * 1000 })
      .then(r => this.finish(r || {}))
      .catch(e => this.set({ state: 'idle', error: this.cancelled ? 'Stopped' : e.message }))
      .finally(() => { this.proc = null; this.cancelled = false; });
    return this.status();
  }

  cancel() {
    if (this.state.state !== 'importing' || !this.proc) return this.status();
    this.cancelled = true;
    try { this.proc.kill('SIGTERM'); } catch {}
    return this.status();
  }

  async finish(r) {
    const notes = [];
    // GOG: Hearth's own list of GOG installs.
    if (r.gog && r.gog.length) {
      this.store.update(d => { d.gogInstalls = { ...(d.gogInstalls || {}) }; for (const g of r.gog) d.gogInstalls[g.id] = g.path; });
    }
    // Lutris: they become your own games in Hearth (Library), started with Proton or natively like any other.
    if (r.lutris && r.lutris.length) {
      const crypto = require('node:crypto');
      const missing = r.lutris.filter(g => !g.exe).map(g => g.name);
      this.store.update(d => {
        d.manual = d.manual || [];
        for (const g of r.lutris.filter(x => x.exe)) {
          if (d.manual.some(m => m.path === g.exe)) continue;
          d.manual.push({ id: 'manual:' + crypto.randomBytes(6).toString('hex'), title: g.name, path: g.exe, added: Date.now() });
        }
      });
      if (missing.length) notes.push(`Copied ${missing.join(', ')} from Lutris, but couldn't tell which program starts ${missing.length === 1 ? 'it' : 'them'}. Add ${missing.length === 1 ? 'it' : 'them'} with Library › Add a game.`);
    }
    // Epic: legendary needs to know about them (and needs you signed in to Epic for that).
    if (r.epic && r.epic.length) {
      this.store.update(d => { d.pendingEpicImports = [...(d.pendingEpicImports || []), ...r.epic]; });
      const left = await this.finishPendingEpic();
      if (left) notes.push(`Sign in to Epic Games (Library › Stores) to finish ${left === 1 ? '1 Epic game' : left + ' Epic games'}.`);
    }
    // Steam reads its game list when it starts: restart it if it's running.
    if (r.steam > 0 && steam.isRunning()) {
      notes.push('Steam restarted to find the new games. It checks each one before you play.');
      try {
        steam.openUri('steam://exit');
        setTimeout(() => steam.ensureRunningSilent().catch(() => {}), 8000);
      } catch {}
    } else if (r.steam > 0) notes.push('Steam checks each game the first time you start it.');
    this.set({ state: 'idle', result: { ...r, notes } });
    try { await this.library.get(true); } catch {}
    this.emit('libraryChanged');
    // The drives changed (moved games): scan again later instead of showing a stale list.
    this.state.scan = null;
  }

  // ------------------------------------------------------------------ remove the old system
  // After importing: erase the other systems' partitions on Hearth's own drive and give Hearth the space
  // (hearth-takeover). Only partitions it offers can be chosen, never Hearth's own.

  async takeoverPlan() {
    if (!this.supported) throw new Error('This needs Hearth, the OS.');
    if (this.state.state !== 'idle') return this.status();
    try { this.set({ takeover: await this.admin.run(['takeover-plan'], { timeout: 60000 }), takeoverError: null }); }
    catch (e) { this.set({ takeover: null, takeoverError: e.message }); }
    return this.status();
  }

  takeoverRun(remove, windowsBoot = true) {
    if (this.state.state !== 'idle') throw new Error('Already busy');
    if (!Array.isArray(remove) || !remove.length) throw new Error('Choose what to remove first');
    this.set({ state: 'takeover', takeoverStage: { phase: 'erase', step: 0, steps: 3, desc: 'Starting' }, takeoverResult: null, takeoverError: null });
    this.admin.run(['takeover-run'], { input: JSON.stringify({ remove, windowsBoot: !!windowsBoot }) + '\n',
      onStage: st => this.set({ takeoverStage: st }), timeout: 60 * 60000 })
      .then(r => { this.set({ state: 'idle', takeoverResult: r || {}, takeover: null, scan: null }); log('takeover: done ' + JSON.stringify(r)); })
      .catch(e => { this.set({ state: 'idle', takeoverError: e.message }); log('takeover: ' + e.message); this.takeoverPlan(); });
    return this.status();
  }

  async finishPendingEpic() {
    const pending = this.store.data.pendingEpicImports || [];
    if (!pending.length || !this.library.accounts.epic.signedIn || this.epicBusy) return pending.length;
    this.epicBusy = true;
    const left = [];
    try {
      for (const g of pending) {
        try { await this.library.accounts.epic.importGame(g.app, g.path); log(`import: Epic ${g.app} registered`); }
        catch (e) { log(`import: Epic ${g.app}: ${e.message}`); if (!/didn't accept/.test(e.message)) left.push(g); }
      }
    } finally { this.epicBusy = false; }
    this.store.update(d => { d.pendingEpicImports = left; });
    if (left.length < pending.length) { try { await this.library.get(true); } catch {} this.emit('libraryChanged'); }
    return left.length;
  }
}

module.exports = { Importer };
