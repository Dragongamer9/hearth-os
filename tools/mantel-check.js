// Release check for Mantel on Windows (Ember): every part of ui/mantel.html, Hearth Settings (mantel-settings.html)
// and Apps (mantel-apps.html) must load and draw without a script error, and the taskbar, launcher, menus, clock
// panel, clipboard history, dialogs and desktop icons must do what Hearth OS's Mantel does. Each page gets a stand-in
// for Ember's WebView2 host that answers bridge calls and records the messages the page sends.
//   npm i --no-save playwright@1.48.2 && npx playwright install chromium && node tools/mantel-check.js
//   SHOTS=<folder> also saves a screenshot of each piece there (ember-<piece>.png).
const { chromium } = require('playwright');
const path = require('path');

const UI = 'file://' + path.resolve(__dirname, '../ui');
const SHOTS = process.env.SHOTS || '';

// The fake host: chrome.webview.postMessage records; {type:'call'} gets an answer from window.__answers (or null).
const FAKE_HOST = () => {
  const listeners = [];
  window.__sent = [];
  window.__answers = window.__answers || {};
  window.__push = (name, data) => listeners.forEach(l => l({ data: { type: 'event', name, data } }));
  window.chrome = { webview: {
    postMessage(m) {
      window.__sent.push(m);
      if (m && m.type === 'call') {
        const a = window.__answers[m.method];
        if (a === '__wait') return;   // a call that's still running (an install)
        const value = typeof a === 'function' ? a(m.args) : a === undefined ? null : a;
        setTimeout(() => listeners.forEach(l => l({ data: { type: 'result', id: m.id, ok: true, value } })), 5);
      }
    },
    addEventListener(type, fn) { if (type === 'message') listeners.push(fn); },
  } };
};

const WINDOWS = [
  { id: 1, title: 'Steam', app: 'steam', key: 'valve.steam', pinId: 'Valve.Steam', name: 'Steam', active: true },
  { id: 2, title: 'Downloads - File Explorer', app: 'explorer', key: 'microsoft.windows.explorer', pinId: 'Microsoft.Windows.Explorer', name: 'File Explorer' },
  { id: 3, title: 'Mods - File Explorer', app: 'explorer', key: 'microsoft.windows.explorer', pinId: 'Microsoft.Windows.Explorer', name: 'File Explorer' },
];
const PINS = [{ id: 'Valve.Steam', name: 'Steam' }, { id: 'Discord.Discord', name: 'Discord' }];

(async () => {
  const browser = await chromium.launch();
  const errors = [];
  const open = async (file, w, h, answers) => {
    const page = await browser.newPage({ viewport: { width: w, height: h } });
    page.on('pageerror', e => errors.push(`${file}: ${e.message}`));
    page.on('console', m => { if (m.type() === 'error' && !/Failed to load resource|net::ERR/.test(m.text())) errors.push(`${file}: console: ${m.text()}`); });
    await page.addInitScript(a => { window.__answers = a; }, answers || {});
    await page.addInitScript(FAKE_HOST);
    await page.goto(`${UI}/${file}`);
    await page.waitForTimeout(300);
    return page;
  };
  const sent = (page, action) => page.evaluate(a => window.__sent.filter(m => m.type === 'mantel' && (!a || m.action === a)), action);
  const shot = async (page, name) => { if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `ember-${name}.png`) }); };
  const check = async (what, fn) => { try { await fn(); } catch (e) { errors.push(`${what}: ${e.message.split('\n')[0]}`); } };
  const expect = (ok, msg) => { if (!ok) throw new Error(msg); };
  // Keys Playwright doesn't have (F23 / F24: Ember types them for X / Y on the controller in its own windows).
  const press = (page, key) => page.evaluate(k => document.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true })), key);

  // ---------------------------------------------------------------- the taskbar
  await check('taskbar', async () => {
    const p = await open('mantel.html?part=bar', 1280, 60, { 'sys.status': { wifi: { available: true, on: true, ssid: 'Home' }, audio: { volume: 60, muted: false }, battery: { present: true, pct: 80 } } });
    await p.evaluate(([pins, wins]) => {
      __push('mantel.state', { pins, full: false, wallpaper: '', clock24: false, dnd: false });
      __push('mantel.windows', wins);
      __push('mantel.desks', [{ id: 'a', name: 'Desktop 1', active: true }, { id: 'b', name: 'Desktop 2', active: false }]);
      __push('mantel.notes', { history: [{ key: 1, app: 'Ember', summary: 'Hi', body: '', time: Date.now() }], unseen: 1, dnd: false });
    }, [PINS, WINDOWS]);
    await p.waitForTimeout(100);
    // Pinned first, one button per app: Steam (pinned, running, in front), Discord (pinned), File Explorer (2 windows).
    const tasks = await p.$$eval('.wins .bb', bs => bs.map(b => ({ title: b.title, cls: b.className })));
    expect(tasks.length === 3, `expected 3 app buttons, got ${tasks.length}`);
    expect(tasks[0].title === 'Steam' && /active/.test(tasks[0].cls) && /pinned/.test(tasks[0].cls), 'Steam should be first, pinned and lit');
    expect(tasks[1].title === 'Discord' && !/run/.test(tasks[1].cls), 'Discord should be pinned and not running');
    expect(/File Explorer \(2 windows\)/.test(tasks[2].title) && /many/.test(tasks[2].cls), 'File Explorer should share one button for its 2 windows');
    expect((await p.$$('.desks .dk')).length === 2, 'the desktop switcher should show 2 desktops');
    expect(await p.$('.bell .dot'), 'the bell should show an unseen dot');
    await shot(p, 'taskbar');
    // Right-click on a running app: its menu with Unpin, Move to desktop, Maximize, Close all.
    await p.click('.wins .bb:nth-child(3)', { button: 'right' });
    const m = (await sent(p, 'menu')).pop();
    expect(m && m.above, 'the taskbar menu opens upwards');
    const labels = m.items.filter(i => !i.separator).map(i => i.label);
    for (const l of ['New window', 'Pin to taskbar', 'Move to desktop', 'Maximize', 'Close all 2 windows']) expect(labels.includes(l), `menu lacks "${l}" (${labels.join(', ')})`);
    // Choosing Pin to taskbar sends the pin to Ember.
    const pinItem = m.items.find(i => i.label === 'Pin to taskbar');
    await p.evaluate(id => __push('mantel.menuPick', { id }), pinItem.id);
    await p.waitForTimeout(50);
    const pin = (await sent(p, 'pin')).pop();
    expect(pin && pin.on === true && pin.id === 'Microsoft.Windows.Explorer', 'Pin to taskbar should pin File Explorer');
    // Right-click on a pinned app offers Unpin.
    await p.click('.wins .bb:nth-child(1)', { button: 'right' });
    const m2 = (await sent(p, 'menu')).pop();
    expect(m2.items.some(i => i.label === 'Unpin from taskbar'), 'a pinned app offers Unpin from taskbar');
    // Middle click closes the window in front; a click on a pinned app that isn't running opens it.
    await p.click('.wins .bb:nth-child(2)');
    expect((await sent(p, 'launch')).some(x => x.id === 'Discord.Discord'), 'clicking a pin that isn\'t running opens it');
    await p.click('.desks .dk:nth-child(2)');
    expect(await p.evaluate(() => __sent.some(m => m.type === 'call' && m.method === 'vdesk.switch' && m.args.index === 1)), 'the desktop switcher switches');
    await p.evaluate(() => __push('mantel.state', { pins: [], clock24: true }));
    expect(/^\d\d:\d\d$/.test(await p.$eval('.bclock b', b => b.textContent)), 'the clock follows 24-hour');
    await p.close();
  });

  // ---------------------------------------------------------------- the launcher: Open / Pin from right-click, hold, Menu key and X
  await check('launcher', async () => {
    const p = await open('mantel.html?part=launcher', 880, 640);
    await p.evaluate(pins => {
      __push('mantel.state', { pins });
      __push('mantel.apps', [{ id: 'Valve.Steam', name: 'Steam' }, { id: 'Discord.Discord', name: 'Discord' }, { id: 'Mozilla.Firefox', name: 'Firefox' }]);
      __push('mantel.opened', {});
    }, PINS);
    await p.waitForTimeout(100);
    expect(await p.$$eval('.cell .pinmark', x => x.length) === 2, 'pinned apps show the pin mark');
    await shot(p, 'launcher');
    await p.click('.cell[title="Firefox"]', { button: 'right' });
    let m = (await sent(p, 'menu')).pop();
    expect(m && m.title === 'Firefox' && m.items.map(i => i.label).join() === 'Open,Pin to taskbar', 'right-click: Open, Pin to taskbar');
    await p.evaluate(id => __push('mantel.menuPick', { id }), m.items[1].id);
    await p.waitForTimeout(50);
    expect((await sent(p, 'pin')).some(x => x.id === 'Mozilla.Firefox' && x.on), 'Pin to taskbar pins Firefox');
    expect(await p.$$eval('.cell .pinmark', x => x.length) === 3, 'the pin mark shows at once');
    for (const key of ['ContextMenu', 'F23']) {   // the Menu key, and X on the controller (Ember types F23)
      const before = (await sent(p, 'menu')).length;
      await press(p, key);
      expect((await sent(p, 'menu')).length === before + 1, `${key} opens the menu`);
    }
    // Press and hold (touch).
    const box = await p.$eval('.cell[title="Steam"]', c => { const r = c.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; });
    const before = (await sent(p, 'menu')).length;
    await p.mouse.move(box.x, box.y); await p.mouse.down(); await p.waitForTimeout(650); await p.mouse.up();
    m = (await sent(p, 'menu')).pop();
    expect((await sent(p, 'menu')).length === before + 1 && m.items[1].label === 'Unpin from taskbar', 'holding a pinned app offers Unpin');
    expect(!(await sent(p, 'launch')).length, 'holding doesn\'t also open the app');
    await p.close();
  });

  // ---------------------------------------------------------------- the right-click menu
  await check('menu', async () => {
    const p = await open('mantel.html?part=menu', 268, 330);
    await p.evaluate(() => __push('mantel.menu', { title: 'File Explorer', items: [
      { id: 'a', label: 'New window', icon: 'open', enabled: true }, { id: 'b', label: 'Pin to taskbar', icon: 'pin', enabled: true }, { separator: true },
      { id: 'c', label: 'Move to desktop', icon: 'desktops', enabled: true, sub: [{ id: 'c1', label: 'Desktop 1 (this one)', enabled: false }, { id: 'c2', label: 'Desktop 2', enabled: true }] },
      { id: 'd', label: 'Maximize', icon: 'window', enabled: true }, { separator: true }, { id: 'e', label: 'Close window', icon: 'close', danger: true, enabled: true }] }));
    await p.waitForTimeout(250);
    await shot(p, 'menu');
    await p.keyboard.press('ArrowDown'); await p.keyboard.press('ArrowDown'); await p.keyboard.press('ArrowDown');
    expect(await p.$eval('.pi2.lit .lab', x => x.textContent) === 'Move to desktop', 'Down skips the separator');
    await p.keyboard.press('ArrowRight');
    expect(await p.$eval('.pm .pt span', x => x.textContent) === 'Move to desktop', 'Right opens the submenu');
    expect(await p.$eval('.pi2.lit .lab', x => x.textContent) === 'Desktop 2', 'the submenu skips the desktop you\'re on');
    expect((await sent(p, 'menuFit')).length === 1, 'the window is resized for the submenu');
    await p.keyboard.press('Enter');
    expect((await sent(p, 'menuPick')).pop().id === 'c2', 'Enter picks');
    await p.keyboard.press('Escape'); await p.keyboard.press('Escape');
    expect((await sent(p, 'menuClose')).length === 1, 'Esc goes back, then closes');
    await p.close();
  });

  // ---------------------------------------------------------------- the clock's panel
  await check('clock', async () => {
    const t = Date.now();
    const p = await open('mantel.html?part=clock', 380, 720, { 'notify.list': { history: [{ key: 2, app: 'Discord', summary: 'Lyra', body: 'tonight?', time: t - 60e3 }, { key: 1, app: 'Ember', summary: 'Hollow Knight is installed', body: '', time: t }], unseen: 2, dnd: false } });
    await p.evaluate(() => __push('mantel.opened', {}));
    await p.waitForTimeout(300);
    expect((await p.$$('.note2')).length === 2, 'the history shows');
    expect(await p.$('.days .d.now'), 'today is marked');
    await shot(p, 'clock');
    await p.keyboard.press('ArrowDown'); await p.keyboard.press('Enter');
    expect(await p.evaluate(() => __sent.some(m => m.type === 'call' && m.method === 'desk.set' && m.args.key === 'dnd' && m.args.value === true)), 'Do not disturb turns on');
    const month = await p.$eval('.cal .mh b', b => b.textContent);
    await p.keyboard.press('PageDown');
    expect(await p.$eval('.cal .mh b', b => b.textContent) !== month, 'Page Down shows the next month');
    await p.click('.note2 .nx');
    expect((await p.$$('.note2')).length === 1, '× dismisses a notification');
    expect((await sent(p, 'clockFit')).length > 0, 'the window fits what\'s in it');
    await p.close();
  });

  // ---------------------------------------------------------------- clipboard history
  await check('clipboard', async () => {
    const p = await open('mantel.html?part=clip', 640, 620, { 'clipboard.list': { available: true, enabled: true, items: [{ id: 'x', kind: 'text', text: 'hello' }, { id: 'y', kind: 'text', text: 'world' }] } });
    await p.evaluate(() => __push('mantel.opened', {}));
    await p.waitForTimeout(150);
    expect((await p.$$('.ci')).length === 2, 'items show');
    await shot(p, 'clipboard');
    await press(p, 'F23');   // X on the controller
    expect((await p.$$('.ci')).length === 1, 'X forgets one');
    await p.keyboard.press('Enter');
    expect((await sent(p, 'clipPaste')).pop().id === 'y', 'Enter pastes');
    await p.close();
  });

  // ---------------------------------------------------------------- dialogs
  await check('dialog', async () => {
    const p = await open('mantel.html?part=dialog', 460, 240);
    await p.evaluate(() => __push('mantel.dialog', { mode: 'ask', title: 'Rename “notes.txt”', value: 'notes.txt', ok: 'Rename', req: 7 }));
    await p.waitForTimeout(100);
    await shot(p, 'dialog');
    await p.keyboard.type('todo');
    await p.keyboard.press('Enter');
    const d = (await sent(p, 'dialogDone')).pop();
    expect(d && d.req === 7 && d.ok && d.value === 'todo.txt', `rename keeps the extension (${d && d.value})`);
    await p.close();
  });

  // ---------------------------------------------------------------- desktop icons (full Mantel)
  await check('desktop', async () => {
    const entries = [{ name: 'Games', label: 'Games', dir: true, col: 0, row: 0, path: 'C:\\Users\\a\\Desktop\\Games' }, { name: 'Steam.lnk', label: 'Steam', launcher: true, col: 0, row: 1, path: 'C:\\Users\\a\\Desktop\\Steam.lnk' }];
    const p = await open('mantel.html?part=desktop', 1280, 800, { 'desk.icons': { dir: 'C:\\Users\\a\\Desktop', exists: true, entries }, 'files.info': { size: 1200, type: 'Shortcut', location: 'C:\\Users\\a\\Desktop', modified: Date.now() } });
    await p.evaluate(() => __push('mantel.state', { full: true, desktopIcons: true, wallpaper: 'anim:fireside' }));
    await p.waitForTimeout(200);
    expect((await p.$$('.di')).length === 2, 'the icons show');
    await shot(p, 'desktop');
    await p.click('.di[data-name="Steam.lnk"]', { button: 'right' });
    let m = (await sent(p, 'menu')).pop();
    expect(m.items.filter(i => !i.separator).map(i => i.label).join() === 'Open,Show in Files,Rename,Move to the Recycle Bin,Properties', 'an icon\'s menu');
    await p.mouse.click(900, 500, { button: 'right' });
    m = (await sent(p, 'menu')).pop();
    expect(m.title === 'Desktop' && m.items.some(i => i.label === 'Show desktop icons' && i.checked === true) && m.items.some(i => i.label === 'Line up icons by name'), 'the background\'s menu');
    // Drag an icon to another place: Ember remembers it.
    await p.mouse.move(64, 60); await p.mouse.down(); await p.mouse.move(200, 70, { steps: 5 }); await p.mouse.up();
    await p.waitForTimeout(50);
    expect(await p.evaluate(() => __sent.some(m => m.type === 'call' && m.method === 'desk.place' && m.args.name === 'Games' && m.args.col === 1)), 'dragging places the icon');
    // Keyboard: F2 renames the selected icon, Delete moves it to the Recycle Bin.
    await p.click('.di[data-name="Steam.lnk"]');
    await p.keyboard.press('Delete');
    expect(await p.evaluate(() => __sent.some(m => m.type === 'call' && m.method === 'files.trash')), 'Delete moves it to the Recycle Bin');
    await p.keyboard.press('F2');
    expect((await sent(p, 'dialog')).pop().mode === 'ask', 'F2 asks for the new name');
    await p.close();
  });

  // ---------------------------------------------------------------- the other parts draw
  for (const [part, w, h] of [['quick', 400, 700], ['power', 360, 430], ['radial', 500, 500], ['help', 640, 520], ['keyboard', 1280, 300]])
    await check(part, async () => { const p = await open(`mantel.html?part=${part}`, w, h); await p.evaluate(() => __push('mantel.opened', {})); await p.waitForTimeout(150); await p.close(); });

  // ---------------------------------------------------------------- Hearth Settings: the new pages
  const SET = {
    'desk.get': { clock24: false },
    'time.get': { now: '2026-10-20T21:41:00-07:00', timezone: 'Pacific Standard Time', zoneName: 'Pacific Time (US & Canada)', region: 'America', ntp: true, canNtp: true },
    'time.zones': { zones: [{ zone: 'Pacific Standard Time', region: 'America', city: 'Pacific Time (US & Canada)', offset: '-07:00' }, { zone: 'Tokyo Standard Time', region: 'Asia', city: 'Osaka, Sapporo, Tokyo', offset: '+09:00' }] },
    'time.setZone': { now: '2026-10-21T13:41:00+09:00', timezone: 'Tokyo Standard Time', zoneName: 'Osaka, Sapporo, Tokyo', region: 'Asia', ntp: true, canNtp: true },
    'keyboard.get': { layouts: [{ tip: '0409:00000409', name: 'English (United States) · US' }], switch: 'alt_shift', delay: 500, rate: 30 },
    'keyboard.layouts': { layouts: [{ tip: '0407:00000407', name: 'German (Germany) · German' }] },
    'keyboard.set': a => ({ layouts: a.layouts.map(t => ({ tip: t, name: t })), switch: a.switch, delay: a.delay, rate: a.rate }),
    'keyboard.shortcuts': { shortcuts: [{ keys: 'Windows', action: 'Open or close the app launcher' }] },
    'defaults.get': { kinds: [{ kind: 'browser', label: 'Web browser', current: 'Firefox' }] },
    'users.get': { admin: true, canAdmin: true, users: [{ name: 'me', fullname: 'Me', admin: true, me: true, home: 'C:\\Users\\me' }, { name: 'kid', fullname: 'Kid', admin: false, me: false, home: 'C:\\Users\\kid' }] },
    'printers.get': { cups: true, printers: [{ name: 'Office', info: 'Office', default: true, state: 'idle' }] },
    'clipboard.status': { available: true, enabled: true },
  };
  for (const pg of ['datetime', 'keyboard', 'defaults', 'users', 'printers', 'clipboard'])
    await check('settings ' + pg, async () => {
      const p = await open(`mantel-settings.html?page=${pg}`, 1100, 720, SET);
      await p.waitForTimeout(200);
      expect(await p.$('.ni.lit') && (await p.$eval('.page h1', h => h.textContent)).length > 0, 'the page draws');
      await shot(p, 'settings-' + pg);
      if (pg === 'datetime') {
        await p.click('[data-input="tz"]'); await p.keyboard.type('tokyo'); await p.waitForTimeout(250);
        await p.click('[data-act="tmZone"]');
        await p.waitForTimeout(50);
        expect(await p.evaluate(() => __sent.some(m => m.method === 'time.setZone' && m.args.zone === 'Tokyo Standard Time')), 'searching and picking sets the time zone');
        await p.click('[data-act="choose"][data-v="true"]');
        expect(await p.evaluate(() => __sent.some(m => m.method === 'desk.set' && m.args.key === 'clock24' && m.args.value === true)), '24-hour clock');
      }
      if (pg === 'keyboard') {
        await p.click('[data-input="kbAdd"]'); await p.keyboard.type('german'); await p.waitForTimeout(250);
        await p.click('[data-act="kbAdd"]');
        await p.waitForTimeout(50);
        expect(await p.evaluate(() => __sent.some(m => m.method === 'keyboard.set' && m.args.layouts.length === 2)), 'adding a layout saves both');
      }
      await p.close();
    });

  // ---------------------------------------------------------------- Apps
  await check('apps', async () => {
    const apps = [{ id: 'Mozilla.Firefox', name: 'Firefox', summary: 'Web browser', installed: true }, { id: 'VideoLAN.VLC', name: 'VLC media player', summary: 'Plays everything' }];
    const p = await open('mantel-apps.html', 1100, 720, { 'apps.status': { winget: true }, 'apps.browse': { apps }, 'apps.updates': { apps: [] }, 'apps.install': '__wait',
      'apps.app': { id: 'VideoLAN.VLC', name: 'VLC media player', summary: 'Plays everything', description: 'A player.', installed: false, catalogued: true, version: '3.0.21' } });
    await p.waitForTimeout(300);
    expect((await p.$$('.row.appr')).length === 2, 'Discover lists apps');
    await shot(p, 'apps');
    await p.click('.row.appr:nth-of-type(2)');
    await p.waitForTimeout(100);
    await shot(p, 'apps-page');
    await p.click('[data-act="install"]');
    expect(await p.evaluate(() => __sent.some(m => m.method === 'apps.install' && m.args.id === 'VideoLAN.VLC')), 'Install installs with winget');
    await p.evaluate(() => __push('apps.progress', { id: 'VideoLAN.VLC', progress: 0.42, line: 'Downloading' }));
    expect(/42%/.test(await p.$eval('.prog', x => x.textContent)), 'progress shows');
    await p.close();
  });

  await browser.close();
  if (errors.length) { console.error('Mantel check failed:\n  ' + errors.join('\n  ')); process.exit(1); }
  console.log('Mantel check passed: taskbar, launcher, menus, clock, clipboard, dialogs, desktop icons, Settings pages and Apps');
})();
