'use strict';
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { BrowserStore, toUrl, displayUrl } = require('../src/web/store');

test('the address bar: URLs, domains, local addresses and searches', () => {
  assert.strictEqual(toUrl('https://example.com/a'), 'https://example.com/a');
  assert.strictEqual(toUrl('example.com'), 'https://example.com');
  assert.strictEqual(toUrl('store.steampowered.com/app/1'), 'https://store.steampowered.com/app/1');
  assert.strictEqual(toUrl('localhost:8080'), 'http://localhost:8080');
  assert.strictEqual(toUrl('192.168.1.1'), 'http://192.168.1.1');
  assert.strictEqual(toUrl('best rpg games'), 'https://duckduckgo.com/?q=best%20rpg%20games');
  assert.strictEqual(toUrl('elden ring', 'google'), 'https://www.google.com/search?q=elden%20ring');
  assert.strictEqual(toUrl('node.js'), 'https://duckduckgo.com/?q=node.js');   // looks like a file, so search it
  assert.strictEqual(toUrl('bbc.co.uk'), 'https://bbc.co.uk');
  assert.strictEqual(toUrl(''), null);
  assert.strictEqual(toUrl('hearth://history'), 'hearth://history');
});

test('how URLs are shown', () => {
  assert.strictEqual(displayUrl('https://www.youtube.com/'), 'youtube.com');
  assert.strictEqual(displayUrl('http://localhost:3000/x'), 'http://localhost:3000/x');
  assert.strictEqual(displayUrl('hearth://start'), '');
});

test('bookmarks, history, suggestions and top sites are kept', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'hb-'));
  const s = new BrowserStore(dir);
  assert.strictEqual(s.toggleBookmark('https://www.protondb.com/', 'ProtonDB'), true);
  s.visit('https://www.youtube.com/watch?v=1', 'Video');
  s.visit('https://www.youtube.com/', 'YouTube');
  s.visit('https://www.youtube.com/', 'YouTube');   // same page again: one entry
  s.visit('hearth://start', 'Start');                // internal pages aren't history
  assert.strictEqual(s.history().length, 2);
  assert.deepStrictEqual(s.suggest('you').map(e => e.url)[0], 'https://www.youtube.com/');
  assert.strictEqual(s.suggest('proton')[0].bookmark, true);
  assert.strictEqual(s.topSites()[0].url, 'https://www.youtube.com/');
  s.set('search', 'brave');
  assert.throws(() => s.set('search', 'nope'));
  s.flush();
  const again = new BrowserStore(dir);
  assert.strictEqual(again.settings.search, 'brave');
  assert.strictEqual(again.isBookmarked('https://www.protondb.com/'), true);
  assert.strictEqual(again.history().length, 2);
});
