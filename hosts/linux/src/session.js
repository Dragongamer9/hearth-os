'use strict';
// "Make Nimbus my shell" on Linux: a "Nimbus" session on the login screen that runs Nimbus in gamescope.
// Installing it needs root, so it goes through pkexec (a password prompt from the desktop's polkit agent).

const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const { run, has, exists, DATA_DIR } = require('./util');

const SESSION_FILE = '/usr/share/wayland-sessions/nimbus.desktop';
const CONF_DIR = path.join(process.env.XDG_CONFIG_HOME || path.join(os.homedir(), '.config'), 'nimbus');
const APP_PATH_FILE = path.join(CONF_DIR, 'app-path');

/** What the session should start: the AppImage when packaged, else this Electron with this app folder. */
function appCommand(appRoot) {
  if (process.env.APPIMAGE) return process.env.APPIMAGE;
  // Development: a small launcher script so the session can start the source checkout.
  const launcher = path.join(DATA_DIR, 'nimbus-dev');
  fs.writeFileSync(launcher, `#!/bin/sh\nexec "${process.execPath}" "${appRoot}" "$@"\n`, { mode: 0o755 });
  return launcher;
}

function status() {
  let value = null;
  try { value = fs.readFileSync(APP_PATH_FILE, 'utf8').trim(); } catch {}
  return {
    running: process.env.NIMBUS_SESSION === '1',
    registered: exists(SESSION_FILE) && !!value,
    value,
    exe: process.env.APPIMAGE || process.execPath,
    gamescope: has('gamescope'),
  };
}

/** Copies the session scripts somewhere root can read (an AppImage's files are a user-only mount), then installs. */
function stageScripts(sessionDir) {
  const dst = path.join(DATA_DIR, 'session');
  fs.mkdirSync(dst, { recursive: true });
  for (const f of ['nimbus-session', 'nimbus-watchdog', 'nimbus.desktop', 'install-session.sh']) {
    fs.copyFileSync(path.join(sessionDir, f), path.join(dst, f));
    fs.chmodSync(path.join(dst, f), f.endsWith('.desktop') ? 0o644 : 0o755);
  }
  return dst;
}

async function register(sessionDir, appRoot) {
  if (!has('pkexec')) throw new Error('pkexec is missing. Run install-session.sh as root instead');
  const staged = stageScripts(sessionDir);
  fs.mkdirSync(CONF_DIR, { recursive: true });
  fs.writeFileSync(APP_PATH_FILE, appCommand(appRoot) + '\n');
  await run('pkexec', [path.join(staged, 'install-session.sh')], { timeout: 120000 });
  return status();
}

async function unregister(sessionDir) {
  try { fs.unlinkSync(APP_PATH_FILE); } catch {}
  if (exists(SESSION_FILE)) {
    const staged = stageScripts(sessionDir);
    await run('pkexec', [path.join(staged, 'install-session.sh'), '--remove'], { timeout: 120000 });
  }
  return status();
}

module.exports = { status, register, unregister };
