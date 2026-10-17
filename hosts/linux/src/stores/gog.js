'use strict';
// GOG without GOG Galaxy. Sign-in and the game list use GOG's web API; downloads use gogdl.
// Port of the Windows host's Stores/GogStore.cs. The sign-in lives in ~/.local/share/nimbus/gog/auth.json
// (gogdl's format, so gogdl reads it too). GOG's Windows builds run with Proton.

const fs = require('node:fs');
const path = require('node:path');
const tools = require('./tools');
const proton = require('./proton');
const { OwnedCache, https } = require('./common');
const { DATA_DIR, readJson, writeJsonAtomic, launchDetached, log } = require('../util');

const CLIENT_ID = '46899977096215655';
const CLIENT_SECRET = '9d85c43b1482497dbbce61f6e4aa173a433796eeae2ca8c5f6129f2dc4de46d9';
const REDIRECT = 'https://embed.gog.com/on_login_success?origin=client';
const LOGIN_URL = `https://auth.gog.com/auth?client_id=${CLIENT_ID}&redirect_uri=${encodeURIComponent(REDIRECT)}&response_type=code&layout=client2`;
const DIR = path.join(DATA_DIR, 'gog');
const AUTH = path.join(DIR, 'auth.json');
const PROFILE = path.join(DIR, 'profile.json');
const CACHE = 'gog-owned.json';
const ENV = { GOGDL_CONFIG_PATH: DIR };

async function getJson(url, init) {
  const r = await fetch(url, { ...init, signal: AbortSignal.timeout(30000) });
  if (!r.ok) { const e = new Error(`HTTP ${r.status}`); e.status = r.status; throw e; }
  return r.json();
}

class GogStore {
  constructor() { this.owned = OwnedCache.load(CACHE); this.tokenLock = Promise.resolve(); }

  get signedIn() { return fs.existsSync(AUTH); }
  profile() { const p = readJson(PROFILE) || {}; return { name: p.name || null, avatar: p.avatar || null }; }

  async signIn(signInWindow) {
    const tool = tools.ensure(tools.TOOLS.gogdl);
    tool.catch(() => {});
    const code = await signInWindow({
      title: 'Sign in to GOG', url: LOGIN_URL,
      onNavigate: url => (url.toLowerCase().startsWith('https://embed.gog.com/on_login_success') ? new URL(url).searchParams.get('code') || '' : null),
    });
    if (!code) return false;
    const url = `https://auth.gog.com/token?client_id=${CLIENT_ID}&client_secret=${CLIENT_SECRET}&grant_type=authorization_code` +
      `&redirect_uri=${encodeURIComponent(REDIRECT)}&code=${encodeURIComponent(code)}`;
    this.saveToken(await getJson(url));
    await this.loadProfile();
    await tool;
    return true;
  }

  signOut() {
    for (const f of [AUTH, PROFILE]) fs.rmSync(f, { force: true });
    OwnedCache.remove(CACHE);
    this.owned = new OwnedCache();
  }

  // gogdl's format: { "<client id>": { access_token, refresh_token, expires_in, user_id, loginTime, ... } }
  saveToken(token) {
    if (!token || !token.access_token) throw new Error("GOG didn't accept the sign-in. Try again.");
    token.loginTime = Date.now() / 1000;
    fs.mkdirSync(DIR, { recursive: true, mode: 0o700 });
    writeJsonAtomic(AUTH, { [CLIENT_ID]: token });
    fs.chmodSync(AUTH, 0o600);
  }

  /** A valid access token, refreshed if it has expired. */
  token() {
    const run = this.tokenLock.then(async () => {
      const all = readJson(AUTH);
      const t = all && all[CLIENT_ID];
      if (!t) throw new Error('Sign in to GOG first.');
      if (Date.now() / 1000 < (t.loginTime || 0) + (t.expires_in || 0) - 120) return t.access_token;
      const url = `https://auth.gog.com/token?client_id=${CLIENT_ID}&client_secret=${CLIENT_SECRET}&grant_type=refresh_token` +
        `&refresh_token=${encodeURIComponent(t.refresh_token)}`;
      let body;
      try { body = await getJson(url); } catch { throw new Error('Your GOG sign-in expired. Sign in again under Library › Stores.'); }
      this.saveToken(body);
      return body.access_token;
    });
    this.tokenLock = run.catch(() => {});
    return run;
  }

  async api(url) { return getJson(url, { headers: { authorization: 'Bearer ' + await this.token() } }); }

  async loadProfile() {
    try {
      const u = await this.api('https://embed.gog.com/userData.json');
      const av = u.avatars || {};
      const avatar = av.menu_user_av_big2 || av.menu_user_av_big || Object.values(av).pop() || null;
      fs.mkdirSync(DIR, { recursive: true });
      writeJsonAtomic(PROFILE, { name: u.username || null, avatar: avatar ? https(avatar) : null });
    } catch (e) { log('gog profile: ' + e.message); }
  }

  /** Downloads the list of Windows games on your GOG account, with cover art. */
  async refreshOwned() {
    if (!this.profile().name) await this.loadProfile();
    const list = [];
    for (let page = 1, pages = 1; page <= pages && page <= 100; page++) {
      const r = await this.api(`https://embed.gog.com/account/getFilteredProducts?mediaType=1&sortBy=title&page=${page}`);
      pages = Number.isInteger(r.totalPages) ? r.totalPages : 1;
      for (const p of r.products || []) {
        if (!p.worksOn || !p.worksOn.Windows || p.id == null) continue;
        const o = { id: String(p.id), title: p.title || String(p.id), art: {}, added: 0, lastPlayed: 0, note: null };
        if (p.image) { o.art.wide = https(p.image) + '_product_card_v2_mobile_slider_639.jpg'; o.art.hero = o.art.wide; }
        list.push(o);
      }
    }
    // Better art (vertical covers, backgrounds) from GOG's game database; kept between refreshes.
    const old = new Map(this.owned.games.map(g => [g.id, g]));
    let i = 0;
    const worker = async () => {
      while (i < list.length) {
        const o = list[i++];
        const prev = old.get(o.id);
        if (prev && prev.art.cover) { o.art = prev.art; continue; }
        try {
          const game = (await getJson(`https://gamesdb.gog.com/platforms/gog/external_releases/${o.id}`)).game || {};
          const f = (k, ext) => game[k] && game[k].url_format ? game[k].url_format.replace('{formatter}', '').replace('{ext}', ext) : null;
          if (f('vertical_cover', 'jpg')) o.art.cover = f('vertical_cover', 'jpg');
          if (f('background', 'jpg')) o.art.hero = f('background', 'jpg');
          if (f('logo', 'jpg')) o.art.wide = f('logo', 'jpg');
        } catch { /* art is optional */ }
      }
    };
    await Promise.all(Array.from({ length: 6 }, worker));
    const cache = new OwnedCache({ fetched: Date.now(), games: list });
    cache.save(CACHE);
    this.owned = cache;
  }

  /** GOG games Nimbus installed (records kept in library.json: id → folder). */
  installed(records) {
    const out = [];
    for (const [id, dir] of Object.entries(records || {})) {
      const info = readJson(path.join(dir, `goggame-${id}.info`));
      if (!info) continue;
      const tasks = info.playTasks || [];
      const task = tasks.find(t => t.isPrimary) || tasks.find(t => t.type === 'FileTask');
      if (!task || !task.path) continue;
      const exe = path.join(dir, task.path.replace(/\\/g, '/'));
      const owned = this.owned.games.find(o => o.id === id);
      let added = 0;
      try { const st = fs.statSync(dir); added = Math.round(st.birthtimeMs || st.mtimeMs); } catch {}
      out.push({
        id: 'gog:' + id, title: info.name || (owned && owned.title) || id, source: 'gog',
        installPath: dir, added, installed: true, state: 'installed', canUninstall: true,
        launchVia: 'proton', windowsExe: exe, args: task.arguments || '',
        workDir: task.workingDir ? path.join(dir, task.workingDir.replace(/\\/g, '/')) : path.dirname(exe),
        gogId: id, art: owned ? { ...owned.art } : {},
        track: { dirs: [dir] },
      });
    }
    return out;
  }

  /** Downloads a game into <root>/GOG/<its folder>. Resolves with the folder. */
  async install(job, id, root, signal) {
    await this.token(); // refresh now so gogdl finds a valid token
    const base = path.join(root, 'GOG');
    fs.mkdirSync(base, { recursive: true });
    const r = await tools.run(tools.TOOLS.gogdl,
      ['--auth-config-path', AUTH, 'download', id, '--platform', 'windows', '--path', base, '--skip-dlcs', '--lang', 'en-US'],
      { env: ENV, onLine: l => job.readLine(l, true), signal });
    const dir = fs.readdirSync(base).map(d => path.join(base, d)).find(d => fs.existsSync(path.join(d, `goggame-${id}.info`)));
    if (!dir) throw new Error('GOG install failed: ' + tools.problem(r));
    return dir;
  }

  launch(g) {
    const args = g.args ? g.args.match(/"[^"]*"|\S+/g).map(a => a.replace(/^"|"$/g, '')) : [];
    return proton.launch({ store: 'gog', id: g.gogId, exe: g.windowsExe, args, cwd: g.workDir });
  }
}

module.exports = { GogStore, LOGIN_URL };
