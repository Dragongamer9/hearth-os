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
  // Game Mode › Store (hearth-core storefront): each store's page from the preview's data, LB/RB between stores, a
  // game's page with the right button (Play/Install when it's in your library, Sign in when the store isn't, Buy on
  // the store's own page otherwise), searching, and B back to the store. Pictures for release.yml.
  const shopCard = t => page.evaluate(t => { const c = [...document.querySelectorAll('#view .sft')].find(e => e.querySelector('.sft-t').textContent === t); if (!c) throw new Error('no card for ' + t); return c.dataset.k; }, t);
  const shopOpen = async (store, title) => {
    await page.evaluate(s => { S.layers = []; if (SHOP.page) shopBack(); shopTab(s); }, store);
    await page.waitForTimeout(400);
    const k = await shopCard(title);
    await page.evaluate(k => act(document.querySelector(`#view [data-k="${k}"]`)), k);
    await page.waitForTimeout(400);
    return page.evaluate(() => { const b = document.querySelector('#view [data-k="sf-buy"]'); return b ? { act: b.dataset.act, text: b.textContent.trim(), v: b.dataset.v || '' } : null; });
  };
  await step('store page', async () => {
    S.layers = []; S.history = []; SHOP.home = {}; SHOP.page = null; SHOP.store = 'steam'; navigate('shop', {}, false);
    await new Promise(r => setTimeout(r, 500));
    for (const k of ['sf-t-steam', 'sf-t-gog', 'sf-t-epic', 'sf-t-xbox', 'sf-search', 'sf-hero', 'sf-featured-0', 'sf-deals-0', 'sf-top-0'])
      if (!document.querySelector(`#view [data-k="${k}"]`)) throw new Error('no ' + k);
    if (!/-50%/.test(document.querySelector('#view [data-k="sf-deals-0"]').textContent)) throw new Error("a deal doesn't show its discount");
    if (!/Buying opens Steam's own store/.test(document.querySelector('#view').textContent)) throw new Error("Steam's account line is missing");
    renderMenu(); if (!document.querySelector('#menu [data-k="m-shop"]')) throw new Error('Store is not in the menu');
    input('rb'); await new Promise(r => setTimeout(r, 400));
    if (SHOP.store !== 'gog' || !document.querySelector('#view [data-k="sf-t-gog"].on')) throw new Error('RB did not go to GOG: ' + SHOP.store);
    input('lb'); if (SHOP.store !== 'steam') throw new Error('LB did not go back to Steam');
  });
  try {
    const owned = await shopOpen('gog', 'Hollow Pines');   // in the preview's library, installed
    if (!owned || owned.act !== 'shopLib' || owned.v !== 'play') errors.push('store: a game in your library offers ' + JSON.stringify(owned));
    const back = await page.evaluate(() => { back(); return { page: !!SHOP.page, focus: focused && focused.dataset.k }; });
    if (back.page || !/^sf-/.test(back.focus || '')) errors.push("store: B on a game's page didn't go back to the store: " + JSON.stringify(back));
    const signIn = await shopOpen('epic', 'Arc Runner');   // Epic isn't signed in in the preview
    if (!signIn || signIn.act !== 'shopSignIn' || !/Sign in to Epic Games/.test(signIn.text)) errors.push('store: an Epic game while signed out offers ' + JSON.stringify(signIn));
    const buy = await shopOpen('xbox', 'Halcyon Drive');
    if (!buy || buy.act !== 'shopWeb' || buy.v !== 'buy' || !/Buy on Xbox/.test(buy.text)) errors.push('store: an Xbox game offers ' + JSON.stringify(buy));
    const steam = await shopOpen('steam', 'Copperline');
    if (!steam || steam.act !== 'shopSteam' || !/Buy in Steam/.test(steam.text)) errors.push('store: a Steam game offers ' + JSON.stringify(steam));
    const page2 = await page.evaluate(() => ({ stats: [...document.querySelectorAll('#view .gstat .k')].map(e => e.textContent).join(' '), linux: (document.querySelector('#view .pdb') || {}).textContent, about: /placeholder description/.test(document.querySelector('#view .shop-desc') ? document.querySelector('#view .shop-desc').textContent : '') }));
    if (page2.stats !== 'Price Rating On Linux Released Developer' || page2.linux !== 'Gold' || !page2.about) errors.push("store: the game's page is missing something: " + JSON.stringify(page2));
    await step('store search', async () => {
      if (SHOP.page) shopBack();
      shopDoSearch('steam', 'lantern'); await new Promise(r => setTimeout(r, 300));
      const r = document.querySelector('#view [data-k="sf-r-0"]');
      if (!r || !/Lantern Keep/.test(r.textContent)) throw new Error('no search result');
      if (!document.querySelector('#view [data-k="sf-clear"]')) throw new Error('no Clear');
      shopDoSearch('steam', '');
      if (document.querySelector('#view [data-k="sf-r-0"]')) throw new Error("Clear left the results");
    });
  } catch (e) { errors.push('store: ' + e.message.split('\n')[0]); }
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

  // The on-screen keyboard driven by a real controller (the Gamepad API, as Game Mode on Linux reads it), not input():
  // in 0.6.5 pollPadsNow threw "Cannot access 'ax' before initialization" every frame while the keyboard had the
  // selection, so it came up but the controller did nothing on it. Each place Game Mode types: search (Y), the
  // Library's "Search your games" prompt, Send logs' "What happened?", a password prompt, and Hearth's own presses
  // (Quick Access / the overlay / the Windows host: onSysEvent 'pad'). Send logs must say which way presses came.
  {
    const op = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    op.on('pageerror', e => errors.push('keyboard with a controller: ' + e.message));
    const oerr = m => errors.push('keyboard with a controller: ' + m);
    try {
      await op.goto('file://' + path.resolve(__dirname, '../ui/index.html'));
      await op.waitForTimeout(1200);
      await op.evaluate(() => {
        const pad = { id: 'Xbox Wireless Controller (STANDARD GAMEPAD Vendor: 045e Product: 0b13)', index: 0, connected: true, mapping: 'standard', axes: [0, 0, 0, 0],
                      buttons: Array.from({ length: 17 }, () => ({ pressed: false, value: 0 })) };
        window.__pad = pad; navigator.getGamepads = () => [pad, null, null, null];
        window.__sent = []; const realSend = window.send;
        window.send = m => { window.__sent.push(m); try { return realSend(m); } catch (e) { return null; } };
        closeAll(); S.history = []; navigate('home', {}, false);
      });
      const B = { a: 0, b: 1, x: 2, y: 3, up: 12, down: 13, left: 14, right: 15 };
      const press = async (k) => {
        await op.evaluate(i => { window.__pad.buttons[i] = { pressed: true, value: 1 }; }, B[k]); await op.waitForTimeout(90);
        await op.evaluate(i => { window.__pad.buttons[i] = { pressed: false, value: 0 }; }, B[k]); await op.waitForTimeout(90);
      };
      const logs = () => op.evaluate(() => window.__sent.filter(m => m && m.type === 'log').map(m => m.text));
      const errs = async () => (await logs()).filter(t => /^controller: /.test(t));
      // Moves right once and presses A, all on the controller: one more character, the one that was selected.
      const typeOne = async (where, textOf) => {
        const before = await op.evaluate(textOf);
        const st0 = await op.evaluate(() => ({ owns: oskOwns(), sel: GOSK.engine && GOSK.engine.view().sel }));
        if (!st0.owns) { oerr(where + ": the keyboard didn't have the selection"); return; }
        await press('right');
        const st1 = await op.evaluate(() => { const v = GOSK.engine.view(), k = v.rows[v.sel.r][v.sel.c]; return { sel: v.sel, k: k && k.t === 'ch' ? k.v : null }; });
        if (st1.sel.r === st0.sel.r && st1.sel.c === st0.sel.c) oerr(where + ': D-pad right on the controller did not move on the keyboard');
        await press('a');
        const after = await op.evaluate(textOf);
        if (after.length !== before.length + 1 || (st1.k && after.slice(-1).toLowerCase() !== st1.k.toLowerCase())) oerr(`${where}: A on the controller typed ${JSON.stringify(before)} → ${JSON.stringify(after)} (key ${st1.k})`);
        // The left stick reaches the keyboard: the Ember Wheel (the default) picks a petal; without it the selection moves.
        const s0 = await op.evaluate(() => GOSK.engine.view().sel);
        await op.evaluate(() => { window.__pad.axes[0] = 0.95; }); await op.waitForTimeout(450);
        const sw = await op.evaluate(() => { const v = GOSK.engine.view(); return { wheel: !!(v.wheel && v.wheel.active), sel: v.sel }; });
        await op.evaluate(() => { window.__pad.axes[0] = 0; }); await op.waitForTimeout(150);
        if (!sw.wheel && s0.r === sw.sel.r && s0.c === sw.sel.c) oerr(where + ': the left stick did nothing on the keyboard (no Ember Wheel petal, no move)');
        if (await op.evaluate(() => { const v = GOSK.engine.view(); return !!(v.wheel && v.wheel.active); })) oerr(where + ': the Ember Wheel stayed open after the stick was let go');
      };
      // 1) Search: Y on the controller.
      await press('y');
      if (!(await op.evaluate(() => S.layers.at(-1) === 'search' && !!document.querySelector('#search .kb .hosk')))) oerr('Y on the controller did not open search with the keyboard');
      await typeOne('search', () => query);
      await press('b');
      if (await op.evaluate(() => oskOwns())) oerr('search: B on the controller did not put the keyboard away');
      // 2) The Library's "Search your games", 3) Send logs' note, 4) a password: openPrompt, as they call it.
      for (const [where, o] of [['Library search prompt', { title: 'Search your games', value: '' }], ['Send logs note', { title: 'What happened?', value: '' }], ['password prompt', { title: 'Wi-Fi password', secret: true }]]) {
        await op.evaluate(o => { closeAll(); window.__pr = openPrompt(o); }, o); await op.waitForTimeout(250);
        await typeOne(where, () => prompt ? prompt.text : '');
        await op.evaluate(() => closeAll()); await op.waitForTimeout(120);
      }
      // Send logs' button itself, when the preview draws it.
      const dev = await op.evaluate(() => { S.settingsTab = 'help'; navigate('settings', {}, false); const b = document.querySelector('[data-act="sendLogsDev"]'); if (!b) return null; act(b); return S.layers.at(-1); });
      if (dev !== null) { await op.waitForTimeout(250); if (dev !== 'prompt') oerr('Send logs: no "What happened?" prompt'); else await typeOne('Send logs (from Settings)', () => prompt ? prompt.text : ''); await op.evaluate(() => closeAll()); }
      const L = await logs();
      if (!L.some(t => /^osk (search|prompt): keyboard has the selection · controller read here \(Gamepad API\) · pads seen 1/.test(t))) oerr("Send logs doesn't say the keyboard got the controller from the Gamepad API: " + JSON.stringify(L.filter(t => /^osk/.test(t))));
      if (!L.some(t => /^osk (search|prompt): first press \w+ via gamepad$/.test(t))) oerr("Send logs doesn't say which way the first press reached the keyboard");
      // 5) Hearth's own presses (the overlay, Quick Access, the Windows host, Multi-seat): no Gamepad API here.
      await op.evaluate(() => { PAD.host = true; closeAll(); window.__pr = openPrompt({ title: 'Message' }); });
      await op.waitForTimeout(250);
      const h = await op.evaluate(() => { const owns = oskOwns(), before = prompt.text, s0 = GOSK.engine.view().sel; onSysEvent('pad', { b: 'right' }); const s1 = GOSK.engine.view().sel; onSysEvent('pad', { b: 'a' }); return { owns, moved: s0.c !== s1.c || s0.r !== s1.r, typed: prompt.text.length - before.length }; });
      if (!h.owns || !h.moved || h.typed !== 1) oerr("Hearth's own controller presses don't drive the keyboard: " + JSON.stringify(h));
      if (!(await logs()).some(t => /^osk prompt: first press \w+ via host$/.test(t))) oerr("Send logs doesn't say the keyboard's presses came from Hearth (host)");
      await op.evaluate(() => { PAD.host = false; closeAll(); });
      const e = await errs();
      if (e.length) oerr('the controller loop threw: ' + e[0].split('\n')[0]);
      await op.evaluate(() => { delete navigator.getGamepads; });
      console.log('on-screen keyboard with a controller (Gamepad API and Hearth host): search, Library search, Send logs, password ok');
    } catch (e) { oerr(e.message.split('\n')[0]); }
    await op.close();
  }

  // The same with the docked keyboard Hearth's apps and Hearth Browser open (HearthOSK.open, which reads the controller
  // itself): one app page per kind (hearth-time's notes, hearth-maps' search, hearth-contacts).
  for (const app of ['hearth-time/ui/notes.html', 'hearth-maps/ui/index.html', 'hearth-contacts/ui/index.html']) {
    const ap = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    const aerr = m => errors.push(`keyboard with a controller (${app}): ` + m);
    ap.on('pageerror', e => aerr(e.message));
    try {
      await ap.addInitScript(() => {
        const pad = { id: 'Xbox Wireless Controller (test)', index: 0, connected: true, mapping: 'standard', axes: [0, 0, 0, 0],
                      buttons: Array.from({ length: 17 }, () => ({ pressed: false, value: 0 })) };
        window.__pad = pad; Object.defineProperty(navigator, 'getGamepads', { value: () => [pad, null, null, null], configurable: true });
      });
      await ap.goto('file://' + path.resolve(__dirname, '../hosts/' + app));
      await ap.waitForTimeout(900);
      const ok = await ap.evaluate(() => { if (!window.HearthOSK) return false; const i = document.createElement('input'); i.id = '__oskt'; document.body.appendChild(i); i.focus(); HearthOSK.open({ target: i }); return HearthOSK.isOpen(); });
      if (!ok) { aerr('the keyboard did not open'); await ap.close(); continue; }
      const press = async i => { await ap.evaluate(i => { window.__pad.buttons[i] = { pressed: true, value: 1 }; }, i); await ap.waitForTimeout(90); await ap.evaluate(i => { window.__pad.buttons[i] = { pressed: false, value: 0 }; }, i); await ap.waitForTimeout(90); };
      await press(15); await press(0); await press(15); await press(0);
      const v = await ap.evaluate(() => document.getElementById('__oskt').value);
      if (v.length !== 2) aerr('D-pad and A on the controller typed ' + JSON.stringify(v));
      await press(1);
      if (await ap.evaluate(() => HearthOSK.isOpen())) aerr('B on the controller did not close the keyboard');
    } catch (e) { aerr(e.message.split('\n')[0]); }
    await ap.close();
  }

  // Hearth Friends on Matrix (the preview's made-up account): the Friends screen's two halves (Hearth Friends and the
  // stores' friends), friends with "Playing" and Joinable, a chat with a game invite card (Play, Join server, Join via
  // Steam), a Play together request (let them join / not now), a picture, the emoji grid, Requests, Settings (privacy,
  // Steam friends, Discord), signed out (with a phone: QR code; in the browser; a password; Create an account), the
  // recovery key question, the friend's sheet with the controller, and
  // Quick Access › Friends with a chat in it. Pictures of the screens go to tools/qa-shots (friends.yml posts them).
  {
    const fp = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    fp.on('pageerror', e => errors.push(`friends: ${e.message}`));
    const shotsDir = path.resolve(__dirname, 'qa-shots');
    try { require('fs').mkdirSync(shotsDir, { recursive: true }); } catch {}
    const shoot = async name => { for (const q of [60, 45, 30, 20]) { const buf = await fp.screenshot({ type: 'jpeg', quality: q }); if (buf.toString('base64').length <= 9 * 3800 || q === 20) { require('fs').writeFileSync(path.join(shotsDir, `friends-${name}.jpg`), buf); break; } } };
    try {
      await fp.goto('file://' + path.resolve(__dirname, '../ui/index.html') + '?preview=linux');
      await fp.waitForTimeout(1200);
      const run = fn => fp.evaluate(fn);
      await run(async () => {
        const wait = ms => new Promise(r => setTimeout(r, ms));
        const has = k => document.querySelector(`#view [data-k="${k}"]`);
        closeAll(); S.layers = []; S.history = []; MF.top = 'mx'; MF.tab = 'friends'; MF.room = null; navigate('friends', {}, false); await wait(300);
        if (!has('mft-mx') || !has('mft-stores')) throw new Error('no Hearth Friends / Steam & other stores switch');
        const sarah = has('mff-@sarah:matrix.org');
        if (!sarah || !/Playing Starfall Drift/.test(sarah.textContent) || !/Joinable/.test(sarah.textContent)) throw new Error('Sarah isn\'t shown playing and joinable');
        if (!/Request sent/.test((has('mff-@jo:matrix.org') || {}).textContent || '')) throw new Error('a sent request isn\'t shown');
        if (!has('mf-add') || !has('mf-code')) throw new Error('no Add friend / My code');
        setFocus(sarah, true); input('a'); await wait(150);
        const sh = document.querySelector('#sheetBox').textContent;
        if (!S.layers.includes('sheet') || !/Message Sarah/.test(sh) || !/Ask to join/.test(sh) || !/Block/.test(sh) || !/Report/.test(sh)) throw new Error('the friend\'s sheet is missing things: ' + sh.slice(0, 200));
        closeAll();
      });
      await shoot('list');
      await run(async () => {
        const wait = ms => new Promise(r => setTimeout(r, ms));
        const has = k => document.querySelector(`#view [data-k="${k}"]`);
        mfOpenChat('!s:matrix.org'); await wait(400);
        for (const k of ['mfx-type', 'mfx-emoji', 'mfx-shot', 'mfx-game', 'mfg-p-$3', 'mfg-j-$3', 'mfg-v-$3', 'mfp-a-$4', 'mfp-k-$4', 'mfp-d-$4', 'mfi-$5'])
          if (!has(k)) throw new Error('the chat has no ' + k);
        if (!/End-to-end encrypted/.test(document.querySelector('#view .mfchathead').textContent)) throw new Error('the chat doesn\'t say it\'s encrypted');
        await mfAct('mfEmoji', {}); await wait(150);
        if (!has('mfe-0')) throw new Error('no emoji grid');
        await mfAct('mfImg', { id: '$5' }); await wait(200);
        if (!document.querySelector('#view .mfimg')) throw new Error('the picture didn\'t show');
      });
      await shoot('chat');
      await run(async () => {
        const wait = ms => new Promise(r => setTimeout(r, ms));
        const has = k => document.querySelector(`#view [data-k="${k}"]`);
        await mfAct('mfClose', {}); MF.emojiOpen = false;
        await mfAct('mfTab', { v: 'requests' }); await wait(150);
        if (!has('mfr-a-req0') || !has('mfr-b-req0')) throw new Error('Requests has no Accept / Block');
        await mfAct('mfTab', { v: 'settings' }); await wait(250);
        for (const k of ['mfs-friendsOnly', 'mfs-sharePlaying', 'mfs-presence', 'mfs-blocked', 'mfs-askToJoin', 'mfs-steam', 'mfs-discord', 'mfs-out'])
          if (!has(k)) throw new Error('Friends › Settings has no ' + k);
        if (!/end-to-end encrypted/.test(document.querySelector('#view').textContent)) throw new Error('no end-to-end note');
      });
      await shoot('settings');
      await run(async () => {
        const wait = ms => new Promise(r => setTimeout(r, ms));
        const has = k => document.querySelector(`#view [data-k="${k}"]`);
        await mfAct('mfTop', { v: 'stores' }); await wait(200);
        if (!document.querySelector('#view .frow') && !/Loading/.test(document.querySelector('#view').textContent)) throw new Error('Steam & other stores shows nothing');
        await mfAct('mfTop', { v: 'mx' });
        window.__mfWas = MF.st; MF.st = { signedIn: false, settings: { homeserver: 'matrix.org' } }; MF.ways = {}; MF.auth = null; render(null, true); await wait(100);
        if (!has('mf-signin') || !has('mf-create') || !has('mf-hs')) throw new Error('signed out: no Sign in / Create an account / server');
        if (!/matrix\.org/.test(document.querySelector('#view').textContent)) throw new Error('signed out: doesn\'t say where the account lives');
        // matrix.org signs people in and up on its own page (OAuth 2.0): with a phone (QR code), or in the browser here.
        await wait(300); render(null, true); await wait(100);
        for (const k of ['mf-qr', 'mf-create', 'mf-web', 'mf-web-new', 'mf-signin', 'mf-hs']) if (!has(k)) throw new Error('signed out on matrix.org: no ' + k);
        await mfAct('mfQrNew', {}); await wait(300);
        const v = document.querySelector('#view').textContent;
        if (!document.querySelector('#view .mfqrcode svg') || !/WDJB-MJHT/.test(v) || !/account\.matrix\.org\/link/.test(v) || !/Create account/.test(v)) throw new Error('the phone sign-up has no QR code / code / short address');
        if (!has('mf-auth-cancel') || !has('mf-auth-web')) throw new Error('the phone sign-up has no Cancel / Use this device instead');
      });
      await shoot('signin');
      await run(async () => {
        const wait = ms => new Promise(r => setTimeout(r, ms));
        const has = k => document.querySelector(`#view [data-k="${k}"]`);
        // The controller reaches Cancel; then the browser on this device.
        await mfAct('mfAuthCancel', {}); await wait(150);
        if (MF.auth || !has('mf-qr')) throw new Error('Cancel didn\'t go back to the sign-in choices');
        await mfAct('mfWeb', {}); await wait(300);
        if (!has('mf-auth-open') || !has('mf-auth-cancel') || !has('mf-auth-qr')) throw new Error('signing in in the browser has no Open again / Cancel / Use my phone');
        await mfAct('mfAuthCancel', {}); await wait(150);
        // A server without its own sign-in page: a password, and sign-up from Hearth.
        MF.hs = 'example.org'; render(null, true); await wait(300); render(null, true);
        if (has('mf-qr') || has('mf-web') || !has('mf-signin') || !has('mf-create')) throw new Error('a password-only server shows the wrong ways to sign in');
        MF.hs = null;
        // Signed in: the recovery key question, once.
        MF.st = Object.assign({}, window.__mfWas, { askRecovery: true, recovery: 'disabled', accountUrl: 'https://account.matrix.org/account/' }); MF.tab = 'friends'; render(null, true); await wait(100);
        if (!has('mf-rec-new') || !has('mf-rec-skip')) throw new Error('no recovery key question after signing in');
        await mfAct('mfTab', { v: 'settings' }); await wait(200);
        if (!has('mfs-recovery') || !has('mfs-account')) throw new Error('Friends › Settings has no Recovery key / account page');
        await mfAct('mfTab', { v: 'friends' });
        MF.st = window.__mfWas; render(null, true);
        // Quick Access › Friends, then a chat in it.
        closeAll(); openLayer('qam'); S.qtab = 'friends'; MF.room = null; renderQam(); await wait(300); renderQam();
        if (!document.querySelector('#qam [data-k="mfq-f-@sarah:matrix.org"]') || !document.querySelector('#qam [data-k="mfq-all"]')) throw new Error('Quick Access › Friends has no friends');
        await mfAct('mfQOpen', { room: '!s:matrix.org' }); await wait(300); renderQam();
        if (!document.querySelector('#qam [data-k="mfx-type"]') || !document.querySelector('#qam [data-k="mfx-game"]')) throw new Error('Quick Access › Friends chat has no Reply / Invite');
      });
      await shoot('qam');
      await run(() => { closeAll(); MF.room = null; });
    } catch (e) { errors.push('friends: ' + e.message.split('\n')[0]); }
    await fp.close();
  }

  // Quick Access is one implementation: Hearth's own (Guide with Hearth on screen, the starting screen, LAST RESORT)
  // and the overlay over a game (?overlay=full, HHD's overlay app or Hearth's overlay window) draw the same tabs,
  // sections and buttons in the same order, with the same actions and the same controller navigation: the top dock
  // with a game running, the side panel (440 px on the right) without one. Screenshots of both go to tools/qa-shots
  // (release.yml posts them as notices).
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
        top: document.querySelector('#qam').classList.contains('top'),
        buttons: [...document.querySelectorAll('#qam .qbody [data-f]')].map(e => `${e.dataset.k}:${e.dataset.act || e.dataset.slider || ''}:${e.textContent.replace(/\s+/g, ' ').trim()}`),
        focus: focused && focused.dataset.k,
      }));
      const rect = () => qp.evaluate(() => { const r = document.querySelector('#qam').getBoundingClientRect(); return `${Math.round(r.left)},${Math.round(r.top)} ${Math.round(r.width)}x${Math.round(r.height)}`; });
      await open();
      out.rectTop = await rect();
      out.now = await read();
      // Controller: down through the panel, then RB through every tab.
      out.nav = await qp.evaluate(() => { const ks = []; for (let i = 0; i < 6; i++) { input('down'); ks.push(focused && focused.dataset.k); } return ks.join(' '); });
      out.tabsVisited = [];
      for (let i = 0; i < 6; i++) { await qp.evaluate(() => input('rb')); await qp.waitForTimeout(await qp.evaluate(() => ['perf', 'apps', 'friends', 'capture', 'controls'].includes(S.qtab)) ? 1500 : 200); const r = await read(); out.tabsVisited.push(r.tab); out['tab-' + r.tab] = out['tab-' + r.tab] || r.buttons.filter(b => !/^n-/.test(b)).join(' | ').replace(/\d+ ?(min|%|°C|W|MHz|MB|GB|fps)\b/g, '#'); }
      out.tabsVisited = out.tabsVisited.join(' ');
      // The screenshot: Now playing with the game running.
      await qp.evaluate(() => { S.qtab = 'now'; renderQam(); setFocus(document.querySelector('#qam .qbody [data-f]'), true); });
      await qp.waitForTimeout(300);
      for (const q of [55, 40, 30, 22, 15]) {
        const buf = await qp.screenshot({ type: 'jpeg', quality: q, clip: { x: 240, y: 0, width: 800, height: 520 } });
        if (buf.toString('base64').length <= 9 * 3800 || q === 15) { require('fs').writeFileSync(path.join(qaShots, `${which}.jpg`), buf); break; }
      }
      // Nothing running: the same "no game" card, buttons and Quick Resume list.
      await qp.evaluate(() => { window.__game = null; S.running = null; closeAll(); });
      await open();
      out.rect = await rect();
      out.idle = await read();
    } catch (e) { errors.push(`quick access (${which}): ${e.message.split('\n')[0]}`); }
    await qp.close();
    return out;
  };
  {
    const main = await qaPanel('main'), ov = await qaPanel('overlay');
    const same = (what, a, b) => { if (JSON.stringify(a) !== JSON.stringify(b)) errors.push(`quick access: ${what} differ between Hearth's panel and the overlay:\n  panel:   ${JSON.stringify(a)}\n  overlay: ${JSON.stringify(b)}`); };
    same('size and position', main.rect, ov.rect);
    same('size and position (top dock)', main.rectTop, ov.rectTop);
    if (!(main.now && main.now.top) || !(ov.now && ov.now.top)) errors.push(`quick access: with a game running it isn't the top dock (panel ${main.now && main.now.top}, overlay ${ov.now && ov.now.top})`);
    if ((main.idle && main.idle.top) || (ov.idle && ov.idle.top)) errors.push('quick access: with no game it should be the side panel');
    for (const k of ['now', 'idle']) { same(`opening tab (${k})`, main[k] && main[k].tab, ov[k] && ov[k].tab); same(`tabs (${k})`, main[k] && main[k].tabs, ov[k] && ov[k].tabs); same(`buttons (${k})`, main[k] && main[k].buttons, ov[k] && ov[k].buttons); same(`first focus (${k})`, main[k] && main[k].focus, ov[k] && ov[k].focus); }
    same('controller navigation', main.nav, ov.nav);
    same('tab order (RB)', main.tabsVisited, ov.tabsVisited);
    // (Notifications: the overlay page starts with none of its own; what's in them isn't a button set.)
    for (const k of Object.keys(main).filter(k => k.startsWith('tab-') && k !== 'tab-notif')) same(`the ${k.slice(4)} tab`, main[k], ov[k]);
    const want = ['q-resume', 'q-invite', 'q-gset', 'q-exit', 'q-pause', 'q-aside', 'q-home'];
    const got = ((main.now && main.now.buttons) || []).map(b => b.split(':')[0]).filter(k => want.includes(k));
    if (JSON.stringify(got) !== JSON.stringify(want)) errors.push(`quick access: Now playing buttons ${got.join(' ')} (want ${want.join(' ')})`);
    if (main.rect && !/^840,0 440x800$/.test(main.rect)) errors.push(`quick access: panel at ${main.rect} (want the overlay's 440 px on the right)`);
    if (main.now && main.now.tabs !== 'now perf controls friends capture quick') errors.push(`quick access: top dock tabs ${main.now.tabs} (want now perf controls friends capture quick)`);
    console.log(`quick access: panel ${main.rect}, overlay ${ov.rect}, top dock ${main.rectTop}; tabs ${main.now && main.now.tabs}; now playing: ${got.join(' ')}`);
  }
  // Put aside only on an explicit choice (0.6.54: over a Steam game Quick Access put the game aside by itself a moment
  // after it opened). An A just after it opens is dropped, Put aside asks first with Cancel selected, the question drops
  // an A in its first moment too, and every step is logged with what chose it. Steam Input's desktop configuration turns
  // the controller into keys for the overlay window as well, so those keys are dropped over a game: that double press is
  // what walked the selection from Back to game onto Put aside in the first place.
  {
    const pp = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    pp.on('pageerror', e => errors.push(`put aside: ${e.message}`));
    try {
      await pp.goto('file://' + path.resolve(__dirname, '../ui/index.html') + '?preview=linux&overlay=full');
      await pp.waitForTimeout(1200);
      const r = await pp.evaluate(async () => {
        const wait = ms => new Promise(res => setTimeout(res, ms)), o = {};
        window.__sent = []; window.send = m => { window.__sent.push(m); };
        HOSTINFO.features = ['quickResume', 'pause']; QR.on = true;
        const id = GAMES.filter(g => !g.cloud)[0].id;
        const real = MOCK.call; window.__calls = [];
        MOCK.call = (m, a) => { window.__calls.push(m); return m === 'perf.stats' ? real(m, a).then(x => Object.assign({}, x, { runningId: id, runningSince: Date.now() - 6e5 })) : real(m, a); };
        S.running = id; S.runningSince = Date.now() - 6e5;
        await overlayShown();
        hostPad('a'); o.early = { layers: S.layers.join(','), close: window.__sent.filter(m => m.type === 'overlay' && m.action === 'close').length };
        await wait(500);
        // Steam Input's keys for the same presses (Steam games): dropped, the controller comes from Hearth.
        PAD.host = true; const f0 = focused && focused.dataset.k;
        for (const key of ['ArrowDown', 'ArrowDown', 'Enter']) document.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
        o.echo = { moved: (focused && focused.dataset.k) !== f0, layers: S.layers.join(','), close: window.__sent.filter(m => m.type === 'overlay' && m.action === 'close').length };
        setFocus(document.querySelector('#qam [data-k="q-aside"]'), true);
        hostPad('a'); o.asked = { top: S.layers.at(-1), focus: focused && focused.dataset.k };
        hostPad('a'); o.quick = S.layers.at(-1);   // on Cancel, in the question's first moment: dropped
        await wait(500); hostPad('a'); o.cancel = S.layers.at(-1);
        hostPad('a'); await wait(500); hostPad('up'); hostPad('a');
        o.calls = window.__sent.filter(m => m.type === 'log' && /^quick resume: putting /.test(m.text)).length;   // (the preview has no host to call)
        o.aside = !S.running && QR.list.some(q => q.id === id);
        o.logs = window.__sent.filter(m => m.type === 'log').map(m => m.text);
        return o;
      });
      const perr = m => errors.push(`put aside: ${m}`);
      if (r.early.layers !== 'qam' || r.early.close) perr(`an A right after it opened acted (${JSON.stringify(r.early)})`);
      if (r.echo.moved || r.echo.layers !== 'qam' || r.echo.close) perr(`Steam's arrow keys / Enter over the game acted (${JSON.stringify(r.echo)})`);
      if (r.asked.top !== 'sheet' || r.asked.focus !== 'sh-x') perr(`Put aside didn't ask first with Cancel selected (${JSON.stringify(r.asked)})`);
      if (r.quick !== 'sheet') perr('an A right after the question opened answered it');
      if (r.cancel !== 'qam') perr('Cancel did not go back to Quick Access');
      if (r.calls !== 1 || !r.aside) perr(`put aside ${r.calls} times (want once, after the question; aside: ${r.aside})`);
      if (!r.logs.some(t => /^quick access: A \/ Enter ignored/.test(t))) perr('the dropped A is not logged');
      if (!r.logs.some(t => /^quick resume: Put aside picked for .* by controller a/.test(t)) || !r.logs.some(t => /^quick resume: putting .* aside: confirmed by controller a/.test(t))) perr('what chose Put aside is not logged: ' + r.logs.join(' | '));
      console.log(`put aside: asked first ${r.asked.top} / ${r.asked.focus}, put aside ${r.calls} time(s), early A dropped ${r.early.layers === 'qam'}, Steam keys dropped ${!r.echo.moved}`);
    } catch (e) { errors.push('put aside: ' + ((e && e.message) || e)); }
    await pp.close();
  }


  // Quick Access over a game is the top dock (the approved design): the overlay page (?overlay=full) with a game
  // running, every tab with the preview's data, the controller (left / right switch tabs, down into the panel, up back
  // to the tab, LB / RB), B and Guide back to the game, the quick glance (a short Guide press: shown without opening
  // Quick Access, hidden again by itself), quick replies, Y to type, X to keep the performance settings for the game,
  // and the mouse (a tab click switches, a click beside it goes back to the game). Pictures to tools/overlay-shots
  // (the ui-shots-overlay job posts them as notices, title "SHOT ovl-<tab> <n>").
  {
    const ovShots = path.resolve(__dirname, 'overlay-shots');
    try { require('fs').mkdirSync(ovShots, { recursive: true }); } catch {}
    const op = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    const oerr = m => errors.push('overlay top dock: ' + m);
    op.on('pageerror', e => oerr(e.message));
    const ev = (f, a) => op.evaluate(f, a);
    const shot = async (name) => {
      for (const q of [50, 38, 28, 20, 14]) {
        const buf = await op.screenshot({ type: 'jpeg', quality: q, clip: { x: 160, y: 0, width: 960, height: 640 } });
        if (buf.toString('base64').length <= 6 * 3800 || q === 14) { require('fs').writeFileSync(path.join(ovShots, `${name}.jpg`), buf); break; }
      }
    };
    try {
      await op.goto('file://' + path.resolve(__dirname, '../ui/index.html') + '?preview=linux&overlay=full');
      await op.waitForTimeout(1200);
      await ev(async () => {
        window.__sent = []; window.__calls = [];
        const realSend = send; window.send = m => { window.__sent.push(m); };
        HOSTINFO.features = ['quickResume', 'pause']; QR.on = true;
        const gs = GAMES.filter(g => !g.cloud); window.__game = gs[0].id;
        const real = MOCK.call;
        MOCK.call = (m, a) => { window.__calls.push([m, a]); return m === 'perf.stats' ? real(m, a).then(r => Object.assign({}, r, { runningId: window.__game, runningSince: Date.now() - 72 * 60e3 })) : real(m, a); };
        S.running = window.__game; S.runningSince = Date.now() - 72 * 60e3;
        // The game behind the overlay (the page is see-through): a picture-like background for the screenshots.
        document.head.insertAdjacentHTML('beforeend', '<style>html.ovfull.shotbg{background:linear-gradient(180deg,#7fb7e0 0%,#d9c08a 55%,#8a6a3a 100%)!important}</style>');
        document.documentElement.classList.add('shotbg');
        await overlayShown();
      });
      await op.waitForTimeout(600);
      let st = await ev(() => ({ top: document.querySelector('#qam').classList.contains('top'), tab: S.qtab, focus: focused && focused.dataset.k, tabs: qdTabs().map(t => t[0]).join(' '), shown: OV.shown }));
      if (!st.top) oerr('Quick Access over a game is not the top dock');
      if (st.tab !== 'now' || st.focus !== 'q-resume') oerr(`it opens on ${st.tab} / ${st.focus} (want Now playing / Back to game)`);
      if (st.tabs !== 'now perf controls friends capture quick') oerr('tabs: ' + st.tabs);
      // The game stays in view: the dock and its panel at the top, nothing dimming the lower part of the screen.
      st = await ev(() => { const b = document.querySelector('#qam .qd-bar').getBoundingClientRect(), p = document.querySelector('#qam .qd-panel').getBoundingClientRect(), bg = getComputedStyle(document.body, '::before').backgroundImage; return { barTop: Math.round(b.top), panelTop: Math.round(p.top), below: Math.round(p.top - b.bottom), grad: bg }; });
      if (st.barTop > 40 || st.below < 0 || st.below > 40) oerr(`dock at ${st.barTop}, panel ${st.below} px under it (want the panel right under the dock at the top)`);
      if (!/gradient/.test(st.grad)) oerr('the game behind is dimmed all over (want only a gradient from the top)');
      // Every tab, with the preview's data.
      await ev(async () => { REC.ok = true; REC.st = { kind: 'replay' }; REC.secs = 30; MOCKHHD = true; imLoadTables(); await imSave(window.__game, { layout: 'rts' }); });
      const want = { now: ['q-resume', 'q-invite', 'q-gset', 'q-exit'], perf: ['pf-0', 'po-0', 'q-pg'], controls: ['qi-off', 'qi-rts', 'qi-edit', 'q-imgyro', 'q-mouse'], friends: ['mfx-game', 'mfq-all'], capture: ['q-shot', 'q-rec', 'q-rsave'], quick: ['q-vol', 'q-mute', 'q-wifi', 'q-bt'] };
      for (const t of Object.keys(want)) {
        await ev(t => { qdGo(t); }, t);
        await op.waitForTimeout(t === 'perf' || t === 'friends' || t === 'controls' ? 1600 : 500);
        await ev(() => { const e = qdFirst(); if (e) setFocus(e, true); });
        await op.waitForTimeout(150);
        const keys = await ev(() => [...document.querySelectorAll('#qam .qd-panel [data-f]')].map(e => e.dataset.k));
        for (const k of want[t]) if (!keys.includes(k)) oerr(`${t}: no ${k} (has ${keys.slice(0, 14).join(' ')})`);
        if (t === 'controls' && !(await ev(() => /Right stick/.test((document.querySelector('#qam .qd-sum') || {}).textContent || '')))) oerr('controller layout: no mapping summary for the layout');
        await shot(t);
      }
      // Friends: a chat with a new message opens by itself; quick replies send; Y types with the on-screen keyboard.
      await ev(() => qdGo('friends')); await op.waitForTimeout(1500);
      st = await ev(() => { const r = document.querySelector('#qam [data-act="qdReply"]'); if (r) act(r); return { room: MF.room, replies: [...document.querySelectorAll('#qam [data-act="qdReply"]')].map(e => e.textContent).join('|'), sent: window.__calls.filter(c => c[0] === 'mfriends.send').map(c => c[1].text) }; });
      if (!st.room) oerr("Friends: the chat with a new message didn't open");
      if (st.replies !== 'On my way|GG|👍') oerr('quick replies: ' + st.replies);
      if (!st.sent.includes('On my way')) oerr('a quick reply was not sent');
      st = await ev(() => { input('y'); const top = S.layers.at(-1); if (top === 'prompt') closeLayer(); return top; });
      if (st !== 'prompt') oerr("Y in Friends didn't open the keyboard to type: " + st);
      // The controller.
      st = await ev(() => {
        const k = () => focused && focused.dataset.k, o = [];
        qdGo('now'); setFocus(document.querySelector('#qam [data-k="q-resume"]'), true);
        input('up'); o.push(k());                          // up from the panel's top row: the tab
        input('right'); o.push(S.qtab + ':' + k());        // along the tabs: switches
        input('down'); o.push(document.querySelector('#qam .qd-panel').contains(focused) ? 'panel' : k());
        input('up'); o.push(k());
        input('left'); o.push(S.qtab);
        input('rb'); input('rb'); o.push(S.qtab);
        input('lb'); o.push(S.qtab);
        return o.join(' ');
      });
      if (st !== 'qt-now perf:qt-perf panel qt-perf now controls perf') oerr('controller: ' + st + ' (want qt-now perf:qt-perf panel qt-perf now controls perf)');
      // X on Performance: this game keeps its own settings (and X again: every game).
      st = await ev(async () => { qdGo('perf'); await new Promise(r => setTimeout(r, 900)); const a = qdPgOn(); input('x'); await new Promise(r => setTimeout(r, 400)); const b = qdPgOn(); input('x'); await new Promise(r => setTimeout(r, 400)); return [a, b, qdPgOn()].join(' '); });
      if (st !== 'false true false') oerr('X on Performance (per game): ' + st);
      // B and Guide go back to the game (the overlay closes; the page asks Hearth to close it).
      st = await ev(() => { window.__sent.length = 0; setFocus(document.querySelector('#qam [data-k="qt-perf"]'), true); input('b'); const b = window.__sent.filter(m => m.type === 'overlay' && m.action === 'close').length; input('guide'); return [b, window.__sent.filter(m => m.type === 'overlay' && m.action === 'close').length]; });
      if (st[0] !== 1 || st[1] !== 2) oerr(`B / Guide didn't go back to the game (${st})`);
      // The mouse: a tab click switches, a click beside the dock goes back to the game.
      await ev(() => { window.__sent.length = 0; setInput('mouse'); qdGo('now'); });
      await op.waitForTimeout(300);
      await op.mouse.move(10, 10); await op.mouse.move(400, 300);
      const tb = await ev(() => { const r = document.querySelector('#qam [data-k="qt-quick"]').getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; });
      await op.mouse.click(tb[0], tb[1]); await op.waitForTimeout(300);
      st = await ev(() => ({ tab: S.qtab, hint: !!document.querySelector('#qam .qd-hb') }));
      if (st.tab !== 'quick') oerr('a click on a tab did not switch to it: ' + st.tab);
      if (!st.hint) oerr('mouse mode has no Back to game button');
      await shot('mouse');
      await op.mouse.click(640, 760); await op.waitForTimeout(200);
      st = await ev(() => window.__sent.filter(m => m.type === 'overlay' && m.action === 'close').length);
      if (st !== 1) oerr('a click beside the dock did not go back to the game');
      // The quick glance: Hearth hides Quick Access and shows the glance (osd kind glance); it hides itself after ~3 s.
      await ev(() => { overlayHidden(); onSysEvent('osd', { kind: 'glance', ms: 3000 }); });
      await op.waitForTimeout(700);
      st = await ev(() => { const g = document.querySelector('#glance'); return { on: !!(g && g.classList.contains('on')), vis: g ? getComputedStyle(g).visibility : '', text: g ? g.textContent : '', qam: getComputedStyle(document.querySelector('#qam')).visibility }; });
      if (!st.on || st.vis === 'hidden') oerr('the quick glance did not show');
      if (!/\d/.test(st.text) || !/Hold Guide/.test(st.text)) oerr('the quick glance has no time / tip: ' + st.text);
      if (st.qam !== 'hidden') oerr('Quick Access showed with the glance');
      await shot('glance');
      await op.waitForTimeout(3000);
      st = await ev(() => document.querySelector('#glance').classList.contains('on'));
      if (st) oerr("the quick glance didn't hide by itself");
    } catch (e) { oerr(e.message.split('\n')[0]); }
    await op.close();
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
  // Pictures go to tools/osk-shots (release.yml posts two as notices and keeps all of them on the run).
  {
    const oskShots = path.resolve(__dirname, 'osk-shots');
    try { require('fs').mkdirSync(oskShots, { recursive: true }); } catch {}
    const kp = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    kp.on('pageerror', e => errors.push(`on-screen keyboard: ${e.message}`));
    const kerr = m => errors.push('on-screen keyboard: ' + m);
    const shot = async (name) => { try { await kp.waitForTimeout(260); await kp.screenshot({ path: path.join(oskShots, name + '.jpg'), type: 'jpeg', quality: 30, clip: { x: 0, y: 390, width: 1280, height: 410 } }); } catch (e) { kerr('picture ' + name + ': ' + e.message); } };
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

  // Hearth Link pairing (Settings › Hearth Link): the code here and what's happening with it, the devices on this
  // network with their kind and edition, picking one and typing its code with the number pad, a wrong code asked
  // again with the tries left (nothing to start over), and short codes checked and formatted as they're typed (the
  // same check character as the service, hosts/hearth-link shortcode.rs).
  {
    const lp = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    const lerr = m => errors.push('Hearth Link pairing: ' + m);
    lp.on('pageerror', e => lerr(e.message));
    try {
      await lp.goto('file://' + path.resolve(__dirname, '../ui/index.html') + '?preview=linux');
      await lp.waitForTimeout(1200);
      const r = await lp.evaluate(async () => {
        const wait = ms => new Promise(res => setTimeout(res, ms));
        const out = {};
        const st = { running: true, me: { id: 'a'.repeat(64), name: 'Deck', ticket: 'a'.repeat(64) }, settings: { sync: true }, peers: [], transfers: [], offers: [], sending: [], signInHints: [], shortCodes: true,
          pairing: { code: '482913', until: Date.now() + 290000, triesLeft: 4, netCode: null },
          pairEvent: { state: 'wrong', name: 'Nvidia-PC', triesLeft: 4, at: Date.now() },
          nearby: [{ id: 'b'.repeat(64), name: 'Nvidia-PC', pairing: true, kind: 'desktop', edition: 'Anatase' }, { id: 'c'.repeat(64), name: 'Office', pairing: false, kind: 'laptop', edition: 'Peppermint' }] };
        const calls = [];
        let joins = 0;
        call = async (m, a) => {
          calls.push([m, a]);
          if (m === 'link.status') return st;
          if (m === 'link.pair.join') { joins++; if (joins === 1) throw new Error('Wrong code. 3 tries left'); return { id: 'b'.repeat(64), name: 'Nvidia-PC' }; }
          return {};
        };
        LK.s = st; closeAll(); S.settingsTab = 'link'; navigate('settings', {}, false); render(null, true); await wait(150);
        const v = () => document.querySelector('#view').textContent;
        out.code = /482 913/.test(v()); out.wrongHere = /Nvidia-PC typed a wrong code \(4 tries left\)/.test(v());
        out.kind = /Desktop · Anatase · Ready to pair/.test(v()); out.notReady = /Laptop · Peppermint · To pair, select Pair a device on it first/.test(v());
        out.netBtn = !!document.querySelector('#view [data-k="lk-pnet"]');
        const keys = [...document.querySelectorAll('#view [data-k]')].map(e => e.dataset.k);
        out.dupKeys = keys.filter((k, i) => keys.indexOf(k) !== i);
        act(document.querySelector('#view [data-k="lk-n0"]')); await wait(200);
        out.prompt1 = prompt && prompt.title; out.mode = prompt && prompt.mode;
        prompt.text = '482914'; promptSubmit(); await wait(400);
        out.prompt2 = prompt && prompt.title;
        if (prompt) { prompt.text = '482 913'; promptSubmit(); }
        await wait(400);
        out.joined = calls.filter(c => c[0] === 'link.pair.join').map(c => c[1].code + '@' + c[1].id.slice(0, 2));
        out.done = /Paired ✓ with Nvidia-PC/.test(v());
        // The internet code showing here.
        st.pairing.netCode = '234-567-89A9'; st.pairing.netUntil = Date.now() + 590000; st.pairEvent = null;
        render(null, true); await wait(100);
        out.net = /234-567-89A9/.test(v()) && /Works once/.test(v());
        // Short codes: the page's check is the service's.
        out.fmt = lkShortFmt('k7m4q'); out.ok = lkShortCheck('234-567-89a9'); out.typo = lkShortCheck('234-567-89AA'); out.amb = lkShortCheck('234-567-89A0'); out.short = lkShortCheck('234-567');
        act(document.querySelector('#view [data-k="lk-entercode"]')); await wait(200);
        for (const ch of '23456789a9') promptKey(ch);
        out.typed = prompt && prompt.text; out.typedTitle = prompt && prompt.title;
        promptKey('bksp'); promptKey('b');
        out.typoTitle = prompt && prompt.title;
        closeAll();
        return out;
      });
      const want = { code: true, wrongHere: true, kind: true, notReady: true, netBtn: true, mode: 'number', done: true, net: true, fmt: 'K7M-4Q', ok: '', typed: '234-567-89A9', typedTitle: 'Looks right: press Pair' };
      for (const k of Object.keys(want)) if (r[k] !== want[k]) lerr(`${k}: ${JSON.stringify(r[k])}, wanted ${JSON.stringify(want[k])}`);
      if (r.dupKeys.length) lerr('two rows with the same key: ' + r.dupKeys.join(', '));
      if (!/The 6-digit code Nvidia-PC shows/.test(r.prompt1 || '')) lerr('picking a device asked: ' + r.prompt1);
      if (!/^Wrong code\. 3 tries left\. Type the code Nvidia-PC shows/.test(r.prompt2 || '')) lerr('a wrong code asked again: ' + r.prompt2);
      if (r.joined.join(' ') !== '482914@bb 482913@bb') lerr('codes sent: ' + r.joined.join(' '));
      if (!/one of the characters is off/.test(r.typo) || !/never have 0/.test(r.amb) || !/10 characters/.test(r.short)) lerr('short code checks: ' + JSON.stringify([r.typo, r.amb, r.short]));
      if (!/doesn't look right/.test(r.typoTitle || '')) lerr('a typo while typing: ' + r.typoTitle);
      if (!errors.some(e => e.startsWith('Hearth Link pairing'))) console.log('Hearth Link pairing: code and states, devices with kind and edition, number pad, wrong code asked again, short codes checked and formatted ok');
    } catch (e) { lerr(e.message.split('\n')[0]); }
    await lp.close();
  }

  // The Store's pictures (Steam's page, and a game's page), half size so each fits in the notices release.yml posts.
  {
    const shots = path.resolve(__dirname, 'store-shots');
    try { require('fs').mkdirSync(shots, { recursive: true }); } catch {}
    let sp;
    try { sp = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 0.5 }); }
    catch { sp = await browser.newPage({ viewport: { width: 960, height: 600 } }); }
    sp.on('pageerror', e => errors.push(`store pictures: ${e.message}`));
    try {
      await sp.goto('file://' + path.resolve(__dirname, '../ui/index.html') + '?preview=linux');
      await sp.waitForTimeout(1200);
      const shot = async name => {
        for (const q of [70, 55, 45, 35, 25, 15]) {
          const buf = await sp.screenshot({ type: 'jpeg', quality: q });
          if (buf.toString('base64').length <= 9 * 3800 || q === 15) { require('fs').writeFileSync(path.join(shots, name + '.jpg'), buf); break; }
        }
      };
      await sp.evaluate(() => { closeAll(); S.history = []; SHOP.store = 'steam'; navigate('shop', {}, false); });
      await sp.waitForTimeout(700);
      await sp.evaluate(() => { setFocus(document.querySelector('#view [data-k="sf-hero"]'), true); $('#view').scrollTop = 0; });
      await sp.waitForTimeout(300); await shot('home');
      await sp.evaluate(() => act(document.querySelector('#view [data-k="sf-deals-1"]')));
      await sp.waitForTimeout(700); await shot('game');
    } catch (e) { errors.push('store pictures: ' + e.message.split('\n')[0]); }
    await sp.close();
  }

  // ---- The Bottom Dock (the main menu) and Settings C. Every control the old Settings pages drew
  // (tools/settings-keys.json, per platform) can still be reached from the new front page: group tab › page row › the
  // page (and its tabs). Then the dock: open and close, moving along it, its cards, up into a card, the game dock,
  // search (settings, games, the store), the settings search, the classic side menu, the mouse. Pictures for
  // release.yml go to tools/dock-shots (its ui-shots job posts them as notices).
  {
    const SKEYS = require('./settings-keys.json');
    const dockShots = path.resolve(__dirname, 'dock-shots');
    require('fs').mkdirSync(dockShots, { recursive: true });
    for (const [plat, q] of [['windows', ''], ['linux', '?preview=linux'], ['android', '?preview=android']]) {
      const rp = await browser.newPage({ viewport: { width: 1280, height: 800 } });
      rp.on('pageerror', e => errors.push(`settings C (${plat}): ${e.message}`));
      try {
        await rp.goto('file://' + path.resolve(__dirname, '../ui/index.html') + q);
        await rp.waitForTimeout(1300);
        const r = await rp.evaluate(async () => {
          const wait = ms => new Promise(res => setTimeout(res, ms));
          closeAll(); S.history = []; navigate('home', {}, false);
          input('start');
          const dk = document.querySelector('#menu [data-k="m-settings"]');
          if (!dk) return { err: 'no Settings in the dock' };
          setFocus(dk); act(dk); await wait(300);
          if (S.view !== 'settings' || !S.sfront) return { err: 'Settings from the dock did not open the front page' };
          const found = new Set(), pages = new Set(), groups = [...document.querySelectorAll('#view [data-act="sgroup"]')].map(b => b.dataset.g);
          for (const g of groups) {
            act(document.querySelector(`#view [data-k="sg-${g}"]`));
            if (S.sgroup !== g) return { err: 'the group tab ' + g + " didn't switch" };
            const rows = [...document.querySelectorAll('#view .sc-main [data-act="stab"]')].map(e => e.dataset.tab);
            for (const t of rows) {
              act(document.querySelector(`#view [data-k="sp-${t}"]`)); await wait(350); render(null, true);
              if (S.sfront || S.settingsTab !== t) return { err: 'the row for ' + t + ' did not open it' };
              pages.add(t);
              document.querySelectorAll('#view [data-k]').forEach(e => found.add(e.dataset.k));
              back();
              if (!S.sfront) return { err: 'B in ' + t + ' did not go back to the front page' };
            }
          }
          return { found: [...found], pages: [...pages], groups };
        });
        if (r.err) { errors.push(`settings C (${plat}): ${r.err}`); continue; }
        const want = SKEYS[plat] || {};
        const lostPages = Object.keys(want).filter(t => !r.pages.includes(t) && !(t === 'audio'));
        if (lostPages.length) errors.push(`settings C (${plat}): pages no longer reachable: ${lostPages.join(', ')}`);
        const lost = [];
        for (const [t, keys] of Object.entries(want)) for (const k of keys) if (!r.found.includes(k)) lost.push(`${t}:${k}`);
        if (lost.length) errors.push(`settings C (${plat}): ${lost.length} old settings no longer reachable: ${lost.slice(0, 30).join(', ')}`);
        if (plat !== 'android' && r.groups.length !== 6) errors.push(`settings C (${plat}): ${r.groups.length} groups, not 6: ${r.groups.join(', ')}`);
      } catch (e) { errors.push(`settings C (${plat}): ${e.message.split('\n')[0]}`); }
      await rp.close();
    }
    // Half-size pictures (a notice holds 3800 characters, a step 10 notices).
    let dp; try { dp = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 0.5 }); } catch { dp = await browser.newPage({ viewport: { width: 1280, height: 800 } }); }
    const derr = m => errors.push('dock: ' + m);
    dp.on('pageerror', e => derr(e.message));
    const dshot = async name => { try { for (const q of [70, 55, 45, 35, 25, 15]) { const buf = await dp.screenshot({ type: 'jpeg', quality: q }); if (buf.toString('base64').length <= 9 * 3800 || q === 15) { require('fs').writeFileSync(path.join(dockShots, name + '.jpg'), buf); break; } } } catch (e) { derr('picture ' + name + ': ' + e.message); } };
    const dev = fn => dp.evaluate(fn);
    try {
      await dp.goto('file://' + path.resolve(__dirname, '../ui/index.html') + '?preview=linux');
      await dp.waitForTimeout(1300);
      // Open and close, and the default style.
      let st = await dev(() => { closeAll(); S.history = []; navigate('home', {}, false); input('start');
        return { open: S.layers.at(-1) === 'menu', dock: !!document.querySelector('#menu .dk-bar'), body: document.body.classList.contains('menu-dock'), f: focused && focused.dataset.k, card: document.querySelector('#dkCard').textContent }; });
      if (!st.open || !st.dock || !st.body) derr('☰ did not open the dock: ' + JSON.stringify(st));
      if (st.f !== 'm-home') derr('the dock did not start on Home (the screen showing): ' + st.f);
      if (!/Continue playing/.test(st.card)) derr("Home's card doesn't show the game to continue: " + st.card.slice(0, 80));
      await dp.waitForTimeout(300); await dshot('dock-home');
      st = await dev(() => { input('b'); return { open: S.layers.includes('menu'), mini: !!document.querySelector('#footer .mdock') }; });
      if (st.open) derr('B did not close the dock');
      if (!st.mini) derr('no small dock in the footer while browsing');
      // Along the dock: Library (its counts), Downloads (the queue, Pause all, the ring), up into the card and back.
      st = await dev(() => { input('start'); input('right'); return { f: focused.dataset.k, card: document.querySelector('#dkCard').textContent }; });
      if (st.f !== 'm-library' || !/Installed/.test(st.card)) derr("right didn't go to Library with its card: " + JSON.stringify(st).slice(0, 160));
      st = await dev(() => { let n = 0; while (focused.dataset.k !== 'm-downloads' && n++ < 8) input('right');
        return { f: focused.dataset.k, pause: !!document.querySelector('#dkCard [data-k="mc-pause"]'), open: !!document.querySelector('#dkCard [data-k="mc-dl"]'), ring: !!document.querySelector('#menu [data-k="m-downloads"] .dk-ring'), q: document.querySelectorAll('#dkCard .dkq').length }; });
      if (st.f !== 'm-downloads' || !st.pause || !st.open || !st.ring || !st.q) derr("Downloads' card or ring is missing: " + JSON.stringify(st));
      await dshot('dock-downloads');
      st = await dev(() => { input('up'); const up = focused.closest('#dkCard') ? focused.dataset.k : null; input('down'); return { up, down: focused.dataset.dock || null }; });
      if (!st.up) derr("up didn't go into the card");
      if (st.down !== 'downloads') derr("down from the card didn't come back to Downloads: " + st.down);
      st = await dev(() => { const l = document.querySelector('#menu [data-k="m-library"]'); setFocus(l); act(l); return { v: S.view, open: S.layers.includes('menu') }; });
      if (st.v !== 'library' || st.open) derr("A on Library didn't open the Library and close the dock: " + JSON.stringify(st));
      st = await dev(() => { closeAll(); navigate('home', {}, false); const b = [...document.querySelectorAll('#footer .mdock button')].find(e => e.getAttribute('aria-label') === 'Library'); if (!b) return 'none'; b.click(); return S.view; });
      if (st !== 'library') derr('the small dock in the footer did not open the Library: ' + st);
      // Over a game: the game's card (Resume, Quick Access, Invite, Exit), its tools, the chips.
      st = await dev(async () => { closeAll(); navigate('home', {}, false); S.running = GAMES.find(g => g.inst).id; S.runningSince = Date.now() - 72 * 60000; input('start');
        await new Promise(r => setTimeout(r, 600));
        return { f: focused.dataset.k, keys: [...document.querySelectorAll('#menu [data-k]')].map(e => e.dataset.k), chips: document.querySelectorAll('#menu .dk-chips span').length }; });
      for (const k of ['m-game', 'm-shot', 'm-perf', 'm-home', 'm-settings', 'm-power', 'mc-resume', 'mc-qam', 'mc-exit']) if (!st.keys.includes(k)) derr('the game dock has no ' + k);
      if (st.f !== 'm-game') derr('the game dock did not start on the game: ' + st.f);
      if (!st.chips) derr('the game dock shows no performance chips');
      await dshot('dock-ingame');
      st = await dev(() => { act(document.querySelector('#menu [data-k="mc-qam"]')); const t = S.layers.at(-1); closeAll(); S.running = null; return t; });
      if (st !== 'qam') derr("Quick Access in the game dock didn't open it: " + st);
      // Search from the dock: grouped results; a setting opens its page with that row selected.
      st = await dev(() => { closeAll(); navigate('home', {}, false); input('start'); const s = document.querySelector('#menu [data-k="m-search"]'); setFocus(s); act(s);
        for (const c of 'refresh') typeKey(c);
        const r = document.querySelector('#search [data-k="ss-r-0"]');
        return { layers: S.layers.join(','), over: document.querySelector('#search').classList.contains('over-dock'), first: r && r.textContent, store: !!document.querySelector('#search [data-k="ss-store"]'), groups: [...document.querySelectorAll('#search .sgh')].map(e => e.textContent) }; });
      if (st.layers !== 'menu,search' || !st.over) derr('Search from the dock did not open over it: ' + st.layers);
      if (!/Refresh rate/.test(st.first || '')) derr('settings search for "refresh" did not find Refresh rate first: ' + st.first);
      if (!st.store || !st.groups.includes('Settings') || !st.groups.includes('Store')) derr('search results are not grouped: ' + st.groups.join(', '));
      await dshot('dock-search');
      st = await dev(async () => { act(document.querySelector('#search [data-k="ss-r-0"]')); await new Promise(r => setTimeout(r, 900)); return { v: S.view, t: S.settingsTab, front: S.sfront, f: focused && focused.dataset.k, layers: S.layers.length }; });
      if (st.v !== 'settings' || st.t !== 'display' || st.front || st.f !== 's-hz' || st.layers) derr('a settings result did not open Display with Refresh rate selected: ' + JSON.stringify(st));
      // The front page's search button, and LB / RB.
      st = await dev(() => { closeAll(); navigate('settings', { sfront: true }, false); act(document.querySelector('#view [data-k="ss-search"]')); const o = S.layers.at(-1); closeAll(); return o; });
      if (st !== 'search') derr("Settings' search button didn't open search");
      st = await dev(() => { closeAll(); S.sgroup = 'you'; navigate('settings', { sfront: true, sgroup: 'you' }, false); input('rb'); const a = S.sgroup; input('lb'); const b = S.sgroup;
        S.settingsTab = 'wifi'; navigate('settings', {}, false); input('rb'); const c = S.settingsTab; input('lb'); const d = S.settingsTab; return { a, b, c, d }; });
      if (st.a !== 'connect' || st.b !== 'you') derr('LB / RB on the front page did not switch groups: ' + JSON.stringify(st));
      if (st.c !== 'bluetooth' || st.d !== 'wifi') derr('LB / RB in Connections did not switch pages: ' + JSON.stringify(st));
      await dev(() => { closeAll(); S.sgroup = 'connect'; navigate('settings', { sfront: true, sgroup: 'connect' }, false); });
      await dp.waitForTimeout(300); await dshot('settings-front');
      await dev(() => { S.settingsTab = 'display'; navigate('settings', {}, false); });
      await dp.waitForTimeout(300); await dshot('settings-display');
      // Go back to the previous version (Hearth OS images only; never on Peppermint or with an update waiting).
      st = await dev(() => { const was = [HOSTINFO.system, HOSTINFO.edition, SYSUPD.s]; HOSTINFO.system = true; HOSTINFO.edition = 'anatase';
        const base = { booted: { version: '0.6.51' }, rollback: { version: '0.6.5' }, staged: null };
        SYSUPD.s = { state: 'idle', status: base }; const a = pageSystemUpdate();
        SYSUPD.s = { state: 'idle', status: Object.assign({}, base, { rollbackQueued: true }) }; const b = pageSystemUpdate();
        SYSUPD.s = { state: 'idle', status: Object.assign({}, base, { staged: { version: '0.6.52' } }) }; const c = pageSystemUpdate();
        HOSTINFO.edition = 'peppermint'; SYSUPD.s = { state: 'idle', status: base }; const d = pageSystemUpdate();
        [HOSTINFO.system, HOSTINFO.edition, SYSUPD.s] = was;
        return { a: /data-k="su-rollback"/.test(a) && /Go back to 0\.6\.5/.test(a), b: /su-rbgo/.test(b) && /Stay on 0\.6\.51/.test(b), c: /su-rollback/.test(c), d: /su-rollback/.test(d) }; });
      if (!st.a || !st.b) derr('Updates has no Go back row (or its undo): ' + JSON.stringify(st));
      if (st.c || st.d) derr('Go back shows with an update waiting or on Peppermint: ' + JSON.stringify(st));
      // The classic side menu (Settings › You › Appearance › Menu style), and back to the dock.
      st = await dev(() => { closeAll(); S.settingsTab = 'appearance'; navigate('settings', {}, false); act(document.querySelector('#view [data-k="ap-ms-side"]'));
        const body = document.body.classList.contains('menu-dock'); input('start');
        const r = { body, side: !!document.querySelector('#menu .mitem[data-k="m-library"]'), dock: !!document.querySelector('#menu .dk-bar'), mini: !!document.querySelector('#footer .mdock') };
        closeAll(); S.settingsTab = 'appearance'; navigate('settings', {}, false); act(document.querySelector('#view [data-k="ap-ms-dock"]')); r.back = document.body.classList.contains('menu-dock') && LOOK.menuStyle === 'dock'; return r; });
      if (st.body || !st.side || st.dock || st.mini) derr('Menu style: Side menu did not bring the classic menu back: ' + JSON.stringify(st));
      if (!st.back) derr('Menu style: Dock did not come back');
      // Mouse: pointing at an item selects it (its card follows), a click opens it.
      await dev(() => { closeAll(); LOOK.madeFor = 'mk'; saveLook(); S.history = []; navigate('home', {}, false); input('start'); NAV_AT = 0; });
      const box = await dev(() => { const r = document.querySelector('#menu [data-k="m-shop"]').getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; });
      await dp.waitForTimeout(900);
      await dp.mouse.move(box.x - 60, box.y - 60); await dp.mouse.move(box.x, box.y, { steps: 4 }); await dp.waitForTimeout(200);
      st = await dev(() => ({ f: focused && focused.dataset.k, card: document.querySelector('#dkCard').textContent }));
      if (st.f !== 'm-shop' || !/Store/.test(st.card)) derr('hovering Store with the mouse did not select it: ' + JSON.stringify(st).slice(0, 120));
      await dp.mouse.click(box.x, box.y); await dp.waitForTimeout(300);
      st = await dev(() => ({ v: S.view, open: S.layers.includes('menu') }));
      if (st.v !== 'shop' || st.open) derr('clicking Store in the dock did not open the Store: ' + JSON.stringify(st));
      await dev(() => { closeAll(); LOOK.madeFor = 'auto'; saveLook(); });
    } catch (e) { derr(e.message.split('\n')[0]); }
    await dp.close();
  }

  await browser.close();
  if (errors.length) {
    for (const e of errors) console.log(`::error title=Hearth page::${e}`);
    process.exit(1);
  }
  console.log(`ok: Hearth's page loads and draws ${9 + tabs.length + layouts.length + libLayouts.length} screens (and Beacon's tab, game page, Game setup, Mods, log, jobs, mod sheets and Stores card, and an expired store sign-in) (and ${hhdSecs.length * 2} Handheld Daemon sections, and Quick Access › Performance ${perfModes.length} ways) (and Hearth for Android: ${androidSteps.length} steps) without errors`);
})();
