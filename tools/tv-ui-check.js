// Hearth TV's page as the APK serves it, in a 1920×1080 browser: a pretend TV (what Bridge.kt answers) and a pretend
// Jellyfin, every screen of the approved mockups (docs/mockups/hearth-tv) photographed into tools/tv-shots/ for a look
// side by side with the mockups. Fails on a page error. Run by .github/workflows/android-tv.yml (page job).
const { chromium } = require('playwright');
const http = require('http');
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const ASSETS = path.join(ROOT, 'hosts/android-tv/app/src/main/assets');
const OUT = path.join(__dirname, 'tv-shots');
fs.mkdirSync(OUT, { recursive: true });

// The APK's assets, with assets/tv/theme/ made the way the build makes it (CopyHearthLooks in app/build.gradle.kts).
function fileFor(url) {
  const p = decodeURIComponent(url.split('?')[0]);
  if (p === '/assets/tv/theme/palettes.json') return path.join(ROOT, 'hosts/linux/desktop/theme/palettes.json');
  if (p.startsWith('/assets/tv/theme/wallpapers/')) return path.join(ROOT, 'hosts/linux/desktop/shell/images/wallpapers', p.slice('/assets/tv/theme/wallpapers/'.length));
  if (p.startsWith('/assets/tv/theme/backgrounds/')) return path.join(ROOT, 'os/branding/wallpapers', p.slice('/assets/tv/theme/backgrounds/'.length));
  if (p.startsWith('/assets/')) return path.join(ASSETS, p.slice('/assets/'.length));
  // The pointer's stage: a pretend website with the pointer layer over it (tools/tv-pointer).
  if (p.startsWith('/test/') && !p.includes('..')) return path.join(__dirname, 'tv-pointer', p.slice('/test/'.length));
  return null;
}
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.jpg': 'image/jpeg', '.png': 'image/png' };
const server = http.createServer((req, res) => {
  const f = fileFor(req.url);
  if (!f || !fs.existsSync(f)) { res.writeHead(404); return res.end(); }
  res.writeHead(200, { 'Content-Type': TYPES[path.extname(f)] || 'application/octet-stream', 'Access-Control-Allow-Origin': '*' });
  fs.createReadStream(f).pipe(res);
});

const W = (n) => `/assets/tv/theme/wallpapers/${n}.jpg`;
const B = (n) => `/assets/tv/theme/backgrounds/${n}.jpg`;

(async () => {
  await new Promise((r) => server.listen(0, r));
  const base = `http://127.0.0.1:${server.address().port}`;
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  page.on('console', (m) => { if (m.type() === 'error' && !/Failed to load resource|ERR_|net::/.test(m.text())) errors.push(m.text()); });

  // A pretend Jellyfin on http://jf.test with a few shows and movies (pictures: Hearth's wallpapers).
  const item = (id, type, name, extra) => Object.assign({ Id: id, Type: type, Name: name, ProductionYear: 2010, Overview: '[Summary from your library]', RunTimeTicks: 15 * 60 * 1e7, BackdropImageTags: ['b'], ImageTags: { Primary: 'p' }, UserData: { PlayedPercentage: 0, Played: false }, DateCreated: new Date().toISOString(), MediaStreams: [{ Type: 'Video', Width: 3840, VideoRange: 'SDR' }] }, extra || {});
  const lib = [
    item('show1', 'Series', 'The Cisco Kid', { ProductionYear: 1950, ChildCount: 3, UserData: { UnplayedItemCount: 2 } }),
    item('mov1', 'Movie', 'Sintel'), item('mov2', 'Movie', 'Big Buck Bunny', { ProductionYear: 2008, UserData: { PlayedPercentage: 60 } }),
    item('mov3', 'Movie', 'Plan 9 from Outer Space', { ProductionYear: 1959, UserData: { PlayedPercentage: 34 } }),
    item('show2', 'Series', 'One Step Beyond', { ProductionYear: 1959, BackdropImageTags: [], ImageTags: {} }),
    item('alb1', 'MusicAlbum', 'The Open Goldberg Variations', { AlbumArtist: 'Kimiko Ishizaka', ChildCount: 4, BackdropImageTags: [], ImageTags: {} })
  ];
  const eps = [1, 3, 4].map((n, i) => item('ep' + n, 'Episode', ['The Gay Caballero', 'Rustling', '[Episode 4]'][i], { IndexNumber: n, ParentIndexNumber: 1, SeriesId: 'show1', SeriesName: 'The Cisco Kid', SeasonId: 's1', RunTimeTicks: 26 * 60 * 1e7, UserData: { PlayedPercentage: [100, 30, 0][i], Played: i === 0 } }));
  const pics = { show1: W('slate-dusk'), mov1: W('slate-nebula'), mov2: B('hearth-peaks'), mov3: W('slate-contours'), ep1: B('hearth-cabin'), ep3: W('slate-ridges'), ep4: B('hearth-lanterns') };
  await page.route('http://jf.test/**', (route) => {
    const u = new URL(route.request().url()), p = u.pathname;
    const json = (o) => route.fulfill({ status: 200, contentType: 'application/json', headers: { 'Access-Control-Allow-Origin': '*' }, body: JSON.stringify(o) });
    if (p.includes('/Images/')) { const id = p.split('/')[2]; return route.fulfill({ status: 302, headers: { Location: base + (pics[id] || W('slate-horizon')) } }); }
    if (p.endsWith('/Items/Resume')) return json({ Items: [eps[1], lib[2], lib[3]] });
    if (p === '/Shows/NextUp') return json({ Items: [eps[1]] });
    if (p.endsWith('/Items/Latest')) return json(lib.slice(0, 4));
    if (p.endsWith('/Seasons')) return json({ Items: [item('s1', 'Season', 'Season 1', { IndexNumber: 1 })] });
    if (p.endsWith('/Episodes')) return json({ Items: eps });
    if (/\/Users\/u\/Items\/show1$/.test(p)) return json(lib[0]);
    if (p.endsWith('/Similar')) return json({ Items: lib.slice(1, 4) });
    if (p.endsWith('/Items')) return json({ Items: u.searchParams.get('searchTerm') ? lib.filter((x) => x.Name.toLowerCase().includes(u.searchParams.get('searchTerm'))) : lib });
    if (p.includes('/PlaybackInfo')) return json({ PlaySessionId: 'x', MediaSources: [{ Id: 'ms', SupportsDirectPlay: true, MediaStreams: [{ Type: 'Video', RealFrameRate: 24 }, { Type: 'Audio', Index: 1, DisplayTitle: 'English 5.1' }, { Type: 'Subtitle', Index: 2, IsExternal: true, DisplayTitle: 'English', Language: 'eng' }] }] });
    return json({});
  });

  // A pretend TV: what Bridge.kt answers.
  await page.addInitScript((wp) => {
    const S = { setupDone: false }, ST = { media: { shared: { app: 'jellyfin', server: 'http://jf.test', token: 't', userId: 'u', name: 'Nvidia PC', user: 'Dragongamer9' } }, profile: 'p1' };
    const apps = [['com.netflix.ninja', 'Netflix'], ['com.google.android.youtube.tv', 'YouTube'], ['com.disney.disneyplus', 'Disney+'], ['com.amazon.amazonvideo.livingroom', 'Prime Video'], ['com.hulu.livingroomplus', 'Hulu'], ['com.spotify.tv.android', 'Spotify'], ['org.jellyfin.androidtv', 'Jellyfin'], ['com.plexapp.android', 'Plex'], ['org.videolan.vlc', 'VLC'], ['com.google.android.youtube.tvmusic', 'YouTube Music'], ['com.android.vending', 'Play Store'], ['com.tcl.tv', 'Live TV']].map(([pkg, label]) => ({ pkg, label, version: '1.0' }));
    const theater = [
      { id: 'a', title: 'Big Buck Bunny', site: 'youtube.com', len: '9:56', kind: 'Videos', image: wp + '/hearth-peaks.jpg', pct: 60, yt: 'aqz-KE-bpKQ', url: 'https://youtu.be/aqz-KE-bpKQ' },
      { id: 'b', title: 'Rocket launch, full replay', site: 'youtube.com', len: '1:10:04', kind: 'Videos', image: wp + '/hearth-embers.jpg', pct: 12, url: 'https://example.com' },
      { id: 'c', title: 'Zoo cams: the bear enclosure', site: 'twitch.tv', len: 'LIVE', kind: 'Live streams', image: wp + '/hearth-ridge.jpg', url: 'https://twitch.tv/x' },
      { id: 'd', title: 'Space and astronomy', site: 'youtube.com · 24 videos', len: 'Playlist', kind: 'Playlists', image: wp + '/hearth-lanterns.jpg', url: 'https://youtube.com/playlist?list=x' }
    ];
    const later = (id, v) => setTimeout(() => window.hearthTV.done(id, v), 30);
    window.HearthTVNative = {
      settings: () => JSON.stringify(S), settingsMerge: (p) => { Object.assign(S, JSON.parse(p)); return JSON.stringify(S); }, settingsEvent: () => '',
      state: () => JSON.stringify(ST), stateMerge: (p) => { Object.assign(ST, JSON.parse(p)); return JSON.stringify(ST); },
      apps: () => JSON.stringify(apps), appOpen: () => true, appSearch: () => true, appClearCache: (id) => later(id, { ok: true }),
      inputs: () => JSON.stringify([{ id: 'h1', label: 'HDMI 1', kind: 'hdmi', state: 'signal' }, { id: 'h2', label: 'HDMI 2', kind: 'hdmi', state: 'none' }, { id: 'h3', label: 'HDMI 3 (eARC)', kind: 'hdmi', state: 'none' }, { id: 'tv', label: 'Antenna', kind: 'tuner', state: 'signal' }, { id: 'av', label: 'AV', kind: 'av', state: 'none' }]),
      inputSwitch: () => true, liveTv: () => true,
      cleanupStatus: () => JSON.stringify({ ads: { on: false, present: true, packages: [] }, acr: { on: false, present: true, packages: [] }, tcl: { on: false, present: true, packages: [] }, home: { on: false, present: true, packages: [] }, voice: { on: true, present: true, packages: [] }, updates: { on: true, present: false, packages: [] } }),
      cleanupSet: (id) => later(id, { ok: true }), cleanupCheck: (id) => later(id, { fixed: [], adb: true }), adbReachable: (id) => later(id, true),
      tvSetting: (id) => later(id, { ok: true, adb: true }), tvSettingGet: () => '1', openSystem: () => true,
      controllers: () => JSON.stringify([{ name: 'TCL remote', pad: false }, { name: 'Xbox Wireless Controller', pad: true }]),
      theater: () => JSON.stringify(theater), theaterSave: () => {}, theaterAdd: () => {}, openViewer: () => {},
      addPage: () => JSON.stringify({ ok: false }), voice: () => true,
      link: (id, cmd) => later(id, cmd === 'status' ? { ok: true, data: { name: 'Living room TV', peers: [{ id: '1', name: 'Nvidia-PC', online: true, lan: true }, { id: '2', name: 'ROG Ally', online: false, lastSeen: Date.now() - 3600e3 }], nearby: [] } } : cmd === 'pair.open' ? { ok: true, data: { code: '482913' } } : { ok: true, data: {} }),
      linkAvailable: () => true, wake: () => true, findJellyfin: (id) => later(id, [{ Name: 'Nvidia-PC', Address: 'http://jf.test' }]),
      learnMac: (id) => later(id, ''), frameRate: () => {}, version: () => '1.0',
      device: () => JSON.stringify({ model: 'Smart TV Pro', maker: 'TCL', build: '[build]', android: '12', sdk: 31 }),
      updateCheck: (id) => later(id, { current: '1.0', latest: '1.0', newer: false }), updateInstall: (id) => later(id, { ok: true }),
      keepOn: () => {}, log: (l) => console.log('page log: ' + l), logs: () => ''
    };
  }, base + '/assets/tv/theme/backgrounds');

  await page.goto(base + '/assets/tv/index.html');
  await page.waitForTimeout(1500);
  const shot = async (name, wait) => { await page.waitForTimeout(wait || 700); await page.screenshot({ path: path.join(OUT, name + '.jpg'), type: 'jpeg', quality: 80 }); console.log('shot', name); };
  const ev = (code) => page.evaluate(code);

  await shot('1-setup-welcome');
  await ev("hearthTV.act('x');hearthTV.screens.setup.act('next')"); await shot('1-setup-your-pc', 1200);
  await ev("hearthTV.screens.setup.act('next')"); await page.waitForTimeout(400); await ev("hearthTV.close&&hearthTV.close()");
  await ev("hearthTV.screens.setup.act('skip')"); await shot('1-setup-apps');
  await ev("hearthTV.screens.setup.act('next')"); await shot('1-setup-clean-up');
  await ev("hearthTV.set('setupDone', true); hearthTV.stack=[]; hearthTV.go('home',null,{replace:true})"); await shot('2-home', 2000);
  await ev("hearthTV.showInputs()"); await shot('2-home-inputs');
  await ev("hearthTV.close(); hearthTV.showProfiles()"); await shot('2-home-profiles');
  await ev("hearthTV.close(); hearthTV.go('apps')"); await shot('3-apps');
  await ev("hearthTV.go('media')"); await shot('4-my-media', 1500);
  await ev("hearthTV.screens.media.act('open','i1')"); await shot('4-my-media-details');
  await ev("hearthTV.go('show','show1')"); await shot('5-show', 1500);
  await ev("hearthTV.subtitlesPanel()"); await shot('5-show-subtitles');
  await ev("hearthTV.close(); hearthTV.go('theater')"); await shot('6-theater');
  // Send to TV (docs/HEARTH-TV.md): what a Hearth PC sends over Hearth Link, as SendToTv.kt hands it to the page.
  // A link (right-click a link › Send to TV): saved in Theater and played. A video file from Hearth Files: Hearth TV's
  // player, from the TV's own Hearth Link address (a pretend one here that never answers). A show: its page in My Media.
  await page.route('http://tv-link.test/**', () => {});
  await ev("hearthTV.onEvent(JSON.stringify({type:'theaterAdded', play:true, by:'Nvidia-PC', item:hearthTV.json('theater')[0]}))"); await shot('6-theater-sent-link', 1500);
  await ev("hearthTV.back(); hearthTV.onEvent(JSON.stringify({type:'sent', item:{kind:'file', url:'http://tv-link.test/0123/Holiday_2026.mp4', title:'Holiday 2026.mp4', from:'Nvidia-PC', file:{name:'Holiday 2026.mp4', size:1}}}))"); await shot('6-theater-sent-video', 1200);
  await ev("hearthTV.back(); hearthTV.onEvent(JSON.stringify({type:'sent', item:{kind:'media', title:'The Cisco Kid', media:{id:'show1', type:'show'}, from:'Nvidia-PC'}}))"); await shot('6-theater-sent-show', 1800);
  await ev("hearthTV.go('theater')");
  await ev("hearthTV.go('search'); hearthTV.screens.search.act('k','s'); hearthTV.screens.search.act('k','i')"); await shot('7-search', 1500);
  for (const p of ['Home screen', 'TV Cleanup', 'TV inputs', 'Profiles', 'My Media', 'Hearth Link', 'Screensaver', 'Picture and sound', 'Remote and controllers', 'About']) {
    await ev(`hearthTV.go('settings', ${JSON.stringify(p)}); hearthTV.screens.settings.act('page', ${JSON.stringify(p)})`);
    await shot('8-settings-' + p.toLowerCase().replace(/ /g, '-'), 900);
  }
  // Settings › Remote and controllers › Pointer on websites, the Pointer size row chosen.
  await ev("hearthTV.go('settings', 'Remote and controllers'); hearthTV.screens.settings.act('page', 'Remote and controllers')"); await page.waitForTimeout(400);
  await ev("(function(){var b=[].slice.call(document.querySelectorAll('.srow')).filter(function(x){return /Pointer size/.test(x.textContent)})[0]; if(!b) throw new Error('no Pointer size row'); hearthTV.render(b.getAttribute('data-key'))})()");
  await shot('10-pointer-settings');
  for (const want of ['Pointer on websites', 'Start websites in pointer mode', 'Pointer speed', 'Pointer size']) {
    if (!(await ev(`document.body.textContent.includes(${JSON.stringify(want)})`))) errors.push('settings: no "' + want + '"');
  }
  await ev("hearthTV.screens.settings.act('row', [].slice.call(document.querySelectorAll('.srow')).filter(function(x){return /Pointer speed/.test(x.textContent)})[0].getAttribute('data-arg'))"); await page.waitForTimeout(200);
  if (!(await ev("hearthTV.settings.pointer.speed === 'fast'"))) errors.push('settings: Pointer speed did not go Normal › Fast');
  await ev("hearthTV.screens.settings.act('row', [].slice.call(document.querySelectorAll('.srow')).filter(function(x){return /Pointer speed/.test(x.textContent)})[0].getAttribute('data-arg'))");
  await ev("hearthTV.screens.settings.act('row', [].slice.call(document.querySelectorAll('.srow')).filter(function(x){return /Pointer speed/.test(x.textContent)})[0].getAttribute('data-arg'))");
  if (!(await ev("hearthTV.settings.pointer.speed === 'normal'"))) errors.push('settings: Pointer speed did not come back to Normal');
  // Opening a website hands the viewer the look's colours for the pointer.
  await ev("window.__viewer = null; HearthTVNative.openViewer = function (u, t, l) { window.__viewer = [u, t, l]; }; hearthTV.theaterPlay(hearthTV.json('theater')[1])");
  const viewer = await ev('window.__viewer');
  if (!viewer || !JSON.parse(viewer[2] || '{}').accent) errors.push('theater: openViewer got no look for the pointer: ' + JSON.stringify(viewer));
  await ev("hearthTV.set('saver', {after:10, pictures:'wallpapers', clock:true}); hearthTV.go('screensaver')"); await shot('9-screensaver', 1500);
  await ev("hearthTV.back(); hearthTV.set('look','hearth'); hearthTV.applyLook(); hearthTV.go('home')"); await shot('looks-hearth-warm', 1500);
  await ev("hearthTV.set('look','gaming'); hearthTV.set('sub','inferno'); hearthTV.applyLook(); hearthTV.render()"); await shot('looks-gaming-inferno', 900);
  await ev("hearthTV.set('look','steam'); hearthTV.set('sub',''); hearthTV.applyLook(); hearthTV.play({id:'mov1',kind:'movie',title:'Sintel',mins:15,pct:0,year:2010,tags:['4K']})"); await shot('player', 1200);

  // The remote's pointer on a website: pointer.html over a pretend video site, stacked as ViewerActivity stacks them.
  // A stand-in for ViewerActivity (HearthPointerNative) clicks, long-presses and scrolls what's under the pointer.
  const pctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  await pctx.addInitScript(() => {
    if (!location.pathname.endsWith('/pointer.html')) return;
    const site = () => parent.document.getElementById('site');
    const at = (fx, fy) => { const f = site(); return f.contentDocument.elementFromPoint(fx * f.clientWidth, fy * f.clientHeight); };
    parent.pointerCalls = [];
    const note = (n, a) => parent.pointerCalls.push([n].concat(a));
    window.HearthPointerNative = {
      ready: () => note('ready', []), move: (fx, fy) => note('move', [fx, fy]), log: () => {},
      tap: (fx, fy) => { note('tap', [fx, fy]); const el = at(fx, fy); if (el) { el.click(); if (el.focus) el.focus(); } },
      longPress: (fx, fy) => { note('longPress', [fx, fy]); const el = at(fx, fy); if (el) el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true })); },
      rightClick: (fx, fy) => { note('rightClick', [fx, fy]); const el = at(fx, fy); if (el) el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, button: 2 })); },
      scroll: (fx, fy, sx, sy) => { note('scroll', [fx, fy, sx, sy]); site().contentWindow.scrollBy(sx * site().clientWidth, sy * site().clientHeight); },
      back: () => note('back', []), setMode: (m) => note('setMode', [m])
    };
  });
  const pp = await pctx.newPage();
  pp.on('pageerror', (e) => errors.push('pointer: ' + e.message));
  const stage = (o) => base + '/test/stage.html#' + encodeURIComponent(JSON.stringify(o));
  const pshot = async (name, wait) => { await pp.waitForTimeout(wait || 300); await pp.screenshot({ path: path.join(OUT, name + '.jpg'), type: 'jpeg', quality: 80 }); console.log('shot', name); };
  const pf = (code) => pp.frames().find((f) => f.url().includes('/pointer.html')).evaluate(code);
  const said = () => pp.evaluate(() => document.getElementById('site').contentDocument.getElementById('said').textContent);
  const steam = { accent: '#66c0f4', panel: '#1e2633', text: '#dce5ed' };
  await pp.goto(stage({ speed: 'normal', size: 'm', startOn: true, look: steam })); await pp.waitForTimeout(1200);
  if (await pf("hp.pointer.mode") !== 'pointer') errors.push('pointer: websites should start in pointer mode');
  await pshot('pointer-1-website-and-hint');
  // Held ▶ for 0.7 s: it starts slow and speeds up.
  const x0 = await pf('hp.pointer.x');
  await pf("hp.key('right', true)"); await pp.waitForTimeout(700); await pf("hp.key('right', false)");
  const x1 = await pf('hp.pointer.x');
  if (!(x1 > x0 + 40)) errors.push(`pointer: holding ▶ moved it only ${x1 - x0}px`);
  // Onto the player's Play button, then OK.
  const play = await pp.evaluate(() => { const f = document.getElementById('site'); const r = f.contentDocument.getElementById('play').getBoundingClientRect(); return [(r.left + r.width / 2) / f.clientWidth, (r.top + r.height / 2) / f.clientHeight]; });
  await pf(`hp.place(${play[0]}, ${play[1]})`);
  await pf('hp.ok(true)'); await pf('hp.ok(false)');
  await pshot('pointer-2-click-on-play', 120);
  if (await said() !== 'Play clicked') errors.push('pointer: OK did not click the Play button under the pointer (' + await said() + ')');
  // Holding OK: the pointer's menu.
  await pf('hp.ok(true)'); await pp.waitForTimeout(800); await pf('hp.ok(false)');
  if (await pf('hp.pointer.menu') !== 0) errors.push('pointer: holding OK did not open the menu');
  await pshot('pointer-3-long-press-menu');
  await pf('hp.ok(true)'); await pf('hp.ok(false)');
  if (await said() !== 'Menu of the player') errors.push('pointer: Long-press here did not reach the page (' + await said() + ')');
  // The menu's Switch to page mode: the pointer hides and the hint says how to get it back.
  await pf('hp.ok(true)'); await pp.waitForTimeout(800); await pf('hp.ok(false)');
  await pf("hp.key('down', true); hp.key('down', false); hp.key('down', true); hp.key('down', false)");
  await pf('hp.ok(true)'); await pf('hp.ok(false)');
  if (await pf('hp.pointer.mode') !== 'page') errors.push('pointer: Switch to page mode did not switch');
  await pshot('pointer-4-page-mode-hint', 500);
  // Back in pointer mode, holding ▼ at the bottom edge scrolls the page.
  await pf("hp.mode('pointer'); hp.place(0.5, 0.98)");
  await pf("hp.key('down', true)"); await pp.waitForTimeout(900); await pf("hp.key('down', false)");
  const scrolled = await pp.evaluate(() => document.getElementById('site').contentWindow.scrollY);
  if (!(scrolled > 50)) errors.push('pointer: the page did not scroll at the bottom edge (' + scrolled + ')');
  await pf('hp.scroll(-1, 0)');
  // Back: goes back a page (ViewerActivity's part), the menu first when it's open.
  await pf('hp.back()');
  const calls = await pp.evaluate(() => window.pointerCalls.map((c) => c[0]));
  for (const want of ['ready', 'move', 'tap', 'longPress', 'scroll', 'setMode', 'back']) if (!calls.includes(want)) errors.push('pointer: no ' + want + ' call');
  // Large, in another look, over the page's white header: still clear.
  await pp.goto(stage({ size: 'l', startOn: true, look: { accent: '#ff7a1a', panel: '#2a1a14', text: '#fff3e8' } })); await pp.waitForTimeout(1000);
  await pf('hp.place(0.42, 0.045)');
  await pshot('pointer-5-large-on-white');
  await pp.goto(stage({ size: 's', startOn: false, look: steam })); await pp.waitForTimeout(1000);
  if (await pf('hp.pointer.mode') !== 'page') errors.push('pointer: "Start websites in pointer mode" off should start in page mode');
  await pctx.close();

  await browser.close(); server.close();
  if (errors.length) { console.log('::error title=Hearth TV page::' + errors.slice(0, 15).join(' | ').slice(0, 3900)); process.exit(1); }
  console.log('::notice title=Hearth TV page::' + fs.readdirSync(OUT).length + ' pictures, no page errors');
})().catch((e) => { console.log('::error title=Hearth TV page::' + e.message); process.exit(1); });
