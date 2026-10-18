'use strict';
// Profiles: several people on one Hearth, each with their own Steam, Epic, GOG and Xbox sign-ins, favorites,
// recent games, play time, name and picture. Installed games, downloads, tools and art are shared, so a game
// installed once is there for everyone (each person plays it with their own account).
//
// How: the files that belong to a person live in Hearth's data folder while their profile is in use, and are
// put away in profiles/<id>/ when someone else's profile is. Switching swaps them and restarts Hearth (and
// Steam, signed in as that profile's Steam account). The first profile is whoever used Hearth before profiles
// existed, so nothing moves for them.

const fs = require('node:fs');
const path = require('node:path');

/** A person's own files and folders, relative to Hearth's data folder. */
const OWN = ['library.json', 'profile', 'accounts', 'legendary/user.json', 'gog/auth.json'];
/** Caches of what each account owns: dropped on a switch (fetched again for the new account). */
const CACHES = ['cache/epic-owned.json', 'cache/gog-owned.json', 'cache/xbox-owned.json', 'cache/xbox-titles.json'];
const COLORS = ['#ff8a3d', '#3fd0b8', '#5aa9ff', '#b57bff', '#ffd166', '#e5483b', '#7ad15a'];

class Profiles {
  constructor(dataDir) {
    this.dir = dataDir;
    this.file = path.join(dataDir, 'profiles.json');
    let d = null;
    try { d = JSON.parse(fs.readFileSync(this.file, 'utf8')); } catch {}
    this.data = d && Array.isArray(d.list) && d.list.length ? d : { active: 'main', askAtStart: false, list: [{ id: 'main', name: null, color: COLORS[0], steam: null }] };
    if (!this.data.list.some(p => p.id === this.data.active)) this.data.active = this.data.list[0].id;
  }
  save() { fs.mkdirSync(this.dir, { recursive: true }); const t = this.file + '.tmp'; fs.writeFileSync(t, JSON.stringify(this.data, null, 2)); fs.renameSync(t, this.file); }
  get active() { return this.data.active; }
  get list() { return this.data.list; }
  get askAtStart() { return !!this.data.askAtStart; }
  current() { return this.data.list.find(p => p.id === this.data.active); }
  find(id) { return this.data.list.find(p => p.id === id); }
  stash(id) { return path.join(this.dir, 'profiles', id); }

  /** The profile's name as Hearth shows it: what's in its library.json, else what was given when it was made. */
  label(p, liveName) { return (p.id === this.active && liveName) || p.name || 'Player'; }

  create({ name, color }) {
    const clean = String(name || '').trim().slice(0, 32);
    if (!clean) throw new Error('Give the profile a name');
    if (this.data.list.length >= 8) throw new Error('Hearth has room for 8 profiles');
    const id = 'p' + Date.now().toString(36);
    const p = { id, name: clean, color: COLORS.includes(color) || /^#[0-9a-f]{6}$/i.test(color || '') ? color : COLORS[this.data.list.length % COLORS.length], steam: null };
    this.data.list.push(p);
    // Its own library.json from the start, with its name and color (and the manual games everyone shares).
    const dst = this.stash(id);
    fs.mkdirSync(dst, { recursive: true });
    let shared = [];
    try { shared = JSON.parse(fs.readFileSync(path.join(this.dir, 'library.json'), 'utf8')).manual || []; } catch {}
    fs.writeFileSync(path.join(dst, 'library.json'), JSON.stringify({ manual: shared, profile: { name: clean, accent: p.color } }));
    this.save();
    return p;
  }

  update(id, patch) {
    const p = this.find(id); if (!p) throw new Error('No such profile');
    if (patch.name != null) p.name = String(patch.name).trim().slice(0, 32) || p.name;
    if (patch.color && /^#[0-9a-f]{6}$/i.test(patch.color)) p.color = patch.color;
    if (patch.steam !== undefined) p.steam = patch.steam ? String(patch.steam).slice(0, 64) : null;
    this.save();
    return p;
  }

  remove(id) {
    if (id === this.active) throw new Error("Switch to another profile before removing this one");
    if (this.data.list.length <= 1) throw new Error('There has to be one profile');
    this.data.list = this.data.list.filter(p => p.id !== id);
    fs.rmSync(this.stash(id), { recursive: true, force: true });
    this.save();
  }

  setAskAtStart(on) { this.data.askAtStart = !!on; this.save(); }

  /**
   * Puts the current person's files away and brings out the other person's. Hearth must restart afterwards
   * (it read the old files at start). Manual games carry over (everyone shares them).
   */
  switchTo(id) {
    const to = this.find(id); if (!to) throw new Error('No such profile');
    if (id === this.active) return to;
    const from = this.active;
    let manual = [];
    try { manual = JSON.parse(fs.readFileSync(path.join(this.dir, 'library.json'), 'utf8')).manual || []; } catch {}
    // 1. put away
    const away = this.stash(from);
    fs.rmSync(away, { recursive: true, force: true });
    fs.mkdirSync(away, { recursive: true });
    for (const rel of OWN) move(path.join(this.dir, rel), path.join(away, rel));
    // 2. bring out
    const back = this.stash(id);
    for (const rel of OWN) move(path.join(back, rel), path.join(this.dir, rel));
    fs.rmSync(back, { recursive: true, force: true });
    for (const rel of CACHES) fs.rmSync(path.join(this.dir, rel), { force: true });
    // 3. shared manual games
    const lib = path.join(this.dir, 'library.json');
    let d = {};
    try { d = JSON.parse(fs.readFileSync(lib, 'utf8')); } catch {}
    const have = new Set((d.manual || []).map(g => g.id));
    d.manual = [...(d.manual || []), ...manual.filter(g => !have.has(g.id))];
    if (!d.profile) d.profile = { name: to.name, accent: to.color };
    fs.writeFileSync(lib, JSON.stringify(d, null, 2));
    this.data.active = id;
    this.save();
    return to;
  }
}

function move(src, dst) {
  if (!fs.existsSync(src)) return;
  fs.mkdirSync(path.dirname(dst), { recursive: true });
  fs.rmSync(dst, { recursive: true, force: true });
  try { fs.renameSync(src, dst); }
  catch { fs.cpSync(src, dst, { recursive: true }); fs.rmSync(src, { recursive: true, force: true }); }
}

// ------------------------------------------------------------------ Steam's account

/**
 * Makes Steam sign in as this account next time it starts (it must have signed in on this device once, with
 * "Remember me"): AutoLoginUser in registry.vdf, and MostRecent in loginusers.vdf. Text edits, so the rest of
 * Steam's files stay exactly as they were. '' asks Steam to show its sign-in screen.
 */
function steamRegistryText(text, account) {
  const v = String(account || '').replace(/"/g, '');
  if (/"AutoLoginUser"\s+"[^"]*"/.test(text)) text = text.replace(/("AutoLoginUser"\s+)"[^"]*"/, `$1"${v}"`);
  else text = text.replace(/("Steam"\s*\{)/, `$1\n\t\t\t\t\t"AutoLoginUser"\t\t"${v}"`);
  if (/"RememberPassword"\s+"[^"]*"/.test(text)) text = text.replace(/("RememberPassword"\s+)"[^"]*"/, '$1"1"');
  return text;
}
function steamLoginUsersText(text, account) {
  const want = String(account || '').toLowerCase();
  return text.replace(/("\d{17}"\s*\{)([^{}]*)(\})/g, (all, open, body, close) => {
    const name = (body.match(/"AccountName"\s+"([^"]*)"/) || [])[1] || '';
    const mine = want && name.toLowerCase() === want;
    const set = (b, key, val) => (new RegExp(`"${key}"\\s+"[^"]*"`).test(b) ? b.replace(new RegExp(`("${key}"\\s+)"[^"]*"`), `$1"${val}"`) : b.replace(/\s*$/, `\n\t\t"${key}"\t\t"${val}"\n\t`));
    let b = set(body, 'MostRecent', mine ? '1' : '0');
    if (mine) b = set(b, 'AllowAutoLogin', '1');
    return open + b + close;
  });
}
/** The Steam accounts that have signed in on this device: [{account, name}]. */
function steamAccounts(loginUsersText) {
  const out = [];
  for (const m of String(loginUsersText || '').matchAll(/"\d{17}"\s*\{([^{}]*)\}/g)) {
    const acc = (m[1].match(/"AccountName"\s+"([^"]*)"/) || [])[1];
    const name = (m[1].match(/"PersonaName"\s+"([^"]*)"/) || [])[1];
    if (acc) out.push({ account: acc, name: name || acc });
  }
  return out;
}

module.exports = { Profiles, steamRegistryText, steamLoginUsersText, steamAccounts, OWN };
