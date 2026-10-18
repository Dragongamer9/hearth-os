'use strict';
// Hearth Browser's own data: bookmarks, history and settings, kept as JSON in its profile folder.
// Small and synchronous on purpose (a few hundred KB at most); writes are batched.

const fs = require('node:fs');
const path = require('node:path');

const SEARCH = {
  duckduckgo: { name: 'DuckDuckGo', url: 'https://duckduckgo.com/?q=%s' },
  google: { name: 'Google', url: 'https://www.google.com/search?q=%s' },
  bing: { name: 'Bing', url: 'https://www.bing.com/search?q=%s' },
  brave: { name: 'Brave Search', url: 'https://search.brave.com/search?q=%s' },
};
const MAX_HISTORY = 5000;
// Endings that look like a domain but are usually file names or code ("node.js", "readme.md"): search those.
const NOT_TLD = new Set(['js', 'ts', 'py', 'sh', 'md', 'rs', 'cs', 'cpp', 'exe', 'txt', 'json', 'html', 'css', 'zip', 'pdf', 'png', 'jpg']);
const DEFAULTS = { search: 'duckduckgo', home: 'hearth://start', restoreTabs: true, blockPopups: true };
const START_TILES = [
  { title: 'YouTube', url: 'https://www.youtube.com/' },
  { title: 'Twitch', url: 'https://www.twitch.tv/' },
  { title: 'Steam Store', url: 'https://store.steampowered.com/' },
  { title: 'ProtonDB', url: 'https://www.protondb.com/' },
  { title: 'Reddit', url: 'https://www.reddit.com/' },
  { title: 'Hearth OS', url: 'https://github.com/Dragongamer9/hearth-os' },
];

/**
 * What the address bar should open for what was typed: a URL as typed, a bare domain or localhost with
 * https:// (http:// for local addresses), or a search with the chosen engine.
 */
function toUrl(input, engine = 'duckduckgo') {
  const s = String(input || '').trim();
  if (!s) return null;
  if (/^(https?|file|hearth|about|data|view-source):/i.test(s)) return s;
  const local = /^(localhost|127\.\d+\.\d+\.\d+|\[::1\]|192\.168\.\d+\.\d+|10\.\d+\.\d+\.\d+)(:\d+)?(\/.*)?$/i.test(s);
  if (local) return 'http://' + s;
  const tld = (s.match(/^[\w-]+(?:\.[\w-]+)*\.([a-z]{2,})(?::\d+)?(?:\/.*)?$/i) || [])[1];
  if (!/\s/.test(s) && tld && !NOT_TLD.has(tld.toLowerCase())) return 'https://' + s;
  const e = SEARCH[engine] || SEARCH.duckduckgo;
  return e.url.replace('%s', encodeURIComponent(s));
}

/** How a URL is shown in the address bar when not editing: without https:// and a trailing slash. */
function displayUrl(url) {
  if (!url || url.startsWith('hearth://')) return '';
  return url.replace(/^https:\/\//, '').replace(/^(www\.)/, '').replace(/\/$/, '');
}

class BrowserStore {
  constructor(dir) {
    this.dir = dir;
    fs.mkdirSync(dir, { recursive: true });
    this.data = {};
    for (const k of ['bookmarks', 'history', 'settings', 'session']) this.data[k] = this.read(k);
    if (!Array.isArray(this.data.bookmarks)) this.data.bookmarks = [];
    if (!Array.isArray(this.data.history)) this.data.history = [];
    this.data.settings = { ...DEFAULTS, ...(this.data.settings && typeof this.data.settings === 'object' ? this.data.settings : {}) };
    if (!Array.isArray(this.data.session)) this.data.session = [];
    this.dirty = new Set();
    this.timer = null;
  }
  file(k) { return path.join(this.dir, k + '.json'); }
  read(k) { try { return JSON.parse(fs.readFileSync(this.file(k), 'utf8')); } catch { return null; } }
  save(k) { this.dirty.add(k); clearTimeout(this.timer); this.timer = setTimeout(() => this.flush(), 800); }
  flush() {
    clearTimeout(this.timer);
    for (const k of this.dirty) {
      const tmp = this.file(k) + '.tmp';
      fs.writeFileSync(tmp, JSON.stringify(this.data[k]));
      fs.renameSync(tmp, this.file(k));
    }
    this.dirty.clear();
  }

  // ---- settings
  get settings() { return this.data.settings; }
  set(k, v) {
    if (!(k in DEFAULTS)) throw new Error('Unknown setting');
    if (k === 'search' && !SEARCH[v]) throw new Error('Unknown search engine');
    this.data.settings[k] = v; this.save('settings');
    return this.settings;
  }

  // ---- bookmarks
  get bookmarks() { return this.data.bookmarks; }
  isBookmarked(url) { return this.data.bookmarks.some(b => b.url === url); }
  toggleBookmark(url, title) {
    if (!/^https?:/i.test(url || '')) return false;
    const i = this.data.bookmarks.findIndex(b => b.url === url);
    if (i >= 0) this.data.bookmarks.splice(i, 1);
    else this.data.bookmarks.unshift({ url, title: String(title || url).slice(0, 200), at: Date.now() });
    this.save('bookmarks');
    return i < 0;
  }
  removeBookmark(url) { this.data.bookmarks = this.data.bookmarks.filter(b => b.url !== url); this.save('bookmarks'); }

  // ---- history
  visit(url, title) {
    if (!/^https?:/i.test(url || '')) return;
    const h = this.data.history;
    const last = h[0];
    if (last && last.url === url) { last.title = title || last.title; last.at = Date.now(); }
    else h.unshift({ url, title: String(title || '').slice(0, 200), at: Date.now() });
    if (h.length > MAX_HISTORY) h.length = MAX_HISTORY;
    this.save('history');
  }
  retitle(url, title) {
    const e = this.data.history.find(x => x.url === url);
    if (e && title) { e.title = String(title).slice(0, 200); this.save('history'); }
  }
  history(query = '', limit = 300) {
    const q = String(query).toLowerCase();
    return this.data.history.filter(e => !q || e.url.toLowerCase().includes(q) || (e.title || '').toLowerCase().includes(q)).slice(0, limit);
  }
  clearHistory() { this.data.history = []; this.save('history'); }
  /** The most visited sites (for the start page), by domain. */
  topSites(n = 8) {
    const count = new Map();
    for (const e of this.data.history) {
      let host; try { host = new URL(e.url).origin; } catch { continue; }
      const c = count.get(host) || { url: host + '/', title: e.title, n: 0 };
      c.n++; if (!c.title && e.title) c.title = e.title;
      count.set(host, c);
    }
    return [...count.values()].sort((a, b) => b.n - a.n).slice(0, n);
  }
  /** Address bar suggestions: bookmarks first, then history, matching what's typed. */
  suggest(text, n = 6) {
    const q = String(text || '').toLowerCase().trim();
    if (!q) return [];
    const seen = new Set(), out = [];
    for (const e of [...this.data.bookmarks.map(b => ({ ...b, bookmark: true })), ...this.data.history]) {
      if (seen.has(e.url)) continue;
      if (e.url.toLowerCase().replace(/^https?:\/\/(www\.)?/, '').startsWith(q) || (e.title || '').toLowerCase().includes(q)) { seen.add(e.url); out.push(e); }
      if (out.length >= n) break;
    }
    return out;
  }

  // ---- open tabs (restored at the next start)
  saveSession(urls) { this.data.session = urls.filter(u => /^https?:|^hearth:/.test(u)).slice(0, 30); this.save('session'); }
  get session() { return this.data.session; }
}

module.exports = { BrowserStore, toUrl, displayUrl, SEARCH, START_TILES };
