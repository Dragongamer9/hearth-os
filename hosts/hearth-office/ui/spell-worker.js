// Hearth Write's spell checker, off the window's thread: Typo.js with the Hunspell en_US dictionary
// (vendor/en_US.aff and .dic, from the dictionary-en package). Messages: {id, type: 'check', data: [words]} →
// {word: true|false}; {id, type: 'suggest', data: word} → [suggestions].
'use strict';
let dict = null;
(async () => {
  try {
    importScripts('vendor/typo.js');
    const [aff, dic] = await Promise.all([fetch('vendor/en_US.aff').then((r) => r.text()), fetch('vendor/en_US.dic').then((r) => r.text())]);
    dict = new Typo('en_US', aff, dic, { platform: 'any' });
    postMessage({ type: 'ready' });
  } catch (e) { postMessage({ type: 'error', error: String(e && e.message || e) }); }
})();
const ok = (w) => dict.check(w) || dict.check(w.toLowerCase()) || (w.endsWith("'s") && dict.check(w.slice(0, -2)));
onmessage = (e) => {
  const { id, type, data } = e.data;
  if (!dict) { postMessage({ id, result: null }); return; }
  if (type === 'check') {
    const out = {};
    for (const w of data) { try { out[w] = ok(w); } catch { out[w] = true; } }
    postMessage({ id, result: out });
  } else if (type === 'suggest') {
    let s = [];
    try { s = dict.suggest(data, 8); } catch {}
    postMessage({ id, result: s });
  }
};
