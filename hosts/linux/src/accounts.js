'use strict';
// The Steam account (read from Steam's files) and the Nimbus profile on Linux.
// Epic, GOG and Xbox accounts are in stores/.
//
// Profile: name, status, picture, accent and background, the same fields as Windows.
// Pictures you choose are copied to ~/.local/share/nimbus/profile/ and served as nimbus-art://profile/<kind>.

const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const vdf = require('./library/vdf');
const steam = require('./library/steam');
const { DATA_DIR, exists } = require('./util');
const { artCache } = require('./art-cache');

const PROFILE_DIR = path.join(DATA_DIR, 'profile');
const IMAGE_EXT = ['.png', '.jpg', '.jpeg', '.webp', '.gif'];

// ------------------------------------------------------------------ who is signed in

/**
 * The Steam account on this device: the user marked MostRecent in loginusers.vdf, else the AutoLoginUser in
 * registry.vdf, else the one that signed in last. Steam rewrites these files while it runs; a read that finds
 * nothing keeps the last good answer instead of flipping to "not signed in".
 */
let lastAccount = null;
function pickUser(users, autoLogin) {
  const list = Object.entries(users || {}).filter(([, u]) => u && typeof u === 'object');
  if (!list.length) return null;
  return list.find(([, u]) => vdf.str(u, 'MostRecent') === '1')
    || (autoLogin && list.find(([, u]) => (vdf.str(u, 'AccountName') || '').toLowerCase() === autoLogin.toLowerCase()))
    || list.sort((a, b) => (+vdf.str(b[1], 'Timestamp') || 0) - (+vdf.str(a[1], 'Timestamp') || 0))[0];
}
function steamAccount(home) {
  const dir = steam.steamPath(home);
  if (!dir) {
    const installed = !home && steam.clientInstalled();
    return { signedIn: false, name: null, avatarFile: null, installed, note: installed ? 'Open Steam once to sign in' : 'Steam is not installed' };
  }
  const file = path.join(dir, 'config', 'loginusers.vdf');
  let users = vdf.obj(vdf.load(file), 'users');
  if (!users && exists(file)) users = vdf.obj(vdf.load(file), 'users');        // caught it mid-write: once more
  const reg = vdf.load(path.join(path.dirname(dir), 'registry.vdf')) || vdf.load(path.join(os.homedir(), '.steam', 'registry.vdf'));
  const autoLogin = vdf.str(reg, 'Registry', 'HKCU', 'Software', 'Valve', 'Steam', 'AutoLoginUser');
  const hit = pickUser(users, autoLogin);
  if (!hit) return (!home && lastAccount && exists(file)) ? lastAccount : { signedIn: false, name: null, avatarFile: null, installed: true };
  const [id64, u] = hit;
  const avatar = path.join(dir, 'config', 'avatarcache', id64 + '.png');
  const out = { signedIn: true, installed: true, name: vdf.str(u, 'PersonaName') || vdf.str(u, 'AccountName') || null, avatarFile: exists(avatar) ? avatar : null };
  if (!home) lastAccount = out;
  return out;
}

// ------------------------------------------------------------------ profile

const PROFILE_DEFAULTS = { name: null, status: null, avatar: 'initials', accent: '#3fd0b8', background: 'default', imageStamp: 0 };

function profileImage(kind) {
  for (const ext of IMAGE_EXT) { const f = path.join(PROFILE_DIR, kind + ext); if (exists(f)) return f; }
  return null;
}

class Profile {
  /** @param store library.json  @param accounts stores/ (for the Xbox gamerpic and GOG avatar) */
  constructor(store, accounts) { this.store = store; this.accounts = accounts; }

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
      storeAvatars: { steam: url('steam'), xbox: this.accounts ? artCache().url(this.accounts.xbox.gamerpic) : null,
        gog: this.accounts ? artCache().url(this.accounts.gog.profile().avatar) : null },
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

module.exports = { Profile, steamAccount, pickUser };
