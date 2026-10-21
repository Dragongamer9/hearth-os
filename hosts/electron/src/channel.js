'use strict';
// The channel to the Hearth core (`hearth --core-stdio`): one JSON object a line, each with "t" (described in
// hosts/hearth-rs/src/winhost.rs). Pure helpers, so they can be tested without Electron.

/** One line out. */
function line(obj) { return JSON.stringify(obj) + '\n'; }

/** One line in: an object with "t", or null (blank, not JSON, or not ours: a stray print can't break the channel). */
function parse(text) {
  const s = String(text || '').trim();
  if (!s || s[0] !== '{') return null;
  try {
    const v = JSON.parse(s);
    return v && typeof v === 'object' && !Array.isArray(v) && typeof v.t === 'string' ? v : null;
  } catch { return null; }
}

/** Splits a stream of chunks into whole lines; returns a function to feed chunks to, calling onLine for each. */
function lineReader(onLine) {
  let buf = '';
  return chunk => {
    buf += chunk;
    let i;
    while ((i = buf.indexOf('\n')) >= 0) { const l = buf.slice(0, i); buf = buf.slice(i + 1); onLine(l); }
  };
}

/** Which windows a core message goes to: its "to" ("*": every one of ours that exists). */
function targets(msg, labels) {
  if (!msg || msg.t !== 'msg') return [];
  if (msg.to === '*') return labels.slice();
  return labels.includes(msg.to) ? [msg.to] : [];
}

/** Arguments for the core: Hearth's own (--windowed, --session, --page X...), not Chromium's (--ozone-platform...). */
const CHROMIUM_FLAGS = /^--(ozone-platform|enable-features|disable-features|no-sandbox|disable-gpu|use-gl|use-angle|enable-wayland-ime|in-process-gpu)(=|$)/;
function coreArgs(argv) {
  const out = ['--core-stdio'];
  for (const a of argv) if (typeof a === 'string' && a.startsWith('--') && !CHROMIUM_FLAGS.test(a) && a !== '--core-stdio') out.push(a);
  // A value after --page (Hearth Settings' page) goes along.
  const i = argv.indexOf('--page');
  if (i >= 0 && argv[i + 1] && !argv[i + 1].startsWith('--')) out.push(argv[i + 1]);
  return out;
}

/** nimbus-art:// addresses only (the core says which file each is; the window app reads it). */
function isArt(url) { return /^nimbus-art:\/\/(file|cache)\//.test(String(url || '')); }

module.exports = { line, parse, lineReader, targets, coreArgs, isArt };
