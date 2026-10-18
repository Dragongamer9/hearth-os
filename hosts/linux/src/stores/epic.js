'use strict';
// Epic Games without the Epic Games Launcher, through legendary. Port of the Windows host's Stores/EpicStore.cs.
// legendary keeps its sign-in and install records in ~/.local/share/nimbus/legendary. Epic games are Windows
// games, so they run with Proton (proton.js).

const fs = require('node:fs');
const path = require('node:path');
const tools = require('./tools');
const proton = require('./proton');
const { OwnedCache, ms, jsonIn } = require('./common');
const { DATA_DIR, readJson, isDir, log } = require('../util');

const CLIENT_ID = '34a02cf8f4414e29b15921876da36f9a';
const REDIRECT = `https://www.epicgames.com/id/api/redirect?clientId=${CLIENT_ID}&responseType=code`;
const LOGIN_URL = 'https://www.epicgames.com/id/login?redirectUrl=' + encodeURIComponent(REDIRECT);
const CONFIG = path.join(DATA_DIR, 'legendary');
const CACHE = 'epic-owned.json';
const ENV = { LEGENDARY_CONFIG_PATH: CONFIG };

class EpicStore {
  constructor() { this.owned = OwnedCache.load(CACHE); }

  get signedIn() { return fs.existsSync(path.join(CONFIG, 'user.json')); }
  displayName() { const u = readJson(path.join(CONFIG, 'user.json')); return u ? u.displayName || null : null; }

  /** Shows Epic's sign-in page; resolves false if you closed it. */
  async signIn(signInWindow) {
    const tool = tools.ensure(tools.TOOLS.legendary); // download while you sign in
    tool.catch(() => {});
    const code = await signInWindow({
      title: 'Sign in to Epic Games', url: LOGIN_URL,
      // After signing in Epic shows a small JSON page with a one-time code.
      onLoaded: async (url, readText, navigate) => {
        if (!url.toLowerCase().startsWith('https://www.epicgames.com/id/api/redirect')) return null;
        let c = null;
        try { c = JSON.parse(await readText()).authorizationCode; } catch {}
        if (c) return c;
        navigate(LOGIN_URL); // not signed in yet
        return null;
      },
    });
    if (!code) return false;
    await tool;
    const r = await tools.run(tools.TOOLS.legendary, ['auth', '--code', code], { env: ENV, timeoutMs: 120000 });
    if (!this.signedIn) throw new Error("Epic sign-in didn't finish: " + tools.problem(r));
    return true;
  }

  async signOut() {
    if (tools.isReady(tools.TOOLS.legendary)) {
      await tools.run(tools.TOOLS.legendary, ['auth', '--delete'], { env: ENV, timeoutMs: 60000 }).catch(() => {});
    }
    fs.rmSync(path.join(CONFIG, 'user.json'), { force: true });
    this.owned = new OwnedCache();
    OwnedCache.remove(CACHE);
  }

  /** Downloads the list of games on your Epic account, with art. */
  async refreshOwned() {
    const r = await tools.run(tools.TOOLS.legendary, ['list', '--json'], { env: ENV, timeoutMs: 240000 });
    const list = jsonIn(r.out, '[');
    if (r.code !== 0 || !Array.isArray(list)) throw new Error("Couldn't get your Epic library: " + tools.problem(r));
    const cache = new OwnedCache({ fetched: Date.now() });
    for (const g of list) {
      if (!g.app_name || !g.app_title) continue;
      const meta = g.metadata || {};
      const o = { id: g.app_name, title: g.app_title.trim(), art: {}, added: ms(meta.creationDate), lastPlayed: 0, note: null };
      for (const img of meta.keyImages || []) {
        if (!img.url) continue;
        switch (img.type) {
          case 'DieselGameBoxTall': o.art.cover = img.url; break;
          case 'DieselGameBox': o.art.hero = img.url; o.art.wide = o.art.wide || img.url; break;
          case 'DieselGameBoxWide': o.art.wide = img.url; break;
          case 'DieselGameBoxLogo': o.art.logo = img.url; break;
          case 'Thumbnail': o.art.cover = o.art.cover || img.url; break;
        }
      }
      const custom = meta.customAttributes || {};
      const third = (custom.ThirdPartyManagedApp || {}).value || (custom.ThirdPartyManagedProvider || {}).value;
      if (third) o.note = `Installs through ${third}`;
      cache.games.push(o);
    }
    cache.save(CACHE);
    this.owned = cache;
  }

  /** Games installed with legendary (by Nimbus), from legendary's own records. */
  installed() {
    const out = [];
    const recs = readJson(path.join(CONFIG, 'installed.json')) || {};
    for (const [app, e] of Object.entries(recs)) {
      if (!e || e.is_dlc || !e.install_path || !e.title || !isDir(e.install_path)) continue;
      const owned = this.owned.games.find(o => o.id === app);
      let added = 0;
      try { const st = fs.statSync(e.install_path); added = Math.round(st.birthtimeMs || st.mtimeMs); } catch {}
      out.push({
        id: 'epic:' + app, title: e.title, source: 'epic',
        installPath: e.install_path, sizeBytes: e.install_size > 0 ? e.install_size : null, added,
        installed: true, state: 'installed', canUninstall: true,
        launchVia: 'legendary', art: owned ? { ...owned.art } : {},
        track: { dirs: [e.install_path] },
      });
    }
    return out;
  }

  /** Installs a game into <root>/Epic. Runs inside the install queue. */
  async install(job, app, root, signal) {
    const base = path.join(root, 'Epic');
    fs.mkdirSync(base, { recursive: true });
    const r = await tools.run(tools.TOOLS.legendary,
      ['-y', 'install', app, '--base-path', base, '--skip-sdl', '--skip-dlcs', '--platform', 'Windows'],
      { env: ENV, onLine: l => job.readLine(l, false), signal });
    if (!this.installed().some(g => g.id === 'epic:' + app)) throw new Error('Epic install failed: ' + tools.problem(r));
  }

  async uninstall(app) {
    const r = await tools.run(tools.TOOLS.legendary, ['-y', 'uninstall', app], { env: ENV, timeoutMs: 600000 });
    if (this.installed().some(g => g.id === 'epic:' + app)) throw new Error("Couldn't uninstall: " + tools.problem(r));
    proton.removePrefix('epic', app);
  }

  /**
   * Asks legendary how to start the game (with a fresh Epic login token) and starts it with Proton.
   * Resolves with the child process.
   */
  async launch(app, options = null) {
    // Offline: straight to legendary's offline launch (the online one would only time out first).
    let r = require('../util').isOnline() ? await tools.run(tools.TOOLS.legendary, ['launch', app, '--json', '--skip-version-check'], { env: ENV, timeoutMs: 60000 })
      : { code: 1, out: '' };
    let p = jsonIn(r.out, '{');
    if (!p) {
      // Offline or Epic is down: games that allow it start without the online token.
      r = await tools.run(tools.TOOLS.legendary, ['launch', app, '--json', '--offline', '--skip-version-check'], { env: ENV, timeoutMs: 60000 });
      p = jsonIn(r.out, '{');
      if (!p) throw new Error("Couldn't start the game: " + tools.problem(r));
    }
    const dir = p.game_directory || '';
    const exe = path.join(dir, p.game_executable || '');
    if (!fs.existsSync(exe)) throw new Error(`The game's program is missing (${exe}). Try reinstalling it.`);
    const args = [...(p.game_parameters || []), ...(p.user_parameters || []), ...(p.egl_parameters || [])].filter(a => typeof a === 'string');
    log(`epic: launching ${app}`);
    return proton.launch({ store: 'epic', id: app, exe, args, cwd: p.working_directory || dir, env: p.environment || {}, options });
  }
}

module.exports = { EpicStore, LOGIN_URL };
