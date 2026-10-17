'use strict';
// Power limit (TDP) for handhelds, through PowerStation (org.shadowblip.PowerStation), the service Bazzite
// and other handheld distros ship for this. Talks to it with busctl; when it isn't there, the overlay
// simply doesn't show the control.

const { run, has } = require('./util');

const SVC = 'org.shadowblip.PowerStation';
const IFACE = 'org.shadowblip.GPU.Card.TDP';
let cardPath = null, checked = false;

async function card() {
  if (checked) return cardPath;
  checked = true;
  if (!has('busctl')) return null;
  const tree = await run('busctl', ['--system', '--list', 'tree', SVC], { timeout: 4000 }).catch(() => '');
  const paths = tree.split('\n').map(l => l.trim()).filter(l => /^\/org\/shadowblip\/Performance\/GPU\/card\d+$/.test(l));
  for (const p of paths) {
    const v = await run('busctl', ['--system', 'get-property', SVC, p, IFACE, 'TDP'], { timeout: 3000 }).catch(() => '');
    if (/^d\s/.test(v.trim())) { cardPath = p; break; }
  }
  return cardPath;
}

let cache = null, cachedAt = 0;
/** {supported, watts, min, max} (cached for a few seconds: the overlay asks every second) */
async function get() {
  if (cache && Date.now() - cachedAt < 5000) return cache;
  cache = await read(); cachedAt = Date.now();
  return cache;
}
async function read() {
  const p = await card();
  if (!p) return { supported: false };
  const v = await run('busctl', ['--system', 'get-property', SVC, p, IFACE, 'TDP'], { timeout: 3000 }).catch(() => '');
  const watts = parseFloat(v.trim().split(/\s+/)[1]);
  return { supported: Number.isFinite(watts), watts: Math.round(watts), min: 5, max: 30 };
}

async function set(watts) {
  const p = await card();
  if (!p) throw new Error("This device's power limit can't be changed here.");
  const w = Math.max(3, Math.min(40, Math.round(+watts)));
  await run('busctl', ['--system', 'set-property', SVC, p, IFACE, 'TDP', 'd', String(w)], { timeout: 4000 });
  cache = null;
  return get();
}

module.exports = { get, set };
