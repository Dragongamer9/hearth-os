'use strict';
const test = require('node:test');
const assert = require('node:assert');
const ch = require('../src/channel');

test('lines out and in', () => {
  assert.strictEqual(ch.line({ t: 'page', win: 'main' }), '{"t":"page","win":"main"}\n');
  assert.deepStrictEqual(ch.parse('{"t":"msg","to":"main","msg":{}}'), { t: 'msg', to: 'main', msg: {} });
  assert.strictEqual(ch.parse('Gtk-WARNING: something'), null, 'stray output is ignored');
  assert.strictEqual(ch.parse('{"no":1}'), null);
  assert.strictEqual(ch.parse('[1]'), null);
  assert.strictEqual(ch.parse(''), null);
});

test('whole lines from chunks', () => {
  const got = [];
  const feed = ch.lineReader(l => got.push(l));
  feed('{"t":"a"}\n{"t":');
  assert.deepStrictEqual(got, ['{"t":"a"}']);
  feed('"b"}\n');
  assert.deepStrictEqual(got, ['{"t":"a"}', '{"t":"b"}']);
});

test('where messages go', () => {
  assert.deepStrictEqual(ch.targets({ t: 'msg', to: 'main' }, ['main', 'overlay']), ['main']);
  assert.deepStrictEqual(ch.targets({ t: 'msg', to: '*' }, ['main', 'overlay']), ['main', 'overlay']);
  assert.deepStrictEqual(ch.targets({ t: 'msg', to: 'overlay' }, ['main']), [], 'no window, nobody');
  assert.deepStrictEqual(ch.targets({ t: 'do' }, ['main']), []);
});

test("the core gets Hearth's arguments, not Chromium's", () => {
  assert.deepStrictEqual(ch.coreArgs(['--session', '--ozone-platform=x11', '--windowed']), ['--core-stdio', '--session', '--windowed']);
  assert.deepStrictEqual(ch.coreArgs(['--page', 'hhd']), ['--core-stdio', '--page', 'hhd']);
  assert.deepStrictEqual(ch.coreArgs(['--core-stdio', 'stray']), ['--core-stdio']);
});

test('art addresses', () => {
  assert.ok(ch.isArt('nimbus-art://cache/abc'));
  assert.ok(ch.isArt('nimbus-art://file//home/me/a.png'));
  assert.ok(!ch.isArt('https://example.com/a.png'));
});
