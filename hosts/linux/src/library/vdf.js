'use strict';
// Minimal parser for Valve's text KeyValues (.vdf / .acf) format. Port of the Windows host's Vdf.cs.
// Keys are matched case-insensitively, like Steam does.

const fs = require('node:fs');

function parse(text) {
  const st = { s: text, i: 0 };
  return parseObject(st);
}

function load(file) {
  try { return parse(fs.readFileSync(file, 'utf8')); } catch { return null; }
}

/** Follows a key path (case-insensitive); returns undefined if any step is missing. */
function get(obj, ...keys) {
  let cur = obj;
  for (const k of keys) {
    if (!cur || typeof cur !== 'object') return undefined;
    const want = k.toLowerCase();
    const hit = Object.keys(cur).find(x => x.toLowerCase() === want);
    if (hit === undefined) return undefined;
    cur = cur[hit];
  }
  return cur;
}
const obj = (o, ...k) => { const v = get(o, ...k); return v && typeof v === 'object' ? v : undefined; };
const str = (o, ...k) => { const v = get(o, ...k); return typeof v === 'string' ? v : undefined; };

function parseObject(st) {
  const d = {};
  for (;;) {
    const key = next(st);
    if (!key || key.brace === '}') return d;
    if (key.brace === '{') continue; // stray brace
    const val = next(st);
    if (!val) return d;
    if (val.brace === '{') d[key.text] = parseObject(st);
    else if (val.brace === '}') return d;
    else d[key.text] = val.text;
  }
}

function next(st) {
  const s = st.s;
  while (st.i < s.length) {
    const c = s[st.i];
    if (/\s/.test(c)) { st.i++; continue; }
    if (c === '/' && s[st.i + 1] === '/') { while (st.i < s.length && s[st.i] !== '\n') st.i++; continue; }
    if (c === '{' || c === '}') { st.i++; return { text: '', brace: c }; }
    if (c === '"') {
      let out = '';
      st.i++;
      while (st.i < s.length && s[st.i] !== '"') {
        if (s[st.i] === '\\' && st.i + 1 < s.length) {
          const n = s[st.i + 1];
          out += n === 'n' ? '\n' : n === 't' ? '\t' : n;
          st.i += 2;
        } else out += s[st.i++];
      }
      st.i++;
      return { text: out, brace: '' };
    }
    // Unquoted token
    let out = '';
    while (st.i < s.length && !/[\s{}"]/.test(s[st.i])) out += s[st.i++];
    return { text: out, brace: '' };
  }
  return null;
}

module.exports = { parse, load, get, obj, str };
