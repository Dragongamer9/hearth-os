'use strict';
// Hearth's on-screen keyboard engine (ui/osk/hearth-osk.js, docs/OSK.md): what typing with a controller, touch or the
// Ember Wheel puts in a field. The page parts are checked in tools/ui-check.js, Desktop Mode in desktop-test.yml.
const test = require('node:test');
const assert = require('node:assert');
const path = require('path');
global.HearthOSK = require(path.join(__dirname, '../../../ui/osk/hearth-osk.js'));
require(path.join(__dirname, '../../../ui/osk/words/en.js'));
require(path.join(__dirname, '../../../ui/osk/words/fr.js'));
const H = global.HearthOSK;

function kb(opts = {}) {
  H.configure({ settings: opts.settings || {}, learned: { learned: {}, next: {}, recentEmoji: [] }, save: null });
  const t = { text: '', caret: 0 }, keys = [];
  const e = H.createEngine(Object.assign({ adapter: H.modelAdapter(() => t, () => {}, k => keys.push(k)) }, opts));
  const find = ch => { const R = e.view().rows; for (let r = 0; r < R.length; r++) for (let c = 0; c < R[r].length; c++) { const k = R[r][c]; if ((k.t === 'ch' && k.v === ch) || k.t === ch) return [r, c]; } throw new Error('no ' + ch); };
  const tap = s => { for (const ch of s) { const [r, c] = find(ch); e.tap(r, c); } };
  const press = b => { e.handle(b, true, 1000); e.handle(b, false, 1010); };
  return { e, t, keys, tap, press, find };
}

test('the D-pad walks the keys and A types; X deletes, Y is a space', () => {
  const k = kb({ mode: 'search' });
  assert.deepStrictEqual(k.e.view().sel, { r: 1, c: 0 });   // q
  k.press('a'); k.press('right'); k.press('a'); k.press('y'); k.press('x'); k.press('down'); k.press('a');
  assert.strictEqual(k.t.text, 'qws');
});

test('capitals start sentences, two spaces type a period, LT is shift', () => {
  const k = kb();
  k.tap('hi'); k.press('y'); k.press('y'); k.tap('ok');
  assert.strictEqual(k.t.text, 'Hi. Ok');
  k.press('lt'); k.tap('a');
  assert.strictEqual(k.t.text, 'Hi. OkA');
});

test('suggestions come from the word list and learning; picking one adds its space', () => {
  const k = kb();
  k.tap('hel');
  assert.ok(k.e.view().suggestions.includes('Hello'), k.e.view().suggestions.join());
  k.e.pickSuggestion(k.e.view().suggestions.indexOf('Hello'));
  assert.strictEqual(k.t.text, 'Hello ');
  k.tap('.');
  assert.strictEqual(k.t.text, 'Hello. ');
  // a word typed twice is suggested
  for (let i = 0; i < 2; i++) { k.tap('zorbix'); k.press('y'); }
  k.tap('zor');
  assert.ok(k.e.view().suggestions.map(s => s.toLowerCase()).includes('zorbix'));
  H.clearLearned();
  assert.strictEqual(H.learnedCount(), 0);
});

test('no suggestions, learning or capitals in a password', () => {
  const k = kb({ mode: 'password' });
  k.tap('hel');
  assert.strictEqual(k.t.text, 'hel');
  assert.deepStrictEqual(k.e.view().suggestions, []);
  assert.strictEqual(H.learnedCount(), 0);
});

test('A held opens the accents, French first for French', () => {
  const k = kb({ settings: { layouts: ['fr'] } });
  const [r, c] = k.find('e'); k.e.state.sel = { r, c };
  k.e.handle('a', true, 1000); k.e.tick(1500);
  const pop = k.e.view().popup;
  assert.ok(pop && pop.alts[0] === 'É', JSON.stringify(pop));   // a capital: the sentence starts
  k.e.handle('a', false, 1600); k.press('right'); k.press('a');
  assert.strictEqual(k.t.text, 'È');
});

test('AZERTY, QWERTZ and the Nordic layouts, and Automatic follows the system', () => {
  for (const [id, row] of [['fr', 'azertyuiop'], ['de', 'qwertzuiopü'], ['sv', 'qwertyuiopå'], ['da', 'qwertyuiopå'], ['es', 'qwertyuiop']]) {
    const k = kb({ settings: { layouts: [id] } });
    assert.strictEqual(k.e.view().rows[2].map(x => x.v).join('').startsWith(row) || k.e.view().rows[1].map(x => x.v).join('').startsWith(row), true, id);
  }
  H.configure({ settings: {}, systemLayouts: ['de', 'us'] });
  assert.deepStrictEqual(H.activeLayouts().slice(0, 2), ['de', 'en-us']);
  H.configure({ systemLayouts: [] });
});

test('emoji, the clipboard panel and the number pad', () => {
  const k = kb();
  k.press('l3'); assert.strictEqual(k.e.view().panel, 'emoji');
  k.e.state.sel = { r: 1, c: 0 }; k.press('a');
  assert.strictEqual(k.t.text, '😀');
  k.press('x'); assert.strictEqual(k.t.text, '');      // one emoji is one character
  const n = kb({ mode: 'number' });
  assert.strictEqual(n.e.view().panel, 'num');
  n.tap('+'); n.tap('42');
  assert.strictEqual(n.t.text, '+42');
});

test('the Ember Wheel: the left stick picks a petal, A B X Y type its four', () => {
  const k = kb({ mode: 'search', settings: { fast: 'wheel' } });
  k.e.axes(0, -1, 0, 0);                // up: "abcd"
  k.press('y'); k.press('x'); k.press('b'); k.press('a');
  k.e.axes(1, 0, 0, 0);                 // right: "ijkl"
  k.press('y');
  k.e.axes(0, 0, 0, 0); k.press('y');   // centred: Y is a space again
  assert.strictEqual(k.t.text, 'abcdi ');
});

test('split sticks: LT and RT type what each stick points at', () => {
  const k = kb({ mode: 'search', settings: { fast: 'split' } });
  k.e.axes(-1, -0.15, 0.2, -0.15);     // the a row: far left, and right of the middle
  k.press('lt'); k.press('rt');
  assert.strictEqual(k.t.text, 'al');
});

test('a source with presses only (no releases) never sticks', () => {
  const k = kb({ mode: 'search' });
  k.e.handle('a'); k.e.handle('a'); k.e.handle('right'); k.e.handle('a');
  assert.strictEqual(k.t.text, 'qqw');
});

test('typing into another program: the keyboard keeps its own idea of the word', () => {
  H.configure({ settings: {}, learned: { learned: {}, next: {} } });
  const out = [];
  const e = H.createEngine({ host: 'mantel', adapter: { insert: t => out.push(t), deleteBack: n => out.push('⌫' + n), key: k => out.push('[' + k + ']') } });
  const R = e.view().rows; const at = ch => { for (let r = 0; r < R.length; r++) for (let c = 0; c < R[r].length; c++) if (R[r][c].v === ch) return [r, c]; };
  for (const ch of 'hel') e.tap(...at(ch));
  assert.ok(e.view().suggestions.includes('hello'));
  e.pickSuggestion(e.view().suggestions.indexOf('hello'));
  assert.deepStrictEqual(out, ['h', 'e', 'l', '⌫3', 'hello ']);
  e.handle('menu');
  assert.strictEqual(out.at(-1), '[enter]');
});
