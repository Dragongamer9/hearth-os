'use strict';
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { Profiles, steamRegistryText, steamLoginUsersText, steamAccounts } = require('../src/profiles');

const w = (f, t) => { fs.mkdirSync(path.dirname(f), { recursive: true }); fs.writeFileSync(f, t); };
const r = f => fs.readFileSync(f, 'utf8');

test('switching profiles swaps each person\'s sign-ins and library, and keeps shared things', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'hp-'));
  w(path.join(dir, 'library.json'), JSON.stringify({ favorites: ['steam:1'], manual: [{ id: 'm1' }], profile: { name: 'Dad' } }));
  w(path.join(dir, 'legendary', 'user.json'), 'dad-epic');
  w(path.join(dir, 'legendary', 'installed.json'), 'shared-installs');
  w(path.join(dir, 'gog', 'auth.json'), 'dad-gog');
  w(path.join(dir, 'accounts', 'xbox.bin'), 'dad-xbox');
  w(path.join(dir, 'cache', 'epic-owned.json'), 'dad-owned');
  const p = new Profiles(dir);
  assert.strictEqual(p.active, 'main');
  const mom = p.create({ name: 'Mom' });
  assert.throws(() => p.create({ name: '  ' }));
  p.switchTo(mom.id);
  assert.strictEqual(p.active, mom.id);
  const lib = JSON.parse(r(path.join(dir, 'library.json')));
  assert.deepStrictEqual(lib.favorites || [], []);
  assert.strictEqual(lib.profile.name, 'Mom');
  assert.deepStrictEqual(lib.manual.map(g => g.id), ['m1']);               // manual games are everyone's
  assert.ok(!fs.existsSync(path.join(dir, 'legendary', 'user.json')));      // Mom isn't signed in to Epic yet
  assert.strictEqual(r(path.join(dir, 'legendary', 'installed.json')), 'shared-installs');
  assert.ok(!fs.existsSync(path.join(dir, 'cache', 'epic-owned.json')));    // Dad's owned list is gone
  w(path.join(dir, 'gog', 'auth.json'), 'mom-gog');
  // and back
  new Profiles(dir).switchTo('main');
  assert.strictEqual(r(path.join(dir, 'legendary', 'user.json')), 'dad-epic');
  assert.strictEqual(r(path.join(dir, 'gog', 'auth.json')), 'dad-gog');
  assert.strictEqual(r(path.join(dir, 'accounts', 'xbox.bin')), 'dad-xbox');
  assert.deepStrictEqual(JSON.parse(r(path.join(dir, 'library.json'))).favorites, ['steam:1']);
  const again = new Profiles(dir);
  again.switchTo(mom.id);
  assert.strictEqual(r(path.join(dir, 'gog', 'auth.json')), 'mom-gog');     // Mom's own sign-in came back
  assert.throws(() => again.remove(mom.id));                                // can't remove the one in use
});

const REG = `"Registry"\n{\n\t"HKCU"\n\t{\n\t\t"Software"\n\t\t{\n\t\t\t"Valve"\n\t\t\t{\n\t\t\t\t"Steam"\n\t\t\t\t{\n\t\t\t\t\t"AutoLoginUser"\t\t"dad"\n\t\t\t\t\t"RememberPassword"\t\t"0"\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n}\n`;
const USERS = `"users"\n{\n\t"76561198000000001"\n\t{\n\t\t"AccountName"\t\t"dad"\n\t\t"PersonaName"\t\t"Dad"\n\t\t"MostRecent"\t\t"1"\n\t}\n\t"76561198000000002"\n\t{\n\t\t"AccountName"\t\t"mom"\n\t\t"PersonaName"\t\t"Mom"\n\t\t"MostRecent"\t\t"0"\n\t}\n}\n`;

test('Steam signs in as the profile\'s account next time', () => {
  const reg = steamRegistryText(REG, 'mom');
  assert.match(reg, /"AutoLoginUser"\s+"mom"/);
  assert.match(reg, /"RememberPassword"\s+"1"/);
  const users = steamLoginUsersText(USERS, 'mom');
  assert.match(users, /"AccountName"\s+"mom"[\s\S]*"MostRecent"\s+"1"/);
  assert.match(users, /"AccountName"\s+"dad"[^}]*"MostRecent"\s+"0"/);
  assert.match(users, /"AllowAutoLogin"\s+"1"/);
  assert.deepStrictEqual(steamAccounts(USERS), [{ account: 'dad', name: 'Dad' }, { account: 'mom', name: 'Mom' }]);
  assert.match(steamRegistryText('"Steam"\n{\n}', 'x'), /"AutoLoginUser"\s+"x"/);
});

test('parental controls: PIN, daily time and allowed games', () => {
  const fs = require('node:fs'), os = require('node:os'), path = require('node:path');
  const { Profiles } = require('../src/profiles');
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'pc-'));
  const ps = new Profiles(dir);
  const kid = ps.create({ name: 'Sam' });
  assert.throws(() => ps.setLimits(kid.id, { minutesPerDay: 60 }), /PIN first/);
  assert.throws(() => ps.setPin('12'), /4 to 8/);
  ps.setPin('2468');
  assert.ok(ps.checkPin('2468'));
  assert.ok(!ps.checkPin('1111'));
  ps.setLimits(kid.id, { minutesPerDay: 60, allowed: ['steam:1', 'steam:1', 'gog:2'] });
  assert.deepStrictEqual(ps.limits(kid.id), { minutesPerDay: 60, allowed: ['steam:1', 'gog:2'] });
  assert.ok(!ps.restricted());          // the main profile has no limits
  assert.ok(ps.canPlay('anything').ok);
  ps.switchTo(kid.id);
  assert.ok(ps.restricted());
  const day = new Date(2026, 9, 18, 15);
  assert.strictEqual(ps.canPlay('xbox:9', { now: day }).ok, false);
  assert.deepStrictEqual(ps.canPlay('steam:1', { now: day }), { ok: true, minutesLeft: 60 });
  ps.addPlayed(kid.id, 45, day);
  assert.strictEqual(ps.canPlay('steam:1', { now: day, sessionMinutes: 10 }).minutesLeft, 5);
  ps.addPlayed(kid.id, 15, day);
  assert.strictEqual(ps.canPlay('steam:1', { now: day }).ok, false);
  assert.strictEqual(ps.canPlay('steam:1', { now: new Date(2026, 9, 19, 9) }).minutesLeft, 60);   // new day
  // kept across restarts
  const again = new Profiles(dir);
  assert.ok(again.checkPin('2468'));
  assert.strictEqual(again.playedToday(kid.id, day), 60);
  again.setPin('');
  assert.ok(!again.restricted(kid.id));
});
