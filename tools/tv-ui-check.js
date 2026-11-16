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
  await ev("hearthTV.go('search'); hearthTV.screens.search.act('k','s'); hearthTV.screens.search.act('k','i')"); await shot('7-search', 1500);
  for (const p of ['Home screen', 'TV Cleanup', 'TV inputs', 'Profiles', 'My Media', 'Hearth Link', 'Screensaver', 'Picture and sound', 'Remote and controllers', 'About']) {
    await ev(`hearthTV.go('settings', ${JSON.stringify(p)}); hearthTV.screens.settings.act('page', ${JSON.stringify(p)})`);
    await shot('8-settings-' + p.toLowerCase().replace(/ /g, '-'), 900);
  }
  await ev("hearthTV.set('saver', {after:10, pictures:'wallpapers', clock:true}); hearthTV.go('screensaver')"); await shot('9-screensaver', 1500);
  await ev("hearthTV.back(); hearthTV.set('look','hearth'); hearthTV.applyLook(); hearthTV.go('home')"); await shot('looks-hearth-warm', 1500);
  await ev("hearthTV.set('look','gaming'); hearthTV.set('sub','inferno'); hearthTV.applyLook(); hearthTV.render()"); await shot('looks-gaming-inferno', 900);
  await ev("hearthTV.set('look','steam'); hearthTV.set('sub',''); hearthTV.applyLook(); hearthTV.go('player'); hearthTV.player.item={id:'mov1',kind:'movie',title:'Sintel',mins:15,pct:0,year:2010,tags:['4K']};hearthTV.render()"); await shot('player', 1200);

  await browser.close(); server.close();
  if (errors.length) { console.log('::error title=Hearth TV page::' + errors.slice(0, 15).join(' | ').slice(0, 3900)); process.exit(1); }
  console.log('::notice title=Hearth TV page::' + fs.readdirSync(OUT).length + ' pictures, no page errors');
})().catch((e) => { console.log('::error title=Hearth TV page::' + e.message); process.exit(1); });
