'use strict';
// Hearth's polkit rule (os/system_files/usr/share/polkit-1/rules.d/49-hearth-admin.rules), run with a stand-in
// for polkit: hearth-admin goes through without a password for the person at the device, except write-file
// (Hearth Text saving a system file), which always asks for an administrator's password.
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');

const RULES = path.join(__dirname, '..', '..', '..', 'os', 'system_files', 'usr', 'share', 'polkit-1', 'rules.d', '49-hearth-admin.rules');

function decide(details, subject) {
  const rules = [];
  const polkit = { addRule: f => rules.push(f), Result: { YES: 'yes', NO: 'no', AUTH_ADMIN: 'auth_admin', AUTH_ADMIN_KEEP: 'auth_admin_keep' } };
  new Function('polkit', fs.readFileSync(RULES, 'utf8'))(polkit);
  const action = { id: 'org.freedesktop.policykit.exec', lookup: k => details[k] };
  const subj = { local: true, active: true, isInGroup: g => g === 'wheel', ...subject };
  for (const r of rules) { const v = r(action, subj); if (v) return v; }
  return 'default';
}
const ADMIN = '/usr/libexec/nimbus/hearth-admin';

test('hearth-admin commands need no password for the person at the device', () => {
  assert.strictEqual(decide({ program: ADMIN, command_line: ADMIN + ' hostname deck' }), 'yes');
  assert.strictEqual(decide({ program: ADMIN, command_line: ADMIN + ' update' }), 'yes');
});

test('write-file always asks for an administrator password', () => {
  assert.strictEqual(decide({ program: ADMIN, command_line: ADMIN + ' write-file /etc/fstab' }), 'auth_admin');
  assert.strictEqual(decide({ program: ADMIN, command_line: ADMIN + ' write-file' }), 'auth_admin');
  // Not fooled by a file called write-file somewhere else on the line.
  assert.strictEqual(decide({ program: ADMIN, command_line: ADMIN + ' hostname write-file' }), 'yes');
});

test('other programs and remote or non-admin users are left to polkit', () => {
  assert.strictEqual(decide({ program: '/usr/bin/bash', command_line: '/usr/bin/bash' }), 'default');
  assert.strictEqual(decide({ program: ADMIN, command_line: ADMIN + ' hostname x' }, { local: false }), 'default');
  assert.strictEqual(decide({ program: ADMIN, command_line: ADMIN + ' hostname x' }, { isInGroup: () => false }), 'default');
});

test('Make a bootable USB: USB drives need no password, internal drives always ask', () => {
  const cl = a => ({ program: ADMIN, command_line: ADMIN + ' ' + a });
  assert.strictEqual(decide(cl('usb-raw /dev/sdb --expect-size 16008609792 --size 2000000')), 'yes');
  assert.strictEqual(decide(cl('usb-prepare /dev/sdb --expect-size 1 --scheme gpt --label WIN11')), 'yes');
  assert.strictEqual(decide(cl('usb-raw /dev/sda --expect-size 1 --allow-internal')), 'auth_admin');
  assert.strictEqual(decide(cl('usb-prepare /dev/sda --allow-internal --expect-size 1 --scheme gpt --label X')), 'auth_admin');
  // Only for usb-* commands, and a remote or non-admin user still gets nothing from Hearth's rule.
  assert.strictEqual(decide(cl('usb-raw /dev/sda --allow-internal'), { isInGroup: () => false }), 'auth_admin');
  assert.strictEqual(decide(cl('hostname x')), 'yes');
});
