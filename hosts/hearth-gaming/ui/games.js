// Hearth Games' page (ui/games.html): your GOG, Epic and Battle.net games. The window's calls go to the Rust side
// (src/apps/gamestore.rs), which makes them on Hearth's own store engine (hearth-core: gogdl, legendary, Faugus), so
// what's installed here is in Hearth's library and in Steam Game Mode (Hearth Sync). Moving around is kit.js's:
// D-pad / stick / arrows, A chooses, B back, X refreshes, Y searches, LB/RB switch Library, Downloads, Stores and
// Recomps (ui/recomps.js: Hearth Recomps, native PC ports of console games).
'use strict';
const { $, esc, human, when } = KIT;
const call = (method, args = {}) => KIT.invoke('call', { method, args });

const S = {
  tab: 'library', loading: true, info: {}, games: [], acc: {}, bn: {}, dl: { active: [], recent: [] }, drives: null,
  filt: 'all', installedOnly: false, q: '', busy: {}, error: '',
};
const TABS = ['library', 'downloads', 'stores', 'recomps'];
const STORE = { gog: 'GOG', epic: 'Epic Games', battlenet: 'Battle.net' };
const MARK = { gog: 'GOG', epic: 'EPIC', battlenet: 'B.net' };
const game = id => S.games.find(g => g.id === id);
const plural = (n, one, many = one + 's') => `${n} ${n === 1 ? one : many}`;
const signedIn = s => !!S.acc[s]?.signedIn;
const anySignedIn = () => signedIn('gog') || signedIn('epic') || !!S.bn.installed;
const downloading = g => g && !g.installed && ['downloading', 'queued', 'paused', 'installing', 'verifying', 'starting'].includes(g.state);
const pic = g => g.art?.cover || g.art?.square || g.art?.portrait || '';
const widePic = g => g.art?.hero || g.art?.wide || g.art?.background || '';

// ---------------------------------------------------------------- loading
async function loadAll(rescan = false) {
  try {
    const [acc, lib, drives, info] = await Promise.all([call('accounts.status'), call('library.get', { rescan }), KIT.invoke('drives'), S.info.version ? S.info : KIT.invoke('info')]);
    S.info = info || {};
    S.acc = Object.fromEntries((acc.accounts || []).filter(a => a.store === 'gog' || a.store === 'epic').map(a => [a.store, a]));
    S.bn = acc.battlenet || {};
    S.games = (lib.games || []).filter(g => !g.hidden);
    S.drives = drives;
    S.error = '';
  } catch (e) { S.error = KIT.err(e); }
  if (S.loading && S.info.store) S.filt = S.info.store;
  if (S.loading && TABS.includes(S.info.tab)) S.tab = S.info.tab;
  S.loading = false;
  render();
}
async function loadDownloads() {
  try { S.dl = await call('library.downloads'); } catch (e) { return; }
  // Progress shows on the library's tiles too.
  for (const d of S.dl.active || []) { const g = game(d.id); if (g) Object.assign(g, d); }
  if (S.tab === 'downloads') render(); else updateTiles();
}
let pollT = 0;
function poll() {
  clearTimeout(pollT);
  const busy = (S.dl.active || []).length > 0 || S.tab === 'downloads';
  pollT = setTimeout(async () => { await loadDownloads(); poll(); }, busy ? 1500 : 8000);
}
KIT.listen('games://event', ev => {
  const e = ev.payload || {};
  if (e.name === 'battlenet') { S.bn = Object.assign({}, S.bn, e.data || {}); render(); return; }
  if (e.name === 'recomps') { rcEvent(e.data); return; }
  if (['library.changed', 'accounts.changed', 'install.done'].includes(e.name)) {
    if (e.name === 'install.done' && e.data?.title) KIT.toast(e.data.error ? `${e.data.title}: ${e.data.error}` : `${e.data.title} is installed`);
    loadAll(); loadDownloads();
  }
});
// Another start asked for a tab (Settings › Games & stores › Recomps: hearth-gamestore --tab recomps) or a store.
KIT.listen('games://open', ev => {
  const o = ev.payload || {};
  if (KIT.dialogOpen()) { RC.page = null; KIT.close(); }
  if (o.store) S.filt = o.store;
  if (TABS.includes(o.tab)) setTab(o.tab);
});
KIT.listen('games://closing', async () => {
  const n = (S.dl.active || []).length;
  const pick = await KIT.choose(`${plural(n, 'game')} still downloading`, [
    { id: 'keep', label: 'Keep downloading', sub: 'Hearth Games closes by itself when they are done' },
    { id: 'close', label: 'Close now', sub: 'They carry on the next time you open Hearth Games' },
  ]);
  if (pick) KIT.invoke('close_window', { keep: pick === 'keep' });
});

// ---------------------------------------------------------------- actions
async function signIn(store) {
  KIT.toast(`Sign in to ${STORE[store]} in the window that opened`);
  S.busy[store] = true; render();
  try { const r = await call('accounts.signIn', { store }); if (r?.ok) KIT.toast(`Signed in to ${STORE[store]}. Getting your games…`); }
  catch (e) { KIT.alert(`Couldn't sign in to ${STORE[store]}`, esc(KIT.err(e)), false); }
  S.busy[store] = false; await loadAll();
}
async function signOut(store) {
  if (!await KIT.confirm({ title: `Sign out of ${STORE[store]}?`, text: 'Your installed games stay. Hearth keeps a private copy of this sign-in in case you want it back.', ok: 'Sign out', danger: true })) return;
  try { await call('accounts.signOut', { store }); } catch (e) { KIT.toast(KIT.err(e)); }
  loadAll();
}
async function refreshStore(store) {
  S.busy[store] = true; render();
  try { await call('accounts.refresh', { store }); KIT.toast(`${STORE[store]}: your games are up to date`); } catch (e) { KIT.toast(KIT.err(e)); }
  S.busy[store] = false; loadAll();
}
async function refreshAll() {
  KIT.toast('Looking for your games again…');
  await Promise.all(['gog', 'epic'].filter(signedIn).map(s => call('accounts.refresh', { store: s }).catch(() => {})));
  loadAll(true);
}
const driveLabel = d => `${d.name}${d.kind === 'home' && !/device/i.test(d.name) ? ' (this device)' : ''}`;
async function install(g) {
  if (g.source === 'battlenet') return openBattlenet('Install it in Battle.net: it shows here when it\'s done.');
  const drives = S.drives?.drives || [];
  let chosen = drives.find(d => d.current) || drives[0];
  if (drives.length > 1) {
    const id = await KIT.choose(`Install ${g.title}`, drives.map(d => ({ id: d.mount, label: driveLabel(d), end: `${human(d.free)} free`, sub: d.current ? 'Where new games go now' : d.mount })),
      g.sizeBytes ? `It needs about ${human(g.sizeBytes)}. Choose the drive.` : 'Choose the drive.');
    if (!id) return;
    chosen = drives.find(d => d.mount === id);
    if (chosen && !chosen.current) {
      try { S.drives = await KIT.invoke('use_drive', { mount: chosen.mount }); } catch (e) { return KIT.alert('Couldn\'t use that drive', esc(KIT.err(e)), false); }
    }
  }
  try { await call('game.install', { id: g.id }); KIT.toast(`${g.title} is downloading`); } catch (e) { return KIT.alert(`Couldn't install ${g.title}`, esc(KIT.err(e)), false); }
  await loadDownloads(); poll(); loadAll();
}
async function uninstall(g) {
  if (!await KIT.confirm({ title: `Uninstall ${g.title}?`, text: 'Its files are deleted. Saves in the cloud stay; other saves stay in the game\'s Proton folder until Clean up in Settings › Storage removes it.', ok: 'Uninstall', danger: true })) return;
  try { await call('game.uninstall', { id: g.id }); KIT.toast(`${g.title} is uninstalled`); } catch (e) { KIT.alert(`Couldn't uninstall ${g.title}`, esc(KIT.err(e)), false); }
  loadAll();
}
async function play(g) {
  try { await KIT.invoke('play', { id: g.id }); KIT.toast(`Starting ${g.title}…`); } catch (e) { KIT.alert(`Couldn't start ${g.title}`, esc(KIT.err(e)), false); }
}
async function dlOp(op, id) { try { await call(op, id ? { id } : {}); } catch (e) { KIT.toast(KIT.err(e)); } loadDownloads(); }
async function openBattlenet(msg) {
  if (!S.bn.installed) return setupBattlenet();
  try { await call('battlenet.signInDesktop'); KIT.toast(msg || 'Opening Battle.net…'); } catch (e) { KIT.toast(KIT.err(e)); }
}
async function setupBattlenet() {
  if (!await KIT.confirm({ title: 'Set up Battle.net?', text: `Hearth downloads Blizzard's own Battle.net app and sets it up${S.bn.faugus ? ' through Faugus Launcher' : ''}. You sign in in Battle.net's own window; Hearth never sees your password.`, ok: 'Set up' })) return;
  try { S.bn = await call('battlenet.install'); } catch (e) { KIT.toast(KIT.err(e)); }
  render();
}

// ---------------------------------------------------------------- a game
function openGame(g) {
  const dl = downloading(g);
  const pct = g.progress != null ? Math.round(g.progress * 100) : null;
  const acts = [];
  if (g.installed) acts.push(`<button class="btn primary f" data-g="play" data-autofocus>▶ Play</button>`);
  else if (dl) {
    acts.push(g.state === 'paused' ? `<button class="btn primary f" data-g="resume" data-autofocus>Carry on</button>` : `<button class="btn f" data-g="pause" data-autofocus>Pause</button>`);
    acts.push(`<button class="btn danger f" data-g="cancel">Cancel</button>`);
  } else if (g.source === 'battlenet' && g.id !== 'battlenet:app') acts.push(`<button class="btn primary f" data-g="install" data-autofocus>Install in Battle.net</button>`);
  else if (g.installKind === 'nimbus') acts.push(`<button class="btn primary f" data-g="install" data-autofocus>Install</button>`);
  if (g.installed && g.canUninstall && g.source !== 'battlenet') acts.push(`<button class="btn danger f" data-g="uninstall">Uninstall</button>`);
  acts.push(`<button class="btn f" data-g="close">Back</button>`);
  const facts = [
    ['Store', STORE[g.source] || g.source],
    g.installed && g.path ? ['Installed in', `<span class="path">${esc(g.path)}</span>`] : null,
    g.sizeBytes ? ['Size', human(g.sizeBytes)] : null,
    g.minutes ? ['Played', g.minutes >= 60 ? `${Math.round(g.minutes / 6) / 10} hours` : plural(g.minutes, 'minute')] : null,
    g.lastPlayed ? ['Last played', when(g.lastPlayed)] : null,
    dl ? ['Download', `${g.state === 'paused' ? 'Paused' : 'Downloading'}${pct != null ? `, ${pct}%` : ''}${g.bytesTotal ? ` of ${human(g.bytesTotal)}` : ''}`] : null,
  ].filter(Boolean);
  const wide = widePic(g) || pic(g);
  const d = KIT.dialog(`${wide ? `<div class="hero" style="background-image:url('${esc(wide)}')"></div>` : ''}<h2>${esc(g.title)}</h2>
    <dl class="kv">${facts.map(([k, v]) => `<dt>${esc(k)}</dt><dd>${typeof v === 'string' && v.startsWith('<') ? v : esc(v)}</dd>`).join('')}</dl>
    ${g.note ? `<p class="note">${esc(g.note)}</p>` : ''}
    ${!g.installed && !dl && g.installKind !== 'nimbus' && g.source !== 'battlenet' ? '<p class="note">Hearth can\'t install this one itself; its store\'s own app can.</p>' : ''}
    <div class="row">${acts.join('')}</div>`, { wide: true });
  d.onclick = e => {
    const b = e.target.closest('[data-g]'); if (!b) return;
    const a = b.dataset.g; KIT.close();
    if (a === 'play') play(g); else if (a === 'install') install(g); else if (a === 'uninstall') uninstall(g);
    else if (a === 'pause') dlOp('game.pauseInstall', g.id); else if (a === 'resume') dlOp('game.resumeInstall', g.id);
    else if (a === 'cancel') dlOp('game.cancelInstall', g.id);
  };
}

// ---------------------------------------------------------------- drawing
function tileHtml(g) {
  const dl = downloading(g);
  const p = pic(g), w = !p && widePic(g);
  const state = g.installed ? '<span class="badge good">Installed</span>' : dl ? `<span class="badge">${g.state === 'paused' ? 'Paused' : g.progress != null ? Math.round(g.progress * 100) + '%' : 'Queued'}</span>` : '';
  return `<button class="tile f ${g.installed || dl ? '' : 'dim'}" data-id="${esc(g.id)}">
    <span class="cover">${p ? `<img loading="lazy" src="${esc(p)}" alt="">` : w ? `<img class="wide" loading="lazy" src="${esc(w)}" alt="">` : `<span class="letter">${esc((g.title || '?')[0])}</span>`}
      ${dl ? `<span class="pbar"><i style="width:${Math.round((g.progress || 0) * 100)}%"></i></span>` : ''}</span>
    <span class="t">${esc(g.title)}</span><span class="s"><span>${esc(STORE[g.source] || g.source)}</span>${state}</span></button>`;
}
function updateTiles() {
  for (const d of S.dl.active || []) {
    const el = document.querySelector(`.tile[data-id="${CSS.escape(d.id)}"]`);
    const g = game(d.id); if (!el || !g) continue;
    const i = el.querySelector('.pbar i'); if (i) i.style.width = `${Math.round((g.progress || 0) * 100)}%`;
    const b = el.querySelector('.badge'); if (b && g.progress != null && g.state !== 'paused') b.textContent = `${Math.round(g.progress * 100)}%`;
  }
}
function shown() {
  const q = S.q.trim().toLowerCase();
  return S.games.filter(g => (S.filt === 'all' || g.source === S.filt) && (!S.installedOnly || g.installed || downloading(g)) && (!q || g.title.toLowerCase().includes(q)))
    .sort((a, b) => (b.installed - a.installed) || (downloading(b) - downloading(a)) || (b.lastPlayed || 0) - (a.lastPlayed || 0) || a.title.localeCompare(b.title));
}
function helloHtml() {
  return `<div class="hello"><h2>Your GOG, Epic and Battle.net games, in one place</h2>
    <p class="msg">Sign in on the store's own page: Hearth never sees your password. Your games show here, install with a press, and show up in Steam's Game Mode too.</p>
    <div class="acts"><button class="btn primary f" data-act="signin" data-store="gog" data-autofocus>Sign in to GOG</button><button class="btn primary f" data-act="signin" data-store="epic">Sign in to Epic Games</button><button class="btn f" data-act="battlenet">${S.bn.installed ? 'Open Battle.net' : 'Set up Battle.net'}</button></div></div>`;
}
function libraryHtml() {
  if (!anySignedIn() && !S.games.length) return `<div class="scroll">${helloHtml()}</div>`;
  const list = shown();
  const chips = [['all', 'All'], ['gog', 'GOG'], ['epic', 'Epic Games'], ['battlenet', 'Battle.net']]
    .map(([k, l]) => `<button class="chip f ${S.filt === k ? 'on' : ''}" data-filt="${k}">${l}</button>`).join('');
  return `<div class="bar">${chips}<button class="chip f ${S.installedOnly ? 'on' : ''}" data-act="installed">Installed</button><span class="spacer"></span>
      <input class="inp f" id="q" placeholder="Search your games" data-osk value="${esc(S.q)}"><button class="btn f" data-act="refresh">Refresh</button></div>
    <div class="scroll"><div class="grid">${list.map(tileHtml).join('') || `<div class="empty" style="grid-column:1/-1">${S.q ? 'No game by that name.' : 'No games here yet.'}</div>`}</div></div>`;
}
function downloadsHtml() {
  const act = S.dl.active || [];
  const rows = act.map(d => {
    const g = game(d.id) || d; const pct = d.progress != null ? Math.round(d.progress * 100) : null;
    return `<div class="item dl"><span class="thumb" style="background-image:url('${esc(pic(g))}')"></span>
      <span class="n">${esc(g.title)}</span>
      <span class="acts">${d.state === 'paused' ? `<button class="btn primary f" data-dl="game.resumeInstall" data-id="${esc(d.id)}">Carry on</button>` : `<button class="btn f" data-dl="game.pauseInstall" data-id="${esc(d.id)}">Pause</button>`}<button class="btn danger f" data-dl="game.cancelInstall" data-id="${esc(d.id)}">Cancel</button></span>
      <span class="m">${esc(STORE[g.source] || '')} · ${d.state === 'paused' ? 'Paused' : d.state === 'queued' ? 'Waiting its turn' : 'Downloading'}${pct != null ? ` · ${pct}%` : ''}${d.bytesTotal ? ` of ${human(d.bytesTotal)}` : ''}${d.bytesPerSec ? ` · ${human(d.bytesPerSec)}/s` : ''}</span>
      <span class="prog" style="margin:.2em 0 0"><i style="width:${pct ?? 0}%"></i></span></div>`;
  }).join('');
  const recent = (S.dl.recent || []).map(r => game(r.id)).filter(Boolean).slice(0, 8);
  return `<div class="bar"><span class="msg">${act.length ? plural(act.length, 'download') : 'Nothing is downloading.'}</span><span class="spacer"></span>
      ${act.length ? '<button class="btn f" data-dl="downloads.pauseAll">Pause all</button><button class="btn f" data-dl="downloads.resumeAll">Carry on with all</button>' : ''}</div>
    <div class="scroll"><div class="list">${rows}</div>
      ${recent.length ? `<h3 style="margin:1.2em 0 .5em">Installed lately</h3><div class="grid">${recent.map(tileHtml).join('')}</div>` : ''}
      ${S.drives?.installRoot ? `<p class="note" style="margin-top:1.2em">New GOG and Epic games go to <span class="path">${esc(S.drives.installRoot)}</span>. Change it on the Stores tab.</p>` : ''}</div>`;
}
function storeCard(s) {
  const a = S.acc[s] || {};
  const busy = S.busy[s] || a.busy;
  const head = `<div class="who"><span class="storemark mark-${s}">${MARK[s]}</span><span><b>${STORE[s]}</b><small>${a.signedIn ? `Signed in${a.name ? ` as ${esc(a.name)}` : ''}` : a.expired ? 'The sign-in ran out' : 'Not signed in'}</small></span></div>`;
  const facts = a.signedIn ? `<p class="msg small">${plural(a.owned || 0, 'game')} on your account · ${a.installed || 0} installed${a.fetched ? ` · looked ${when(a.fetched)}` : ''}${busy ? ' · getting your games…' : ''}</p>` : '';
  const err = a.error ? `<p class="bad small">${esc(a.error)}</p>` : '';
  const acts = a.signedIn
    ? `<button class="btn f" data-act="refresh-store" data-store="${s}" ${busy ? 'disabled' : ''}>Look for new games</button><button class="btn f" data-act="shop" data-store="${s}">Open the store</button><button class="btn danger f" data-act="signout" data-store="${s}">Sign out</button>`
    : `<button class="btn primary f" data-act="signin" data-store="${s}" ${busy ? 'disabled' : ''}>${a.expired ? 'Sign in again' : 'Sign in'}</button><button class="btn f" data-act="shop" data-store="${s}">Open the store</button>`;
  return `<div class="panel card">${head}${facts}${err}<div class="acts">${acts}</div></div>`;
}
function battlenetCard() {
  const b = S.bn;
  const state = b.busy ? `Setting up: ${esc(b.stage || 'working')}…` : b.installed ? `Set up${b.via === 'faugus' ? ' through Faugus Launcher' : ''} · ${plural(b.games || 0, 'game')} installed` : 'Not set up';
  const acts = b.installed
    ? `<button class="btn primary f" data-act="battlenet">Open Battle.net</button><button class="btn f" data-act="shop" data-store="battlenet">Open the shop</button>`
    : `<button class="btn primary f" data-act="battlenet-setup" ${b.busy ? 'disabled' : ''}>Set up Battle.net</button>`;
  return `<div class="panel card"><div class="who"><span class="storemark mark-battlenet">${MARK.battlenet}</span><span><b>Battle.net</b><small>${state}</small></span></div>
    <p class="msg small">Blizzard's own app${b.faugus ? ', run through Faugus Launcher' : ''}. You sign in, install and update Blizzard games there; they show here and in Steam's Game Mode.</p>
    ${b.error ? `<p class="bad small">${esc(b.error)}</p>` : ''}<div class="acts">${acts}</div></div>`;
}
function storesHtml() {
  const drives = S.drives?.drives || [];
  const proton = S.info.proton ? `Windows games start with ${esc(S.info.proton)}, built into Hearth OS, so they work offline.` : 'Windows games start with Proton, downloaded the first time a game needs it.';
  return `<div class="scroll"><div class="cards">${storeCard('gog')}${storeCard('epic')}${battlenetCard()}
    <div class="panel card"><h3>Where new games go</h3>
      <div class="chips">${drives.map(d => `<button class="chip f ${d.current ? 'on' : ''}" data-drive="${esc(d.mount)}">${esc(driveLabel(d))}<small>${human(d.free)} free</small></button>`).join('') || '<span class="msg">Looking at your drives…</span>'}</div>
      ${S.drives?.installRoot ? `<p class="note">Now: <span class="path">${esc(S.drives.installRoot)}</span></p>` : ''}
      <p class="note">${proton}</p></div>
  </div></div>`;
}
function render() {
  document.querySelectorAll('#tabs .tab').forEach(t => t.classList.toggle('on', t.dataset.tab === S.tab));
  const n = S.games.length, inst = S.games.filter(g => g.installed).length;
  const ports = S.tab === 'recomps' && RC.data ? rcAll().filter(e => !e.hidden) : null;
  $('#sub').textContent = ports ? `${plural(ports.length, 'port')} · ${ports.filter(e => e.installed).length} installed`
    : S.loading ? 'Reading your library…' : n ? `${plural(n, 'game')} · ${inst} installed` : '';
  const ae = document.activeElement;
  const had = ae?.closest('#main') ? (ae.dataset.id || ae.dataset.act || ae.dataset.filt || ae.dataset.rc || ae.dataset.rcs || ae.dataset.rca || ae.id) : null;
  const main = $('#main');
  if (S.loading) main.innerHTML = '<div class="empty"><div class="prog spin"><i></i></div>Reading your library…</div>';
  else main.innerHTML = (S.error ? `<p class="bad">${esc(S.error)}</p>` : '') + (S.tab === 'library' ? libraryHtml() : S.tab === 'downloads' ? downloadsHtml() : S.tab === 'recomps' ? recompsHtml() : storesHtml());
  $('#foot').innerHTML = '<span><b>A</b>Choose</span><span><b>B</b>Back</span><span><b>X</b>Refresh</span>' + (S.tab === 'library' || S.tab === 'recomps' ? '<span><b>Y</b>Search</span>' : '') + '<span><b class="wide">LB</b><b class="wide">RB</b>Library, Downloads, Stores, Recomps</span>';
  if (had && !KIT.dialogOpen()) {
    const h = CSS.escape(had);
    const el = document.querySelector(`#main [data-id="${h}"], #main [data-act="${h}"], #main [data-filt="${h}"], #main [data-rc="${h}"], #main [data-rcs="${h}"], #main [data-rca="${h}"], #main #${h}`);
    if (el) el.focus();
  }
  if (S.tab === 'recomps') rcAfterRender();
  const q = $('#q'); if (q) q.oninput = () => { S.q = q.value; const g = $('.grid'); if (g) g.innerHTML = shown().map(tileHtml).join('') || '<div class="empty" style="grid-column:1/-1">No game by that name.</div>'; };
}
function setTab(t) { S.tab = t; render(); KIT.focusFirst('#main .f'); poll(); }

document.addEventListener('click', e => {
  if (KIT.dialogOpen()) return;
  const t = e.target.closest('.tab'); if (t) return setTab(t.dataset.tab);
  if (S.tab === 'recomps' && rcClick(e)) return;
  const tile = e.target.closest('.tile'); if (tile) { const g = game(tile.dataset.id); if (g) openGame(g); return; }
  const f = e.target.closest('[data-filt]'); if (f) { S.filt = f.dataset.filt; render(); return; }
  const dv = e.target.closest('[data-drive]'); if (dv) { KIT.invoke('use_drive', { mount: dv.dataset.drive }).then(r => { S.drives = r; KIT.toast('New games go there now'); render(); }).catch(err => KIT.toast(KIT.err(err))); return; }
  const dl = e.target.closest('[data-dl]'); if (dl) return dlOp(dl.dataset.dl, dl.dataset.id);
  const b = e.target.closest('[data-act]'); if (!b) return;
  const s = b.dataset.store;
  switch (b.dataset.act) {
    case 'signin': return signIn(s);
    case 'signout': return signOut(s);
    case 'refresh-store': return refreshStore(s);
    case 'refresh': return refreshAll();
    case 'installed': S.installedOnly = !S.installedOnly; return render();
    case 'shop': return KIT.invoke('open_store', { store: s }).then(() => KIT.toast('Opened in your browser. Bought something? Look for new games here after.')).catch(err => KIT.toast(KIT.err(err)));
    case 'battlenet': return openBattlenet();
    case 'battlenet-setup': return setupBattlenet();
  }
});
KIT.onTab = d => setTab(TABS[(TABS.indexOf(S.tab) + d + TABS.length) % TABS.length]);
KIT.onX = () => { if (S.tab === 'recomps') { rcLoad(); KIT.toast('Recomps: looking again…'); } else refreshAll(); };
KIT.onY = () => { const q = S.tab === 'library' ? $('#q') : S.tab === 'recomps' ? $('#rq') : null; if (q) { q.focus(); if (KIT.usingPad()) KIT.openOsk(q); } };
KIT.onBack = () => {
  if (S.tab === 'recomps' && RC.q) { RC.q = ''; render(); }
  else if (S.tab !== 'library') setTab('library'); else if (S.q) { S.q = ''; render(); }
};

// ---------------------------------------------------------------- preview without the app (a plain browser)
if (!KIT.live) {
  const cover = (h, s) => `data:image/svg+xml;utf8,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 300"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${h},55%,42%)"/><stop offset="1" stop-color="hsl(${(h + 40) % 360},60%,16%)"/></linearGradient></defs><rect width="200" height="300" fill="url(#g)"/><text x="100" y="250" font-family="sans-serif" font-size="22" font-weight="700" fill="#fff" text-anchor="middle">${s}</text></svg>`)}`;
  const G = (id, title, source, h, extra = {}) => Object.assign({ id, title, source, art: { cover: cover(h, title.split(' ')[0]) }, installKind: source === 'battlenet' ? null : 'nimbus', installed: false, state: 'notInstalled', canUninstall: true }, extra);
  const games = [
    G('gog:1', 'Witcher Tales', 'gog', 10, { installed: true, state: 'installed', path: '/home/you/Games/Witcher Tales', sizeBytes: 48e9, minutes: 1260, lastPlayed: Date.now() - 864e5 }),
    G('epic:a', 'Neon Drift', 'epic', 200, { state: 'downloading', progress: .42, bytesTotal: 31e9, bytesDone: 13e9 }),
    G('gog:2', 'Pixel Keep', 'gog', 120, { installed: true, state: 'installed', sizeBytes: 2.1e9 }),
    G('battlenet:wow', 'World of Warcraft', 'battlenet', 30, { installed: true, state: 'installed' }),
    G('epic:b', 'Starfall Arena', 'epic', 260), G('gog:3', 'Old Ruins', 'gog', 80), G('epic:c', 'Moss Garden', 'epic', 150),
    G('gog:4', 'Copper Skies', 'gog', 40), G('epic:d', 'Hollow Tide', 'epic', 300), G('gog:5', 'Lantern Road', 'gog', 60),
  ];
  KIT.demo = (cmd, a) => {
    if (cmd === 'info') return { version: '1.7.0', proton: 'GE-Proton10-25', canPlay: true };
    if (cmd === 'drives') return { installRoot: '/home/you/Games', drives: [{ name: 'This device', kind: 'home', mount: '/', free: 212e9, current: true }, { name: 'Games SSD', kind: 'disk', mount: '/var/mnt/games-1', free: 1.4e12 }] };
    if (cmd === 'call') {
      const m = a.method;
      if (m.startsWith('recomps.')) return rcDemo(m, a.args || {});
      if (m === 'accounts.status') return { accounts: [{ store: 'gog', signedIn: true, name: 'dragon', owned: 6, installed: 2, fetched: Date.now() - 36e5 }, { store: 'epic', signedIn: true, name: 'dragon', owned: 4, installed: 0, fetched: Date.now() - 72e5 }], battlenet: { installed: true, via: 'faugus', faugus: true, games: 1 } };
      if (m === 'library.get') return { games };
      if (m === 'library.downloads') return { active: [games[1]], recent: [{ id: 'gog:2' }] };
      return {};
    }
    return {};
  };
  const want = new URLSearchParams(location.search).get('tab'); if (TABS.includes(want)) S.tab = want;
}

render();
loadAll().then(() => { loadDownloads(); poll(); KIT.focusFirst('#main .f'); });
