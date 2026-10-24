// Release check: Hearth's page (ui/index.html) must load and draw every screen without a script error.
// A single error can leave the app blank, so a release that fails this isn't published.
//   npm i --no-save playwright@1.48.2 && npx playwright install chromium && node tools/ui-check.js
const { chromium } = require('playwright');
// 'up' / 'down' when a list of names is sorted that way.
const MKSORTED = L => L.every((x, i) => !i || L[i - 1].localeCompare(x) <= 0) ? 'up' : L.every((x, i) => !i || L[i - 1].localeCompare(x) >= 0) ? 'down' : 'neither';
const path = require('path');

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  page.on('console', m => { if (m.type() === 'error' && !/Failed to load resource|net::ERR/.test(m.text())) errors.push('console: ' + m.text()); });
  await page.goto('file://' + path.resolve(__dirname, '../ui/index.html'));
  await page.waitForTimeout(1500);
  const step = async (what, fn, arg) => {
    try { await page.evaluate(fn, arg); await page.waitForTimeout(250); }
    catch (e) { errors.push(`${what}: ${e.message.split('\n')[0]}`); }
  };
  // Every screen, every Settings page, every home layout.
  for (const v of ['home', 'library', 'downloads', 'friends', 'shots', 'stores'])
    await step('screen ' + v, v => { S.layers = []; navigate(v, {}, false); }, v);
  // A game's settings, and its controller layout editor (Hearth Input).
  await step('screen gameset', () => { S.layers = []; navigate('gameset', {game: (GAMES.find(g => !g.cloud) || GAMES[0]).id}, false); });
  await step('screen inputmap', async () => {
    const g = GAMES.find(x => !x.cloud) || GAMES[0];
    imLoadTables(); imLoad(g.id, true);
    await new Promise(r => setTimeout(r, 200));
    imEdit(g.id);
  });
  // A tap or click on one of its sliders sets the value it points at (it did nothing).
  await step('controller layout slider tap', () => {
    const el = document.querySelector('#view [data-slider="im:rs.sensitivity"]'); if (!el) throw new Error('no sensitivity slider');
    S.set['im:rs.sensitivity'] = 2; adjustSlider(el, 0);
    if (imGet('rs.sensitivity') !== 2 || 'im:rs.sensitivity' in S.set) throw new Error('the tap was ignored: ' + imGet('rs.sensitivity'));
  });
  // Beacon (BFME, Age of the Ring, LOTR: Conquest): its library tab, a game page with its mod, Play with…, the mod
  // choice in Game settings, the Strategy (RTS) controller layout it starts with, and its card in Library › Stores.
  await step('beacon tab', () => {
    S.layers = []; S.tab = 'beacon'; navigate('library', {}, false);
    if (!document.querySelector('#view [data-k="tab-beacon"]')) throw new Error('no Beacon tab');
    if (!document.querySelector('#view [data-k="l-beacon:rotwk"]')) throw new Error("Beacon's games aren't in its tab");
  });
  await step('beacon game page', () => {
    S.layers = []; navigate('game', {game: 'beacon:bfme2'}, false);
    if (!document.querySelector('#view .gstat') || !/Edain/.test(document.querySelector('#view .gstats').textContent)) throw new Error("the chosen mod isn't shown");
    if (!document.querySelector('#view [data-k="g-playwith"]')) throw new Error('no Play with…');
  });
  await step('beacon play with', () => {
    beaconPlayWith(G('beacon:aotr'));
    if (!S.layers.includes('sheet') || !/Age of the Ring/.test(document.body.textContent)) throw new Error('no mod sheet');
    closeAll();
  });
  await step('beacon not set up', () => { S.layers = []; navigate('game', {game: 'beacon:bfme1'}, false); if (!document.querySelector('#view [data-k="g-play"][data-act="bcnSetup"]') || !document.querySelector('#view [data-k="g-bsetup"]')) throw new Error('no Set up button'); });
  // Beacon's screens in Hearth (beacon.rs): Game setup with its settings, the CD key prompt, a job with progress,
  // installing a game that isn't set up, community versions, the Mods page with the catalog, the game's log, Find
  // games in the Beacon tab, and the Stores card when Beacon isn't installed.
  const wait = ms => page.waitForTimeout(ms);
  const has = k => page.evaluate(k => !!document.querySelector(`#view [data-k="${k}"]`), k);
  const need = async (what, keys) => { for (const k of keys) if (!(await has(k))) errors.push(`${what}: no ${k}`); };
  const press = k => page.evaluate(k => act(document.querySelector(`#view [data-k="${k}"]`)), k);
  await step('beacon game setup', () => { S.layers = []; navigate('game', {game: 'beacon:rotwk'}, false); act(document.querySelector('#view [data-k="g-bsetup"]')); });
  await wait(500);
  await need('beacon game setup', ['bc-key', 'bc-runner', 'bc-res', 'bc-windowed', 'bc-args', 'bc-locate', 'bc-prepare', 'bc-patch', 'bc-log', 'bc-mods', 'bc-open', 'bc-fix-0']);
  await step('beacon runner and resolution', async () => {
    act(document.querySelector('#view [data-k="bc-runner"]'));
    if (!S.layers.includes('sheet') || !/GE-Proton9-20/.test(document.querySelector('#sheetBox').textContent)) throw new Error('no runner choice');
    sheetPick(1); await new Promise(r => setTimeout(r, 400));
    if (BC.d['beacon:rotwk'].status.runnerChoice !== 'proton:GE-Proton9-20') throw new Error("the runner wasn't saved");
    act(document.querySelector('#view [data-k="bc-res"]'));
    if (!/Match the screen/.test(document.querySelector('#sheetBox').textContent)) throw new Error('no Match the screen');
    sheetPick(2); await new Promise(r => setTimeout(r, 400));
    if (!/x/.test(BC.d['beacon:rotwk'].status.resolution || '')) throw new Error("the resolution wasn't saved");
    act(document.querySelector('#view [data-k="bc-windowed"]')); await new Promise(r => setTimeout(r, 400));
    if (!BC.d['beacon:rotwk'].status.windowed) throw new Error("windowed wasn't saved");
  });
  await step('beacon cd key', async () => {
    S.layers = []; act(document.querySelector('#view [data-k="bc-key"]'));
    if (!S.layers.includes('prompt') || !prompt.secret) throw new Error('no secret prompt for the key');
    prompt.text = 'ABCD-1234-EFGH-5678'; promptSubmit(); await new Promise(r => setTimeout(r, 400));
    if (!BC.d['beacon:rotwk'].status.hasCdKey) throw new Error("the key wasn't saved");
    if (/ABCD-1234/.test(document.body.textContent)) throw new Error('the key is shown on the page');
  });
  await step('beacon prepare', () => { S.layers = []; act(document.querySelector('#view [data-k="bc-prepare"]')); });
  await wait(900);
  await need('beacon prepare (running)', ['bc-job']);
  await wait(2200);
  await step('beacon prepare (done)', () => { if (BC.job) throw new Error('the job never finished'); if (document.querySelector('#view [data-k="bc-job"]')) throw new Error('the progress row stayed'); });
  await step('beacon log', () => { S.layers = []; act(document.querySelector('#view [data-k="bc-log"]')); });
  await wait(400);
  await step('beacon log (shown)', () => { if (S.view !== 'blog' || !document.querySelector('#view [data-k="bl-text"]')) throw new Error('no log'); });
  await step('beacon install', () => { S.layers = []; navigate('game', {game: 'beacon:bfme1'}, false); act(document.querySelector('#view [data-k="g-play"]')); });
  await wait(500);
  await need('beacon install', ['bc-install', 'bc-locate', 'bc-scan', 'bc-patch']);
  await press('bc-locate'); await wait(500);
  await step('beacon located', () => { if (!G('beacon:bfme1').inst || !document.querySelector('#view [data-k="bc-runner"]')) throw new Error("locating the game didn't set it up"); });
  await step('beacon community', () => { S.layers = []; beaconSetup(G('beacon:swgb')); });
  await wait(500);
  await need('beacon community', ['bc-edition', 'bc-step-0', 'bc-step-1', 'bc-vd', 'bc-steam']);
  await step('beacon community sheets', async () => {
    act(document.querySelector('#view [data-k="bc-step-1"]'));
    if (!/Add the file you downloaded/.test(document.querySelector('#sheetBox').textContent)) throw new Error('no Add files');
    closeAll(); act(document.querySelector('#view [data-k="bc-edition"]'));
    if (!/Expanding Fronts/.test(document.querySelector('#sheetBox').textContent)) throw new Error('no editions');
    sheetPick(1); await new Promise(r => setTimeout(r, 400));
    if ((BC.d['beacon:swgb'].community || {}).chosen !== 'saga') throw new Error("the version wasn't saved");
  });
  await step('beacon mods page', () => { S.layers = []; navigate('game', {game: 'beacon:rotwk'}, false); act(document.querySelector('#view [data-k="g-bmods"]')); });
  await wait(500);
  await need('beacon mods page', ['bm-playwith', 'bm-mod-0', 'bm-mod-1', 'bm-none', 'bm-add', 'bm-find', 'bm-cat-0']);
  await step('beacon mods sheets', async () => {
    act(document.querySelector('#view [data-k="bm-cat-0"]'));
    if (!/Open its page in Hearth Browser/.test(document.querySelector('#sheetBox').textContent)) throw new Error('no link to the mod page');
    closeAll(); act(document.querySelector('#view [data-k="bm-mod-1"]'));
    if (!/Remove from Beacon/.test(document.querySelector('#sheetBox').textContent)) throw new Error('no Remove for a linked mod');
    closeAll(); act(document.querySelector('#view [data-k="bm-none"]')); await new Promise(r => setTimeout(r, 500));
    if (BC.d['beacon:rotwk'].mods.selected) throw new Error("No mod wasn't saved");
  });
  await step('beacon find games', () => { S.layers = []; S.tab = 'beacon'; navigate('library', {}, false); act(document.querySelector('#view [data-k="lib-bscan"]')); });
  await wait(900);
  await step('beacon find games (progress)', () => { if (!/Looking for games/.test(document.querySelector('#view [data-k="lib-bscan"]').textContent)) throw new Error('no progress in the Beacon tab'); });
  await wait(2200);
  await step('beacon stores card, not installed', async () => {
    S.layers = []; await loadAccounts(); const was = ACC.beacon; ACC.beacon = {installed: false, app: false, games: 0, setUp: 0, error: null};
    navigate('stores', {}, false);
    const ok = !!document.querySelector('#view [data-k="sa-beacon-get"]') && /Hearth OS comes with Beacon/.test(document.querySelector('#view').textContent);
    ACC.beacon = was; if (!ok) throw new Error('no Get Beacon');
  });
  await step('beacon game settings', async () => {
    S.layers = []; navigate('gameset', {game: 'beacon:rotwk'}, false);
    await new Promise(r => setTimeout(r, 300));
    if (!document.querySelector('#view [data-k="gs-mod"]') || !document.querySelector('#view [data-k="gs-beacon"]')) throw new Error('no Mod / Manage mods rows');
    beaconPickMod(G('beacon:rotwk'), 'gs-mod');
    if (!S.layers.includes('sheet')) throw new Error('no mod choice');
    closeAll();
    imLoadTables(); imLoad('beacon:rotwk', true);
    await new Promise(r => setTimeout(r, 200));
    if (IM.games['beacon:rotwk'].layout !== 'rts') throw new Error("Beacon's game doesn't start with Strategy (RTS)");
    if (IM.games['beacon:conquest'] && IM.games['beacon:conquest'].layout !== 'off') throw new Error('Conquest should start Off');
  });
  await step('beacon stores card', async () => {
    S.layers = []; await loadAccounts(); navigate('stores', {}, false);
    if (!document.querySelector('#view [data-k="sa-beacon-open"]')) throw new Error('no Beacon card');
  });
  await step('beacon leave', () => { S.layers = []; S.tab = 'inst'; navigate('home', {}, false); });
  // ---- Hearth's gaming apps (with the host's "gamingApps" feature): Settings › Controls › Controller Lab, and a
  // game's ⋯ menu › Back up saves / Mods (Beacon's games keep only Beacon's own Mods).
  await step('gaming apps entries', async () => {
    const had = HOSTINFO.features;
    HOSTINFO.features = [...(had || []), 'gamingApps'];
    try {
      S.layers = []; S.settingsTab = 'controller'; navigate('settings', {}, false);
      if (!document.querySelector('#view [data-k="ctl-lab"]')) throw new Error('no Controller Lab row in Settings › Controls');
      const g = GAMES.find(x => x.inst && !x.cloud && x.src !== 'beacon');
      gameOptions(g);
      const t = document.querySelector('#sheet') ? document.querySelector('#sheet').textContent : document.body.textContent;
      closeAll();
      if (!/Back up saves/.test(t) || !/Mods/.test(t)) throw new Error('no Back up saves / Mods in the game menu');
      const b = GAMES.find(x => x.src === 'beacon' && x.inst);
      if (b) { const items = []; gamingGameItems(b, items); if (items.some(i => i.label === 'Mods')) throw new Error("Beacon's game got a second Mods entry"); }
    } finally { HOSTINFO.features = had; S.layers = []; closeAll(); navigate('home', {}, false); }
  });
  // ---- end of Hearth's gaming apps
  // An expired store sign-in (the preview's GOG): the host says so (accounts.expired), the bell keeps an entry with
  // a warning badge, Library › Stores shows "Sign-in expired" with Sign in again, Downloads and an install say why
  // and offer the same, and signing in again clears it all.
  await step('expired sign-in notice', async () => {
    MOCKEXP.gog = Date.now() - 60000;
    onSysEvent('accounts.expired', {store: 'gog', name: 'geralt', at: MOCKEXP.gog});
    await new Promise(r => setTimeout(r, 300));
    if (!isExpired('gog')) throw new Error("the Stores state doesn't say GOG expired");
    if (!NOTIFS.some(n => n.expired === 'gog')) throw new Error('no bell entry');
    if (!document.querySelector('#nbadge.warn')) throw new Error('no warning on the bell');
    if (!/GOG sign-in expired/.test(document.querySelector('#toasts').textContent)) throw new Error('no toast');
  });
  await step('expired stores card', () => {
    S.layers = []; navigate('stores', {}, false);
    const card = document.querySelector('#view [data-k="sa-gog-accSignIn"]');
    if (!card || !/Sign in again/.test(card.textContent)) throw new Error('no Sign in again button');
    if (!/Sign-in expired/.test(card.closest('.scard').textContent)) throw new Error('no "Sign-in expired"');
  });
  await step('expired downloads', () => { S.layers = []; navigate('downloads', {}, false); if (!document.querySelector('#view [data-k="dl-exp-gog"]')) throw new Error('Downloads has no Sign in again'); });
  await step('expired install', () => {
    const g = GAMES.find(x => x.src === 'gog' && !x.inst) || Object.assign({}, GAMES[0], {src: 'gog', inst: false, note: null, title: 'A GOG game'});
    installGame(g);
    if (!S.layers.includes('sheet') || !/GOG sign-in expired/.test(document.body.textContent)) throw new Error('installing from GOG shows no Sign in again');
    closeAll();
  });
  await step('expired bell and sign in again', async () => {
    closeAll(); openLayer('qam'); S.qtab = 'notif'; renderQam();
    const n = document.querySelector('#qam [data-k="n-exp-gog"]');
    if (!n) throw new Error('the bell has no Sign in again entry');
    act(n);
    await new Promise(r => setTimeout(r, 700));
    if (S.view !== 'stores') throw new Error("Sign in again didn't open Library › Stores");
    if (isExpired('gog') || NOTIFS.some(x => x.expired) || document.querySelector('#nbadge.warn')) throw new Error("signing in again didn't clear it");
  });
  await step('expired leave', () => { closeAll(); S.layers = []; navigate('home', {}, false); });
  // Settings › Controls › Handheld Daemon: every section, with advanced settings shown and hidden, a choice list,
  // a slider moved, and a change made (the preview's HHD is a ROG Ally's).
  await step('screen hhd', async () => { S.layers = []; navigate('hhd', {}, false); await new Promise(r => setTimeout(r, 600)); if (!HHDP.schema) throw new Error('Handheld Daemon page has no settings'); });
  const hhdSecs = await page.evaluate(() => typeof hhdSections === 'function' && HHDP.schema ? hhdSections() : []);
  if (!hhdSecs.length) errors.push('hhd: no sections');
  for (const adv of [false, true])
    for (const s of hhdSecs) await step(`hhd ${s}${adv ? ' (advanced)' : ''}`, ([s, adv]) => {
      HHDP.adv = adv; HHDP.sec = s; render(null, true);
      if (!document.querySelector('#view [data-k="hh-adv"]')) throw new Error('page not drawn');
    }, [s, adv]);
  await step('hhd pick', () => { HHDP.sec = 'controllers'; render(null, true); hhdAct('pick', 'controllers.rog_ally.controller_mode.mode'); if (!S.layers.includes('sheet')) throw new Error('no choice list'); closeAll(); });
  await step('hhd slider', () => { HHDP.sec = 'controllers'; render(null, true); const el = document.querySelector('#view [data-slider="hhd:controllers.rog_ally.imu_hz"]'); if (!el) throw new Error('no slider'); adjustSlider(el, 1); });
  await step('hhd switch', async () => { await hhdWrite('controllers.rog_ally.imu', false); if (hv('controllers.rog_ally.imu') !== false) throw new Error('change not shown'); });
  await step('hhd power mode', async () => { HHDP.sec = 'tdp'; await hhdWrite('tdp.unified.tdp.mode', 'custom'); if (!document.querySelector('#view [data-slider="hhd:tdp.unified.tdp.custom.tdp"]')) throw new Error('custom power limit not shown'); });
  await step('hhd leave', () => { S.layers = []; navigate('home', {}, false); });
  const tabs = await page.evaluate(() => typeof STABS !== 'undefined' ? STABS.map(t => t[0]) : []);
  for (const t of tabs) await step('settings ' + t, t => { S.settingsTab = t; navigate('settings', {}, false); }, t);
  // Settings › Multi-seat (Beta Experimental): the set-up with each way to play (another screen, part of this screen
  // with its live preview, layouts and parts, streamed with its picture choice), "press A", and a set-up that's done
  // (streamed players' pairing, split screen sound). The preview's PC is made up (seatsMock).
  await step('multi-seat set-up', () => {
    HOSTINFO.features = [...(HOSTINFO.features || []), 'seats'];   // as on Hearth OS with hearth-seats
    S.layers = []; SE.status = null; SE.draft = null; S.settingsTab = 'seats'; navigate('settings', {}, false); loadSeats(); render(null, true);
    if (S.settingsTab !== 'seats') throw new Error('no Multi-seat page');
    const has = k => !!document.querySelector(`#view [data-k="${k}"]`);
    if (!has('se-p2') || !has('se-c2') || !has('se-setup')) throw new Error('no players, controller rows or Set up');
    if (!seatsSetKind(2, 'split')) throw new Error("couldn't split the screen");
    render(null, true);
    if (!document.querySelector('#view .se-prev') || !has('se-layout') || !has('se-r2')) throw new Error('no split preview, layout or parts');
    seatsAct({v: 'layout'}); seatsAct({v: 'region', n: '2'});
    if (SE.draft.seats[0].mode !== 'split' || SE.draft.seats[1].region === SE.draft.seats[0].region) throw new Error('split players share a part');
    seatsAct({v: 'add'});
    if (SE.draft.seats.length !== 3 || SE.draft.split.layout !== 'grid') throw new Error('a third split player needs four corners');
    if (!seatsSetKind(3, 'stream')) throw new Error("couldn't stream player 3");
    render(null, true);
    if (!has('se-q3') || has('se-c3')) throw new Error('a streamed player needs a picture row and no controller row');
    if (SE.draft.split.layout !== 'grid' || SE.draft.seats.filter(s => s.mode === 'split').length !== 2) throw new Error('split players changed');
    seatsAct({v: 'picture', n: '3'}); if (!S.layers.includes('sheet')) throw new Error('no picture choice'); closeAll();
    seatsAct({v: 'player', n: '2'}); if (!S.layers.includes('sheet') || !/Streamed to another device/.test(document.body.textContent)) throw new Error('no way-to-play choice'); closeAll();
    seatsAct({v: 'claim', n: '2'});
    if (!seatsSetKind(2, 'nested') || SE.draft.seats[0].mode !== 'nested' || SE.draft.seats[1].output === SE.draft.seats[0].output) throw new Error('back to another screen went wrong');
    render(null, true);
  });
  await step('multi-seat set up', () => {
    SE.status = seatsMock('configured'); SE.stream = seatsMock('stream'); render(null, true);
    const has = k => !!document.querySelector(`#view [data-k="${k}"]`);
    if (!has('se-pair3') || !has('se-onlyp1') || !has('se-vol2') || !has('se-c2') || !has('se-remove')) throw new Error('pairing, sound or controller rows missing');
    if (!/hearth-pc Player 3/.test(document.querySelector('#view').textContent)) throw new Error("the streamed player's name isn't shown");
    seatsAct({v: 'pair', n: '3'}); if (!S.layers.includes('sheet') || !/Moonlight/.test(document.body.textContent)) throw new Error('no pairing sheet'); closeAll();
    SE.status = null; SE.stream = null; SE.draft = null; S.layers = []; HOSTINFO.features = HOSTINFO.features.filter(f => f !== 'seats'); navigate('home', {}, false);
  });
  const layouts = await page.evaluate(() => [...(typeof LAYOUTS !== 'undefined' ? LAYOUTS : []), ...(typeof XLAYOUTS !== 'undefined' ? XLAYOUTS : []), ...(typeof MK_LAYOUTS !== 'undefined' ? MK_LAYOUTS : [])].map(l => l.id));
  for (const id of layouts) await step('layout ' + id, id => { LOOK.layout = id; saveLook(); S.layers = []; navigate('home', {}, false); }, id);
  // Library layouts (Settings › You › Library): each shows every game of the tab, moving fills its panel / info bar
  // without drawing the page again, and Back from a game's page finds the same game.
  const libLayouts = await page.evaluate(() => [...(typeof LIB_LAYOUTS !== 'undefined' ? LIB_LAYOUTS : []), ...(typeof XLIB_LAYOUTS !== 'undefined' ? XLIB_LAYOUTS : [])].map(l => l.id));
  for (const id of libLayouts) await step('library layout ' + id, async id => {
    LOOK.libLayout = id; saveLook(); S.layers = []; S.tab = 'all'; S.lfilter = ''; navigate('library', {}, false);
    const games = [...document.querySelectorAll('#view .lwrap [data-f][data-act="game"][data-id]')];
    const shown = new Set(games.map(e => e.dataset.id)).size, want = libGames().length;
    if (id !== 'shelves' ? shown !== want : !shown) throw new Error(`shows ${shown} of ${want} games`);
    setFocus(games[0]); const wrap = document.querySelector('#view .lwrap');
    for (const d of ['right', 'down', 'left', 'up', 'right']) move(d);
    if (!focused || !document.contains(focused)) throw new Error('moving lost the focus');
    if (document.querySelector('#view .lwrap') !== wrap) throw new Error('moving drew the page again');
    const g = focused.dataset.act === 'game' ? focused : games[0]; setFocus(g); const k = g.dataset.k;
    navigate('game', {game: g.dataset.id}); await new Promise(r => setTimeout(r, 100)); goBack(); await new Promise(r => setTimeout(r, 200));
    if (S.view !== 'library' || !focused || focused.dataset.k !== k) throw new Error("Back from the game's page didn't find the game");
  }, id);
  await step('library layout leave', () => { LOOK.libLayout = 'covers'; saveLook(); S.layers = []; navigate('home', {}, false); });
  // Home's Game case shelf: down from a spine reaches Play under the shelf (it went along the spines before).
  await step('game case shelf reaches Play', () => {
    LOOK.layout = 'x-spines'; saveLook(); S.layers = []; navigate('home', {}, false);
    setFocus(document.querySelector('#view .xs-sp')); move('down');
    const k = focused && focused.dataset.k;
    LOOK.layout = 'shelf'; saveLook(); navigate('home', {}, false);
    if (!k || k.startsWith('xs-')) throw new Error('down stayed on the shelf: ' + k);
  });
  // "Played …" follows the clock, not the moment the page loaded (Hearth stays open for days).
  await step('played days ago, days later', () => {
    const real = Date.now, t = real() - 60000;
    Date.now = () => real() + 3 * 864e5;
    let r; try { r = ago(t); } finally { Date.now = real; }
    if (r !== '3 days ago') throw new Error('a game played 3 days ago says ' + r);
    if (hrs(59.7 / 60) !== '1.0 hrs' || hrs(9.97) !== '10 hrs' || hrs(0.5) !== '30 min') throw new Error('play time: ' + [hrs(59.7 / 60), hrs(9.97), hrs(0.5)]);
    if (gb(6e8) !== '600 MB' || gb(512e9) !== '512 GB') throw new Error('sizes: ' + gb(6e8));
  });
  // A collection's name is shown as text in Library's tabs, never read as HTML.
  await step('collection name is text', () => {
    const name = '<img src=x onerror="window.__xss=1">Mine';
    COLS.push({id: 'cx', name}); S.layers = []; S.tab = 'all'; navigate('library', {}, false);
    const tab = document.querySelector('#view [data-k="tab-c:cx"]'), bad = !!document.querySelector('#view .tabs img');
    COLS = COLS.filter(c => c.id !== 'cx'); navigate('home', {}, false);
    if (!tab || tab.textContent !== name || bad) throw new Error('the name went in as HTML');
  });
  // Search, for a profile with parental limits: only the games it's allowed (and the Hidden tab too).
  await step('search with parental limits', () => {
    const was = PROF.s, hid = G('g02').hidden;
    PROF.s = {hasPin: true, list: [{id: 'k', current: true, limits: {allowed: ['g01']}}]};
    G('g02').hidden = true;
    closeAll(); openLayer('search'); query = 'o'; renderSearch();
    const ids = [...document.querySelectorAll('#search .sres [data-id]')].map(e => e.dataset.id);
    S.tab = 'hidden'; const hidden = libGames().map(g => g.id);
    closeAll(); PROF.s = was; G('g02').hidden = hid; S.tab = 'inst';
    if (ids.some(i => i !== 'g01')) throw new Error('search found ' + ids.join(', '));
    if (hidden.length) throw new Error('the Hidden tab shows ' + hidden.join(', '));
  });
  // Search with a real keyboard: any character a title has (' : & . accents), not only letters and digits.
  await step('search types punctuation', () => {
    closeAll(); query = ''; openLayer('search');
    for (const k of ["'", ':', '&', 'é']) document.dispatchEvent(new KeyboardEvent('keydown', {key: k, bubbles: true, cancelable: true}));
    const q = query; closeAll();
    if (q !== "':&é") throw new Error('typed ' + JSON.stringify(q));
  });
  // A real keyboard never brings up the on-screen keyboard: typing opens search with no keyboard on screen, a text
  // prompt opened after keyboard or mouse input has none either, and clicking its field with the mouse doesn't bring
  // it up. A controller's A on the field does; typing on the real keyboard again sends it away.
  {
    const osk = sel => page.evaluate(sel => !!document.querySelector(sel + ' .kb'), sel);
    try {
      await page.evaluate(() => { closeAll(); S.history = []; navigate('home', {}, false); });
      await page.keyboard.press('x'); await page.keyboard.press('/'); await wait(250);
      if (!(await page.evaluate(() => S.layers.includes('search')))) errors.push('keyboard: / did not open search');
      if (await osk('#search')) errors.push('keyboard: the on-screen keyboard came up for search after typing on a keyboard');
      await page.keyboard.type('sta'); await wait(150);
      const q = await page.evaluate(() => query);
      if (q !== 'sta') errors.push('keyboard: typed into search ' + JSON.stringify(q));
      if (await osk('#search')) errors.push('keyboard: typing in search brought up the on-screen keyboard');
      await page.keyboard.press('Escape'); await wait(150);
      await page.evaluate(() => { window.__pr = openPrompt({title: 'Name'}); });
      await wait(250);
      if (await osk('#prompt')) errors.push('keyboard: a text prompt opened after keyboard input showed the on-screen keyboard');
      const box = await page.evaluate(() => { const r = document.querySelector('#prompt [data-k="pk-field"]').getBoundingClientRect(); return {x: r.left + 30, y: r.top + r.height / 2}; });
      await page.mouse.move(box.x - 40, box.y); await page.mouse.move(box.x, box.y, {steps: 3}); await page.mouse.click(box.x, box.y); await wait(200);
      if (await osk('#prompt')) errors.push('mouse: clicking the text field brought up the on-screen keyboard');
      await page.keyboard.type('Al'); await wait(100);
      if ((await page.evaluate(() => prompt && prompt.text)) !== 'Al') errors.push('keyboard: typing into the prompt did not reach it');
      await page.evaluate(() => { setFocus(document.querySelector('#prompt [data-k="pk-field"]'), true); padPressed('a'); });
      await wait(200);
      if (!(await osk('#prompt'))) errors.push('controller: A on a focused text field did not bring up the on-screen keyboard');
      await page.keyboard.type('x'); await wait(150);
      if (await osk('#prompt')) errors.push('keyboard: typing on a real keyboard left the on-screen keyboard up');
      const got = await page.evaluate(async () => { promptSubmit(); return await window.__pr; });
      if (got !== 'Alx') errors.push('prompt: got ' + JSON.stringify(got));
      // A controller opening search gets the keyboard straight away.
      await page.evaluate(() => { closeAll(); padPressed('y'); }); await wait(200);
      if (!(await osk('#search'))) errors.push('controller: Y did not open search with the on-screen keyboard');
      await page.evaluate(() => { closeAll(); LOOK.madeFor = 'auto'; saveLook(); });
      // Settings › Appearance › Made for
      await page.evaluate(() => { S.settingsTab = 'appearance'; navigate('settings', {}, false); });
      await wait(200);
      if (!(await page.evaluate(() => ['auto', 'mk', 'pad'].every(k => document.querySelector(`#view [data-k="ap-mf-${k}"]`))))) errors.push('Appearance: no Made for choice');
      await page.evaluate(() => act(document.querySelector('#view [data-k="ap-mf-pad"]')));
      if (!(await page.evaluate(() => LOOK.madeFor === 'pad' && !document.body.classList.contains('mk-on')))) errors.push('Made for: Controller did not turn mouse & keyboard off');
      await page.evaluate(() => act(document.querySelector('#view [data-k="ap-mf-mk"]')));
      if (!(await page.evaluate(() => document.body.classList.contains('mk-on')))) errors.push('Made for: Mouse & keyboard did not turn it on');
      await page.evaluate(() => { act(document.querySelector('#view [data-k="ap-mf-auto"]')); closeAll(); navigate('home', {}, false); });
    } catch (e) { errors.push('keyboard / on-screen keyboard: ' + e.message.split('\n')[0]); }
  }
  // The experimental layouts sit in drop-downs (closed at first, they stay as you left them): Home's in Appearance,
  // the Library's in Library layout, each with its own Keyboard & mouse one; the upvote buttons are still there.
  await step('experimental drop-downs', () => {
    localStorage.removeItem('hearth.xopen'); for (const k in XOPEN) delete XOPEN[k];
    closeAll(); S.settingsTab = 'appearance'; navigate('settings', {}, false);
    const q = k => document.querySelector(`#view [data-k="${k}"]`);
    if (!q('xd-home-x')) throw new Error('no Experimental layouts drop-down in Appearance');
    if (q('ap-l-x-carousel')) throw new Error('the experimental layouts show before the drop-down is opened');
    act(q('xd-home-x'));
    if (!q('ap-l-x-carousel') || !q('xv-x-carousel')) throw new Error('opening the drop-down shows no layouts or no upvote');
    if (!JSON.parse(localStorage.getItem('hearth.xopen') || '{}')['home-x']) throw new Error("the drop-down's open state isn't kept");
    act(q('xd-home-x')); if (q('ap-l-x-carousel')) throw new Error("the drop-down doesn't close");
    S.settingsTab = 'liblook'; render(null, true);
    if (!q('xd-lib-x')) throw new Error('no Experimental drop-down in Library layout');
    act(q('xd-lib-x')); if (!q('ap-l-x-lib-flow') || !q('xv-x-lib-flow')) throw new Error('the Library drop-down shows no layouts');
    act(q('xd-lib-x'));
    closeAll(); navigate('home', {}, false);
  });
  // Keyboard & mouse layouts (Desk grid, Table, Launcher; Home and Library), driven with a real keyboard and mouse:
  // hover shows Play, right-click opens Hearth's menu (never a browser dialog), Enter plays, Space opens the game,
  // F favorites, Ctrl+K and typing search, double-click plays; Table headers sort and Shift/Ctrl-click pick several;
  // a controller's A still opens the game.
  {
    const ev = f => page.evaluate(f);
    const mk = async (view, id) => {
      await page.evaluate(([view, id]) => { closeAll(); S.history = []; S.tab = 'all'; S.lfilter = ''; S.sort = 'recent'; MK.q = ''; MK.sel.clear(); MK.cur = null;
        window.__launched = null; window.launch = g => { window.__launched = g.id; };
        if (view === 'home') LOOK.layout = id; else { LOOK.layout = 'shelf'; LOOK.libLayout = id; }
        saveLook(); navigate(view, {}, false); }, [view, id]);
      await wait(200);
    };
    const center = sel => page.evaluate(sel => { const e = document.querySelector(sel); if (!e) return null; e.scrollIntoView({block: 'nearest'}); const r = e.getBoundingClientRect(); return {x: r.left + r.width / 2, y: r.top + Math.min(r.height / 2, 40)}; }, sel);
    const realLaunch = await ev(() => { window.__realLaunch = window.launch; window.__dialogs = 0; window.alert = window.confirm = () => { window.__dialogs++; return true; }; return true; });
    try {
      for (const [view, id] of [['library', 'mk-lib-desk'], ['library', 'mk-lib-table'], ['library', 'mk-lib-launcher'], ['home', 'mk-desk'], ['home', 'mk-table'], ['home', 'mk-launcher']]) {
        await mk(view, id);
        const n = await ev(() => ({games: document.querySelectorAll('#view [data-f][data-mkplay]').length, q: !!document.querySelector('#view [data-k="mk-q"]'), err: !document.querySelector('#view .mk')}));
        if (n.err || !n.games || !n.q) errors.push(`${id}: drew ${n.games} games, search box ${n.q}`);
      }
      // Desk grid
      await mk('library', 'mk-lib-desk');
      await page.keyboard.press('Shift');   // a keyboard
      const card = '#view .mkgrid .mkc';
      const id0 = await ev(() => { const c = document.querySelector('#view .mkgrid .mkc'); setFocus(c, true); return c.dataset.id; });
      const fav0 = await ev(() => !!G(document.querySelector('#view .mkgrid .mkc').dataset.id).fav);
      await page.keyboard.press('f'); await wait(100);
      if ((await page.evaluate(id => !!G(id).fav, id0)) === fav0) errors.push('Desk grid: F did not change the favorite');
      await page.evaluate(id => setFocus(document.querySelector(`#view [data-k="mkd-${id}"]`), true), id0);
      await page.keyboard.press('Enter'); await wait(100);
      if ((await ev(() => window.__launched)) !== id0) errors.push('Desk grid: Enter did not play the game');
      await page.keyboard.press(' '); await wait(150);
      if ((await ev(() => S.view + ':' + S.game)) !== 'game:' + id0) errors.push('Desk grid: Space did not open the game');
      await ev(() => goBack()); await wait(200);
      await page.keyboard.press('Control+k'); await wait(100);
      if ((await ev(() => focused && focused.dataset.k)) !== 'mk-q') errors.push('Desk grid: Ctrl+K did not go to the search box');
      await page.keyboard.type('zzqx'); await wait(150);
      if ((await ev(() => [MK.q, document.querySelectorAll('#view .mkgrid .mkc').length].join()))!== 'zzqx,0') errors.push('Desk grid: typing in the search box did not filter: ' + (await ev(() => MK.q)));
      await page.keyboard.press('Escape'); await wait(100);
      if (await ev(() => MK.q)) errors.push('Desk grid: Esc did not clear the search');
      await ev(() => setFocus(document.querySelector('#view .mkgrid .mkc'), true));
      await page.keyboard.press('s'); await wait(100);
      if ((await ev(() => [focused && focused.dataset.k, MK.q].join())) !== 'mk-q,s') errors.push('Desk grid: typing a letter did not start a search');
      await ev(() => { MK.q = ''; mkRender(); });
      let c = await center(card);
      await page.mouse.move(c.x - 50, c.y); await page.mouse.move(c.x, c.y, {steps: 4}); await wait(250);
      if (!(await ev(() => { const o = document.querySelector('#view .mkgrid .mkc:hover .mko'); return o && +getComputedStyle(o).opacity > .5; }))) errors.push('Desk grid: hovering a game shows no Play');
      await page.mouse.click(c.x, c.y, {button: 'right'}); await wait(200);
      if (!(await ev(() => S.layers.includes('mkmenu') && document.querySelectorAll('#mkmenu .mkm-i').length >= 4))) errors.push("Desk grid: right-click did not open Hearth's menu");
      await page.keyboard.press('Escape'); await wait(150);
      if (await ev(() => S.layers.includes('mkmenu'))) errors.push('Desk grid: Esc did not close the menu');
      await ev(() => { window.__launched = null; });
      c = await center(card + ' > b');
      await page.mouse.dblclick(c.x, c.y); await wait(450);
      if (!(await ev(() => window.__launched))) errors.push('Desk grid: double-click did not play');
      if ((await ev(() => S.view)) !== 'library') { errors.push('Desk grid: double-click also opened the game page'); await ev(() => goBack()); }
      // Table
      await mk('library', 'mk-lib-table');
      c = await center('#view [data-k="mkh-name"]'); await page.mouse.click(c.x, c.y); await wait(150);
      const order = await ev(() => [...document.querySelectorAll('#view .mk-tbody [data-id]')].map(e => G(e.dataset.id).title.toLowerCase()));
      if (MKSORTED(order) !== 'up') errors.push('Table: the Name header did not sort A–Z');
      c = await center('#view [data-k="mkh-name"]'); await page.mouse.click(c.x, c.y); await wait(150);
      const order2 = await ev(() => [...document.querySelectorAll('#view .mk-tbody [data-id]')].map(e => G(e.dataset.id).title.toLowerCase()));
      if (MKSORTED(order2) !== 'down') errors.push('Table: clicking Name again did not reverse it');
      const rows = await ev(() => [...document.querySelectorAll('#view .mk-tbody .mkr')].slice(0, 5).map(e => e.dataset.k));
      const rowAt = async k => { const p = await center(`#view [data-k="${k}"] .mkr-n`); return p; };
      let p = await rowAt(rows[0]); await page.mouse.click(p.x, p.y); await wait(120);
      p = await rowAt(rows[2]); await page.keyboard.down('Shift'); await page.mouse.click(p.x, p.y); await page.keyboard.up('Shift'); await wait(120);
      if ((await ev(() => MK.sel.size)) !== 3) errors.push('Table: Shift-click did not pick the range: ' + (await ev(() => MK.sel.size)));
      p = await rowAt(rows[4]); await page.keyboard.down('Control'); await page.mouse.click(p.x, p.y); await page.keyboard.up('Control'); await wait(120);
      if ((await ev(() => MK.sel.size)) !== 4) errors.push('Table: Ctrl-click did not add one');
      if (!(await ev(() => !!document.querySelector('#view .mk-bulk')))) errors.push('Table: no bar for the picked games');
      if ((await ev(() => S.view)) !== 'library') errors.push('Table: clicking rows left the Library');
      await ev(() => { setFocus(document.querySelector('#view .mk-tbody .mkr'), true); padPressed('a'); }); await wait(150);
      if ((await ev(() => S.view)) !== 'game') errors.push("Table: a controller's A on a row did not open the game");
      await ev(() => { closeAll(); goBack(); });
      // Launcher
      await mk('library', 'mk-lib-launcher');
      const pick = await ev(() => { const e = [...document.querySelectorAll('#view .mkl-g')][2]; return e && e.dataset.k; });
      p = await center(`#view [data-k="${pick}"]`); await page.mouse.click(p.x, p.y); await wait(150);
      if (!(await page.evaluate(k => document.querySelector('#mkHero').dataset.id === document.querySelector(`#view [data-k="${k}"]`).dataset.id, pick))) errors.push('Launcher: clicking a game did not show it');
      await ev(() => { window.__launched = null; }); await page.keyboard.press('Enter'); await wait(100);
      if (!(await ev(() => window.__launched))) errors.push('Launcher: Enter did not play the picked game');
      if (await ev(() => window.__dialogs)) errors.push('a browser dialog (alert / confirm) came up');
    } catch (e) { errors.push('keyboard & mouse layouts: ' + e.message.split('\n')[0]); }
    await ev(() => { window.launch = window.__realLaunch; closeAll(); LOOK.layout = 'shelf'; LOOK.libLayout = 'covers'; saveLook(); S.history = []; navigate('home', {}, false); });
  }
  // The lock screen's and the Focus layout's big clocks keep time (they kept the time they were drawn at).
  await step('big clocks keep time', () => {
    S.layers = []; S.view = 'lock'; render(null, true);
    document.querySelector('#view .lk-time').textContent = 'old'; tick();
    const lock = document.querySelector('#view .lk-time').textContent;
    LOOK.layout = 'x-focus'; saveLook(); S.view = 'home'; render(null, true);
    document.querySelector('#view .xf-clock').textContent = 'old'; tick();
    const focus = document.querySelector('#view .xf-clock').textContent;
    LOOK.layout = 'shelf'; saveLook(); navigate('home', {}, false);
    if (lock === 'old' || focus === 'old') throw new Error(`lock ${lock}, focus ${focus}`);
  });
  // Ember › Switch to Hearth OS: the sizes are GiB, as diskpart shrinks C:, and Windows keeps 20 GiB.
  await step('switch sizes in GiB', () => {
    const was = SW.s.free;
    try {
      SW.s.free = 84e9; closeAll(); swPrep();
      if (S.layers.includes('sheet')) throw new Error('84 GB free offered a size (Windows would keep under 20 GiB)');
      SW.s.free = 85 * 2 ** 30; swPrep();
      const t = document.querySelector('#sheetBox').textContent;
      if (!/64 GiB \(69 GB\)/.test(t) || /128 GiB/.test(t)) throw new Error('sizes: ' + t.slice(0, 160));
    } finally { SW.s.free = was; closeAll(); }
  });
  // Starting a game while one that isn't in the library runs (Quick Resume off): a message, not a script error.
  await step('start while an unknown game runs', () => {
    const was = S.running; S.running = 'steam:999999';
    try { launch(G('g01')); } finally { S.running = was; closeAll(); }
  });
  // Game settings that can't be read: a message with Try again (it said "Loading…" for good).
  await step('game settings not read', async () => {
    const orig = MOCK.call;
    MOCK.call = (m, a) => m === 'game.options.get' ? Promise.reject(new Error('No answer')) : orig(m, a);
    try {
      S.layers = []; navigate('gameset', {game: 'g01'}, false);
      await new Promise(r => setTimeout(r, 100));
      if (!document.querySelector('#view [data-k="gs-retry"]')) throw new Error('no Try again');
    } finally { MOCK.call = orig; }
    act(document.querySelector('#view [data-k="gs-retry"]'));
    await new Promise(r => setTimeout(r, 150));
    if (!document.querySelector('#view [data-k="gs-fps"]')) throw new Error('Try again did not load the settings');
    navigate('home', {}, false);
  });
  // Quick Access › Performance, as Hearth OS (?preview=linux): with Handheld Daemon (its own power settings in place
  // of Hearth's), without it (?nohhd: Hearth's power mode and limit, as before), and over a game (the overlay page),
  // where a power change is kept for the game.
  const perfModes = [['hhd', 'preview=linux'], ['no hhd', 'preview=linux&nohhd'], ['overlay hhd', 'preview=linux&overlay'], ['overlay no hhd', 'preview=linux&overlay&nohhd']];
  for (const [name, q] of perfModes) {
    await page.goto('file://' + path.resolve(__dirname, '../ui/index.html') + '?' + q);
    await page.waitForTimeout(1200);
    const hhd = !q.includes('nohhd'), overlay = q.includes('overlay');
    await step(`performance (${name})`, async ([hhd, overlay]) => {
      if (overlay) { await overlayShown(); S.running = (GAMES.find(g => !g.cloud) || GAMES[0]).id; }   // a game plays (the preview runs none)
      else { closeAll(); openLayer('qam'); }
      S.qtab = 'perf'; renderQam(); startPerf();
      await new Promise(r => setTimeout(r, 1500));
      const q = document.querySelector('#qam .qbody');
      const has = sel => !!q.querySelector(sel);
      if (!has('[data-act="fpsLimit"]') || !has('[data-act="perfOverlay"]')) throw new Error("Hearth's frame-rate limit or overlay is missing");
      if (hhd) {
        if (!has('.qhhd [data-key="tdp.unified.tdp.mode"]')) throw new Error("Handheld Daemon's power mode isn't shown");
        if (has('[data-act="tdp"]') || has('[data-act="perfPower"]')) throw new Error("Hearth's own power controls show next to Handheld Daemon's");
        if (has('[data-slider="hhd:tdp.unified.fan.manual_edge.st40"]')) throw new Error('a fan curve point shows in Quick Access');
        // The same in Hearth's panel and over a game: where its whole page is, not a button only one of them had.
        if (has('[data-act="hhdOpen"]') || !has('.qhhd-note')) throw new Error("Quick Access doesn't say where Handheld Daemon's settings are (or has a button only Hearth's panel had)");
      } else {
        if (has('.qhhd')) throw new Error("Handheld Daemon's settings show without it");
        if (!has('[data-act="tdp"]') || !has('[data-act="perfPower"]')) throw new Error("Hearth's power controls are missing");
      }
    }, [hhd, overlay]);
    if (!hhd) continue;
    // The custom mode's limit as a slider (moved with the controller), kept for the game when one plays.
    await step(`performance custom limit (${name})`, async (overlay) => {
      await hhdWrite('tdp.unified.tdp.mode', 'custom');
      const el = document.querySelector('#qam [data-slider="hhd:tdp.unified.tdp.custom.tdp"]');
      if (!el) throw new Error('custom power limit not shown');
      if (overlay && !(PERF.s.gamePower && PERF.s.gamePower.saved)) throw new Error('the change was not kept for the game');
      if (overlay && !document.querySelector('#qam [data-act="gamePowerForget"]')) throw new Error('no way back to the usual power settings');
    }, overlay);
  }
  // Hearth for Android (hosts/android): the page with a pretend Android host (window.nimbusHost, platform 'android',
  // answering like Bridge.kt): only the Settings pages Android has, its own rows there, no Downloads, the Android
  // library tab, the Android power menu and "Add an app".
  const androidSteps = [];
  {
    const ap = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    ap.on('pageerror', e => errors.push('android: ' + e.message));
    ap.on('console', m => { if (m.type() === 'error' && !/Failed to load resource|net::ERR/.test(m.text())) errors.push('android console: ' + m.text()); });
    await ap.addInitScript(() => {
      const day = 864e5, now = Date.now(), L = [];
      const games = [
        {id: 'android:com.retroarch', title: 'RetroArch', source: 'android', installed: true, state: 'installed', art: {}, minutes: 30, lastPlayed: now - day, added: now - 5 * day, cols: []},
        {id: 'android:com.example.racer', title: 'Racer', source: 'android', installed: true, state: 'installed', art: {}, minutes: 0, lastPlayed: 0, added: now - day, cols: [], canRemove: true},
        {id: 'android:com.example.notes', title: 'Notes', source: 'manual', installed: true, state: 'installed', art: {}, added: now - 9 * day, cols: []},
        {id: 'emu:snes:Super Mario World (USA).sfc', title: 'Super Mario World', source: 'emu', system: 'Super Nintendo', installed: true, state: 'installed', art: {}, added: now - 2 * day, cols: []}];
      const R = {
        'host.info': {platform: 'android', name: 'Test Phone', host: 'Hearth (Android)', version: '0.2.1', edition: 'android', features: ['android', 'sendLogs', 'linux', 'linuxDesktop'], setupDone: true, live: false, isDefaultHome: false},
        'sys.status': {wifi: {available: true, on: true, ssid: 'Wi-Fi', bars: 3}, bt: {available: true, on: false}, airplane: false, battery: {present: true, pct: 64, charging: true}, audio: {volume: 40, muted: false}, display: {supported: false}},
        'library.get': {games, sources: {}, collections: [], counts: {android: 2, manual: 1, emu: 1}, runningId: null},
        'profile.get': {name: 'Test Phone', status: null, avatar: 'initials', accent: '#3fd0b8', background: 'default', customAvatar: null, customBackground: null, storeAvatars: {}},
        'android.status': {showAll: true, home: {isDefault: false}, roms: {folder: 'Internal storage › ROMs', count: 1, systems: [{dir: 'snes', name: 'Super Nintendo', count: 1}]}, retroarch: {pkg: 'com.retroarch'}, pads: [{name: 'Xbox Wireless Controller'}]},
        'android.apps': [{id: 'com.example.notes', title: 'Notes'}],
        'android.openSettings': {ok: true},
        'android.openUrl': {ok: true},
        // Hearth Linux (LinuxEnv.kt): installed, nothing running.
        'linux.status': {installed: true, sizeBytes: 4.3e8, distro: 'Debian 13 (trixie)', version: 'Debian 13.1', running: false, jobs: [], busy: null, x11App: false, x11Running: false, supported: true, unsupported: null, freeBytes: 6e10, needBytes: 1.5e9, sdcard: false, error: null},
        'linux.run': {job: 'j1'},
        'linux.apt': {job: 'j2'},
        'linux.stop': {ok: true},
        'linux.openX11': {ok: false, needApp: true, url: 'https://github.com/termux/termux-x11/releases', text: 'Install Termux:X11 (com.termux.x11) to see Linux apps.'},
        'audio.get': {volume: 40, muted: false, max: 100, devices: []},
        'display.get': {supported: false, width: 2400, height: 1080, refreshRate: 120, resolution: '2400 × 1080'},
        'power.get': {present: true, pct: 64, charging: true, mode: null, modes: [], modeSupported: false},
        'system.info': {device: 'Test Phone', hostname: 'Test Phone', os: 'Android', osVersion: '15', osPretty: 'Android 15 (API 35)', cpu: 'Test SoC', cores: 8, memoryBytes: 8e9, storage: {totalBytes: 128e9, freeBytes: 64e9}, uptimeSec: 3600},
        'pad.status': {connected: 1, mouseMode: false, mouseOn: false},
        // The Linux desktop (stage 2): installed, not running; Hearth mode answers with its WebSocket and password.
        'linux.desktop.status': () => window.__desk,
        'linux.desktop.install': {started: true, job: 'j3'},
        'linux.desktop.start': a => a.mode === 'x11' ? {ok: false, needApp: true, url: 'https://github.com/termux/termux-x11/releases', text: 'Install Termux:X11.'}
          : {ok: true, mode: 'hearth', running: true, ready: false, port: 41234, token: 'abc123', password: 'pw123456', width: 1280, height: 800},
        'linux.desktop.stop': {ok: true, running: false},
        'linux.trustX11': {ok: true},
        'pad.raw': a => ({on: !!a.on})};
      window.__desk = {linux: true, installed: true, missing: [], packages: ['xfce4', 'xfce4-terminal', 'dbus-x11', 'tigervnc-standalone-server', 'xkb-data'], mode: 'hearth', x11App: false, installing: false, running: false, method: null, ready: false};
      window.__args = [];
      window.__calls = [];
      window.nimbusHost = {platform: 'android',
        postMessage(m) {
          if (m.type !== 'call') return;
          window.__calls.push(m.method);
          window.__args.push([m.method, m.args || {}]);
          const v = typeof R[m.method] === 'function' ? R[m.method](m.args || {}) : R[m.method];
          const r = m.method in R ? {type: 'result', id: m.id, ok: true, value: JSON.parse(JSON.stringify(v))} : {type: 'result', id: m.id, ok: false, error: "That isn't in Hearth for Android yet"};
          setTimeout(() => L.forEach(f => f({data: r})), 5);
        },
        addEventListener(t, f) { if (t === 'message') L.push(f); }};
    });
    await ap.goto('file://' + path.resolve(__dirname, '../ui/index.html'));
    await ap.waitForTimeout(1500);
    await ap.evaluate(() => { window.__openPrompt = openPrompt; });
    const astep = async (what, fn, arg) => {
      androidSteps.push(what);
      try { await ap.evaluate(fn, arg); await ap.waitForTimeout(200); }
      catch (e) { errors.push(`android ${what}: ${e.message.split('\n')[0]}`); }
    };
    await astep('platform', () => {
      if (!ANDROID || LINUX || APP !== 'Hearth') throw new Error(`not Android: ${HOSTINFO.platform} ${APP}`);
      if (QTABS.some(q => q[0] === 'perf' || q[0] === 'apps')) throw new Error('Quick Access has Performance or Open apps');
      if (GAMES.length !== 4) throw new Error(`the library has ${GAMES.length} games, not 4`);
    });
    await astep('menu', () => {
      S.layers = []; openLayer('menu');
      if (document.querySelector('#menu [data-k="m-downloads"]')) throw new Error('Downloads is in the menu');
      if (document.querySelector('#topnav [data-k="tn-downloads"]')) throw new Error('Downloads is in the top bar');
      closeAll();
    });
    for (const v of ['home', 'library']) await astep('screen ' + v, v => { S.layers = []; navigate(v, {}, false); }, v);
    await astep('library tab', () => {
      S.layers = []; S.tab = 'android'; navigate('library', {}, false);
      if (!document.querySelector('#view [data-k="tab-android"]')) throw new Error('no Android tab');
      if (!document.querySelector('#view [data-k="tab-emu"]')) throw new Error('no Emulators tab');
    });
    await astep('game page', () => { S.layers = []; navigate('game', {game: 'android:com.retroarch'}, false); if (!document.querySelector('#view [data-k="g-play"]')) throw new Error('no Play'); });
    await astep('game options', () => {
      gameOptions(G('android:com.racer') || G('android:com.example.racer'));
      const t = document.querySelector('#sheetBox').textContent;
      if (!/App info/.test(t) || !/Remove from Hearth/.test(t) || /Game settings/.test(t)) throw new Error('the options are wrong: ' + t.slice(0, 200));
      closeAll();
    });
    const shown = await ap.evaluate(() => STABS.map(t => t[0]).filter(stabShown));
    const want = ['profile', 'appearance', 'liblook', 'wifi', 'bluetooth', 'display', 'power', 'controller', 'sources', 'linux', 'system', 'about'];
    if (JSON.stringify(shown) !== JSON.stringify(want)) errors.push(`android settings: shows ${shown.join(', ')}`);
    const rows = {wifi: ['an-wset', 'an-air'], bluetooth: ['an-bt'], display: ['q-vol', 'a-mute', 'an-disp', 'an-sound'], power: ['s-batt', 'an-batt'],
      controller: ['s-an-pad0', 'an-pair'], sources: ['an-all', 's-addexe', 's-rescan', 'an-roms', 's-an-ra', 'an-romsx'], system: ['an-home', 'an-apps', 'sy-send', 'sy-senddev'], about: ['s-host'],
      linux: ['s-lx-ver', 's-lx-size', 'lx-run', 'lx-root', 'lx-pkg', 'lx-upg', 'lx-x11', 'lx-reset', 'lx-remove']};
    for (const t of shown) {
      await astep('settings ' + t, t => { S.layers = []; S.settingsTab = t; navigate('settings', {}, false); onSettingsTab(t); }, t);
      await ap.waitForTimeout(150);
      await astep('settings ' + t + ' rows', ([t, keys]) => {
        render(null, true);
        const miss = keys.filter(k => !document.querySelector(`#view [data-k="${k}"]`));
        if (miss.length) throw new Error('missing ' + miss.join(', '));
        if (t === 'system' && document.querySelector('#view [data-k="s-startOnBoot"]')) throw new Error("Windows' shell switch is on Android's System page");
      }, [t, rows[t] || []]);
    }
    await astep('open a settings page', async () => {
      S.layers = []; S.settingsTab = 'system'; navigate('settings', {}, false); await new Promise(r => setTimeout(r, 50));
      act(document.querySelector('#view [data-k="an-home"]'));
    });
    await astep('power menu', () => {
      S.layers = []; navigate('home', {}, false); openLayer('power');
      if (!document.querySelector('#powerBox [data-k="p-desktop"]') || document.querySelector('#powerBox [data-k="p-shutdown"]')) throw new Error('not the Android power menu');
      closeAll();
    });
    await astep('add an app', async () => {
      await addManual(); await new Promise(r => setTimeout(r, 100));
      if (!S.layers.includes('sheet') || !/Notes/.test(document.querySelector('#sheetBox').textContent)) throw new Error('no app list');
      closeAll();
    });
    await astep('quick access', () => { S.layers = []; openLayer('qam'); S.qtab = 'quick'; renderQam(); closeAll(); });
    // Hearth Linux: a command with its output, the install's progress, Termux:X11 missing, a bad package name.
    await astep('linux command', async () => {
      S.layers = []; S.settingsTab = 'linux'; navigate('settings', {}, false); onSettingsTab('linux'); await new Promise(r => setTimeout(r, 60));
      window.openPrompt = async () => 'uname -a';
      await lxAct('run'); await new Promise(r => setTimeout(r, 30));
      if (LX.job !== 'j1') throw new Error('no job: ' + LX.job);
      onHostMessage({type: 'event', name: 'linux.output', data: {job: 'j1', lines: ['Linux localhost 6.6.0-hearth aarch64 GNU/Linux']}});
      if (!/aarch64/.test(document.querySelector('#lxout').textContent)) throw new Error('no output shown');
      if (!document.querySelector('#view [data-k="lx-stop"]')) throw new Error('no Stop');
      onHostMessage({type: 'event', name: 'linux.exit', data: {job: 'j1', kind: 'run', code: 0}});
      if (LX.job) throw new Error('the job is still running');
    });
    await astep('linux install progress', async () => {
      const st = JSON.parse(JSON.stringify(LX.s));
      LX.s.installed = false; render(null, true);
      if (!document.querySelector('#view [data-k="lx-install"]')) throw new Error('no Install');
      onHostMessage({type: 'event', name: 'linux.progress', data: {kind: 'install', pct: 42, text: 'Downloading Linux… 50 MB of 118 MB'}});
      render(null, true);
      if (!document.querySelector('#lxbar') || document.querySelector('#lxbar').style.width !== '42%') throw new Error('no progress bar');
      if (!document.querySelector('#view [data-k="lx-cancel"]')) throw new Error('no Cancel');
      onHostMessage({type: 'event', name: 'linux.progress', data: {kind: 'install', pct: 60, text: 'Unpacking'}});
      if (document.querySelector('#lxbar').style.width !== '60%') throw new Error('the bar did not move');
      onHostMessage({type: 'event', name: 'linux.done', data: {kind: 'install', ok: true, text: 'Linux is ready', status: st}});
      if (!document.querySelector('#view [data-k="lx-run"]')) throw new Error('not installed after done');
    });
    await astep('linux termux:x11 missing', async () => {
      window.openPrompt = async () => 'xterm';
      await lxAct('x11'); await new Promise(r => setTimeout(r, 30));
      if (!S.layers.includes('sheet') || !/Termux:X11/.test(document.querySelector('#sheetBox').textContent)) throw new Error('no Termux:X11 sheet');
      closeAll();
    });
    await astep('linux packages', async () => {
      const before = window.__calls.filter(m => m === 'linux.apt').length;
      window.openPrompt = async () => 'vim; rm -rf';
      await lxAct('pkg');
      if (window.__calls.filter(m => m === 'linux.apt').length !== before) throw new Error('a bad package name went to the host');
      window.openPrompt = async () => 'vim git';
      await lxAct('pkg'); await new Promise(r => setTimeout(r, 30));
      if (LX.job !== 'j2') throw new Error('apt did not start');
    });
    // The Linux desktop inside Hearth (stage 2): install with progress, the full-screen view with a stand-in for noVNC's
    // RFB, the controller as pointer and keys (and Hearth's own navigation standing still meanwhile), Hearth's keyboard,
    // the bar, touch, reconnecting, back to Hearth, stopping, Termux:X11 missing, the desktop stopping by itself.
    await astep('desktop rows', async () => {
      S.layers = []; S.settingsTab = 'linux'; navigate('settings', {}, false); onSettingsTab('linux'); await new Promise(r => setTimeout(r, 120));
      for (const k of ['lx-desk', 'lx-dmode']) if (!document.querySelector(`#view [data-k="${k}"]`)) throw new Error('no ' + k);
    });
    await astep('desktop install', async () => {
      window.__desk.installed = false; window.__desk.missing = ['xfce4', 'tigervnc-standalone-server'];
      await lxAct('desk');
      if (!S.layers.includes('sheet') || !/tigervnc-standalone-server/.test(document.querySelector('#sheetBox').textContent)) throw new Error('no install sheet');
      sheetPick(0); await new Promise(r => setTimeout(r, 60));
      if (!window.__calls.includes('linux.desktop.install')) throw new Error('install not asked');
      if (!LX.s.busy || LX.s.busy.kind !== 'desktop') throw new Error('not busy installing');
      onHostMessage({type: 'event', name: 'linux.progress', data: {kind: 'desktop', pct: 37, text: 'Retrieving file 9 of 200'}});
      render(null, true);
      if (!document.querySelector('#view [data-k="lx-dprog"]') || document.querySelector('#lxbar').style.width !== '37%') throw new Error('no desktop progress');
      if (!document.querySelector('#view [data-k="lx-cancel"]')) throw new Error('no Cancel');
      onHostMessage({type: 'event', name: 'linux.progress', data: {kind: 'desktop', pct: 64, text: 'Installing xfce4-panel'}});
      if (document.querySelector('#lxbar').style.width !== '64%') throw new Error('the bar did not move');
      window.__desk.installed = true; window.__desk.missing = [];
      const st = JSON.parse(JSON.stringify(LX.s)); st.busy = null;
      onHostMessage({type: 'event', name: 'linux.done', data: {kind: 'desktop', ok: true, text: 'The Linux desktop is installed', status: st}});
      await new Promise(r => setTimeout(r, 80));
      if (document.querySelector('#view [data-k="lx-dprog"]') || !/Linux desktop/.test(document.querySelector('#view [data-k="lx-desk"]').textContent)) throw new Error('still installing');
    });
    await astep('desktop opens', async () => {
      window.__rfbs = [];
      window.HearthRFB = class extends EventTarget {
        constructor(t, url, o) { super(); this.url = url; this.opts = o; this.keys = []; this.pointer = []; window.__rfbs.push(this); setTimeout(() => this.dispatchEvent(new CustomEvent('connect', {detail: {}})), 20); }
        sendKey(s, c, d) { if (d === undefined) { this.keys.push([s, true]); this.keys.push([s, false]); } else this.keys.push([s, d]); }
        sendPointer(x, y, m) { this.pointer.push([x, y, m]); }
        sendCredentials() {} focus() {} blur() {}
        disconnect() { this.dispatchEvent(new CustomEvent('disconnect', {detail: {clean: true}})); }
      };
      window.__args = [];
      await deskOpen();
      if (!DESK.open || !document.querySelector('#desk.open')) throw new Error('the view did not open');
      const start = window.__args.find(a => a[0] === 'linux.desktop.start');
      if (!start || start[1].mode !== 'hearth' || !(start[1].width > 0) || !(start[1].height > 0)) throw new Error('bad start: ' + JSON.stringify(start));
      if (!window.__args.some(a => a[0] === 'pad.raw' && a[1].on === true)) throw new Error('controller not switched to raw');
      if (window.__rfbs.length) throw new Error('connected before the desktop was ready');
      if (!/Starting/.test(document.querySelector('#deskMsg').textContent)) throw new Error('no starting message');
      onHostMessage({type: 'event', name: 'linux.desktop', data: {state: 'ready', mode: 'hearth', port: 41234, token: 'abc123', password: 'pw123456', width: 1280, height: 800, ok: true, running: true, ready: true}});
      await new Promise(r => setTimeout(r, 120));
      const rfb = window.__rfbs[0];
      if (!rfb || rfb.url !== 'ws://127.0.0.1:41234/abc123' || rfb.opts.credentials.password !== 'pw123456') throw new Error('noVNC not connected right: ' + (rfb && rfb.url));
      if (document.querySelector('#deskMsg').classList.contains('show')) throw new Error('the message stayed');
    });
    await astep('desktop controller: keys, not navigation', async () => {
      const rfb = window.__rfbs[0], view = S.view, k0 = focused && focused.dataset.k; rfb.keys = [];
      onHostMessage({type: 'event', name: 'pad', data: {b: 'down', down: true}});
      onHostMessage({type: 'event', name: 'pad', data: {b: 'down', down: false}});
      if (S.view !== view || S.layers.length || (focused && focused.dataset.k) !== k0) throw new Error("Hearth's navigation moved");
      if (JSON.stringify(rfb.keys) !== JSON.stringify([[0xff54, true], [0xff54, false]])) throw new Error('no Down arrow: ' + JSON.stringify(rfb.keys));
      rfb.keys = [];
      for (const [b, d] of [['x', true], ['x', false], ['start', true], ['start', false]]) onHostMessage({type: 'event', name: 'pad', data: {b, down: d}});
      if (JSON.stringify(rfb.keys) !== JSON.stringify([[0xff1b, true], [0xff1b, false], [0xffe9, true], [0xffbe, true], [0xffbe, false], [0xffe9, false]])) throw new Error('X/Start keys: ' + JSON.stringify(rfb.keys));
    });
    await astep('desktop controller: pointer', async () => {
      const rfb = window.__rfbs[0]; rfb.pointer = [];
      const x0 = DESK.x;
      onHostMessage({type: 'event', name: 'padAxes', data: {lx: 0, ly: 0, rx: 1, ry: 0, lt: 0, rt: 0}});
      await new Promise(r => setTimeout(r, 300));
      onHostMessage({type: 'event', name: 'padAxes', data: {lx: 0, ly: 0, rx: 0, ry: 0, lt: 0, rt: 0}});
      if (!(DESK.x > x0 + 40)) throw new Error(`the pointer did not move (${x0} → ${DESK.x})`);
      if (!rfb.pointer.length || rfb.pointer[rfb.pointer.length - 1][0] !== Math.round(DESK.x)) throw new Error('pointer not sent');
      const x1 = DESK.x; await new Promise(r => setTimeout(r, 150));
      if (DESK.x !== x1) throw new Error('the pointer kept moving');
      rfb.pointer = [];
      const masks = () => rfb.pointer.map(p => p[2]);
      onHostMessage({type: 'event', name: 'pad', data: {b: 'a', down: true}});
      if (masks().at(-1) !== 1) throw new Error('A is not the left button');
      onHostMessage({type: 'event', name: 'pad', data: {b: 'a', down: false}});
      onHostMessage({type: 'event', name: 'pad', data: {b: 'b', down: true}});
      onHostMessage({type: 'event', name: 'pad', data: {b: 'b', down: false}});
      onHostMessage({type: 'event', name: 'pad', data: {b: 'rb', down: true}});
      onHostMessage({type: 'event', name: 'pad', data: {b: 'rb', down: false}});
      if (JSON.stringify(masks()) !== JSON.stringify([1, 0, 4, 0, 16, 0])) throw new Error('buttons: ' + JSON.stringify(masks()));
      rfb.keys = [];
      onHostMessage({type: 'event', name: 'padAxes', data: {lx: 0, ly: 0, rx: 0, ry: 0, lt: 0, rt: 1}});
      await new Promise(r => setTimeout(r, 60));
      onHostMessage({type: 'event', name: 'padAxes', data: {lx: 0, ly: 0, rx: 0, ry: 0, lt: 0, rt: 0}});
      await new Promise(r => setTimeout(r, 60));
      if (JSON.stringify(rfb.keys) !== JSON.stringify([[0xff0d, true], [0xff0d, false]])) throw new Error('RT is not Enter: ' + JSON.stringify(rfb.keys));
    });
    await astep('desktop keyboard', async () => {
      window.openPrompt = window.__openPrompt;           // earlier steps put a stand-in there
      const rfb = window.__rfbs[0]; rfb.keys = [];
      onHostMessage({type: 'event', name: 'pad', data: {b: 'y', down: true}});
      onHostMessage({type: 'event', name: 'pad', data: {b: 'y', down: false}});
      await new Promise(r => setTimeout(r, 30));
      if (!S.layers.includes('prompt') || getComputedStyle(document.querySelector('#prompt')).zIndex <= getComputedStyle(document.querySelector('#desk')).zIndex) throw new Error("Hearth's keyboard isn't over the desktop: " + S.layers.join() + ' ' + getComputedStyle(document.querySelector('#prompt')).zIndex + ' ' + DESK.open);
      // The prompt has the controller now (raw presses work there; releases are ignored).
      onHostMessage({type: 'event', name: 'pad', data: {b: 'right', down: true}});
      onHostMessage({type: 'event', name: 'pad', data: {b: 'right', down: false}});
      if (rfb.keys.length) throw new Error('keys reached Linux while typing');
      prompt.text = 'ls -lé'; promptSubmit(); await new Promise(r => setTimeout(r, 60));
      const want = [...'ls -l'].map(c => c.charCodeAt(0)).concat([0xe9]).flatMap(c => [[c, true], [c, false]]);
      if (JSON.stringify(rfb.keys) !== JSON.stringify(want)) throw new Error('typed: ' + JSON.stringify(rfb.keys));
    });
    await astep('desktop bar', async () => {
      const rfb = window.__rfbs[0]; rfb.keys = [];
      onHostMessage({type: 'event', name: 'pad', data: {b: 'guide', down: true}});
      onHostMessage({type: 'event', name: 'pad', data: {b: 'guide', down: false}});
      if (!DESK.bar || !document.querySelector('#deskBar.show')) throw new Error('no bar');
      for (const k of ['dk-kbd', 'dk-Control_L', 'dk-Alt_L', 'dk-Super_L', 'dk-Escape', 'dk-Tab', 'dk-stop', 'dk-home']) if (!document.querySelector(`#deskBar [data-k="${k}"]`)) throw new Error('no ' + k);
      onHostMessage({type: 'event', name: 'pad', data: {b: 'right', down: true}});
      onHostMessage({type: 'event', name: 'pad', data: {b: 'a', down: true}});
      if (!DESK.mods.Control_L || !document.querySelector('#deskBar [data-k="dk-Control_L"].on')) throw new Error('Ctrl not held');
      act(document.querySelector('#deskBar [data-k="dk-Tab"]'));
      if (JSON.stringify(rfb.keys) !== JSON.stringify([[0xffe3, true], [0xff09, true], [0xff09, false], [0xffe3, false]])) throw new Error('Ctrl+Tab: ' + JSON.stringify(rfb.keys));
      onHostMessage({type: 'event', name: 'pad', data: {b: 'b', down: true}});
      if (DESK.bar) throw new Error('B did not close the bar');
      rfb.pointer = [];
      onHostMessage({type: 'event', name: 'pad', data: {b: 'b', down: false}});
      if (rfb.pointer.length) throw new Error("closing the bar's B clicked");
    });
    await astep('desktop touch', async () => {
      const rfb = window.__rfbs[0], el = document.querySelector('#deskTouch');
      const pe = (type, id, x, y) => el.dispatchEvent(new PointerEvent(type, {pointerId: id, pointerType: 'touch', clientX: x, clientY: y, bubbles: true, cancelable: true, isPrimary: id === 1}));
      rfb.pointer = [];
      pe('pointerdown', 1, 300, 300); pe('pointerup', 1, 300, 300);
      if (JSON.stringify(rfb.pointer.map(p => p[2])) !== '[1,0]') throw new Error('tap: ' + JSON.stringify(rfb.pointer));
      await new Promise(r => setTimeout(r, 350));
      rfb.pointer = []; const x0 = DESK.x;
      pe('pointerdown', 1, 300, 300); pe('pointermove', 1, 360, 300); pe('pointermove', 1, 420, 300); pe('pointerup', 1, 420, 300);
      if (!(DESK.x > x0 + 50) || rfb.pointer.some(p => p[2])) throw new Error('drag did not move the pointer (or clicked)');
      await new Promise(r => setTimeout(r, 350));
      rfb.pointer = [];
      pe('pointerdown', 1, 300, 300); pe('pointerdown', 2, 360, 300); pe('pointerup', 1, 300, 300); pe('pointerup', 2, 360, 300);
      if (JSON.stringify(rfb.pointer.map(p => p[2])) !== '[4,0]') throw new Error('two-finger tap: ' + JSON.stringify(rfb.pointer));
      rfb.pointer = [];
      pe('pointerdown', 1, 300, 300); pe('pointerdown', 2, 360, 300); pe('pointermove', 1, 300, 360); pe('pointermove', 2, 360, 360); pe('pointerup', 1, 300, 360); pe('pointerup', 2, 360, 360);
      if (!rfb.pointer.some(p => p[2] === 8)) throw new Error('two-finger scroll: ' + JSON.stringify(rfb.pointer));
    });
    await astep('desktop reconnects', async () => {
      window.__desk.running = true; window.__desk.method = 'hearth';
      const n = window.__rfbs.length;
      window.__rfbs[n - 1].dispatchEvent(new CustomEvent('disconnect', {detail: {clean: false}}));
      await new Promise(r => setTimeout(r, 700));
      if (window.__rfbs.length !== n + 1 || window.__rfbs[n].url !== 'ws://127.0.0.1:41234/abc123') throw new Error('did not connect again');
    });
    await astep('desktop back to Hearth', async () => {
      window.__args = [];
      onHostMessage({type: 'event', name: 'pad', data: {b: 'select', down: true}});
      act(document.querySelector('#deskBar [data-k="dk-home"]'));
      if (DESK.open || document.querySelector('#desk.open')) throw new Error('still open');
      if (!window.__args.some(a => a[0] === 'pad.raw' && a[1].on === false)) throw new Error('controller still raw');
      if (window.__calls.filter(m => m === 'linux.desktop.stop').length) throw new Error('Back to Hearth stopped the desktop');
      const k0 = focused && focused.dataset.k;
      onHostMessage({type: 'event', name: 'pad', data: {b: 'down'}});
      if ((focused && focused.dataset.k) === k0) throw new Error("Hearth's navigation didn't come back");
    });
    await astep('desktop stops by itself', async () => {
      await deskOpen();
      onHostMessage({type: 'event', name: 'linux.desktop', data: {state: 'ready', mode: 'hearth', port: 41234, token: 'abc123', password: 'pw123456', width: 1280, height: 800}});
      await new Promise(r => setTimeout(r, 80));
      onHostMessage({type: 'event', name: 'linux.desktop', data: {state: 'stopped', mode: 'hearth', code: 0}});
      if (!/stopped/.test(document.querySelector('#deskMsg').textContent)) throw new Error('no stopped message');
      onHostMessage({type: 'event', name: 'pad', data: {b: 'b', down: true}});
      if (DESK.open) throw new Error('B did not go back to Hearth');
    });
    await astep('desktop stop from the bar', async () => {
      await deskOpen();
      deskBarToggle(true); act(document.querySelector('#deskBar [data-k="dk-stop"]'));
      await new Promise(r => setTimeout(r, 30));
      if (DESK.open || !window.__calls.includes('linux.desktop.stop')) throw new Error('not stopped');
    });
    await astep('desktop in termux:x11 (missing)', async () => {
      deskSetMode('x11'); await deskOpen(); await new Promise(r => setTimeout(r, 30));
      if (DESK.open || !S.layers.includes('sheet') || !/Termux:X11/.test(document.querySelector('#sheetBox').textContent)) throw new Error('no Termux:X11 sheet');
      closeAll(); deskSetMode('hearth');
    });
    await astep('linux remove asks first', () => { lxAct('remove'); if (!S.layers.includes('sheet')) throw new Error('no confirmation'); closeAll(); });
    await astep('controller from the host', () => { onHostMessage({type: 'event', name: 'padHost', data: {connected: 1}}); onHostMessage({type: 'event', name: 'pad', data: {b: 'down'}}); onHostMessage({type: 'event', name: 'pad', data: {b: 'b'}}); });
    const asked = await ap.evaluate(() => [...new Set(window.__calls)]);
    for (const m of ['host.info', 'library.get', 'android.status', 'android.openSettings', 'android.apps', 'linux.status', 'linux.run', 'linux.apt', 'linux.openX11', 'linux.desktop.status', 'linux.desktop.install', 'linux.desktop.start', 'linux.desktop.stop', 'pad.raw']) if (!asked.includes(m)) errors.push(`android: never called ${m}`);
    await ap.close();
  }
  // Hearth Linux in the browser preview (?preview=android, no app): the pretend Linux installs and runs a command.
  {
    const lp = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    lp.on('pageerror', e => errors.push('linux preview: ' + e.message));
    await lp.goto('file://' + path.resolve(__dirname, '../ui/index.html') + '?preview=android');
    await lp.waitForTimeout(800);
    const lstep = async (what, fn) => {
      androidSteps.push('preview ' + what);
      try { await lp.evaluate(fn); } catch (e) { errors.push(`linux preview ${what}: ${e.message.split('\n')[0]}`); }
    };
    await lstep('tab', async () => {
      if (!ANDROID || !lxOn()) throw new Error('not the Android preview');
      S.layers = []; S.settingsTab = 'linux'; navigate('settings', {}, false); onSettingsTab('linux');
      await new Promise(r => setTimeout(r, 300));
      if (!document.querySelector('#view [data-k="lx-install"]')) throw new Error('no Install');
    });
    await lstep('install', async () => {
      await lxAct('install'); await new Promise(r => setTimeout(r, 1700));
      if (!LX.s.installed || !document.querySelector('#view [data-k="lx-run"]')) throw new Error('did not install');
    });
    await lstep('run', async () => {
      window.openPrompt = async () => 'uname -a';
      await lxAct('run'); await new Promise(r => setTimeout(r, 600));
      if (!/aarch64/.test(LX.out.join('\n'))) throw new Error('no output: ' + LX.out.join(' | '));
    });
    await lstep('desktop', async () => {
      await lxAct('desk'); await new Promise(r => setTimeout(r, 30));
      if (!S.layers.includes('sheet')) throw new Error('no install sheet');
      sheetPick(0); await new Promise(r => setTimeout(r, 1600));
      if (!DESK.s || !DESK.s.installed) throw new Error('the desktop did not install');
      await deskOpen(); await new Promise(r => setTimeout(r, 150));
      if (!(DESK.rfb instanceof DeskPreviewRFB) || !document.querySelector('#deskScreen canvas')) throw new Error('no pretend desktop');
      onSysEvent('pad', {b: 'a'}); await new Promise(r => setTimeout(r, 120));
      if (JSON.stringify(DESK.rfb.pointer.map(p => p[2]).slice(-2)) !== '[1,0]') throw new Error('A did not click: ' + JSON.stringify(DESK.rfb.pointer));
      deskClose(true); await new Promise(r => setTimeout(r, 150));
      if (DESK.open || DESK.s.running) throw new Error('did not stop');
    });
    await lp.close();
  }
  // Game Mode, Guide over a game (gamemode.rs main_page_reads_pad): while Hearth says its window isn't on screen (set
  // aside for a game, or Quick Access open over it), the controller this page reads itself does nothing here; a button
  // still held when it gets the controller back isn't a new press; on screen again, Guide opens Quick Access as always.
  // Its own page: the one above was last left on the overlay's (?overlay), which never reads the controller itself.
  const gp = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  gp.on('pageerror', e => errors.push('guide over a game: ' + e.message));
  try {
    await gp.goto('file://' + path.resolve(__dirname, '../ui/index.html'));
    await gp.waitForTimeout(1200);
    await gp.evaluate(() => {
      const pad = { id: 'Xbox Wireless Controller (test)', index: 0, connected: true, mapping: 'standard', axes: [0, 0, 0, 0],
                    buttons: Array.from({ length: 17 }, () => ({ pressed: false, value: 0 })) };
      window.__pad = pad;
      navigator.getGamepads = () => [pad];
      window.__sent = [];
      const realSend = window.send;
      window.send = m => { window.__sent.push(m); try { return realSend(m); } catch (e) { return null; } };
      closeAll(); S.history = []; navigate('home', {}, false);
    });
    const guide = on => gp.evaluate(on => { window.__pad.buttons[16] = { pressed: on, value: on ? 1 : 0 }; }, on);
    const state = () => gp.evaluate(() => ({ qam: S.layers.includes('qam'), resume: window.__sent.filter(m => m && m.type === 'resume').length }));
    await gp.evaluate(() => onSysEvent('hostScreen', { pads: false }));
    await guide(true); await gp.waitForTimeout(200); await guide(false); await gp.waitForTimeout(200);
    let st = await state();
    if (st.qam) errors.push("guide over a game: Hearth's hidden window opened its own Quick Access");
    await guide(true); await gp.waitForTimeout(200); await guide(false); await gp.waitForTimeout(200);
    st = await state();
    if (st.qam || st.resume) errors.push(`guide over a game, second press: the hidden window acted (Quick Access ${st.qam}, back-to-game ${st.resume})`);
    await guide(true); await gp.waitForTimeout(150);
    await gp.evaluate(() => onSysEvent('hostScreen', { pads: true }));
    await gp.waitForTimeout(200);
    st = await state();
    if (st.qam) errors.push('a Guide still held when Hearth came back was taken for a new press');
    await guide(false); await gp.waitForTimeout(150);
    await guide(true); await gp.waitForTimeout(200); await guide(false); await gp.waitForTimeout(200);
    st = await state();
    if (!st.qam) errors.push('Hearth on screen: Guide no longer opens its Quick Access');
    await gp.evaluate(() => { closeAll(); delete navigator.getGamepads; });
  } catch (e) { errors.push('guide over a game: ' + e.message.split('\n')[0]); }
  await gp.close();

  // Quick Access is one panel: Hearth's own (Guide with Hearth on screen, the starting screen, LAST RESORT) and the
  // overlay over a game (?overlay=full, HHD's overlay app or Hearth's overlay window) draw the same size, tabs,
  // sections and buttons in the same order, with the same actions and the same controller navigation. Screenshots of
  // both go to tools/qa-shots (release.yml posts them as notices).
  const qaShots = path.resolve(__dirname, 'qa-shots');
  try { require('fs').mkdirSync(qaShots, { recursive: true }); } catch {}
  const qaPanel = async (which) => {
    const qp = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    qp.on('pageerror', e => errors.push(`quick access (${which}): ${e.message}`));
    const out = {};
    try {
      await qp.goto('file://' + path.resolve(__dirname, '../ui/index.html') + (which === 'overlay' ? '?preview=linux&overlay=full' : '?preview=linux'));
      await qp.waitForTimeout(1200);
      // A game running, another put aside (Quick Resume), Quick Resume and Pause available.
      await qp.evaluate(() => {
        HOSTINFO.features = ['quickResume', 'pause']; QR.on = true;
        const gs = GAMES.filter(g => !g.cloud);
        window.__game = gs[0].id; QR.list = [{ id: gs[1].id, frozenAt: Date.now() - 6e5 }];
        const real = MOCK.call;
        MOCK.call = (m, a) => m === 'perf.stats' ? real(m, a).then(r => Object.assign({}, r, { runningId: window.__game, runningSince: Date.now() - 6e5 })) : real(m, a);
        S.running = window.__game; S.runningSince = Date.now() - 6e5;
      });
      // How it opens: Guide in Hearth's window; Hearth's "shown" over a game.
      const open = async () => {
        if (which === 'overlay') await qp.evaluate(async () => { await overlayShown(); });
        else await qp.evaluate(() => { closeAll(); S.history = []; navigate('home', {}, false); input('guide'); });
        await qp.waitForTimeout(400);
      };
      const read = () => qp.evaluate(() => ({
        tab: S.qtab,
        tabs: [...document.querySelectorAll('#qam .qtab')].map(e => e.dataset.tab).join(' '),
        buttons: [...document.querySelectorAll('#qam .qbody [data-f]')].map(e => `${e.dataset.k}:${e.dataset.act || e.dataset.slider || ''}:${e.textContent.replace(/\s+/g, ' ').trim()}`),
        focus: focused && focused.dataset.k,
      }));
      await open();
      out.rect = await qp.evaluate(() => { const r = document.querySelector('#qam').getBoundingClientRect(); return `${Math.round(r.left)},${Math.round(r.top)} ${Math.round(r.width)}x${Math.round(r.height)}`; });
      out.now = await read();
      // Controller: down through the panel, then RB through every tab.
      out.nav = await qp.evaluate(() => { const ks = []; for (let i = 0; i < 6; i++) { input('down'); ks.push(focused && focused.dataset.k); } return ks.join(' '); });
      out.tabsVisited = [];
      for (let i = 0; i < 6; i++) { await qp.evaluate(() => input('rb')); await qp.waitForTimeout(await qp.evaluate(() => S.qtab === 'perf' || S.qtab === 'apps') ? 1500 : 200); const r = await read(); out.tabsVisited.push(r.tab); out['tab-' + r.tab] = out['tab-' + r.tab] || r.buttons.join(' | ').replace(/\d+ ?(min|%|°C|W|MHz|MB|GB|fps)\b/g, '#'); }
      out.tabsVisited = out.tabsVisited.join(' ');
      // The screenshot: Now playing with the game running.
      await qp.evaluate(() => { S.qtab = 'now'; renderQam(); setFocus(document.querySelector('#qam .qbody [data-f]'), true); });
      await qp.waitForTimeout(300);
      for (const q of [55, 40, 30, 22, 15]) {
        const buf = await qp.screenshot({ type: 'jpeg', quality: q, clip: { x: 1280 - 460, y: 0, width: 460, height: 800 } });
        if (buf.toString('base64').length <= 9 * 3800 || q === 15) { require('fs').writeFileSync(path.join(qaShots, `${which}.jpg`), buf); break; }
      }
      // Nothing running: the same "no game" card, buttons and Quick Resume list.
      await qp.evaluate(() => { window.__game = null; S.running = null; closeAll(); });
      await open();
      out.idle = await read();
    } catch (e) { errors.push(`quick access (${which}): ${e.message.split('\n')[0]}`); }
    await qp.close();
    return out;
  };
  {
    const main = await qaPanel('main'), ov = await qaPanel('overlay');
    const same = (what, a, b) => { if (JSON.stringify(a) !== JSON.stringify(b)) errors.push(`quick access: ${what} differ between Hearth's panel and the overlay:\n  panel:   ${JSON.stringify(a)}\n  overlay: ${JSON.stringify(b)}`); };
    same('size and position', main.rect, ov.rect);
    for (const k of ['now', 'idle']) { same(`opening tab (${k})`, main[k] && main[k].tab, ov[k] && ov[k].tab); same(`tabs (${k})`, main[k] && main[k].tabs, ov[k] && ov[k].tabs); same(`buttons (${k})`, main[k] && main[k].buttons, ov[k] && ov[k].buttons); same(`first focus (${k})`, main[k] && main[k].focus, ov[k] && ov[k].focus); }
    same('controller navigation', main.nav, ov.nav);
    same('tab order (RB)', main.tabsVisited, ov.tabsVisited);
    // (Notifications: the overlay page starts with none of its own; what's in them isn't a button set.)
    for (const k of Object.keys(main).filter(k => k.startsWith('tab-') && k !== 'tab-notif')) same(`the ${k.slice(4)} tab`, main[k], ov[k]);
    const want = ['q-shot', 'q-mouse', 'q-home', 'q-resume', 'q-pause', 'q-aside', 'q-exit'];
    const got = ((main.now && main.now.buttons) || []).map(b => b.split(':')[0]).filter(k => want.includes(k));
    if (JSON.stringify(got) !== JSON.stringify(want)) errors.push(`quick access: Now playing buttons ${got.join(' ')} (want ${want.join(' ')})`);
    if (main.rect && !/^840,0 440x800$/.test(main.rect)) errors.push(`quick access: panel at ${main.rect} (want the overlay's 440 px on the right)`);
    console.log(`quick access: panel ${main.rect}, overlay ${ov.rect}; tabs ${main.now && main.now.tabs}; now playing: ${got.join(' ')}`);
  }

  // Settings › Default apps (Hearth OS, ?preview=linux with its made-up apps): in Settings › System, a row per kind
  // with the app in use (Email only once an app can do it), picking another app with the controller (A on a row opens
  // its choices), resetting one kind from its choices, and Reset to Hearth's defaults.
  {
    const dp = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    dp.on('pageerror', e => errors.push(`default apps: ${e.message}`));
    try {
      await dp.goto('file://' + path.resolve(__dirname, '../ui/index.html') + '?preview=linux');
      await dp.waitForTimeout(1200);
      const rows = await dp.evaluate(async () => {
        const wait = ms => new Promise(r => setTimeout(r, ms));
        const has = k => document.querySelector(`#view [data-k="${k}"]`);
        closeAll(); S.layers = []; S.settingsTab = 'defaults'; navigate('settings', {}, false); onSettingsTab('defaults'); await wait(200);
        if (!STABS.some(t => t[0] === 'defaults') || stabGroup('defaults') !== 'system') throw new Error("Default apps isn't in Settings › System");
        for (const k of ['browser', 'files', 'text', 'image', 'video', 'music', 'ebooks', 'documents', 'sheets', 'slides', 'terminal', 'calendar', 'contacts', 'maps', 'archives'])
          if (!has('df-' + k)) throw new Error('no row for ' + k);
        if (has('df-email')) throw new Error('Email shows with no app that can do it');
        if (!/Hearth Browser/.test(has('df-browser').textContent)) throw new Error("the Web browser row doesn't say Hearth Browser");
        setFocus(has('df-image'), true); input('a'); await wait(150);
        if (!S.layers.includes('sheet') || !/Paint/.test(document.querySelector('#sheetBox').textContent)) throw new Error('A on Images opened no choices');
        sheetPick(sheet.items.findIndex(x => /Paint/.test(x.label))); await wait(200);
        if (!/Paint · your choice/.test(has('df-image').textContent)) throw new Error(`picking Paint didn't show: ${has('df-image').textContent}`);
        dfAct('dfPick', { kind: 'image' });
        const j = sheet.items.findIndex(x => /Reset to Hearth/.test(x.label));
        if (j < 0) throw new Error('no Reset in the Images choices');
        sheetPick(j); await wait(200);
        if (!/Photos · Hearth/.test(has('df-image').textContent)) throw new Error("resetting Images didn't bring Photos back");
        dfAct('dfPick', { kind: 'text' }); sheetPick(sheet.items.findIndex(x => /Kate/.test(x.label))); await wait(200);
        if (!DF.s.kinds.find(k => k.kind === 'text').yours) throw new Error("picking Kate wasn't kept");
        setFocus(has('df-reset'), true); input('a'); await wait(150); sheetPick(0); await wait(200);
        if (DF.s.kinds.some(k => k.yours)) throw new Error('Reset to Hearth\'s defaults left a choice');
        return document.querySelectorAll('#view [data-k^="df-"]').length;
      });
      console.log(`default apps: ${rows} rows`);
    } catch (e) { errors.push('default apps: ' + e.message.split('\n')[0]); }
    await dp.close();
  }

  // Hearth's on-screen keyboard (ui/osk/hearth-osk.js, docs/OSK.md) in Game Mode: typing with the controller (moving
  // to each key with the D-pad), word suggestions, autocapitals and double-space period, accents (A held), emoji,
  // layouts (AZERTY), shift on LT, B hiding it, the Ember Wheel, split sticks, touch, and Settings › Keyboard.
  // Pictures go to tools/osk-shots (release.yml posts them as notices).
  {
    const oskShots = path.resolve(__dirname, 'osk-shots');
    try { require('fs').mkdirSync(oskShots, { recursive: true }); } catch {}
    const kp = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    kp.on('pageerror', e => errors.push(`on-screen keyboard: ${e.message}`));
    const kerr = m => errors.push('on-screen keyboard: ' + m);
    const shot = async (name) => { try { await kp.waitForTimeout(260); await kp.screenshot({ path: path.join(oskShots, name + '.jpg'), type: 'jpeg', quality: 42, clip: { x: 0, y: 380, width: 1280, height: 420 } }); } catch (e) { kerr('picture ' + name + ': ' + e.message); } };
    try {
      await kp.goto('file://' + path.resolve(__dirname, '../ui/index.html'));
      await kp.waitForTimeout(1500);
      await kp.evaluate(() => {
        try { localStorage.removeItem('hearth.osk'); localStorage.removeItem('hearth.osk.learned'); } catch (e) {}
        HearthOSK.configure({ settings: {}, learned: { learned: {}, next: {} } });
        // Walks the selection to each character with the D-pad (as a controller does) and presses A.
        window.__padType = (word) => {
          const e = GOSK.engine;
          for (const ch of word) {
            const R = e.view().rows; let to = null;
            for (let r = 0; r < R.length && !to; r++) for (let c = 0; c < R[r].length; c++) { const k = R[r][c]; if ((k.t === 'ch' && k.v === ch.toLowerCase()) || (ch === ' ' && k.t === 'space')) { to = [r, c]; break; } }
            if (!to) throw new Error('no key for ' + ch);
            for (let i = 0; i < 40; i++) {
              const s = e.view().sel; if (s.r === to[0] && s.c === to[1]) break;
              input(s.r < to[0] ? 'down' : s.r > to[0] ? 'up' : s.c < to[1] ? 'right' : 'left');
            }
            const s = e.view().sel; if (s.r !== to[0] || s.c !== to[1]) throw new Error("the D-pad didn't reach " + ch + ' (at ' + s.r + ',' + s.c + ')');
            input('a');
          }
        };
      });
      // Y with the controller opens search with the keyboard, the keyboard has the selection.
      let st = await kp.evaluate(() => { closeAll(); S.history = []; navigate('home', {}, false); padPressed('y'); return { kb: !!document.querySelector('#search .kb .hosk'), f: focused && focused.dataset.k, active: GOSK.engine.isActive() }; });
      if (!st.kb || st.f !== 'sk-kb' || !st.active) kerr('Y did not open search with the keyboard selected: ' + JSON.stringify(st));
      await kp.evaluate(() => __padType('mo'));
      st = await kp.evaluate(() => ({ q: query, sugg: GOSK.engine.view().suggestions, res: document.querySelectorAll('#search .sres [data-f]').length }));
      if (st.q !== 'mo') kerr('typing with the D-pad and A gave ' + JSON.stringify(st.q));
      if (!st.res) kerr('search found nothing for "mo"');
      await shot('1-search');
      // A text prompt: autocapital, suggestions (up into the strip, A picks), double space period, X backspace.
      await kp.evaluate(() => { closeAll(); window.__pr = openPrompt({ title: 'Message' }); });
      await kp.waitForTimeout(500);
      st = await kp.evaluate(() => ({ f: focused && focused.dataset.k, shift: GOSK.engine.view().shift, words: HearthOSK.hasWords('en') }));
      if (st.f !== 'pk-kb') kerr('a prompt opened with the controller did not select its keyboard: ' + st.f);
      if (!st.shift) kerr('no capital at the start of a sentence');
      if (!st.words) kerr("the English word list didn't load (ui/osk/words/en.js)");
      await kp.evaluate(() => __padType('hel'));
      st = await kp.evaluate(() => ({ t: prompt.text, sugg: GOSK.engine.view().suggestions }));
      if (st.t !== 'Hel') kerr('typed ' + JSON.stringify(st.t) + ' (wanted "Hel": a capital first)');
      if (!st.sugg.includes('Hello')) kerr('no "Hello" in the suggestions: ' + JSON.stringify(st.sugg));
      await shot('2-suggestions');
      st = await kp.evaluate(() => {
        const e = GOSK.engine, i = e.view().suggestions.indexOf('Hello');
        for (let n = 0; n < 8 && e.view().sel.r !== -1; n++) input('up');
        if (e.view().sel.r !== -1) return 'the D-pad never reached the suggestions';
        while (e.view().sel.c > i) input('left'); while (e.view().sel.c < i) input('right');
        input('a'); input('y');
        return prompt.text;
      });
      if (st !== 'Hello. ') kerr('picking a suggestion then Y gave ' + JSON.stringify(st) + ' (wanted "Hello. ")');
      st = await kp.evaluate(() => { input('x'); input('x'); return prompt.text; });
      if (st !== 'Hello') kerr('X twice gave ' + JSON.stringify(st));
      // Accents: A held on "e" opens them, A picks one (it replaces the e).
      st = await kp.evaluate(() => {
        const e = GOSK.engine; input('y');
        const R = e.view().rows; let to; R.forEach((row, r) => row.forEach((k, c) => { if (k.t === 'ch' && k.v === 'e') to = { r, c }; }));
        e.state.sel = to; const t = Date.now();
        e.handle('a', true, t); e.tick(t + 520);
        const pop = e.view().popup; if (!pop) return 'no accents after holding A';
        e.handle('a', false, t + 560);
        const want = pop.alts.indexOf('é'); for (let i = 0; i < want; i++) input('right');
        input('a');
        return prompt.text;
      });
      if (st !== 'Hello é') kerr('accents gave ' + JSON.stringify(st));
      // LT is shift (one letter), L3 the emoji, B hides the keyboard and A on the field brings it back.
      st = await kp.evaluate(() => {
        const out = {};
        input('lt'); __padType('a'); out.shift = prompt.text.endsWith('A');
        input('l3'); out.panel = GOSK.engine.view().panel; GOSK.engine.state.sel = { r: 1, c: 0 }; input('a'); out.emoji = /\p{Extended_Pictographic}/u.test(prompt.text.slice(-2));
        input('l3'); out.back = GOSK.engine.view().panel;
        input('b'); out.hidden = !document.querySelector('#prompt .kb') && focused && focused.dataset.k;
        input('a'); out.again = !!document.querySelector('#prompt .kb') && focused && focused.dataset.k;
        return out;
      });
      if (!st.shift) kerr('LT did not make the next letter a capital');
      if (st.panel !== 'emoji' || !st.emoji || st.back !== 'letters') kerr('emoji (L3): ' + JSON.stringify(st));
      if (st.hidden !== 'pk-field' || st.again !== 'pk-kb') kerr('B / A on the field: ' + JSON.stringify(st));
      await kp.evaluate(() => { GOSK.engine.handle('l3'); });
      await shot('3-emoji');
      await kp.evaluate(() => { GOSK.engine.handle('l3'); });
      // Layouts: with English and French, R3 switches to AZERTY.
      st = await kp.evaluate(() => {
        HearthOSK.setSetting('layouts', ['en-us', 'fr']); input('r3');
        const v = GOSK.engine.view(), row = v.rows[1].map(k => k.v).join('');
        input('r3'); HearthOSK.setSetting('layouts', ['auto']);
        return { lay: v.layout, row };
      });
      if (st.lay !== 'fr' || !st.row.startsWith('azerty')) kerr('R3 did not switch to AZERTY: ' + JSON.stringify(st));
      // The Ember Wheel: the left stick tilted up is "abcd", Y types its top letter.
      st = await kp.evaluate(() => {
        const e = GOSK.engine, before = prompt.text;
        e.axes(0, -1, 0, 0); const w = e.view().wheel; e.handle('y'); const typed = prompt.text.slice(before.length);
        return { petal: w && w.petal, active: w && w.active, typed };
      });
      if (st.petal !== 0 || !st.active || st.typed.toLowerCase() !== 'a') kerr('Ember Wheel: ' + JSON.stringify(st));
      await kp.evaluate(() => { HearthOSK.setSetting('theme', 'ember'); GOSK.engine.axes(0.7, -0.7, 0, 0); });
      await shot('4-ember-wheel');
      await kp.evaluate(() => { GOSK.engine.axes(0, 0, 0, 0); HearthOSK.setSetting('theme', 'hearth'); });
      // Split sticks: each stick points into its half, LT / RT type.
      st = await kp.evaluate(() => {
        const e = GOSK.engine, before = prompt.text; HearthOSK.setSetting('fast', 'split');
        e.axes(-1, -0.2, 1, -0.2); const v = e.view(); e.handle('lt', true); e.handle('lt', false); e.handle('rt', true); e.handle('rt', false);
        e.axes(0, 0, 0, 0); HearthOSK.setSetting('fast', 'wheel');
        return { l: v.split && v.split.l, r: v.split && v.split.r, typed: prompt.text.slice(before.length) };
      });
      if (!st.l || !st.r || st.typed.length !== 2) kerr('split sticks: ' + JSON.stringify(st));
      // Touch: a tap on a key types it.
      const key = await kp.evaluate(() => { const b = [...document.querySelectorAll('#prompt .hosk-key.k-ch')].find(x => x.textContent.trim().toLowerCase() === 'k'); const r = b.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; });
      await kp.mouse.click(key.x, key.y); await kp.waitForTimeout(100);
      st = await kp.evaluate(() => prompt.text.slice(-1).toLowerCase());
      if (st !== 'k') kerr('a click on k typed ' + JSON.stringify(st));
      // ☰ finishes the prompt.
      st = await kp.evaluate(async () => { input('start'); return await Promise.race([window.__pr, new Promise(r => setTimeout(() => r('still open'), 400))]); });
      if (typeof st !== 'string' || !st.startsWith('Hello é')) kerr('☰ did not finish the prompt: ' + JSON.stringify(st));
      // Split shape and Daylight, for the picture; then a real keyboard sends it away again.
      await kp.evaluate(() => { HearthOSK.setSetting('shape', 'split'); HearthOSK.setSetting('theme', 'daylight'); closeAll(); window.__pr = openPrompt({ title: 'Name', osk: true }); });
      await shot('5-split-daylight');
      await kp.evaluate(() => { HearthOSK.setSetting('shape', 'full'); HearthOSK.setSetting('theme', 'hearth'); });
      await kp.keyboard.type('Zo'); await kp.waitForTimeout(150);
      st = await kp.evaluate(() => ({ kb: !!document.querySelector('#prompt .kb'), t: prompt && prompt.text }));
      if (st.kb) kerr('typing on a real keyboard left the on-screen keyboard up');
      if (st.t !== 'Zo') kerr('a real keyboard typed ' + JSON.stringify(st.t));
      // Learned words: "hel" typed often enough comes back; Clear forgets.
      st = await kp.evaluate(() => { const n = HearthOSK.learnedCount(); HearthOSK.clearLearned(); return { n, after: HearthOSK.learnedCount() }; });
      if (!st.n || st.after) kerr('learned words: ' + JSON.stringify(st));
      // Settings › Keyboard: every choice is there and saved.
      st = await kp.evaluate(() => {
        closeAll(); S.settingsTab = 'keyboard'; navigate('settings', {}, false);
        const need = ['kb-lay-auto', 'kb-lay-fr', 'kb-suggestions', 'kb-learn', 'kb-autocap', 'kb-doubleSpace', 'kb-fast-wheel', 'kb-fast-split', 'kb-fast-off', 'kb-theme-ember', 'kb-size-l', 'kb-shape-split', 'kb-position-top', 'kb-haptics-strong', 'kb-trail', 'kb-clear', 'kb-try'];
        const missing = need.filter(k => !document.querySelector(`#view [data-k="${k}"]`));
        act(document.querySelector('#view [data-k="kb-theme-ember"]'));
        act(document.querySelector('#view [data-k="kb-lay-de"]'));
        let saved = {}; try { saved = JSON.parse(localStorage.getItem('hearth.osk') || '{}'); } catch (e) {}
        act(document.querySelector('#view [data-k="kb-theme-hearth"]')); act(document.querySelector('#view [data-k="kb-lay-de"]'));
        return { missing, theme: saved.theme, layouts: saved.layouts };
      });
      if (st.missing.length) kerr('Settings › Keyboard is missing ' + st.missing.join(', '));
      if (st.theme !== 'ember' || !(st.layouts || []).includes('de')) kerr("Settings › Keyboard didn't save: " + JSON.stringify(st));
      console.log('on-screen keyboard: controller, suggestions, accents, emoji, layouts, wheel, split sticks, touch and settings ok');
    } catch (e) { kerr(e.message.split('\n')[0]); }
    await kp.close();
  }

  await browser.close();
  if (errors.length) {
    for (const e of errors) console.log(`::error title=Hearth page::${e}`);
    process.exit(1);
  }
  console.log(`ok: Hearth's page loads and draws ${9 + tabs.length + layouts.length + libLayouts.length} screens (and Beacon's tab, game page, Game setup, Mods, log, jobs, mod sheets and Stores card, and an expired store sign-in) (and ${hhdSecs.length * 2} Handheld Daemon sections, and Quick Access › Performance ${perfModes.length} ways) (and Hearth for Android: ${androidSteps.length} steps) without errors`);
})();
