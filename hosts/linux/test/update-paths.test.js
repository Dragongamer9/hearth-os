'use strict';
// Every kind of install must be able to apply an update, or people get stuck on an old version.
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const { EventEmitter } = require('node:events');
const { Updates } = require('../src/updates');
const { Bridge } = require('../src/bridge');

const UI = fs.readFileSync(path.join(__dirname, '..', '..', '..', 'ui', 'index.html'), 'utf8');

test('the AppImage installs its own updates', () => {
  assert.strictEqual(new Updates({ version: '0.0.10', appImage: '/home/me/Hearth.AppImage' }).status().canInstallHere, true);
});

test('Hearth OS: the Updates page offers Update now, and it runs the system update', async () => {
  // UI: on the OS install the Updates page is the system section, which has the Update now button.
  assert.match(UI, /if\(LINUX && HOSTINFO\.system\) return pageSystemUpdate\(\);/);
  const page = UI.slice(UI.indexOf('function pageSystemUpdate'), UI.indexOf('// ---------- Settings › Shell'));
  assert.match(page, /data-act="sysUpdInstall"/);
  assert.match(page, /data-act="pwr" data-p="restart"/);
  assert.match(UI, /case 'sysUpdInstall': return call\('system\.update\.install'\)/);

  // Host: the call reaches hearth-admin's update command.
  const calls = [];
  const admin = Object.assign(new EventEmitter(), {
    updateStatus: () => ({ state: 'updating' }),
    installUpdate: () => { calls.push('update'); return Promise.resolve(); },
  });
  const library = Object.assign(new EventEmitter(), { accounts: new EventEmitter(), store: { data: {} } });
  const b = new Bridge({ post() {}, library, updates: new Updates({ version: '0.0.10' }), platform: { admin, info: () => ({}) } });
  clearInterval(b.statusTimer);
  await b.dispatch('system.update.install', {});
  assert.deepStrictEqual(calls, ['update']);
});

test('Windows: the updater installs releases (UpdateService) and the UI shows its download button', () => {
  const svc = fs.readFileSync(path.join(__dirname, '..', '..', 'windows', 'Nimbus.Host', 'Updates', 'UpdateService.cs'), 'utf8');
  assert.match(svc, /Nimbus-win-x64\.zip/);
  assert.match(UI, /data-act="updDownload"/);
  assert.match(UI, /data-act="updInstall"/);
});
