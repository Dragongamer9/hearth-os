'use strict';
// Store accounts and the Nimbus profile on Linux.
//
// Accounts: on Linux the stores' own apps hold the sign-ins. Steam signs in in Steam; Epic and GOG in
// Heroic Games Launcher; Xbox in the cloud-gaming window the first time you stream. Nimbus reads who is
// signed in from their files and opens the right app when you choose Sign in.
//
// Profile: name, status, picture, accent and background, the same fields as Windows.
// Pictures you choose are copied to ~/.local/share/nimbus/profile/ and served as nimbus-art://profile/<kind>.

const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const vdf = require('./library/vdf');
const steam = require('./library/steam');
const { DATA_DIR, readJson, isDir, exists, has, launchDetached } = require('./util');

const PROFILE_DIR = path.join(DATA_DIR, 'profile');
const IMAGE_EXT = ['.png', '.jpg', '.jpeg', '.webp', '.gif'];
const HEROIC_FLATPAK = 'com.heroicgameslauncher.hgl';

// ------------------------------------------------------------------ who is signed in

function steamAccount(home) {
  const dir = steam.steamPath(home);
  if (!dir) return { signedIn: false, name: null, avatarFile: null, note: 'Steam is not installed' };
  const users = vdf.obj(vdf.load(path.join(dir, 'config', 'loginusers.vdf')), 'users') || {};
  for (const [id64, u] of Object.entries(users)) {
    if (typeof u !== 'object' || vdf.str(u, 'MostRecent') !== '1') continue;
    const avatar = path.join(dir, 'config', 'avatarcache', id64 + '.png');
    return { signedIn: true, name: vdf.str(u, 'PersonaName') || vdf.str(u, 'AccountName') || null, avatarFile: exists(avatar) ? avatar : null };
  }
  return { signedIn: false, name: null, avatarFile: null };
}

function heroicDirs(home = os.homedir()) {
  return [
    path.join(process.env.XDG_CONFIG_HOME || path.join(home, '.config'), 'heroic'),
    path.join(home, '.var', 'app', HEROIC_FLATPAK, 'config', 'heroic'),
  ].filter(isDir);
}

function heroicAccount(store, home) {
  for (const dir of heroicDirs(home)) {
    if (store === 'epic') {
      const u = readJson(path.join(dir, 'legendaryConfig', 'legendary', 'user.json'));
      if (u) return { signedIn: true, name: u.displayName || null };
    } else {
      const auth = readJson(path.join(dir, 'gog_store', 'auth.json'));
      if (auth && Object.keys(auth).length) {
        const user = readJson(path.join(dir, 'gog_store', 'user.json')) || {};
        return { signedIn: true, name: user.username || null };
      }
    }
  }
  return { signedIn: false, name: null, note: heroicDirs(home).length ? null : 'Install Heroic Games Launcher to add Epic and GOG games' };
}

function openHeroic() {
  const flatpak = heroicDirs().some(d => d.includes(HEROIC_FLATPAK));
  if (has('heroic') && !flatpak) return launchDetached('heroic');
  if (has('flatpak') && flatpak) return launchDetached('flatpak', ['run', HEROIC_FLATPAK]);
  if (has('heroic')) return launchDetached('heroic');
  throw new Error('Heroic Games Launcher is not installed. Install it (Flathub: com.heroicgameslauncher.hgl), sign in there, and its games show up here.');
}

class Accounts {
  constructor({ library, openXboxSignIn }) {
    this.library = library;
    this.openXboxSignIn = openXboxSignIn; // resolves when the sign-in window closes
  }

  status() {
    const installed = src => [...this.library.games.values()].filter(g => g.source === src && g.installed).length;
    const st = steamAccount();
    const ep = heroicAccount('epic'), gg = heroicAccount('gog');
    const cloud = [...this.library.games.values()].filter(g => g.source === 'xbox' && g.id !== 'xbox:cloud').length;
    const base = { owned: 0, fetched: 0, busy: false, error: null, toolReady: true, avatar: null };
    return {
      installRoot: '',
      accounts: [
        { ...base, store: 'steam', signedIn: st.signedIn, name: st.name, note: st.note || null,
          avatar: st.avatarFile ? 'nimbus-art://profile/steam' : null, installed: installed('steam') },
        // Heroic keeps the game lists; Nimbus reads them at every scan, so they're always current.
        { ...base, store: 'epic', signedIn: ep.signedIn, name: ep.name, error: ep.note || null, installed: installed('epic'), fetched: ep.signedIn ? Date.now() : 0 },
        { ...base, store: 'gog', signedIn: gg.signedIn, name: gg.name, error: gg.note || null, installed: installed('gog'), fetched: gg.signedIn ? Date.now() : 0 },
        { ...base, store: 'xbox', signedIn: false, name: null, owned: cloud, installed: 0 },
      ],
    };
  }

  async signIn(store) {
    if (store === 'steam') { steam.openUri('steam://open/main'); return { ok: true }; }
    if (store === 'epic' || store === 'gog') { openHeroic(); return { ok: false }; }
    if (store === 'xbox') { await this.openXboxSignIn(); return { ok: false }; }
    throw new Error('Unknown store');
  }

  signOut(store) {
    if (store === 'epic' || store === 'gog') {
      openHeroic();
      throw new Error('Sign out in Heroic Games Launcher (it is opening now)');
    }
    throw new Error('Sign out in the store\'s own app');
  }

  async refresh() { await this.library.scan(); }
}

// ------------------------------------------------------------------ profile

const PROFILE_DEFAULTS = { name: null, status: null, avatar: 'initials', accent: '#3fd0b8', background: 'default', imageStamp: 0 };

function profileImage(kind) {
  for (const ext of IMAGE_EXT) { const f = path.join(PROFILE_DIR, kind + ext); if (exists(f)) return f; }
  return null;
}

class Profile {
  constructor(store) { this.store = store; }

  get data() { return { ...PROFILE_DEFAULTS, ...(this.store.data.profile || {}) }; }

  /** Local file behind nimbus-art://profile/<kind>. */
  file(kind) {
    if (kind === 'avatar' || kind === 'background') return profileImage(kind);
    if (kind === 'steam') return steamAccount().avatarFile;
    return null;
  }

  get() {
    const p = this.data;
    const url = kind => (this.file(kind) ? `nimbus-art://profile/${kind}?v=${p.imageStamp}` : null);
    return {
      name: p.name || os.userInfo().username, status: p.status, avatar: p.avatar, accent: p.accent, background: p.background,
      customAvatar: url('avatar'), customBackground: url('background'),
      storeAvatars: { steam: url('steam'), xbox: null, gog: null },
    };
  }

  set(c) {
    const clip = (v, n) => { const t = String(v).trim(); return t ? t.slice(0, n) : null; };
    this.store.update(d => {
      const p = d.profile = { ...PROFILE_DEFAULTS, ...(d.profile || {}) };
      if (c.name != null) p.name = clip(c.name, 32);
      if (c.status != null) p.status = clip(c.status, 60);
      if (c.avatar != null) p.avatar = String(c.avatar);
      if (c.accent != null && /^#[0-9a-f]{6}$/i.test(c.accent)) p.accent = c.accent;
      if (c.background != null) p.background = String(c.background);
    });
    return this.get();
  }

  /** Copies a picture you chose into Nimbus's profile folder and uses it. */
  setImage(kind, file) {
    if (kind !== 'avatar' && kind !== 'background') throw new Error('Unknown picture');
    const ext = path.extname(file).toLowerCase();
    if (!IMAGE_EXT.includes(ext)) throw new Error('Pick a PNG, JPG, WebP or GIF picture.');
    if (fs.statSync(file).size > 20_000_000) throw new Error('That picture is too big (over 20 MB).');
    fs.mkdirSync(PROFILE_DIR, { recursive: true });
    const old = profileImage(kind);
    if (old) fs.unlinkSync(old);
    fs.copyFileSync(file, path.join(PROFILE_DIR, kind + ext));
    this.store.update(d => {
      const p = d.profile = { ...PROFILE_DEFAULTS, ...(d.profile || {}) };
      p.imageStamp = Date.now();
      if (kind === 'avatar') p.avatar = 'custom'; else p.background = 'custom';
    });
    return this.get();
  }
}

module.exports = { Accounts, Profile, steamAccount, heroicAccount };
