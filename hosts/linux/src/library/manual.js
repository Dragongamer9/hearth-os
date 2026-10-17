'use strict';
// Games the user added themselves: a Linux program or script, an AppImage, a .desktop file,
// or a Windows .exe (run through Proton with umu-launcher, or Wine as a fallback).

const fs = require('node:fs');
const path = require('node:path');
const { has, launchDetached, readText } = require('../util');
const { apply } = require('./game-options');

/** Reads Name= and Exec= from a .desktop file's [Desktop Entry] section. */
function parseDesktop(file) {
  const text = readText(file) || '';
  const out = {};
  let inEntry = false;
  for (const line of text.split('\n')) {
    const t = line.trim();
    if (t.startsWith('[')) { inEntry = t === '[Desktop Entry]'; continue; }
    if (!inEntry) continue;
    const m = /^(Name|Exec|Path)=(.*)$/.exec(t);
    if (m && !(m[1] in out)) out[m[1]] = m[2];
  }
  return out;
}

/** Splits a .desktop Exec line into argv, dropping field codes like %f %U. */
function execArgv(exec) {
  const argv = [];
  let cur = '', q = false, any = false;
  for (let i = 0; i < exec.length; i++) {
    const c = exec[i];
    if (c === '"') { q = !q; any = true; continue; }
    if (c === '\\' && q && i + 1 < exec.length) { cur += exec[++i]; continue; }
    if (c === ' ' && !q) { if (any || cur) argv.push(cur); cur = ''; any = false; continue; }
    cur += c;
  }
  if (any || cur) argv.push(cur);
  return argv.filter(a => !/^%[fFuUdDnNickvm]$/.test(a)).map(a => a.replace(/%%/g, '%'));
}

function titleFor(file) {
  if (file.endsWith('.desktop')) { const d = parseDesktop(file); if (d.Name) return d.Name; }
  return path.basename(file).replace(/\.(exe|desktop|appimage|sh)$/i, '').replace(/[_.]+/g, ' ').trim();
}

const kindOf = file => /\.exe$/i.test(file) ? 'windows' : /\.desktop$/i.test(file) ? 'desktop' : 'native';

/** Launches a manual game with its settings (options). Returns the child process, when Nimbus started one it can watch. */
function launch(g, options = null) {
  const go = (cmd, args, opts = {}, proton = false) => {
    const r = apply(options, { cmd, args, env: opts.env || {} }, { proton });
    return launchDetached(r.cmd, r.args, { ...opts, env: { ...process.env, ...r.env } });
  };
  const file = g.exe;
  if (!fs.existsSync(file)) throw new Error(`File not found: ${file}`);
  const cwd = path.dirname(file);
  const args = g.args ? execArgv(g.args) : [];

  switch (kindOf(file)) {
    case 'desktop': {
      const d = parseDesktop(file);
      if (!d.Exec) throw new Error("That .desktop file has no Exec line");
      const [cmd, ...rest] = execArgv(d.Exec);
      return go(cmd, [...rest, ...args], { cwd: d.Path || cwd });
    }
    case 'windows':
      // umu-launcher runs Windows games with Proton outside Steam (downloads Proton on first use).
      if (has('umu-run')) return go('umu-run', [file, ...args], { cwd, env: { GAMEID: process.env.GAMEID || 'umu-default' } }, true);
      if (has('wine')) return go('wine', [file, ...args], { cwd });
      throw new Error('To run Windows games, install umu-launcher (recommended) or Wine');
    default:
      try { fs.accessSync(file, fs.constants.X_OK); }
      catch { throw new Error(`${path.basename(file)} isn't executable. Mark it as executable (chmod +x) and try again`); }
      return go(file, args, { cwd });
  }
}

module.exports = { launch, titleFor, parseDesktop, execArgv, kindOf };
