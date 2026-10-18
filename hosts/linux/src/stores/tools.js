'use strict';
// The open-source command-line tools Nimbus uses for Epic, GOG and Windows games on Linux, same as on Windows:
//   legendary  (Epic)                       github.com/legendary-gl/legendary
//   gogdl      (GOG, from Heroic)           github.com/Heroic-Games-Launcher/heroic-gogdl
//   umu-run    (runs Windows games with Proton outside Steam)  github.com/Open-Wine-Components/umu-launcher
// Downloaded from their GitHub releases the first time they're needed, checked against a pinned SHA-256,
// kept in ~/.local/share/nimbus/tools. An umu-run already installed on the system is used instead of downloading.

const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { spawn, execFile } = require('node:child_process');
const { DATA_DIR, has, log } = require('../util');

const TOOLS_DIR = path.join(DATA_DIR, 'tools');

const TOOLS = {
  legendary: {
    name: 'legendary', file: 'legendary',
    url: 'https://github.com/legendary-gl/legendary/releases/download/0.21.1/legendary_linux_x64',
    sha256: 'dbed33bbe96031e65858233e4badf0a1d401bb6cb0cc995d1237cf3a0c826a45',
  },
  gogdl: {
    name: 'gogdl', file: 'gogdl',
    url: 'https://github.com/Heroic-Games-Launcher/heroic-gogdl/releases/download/v1.3.0/gogdl_linux_x86_64',
    sha256: 'cba013d42767c808237c437335ab1d56f58405d07e8f37b3324d264ea5c49655',
  },
  // A tar holding umu/umu-run (a Python zipapp; needs python3, which every desktop distro has).
  umu: {
    name: 'umu-launcher', file: 'umu/umu-run', archive: 'tar',
    url: 'https://github.com/Open-Wine-Components/umu-launcher/releases/download/1.4.4/umu-launcher-1.4.4-zipapp.tar',
    sha256: 'eb590691841f7fad3fc3ad8fd5db4ccb87849fe7948e62b28ece7a4ee48cc851',
    system: 'umu-run',
  },
};

const pathOf = t => path.join(TOOLS_DIR, t.file);
const isReady = t => (t.system && has(t.system)) || fs.existsSync(pathOf(t));

let downloading = Promise.resolve();

/** Makes sure the tool is downloaded and intact. Resolves with the program to run. */
function ensure(t, progress) {
  if (t.system && has(t.system)) return Promise.resolve(t.system);
  if (fs.existsSync(pathOf(t))) return Promise.resolve(pathOf(t));
  // One download at a time.
  const job = downloading.then(() => download(t, progress));
  downloading = job.catch(() => {});
  return job;
}

async function download(t, progress) {
  const dst = pathOf(t);
  if (fs.existsSync(dst)) return dst;
  fs.mkdirSync(TOOLS_DIR, { recursive: true });
  const tmp = path.join(TOOLS_DIR, path.basename(t.url) + '.download');
  const r = await fetch(t.url, { headers: { 'user-agent': 'Nimbus' } });
  if (!r.ok || !r.body) throw new Error(`Couldn't download ${t.name} (HTTP ${r.status})`);
  const total = +r.headers.get('content-length') || 0;
  const hash = crypto.createHash('sha256');
  const out = fs.createWriteStream(tmp);
  let done = 0;
  for await (const chunk of r.body) {
    hash.update(chunk);
    if (!out.write(chunk)) await new Promise(res => out.once('drain', res));
    done += chunk.length;
    if (progress && total) progress(Math.min(1, done / total));
  }
  await new Promise((res, rej) => out.end(e => (e ? rej(e) : res())));
  if (hash.digest('hex') !== t.sha256) {
    fs.rmSync(tmp, { force: true });
    throw new Error(`The download of ${t.name} didn't match its checksum, so Hearth didn't use it. Try again later.`);
  }
  if (t.archive === 'tar') {
    await new Promise((res, rej) => execFile('tar', ['-xf', tmp, '-C', TOOLS_DIR], e => (e ? rej(e) : res())));
    fs.rmSync(tmp, { force: true });
    if (!fs.existsSync(dst)) throw new Error(`${t.name}'s download didn't contain ${t.file}`);
  } else fs.renameSync(tmp, dst);
  fs.chmodSync(dst, 0o755);
  log(`tools: downloaded ${t.name}`);
  return dst;
}

/** The last meaningful error line of a tool's output, for showing to the user. */
function problem({ out = '', err = '', code }) {
  const lines = (err + '\n' + out).split('\n').map(l => l.trim()).filter(Boolean);
  let line = [...lines].reverse().find(l => /ERROR|CRITICAL/.test(l)) || lines[lines.length - 1] || `exit code ${code}`;
  // "[cli] ERROR: Login failed" → "Login failed"
  const i = line.indexOf(': ');
  if (line.startsWith('[') && i > 0) line = line.slice(i + 2);
  return line;
}

/**
 * Runs a tool and collects its output. onLine sees every line from stdout and stderr as it arrives
 * (both tools log progress to stderr). signal (AbortSignal) cancels it; timeoutMs gives up.
 */
async function run(t, args, { env = {}, onLine, signal, timeoutMs } = {}) {
  const exe = await ensure(t);
  return new Promise((resolve, reject) => {
    const p = spawn(exe, args, {
      cwd: TOOLS_DIR,
      env: { ...process.env, PYTHONIOENCODING: 'utf-8', PYTHONUNBUFFERED: '1', ...env },
      stdio: ['ignore', 'pipe', 'pipe'],   // nothing will ever answer a prompt: make it fail instead of hang
      detached: true,                      // its own process group, so cancel stops everything it started
    });
    let out = '', err = '', bufO = '', bufE = '';
    const feed = (which, chunk) => {
      const s = chunk.toString('utf8');
      if (which === 'o') { out += s; bufO += s; } else { err += s; bufE += s; }
      let buf = which === 'o' ? bufO : bufE, nl;
      while ((nl = buf.search(/[\r\n]/)) >= 0) { const line = buf.slice(0, nl); buf = buf.slice(nl + 1); if (line && onLine) onLine(line); }
      if (which === 'o') bufO = buf; else bufE = buf;
    };
    p.stdout.on('data', c => feed('o', c));
    p.stderr.on('data', c => feed('e', c));
    const kill = () => { try { process.kill(-p.pid, 'SIGTERM'); } catch {} setTimeout(() => { try { process.kill(-p.pid, 'SIGKILL'); } catch {} }, 3000); };
    let timedOut = false, aborted = false;
    const timer = timeoutMs ? setTimeout(() => { timedOut = true; kill(); }, timeoutMs) : null;
    const onAbort = () => { aborted = true; kill(); };
    if (signal) { if (signal.aborted) onAbort(); else signal.addEventListener('abort', onAbort, { once: true }); }
    p.on('error', e => { clearTimeout(timer); reject(e); });
    p.on('close', code => {
      clearTimeout(timer);
      if (signal) signal.removeEventListener('abort', onAbort);
      if (aborted) { const e = new Error('cancelled'); e.cancelled = true; return reject(e); }
      if (timedOut) return reject(new Error(`${t.name} took too long to respond.`));
      resolve({ code, out, err });
    });
  });
}

module.exports = { TOOLS, ensure, run, problem, isReady, pathOf, TOOLS_DIR };
