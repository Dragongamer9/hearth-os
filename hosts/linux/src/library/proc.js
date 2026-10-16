'use strict';
// Reading /proc to tell whether a game is running, and to stop it.

const fs = require('node:fs');
const path = require('node:path');

/** One snapshot of every process: pid, parent, command line and working folder. */
function snapshot(procRoot = '/proc') {
  const out = [];
  let pids;
  try { pids = fs.readdirSync(procRoot).filter(p => /^\d+$/.test(p)); } catch { return out; }
  for (const pid of pids) {
    try {
      const stat = fs.readFileSync(path.join(procRoot, pid, 'stat'), 'utf8');
      // "pid (comm with spaces) S ppid ..." — the comm can contain ")", so split at the last one.
      const ppid = +stat.slice(stat.lastIndexOf(')') + 2).split(' ')[1];
      const cmdline = fs.readFileSync(path.join(procRoot, pid, 'cmdline'), 'utf8').split('\0').filter(Boolean);
      let cwd = null;
      try { cwd = fs.readlinkSync(path.join(procRoot, pid, 'cwd')); } catch { /* other users' processes */ }
      out.push({ pid: +pid, ppid, cmdline, cwd });
    } catch { /* process exited while we looked */ }
  }
  return out;
}

const under = (p, dir) => !!p && !!dir && (p === dir || p.startsWith(dir.endsWith('/') ? dir : dir + '/'));

/**
 * Processes that belong to a game. `track` is one of:
 *   {steamAppId}  Steam's launcher wrapper (reaper) has "AppId=<id>" on its command line
 *   {dirs}        anything started from, or running in, the game's install folder (native or Wine)
 *   {pid}         a process Nimbus started itself
 * Children of matching processes count too (Proton, Wine, launchers).
 */
function gameProcesses(track, procs = snapshot()) {
  if (!track) return [];
  const roots = new Set();
  for (const p of procs) {
    if (track.pid && p.pid === track.pid) roots.add(p.pid);
    if (track.steamAppId && p.cmdline.includes(`AppId=${track.steamAppId}`)) roots.add(p.pid);
    if (track.dirs && track.dirs.length) {
      // The program can be the first argument or, under Wine/Proton, a later one written as Z:\home\...;
      // turn those back into Linux paths for the comparison.
      const paths = p.cmdline.slice(0, 4).map(a => (/^[A-Za-z]:\\/.test(a) ? a.slice(2).replace(/\\/g, '/') : a));
      if (track.dirs.some(d => under(p.cwd, d) || paths.some(a => under(a, d)))) roots.add(p.pid);
    }
  }
  return withDescendants(roots, procs);
}

function withDescendants(roots, procs) {
  const kids = new Map();
  for (const p of procs) { if (!kids.has(p.ppid)) kids.set(p.ppid, []); kids.get(p.ppid).push(p.pid); }
  const all = new Set(roots);
  const queue = [...roots];
  while (queue.length) for (const k of kids.get(queue.pop()) || []) if (!all.has(k)) { all.add(k); queue.push(k); }
  all.delete(process.pid); // never ourselves
  return [...all];
}

function alive(pid) { try { process.kill(pid, 0); return true; } catch { return false; } }

/** SIGTERM, then SIGKILL whatever is still there after `graceMs`. Resolves true if everything stopped. */
async function stop(pids, graceMs = 5000) {
  for (const pid of pids) { try { process.kill(pid, 'SIGTERM'); } catch {} }
  const until = Date.now() + graceMs;
  while (Date.now() < until && pids.some(alive)) await new Promise(r => setTimeout(r, 250));
  for (const pid of pids.filter(alive)) { try { process.kill(pid, 'SIGKILL'); } catch {} }
  await new Promise(r => setTimeout(r, 300));
  return !pids.some(alive);
}

module.exports = { snapshot, gameProcesses, withDescendants, stop, under };
