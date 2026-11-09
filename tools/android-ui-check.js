// Hearth for Android's page check (.github/workflows/android.yml): Hearth's page as the APK serves it, on a Samsung
// Galaxy Tab A9 (8.7", 1340×800, about 1005×600 dp) in landscape and portrait, and on a phone in landscape.
//   - Served like MainActivity.kt does: from https://appassets.androidplatform.net/ui/, with ONLY the files the APK
//     has (ui/ minus hosts/android/ui-exclude.txt), the real host shim from MainActivity.kt and the viewport
//     Viewport.kt picks. A file the page loads that the APK lacks fails the check.
//   - A pretend host that answers exactly the methods Bridge.kt answers (read from Bridge.kt), everything else with
//     Bridge.kt's "That isn't in Hearth for Android yet". Methods the page asks for that Bridge.kt doesn't answer
//     are listed (a notice, not a failure: the page copes without them).
//   - Home, Library and Settings (the device's settings inside Hearth: android-settings.js) must draw without a script
//     error; the controller moves through Settings and changes the volume; a permission Hearth doesn't have is asked
//     for, and a "no" changes nothing.
//   - Pictures (tools/android-shots/*.jpg, small enough for the workflow's SHOT notices): Home and Library on the
//     Tab A9 in landscape and portrait, and its Display & sound page.
//   npm i --no-save playwright@1.48.2 && npx playwright install chromium && node tools/android-ui-check.js
const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const UI = path.join(ROOT, 'ui');
const AND = path.join(ROOT, 'hosts/android');
const KT = path.join(AND, 'app/src/main/java/dev/hearthos/android');
const ORIGIN = 'https://appassets.androidplatform.net';
const SHOTS = path.join(__dirname, 'android-shots');
const MAX_SHOT = 28000;   // bytes: 10 notices of 3800 base64 characters

const errors = [], notices = [];
const ann = (kind, title, msg) => console.log(`::${kind} title=${title}::${String(msg).replace(/\r?\n/g, ' | ').slice(0, 3900)}`);

// ---------------------------------------------------------------- what the APK has and does
const exclude = fs.readFileSync(path.join(AND, 'ui-exclude.txt'), 'utf8').split('\n').map(l => l.trim()).filter(l => l && !l.startsWith('#'))
  .map(p => new RegExp('^' + p.split('*').map(x => x.replace(/[.+?^${}()|[\]\\]/g, '\\$&')).join('.*') + '$'));
const inApk = rel => { const top = rel.split('/')[0]; return !exclude.some(r => r.test(top)); };
const mainKt = fs.readFileSync(path.join(KT, 'MainActivity.kt'), 'utf8');
const shimMatch = /const val SHIM = """([\s\S]*?)"""/.exec(mainKt);
if (!shimMatch) { ann('error', 'Android page check', "MainActivity.kt's SHIM wasn't found"); process.exit(1); }
const SHIM = shimMatch[1];
const bridgeKt = fs.readFileSync(path.join(KT, 'Bridge.kt'), 'utf8');
const dispatch = bridgeKt.slice(bridgeKt.indexOf('private fun dispatch('), bridgeKt.indexOf('// ================================================================ library'));
const BRIDGE = new Set([...dispatch.matchAll(/"([a-z][a-zA-Z]*(?:\.[a-zA-Z]+)+)"/g)].map(m => m[1]).filter(m => !/^(android|com|dev)\.[a-z]+\.[A-Z]/.test(m)));
// Viewport.kt, the same rule (keep in step): landscape under 1280 dp → 1280 px; portrait under 800 dp → 800 px.
function viewportMeta(wDp, hDp){
  const w = wDp >= hDp ? (wDp < 1280 ? 1280 : 0) : (wDp < 800 ? 800 : 0);
  if (!w) return 'width=device-width, initial-scale=1, viewport-fit=cover';
  const s = (wDp / w).toFixed(4);
  return `width=${w}, initial-scale=${s}, minimum-scale=${s}, maximum-scale=${s}, user-scalable=no, viewport-fit=cover`;
}
const MIME = {html: 'text/html', js: 'text/javascript', css: 'text/css', svg: 'image/svg+xml', png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', json: 'application/json', woff2: 'font/woff2', webp: 'image/webp'};

// ---------------------------------------------------------------- the pretend host (answers like Bridge.kt)
function pretendHost(){
  const day = 864e5, now = Date.now(), L = [];
  const games = [
    {id: 'android:com.retroarch', title: 'RetroArch', source: 'android', installed: true, state: 'installed', art: {}, minutes: 30, lastPlayed: now - day, added: now - 5 * day, cols: [], fav: true},
    {id: 'android:com.example.racer', title: 'Asphalt Racer', source: 'android', installed: true, state: 'installed', art: {}, minutes: 95, lastPlayed: now - 2 * day, added: now - day, cols: []},
    {id: 'android:com.example.quest', title: 'Pocket Quest', source: 'android', installed: true, state: 'installed', art: {}, minutes: 12, lastPlayed: now - 3 * day, added: now - 3 * day, cols: []},
    {id: 'android:com.moonlight_stream.moonlight', title: 'Moonlight', source: 'android', installed: true, state: 'installed', art: {}, added: now - 8 * day, cols: []},
    {id: 'android:com.example.notes', title: 'Notes', source: 'manual', installed: true, state: 'installed', art: {}, added: now - 9 * day, cols: [], canRemove: true},
    {id: 'emu:snes:Super Mario World (USA).sfc', title: 'Super Mario World', source: 'emu', system: 'Super Nintendo', installed: true, state: 'installed', art: {}, added: now - 2 * day, cols: []}];
  const st = window.__st = {
    perms: {writeSettings: false, dnd: false, location: false, btConnect: false, batteryOpt: false, notifications: true},
    grant: {}, vol: {media: 7, ring: 5, notification: 5, alarm: 9}, max: {media: 15, ring: 15, notification: 15, alarm: 15}, muted: false,
    bright: 60, auto: true, timeout: 60, rotate: true, dnd: 'all'};
  const settingsState = () => ({sdk: 34, samsung: true, panels: true,
    audio: {streams: Object.fromEntries(Object.keys(st.vol).map(k => [k, {v: st.vol[k], max: st.max[k], muted: k === 'media' && st.muted}])), ringer: 'normal'},
    display: {canWrite: st.perms.writeSettings, brightness: st.bright, auto: st.auto, timeoutSec: st.timeout, timeouts: [15, 30, 60, 120, 300, 600, 1800], rotate: st.rotate, dark: true},
    dnd: {access: st.perms.dnd, filter: st.dnd},
    wifi: {available: true, on: true, ssid: st.perms.location ? 'Casa Wi-Fi' : 'Wi-Fi', bars: 3, online: true, canSeeName: st.perms.location},
    bt: {supported: true, on: true, canList: st.perms.btConnect, paired: st.perms.btConnect ? [{name: 'Xbox Wireless Controller', address: 'AA', kind: 'controller'}, {name: 'Galaxy Buds2', address: 'BB', kind: 'audio'}] : []},
    battery: {present: true, pct: 64, charging: false, saver: false, background: st.perms.batteryOpt}, nfc: {supported: false, on: false},
    airplane: false, perms: {...st.perms}});
  const need = {brightness: 'writeSettings', autoBrightness: 'writeSettings', timeout: 'writeSettings', autoRotate: 'writeSettings', dnd: 'dnd'};
  const R = {
    'host.info': {platform: 'android', name: 'Galaxy Tab A9', host: 'Hearth (Android)', version: '1.9.0', edition: 'android', features: ['android', 'sendLogs', 'linux', 'linuxDesktop'], deviceId: '0'.repeat(32), setupDone: true, live: false, autoReport: false, isDefaultHome: true},
    'sys.status': {wifi: {available: true, on: true, ssid: 'Wi-Fi', bars: 3, online: true}, bt: {available: true, on: true}, airplane: false, battery: {present: true, pct: 64, charging: false}, audio: {volume: 47, muted: false, max: 100, devices: []}, display: {supported: false, brightness: 60}},
    'library.get': {games, sources: {}, collections: [], counts: {android: 4, manual: 1, emu: 1}, runningId: null},
    'profile.get': {name: 'Galaxy Tab A9', status: null, avatar: 'initials', accent: '#3fd0b8', background: 'default', customAvatar: null, customBackground: null, storeAvatars: {steam: null, xbox: null, gog: null}},
    'android.status': {showAll: false, home: {isDefault: true}, roms: {folder: 'Internal storage › ROMs', count: 1, systems: [{dir: 'snes', name: 'Super Nintendo', count: 1}]}, retroarch: {pkg: 'com.retroarch'}, pads: [{name: 'Xbox Wireless Controller', id: 5}]},
    'android.apps': [{id: 'com.example.notes', title: 'Notes'}],
    'android.openSettings': {ok: true}, 'android.openUrl': {ok: true}, 'android.panel': {ok: true}, 'android.uninstall': {ok: true},
    'android.settings.get': () => settingsState(),
    'android.settings.set': a => {
      const n = need[a.key];
      if (n && !st.perms[n]) return {ok: false, need: n, why: 'Android asks once', state: settingsState()};
      if (a.key.startsWith('vol.')) st.vol[a.key.slice(4)] = a.value;
      if (a.key === 'mute') st.muted = !!a.value;
      if (a.key === 'brightness') st.bright = a.value;
      if (a.key === 'autoBrightness') st.auto = !!a.value;
      if (a.key === 'autoRotate') st.rotate = !!a.value;
      if (a.key === 'timeout') st.timeout = a.value;
      if (a.key === 'dnd') st.dnd = a.value;
      return {ok: true, state: settingsState()};
    },
    'android.settings.ask': a => { st.perms[a.perm] = !!st.grant[a.perm]; return {perm: a.perm, granted: st.perms[a.perm], state: settingsState()}; },
    'android.wifi.scan': () => st.perms.location ? {networks: [{ssid: 'Casa Wi-Fi', bars: 4, security: 'wpa'}, {ssid: 'Cafe', bars: 2, security: 'open'}], on: true} : {networks: [], need: 'location', why: 'location'},
    'android.wifi.join': {ok: true},
    'android.bt.enable': () => ({ok: true, state: settingsState()}),
    'android.bt.pair': {ok: true, device: 'Xbox Wireless Controller'},
    'uiprefs.get': () => window.__uiprefs, 'uiprefs.set': a => { Object.assign(window.__uiprefs, a.values || {}); return {ok: true}; },
    'audio.get': {volume: 47, muted: false, max: 100, devices: []},
    'display.get': {supported: false, brightness: 60, width: 1340, height: 800, refreshRate: 60, resolution: '1340 × 800'},
    'power.get': {present: true, pct: 64, charging: false, mode: null, modes: [], modeSupported: false},
    'system.info': {device: 'Samsung SM-X110', hostname: 'Galaxy Tab A9', os: 'Android', osVersion: '14', osPretty: 'Android 14 (API 34)', cpu: 'Mediatek MT6789', cores: 8, memoryBytes: 4e9, storage: {totalBytes: 64e9, freeBytes: 30e9}, uptimeSec: 3600},
    'pad.status': {connected: 1, mouseMode: false, mouseOn: false},
    'linux.status': {installed: false, sizeBytes: 0, distro: 'Debian 13 (trixie)', running: false, jobs: [], busy: null, x11App: false, x11Running: false, supported: true, unsupported: null, freeBytes: 3e10, needBytes: 1.5e9, sdcard: false, error: null},
    'linux.desktop.status': {linux: true, installed: false, missing: [], packages: [], mode: 'hearth', x11App: false, installing: false, running: false, method: null, ready: false},
    'library.downloads': {active: [], recent: [], speeds: []}, 'game.info': {}, 'setup.done': {ok: true}, 'pad.raw': a => ({on: !!a.on})};
  window.__uiprefs = {};
  window.__calls = []; window.__unknown = []; window.__msgs = [];
  // The WebView's JavaScript interface (HearthJsInterface): MainActivity's shim calls HearthAndroid.postMessage(json).
  window.HearthAndroid = {postMessage(json){
    const m = JSON.parse(json);
    window.__msgs.push(m.type);
    if (m.type !== 'call') return;
    window.__calls.push(m.method);
    const known = window.__BRIDGE.includes(m.method);
    let r;
    if (!known) { window.__unknown.push(m.method); r = {type: 'result', id: m.id, ok: false, error: "That isn't in Hearth for Android yet"}; }
    else if (m.method in R) { const v = typeof R[m.method] === 'function' ? R[m.method](m.args || {}) : R[m.method]; r = {type: 'result', id: m.id, ok: true, value: v === undefined ? null : JSON.parse(JSON.stringify(v))}; }
    else r = {type: 'result', id: m.id, ok: true, value: null};
    setTimeout(() => window.__hearthRecv && window.__hearthRecv(r), 5);
  }};
}

async function shot(page, name){
  fs.mkdirSync(SHOTS, {recursive: true});
  const png = await page.screenshot();
  // Made small enough for the SHOT notices: drawn smaller and as JPEG until it fits.
  const jpg = await page.evaluate(async ([b64, max]) => {
    const img = new Image(); img.src = 'data:image/png;base64,' + b64; await img.decode();
    for (const [scale, q] of [[.6, .7], [.5, .6], [.45, .5], [.4, .45], [.35, .4], [.3, .35]]) {
      const c = document.createElement('canvas'); c.width = Math.round(img.width * scale); c.height = Math.round(img.height * scale);
      c.getContext('2d').drawImage(img, 0, 0, c.width, c.height);
      const d = c.toDataURL('image/jpeg', q).split(',')[1];
      if (d.length * 3 / 4 <= max) return d;
    }
    return null;
  }, [png.toString('base64'), MAX_SHOT]);
  if (!jpg) { notices.push(`picture ${name}: too big for a notice`); return; }
  fs.writeFileSync(path.join(SHOTS, name + '.jpg'), Buffer.from(jpg, 'base64'));
}

(async () => {
  const browser = await chromium.launch();
  const devices = [
    {name: 'tab-a9-landscape', viewport: {width: 1005, height: 600}, deviceScaleFactor: 1340 / 1005, shots: true},
    {name: 'tab-a9-portrait', viewport: {width: 600, height: 1005}, deviceScaleFactor: 1340 / 1005, shots: true},
    {name: 'phone-landscape', viewport: {width: 915, height: 412}, deviceScaleFactor: 2.625, shots: false}];
  const unknown = new Set(), missing = new Set();
  for (const dev of devices) {
    const ctx = await browser.newContext({viewport: dev.viewport, deviceScaleFactor: dev.deviceScaleFactor, isMobile: true, hasTouch: true,
      userAgent: 'Mozilla/5.0 (Linux; Android 14; SM-X110 Build/UP1A; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/130.0.0.0 Safari/537.36'});
    const page = await ctx.newPage();
    const E = m => errors.push(`${dev.name}: ${m}`);
    page.on('pageerror', e => E(e.message));
    page.on('console', m => { if (m.type() === 'error' && !/Failed to load resource|net::ERR/.test(m.text())) E('console: ' + m.text()); });
    await ctx.route('**/*', async route => {
      const u = new URL(route.request().url());
      if (u.origin !== ORIGIN) return route.abort();   // box art and the like: not needed for the check
      if (!u.pathname.startsWith('/ui/')) return route.fulfill({status: 404, body: ''});
      const rel = decodeURIComponent(u.pathname.slice(4)) || 'index.html';
      const f = path.join(UI, rel);
      if (rel.includes('..') || !inApk(rel) || !fs.existsSync(f) || !fs.statSync(f).isFile()) { missing.add(rel); return route.fulfill({status: 404, body: ''}); }
      let body = fs.readFileSync(f);
      if (rel === 'index.html') {
        let html = body.toString('utf8').replace(/<meta name="viewport" content="[^"]*">/, `<meta name="viewport" content="${viewportMeta(dev.viewport.width, dev.viewport.height)}">`);
        const i = html.toLowerCase().indexOf('<head>');
        html = html.slice(0, i + 6) + SHIM + html.slice(i + 6);
        body = Buffer.from(html, 'utf8');
      }
      route.fulfill({status: 200, body, contentType: MIME[path.extname(f).slice(1).toLowerCase()] || 'application/octet-stream'});
    });
    await page.addInitScript(b => { window.__BRIDGE = b; }, [...BRIDGE]);
    await page.addInitScript(pretendHost);
    await page.goto(ORIGIN + '/ui/index.html');
    await page.waitForTimeout(2000);
    const step = async (what, fn, arg) => { try { await page.evaluate(fn, arg); await page.waitForTimeout(300); } catch (e) { E(`${what}: ${e.message.split('\n')[0]}`); } };
    await step('platform', () => {
      if (!ANDROID || APP !== 'Hearth') throw new Error('not Hearth for Android');
      if (!window.__msgs.includes('ready')) throw new Error("the page never said it's ready");
      if (!window.__calls.includes('host.info') || !window.__calls.includes('library.get')) throw new Error('the page never asked for host.info / library.get');
      if (typeof anPage !== 'function') throw new Error('android-settings.js did not load');
    });
    await step('layout width', ([w]) => { if (Math.abs(window.innerWidth - w) > 2) throw new Error(`laid out ${window.innerWidth} px wide, not ${w}`); },
      [dev.viewport.width >= dev.viewport.height ? 1280 : 800]);
    for (const v of ['home', 'library']) {
      await step('screen ' + v, v => { S.layers = []; navigate(v, {}, false); if (!document.querySelector('#view').textContent.trim()) throw new Error('empty'); }, v);
      if (v === 'library') await step('library has the games', () => { if (!/Asphalt Racer/.test(document.querySelector('#view').textContent)) throw new Error('no games shown'); });
      await step('no sideways scrolling on ' + v, () => { if (document.documentElement.scrollWidth > window.innerWidth + 4) throw new Error(`page is ${document.documentElement.scrollWidth} px wide in a ${window.innerWidth} px window`); });
      if (dev.shots) await shot(page, `${dev.name}-${v}`);
    }
    for (const t of ['wifi', 'bluetooth', 'display', 'power', 'system', 'sources', 'controller', 'about'])
      await step('settings ' + t, async t => { S.layers = []; S.settingsTab = t; navigate('settings', {}, false); onSettingsTab(t); await new Promise(r => setTimeout(r, 250)); render(null, true); if (!document.querySelector('#view .row, #view .ctl')) throw new Error('no rows'); }, t);
    if (dev.name === 'tab-a9-landscape') {
      // The device's settings inside Hearth: rows there, the controller changes the volume, a missing permission is
      // asked for (and a "no" changes nothing), a "yes" lets the change through.
      await step('display & sound rows', async () => {
        S.layers = []; S.settingsTab = 'display'; navigate('settings', {}, false); onSettingsTab('display'); await new Promise(r => setTimeout(r, 300)); render(null, true);
        const miss = ['an-sl-vol-media', 'an-sl-vol-ring', 'an-sl-vol-alarm', 'an-sl-brightness', 'a-mute', 'an-dnd', 'an-autob', 'an-timeout', 'an-rotate', 'an-volp', 'an-text', 'an-disp', 'an-wsperm'].filter(k => !document.querySelector(`#view [data-k="${k}"]`));
        if (miss.length) throw new Error('missing ' + miss.join(', '));
      });
      await shot(page, 'tab-a9-settings-display');
      await step('volume with the controller', async () => {
        const el = document.querySelector('#view [data-k="an-sl-vol-media"]'); setFocus(el, true);
        onHostMessage({type: 'event', name: 'pad', data: {b: 'a'}}); await new Promise(r => setTimeout(r, 50));
        onHostMessage({type: 'event', name: 'pad', data: {b: 'right'}}); onHostMessage({type: 'event', name: 'pad', data: {b: 'right'}});
        await new Promise(r => setTimeout(r, 400));
        if (window.__st.vol.media !== 9) throw new Error(`media volume is ${window.__st.vol.media}, not 9`);
        onHostMessage({type: 'event', name: 'pad', data: {b: 'b'}}); await new Promise(r => setTimeout(r, 50));
      });
      await step('a declined permission changes nothing', async () => {
        window.__st.grant.writeSettings = false;
        await anAct('anToggle', {key: 'autoRotate'}); await new Promise(r => setTimeout(r, 80));
        if (!S.layers.includes('sheet') || !/okay/.test(document.querySelector('#sheetBox').textContent)) throw new Error('no "Android needs your okay" sheet');
        sheetPick(0); await new Promise(r => setTimeout(r, 120));
        if (window.__st.perms.writeSettings || window.__st.rotate !== true) throw new Error('something changed after a no');
      });
      await step('a given permission lets the change through', async () => {
        window.__st.grant.writeSettings = true;
        await anAsk('writeSettings'); await new Promise(r => setTimeout(r, 80));
        await anAct('anToggle', {key: 'autoRotate'}); await new Promise(r => setTimeout(r, 120));
        if (window.__st.rotate !== false) throw new Error('auto-rotate did not change');
        if (document.querySelector('#view [data-k="an-wsperm"]')) throw new Error('still asks for the permission');
      });
      await step('wifi: name, nearby networks, join', async () => {
        window.__st.grant.location = true;
        S.layers = []; S.settingsTab = 'wifi'; navigate('settings', {}, false); onSettingsTab('wifi'); await new Promise(r => setTimeout(r, 200));
        if (!document.querySelector('#view [data-k="an-wloc"]') || !document.querySelector('#view [data-k="an-wpanel"]') || !document.querySelector('#view [data-k="an-inet"]')) throw new Error('rows missing');
        await anAsk('location'); await new Promise(r => setTimeout(r, 300));
        if (!/Casa Wi-Fi/.test(document.querySelector('#view').textContent)) throw new Error('no network name / nearby list');
        window.openPrompt = async () => 'hunter22hunter';
        await anAct('anJoin', {i: '0'}); await new Promise(r => setTimeout(r, 80));
        if (!window.__calls.includes('android.wifi.join')) throw new Error('join not asked');
      });
      await step('bluetooth: paired devices after "Nearby devices"', async () => {
        window.__st.grant.btConnect = true;
        S.layers = []; S.settingsTab = 'bluetooth'; navigate('settings', {}, false); onSettingsTab('bluetooth'); await new Promise(r => setTimeout(r, 200));
        if (!document.querySelector('#view [data-k="an-btl"]') || !document.querySelector('#view [data-k="an-pair"]')) throw new Error('rows missing');
        await anAsk('btConnect'); await new Promise(r => setTimeout(r, 100));
        if (!/Xbox Wireless Controller/.test(document.querySelector('#view').textContent)) throw new Error('no paired devices');
        await anAct('anBtPair', {}); await new Promise(r => setTimeout(r, 50));
      });
      await step('system: Android pages', async () => {
        S.layers = []; S.settingsTab = 'system'; navigate('settings', {}, false); onSettingsTab('system'); await new Promise(r => setTimeout(r, 200));
        const miss = ['an-home', 'an-notif', 'an-storage', 'an-date', 'an-lang', 'an-a11y', 'an-dev', 'an-apps', 'an-bg2'].filter(k => !document.querySelector(`#view [data-k="${k}"]`));
        if (miss.length) throw new Error('missing ' + miss.join(', '));
        act(document.querySelector('#view [data-k="an-storage"]'));
      });
      await step('game options: Uninstall', () => {
        gameOptions(G('android:com.example.racer'));
        if (!/Uninstall/.test(document.querySelector('#sheetBox').textContent)) throw new Error('no Uninstall');
        closeAll();
      });
      await step('Appearance is kept by the host (uiprefs)', async () => {
        LOOK.textSize = 'large'; saveLook(); await new Promise(r => setTimeout(r, 80));
        if (!/large/.test(window.__uiprefs['nimbus.look'] || '')) throw new Error('uiprefs.set did not get the look');
      });
    }
    (await page.evaluate(() => window.__unknown)).forEach(m => unknown.add(m));
    await ctx.close();
  }
  await browser.close();
  if (missing.size) errors.push(`the page asked for files the APK doesn't have: ${[...missing].join(', ')}`);
  if (unknown.size) notices.push(`the page asked for methods Hearth for Android doesn't answer (it copes): ${[...unknown].sort().join(', ')}`);
  notices.forEach(n => ann('notice', 'Android page check', n));
  if (errors.length) { errors.forEach(e => ann('error', 'Android page check', e)); console.error(errors.join('\n')); process.exit(1); }
  ann('notice', 'Android page check', `Hearth's page as Hearth for Android: Home, Library and Settings on the Galaxy Tab A9 (landscape, portrait) and a phone, without errors; ${BRIDGE.size} host methods`);
})().catch(e => { ann('error', 'Android page check', e.stack || e.message); process.exit(1); });
