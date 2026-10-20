// Release check: Hearth's page (ui/index.html) must load and draw every screen without a script error.
// A single error can leave the app blank, so a release that fails this isn't published.
//   npm i --no-save playwright@1.48.2 && npx playwright install chromium && node tools/ui-check.js
const { chromium } = require('playwright');
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
  const layouts = await page.evaluate(() => [...(typeof LAYOUTS !== 'undefined' ? LAYOUTS : []), ...(typeof XLAYOUTS !== 'undefined' ? XLAYOUTS : [])].map(l => l.id));
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
        if (!overlay !== has('[data-act="hhdOpen"]')) throw new Error('the Handheld Daemon settings button is wrong for this page');
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
  await browser.close();
  if (errors.length) {
    for (const e of errors) console.log(`::error title=Hearth page::${e}`);
    process.exit(1);
  }
  console.log(`ok: Hearth's page loads and draws ${9 + tabs.length + layouts.length + libLayouts.length} screens (and Beacon's tab, game page, Game setup, Mods, log, jobs, mod sheets and Stores card, and an expired store sign-in) (and ${hhdSecs.length * 2} Handheld Daemon sections, and Quick Access › Performance ${perfModes.length} ways) without errors`);
})();
