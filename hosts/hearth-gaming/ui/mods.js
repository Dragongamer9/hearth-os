// Hearth Mods' page (ui/mods.html, ui/mods.css): the window's calls go to the Rust side (src/apps/mods.rs), the
// moving around is kit.js's (D-pad / stick / arrows, A chooses, B back, X and Y this page's, LB/RB the sections).
//
// Game first: the Library is your moddable games as art cards; a game opens with its hero (art, how many mods are
// on, how healthy the load order is, when the mods last went in, and Play / Deploy / Add mods / Nexus / Modlists),
// then the mod list as a priority ladder (lower wins) with a details panel (about, conflicts, notes). A game with a
// Hearth Mod Organizer instance shows the organizer's list (separators, plugins, profiles) in the same place.
// Modlists, Downloads, Organizer, Accounts and Settings are the rail's other sections.
'use strict';
const { $, esc, human, when } = KIT;
// Every call but the window's own (the file chooser) goes through `call`: answered by this window when it's Hearth
// Mods' worker, or by the worker that's running (Game Mode's), so both see the same queue and installs.
{ const direct = KIT.invoke, own = ['info', 'pick_path']; KIT.invoke = (cmd, args = {}) => KIT.live && !own.includes(cmd) ? direct('call', { name: cmd, args }) : direct(cmd, args); }

const S = {
  section: 'library', info: {}, loading: true, games: [], insts: [],
  view: null,                // null: the library; {type: 'game', id} or {type: 'inst', id, game}
  sum: null, org: null, orgTab: 'mods', sel: null, q: '', filt: 'all', detTab: 'about', grab: null,
  updates: {}, nx: {}, conf: {}, art: {},
  acct: undefined, accts: [], dls: { items: [], active: 0, quota: {} }, engine: null, gallery: null, galleryErr: '', jobs: [],
  wjGame: 'all', wjQ: '', nsfw: false,
};
const SECTIONS = [['library', 'Library'], ['modlists', 'Modlists'], ['downloads', 'Downloads'], ['organizer', 'Organizer'], ['accounts', 'Accounts'], ['settings', 'Settings']];
const SRC = { steam: 'Steam', epic: 'Epic', gog: 'GOG', manual: 'Your game', beacon: 'Beacon', 'heroic-epic': 'Heroic (Epic)', 'heroic-gog': 'Heroic (GOG)' };
// Wabbajack's names for the games it has modlists for (Steam app → name), for a game's "Install a modlist".
const WJ_STEAM = { 489830: 'SkyrimSpecialEdition', 72850: 'Skyrim', 611670: 'SkyrimVR', 377160: 'Fallout4', 611660: 'Fallout4VR', 22380: 'FalloutNewVegas', 22490: 'FalloutNewVegas', 22300: 'Fallout3', 22330: 'Oblivion', 2623190: 'OblivionRemastered', 1716740: 'Starfield', 1091500: 'Cyberpunk2077', 1086940: 'BaldursGate3' };
const WJ_NAMES = { SkyrimSpecialEdition: 'Skyrim SE', Skyrim: 'Skyrim', SkyrimVR: 'Skyrim VR', Fallout4: 'Fallout 4', Fallout4VR: 'Fallout 4 VR', FalloutNewVegas: 'New Vegas', Fallout3: 'Fallout 3', Oblivion: 'Oblivion', OblivionRemastered: 'Oblivion Remastered', Starfield: 'Starfield', Cyberpunk2077: 'Cyberpunk 2077', BaldursGate3: "Baldur's Gate 3" };
const game = id => S.games.find(g => g.id === id);
const instOf = gid => S.insts.find(i => i.libraryId === gid);
const plural = (n, one, many = one + 's') => `${n} ${n === 1 ? one : many}`;

// ---------------------------------------------------------------- icons (24px line icons)
const svg = (d, fill) => `<svg viewBox="0 0 24 24" fill="${fill ? 'currentColor' : 'none'}" stroke="${fill ? 'none' : 'currentColor'}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">${d}</svg>`;
const I = {
  layers: svg('<path d="M4 7l8-4 8 4-8 4z"/><path d="M4 12l8 4 8-4M4 17l8 4 8-4"/>'),
  library: svg('<rect x="3.5" y="4" width="7" height="7" rx="1.5"/><rect x="13.5" y="4" width="7" height="7" rx="1.5"/><rect x="3.5" y="14" width="7" height="6" rx="1.5"/><rect x="13.5" y="14" width="7" height="6" rx="1.5"/>'),
  modlists: svg('<path d="M5 5h14M5 10h14M5 15h9"/><circle cx="18" cy="17.5" r="3"/><path d="M18 16v1.5l1 .8"/>'),
  downloads: svg('<path d="M12 4v11M7.5 10.5 12 15l4.5-4.5M5 19h14"/>'),
  organizer: svg('<path d="M4 6h10M4 12h16M4 18h12"/><circle cx="17" cy="6" r="2"/><circle cx="19" cy="18" r="2"/>'),
  accounts: svg('<circle cx="12" cy="8.5" r="3.5"/><path d="M5 20c1.2-3.6 4-5.4 7-5.4s5.8 1.8 7 5.4"/>'),
  settings: svg('<path d="M12 15.2a3.2 3.2 0 1 0 0-6.4 3.2 3.2 0 0 0 0 6.4zM19.4 13.5l1.6 1.2-1.8 3.1-1.9-.7a7.4 7.4 0 0 1-2 1.2l-.3 2h-3.6l-.3-2a7.4 7.4 0 0 1-2-1.2l-1.9.7-1.8-3.1 1.6-1.2a7.6 7.6 0 0 1 0-2.4L3.4 9.7l1.8-3.1 1.9.7a7.4 7.4 0 0 1 2-1.2l.3-2h3.6l.3 2a7.4 7.4 0 0 1 2 1.2l1.9-.7 1.8 3.1-1.6 1.2a7.6 7.6 0 0 1 0 2.4z"/>'),
  play: svg('<path d="M8 5.5v13a1 1 0 0 0 1.5.86l10.4-6.5a1 1 0 0 0 0-1.72L9.5 4.64A1 1 0 0 0 8 5.5z"/>', true),
  plus: svg('<path d="M12 5v14M5 12h14"/>'),
  deploy: svg('<path d="M12 3v10M8 9l4 4 4-4"/><path d="M4 15v3a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3"/>'),
  globe: svg('<circle cx="12" cy="12" r="8.5"/><path d="M3.5 12h17M12 3.5c2.5 2.6 3.6 5.4 3.6 8.5S14.5 17.9 12 20.5C9.5 17.9 8.4 15.1 8.4 12S9.5 6.1 12 3.5z"/>'),
  more: svg('<circle cx="5.5" cy="12" r="1.4"/><circle cx="12" cy="12" r="1.4"/><circle cx="18.5" cy="12" r="1.4"/>', true),
  back: svg('<path d="M14.5 6 8.5 12l6 6"/>'),
  search: svg('<circle cx="10.5" cy="10.5" r="6"/><path d="M15 15l5 5"/>'),
  grip: '<svg viewBox="0 0 10 16" fill="currentColor"><circle cx="3" cy="3" r="1.3"/><circle cx="7" cy="3" r="1.3"/><circle cx="3" cy="8" r="1.3"/><circle cx="7" cy="8" r="1.3"/><circle cx="3" cy="13" r="1.3"/><circle cx="7" cy="13" r="1.3"/></svg>',
  up: svg('<path d="M12 19V5M6 11l6-6 6 6"/>'),
  down: svg('<path d="M12 5v14M6 13l6 6 6-6"/>'),
  over: svg('<path d="M12 18V7M7.5 11 12 6.5l4.5 4.5"/>'),
  under: svg('<path d="M12 6v11M7.5 13 12 17.5l4.5-4.5"/>'),
  upd: svg('<path d="M20 12a8 8 0 1 1-2.3-5.6M20 4v4.5h-4.5"/>'),
  folder: svg('<path d="M3.5 7a2 2 0 0 1 2-2h4l2 2.2h7a2 2 0 0 1 2 2V17a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/>'),
  trash: svg('<path d="M5 7h14M10 7V5h4v2M7 7l1 12h8l1-12"/>'),
  pause: svg('<path d="M9 6v12M15 6v12"/>'),
  check: svg('<path d="m5 12.5 4.5 4.5L19 7.5"/>'),
  warn: svg('<path d="M12 4 2.8 19.5h18.4z"/><path d="M12 10v4M12 17v.2"/>'),
  steamadd: svg('<rect x="3.5" y="5" width="12" height="14" rx="2"/><path d="M9.5 9.5v5M7 12h5M19.5 8.5v7M16 12h7"/>'),
  link: svg('<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/>'),
};

// ---------------------------------------------------------------- loading
async function load() {
  S.loading = true; render();
  try { S.games = await KIT.invoke('list'); } catch (e) { S.games = []; KIT.toast(KIT.err(e)); }
  await loadInsts();
  S.loading = false;
  const start = S.info.game && game(S.info.game);
  if (start) return openGame(start.id);
  render();
}
async function loadInsts() { try { S.insts = (await org(['instances'])).instances || []; } catch (e) { S.insts = []; } }
/** A library game's picture: the art cache's copy (as a data: address) or its web address, fetched once. */
function art(id, kind) {
  const k = `${id}|${kind}`;
  if (k in S.art) return S.art[k] || '';
  const g = game(id); if (!g || !g.art || !g.art[kind]) { S.art[k] = null; return ''; }
  S.art[k] = null;
  KIT.invoke('art', { id, kind }).then(u => { if (!u) return; S.art[k] = u; document.querySelectorAll(`[data-art="${CSS.escape(k)}"]`).forEach(el => { el.style.backgroundImage = `url("${u}")`; el.classList.add('has-art'); }); }).catch(() => {});
  return '';
}
const artAttr = (id, kind) => { const u = id ? art(id, kind) : ''; return `data-art="${esc(`${id}|${kind}`)}" style="${u ? `background-image:url('${esc(u)}')` : ''}"`; };
const wideOf = g => g.art?.hero && !g.art?.wide ? 'hero' : 'wide';

async function openGame(id, focus = true) {
  const g = game(id); if (!g) return;
  S.section = 'library'; S.view = { type: 'game', id }; S.sum = null; S.org = null; S.sel = null; S.q = ''; S.filt = 'all'; S.grab = null;
  render();
  if (!g.beacon) {
    try { S.sum = await KIT.invoke('mods_of', { id }); } catch (e) { KIT.toast(KIT.err(e)); }
    if (S.sum?.organizer?.id) { S.view = { type: 'inst', id: S.sum.organizer.id, game: id }; await loadOrg(); }
  }
  render();
  if (focus) KIT.focusFirst(g.beacon ? '[data-key="beacon"]' : '[data-key="play"]');
}
async function openInst(iid) {
  const inst = S.insts.find(i => i.id === iid);
  const g = inst && (game(inst.libraryId) || S.games.find(x => inst.gameDir && x.install === inst.gameDir));
  S.section = S.section === 'organizer' ? 'organizer' : 'library'; S.view = { type: 'inst', id: iid, game: g?.id || null }; S.sum = null; S.org = null; S.sel = null; S.q = ''; S.filt = 'all'; S.orgTab = 'mods';
  render();
  await loadOrg(); render(); KIT.focusFirst('[data-key="play"]');
}
async function loadOrg() { try { S.org = await org([S.view.id]); } catch (e) { S.org = null; KIT.toast(KIT.err(e)); } }
async function reloadView() {
  if (!S.view) return;
  if (S.view.type === 'inst') await loadOrg();
  else { try { S.sum = await KIT.invoke('mods_of', { id: S.view.id }); } catch (e) { KIT.toast(KIT.err(e)); } }
  render();
}
function applyResult(r) {
  S.sum = r.summary; S.conf = {};
  const g = game(S.view?.id); if (g) { g.mods = r.summary.mods.length; g.modsOn = r.summary.mods.filter(m => m.enabled).length; }
  const rep = r.report || {};
  render();
  if (rep.problems?.length) KIT.alert('Done, with problems', rep.problems.map(esc).join('<br>'), false);
  else if (rep.keptChanged?.length) KIT.alert('Done', `The game had changed ${rep.keptChanged.length} of the mod's files while it was on; those copies are kept in the mod folder (changed/).`);
}

// ---------------------------------------------------------------- the mod list, the same for both kinds of game
/** The rows to draw: Hearth Mods' own list or the organizer's, in one shape (lower in the list wins). */
function modsModel() {
  if (S.view?.type === 'inst') {
    if (!S.org) return [];
    if (S.orgTab === 'plugins') return [];
    let rank = 0;
    return S.org.mods.map((m, i) => ({ id: m.id, name: m.name, kind: m.kind, enabled: m.enabled, pos: i, prio: m.kind === 'separator' ? -1 : rank++, version: m.version || '', files: m.files, plugins: m.plugins || [],
      beats: m.conflicts?.winsOver || [], beatenBy: m.conflicts?.losesTo || [], losing: m.conflicts?.losing || 0, winning: m.conflicts?.winning || 0, nexusId: m.nexusId, from: m.installedFrom, hidden: m.hidden }));
  }
  return (S.sum?.mods || []).map((m, i) => ({ id: m.id, name: m.name, kind: 'mod', enabled: m.enabled, pos: i, prio: m.order, size: m.size, files: m.files,
    beats: m.enabled ? (m.beats || []) : [], beatenBy: m.enabled ? (m.overriddenBy || []) : [], losing: m.enabled ? Math.max(0, m.files - m.winning) : 0, winning: m.winning,
    target: m.targetLabel, sub: m.sub, root: m.root, added: m.added, source: m.source, nexus: m.nexus, note: m.note || '', version: m.nexus?.version || '' }));
}
const updatesFor = () => S.updates[S.view?.game || S.view?.id] || {};
function visibleMods(all) {
  const q = S.q.trim().toLowerCase(), up = updatesFor();
  return all.filter(m => {
    if (m.kind !== 'mod') return !q && S.filt === 'all';
    if (q && !m.name.toLowerCase().includes(q)) return false;
    if (S.filt === 'on') return m.enabled;
    if (S.filt === 'off') return !m.enabled;
    if (S.filt === 'conf') return m.beatenBy.length || m.beats.length;
    if (S.filt === 'upd') return !!up[m.id];
    return true;
  });
}
const canReorder = () => !S.q.trim() && S.filt === 'all' && !(S.view?.type === 'inst' && S.orgTab !== 'mods');
function health(all) {
  const mods = all.filter(m => m.kind === 'mod');
  const over = mods.filter(m => m.enabled && m.beatenBy.length && !m.winning).length;
  const part = mods.filter(m => m.enabled && m.beatenBy.length && m.winning).length;
  const missing = S.org ? S.org.plugins.filter(p => p.active && p.missing?.length).length : 0;
  if (missing) return { cls: 'bad', text: `${plural(missing, 'plugin')} missing masters` };
  if (over) return { cls: 'warn', text: `${plural(over, 'mod')} fully overridden` };
  if (part) return { cls: 'good', text: `Healthy, ${part} with overrides` };
  return { cls: 'good', text: mods.length ? 'Healthy' : 'Nothing to check yet' };
}

// ---------------------------------------------------------------- drawing
const scrollKeep = {};
function render() {
  const focusKey = document.activeElement?.dataset?.key;
  document.querySelectorAll('[data-scroll]').forEach(el => { scrollKeep[el.dataset.scroll] = el.scrollTop; });
  rail();
  const st = $('#stage');
  st.innerHTML = S.section === 'library' ? (S.view ? gameView() : libraryView())
    : S.section === 'modlists' ? modlistsView() : S.section === 'downloads' ? downloadsView()
    : S.section === 'organizer' ? (S.view?.type === 'inst' ? gameView() : organizerView())
    : S.section === 'accounts' ? accountsView() : settingsView();
  document.querySelectorAll('[data-scroll]').forEach(el => { if (scrollKeep[el.dataset.scroll] != null) el.scrollTop = scrollKeep[el.dataset.scroll]; });
  const s = $('#q'); if (s) s.addEventListener('input', e => { S.q = e.target.value; drawList(); });
  const w = $('#wjq'); if (w) w.addEventListener('input', e => { S.wjQ = e.target.value; const c = $('#lists'); if (c) c.innerHTML = galleryCards(); });
  footer();
  const back = focusKey && document.querySelector(`[data-key="${CSS.escape(focusKey)}"]`);
  if (back) back.focus({ preventScroll: true });
  else if (!KIT.dialogOpen() && (!document.activeElement || document.activeElement === document.body)) KIT.focusFirst('.sec.on');
}
function rail() {
  const active = S.dls.active || 0, jobs = S.jobs.filter(j => j.running).length;
  let h = `<div class="mark" title="Hearth Mods">${I.layers}</div><div class="bumper"><b>LB</b><b>RB</b></div>`;
  h += SECTIONS.map(([id, label]) => `<button class="sec f ${S.section === id ? 'on' : ''}" data-key="sec:${id}" data-sec="${id}">${I[id]}<span>${label}</span>${id === 'downloads' && active ? `<i class="count">${active}</i>` : id === 'modlists' && jobs ? `<i class="count">${jobs}</i>` : ''}</button>`).join('');
  if (S.section === 'library' && S.view) {
    const cur = S.view.game || S.view.id;
    h += `<div class="covers" data-scroll="covers">${S.games.map(g => { const i = instOf(g.id); const on = i ? i.modsOn : g.modsOn;
      return `<button class="cv f ${g.id === cur ? 'on' : ''}" data-key="cv:${esc(g.id)}" data-game="${esc(g.id)}" title="${esc(g.title)}" ${artAttr(g.id, 'cover')}>${g.art?.cover ? '' : `<span class="ini">${esc((g.title || '?')[0])}</span>`}${on ? `<i class="pip">${on}</i>` : ''}</button>`; }).join('')}</div>`;
  }
  $('#rail').innerHTML = h;
}
/** Only the rail's counts (the rest of the rail keeps its focus while downloads tick). */
function railCounts() {
  const n = { downloads: S.dls.active || 0, modlists: S.jobs.filter(j => j.running).length };
  for (const [id, v] of Object.entries(n)) {
    const b = document.querySelector(`.sec[data-sec="${id}"]`); if (!b) continue;
    let c = b.querySelector('.count');
    if (!v) { c?.remove(); continue; }
    if (!c) { c = document.createElement('i'); c.className = 'count'; b.appendChild(c); }
    c.textContent = v;
  }
}
function hint(k, t, cls = '') { return t ? `<span class="hint"><b class="${cls || (k.length > 1 ? 'wide' : 'btn' + k)}">${k}</b>${t}</span>` : ''; }
function footer() {
  const a = document.activeElement?.dataset || {};
  let h = [];
  if (S.grab) h = [hint('A', 'Drop here'), hint('↑↓', 'Move', 'wide'), hint('B', 'Cancel')];
  else if (a.mod) h = [hint('A', 'Pick up to move'), hint('X', 'On / off'), hint('Y', 'More'), hint('B', 'Back')];
  else if (a.plug) h = [hint('A', 'Options'), hint('X', 'On / off'), hint('B', 'Back')];
  else if (a.dl) h = [hint('A', 'Options'), hint('X', 'Pause / resume'), hint('Y', 'Add a Nexus link')];
  else if (S.section === 'library' && S.view) h = [hint('A', 'Choose'), hint('B', 'Library'), hint('Y', 'Add mods'), hint('☰', 'Play modded', 'wide')];
  else if (S.section === 'downloads') h = [hint('A', 'Choose'), hint('Y', 'Add a Nexus link')];
  else h = [hint('A', 'Choose'), hint('B', 'Back')];
  h.push(hint('LB RB', 'Sections'));
  const ctx = S.section === 'library' && S.view ? (S.org ? `${S.org.instance.title} › ${S.org.instance.profile}` : game(S.view.id)?.title || '') : '';
  $('#foot').innerHTML = h.join('') + (ctx ? `<span class="ctx">${esc(ctx)}</span>` : '');
}

// ---------------------------------------------------------------- the library
function libraryView() {
  if (S.loading) return `<div class="top"><div class="grow"><h1>Library</h1><p class="lede">Reading your games…</p></div></div><div class="body"><div class="prog spin" style="max-width:20em"><i></i></div></div>`;
  const moddable = S.games.filter(g => !g.beacon);
  const onOf = g => { const i = instOf(g.id); return i ? i.modsOn : g.modsOn || 0; };
  const totOf = g => { const i = instOf(g.id); return i ? i.mods : g.mods || 0; };
  const modded = moddable.filter(g => totOf(g) > 0), rest = moddable.filter(g => !totOf(g)), beacon = S.games.filter(g => g.beacon);
  const card = g => { const on = onOf(g), tot = totOf(g), i = instOf(g.id), k = wideOf(g);
    return `<button class="gcard f" data-key="g:${esc(g.id)}" data-game="${esc(g.id)}" ${artAttr(g.id, k)}>${g.art?.[k] ? '' : `<span class="ini">${esc((g.title || '?')[0])}</span>`}
      ${g.beacon ? '<span class="flag"><span class="pill calm">Beacon</span></span>' : i ? '<span class="flag"><span class="pill calm">Organizer</span></span>' : ''}
      <span class="gt"><b>${esc(g.title)}</b><span>${g.beacon ? 'Mods managed in Beacon' : tot ? `<span class="pill">${on} of ${tot} on</span>` : 'No mods yet'}<span>${esc(SRC[g.source] || g.source)}</span></span></span></button>`; };
  const sect = (title, list, note) => list.length ? `<div class="group-h">${title}<small>${note || list.length}</small></div><div class="shelf">${list.map(card).join('')}</div>` : '';
  const jobs = S.jobs.filter(j => j.running);
  return `<div class="top"><div class="grow"><h1>Library</h1><p class="lede">${S.games.length ? `${plural(moddable.length, 'game')} you can mod. Choose one to see its mods in priority order, turn them on and off, and play it modded.` : ''}</p></div>
      <div class="lib-acts"><button class="btn f" data-key="lib-wj" data-act="toModlists">${I.modlists}Install a modlist</button><button class="btn f" data-key="lib-dl" data-act="toDownloads">${I.downloads}Downloads${S.dls.active ? ` (${S.dls.active})` : ''}</button></div></div>
    <div class="body" data-scroll="lib">
      ${jobs.length ? `<div class="activity">${jobs.map(actCard).join('')}</div>` : ''}
      ${sect('Modded', modded)}${sect(modded.length ? 'Ready for mods' : 'Your games', rest)}${sect('In Beacon', beacon)}
      ${S.games.length ? '' : '<div class="empty">No installed games found. Install a game in Hearth\'s library, then come back.</div>'}
    </div>`;
}

// ---------------------------------------------------------------- a game
function gameView() {
  const inst = S.view.type === 'inst';
  const g = game(S.view.game || S.view.id);
  if (!inst && g?.beacon) return beaconView(g);
  const title = inst ? (S.org?.instance.title || S.insts.find(i => i.id === S.view.id)?.title || 'Organizer') : g?.title || '';
  const all = modsModel(), mods = all.filter(m => m.kind === 'mod');
  const on = mods.filter(m => m.enabled).length;
  const hl = health(all);
  const ready = inst ? !!S.org : !!S.sum;
  let read = '';
  if (ready) {
    const dep = inst ? S.org.deployed : null;
    const when_ = inst ? (dep ? `In the game now${dep.running ? ', playing' : ''}` : 'Laid over the game while it runs') : S.sum.deployedAt ? `Deployed ${esc(when(S.sum.deployedAt))}` : 'Not deployed yet';
    read = `<div class="readout"><span><b>${on}</b> of ${plural(mods.length, 'mod')} on</span><span><i class="dot ${hl.cls === 'good' ? 'good' : 'bad'}"></i>${esc(hl.text)}</span><span>${when_}${inst ? '' : ` as <button class="f" data-key="dm" data-act="deployMode">${esc(deployLabel(S.sum).toLowerCase())}</button>`}</span>
      ${inst ? `<span>Profile <button class="f" data-key="prof" data-oa="profile">${esc(S.org.instance.profile)}</button></span>` : ''}${Object.keys(updatesFor()).length ? `<span class="upd">${plural(Object.keys(updatesFor()).length, 'update')} on Nexus</span>` : ''}</div>`;
  } else read = '<div class="readout"><span>Reading its mods…</span></div>';
  const wj = g?.steamApp && WJ_STEAM[g.steamApp];
  const acts = ready ? (inst
    ? `<button class="btn primary big f" data-key="play" data-oa="play">${I.play}Play modded</button>
       <button class="btn big f" data-key="dep" data-oa="${S.org.deployed ? 'undeploy' : 'deploy'}">${I.deploy}${S.org.deployed ? 'Take mods out' : 'Deploy now'}</button>
       <button class="btn big f" data-key="add" data-oa="add">${I.plus}Add mods</button>
       ${S.view.game ? `<button class="btn big f" data-key="nx" data-act="nexusGet">${I.globe}Browse Nexus</button>` : ''}
       ${S.view.game ? `<button class="btn big f" data-key="steam" data-act="addToSteam">${I.steamadd}Add to Steam</button>` : ''}
       <button class="btn big f" data-key="more" data-act="more" aria-label="More">${I.more}</button>`
    : `<button class="btn primary big f" data-key="play" data-act="play">${I.play}Play modded</button>
       <button class="btn big f" data-key="dep" data-act="deployMode">${I.deploy}Deploy: ${esc(deployLabel(S.sum))}</button>
       <button class="btn big f" data-key="add" data-act="add">${I.plus}Add mods</button>
       <button class="btn big f" data-key="nx" data-act="nexusGet">${I.globe}Browse Nexus</button>
       ${wj ? `<button class="btn big f" data-key="wjg" data-act="gameModlists" data-wj="${esc(wj)}">${I.modlists}Install a modlist</button>` : ''}
       <button class="btn big f" data-key="steam" data-act="addToSteam">${I.steamadd}Add to Steam</button>
       <button class="btn big f" data-key="more" data-act="more" aria-label="More">${I.more}</button>`) : '';
  const hero = `<section class="hero"><div class="art" ${artAttr(g?.id, g?.art?.hero ? 'hero' : wideOf(g || {}))}></div>
    <div class="hero-in"> <div class="cover ${g?.art?.cover ? '' : 'noart'}" ${artAttr(g?.id, 'cover')}>${g?.art?.cover ? '' : esc((title || '?')[0])}</div>
      <div><button class="back f" data-key="back" data-act="backToLib">${I.back}${S.section === 'organizer' ? 'Organizer' : 'Library'}</button>
        <h1>${esc(title)}</h1>${read}<div class="acts">${acts}</div></div></div></section>`;
  if (!ready) return hero + `<div class="work solo"><div class="empty"><div class="prog spin" style="max-width:20em;margin:0 auto 1em"><i></i></div>Reading its mods…</div></div>`;
  return hero + `<div class="work" id="work"><div class="listcol">${toolsBar(all)}<div class="legend" id="legend">${legend(all)}</div><div class="ladder" id="ladder" data-scroll="ladder">${listRows(all)}</div></div><aside class="det" id="det">${details()}</aside></div>`;
}
function beaconView(g) {
  return `<section class="hero"><div class="art" ${artAttr(g.id, g.art?.hero ? 'hero' : wideOf(g))}></div><div class="hero-in"><div class="cover" ${artAttr(g.id, 'cover')}></div>
    <div><button class="back f" data-key="back" data-act="backToLib">${I.back}Library</button><h1>${esc(g.title)}</h1>
    <div class="readout"><span>Beacon manages this game and its mods (Battle for Middle-earth, Age of the Ring and the others).</span></div>
    <div class="acts"><button class="btn primary big f" data-key="beacon" data-act="beacon">${I.play}Open Beacon</button></div></div></div></section><div></div>`;
}
function toolsBar(all) {
  const inst = S.view.type === 'inst', mods = all.filter(m => m.kind === 'mod'), up = updatesFor();
  const n = { on: mods.filter(m => m.enabled).length, off: mods.filter(m => !m.enabled).length, conf: mods.filter(m => m.beatenBy.length || m.beats.length).length, upd: Object.keys(up).length };
  const seg = inst && S.org.instance.plugins ? `<div class="seg"><button class="f ${S.orgTab === 'mods' ? 'on' : ''}" data-key="ot:mods" data-ot="mods">Mods <i>${mods.length}</i></button><button class="f ${S.orgTab === 'plugins' ? 'on' : ''}" data-key="ot:plugins" data-ot="plugins">Plugins <i>${S.org.plugins.filter(p => p.active).length}/${S.org.plugins.length}</i></button></div>` : '';
  if (inst && S.orgTab === 'plugins') return `<div class="tools">${seg}<button class="btn f" data-key="osort" data-oa="sort">Sort plugins</button></div>`;
  const chip = (id, label, c) => `<button class="fchip f ${S.filt === id ? 'on' : ''}" data-key="ff:${id}" data-filt="${id}">${label}${c != null ? `<i>${c}</i>` : ''}</button>`;
  const nexusMods = !inst && mods.some(m => m.nexus);
  return `<div class="tools">${seg}<label class="search">${I.search}<input class="inp f" id="q" data-key="q" data-osk placeholder="Search mods" value="${esc(S.q)}" spellcheck="false" autocomplete="off"></label>
    <div class="filters">${chip('all', 'All', mods.length)}${chip('on', 'On', n.on)}${chip('off', 'Off', n.off)}${chip('conf', 'Conflicts', n.conf)}${n.upd ? chip('upd', 'Updates', n.upd) : ''}</div>
    ${nexusMods ? `<button class="btn f" data-key="cu" data-act="checkUpdates">${I.upd}Check for updates</button>` : ''}</div>`;
}
function legend(all) {
  if (S.view.type === 'inst' && S.orgTab === 'plugins') return '<span class="tip">Plugins load from the top; a plugin needs its masters above it.</span>';
  const m = all.find(x => x.id === S.sel && x.kind === 'mod');
  if (m && (m.beats.length || m.beatenBy.length)) return `<span class="k" style="--c:var(--ember)">Overridden by ${plural(m.beatenBy.length, 'mod')}</span><span class="k" style="--c:var(--lose)">Overrides ${plural(m.beats.length, 'mod')}</span>`;
  return `<span class="tip">Lower in the list wins when two mods have the same file.${canReorder() ? ' Drag a handle, or press A on a mod, to move it.' : ''}</span>`;
}
function rowHtml(m, up) {
  const flags = [];
  if (m.enabled && m.beats.length) flags.push(`<span class="flag over" title="Wins over ${esc(m.beats.join(', '))}">${I.over}${m.beats.length}</span>`);
  if (m.enabled && m.beatenBy.length) flags.push(`<span class="flag ${m.winning ? 'under' : 'bad'}" title="Loses ${m.losing || ''} files to ${esc(m.beatenBy.join(', '))}">${I.under}${m.winning ? m.beatenBy.length : 'All'}</span>`);
  if (up[m.id]) flags.push(`<span class="flag upd">${I.upd}Update</span>`);
  if (m.plugins?.length) flags.push(`<span class="flag calm">${plural(m.plugins.length, 'plugin')}</span>`);
  const subs = [];
  if (m.kind === 'overwrite') subs.push('What the game and tools wrote');
  else if (m.kind === 'unmanaged') subs.push('The game\'s own');
  else {
    if (m.version) subs.push(`<span>${esc(m.version)}</span>`);
    if (m.files != null) subs.push(`<span>${plural(m.files, 'file')}</span>`);
    if (m.size) subs.push(`<span class="sz">${esc(human(m.size))}</span>`);
    if (m.target && m.target !== 'Game folder') subs.push(`<span>${esc(m.target)}</span>`);
    if (m.note) subs.push(`<span>Note</span>`);
  }
  const lens = S.sel && S.sel !== m.id ? lensOf(m) : '';
  const movable = m.kind === 'mod' || (m.kind === 'separator');
  return `<button class="mrow f ${m.enabled ? '' : 'off'} ${S.sel === m.id ? 'sel' : ''} ${lens} ${S.grab?.id === m.id ? 'grab' : ''}" data-key="m:${esc(m.id)}" data-mod="${esc(m.id)}" data-pos="${m.pos}">
    <span class="grip" ${movable && canReorder() ? `data-grip="${esc(m.id)}"` : 'style="visibility:hidden"'} title="Drag to change its place">${I.grip}</span><span class="prio">${m.prio + 1}</span>
    <span class="tog ${m.enabled ? 'on' : ''}" ${m.kind === 'mod' ? `data-tog="${esc(m.id)}"` : 'style="visibility:hidden"'} role="switch" aria-checked="${m.enabled}"></span>
    <span class="mid"><span class="nm">${esc(m.name)}</span><span class="sub">${subs.join('')}</span></span><span class="end">${flags.join('')}</span></button>`;
}
function lensOf(m) {
  const s = modsModel().find(x => x.id === S.sel); if (!s) return '';
  if (s.beatenBy.includes(m.name)) return 'l-over';
  if (s.beats.includes(m.name)) return 'l-under';
  return '';
}
function listRows(all) {
  if (S.view.type === 'inst' && S.orgTab === 'plugins') return pluginRows();
  let list = visibleMods(all);
  if (S.grab) { const i = list.findIndex(m => m.id === S.grab.id); if (i >= 0) { const [m] = list.splice(i, 1); list.splice(S.grab.to, 0, m); let rank = 0; list = list.map(x => x.kind === 'mod' ? { ...x, prio: rank++ } : x); } }
  const up = updatesFor();
  let html = '';
  const seps = S.view.type === 'inst' && !S.q.trim() && S.filt === 'all';
  for (let i = 0; i < list.length; i++) {
    const m = list[i];
    if (m.kind === 'separator') {
      if (!seps) continue;
      let n = 0; for (let j = i + 1; j < list.length && list[j].kind !== 'separator'; j++) if (list[j].kind === 'mod') n++;
      html += `<div class="msep" data-pos="${m.pos}" data-sepid="${esc(m.id)}">${esc(m.name)}<small>${n}</small></div>`;
      continue;
    }
    html += rowHtml(m, up);
  }
  if (!list.some(m => m.kind !== 'separator')) html += S.q || S.filt !== 'all' ? '<div class="empty">No mods match.</div>'
    : `<div class="empty">No mods yet. <b>Add mods</b> from a .zip, .7z or .rar file or a folder, or get them from Nexus Mods.</div>`;
  return html;
}
function pluginRows() {
  const P = S.org.plugins;
  if (!P.length) return '<div class="empty">No plugins.</div>';
  return P.map(p => `<button class="mrow plug f ${p.active ? '' : 'off'} ${S.sel === 'p:' + p.name ? 'sel' : ''}" data-key="op:${esc(p.name)}" data-plug="${esc(p.name)}">
    <span class="grip" style="visibility:hidden"></span><span class="prio">${esc(p.loadIndex || '–')}</span><span class="tog ${p.active ? 'on' : ''}" ${p.implicit ? 'style="visibility:hidden"' : `data-ptog="${esc(p.name)}"`}></span>
    <span class="mid"><span class="nm">${esc(p.name)}</span><span class="sub"><span>${esc(p.mod || 'The game')}</span>${p.implicit ? '<span>Always loaded</span>' : ''}</span></span>
    <span class="end">${p.missing?.length ? `<span class="flag bad">${I.warn}Needs ${esc(p.missing.map(x => x.master).join(', '))}</span>` : ''}${p.light ? '<span class="flag calm">Light</span>' : p.master ? '<span class="flag calm">Master</span>' : ''}</span></button>`).join('');
}
/** Redraws only the list, the legend and the details (typing in the search box keeps its focus). */
function drawList() {
  const all = modsModel();
  const l = $('#ladder'); if (l) l.innerHTML = listRows(all);
  const g = $('#legend'); if (g) g.innerHTML = legend(all);
  document.querySelectorAll('.fchip').forEach(c => c.classList.toggle('on', c.dataset.filt === S.filt));
}
function drawDetails() { const d = $('#det'); if (d) d.innerHTML = details(); const g = $('#legend'); if (g) g.innerHTML = legend(modsModel());
  document.querySelectorAll('.mrow[data-mod]').forEach(r => { const m = modsModel().find(x => x.id === r.dataset.mod); if (!m) return; r.classList.toggle('sel', S.sel === m.id); r.classList.remove('l-over', 'l-under'); const l = S.sel && S.sel !== m.id ? lensOf(m) : ''; if (l) r.classList.add(l); });
  document.querySelectorAll('.mrow[data-plug]').forEach(r => r.classList.toggle('sel', S.sel === 'p:' + r.dataset.plug)); }

// ---------------------------------------------------------------- the details panel
function details() {
  const inst = S.view.type === 'inst';
  if (S.sel?.startsWith('p:')) return pluginDetails(S.sel.slice(2));
  const m = modsModel().find(x => x.id === S.sel);
  if (!m) {
    const all = modsModel().filter(x => x.kind === 'mod');
    const conf = all.filter(x => x.enabled && x.beatenBy.length);
    const warn = inst ? S.org.warnings : [];
    return `<div class="det-h"><h2>${inst ? 'Hearth Mod Organizer' : 'Load order'}</h2><div class="sub">${inst ? `Profile ${esc(S.org.instance.profile)}` : `Deploy: ${esc(deployLabel(S.sum))}`}</div></div><div class="dtabs"></div>
      <div class="dbody"><div class="stats"><span><b>${all.filter(x => x.enabled).length}</b>on</span><span><b>${all.length}</b>installed</span><span><b>${conf.length}</b>overridden</span></div>
      ${warn.map(w => `<p class="warn">${esc(w)}</p>`).join('')}
      ${inst ? '<p class="msg">The game\'s folder is never changed: the mods are laid over it while you play. Profiles keep their own mod list, and can keep their own saves and game settings.</p>'
        : `<p class="msg">Choose a mod to see what it is, which of its files win and lose, and your notes. ${S.sum?.overlay ? `The overlay is ${S.sum.overlay.mounted ? 'on' : 'off'}: the game folder isn't changed, what the game writes there is kept apart (${esc(human(S.sum.overlay.changed))}).` : 'Turning a mod off puts the game\'s own files back.'}</p>`}
      ${conf.length ? `<p class="msg small">Overridden: ${conf.slice(0, 8).map(x => `<button class="f" style="text-decoration:underline" data-key="jump:${esc(x.id)}" data-jump="${esc(x.id)}">${esc(x.name)}</button>`).join(', ')}</p>` : ''}</div>
      <div class="dacts"><button class="btn f" data-key="d-add" ${inst ? 'data-oa="add"' : 'data-act="add"'}>${I.plus}Add mods</button><button class="btn f" data-key="d-folder" data-act="${inst ? 'orgFolder' : 'openStore'}">${I.folder}Open mod files</button></div>`;
  }
  const up = updatesFor()[m.id];
  const tabs = [['about', 'About'], ['conf', `Conflicts${m.beats.length + m.beatenBy.length ? ` (${m.beats.length + m.beatenBy.length})` : ''}`]];
  if (!inst) tabs.push(['notes', 'Notes']);
  if (!tabs.find(t => t[0] === S.detTab)) S.detTab = 'about';
  let body = '';
  if (S.detTab === 'about') body = aboutTab(m, up);
  else if (S.detTab === 'conf') body = conflictsTab(m);
  else body = m.note ? `<div class="note">${esc(m.note)}</div><button class="btn f" data-key="d-note" data-act="note">Edit the note</button>` : `<p class="msg">Your own notes about this mod: why it's here, what it needs, what to watch for.</p><button class="btn f" data-key="d-note" data-act="note">Write a note</button>`;
  const acts = m.kind === 'mod' ? `<button class="btn f ${m.enabled ? '' : 'primary'}" data-key="d-tog" data-act="toggleSel">${m.enabled ? 'Turn off' : 'Turn on'}</button>
      ${canReorder() ? `<button class="btn f" data-key="d-up" data-act="moveSel" data-by="-1" aria-label="Lower priority">${I.up}</button><button class="btn f" data-key="d-down" data-act="moveSel" data-by="1" aria-label="Higher priority">${I.down}</button>` : ''}
      <button class="btn f" data-key="d-open" data-act="openSel">${I.folder}Files</button>${up ? `<button class="btn f" data-key="d-upd" data-act="getUpdate">${I.upd}Get update</button>` : ''}<button class="btn danger f" data-key="d-rm" data-act="removeSel" aria-label="Remove">${I.trash}</button>`
    : m.kind === 'overwrite' ? `<button class="btn f" data-key="d-open" data-act="openSel">${I.folder}Open the folder</button>` : '';
  return `<div class="det-h"><h2>${esc(m.name)}</h2><div class="sub">${m.kind === 'mod' ? `<span class="pill ${m.enabled ? 'good' : 'calm'}">${m.enabled ? 'On' : 'Off'}</span>` : ''}<span>Priority ${m.prio + 1}</span>${m.version ? `<span>Version ${esc(m.version)}</span>` : ''}${up ? '<span class="pill good">Update on Nexus</span>' : ''}</div></div>
    <div class="dtabs">${tabs.map(([id, l]) => `<button class="dtab f ${S.detTab === id ? 'on' : ''}" data-key="dt:${id}" data-dtab="${id}">${l}</button>`).join('')}</div>
    <div class="dbody" data-scroll="dbody">${body}</div><div class="dacts">${acts}</div>`;
}
function aboutTab(m, up) {
  const nx = m.nexus ? nexusInfo(m.nexus.domain, m.nexus.modId) : (S.view.type === 'inst' && m.nexusId && S.org.instance.game ? null : null);
  let h = '';
  if (nx && nx !== 'loading' && !nx.error) {
    h += `${nx.picture ? `<div class="nx-pic" style="background-image:url('${esc(nx.picture)}')"></div>` : ''}<div class="stats"><span><b>${Number(nx.endorsements || 0).toLocaleString()}</b>endorsements</span><span><b>${Number(nx.downloads || 0).toLocaleString()}</b>downloads</span>${nx.version ? `<span><b>${esc(nx.version)}</b>on Nexus</span>` : ''}</div>
      ${nx.summary ? `<p>${esc(nx.summary)}</p>` : ''}${nx.author ? `<p class="msg small">By ${esc(nx.author)}</p>` : ''}`;
  } else if (nx === 'loading') h += '<p class="msg small">Reading its Nexus Mods page…</p>';
  else if (nx?.error) h += `<p class="msg small">${esc(nx.error)}</p>`;
  if (up) h += `<p class="ok small">A newer file is on Nexus Mods: ${esc(up.newer?.name || '')} ${esc(up.newer?.version || '')}</p>`;
  const kv = [];
  if (m.files != null) kv.push(['Files', `${m.files}${m.size ? `, ${esc(human(m.size))}` : ''}`]);
  if (m.target) kv.push(['Goes into', `${esc(m.target)}${m.sub ? ` › ${esc(m.sub)}` : ''}`]);
  if (m.root) kv.push(['Folder', `<span class="path">${esc(m.root)}</span>`]);
  if (m.added) kv.push(['Added', esc(when(m.added))]);
  if (m.plugins?.length) kv.push(['Plugins', esc(m.plugins.join(', '))]);
  if (m.hidden) kv.push(['Hidden files', String(m.hidden)]);
  if (m.source || m.from) kv.push(['From', `<span class="path">${esc(m.source || m.from)}</span>`]);
  if (m.nexus) kv.push(['Nexus Mods', `<button class="f" style="text-decoration:underline" data-key="d-nxp" data-act="nexusPage">Open its page</button>`]);
  h += `<dl class="kv">${kv.map(([k, v]) => `<dt>${k}</dt><dd>${v}</dd>`).join('')}</dl>`;
  if (m.note && S.detTab === 'about') h += `<div class="note">${esc(m.note)}</div>`;
  return h;
}
function nexusInfo(domain, modId) {
  if (!S.acct) return null;
  const k = `${domain}/${modId}`;
  if (k in S.nx) return S.nx[k];
  S.nx[k] = 'loading';
  KIT.invoke('nexus_mod', { domain, modId }).then(v => { S.nx[k] = v; }).catch(e => { S.nx[k] = { error: KIT.err(e) }; }).finally(() => { if (S.sel) drawDetails(); });
  return 'loading';
}
function conflictsTab(m) {
  if (m.kind !== 'mod') return '<p class="msg">Nothing to compare.</p>';
  const who = (list, cls) => `<div class="who">${list.map(n => { const t = modsModel().find(x => x.name === n); return `<button class="f" data-key="cw:${esc(n)}" ${t ? `data-jump="${esc(t.id)}"` : ''}>${esc(n)}</button>`; }).join('')}</div>`;
  let h = `<div class="stats"><span><b>${m.enabled ? m.winning ?? '–' : '–'}</b>files win</span><span><b>${m.enabled ? m.losing : '–'}</b>files lose</span></div>`;
  if (!m.enabled) h += '<p class="msg small">It\'s off: here is what would happen if it were on.</p>';
  if (m.beats.length) h += `<div class="small" style="font-weight:650">Wins over</div>${who(m.beats)}`;
  if (m.beatenBy.length) h += `<div class="small" style="font-weight:650">Loses to</div>${who(m.beatenBy)}`;
  const key = `${S.view.id}|${m.id}|${m.prio}|${m.enabled}`;
  const c = S.conf[key];
  if (c === undefined) {
    S.conf[key] = null;
    const p = S.view.type === 'inst' ? org([S.view.id, 'conflicts', m.id]).then(r => ({ files: (r.files || []).filter(x => x.status !== 'alone') })) : KIT.invoke('mod_conflicts', { id: S.view.id, m: m.id });
    p.then(r => { S.conf[key] = r; }).catch(e => { S.conf[key] = { error: KIT.err(e) }; }).finally(() => { if (S.sel === m.id && S.detTab === 'conf') drawDetails(); });
  }
  if (!c) return h + '<p class="msg small">Reading its files…</p>';
  if (c.error) return h + `<p class="warn small">${esc(c.error)}</p>`;
  if (!c.files.length) return h + '<p class="msg">None of its files are in another mod.</p>';
  return h + `<div class="files">${c.files.slice(0, 400).map(x => `<div class="file"><span class="flag ${x.status === 'wins' ? 'over' : x.status === 'loses' ? 'under' : 'calm'}">${x.status === 'wins' ? I.over : x.status === 'loses' ? I.under : ''}${x.status === 'wins' ? 'Wins' : x.status === 'loses' ? 'Loses' : 'Hidden'}</span><span class="path">${esc(x.path)}</span><small>${x.status === 'wins' ? `over ${esc((x.over || []).join(', '))}` : x.status === 'loses' ? `to ${esc(x.to)}` : ''}</small></div>`).join('')}${c.files.length > 400 ? `<p class="msg small">And ${c.files.length - 400} more.</p>` : ''}</div>`;
}
function pluginDetails(name) {
  const p = S.org.plugins.find(x => x.name === name); if (!p) return '';
  return `<div class="det-h"><h2>${esc(p.name)}</h2><div class="sub"><span class="pill ${p.active ? 'good' : 'calm'}">${p.active ? 'Loaded' : 'Off'}</span>${p.loadIndex ? `<span>Load order ${esc(p.loadIndex)}</span>` : ''}${p.light ? '<span>Light</span>' : p.master ? '<span>Master</span>' : ''}</div></div><div class="dtabs"></div>
    <div class="dbody"><dl class="kv"><dt>From</dt><dd>${esc(p.mod || 'The game')}</dd>${p.author ? `<dt>By</dt><dd>${esc(p.author)}</dd>` : ''}<dt>Needs</dt><dd>${p.masters?.length ? esc(p.masters.join(', ')) : 'Nothing'}</dd></dl>
    ${p.missing?.length ? `<p class="warn">Missing or loading after it: ${esc(p.missing.map(x => x.master).join(', '))}</p>` : ''}${p.implicit ? '<p class="msg">The game always loads it.</p>' : ''}</div>
    <div class="dacts">${p.implicit ? '' : `<button class="btn f" data-key="pd-t" data-pa="toggle">${p.active ? 'Turn off' : 'Turn on'}</button><button class="btn f" data-key="pd-u" data-pa="up">${I.up}Earlier</button><button class="btn f" data-key="pd-d" data-pa="down">${I.down}Later</button>`}</div>`;
}

// ---------------------------------------------------------------- choosing things
const views = {};
function setSection(id, focus = true) {
  S.grab = null; S.sel = null;
  if (id === S.section && S.view) S.view = null;   // the section again: back to its start
  else if (id !== S.section) { views[S.section] = S.view; S.view = views[id] || null; }
  S.section = id;
  render();
  if (focus) KIT.focusFirst(`.sec[data-sec="${id}"]`);
  if (S.view) reloadView();
  if (id === 'downloads') refreshDownloads();
  if (id === 'accounts') { if (S.acct === undefined) loadAccount(); loadAccounts(); }
  if (id === 'modlists' || id === 'settings') loadModlists();
  if (id === 'organizer') loadInsts().then(() => { if (S.section === 'organizer' && !S.view) render(); });
}
const focusedMod = () => { const id = document.activeElement?.dataset?.mod; return id ? modsModel().find(m => m.id === id) : null; };
function select(id) { if (S.sel === id) return; S.sel = id; drawDetails(); }

document.addEventListener('focusin', e => {
  const d = e.target.dataset || {};
  if (!S.grab && d.mod) select(d.mod);
  else if (d.plug) select('p:' + d.plug);
  footer();
});
document.addEventListener('click', e => {
  const b = e.target.closest('button'); if (!b || b.disabled || b.closest('#kit-layer') || KIT.dialogOpen()) return;
  if (b.dataset.sec) return setSection(b.dataset.sec);
  if (b.dataset.game) return openGame(b.dataset.game, KIT.usingPad() || e.detail === 0);
  if (b.dataset.oi) return openInst(b.dataset.oi);
  if (b.dataset.filt) { S.filt = b.dataset.filt; drawList(); return; }
  if (b.dataset.ot) { S.orgTab = b.dataset.ot; S.sel = null; return render(); }
  if (b.dataset.dtab) { S.detTab = b.dataset.dtab; drawDetails(); KIT.focusFirst(`[data-dtab="${b.dataset.dtab}"]`); return; }
  if (b.dataset.jump) { S.sel = b.dataset.jump; S.filt = 'all'; S.q = ''; render(); const r = document.querySelector(`[data-mod="${CSS.escape(b.dataset.jump)}"]`); r?.scrollIntoView({ block: 'center' }); return; }
  if (b.dataset.mod) {
    const id = b.dataset.mod;
    if (S.grab) return dropGrab();
    if (e.target.closest('[data-tog]')) { const m = modsModel().find(x => x.id === id); return toggleMod(m, !m.enabled); }
    if (e.target.closest('[data-grip]')) return;
    if (S.sel !== id) { select(id); if (e.detail !== 0) return; }
    const m = modsModel().find(x => x.id === id);
    if (e.detail === 0 && m?.kind === 'mod' && canReorder()) return startGrab(id);   // A on a mod (or Enter): pick it up
    if (e.detail === 0 && m) return modMenu(m);
    return;
  }
  if (b.dataset.plug) {
    if (e.target.closest('[data-ptog]')) return orgPlugin(b.dataset.plug, 'toggle');
    select('p:' + b.dataset.plug); if (e.detail === 0) return orgPlugin(b.dataset.plug); return;
  }
  if (b.dataset.pa) { const name = S.sel?.slice(2); if (name) return orgPlugin(name, b.dataset.pa); return; }
  if (b.dataset.oa) return orgAction(b.dataset.oa);
  if (b.dataset.dl && S.section === 'downloads' && !b.dataset.act) return downloadActions(b.dataset.dl);
  if (b.dataset.ml) { const m = S.gallery.find(x => x.id === b.dataset.ml); if (m) return m.down ? KIT.alert('This list is down for now', 'Its author or Wabbajack marked it as broken. Try again later.', false) : wjInstall(m); }
  if (b.dataset.act) return act(b.dataset.act, b);
});
KIT.onBack = () => {
  if (S.grab) return cancelGrab();
  if (S.section === 'library' && S.view) {
    const a = document.activeElement;
    if (a?.closest('#det')) { const r = document.querySelector('.mrow.sel'); if (r) return r.focus(); }
    if (a?.dataset?.mod || a?.dataset?.plug || a?.closest('.tools')) return KIT.focusFirst('[data-key="play"]');
    S.view = null; S.sel = null; render(); return KIT.focusFirst(`[data-key="g:${CSS.escape(S.games[0]?.id || '')}"]`) || KIT.focusFirst();
  }
  if (S.section === 'organizer' && S.view) { S.view = null; render(); return KIT.focusFirst('.sec.on'); }
  KIT.focusFirst(`.sec[data-sec="${S.section}"]`);
};
KIT.onX = () => {
  if (KIT.dialogOpen() && document.querySelector('[data-g="file"]')) return document.querySelector('[data-g="file"]').click();
  if (S.grab) return;
  const d = document.activeElement?.dataset || {};
  if (d.mod) { const m = modsModel().find(x => x.id === d.mod); if (m?.kind === 'mod') toggleMod(m, !m.enabled); return; }
  if (d.plug) return orgPlugin(d.plug, 'toggle');
  if (S.section === 'downloads' && d.dl) { const x = S.dls.items.find(i => i.id === d.dl); if (x) act(['downloading', 'queued'].includes(x.state) ? 'dlPause' : 'dlResume', document.activeElement); }
};
KIT.onY = () => {
  if (KIT.dialogOpen() && document.querySelector('[data-g="skip"]')) return document.querySelector('[data-g="skip"]').click();
  if (S.grab) return;
  if (S.section === 'downloads') return pasteLink();
  const m = focusedMod(); if (m) return modMenu(m);
  if (S.view?.type === 'inst') return orgAction('add');
  if (S.view && !game(S.view.id)?.beacon) return addMod();
};
KIT.onStart = () => { if (S.view && !S.grab) document.querySelector('[data-key="play"]')?.click(); };
KIT.onTab = d => { const ids = SECTIONS.map(s => s[0]); setSection(ids[(ids.indexOf(S.section) + d + ids.length) % ids.length]); };

// Moving a mod with the controller or keys: A picks it up, up and down move it, A drops it, B puts it back.
const baseMove = KIT.move;
KIT.move = dir => {
  if (S.grab && !KIT.dialogOpen()) {
    if (dir === 'up' || dir === 'down') {
      const n = visibleMods(modsModel()).length;
      S.grab.to = Math.max(0, Math.min(n - 1, S.grab.to + (dir === 'up' ? -1 : 1)));
      drawList(); const r = document.querySelector(`[data-mod="${CSS.escape(S.grab.id)}"]`); r?.focus({ preventScroll: true }); r?.scrollIntoView({ block: 'nearest' });
    }
    return;
  }
  baseMove(dir);
};
function startGrab(id) { const list = visibleMods(modsModel()); const i = list.findIndex(m => m.id === id); if (i < 0) return; S.grab = { id, from: i, to: i }; drawList(); document.querySelector(`[data-mod="${CSS.escape(id)}"]`)?.focus({ preventScroll: true }); footer(); }
function cancelGrab() { const id = S.grab.id; S.grab = null; drawList(); document.querySelector(`[data-mod="${CSS.escape(id)}"]`)?.focus({ preventScroll: true }); footer(); }
async function dropGrab() {
  const g = S.grab; S.grab = null;
  if (g.to === g.from) { drawList(); document.querySelector(`[data-mod="${CSS.escape(g.id)}"]`)?.focus({ preventScroll: true }); footer(); return; }
  await moveMod(g.id, g.to - g.from);
  footer();
}
// Dragging a mod by its handle with the mouse or a finger.
let drag = null;
document.addEventListener('pointerdown', e => {
  const h = e.target.closest('[data-grip]'); if (!h || !canReorder() || KIT.dialogOpen()) return;
  e.preventDefault();
  const row = h.closest('.mrow'); row.classList.add('drag');
  drag = { id: h.dataset.grip, from: +row.dataset.pos, to: +row.dataset.pos, row };
  try { h.setPointerCapture(e.pointerId); } catch (er) {}
});
document.addEventListener('pointermove', e => {
  if (!drag) return;
  const rows = [...document.querySelectorAll('#ladder [data-pos]')];
  document.querySelectorAll('.drop-before,.drop-after').forEach(r => r.classList.remove('drop-before', 'drop-after'));
  let target = null;
  for (const r of rows) { const b = r.getBoundingClientRect(); if (e.clientY >= b.top - 3 && e.clientY <= b.bottom + 3) { target = r; break; } }
  if (!target) { const first = rows[0]?.getBoundingClientRect(), last = rows[rows.length - 1]?.getBoundingClientRect(); if (first && e.clientY < first.top) target = rows[0]; else if (last && e.clientY > last.bottom) target = rows[rows.length - 1]; }
  if (!target) return;
  drag.to = +target.dataset.pos;
  if (drag.to !== drag.from) target.classList.add(drag.to < drag.from ? 'drop-before' : 'drop-after');
  const l = $('#ladder'), lb = l.getBoundingClientRect();
  if (e.clientY < lb.top + 30) l.scrollTop -= 12; else if (e.clientY > lb.bottom - 30) l.scrollTop += 12;
});
document.addEventListener('pointerup', () => {
  if (!drag) return;
  const d = drag; drag = null;
  d.row.classList.remove('drag');
  document.querySelectorAll('.drop-before,.drop-after').forEach(r => r.classList.remove('drop-before', 'drop-after'));
  if (d.to !== d.from) moveMod(d.id, d.to - d.from);
});

// ---------------------------------------------------------------- changing mods (Hearth Mods' list or the organizer's)
async function toggleMod(m, on) {
  if (!m || m.kind !== 'mod') return;
  if (S.view.type === 'inst') return orgDo([on ? 'enable' : 'disable', m.id], `${m.name}: ${on ? 'on' : 'off'}`);
  try { applyResult(await KIT.invoke('toggle', { id: S.view.id, m: m.id, on })); KIT.toast(`${m.name}: ${on ? 'on' : 'off'}`); } catch (e) { KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
}
async function moveMod(id, by) {
  try {
    if (S.view.type === 'inst') await orgDo(['move', id, '--by', String(by)]);
    else applyResult(await KIT.invoke('move_mod', { id: S.view.id, m: id, by }));
  } catch (e) { KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
  S.sel = id; drawDetails();
  const r = document.querySelector(`[data-mod="${CSS.escape(id)}"]`); r?.focus({ preventScroll: true }); r?.scrollIntoView({ block: 'nearest' });
}
async function removeMod(m) {
  const inst = S.view.type === 'inst';
  if (!await KIT.confirm({ title: `Remove ${m.name}?`, text: inst ? '<p>Its folder in the organizer is deleted. The file you installed it from isn\'t touched.</p>' : '<p>Its files come out of the game (the game\'s own files go back), and Hearth\'s copy of the mod is deleted. The file you installed it from isn\'t touched.</p>', ok: 'Remove', danger: true })) return;
  S.sel = null;
  if (inst) return orgDo(['remove', m.id], `${m.name} removed`);
  try { applyResult(await KIT.invoke('remove_mod', { id: S.view.id, m: m.id })); KIT.toast(`${m.name} removed`); } catch (e) { KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
}
function openModFiles(m) {
  if (S.view.type === 'inst') { const i = S.org.instance; return KIT.invoke('open_folder', { path: m.kind === 'overwrite' ? i.overwriteDir : `${i.modsDir}/${m.id}` }).catch(e => KIT.toast(KIT.err(e))); }
  return KIT.invoke('open_folder', { path: `${S.sum.folder}/files/${m.id}` }).catch(e => KIT.toast(KIT.err(e)));
}
/** Y on a mod: everything you can do with it. */
async function modMenu(m) {
  if (m.kind === 'overwrite') return openModFiles(m);
  if (m.kind !== 'mod') return;
  const inst = S.view.type === 'inst', up = updatesFor()[m.id];
  const items = [{ id: 'toggle', label: m.enabled ? 'Turn off' : 'Turn on', sub: inst ? '' : m.enabled ? 'Takes its files out and puts the game\'s own back' : 'Puts its files in the game' }];
  if (canReorder()) items.push({ id: 'move', label: 'Move it…', sub: 'Up and down to choose its place, A to drop it' });
  items.push({ id: 'conf', label: 'Its conflicts', sub: m.beats.length + m.beatenBy.length ? `Wins over ${m.beats.length}, loses to ${m.beatenBy.length}` : 'None' });
  if (!inst) items.push({ id: 'note', label: m.note ? 'Edit the note' : 'Write a note' });
  if (up) items.push({ id: 'update', label: 'Get the update', sub: `${up.newer?.name || ''} ${up.newer?.version || ''}` });
  items.push({ id: 'open', label: 'Open its files' });
  if (!inst) items.push({ id: 'where', label: 'Open where it goes' });
  items.push({ id: 'remove', label: 'Remove…', sub: inst ? 'Deletes the mod\'s folder' : 'Turns it off and deletes Hearth\'s copy of it' });
  const a = await KIT.choose(m.name, items, m.files != null ? `${plural(m.files, 'file')}${m.size ? `, ${human(m.size)}` : ''}${m.added ? `, added ${when(m.added)}` : ''}` : '');
  if (a === 'toggle') return toggleMod(m, !m.enabled);
  if (a === 'move') { setTimeout(() => { document.querySelector(`[data-mod="${CSS.escape(m.id)}"]`)?.focus(); startGrab(m.id); }, 0); return; }
  if (a === 'conf') { S.sel = m.id; S.detTab = 'conf'; drawDetails(); return setTimeout(() => KIT.focusFirst('[data-dtab="conf"]'), 0); }
  if (a === 'note') return editNote(m);
  if (a === 'update') return getUpdate(m);
  if (a === 'open') return openModFiles(m);
  if (a === 'where') return m.root ? KIT.invoke('open_folder', { path: m.root }) : KIT.toast('That folder isn\'t there yet.');
  if (a === 'remove') return removeMod(m);
}
async function editNote(m) {
  const v = await ask(`A note on ${m.name}`, 'Only for you: why it\'s here, what it needs.', m.note || '');
  if (v == null) return;
  try { S.sum = await KIT.invoke('set_note', { id: S.view.id, m: m.id, note: v }); render(); KIT.toast(v.trim() ? 'Note saved' : 'Note removed'); } catch (e) { KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
}
async function checkUpdates() {
  const gid = S.view.id;
  KIT.progress('Checking Nexus Mods', 'Looking for newer files of your mods');
  try {
    const r = await KIT.invoke('check_updates', { id: gid });
    KIT.close();
    S.updates[gid] = Object.fromEntries(r.updates.map(u => [u.mod, u]));
    render();
    KIT.toast(r.updates.length ? `${plural(r.updates.length, 'update')} on Nexus Mods` : `All ${plural(r.checked, 'Nexus mod')} are up to date`);
  } catch (e) { KIT.close(); KIT.alert('Couldn\'t check', esc(KIT.err(e)), false); }
}
async function getUpdate(m) {
  const u = updatesFor()[m.id]; if (!u) return;
  if (S.acct?.premium) {
    try { await KIT.invoke('download_file', { domain: u.domain, modId: u.modId, fileId: u.newer.fileId, game: S.view.id }); KIT.toast('Downloading the update: see Downloads'); refreshDownloads(); } catch (e) { KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
  } else openUrl(u.page);
}

// ---------------------------------------------------------------- the other sections
function organizerView() {
  const L = S.insts;
  const row = i => { const g = game(i.libraryId) || S.games.find(x => i.gameDir && x.install === i.gameDir);
    return `<button class="gcard f" data-key="oi:${esc(i.id)}" data-oi="${esc(i.id)}" ${artAttr(g?.id, g ? wideOf(g) : 'wide')}>${g?.art ? '' : `<span class="ini">${esc((i.title || '?')[0])}</span>`}
      <span class="flag">${i.deployed ? '<span class="pill">In the game</span>' : i.wabbajack ? '<span class="pill calm">Wabbajack</span>' : ''}</span>
      <span class="gt"><b>${esc(i.title)}</b><span>${i.error ? `<span class="warn">${esc(i.error)}</span>` : `<span class="pill">${i.modsOn || 0} of ${i.mods || 0} on</span><span>${esc(i.gameName || '')}${i.profile ? `, ${esc(i.profile)}` : ''}</span>`}</span></span></button>`; };
  return `<div class="top"><div class="grow"><h1>Hearth Mod Organizer</h1><p class="lede">Hearth's own organizer, made to work like Mod Organizer 2 and with its files: profiles, priorities, conflicts, plugins and FOMOD installers. The game's folder is never changed: the mods are laid over it while you play.</p></div>
      <div class="lib-acts"><button class="btn primary f" data-key="onew" data-oa="new">${I.plus}New for a game</button><button class="btn f" data-key="oimp" data-oa="import">${I.folder}Use a Mod Organizer 2 folder</button></div></div>
    <div class="body" data-scroll="org">${L.length ? `<div class="shelf">${L.map(row).join('')}</div>` : '<div class="empty">No organizer instances yet. Make one for a game, or use a Mod Organizer 2 folder you already have. Wabbajack modlists get their own.</div>'}</div>`;
}
const DL_STATE = { queued: 'Waiting', downloading: 'Downloading', paused: 'Paused', done: 'Downloaded', failed: 'Failed', 'needs-link': 'Needs “Mod Manager Download”' };
function actCard(j) {
  const pct = j.count ? j.count[0] / j.count[1] * 100 : null;
  return `<button class="act f" data-key="ac:${esc(j.id)}" data-act="toModlists"><b>${esc(j.title)}</b><small>${esc(JOB_STATE[j.state] || j.state)}${j.step ? `, step ${j.step[0]} of ${j.step[1]}` : ''}${j.count ? `, ${j.count[0]} of ${j.count[1]}` : ''}</small><div class="prog ${pct == null ? 'spin' : ''}"><i ${pct != null ? `style="width:${pct.toFixed(1)}%"` : ''}></i></div></button>`;
}
function dlRow(x) {
  const pct = x.size ? Math.min(100, x.done / x.size * 100) : 0;
  const live = ['downloading', 'queued', 'paused'].includes(x.state), need = x.state === 'needs-link' || x.state === 'failed';
  const ic = x.state === 'done' ? I.check : need ? I.warn : x.state === 'paused' ? I.pause : I.downloads;
  const jt = x.job && S.jobs.find(j => j.id === x.job)?.title;
  return `<button class="dl2 f ${x.state === 'downloading' ? 'live' : need ? 'need' : x.state === 'done' ? 'fin' : ''}" data-key="d:${esc(x.id)}" data-dl="${esc(x.id)}"><span class="ic">${ic}</span><span class="n">${esc(x.name)}</span>
    <span class="end"><span class="pill ${x.state === 'done' ? 'good' : need ? 'warn' : x.state === 'paused' ? 'calm' : ''}">${esc(DL_STATE[x.state] || x.state)}</span><span>${x.state === 'downloading' ? `${human(x.speed)}/s` : esc(human(x.size))}</span></span>
    <span class="m"><span>${esc(x.fileName)}</span>${x.state === 'downloading' ? `<span>${human(x.done)} of ${human(x.size)}</span>` : ''}${jt ? `<span>For ${esc(jt)}</span>` : x.job ? '<span>For a modlist</span>' : ''}${x.error ? `<span class="warn">${esc(x.error)}</span>` : ''}</span>
    ${live ? `<div class="prog"><i style="width:${pct.toFixed(1)}%"></i></div>` : ''}</button>`;
}
function downloadsView() {
  const d = S.dls, q = d.quota || {};
  const need = d.items.filter(x => x.state === 'needs-link' || x.state === 'failed'), live = d.items.filter(x => ['downloading', 'queued', 'paused'].includes(x.state)), done = d.items.filter(x => x.state === 'done');
  const jobs = S.jobs.filter(j => j.running || j.state === 'manual');
  const signed = S.acct === undefined ? '' : S.acct ? `<span>Nexus Mods: <b>${esc(S.acct.name)}</b>${S.acct.premium ? ', Premium' : ', free account'}</span>` : '<span class="warn">Not signed in to Nexus Mods</span>';
  const grp = (t, list, note) => list.length ? `<div class="group-h">${t}<small>${note || list.length}</small></div>${list.map(dlRow).join('')}` : '';
  return `<div class="top"><div class="grow"><h1>Downloads</h1><p class="lede">On a mod's page on Nexus Mods, press <b>Mod Manager Download</b>: the file comes here${S.acct?.premium ? ', or download straight from a game\'s Browse Nexus' : ''}.</p>
      <div class="quota" style="margin-top:.4em">${signed}${q.hourly != null ? `<span><b>${q.hourly}</b> Nexus requests left this hour</span>` : ''}${q.daily != null ? `<span><b>${q.daily}</b> today</span>` : ''}</div></div>
      <div class="lib-acts"><button class="btn primary f" data-key="pl" data-act="pasteLink">${I.link}Add an nxm:// link</button>${done.length ? `<button class="btn f" data-key="cf" data-act="dlClear">Clear finished</button>` : ''}</div></div>
    <div class="body" data-scroll="dls">${jobs.length ? `<div class="group-h">Modlists installing<small>${jobs.length}</small></div><div class="activity">${jobs.map(actCard).join('')}</div>` : ''}
      ${grp('Needs you', need, need.length ? 'press Mod Manager Download on each file\'s page' : '')}${grp('Downloading', live)}${grp('Finished', done)}
      ${d.items.length ? '' : '<div class="empty">Nothing downloaded yet. Mods you download from Nexus Mods show up here, ready to install into a game.</div>'}</div>`;
}

// Modlists: what's installing (with each file Hearth fetches and the sign-ins it needs), the featured lists, all of them.
const JOB_STATE = { installing: 'Installing', manual: 'Needs files', 'setting-up': 'Setting up', done: 'Installed', failed: 'Failed', cancelled: 'Stopped', stopped: 'Stopped' };
function siteOf(m) { const s = `${m.site || ''} ${m.url || ''}`.toLowerCase(); return m.nexus ? 'nexus' : s.includes('loverslab') ? 'loverslab' : s.includes('mega') ? 'mega' : s.includes('google') || m.gdrive ? 'google' : 'web'; }
function signRow(j) {
  const sites = new Set((j.manual || []).map(siteOf));
  if (j.premiumNeeded || sites.has('nexus') || !j.manual?.length) sites.add('nexus');
  const chips = [...sites].filter(s => s !== 'web').map(s => {
    if (s === 'nexus') { const ok = !!S.acct; return `<span class="sign"><i class="dot ${ok ? (S.acct.premium ? 'good' : '') : 'bad'}"></i>Nexus Mods: ${ok ? (S.acct.premium ? 'Premium, automatic' : 'free, press each file') : 'sign in'}</span>`; }
    const a = S.accts.find(x => x.id === s); const name = a?.name || ({ loverslab: 'LoversLab', mega: 'MEGA', google: 'Google' })[s];
    return `<span class="sign"><i class="dot ${a?.signedIn ? 'good' : ''}"></i>${esc(name)}: ${a?.signedIn ? 'signed in' : a && !a.registered ? 'in the browser' : 'sign in'}</span>`;
  });
  if (sites.has('web')) chips.push('<span class="sign"><i class="dot"></i>Other sites: in the browser</span>');
  return `<div class="need-row"><span>Sign-ins</span>${chips.join('')}${chips.some(c => c.includes('sign in<')) ? '<button class="btn f" data-key="j-acc" data-act="toAccounts">Accounts</button>' : ''}</div>`;
}
function archRows(j) {
  const dls = S.dls.items.filter(x => x.job === j.id);
  const rows = (j.manual || []).map(m => {
    const d = dls.find(x => x.fileName === m.fileName);
    let st, cls = '', bar = '';
    if (m.here) { st = 'Here'; cls = 'good'; }
    else if (d && d.state === 'downloading') { st = `${Math.round(d.done / (d.size || 1) * 100)}%`; bar = `<div class="bar"><i style="width:${(d.done / (d.size || 1) * 100).toFixed(1)}%"></i></div>`; }
    else if (d && d.state === 'needs-link') { st = 'Press download'; cls = 'warn'; }
    else if (m.skipped) { st = 'Skipped'; cls = 'calm'; }
    else if (m.problem) { st = 'Problem'; cls = 'bad'; }
    else { st = m.nexus ? 'Waiting' : 'In the browser'; cls = 'warn'; }
    return `<div class="arch"><span class="flag ${cls === 'good' ? 'upd' : cls === 'warn' ? 'under' : cls === 'bad' ? 'bad' : 'calm'}">${esc(st)}</span><span class="an">${esc(m.modName || m.fileName)}</span>
      ${m.url && !m.here ? `<button class="btn f" style="min-height:1.9em;padding:.15em .6em;font-size:.85em" data-key="mn:${esc(j.id)}:${esc(m.fileName)}" data-act="needOpen" data-url="${esc(m.url)}">Open</button>` : '<span></span>'}
      <small>${esc(m.fileName)}, ${esc(human(m.size))}${m.site ? `, ${esc(m.site)}` : m.nexus ? ', Nexus Mods' : ''}${m.problem ? `: ${esc(m.problem)}` : ''}</small>${bar}</div>`;
  });
  const here = (j.manual || []).filter(m => m.here).length;
  return rows.length ? `<div class="arch-h"><span>Files Hearth fetches for you</span><span>${here} of ${rows.length} here</span></div>${rows.join('')}`
    : `<div class="arch-h"><span>Files</span></div><p class="msg small">The engine downloads every file itself${j.count ? `: ${j.count[0]} of ${j.count[1]} so far` : ''}.</p>`;
}
function jobView(j) {
  const pct = j.count ? j.count[0] / j.count[1] * 100 : null, live = j.running;
  return `<div class="job2"><div class="jl"><h3>${esc(j.title)}<span class="pill ${j.state === 'done' ? 'good' : j.state === 'failed' ? 'bad' : j.state === 'manual' ? 'warn' : ''}">${esc(JOB_STATE[j.state] || j.state)}</span>${j.step ? `<span class="msg small">Step ${j.step[0]} of ${j.step[1]}</span>` : ''}</h3>
    ${live || j.state === 'manual' ? `<div class="msg small">${esc(j.phase || '')}${j.count ? `: ${j.count[0]} of ${j.count[1]}` : ''}</div><div class="prog ${pct == null ? 'spin' : ''}"><i ${pct != null ? `style="width:${pct.toFixed(1)}%"` : ''}></i></div><div class="status">${esc(j.status || '')}</div>` : ''}
    ${j.error ? `<p class="warn small" style="margin:0">${esc(j.error)}</p>` : ''}${(j.notes || []).map(n => `<p class="msg small" style="margin:0">${esc(n)}</p>`).join('')}
    ${j.repair ? `<div class="need-row"><span class="${j.repair === 'failed' ? 'warn' : 'msg'} small">${j.repair === 'running' ? 'Repair: setting up its Proton folder…' : j.repair === 'done' ? 'Repair: done.' : 'Repair didn\'t finish.'} ${(j.repairNotes || []).map(esc).join(' ')}</span></div>` : ''}
    ${j.state !== 'done' ? signRow(j) : ''}
    ${j.guide && j.guide.item ? `<div class="need-row"><span>${esc(j.guide.label)} to fetch in Hearth Browser</span><button class="btn primary f" data-key="gs:${esc(j.id)}" data-act="guideStart" data-job="${esc(j.id)}">Fetch them</button></div>` : ''}
    <div class="acts" style="margin-top:.3em">${live ? `<button class="btn f" data-key="jc:${esc(j.id)}" data-act="jobCancel" data-job="${esc(j.id)}">${I.pause}Stop</button>` : ''}${!live && j.state !== 'done' ? `<button class="btn primary f" data-key="jr:${esc(j.id)}" data-act="jobResume" data-job="${esc(j.id)}">Resume</button>` : ''}${j.state === 'done' ? `<button class="btn f" data-key="jp:${esc(j.id)}" data-act="jobRepair" data-job="${esc(j.id)}">Repair its Proton folder</button><button class="btn f" data-key="jf:${esc(j.id)}" data-act="jobFolder" data-job="${esc(j.id)}">${I.folder}Open its folder</button>` : ''}<button class="btn f" data-key="jl:${esc(j.id)}" data-act="jobLog" data-job="${esc(j.id)}">Log</button>${!live ? `<button class="btn danger f" data-key="jx:${esc(j.id)}" data-act="jobForget" data-job="${esc(j.id)}">Remove…</button>` : ''}</div></div>
    <div class="jr" data-scroll="jr:${esc(j.id)}">${archRows(j)}</div></div>`;
}
function listState(m) { const j = S.jobs.find(x => x.list === m.id); return j ? (j.state === 'done' ? '<span class="pill good">Installed</span>' : `<span class="pill">${esc(JOB_STATE[j.state] || j.state)}</span>`) : ''; }
function galleryCards() {
  const q = S.wjQ.trim().toLowerCase();
  const list = (S.gallery || []).filter(m => (S.nsfw || !m.nsfw) && (S.wjGame === 'all' || m.game === S.wjGame) && (!q || `${m.title} ${m.author} ${(m.tags || []).join(' ')}`.toLowerCase().includes(q)));
  return list.map(m => `<button class="lcard f ${m.down ? 'down' : ''}" data-key="ml:${esc(m.id)}" data-ml="${esc(m.id)}"><span class="art" style="${m.image ? `background-image:url('${esc(m.image)}')` : ''}"><span class="flags">${listState(m)}${m.down ? '<span class="pill bad">Down for now</span>' : ''}${m.nsfw ? '<span class="pill">Adult</span>' : ''}</span></span>
    <span class="lb"><span class="lt">${esc(m.title)}</span><span class="la">${esc(m.gameName || m.game)}${m.author ? `, by ${esc(m.author)}` : ''}</span><span class="sizes"><span><b>${esc(human(m.download))}</b> download</span><span><b>${esc(human(m.install))}</b> installed</span></span></span></button>`).join('') || '<div class="empty">No modlists for that.</div>';
}
function modlistsView() {
  const e = S.engine;
  const head = `<div class="top"><div class="grow"><h1>Modlists</h1><p class="lede">A modlist is a whole modded game put together by its author: Hearth downloads every mod it needs, builds it and puts it in your library. Big lists need hundreds of GB, and most aren't made for Linux, so some need fixes.</p></div>
    <div class="lib-acts">${e?.path ? `<button class="btn f" data-key="wf" data-act="wjFile">${I.folder}From a .wabbajack file</button><button class="btn f" data-key="wr" data-act="wjRefresh">${I.upd}Refresh</button>` : `<button class="btn primary f" data-key="we" data-act="wjEngine">${I.downloads}Get the Wabbajack engine</button>`}</div></div>`;
  let body = S.jobs.length ? `<div class="jobs">${S.jobs.map(jobView).join('')}</div>` : '';
  if (!e?.path) body += '<div class="empty">Hearth OS normally comes with the Wabbajack engine (jackify-engine, GPL-3.0). Get it here, from its GitHub releases.</div>';
  else if (S.gallery == null) body += `<div class="empty"><div class="prog spin" style="max-width:20em;margin:0 auto 1em"><i></i></div>Getting the list of modlists…</div>`;
  else if (S.galleryErr) body += `<div class="empty">${esc(S.galleryErr)}</div>`;
  else {
    const counts = {}; S.gallery.filter(m => S.nsfw || !m.nsfw).forEach(m => { counts[m.game] = (counts[m.game] || 0) + 1; });
    const gamesF = Object.keys(counts).sort((a, b) => counts[b] - counts[a]).slice(0, 7);
    const feat = S.gallery.filter(m => m.official && !m.down && (S.nsfw || !m.nsfw) && (S.wjGame === 'all' || m.game === S.wjGame)).slice(0, 2);
    body += feat.length ? `<div class="group-h">Featured<small>official lists</small></div><div class="featured">${feat.map(m => `<button class="lcard big f" data-key="mf:${esc(m.id)}" data-ml="${esc(m.id)}"><span class="art" style="${m.image ? `background-image:url('${esc(m.image)}')` : ''}"><span class="flags">${listState(m)}</span></span>
      <span class="lb"><span class="lt">${esc(m.title)}</span><span class="la">${esc(m.gameName || m.game)}${m.author ? `, by ${esc(m.author)}` : ''}${m.version ? `, ${esc(m.version)}` : ''}</span>${m.description ? `<span class="ld">${esc(m.description)}</span>` : ''}<span class="sizes"><span><b>${esc(human(m.download))}</b> download</span><span><b>${esc(human(m.install))}</b> installed</span>${m.archives ? `<span><b>${m.archives}</b> files</span>` : ''}</span></span></button>`).join('')}</div>` : '';
    body += `<div class="group-h">All modlists<small>${S.gallery.length}</small></div><div class="tools" style="margin-bottom:.8em"><label class="search">${I.search}<input class="inp f" id="wjq" data-key="wjq" data-osk placeholder="Search modlists" value="${esc(S.wjQ)}" spellcheck="false" autocomplete="off"></label>
      <div class="filters"><button class="fchip f ${S.wjGame === 'all' ? 'on' : ''}" data-key="wg:all" data-act="wjGame" data-g="all">All games</button>${gamesF.map(g => `<button class="fchip f ${S.wjGame === g ? 'on' : ''}" data-key="wg:${esc(g)}" data-act="wjGame" data-g="${esc(g)}">${esc(WJ_NAMES[g] || g)}<i>${counts[g]}</i></button>`).join('')}<button class="fchip f ${S.nsfw ? 'on' : ''}" data-key="fn" data-act="wjNsfw">Adult lists</button></div></div>
      <div class="lists" id="lists">${galleryCards()}</div>`;
  }
  return head + `<div class="body" data-scroll="wj">${body}</div>`;
}

function accountsView() {
  return `<div class="top"><div class="grow"><h1>Accounts</h1><p class="lede">Sign-ins stay on this PC, readable by you only, and go only to their own site and the Wabbajack engine.</p></div></div>
    <div class="body" data-scroll="acc"><div class="acc-grid">${nexusView()}${S.accts.map(acctPanel).join('')}</div></div>`;
}
function settingsView() {
  const e = S.engine, q = S.dls.quota || {};
  const row = (k, t, sub, v, attrs) => `<button class="set f" data-key="${k}" ${attrs}><b>${t}</b><small>${sub}</small><span class="v">${v} ›</span></button>`;
  return `<div class="top"><div class="grow"><h1>Settings</h1><p class="lede">How each game's mods go in (links, copies or the overlay) is set on the game's page. Here is what Hearth Mods shares across games.</p></div></div>
    <div class="body settings" data-scroll="set">
      <div class="group-h">Modlists</div>
      ${row('st-eng', 'Wabbajack engine', e?.path ? esc(e.path) : 'Not installed yet', e?.path ? esc(e.version || 'Installed') : 'Get it', `data-act="${e?.path ? 'engineInfo' : 'wjEngine'}"`)}
      ${row('st-nsfw', 'Adult modlists', 'Show modlists marked as adult in the gallery', S.nsfw ? 'Shown' : 'Hidden', 'data-act="wjNsfw"')}
      <div class="group-h">Nexus Mods</div>
      ${row('st-nx', 'Account', S.acct ? `${esc(S.acct.name)}${q.hourly != null ? `, ${q.hourly} requests left this hour` : ''}` : 'Not signed in', S.acct ? (S.acct.premium ? 'Premium' : 'Free') : 'Sign in', 'data-act="toAccounts"')}
      ${row('st-dl', 'Downloads', 'The queue: two at a time, resumable, kept when the app closes', S.dls.active ? `${S.dls.active} active` : 'Open', 'data-act="toDownloads"')}
      <div class="group-h">Organizer</div>
      ${row('st-org', 'Hearth Mod Organizer', 'Instances made to work like Mod Organizer 2, with profiles and plugins', plural(S.insts.length, 'instance'), 'data-sec="organizer"')}
      <div class="group-h">About</div>
      <div class="set"><b>Hearth Mods</b><small>Nexus Mods, Wabbajack modlists and Hearth's own organizer. Hearth OS ${esc(S.info.version || '')}</small><span class="v"></span></div>
    </div>`;
}
// ---------------------------------------------------------------- adding a mod
async function addMod() {
  const g = game(S.view?.id); if (!g || g.beacon) return;
  const start = (await KIT.invoke('places').catch(() => [])).find(p => p.label === 'Downloads')?.path;
  let src;
  if (KIT.live) {
    const how = await KIT.choose('Add a mod', [{ id: 'file', label: 'From a file', sub: 'A .zip, .7z or .rar file' }, { id: 'folder', label: 'From a folder', sub: 'A folder with the mod in it' }]);
    if (!how) return;
    src = await pick({ folder: how === 'folder', title: how === 'folder' ? 'Choose the mod\'s folder' : 'Choose a mod', start, filters: ['zip', '7z', 'rar', 'tar', 'gz', 'xz', 'zst', 'tgz'] });
  } else src = await KIT.pickPath({ title: 'Choose a mod', note: 'A .zip, .7z or .rar file, or a folder with the mod in it.', files: true, archives: true, folder: true, start });
  if (!src) return;
  KIT.progress('Unpacking the mod');
  let st;
  try { st = await KIT.invoke('stage', { id: g.id, path: src }); } catch (e) { KIT.close(); return KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
  KIT.close();
  setTimeout(() => installDialog(st), 0);
}
/** The "Install a mod" dialog for an unpacked mod (from a file, a folder or a Nexus download), for the chosen game. */
function installDialog(st) {
  const g = game(S.view?.id); if (!g) return;
  const pick = { name: st.name, target: st.suggestTarget || 'game', sub: st.suggestSub || '', enable: true };
  const targets = S.sum?.targets || [{ id: 'game', label: 'Game folder', path: g.install }];
  const draw = () => {
    const root = (targets.find(t => t.id === pick.target) || {}).path || '';
    KIT.dialog(`<h2>Install a mod for ${esc(g.title)}</h2>
      <div class="field"><label for="mname">Name</label><input class="inp f" id="mname" data-osk maxlength="120" value="${esc(pick.name)}" spellcheck="false" autocomplete="off"></div>
      <p class="msg small">${st.files} file${st.files === 1 ? '' : 's'}, ${esc(human(st.size))}${st.unwrapped ? ` · the folder “${esc(st.unwrapped)}” around them is left out` : ''}. At its top:</p>
      <div class="tops">${st.top.slice(0, 14).map(t => `<span>${esc(t)}</span>`).join('')}${st.top.length > 14 ? `<span>+${st.top.length - 14} more</span>` : ''}</div>
      <div class="field"><label>Where it goes</label><div class="chips">${targets.map(t => `<button class="chip f ${t.id === pick.target ? 'on' : ''}" data-tgt="${esc(t.id)}">${esc(t.label)}</button>`).join('')}</div></div>
      <div class="field"><label>Folder inside it</label><button class="btn f" data-dl="sub">${pick.sub ? esc(pick.sub) : '(its top)'} ›</button> ${pick.sub ? '<button class="btn f" data-dl="nosub">Use its top</button>' : ''}<div class="path" style="margin-top:.35em">${esc(root)}${pick.sub ? '/' + esc(pick.sub) : ''}</div></div>
      <div class="field"><button class="chip f ${pick.enable ? 'on' : ''}" data-dl="enable"><span class="sw ${pick.enable ? 'on' : ''}" style="display:inline-block;vertical-align:middle;margin-right:.5em"></span>Turn it on now</button></div>
      <div class="row"><button class="btn f" data-dl="cancel">Cancel</button><button class="btn primary f" data-dl="go" data-autofocus>Install</button></div>`,
      { wide: true, onBack: cancel });
    $('#kit-layer').onclick = async e => {
      const b = e.target.closest('button'); if (!b) return;
      if (b.dataset.tgt) { pick.target = b.dataset.tgt; pick.sub = ''; return draw(); }
      const d = b.dataset.dl;
      if (d === 'cancel') return cancel();
      if (d === 'enable') { pick.enable = !pick.enable; return draw(); }
      if (d === 'nosub') { pick.sub = ''; return draw(); }
      if (d === 'sub') {
        $('#kit-layer').onclick = null;
        const p = await KIT.pickPath({ title: 'Which folder inside it?', start: root + (pick.sub ? '/' + pick.sub : ''), note: root });
        if (p && (p === root || p.startsWith(root + '/'))) pick.sub = p.slice(root.length + 1); else if (p) KIT.toast('Choose a folder inside ' + root);
        return draw();
      }
      if (d === 'go') {
        $('#kit-layer').onclick = null;
        KIT.progress(`Installing ${pick.name}`);
        try { const r = await KIT.invoke('install', { id: g.id, token: st.token, name: pick.name, target: pick.target, sub: pick.sub, enable: pick.enable }); KIT.close(); applyResult(r); KIT.toast(`${pick.name} installed${pick.enable ? ' and on' : ''}`); }
        catch (err) { KIT.close(); KIT.alert('That didn\'t work', esc(KIT.err(err)), false); }
      }
    };
    $('#mname').addEventListener('input', e => { pick.name = e.target.value; });
  };
  const cancel = () => { $('#kit-layer').onclick = null; KIT.invoke('cancel_stage', { token: st.token }).catch(() => {}); KIT.close(); };
  draw();
}

// ---------------------------------------------------------------- choosing files: the standard file chooser (Hearth Files), Hearth's own browser as a fallback
async function pick({ folder = false, title = '', start, filters, fallback = {} }) {
  if (KIT.live) { try { return await KIT.invoke('pick_path', { folder, title, start: start || null, filters: filters || null }); } catch (e) { console.warn(e); } }
  return KIT.pickPath(Object.assign({ title, start, folder, files: !folder }, fallback));
}

// ---------------------------------------------------------------- how mods go in
function deployLabel(s) { return ({ hardlink: 'Links', copy: 'Copies', overlay: 'Overlay' })[s.deploy] || 'Links'; }
async function deployMode() {
  const s = S.sum; if (!s) return;
  const sub = { hardlink: 'No extra space and instant. Across drives, copies.', copy: 'Real copies in the game folder: takes the mods\' space twice.', overlay: 'Beta Experimental. The game folder itself is never changed: the mods are laid over it while you play.' };
  const modes = s.modes || [{ id: 'hardlink', label: 'Links' }, { id: 'copy', label: 'Copies' }, { id: 'overlay', label: 'Overlay (Beta Experimental)' }];
  const m = await KIT.choose('How mods are put in place', modes.map(x => ({ id: x.id, label: x.label, sub: sub[x.id], end: x.id === s.deploy ? 'Now' : '' })),
    'Every mod comes out the old way and goes back in the new way. Close the game first.');
  if (!m || m === s.deploy) return;
  KIT.progress('Switching', 'Taking the mods out and putting them back');
  try { const r = await KIT.invoke('set_deploy', { id: S.view.id, mode: m }); KIT.close(); applyResult(r); KIT.toast(`Mods now go in as ${deployLabel(r.summary).toLowerCase()}`); }
  catch (e) { KIT.close(); KIT.alert('Couldn\'t switch', esc(KIT.err(e)), false); }
}

// ---------------------------------------------------------------- Nexus Mods: getting a mod for this game
async function nexusGet() {
  const g = game(S.view?.game || S.view?.id); if (!g) return;
  if (S.acct === undefined) await loadAccount();
  const domain = S.sum?.nexus || '';
  const items = [];
  if (domain) items.push({ id: 'site', label: 'Browse its mods on Nexus Mods', sub: 'Opens the site. Press "Mod Manager Download" on a file and it comes to Downloads here.' });
  else items.push({ id: 'pick', label: 'Which game is it on Nexus Mods?', sub: 'Type the part of its Nexus address after nexusmods.com/ (skyrimspecialedition, stardewvalley…)' });
  if (S.acct?.premium) items.push({ id: 'url', label: 'Download a mod by its address (Premium)', sub: 'Paste a mod page\'s address and pick the file, without the site.' });
  if (!S.acct) items.push({ id: 'signin', label: 'Sign in to Nexus Mods', sub: 'Needed for downloads through Hearth.' });
  if (domain) items.push({ id: 'pick', label: `Nexus game: ${domain}`, sub: 'Change which Nexus game this is' });
  const a = await KIT.choose(`${g.title} on Nexus Mods`, items);
  if (a === 'site') return openUrl(`https://www.nexusmods.com/${domain}/mods/`);
  if (a === 'signin') return setSection('accounts');
  if (a === 'url') return premiumPick(g);
  if (a === 'pick') {
    const v = await ask('Nexus game', 'The name in its Nexus address: nexusmods.com/<b>this part</b>/mods', domain);
    if (v == null) return;
    try { const r = await KIT.invoke('set_nexus_game', { id: g.id, domain: v.trim().toLowerCase() }); if (S.view?.type === 'game') S.sum = r; else if (S.sum) S.sum.nexus = r.nexus; render(); KIT.toast('Saved'); } catch (e) { KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
  }
}
/** A one-line text question (with the on-screen keyboard from a controller). */
function ask(title, note, value = '', mono = false) {
  return new Promise(res => {
    KIT.dialog(`<h2>${esc(title)}</h2>${note ? `<p class="msg small">${note}</p>` : ''}<div class="field"><input class="inp f ${mono ? 'mono' : ''}" id="askv" data-osk maxlength="400" value="${esc(value)}" spellcheck="false" autocomplete="off" data-autofocus></div>
      <div class="row"><button class="btn f" data-k="no">Cancel</button><button class="btn primary f" data-k="ok">OK</button></div>`, { wide: true, onBack: () => { $('#kit-layer').onclick = null; KIT.close(); res(null); } });
    $('#kit-layer').onclick = e => { const b = e.target.closest('[data-k]'); if (!b) return; const v = $('#askv').value; $('#kit-layer').onclick = null; KIT.close(); res(b.dataset.k === 'ok' ? v : null); };
  });
}
function askSecret(title, note) {
  return new Promise(res => {
    KIT.dialog(`<h2>${esc(title)}</h2>${note ? `<p class="msg small">${esc(note)}</p>` : ''}<div class="field"><input class="inp f" id="asks" type="password" data-osk maxlength="400" spellcheck="false" autocomplete="off" data-autofocus></div>
      <div class="row"><button class="btn f" data-k="no">Cancel</button><button class="btn primary f" data-k="ok">OK</button></div>`, { wide: true, onBack: () => { $('#kit-layer').onclick = null; KIT.close(); res(null); } });
    $('#kit-layer').onclick = e => { const b = e.target.closest('[data-k]'); if (!b) return; const v = $('#asks').value; $('#asks').value = ''; $('#kit-layer').onclick = null; KIT.close(); res(b.dataset.k === 'ok' ? v : null); };
  });
}

// The guided flow: one file at a time in Hearth Browser. Hearth opens the file's page, you press Download there, the
// app checks what lands in Downloads against the list and opens the next page by itself. A (Open again), X (I have it:
// choose the file), Y (Skip for now), B (Stop).
async function guideFlow(jobId) {
  let shown = null, alive = true, gen = 0;
  const stop = () => { alive = false; $('#kit-layer').onclick = null; KIT.close(); refreshJobs(); };
  const draw = (j, g) => {
    const m = g.item;
    KIT.dialog(`<h2 id="gl">${esc(g.label)}</h2><div class="prog"><i id="gp" style="width:${(g.done / g.total * 100).toFixed(1)}%"></i></div>
      <p class="msg" style="margin:.7em 0 .2em"><b>${esc(m.modName || m.fileName)}</b></p>
      <p class="msg small" style="margin:0">${esc(m.fileName)} · ${esc(human(m.size))}${m.site ? ` · ${esc(m.site)}` : ''}</p>
      <p class="msg small">The page is open in Hearth Browser: press <b>Download</b> for this file (sign in there first if the site asks). Hearth checks it and opens the next one by itself.</p>
      <p class="warn small" id="gw">${esc(m.problem || '')}</p>
      <div class="row"><button class="btn f" data-g="stop">Stop</button><button class="btn f" data-g="skip">Skip for now</button><button class="btn f" data-g="file">I have it: choose the file…</button><button class="btn primary f" data-g="open" data-autofocus>Open the page again</button></div>`,
      { wide: true, onBack: stop });
    $('#kit-layer').onclick = async e => {
      const b = e.target.closest('[data-g]'); if (!b) return;
      const k = b.dataset.g;
      if (k === 'stop') return stop();
      if (k === 'open') return KIT.invoke('open_page', { url: m.url }).catch(err => KIT.toast(KIT.err(err)));
      if (k === 'skip') { await KIT.invoke('wj_guide_skip', { id: jobId, file: m.fileName }); shown = null; return poll(true, ++gen); }
      if (k === 'file') {
        gen++;
        const p = await pick({ title: `Choose ${m.fileName}` });
        if (p) { try { await KIT.invoke('wj_manual_file', { id: jobId, file: m.fileName, path: p }); } catch (err) { KIT.toast(KIT.err(err)); } }
        shown = null; return poll(true, ++gen);
      }
    };
  };
  const poll = async (now, my) => {
    if (!alive || my !== gen) return;
    try { S.jobs = await KIT.invoke('wj_jobs'); } catch (e) {}
    const j = S.jobs.find(x => x.id === jobId), g = j && j.guide;
    if (!g || !g.item) { alive = false; $('#kit-layer').onclick = null; KIT.close(); KIT.toast(g ? 'All the files are here. The install carries on by itself.' : 'Nothing left to fetch.'); return refreshJobs(); }
    if (shown !== g.item.fileName) {
      if (shown !== null && !now) KIT.toast('Got it. Opening the next one');
      shown = g.item.fileName;
      draw(j, g);
      KIT.invoke('open_page', { url: g.item.url }).catch(err => KIT.toast(KIT.err(err)));
    } else {
      const gl = $('#gl'), gp = $('#gp'), gw = $('#gw');
      if (gl) gl.textContent = g.label;
      if (gp) gp.style.width = `${(g.done / g.total * 100).toFixed(1)}%`;
      if (gw) gw.textContent = g.item.problem || '';
    }
    setTimeout(() => poll(false, my), 1500);
  };
  poll(true, gen);
}

function openUrl(url) { return KIT.invoke('open_url', { url }).then(() => KIT.toast('Opening the page…')).catch(e => KIT.alert('Couldn\'t open it', esc(KIT.err(e)), false)); }
async function premiumPick(g) {
  const url = await ask('Download from Nexus Mods', 'Paste the mod page\'s address.', '', true);
  if (!url) return;
  KIT.progress('Looking it up');
  let m;
  try { m = await KIT.invoke('nexus_files', { url }); } catch (e) { KIT.close(); return KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
  KIT.close();
  const f = await KIT.choose(m.name || 'Mod', m.files.map(x => ({ id: x.fileId, label: x.name + (x.version ? ` ${x.version}` : ''), sub: `${x.category || ''} · ${x.fileName || ''}`, end: human(x.size) })), esc(m.summary || ''));
  if (f == null) return;
  try { await KIT.invoke('download_file', { domain: m.domain, modId: m.modId, fileId: f, game: g?.id || '' }); KIT.toast('Downloading — see Downloads'); refreshDownloads(); }
  catch (e) { KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
}

// ---------------------------------------------------------------- Downloads
let dlTimer = 0;
async function refreshDownloads() {
  clearTimeout(dlTimer);
  try {
    S.dls = await KIT.invoke('downloads_list');
    for (const n of S.dls.notices || []) { if (n.ok) KIT.toast(n.text); else KIT.alert('A Nexus download didn\'t start', esc(n.text), false); }
  } catch (e) {}
  if (S.section === 'downloads' && !KIT.dialogOpen()) render(); else railCounts();
  dlTimer = setTimeout(refreshDownloads, S.section === 'downloads' || S.dls.active ? 1000 : 4000);
}

async function pasteLink() {
  const v = await ask('A Nexus Mods download link', 'The site\'s "Mod Manager Download" opens Hearth Mods by itself. If it didn\'t, copy the nxm:// link here.', '', true);
  if (!v) return;
  try { await KIT.invoke('download_nxm', { link: v.trim() }); KIT.toast('Downloading'); refreshDownloads(); } catch (e) { KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
}
async function downloadActions(id) {
  const x = S.dls.items.find(i => i.id === id); if (!x) return;
  const items = [];
  if (x.state === 'done' && !x.job) items.push({ id: 'install', label: 'Install into a game…', sub: 'Unpacks it and asks where it goes' });
  if (['downloading', 'queued'].includes(x.state)) items.push({ id: 'pause', label: 'Pause' });
  if (['paused', 'failed'].includes(x.state)) items.push({ id: 'resume', label: 'Resume' });
  if (x.state === 'needs-link' || x.state === 'failed') items.push({ id: 'site', label: 'Open on Nexus Mods', sub: 'Press "Mod Manager Download" there' });
  if (x.state === 'done') items.push({ id: 'folder', label: 'Open its folder' });
  items.push({ id: 'remove', label: 'Take out of the list' });
  if (x.state === 'done') items.push({ id: 'delete', label: 'Delete the file…' });
  const a = await KIT.choose(x.name, items, `<span class="path">${esc(x.path)}</span>`);
  try {
    if (a === 'install') return installDownload(x);
    if (a === 'pause') await KIT.invoke('download_pause', { id });
    if (a === 'resume') await KIT.invoke('download_resume', { id });
    if (a === 'site') return openUrl(x.page);
    if (a === 'folder') return KIT.invoke('open_folder', { path: x.path.replace(/\/[^/]*$/, '') });
    if (a === 'remove') await KIT.invoke('download_remove', { id, deleteFile: false });
    if (a === 'delete' && await KIT.confirm({ title: 'Delete the file?', text: `<p class="path">${esc(x.path)}</p>`, ok: 'Delete', danger: true })) await KIT.invoke('download_remove', { id, deleteFile: true });
  } catch (e) { KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
  refreshDownloads();
}
async function installDownload(x) {
  KIT.progress('Unpacking the mod');
  let st;
  try { st = await KIT.invoke('stage_download', { id: x.id, gameId: '' }); }
  catch (e) {
    KIT.close();
    if (!String(KIT.err(e)).includes('Choose which game')) return KIT.alert('That didn\'t work', esc(KIT.err(e)), false);
    const gid = await KIT.choose('Which game is it for?', S.games.filter(g => !g.beacon).map(g => ({ id: g.id, label: g.title })));
    if (!gid) return;
    KIT.progress('Unpacking the mod');
    try { st = await KIT.invoke('stage_download', { id: x.id, gameId: gid }); } catch (e2) { KIT.close(); return KIT.alert('That didn\'t work', esc(KIT.err(e2)), false); }
  }
  KIT.close();
  await openGame(st.game, false);
  installDialog(st);
}

// ---------------------------------------------------------------- Nexus account
async function loadAccount() { try { S.acct = await KIT.invoke('nexus_account'); } catch (e) { S.acct = null; } if (S.section === 'accounts') render(); }
// ---------------------------------------------------------------- LoversLab, MEGA, Google (accounts.rs)
let acctTimer = 0;
async function loadAccounts() {
  clearTimeout(acctTimer);
  try { S.accts = await KIT.invoke('accounts_list'); } catch (e) {}
  if (S.section === 'accounts' && !KIT.dialogOpen()) render();
  if (S.accts.some(a => a.pending === 'waiting')) acctTimer = setTimeout(loadAccounts, 1500);
}


// ---------------------------------------------------------------- Modlists (Wabbajack)
let jobTimer = 0;
async function loadModlists(refresh = false) {
  try { S.engine = await KIT.invoke('wj_engine'); } catch (e) { S.engine = null; }
  await refreshJobs();
  if (S.engine?.path && (S.gallery == null || refresh)) {
    S.galleryErr = ''; render();
    try { S.gallery = await KIT.invoke('wj_gallery', { refresh }); } catch (e) { S.gallery = []; S.galleryErr = KIT.err(e); }
  }
  if (S.section === 'modlists' || S.section === 'settings') render();
}
async function refreshJobs() {
  clearTimeout(jobTimer);
  try { S.jobs = await KIT.invoke('wj_jobs'); } catch (e) {}
  const live = S.jobs.some(j => j.running || j.repair === 'running');
  if (S.section === 'modlists' && !KIT.dialogOpen() && document.activeElement?.id !== 'wjq') render(); else railCounts();
  if (live || S.section === 'modlists') jobTimer = setTimeout(refreshJobs, live ? 1000 : 5000);
}

async function wjInstall(m) {
  const d = await KIT.invoke('wj_defaults', { title: m.title, game: m.game });
  const ch = { install: d.installDir, downloads: d.downloadsDir, gameDir: d.gameDir || '' };
  const draw = () => {
    KIT.dialog(`<h2>Install ${esc(m.title)}</h2>
      <p class="msg small">${esc(m.author ? 'By ' + m.author + ' · ' : '')}${esc(m.gameName || m.game)} · ${esc(human(m.download))} to download, ${esc(human(m.install))} installed${m.archives ? ` · ${m.archives} files` : ''}${d.free != null ? ` · ${esc(human(d.free))} free there` : ''}</p>
      ${m.description ? `<p class="msg small">${esc(m.description).slice(0, 400)}</p>` : ''}
      ${!d.known ? '<p class="warn small">Hearth doesn\'t know this game yet: the list installs, but its Proton folder may need setting up by hand.</p>' : ''}
      ${S.acct ? (S.acct.premium ? '' : '<p class="msg small"><b>Free Nexus account:</b> after the list is read, you\'ll press “Mod Manager Download” once for each Nexus file. Premium does it all by itself.</p>') : '<p class="warn small">Sign in to Nexus Mods first (Accounts): nearly every list downloads from it.</p>'}
      <div class="field"><label>The game</label><button class="btn f" data-k="game">${ch.gameDir ? esc(ch.gameDir) : 'Choose its folder…'}</button>${d.gameTitle ? `<div class="msg small">${esc(d.gameTitle)} from your library</div>` : ''}</div>
      <div class="field"><label>The modlist goes in</label><button class="btn f" data-k="inst">${esc(ch.install)} ›</button></div>
      <div class="field"><label>Its downloads go in</label><button class="btn f" data-k="dls">${esc(ch.downloads)} ›</button></div>
      <div class="row">${m.readme ? '<button class="btn f" data-k="readme">Read me</button>' : ''}<button class="btn f" data-k="no">Cancel</button><button class="btn primary f" data-k="go" ${ch.gameDir ? '' : 'disabled'} data-autofocus>Install</button></div>`, { wide: true });
    $('#kit-layer').onclick = async e => {
      const b = e.target.closest('[data-k]'); if (!b) return;
      const k = b.dataset.k;
      if (k === 'no') { $('#kit-layer').onclick = null; return KIT.close(); }
      if (k === 'readme') return openUrl(m.readme);
      if (['game', 'inst', 'dls'].includes(k)) {
        $('#kit-layer').onclick = null;
        const cur = k === 'game' ? ch.gameDir : k === 'inst' ? ch.install : ch.downloads;
        const p = await pick({ folder: true, title: k === 'game' ? 'The game\'s folder' : k === 'inst' ? 'Where the modlist goes (an empty folder)' : 'Where its downloads go', start: cur ? cur.replace(/\/[^/]*$/, '') : undefined });
        if (p) { if (k === 'game') ch.gameDir = p; else if (k === 'inst') ch.install = p + '/' + (m.title || 'Modlist').replace(/[\/:*?"<>|]/g, ' ').trim(); else ch.downloads = p; }
        return draw();
      }
      if (k === 'go') {
        $('#kit-layer').onclick = null; KIT.close();
        try { await KIT.invoke('wj_start', { title: m.title, list: m.id, game: m.game, gameDir: ch.gameDir, installDir: ch.install, downloadsDir: ch.downloads }); KIT.toast(`Installing ${m.title}`); refreshJobs(); }
        catch (err) { KIT.alert('Couldn\'t start', esc(KIT.err(err)), false); }
      }
    };
  };
  draw();
}

async function act(a, b) {
  const job = b?.dataset?.job;
  const sel = () => modsModel().find(m => m.id === S.sel);
  try {
    if (a === 'toModlists') return setSection('modlists');
    if (a === 'toDownloads') return setSection('downloads');
    if (a === 'toAccounts') return setSection('accounts');
    if (a === 'backToLib') { S.view = null; S.sel = null; render(); return KIT.focusFirst('.sec.on'); }
    if (a === 'beacon') return KIT.invoke('open_beacon').then(() => KIT.toast('Opening Beacon…'));
    if (a === 'play') { try { await KIT.invoke('play', { id: S.view.id }); } catch (e) { return KIT.alert('It didn\'t start', `<p>${esc(KIT.err(e)).replace(/\n/g, '<br>')}</p><p class="msg small">Every step is in Hearth's log: Settings › System › Send logs.</p>`, false); } return KIT.toast(`Starting ${game(S.view.id)?.title || 'the game'} with its mods…`); }
    if (a === 'deployMode') return deployMode();
    if (a === 'add') return addMod();
    if (a === 'nexusGet') return nexusGet();
    if (a === 'more') return gameMore();
    if (a === 'addToSteam') return addToSteam();
    if (a === 'checkUpdates') return checkUpdates();
    if (a === 'gameModlists') { S.wjGame = b.dataset.wj; return setSection('modlists'); }
    if (a === 'wjGame') { S.wjGame = b.dataset.g; return render(); }
    if (a === 'toggleSel') { const m = sel(); return m && toggleMod(m, !m.enabled); }
    if (a === 'moveSel') { const m = sel(); if (!m) return; await moveMod(m.id, +b.dataset.by); return KIT.focusFirst(`[data-key="${b.dataset.key}"]`); }
    if (a === 'openSel') { const m = sel(); return m && openModFiles(m); }
    if (a === 'removeSel') { const m = sel(); return m && removeMod(m); }
    if (a === 'getUpdate') { const m = sel(); return m && getUpdate(m); }
    if (a === 'note') { const m = sel(); return m && editNote(m); }
    if (a === 'nexusPage') { const m = sel(); return m?.nexus && openUrl(`https://www.nexusmods.com/${m.nexus.domain}/mods/${m.nexus.modId}`); }
    if (a === 'openStore') return KIT.invoke('open_folder', { path: S.sum.folder + '/files' });
    if (a === 'orgFolder') return KIT.invoke('open_folder', { path: S.org.instance.modsDir });
    if (a === 'engineInfo') return KIT.alert('Wabbajack engine', `<p>jackify-engine ${esc(S.engine?.version || '')} (GPL-3.0)${S.engine?.system ? ', from Hearth OS' : ''}.</p><p class="path">${esc(S.engine?.path || '')}</p>`);
    if (a === 'pasteLink') return pasteLink();
    if (a === 'dlClear') { await KIT.invoke('downloads_clear'); return refreshDownloads(); }
    if (a === 'dlPause') { await KIT.invoke('download_pause', { id: b.dataset.dl }); return refreshDownloads(); }
    if (a === 'dlResume') { await KIT.invoke('download_resume', { id: b.dataset.dl }); return refreshDownloads(); }
    if (a === 'keyPage') { const p = await KIT.invoke('nexus_pages'); return openUrl(p.keys); }
    if (a === 'keyIn') {
      const k = await ask('Your Nexus Mods API key', 'Paste the whole key.', '', true); if (!k) return;
      KIT.progress('Checking the key');
      try { S.acct = await KIT.invoke('nexus_sign_in', { key: k }); KIT.close(); render(); KIT.toast(`Signed in as ${S.acct.name}`); } catch (e) { KIT.close(); KIT.alert('Couldn\'t sign in', esc(KIT.err(e)), false); }
      return;
    }
    if (a === 'nexusRefresh') { S.acct = await KIT.invoke('nexus_refresh'); render(); return KIT.toast('Checked'); }
    if (a === 'nexusSite') return openUrl('https://www.nexusmods.com/');
    if (a === 'nexusOut') { if (!await KIT.confirm({ title: 'Sign out of Nexus Mods?', text: '<p>Hearth forgets the key. Downloads waiting in the queue stop until you sign in again.</p>', ok: 'Sign out', danger: true })) return; await KIT.invoke('nexus_sign_out'); S.acct = null; return render(); }
    if (a === 'wjEngine') { KIT.progress('Getting the Wabbajack engine', 'Downloading jackify-engine'); try { S.engine = await KIT.invoke('wj_fetch_engine'); KIT.close(); } catch (e) { KIT.close(); return KIT.alert('Couldn\'t get it', esc(KIT.err(e)), false); } return loadModlists(true); }
    if (a === 'wjRefresh') { S.gallery = null; return loadModlists(true); }
    if (a === 'wjFilter') { S.filter = b.dataset.f; return render(); }
    if (a === 'wjNsfw') { S.nsfw = !S.nsfw; return render(); }
    if (a === 'wjFile') {
      const f = await pick({ title: 'Choose a .wabbajack file', filters: ['wabbajack'] }); if (!f) return;
      if (!f.endsWith('.wabbajack')) return KIT.alert('That isn\'t a .wabbajack file', '', false);
      const name = f.replace(/^.*\//, '').replace(/\.wabbajack$/, '');
      const game = await KIT.choose('Which game is it for?', [['SkyrimSpecialEdition', 'Skyrim Special Edition'], ['Fallout4', 'Fallout 4'], ['FalloutNewVegas', 'Fallout: New Vegas'], ['Fallout3', 'Fallout 3'], ['Oblivion', 'Oblivion'], ['Starfield', 'Starfield'], ['Cyberpunk2077', 'Cyberpunk 2077'], ['BaldursGate3', "Baldur's Gate 3"]].map(([id, label]) => ({ id, label })));
      if (!game) return;
      return wjInstall({ id: f, title: name, game, gameName: game });
    }
    if (a === 'needOpen') return openUrl(b.dataset.url);
    if (a === 'guideStart') return guideFlow(job);
    if (a === 'acctBegin') { const r = await KIT.invoke('account_begin', { id: b.dataset.p }); KIT.toast('Sign in in the browser window'); S.accts = S.accts.map(x => x.id === r.id ? r : x); render(); return loadAccounts(); }
    if (a === 'acctOut') { const p = S.accts.find(x => x.id === b.dataset.p); if (!await KIT.confirm({ title: `Sign out of ${p.name}?`, text: '<p>Hearth forgets the sign-in. Files from there are fetched in the browser again.</p>', ok: 'Sign out', danger: true })) return; await KIT.invoke('account_sign_out', { id: p.id }); return loadAccounts(); }
    if (a === 'megaIn') {
      const email = await ask('Your MEGA email address', '', '', true); if (!email) return;
      const pw = await askSecret('Your MEGA password', 'Hearth works out MEGA\'s login from it and forgets the password.'); if (!pw) return;
      KIT.progress('Signing in to MEGA');
      try { await KIT.invoke('mega_sign_in', { email: email.trim(), password: pw }); KIT.close(); KIT.toast('Signed in to MEGA'); } catch (e) { KIT.close(); KIT.alert('Couldn\'t sign in', esc(KIT.err(e)), false); }
      return loadAccounts();
    }
    if (a === 'jobCancel') { await KIT.invoke('wj_cancel', { id: job }); return refreshJobs(); }
    if (a === 'jobResume') { await KIT.invoke('wj_resume', { id: job }); return refreshJobs(); }
    if (a === 'jobFolder') { const j = S.jobs.find(x => x.id === job); return KIT.invoke('open_folder', { path: j.installDir }); }
    // Repair runs on its own (it takes many minutes): the call only starts it; the list's card shows how it goes.
    if (a === 'jobRepair') { let n; try { n = await KIT.invoke('wj_repair', { id: job }); } catch (e) { return KIT.alert('Repair', esc(String(e))); } refreshJobs(); return KIT.alert('Repair', (n || []).map(esc).join('<br>') || 'Started.'); }
    if (a === 'jobLog') { const t = await KIT.invoke('wj_log', { id: job }); return KIT.alert('Log', `<pre class="status" style="white-space:pre-wrap;max-height:55vh;overflow:auto">${esc(t || 'Nothing yet.')}</pre>`); }
    if (a === 'jobForget') {
      const c = await KIT.choose('Remove this modlist?', [{ id: 'keep', label: 'Take it out of Hearth only', sub: 'Its folder and downloads stay' }, { id: 'files', label: 'Delete the modlist\'s folder', sub: 'The downloads stay, for installing it again' }, { id: 'all', label: 'Delete its folder and its downloads' }]);
      if (!c) return;
      await KIT.invoke('wj_forget', { id: job, deleteFiles: c !== 'keep', deleteDownloads: c === 'all' });
      return refreshJobs();
    }
  } catch (e) { KIT.close(); KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
}

// ---------------------------------------------------------------- the Hearth Mod Organizer (organizer/ in the library; MO2's design)
const org = args => KIT.invoke('organizer', { args });
function orgApply(r) {
  if (r?.summary) S.org = r.summary; S.conf = {};
  render();
  const n = [...(r?.notes || []), ...(r?.deployProblem ? [r.deployProblem] : [])];
  if (n.length) KIT.alert('Done', n.map(esc).join('<br>'));
}
async function orgDo(args, label) {
  try { const r = await org([S.view.id, ...args]); orgApply(r); if (label) KIT.toast(label); return r; } catch (e) { KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
}

async function orgPlugin(name, how) {
  const p = S.org?.plugins.find(x => x.name === name); if (!p) return;
  if (p.implicit) return KIT.toast('The game always loads it.');
  const idx = S.org.plugins.indexOf(p);
  const a = how || await KIT.choose(name, [{ id: 'toggle', label: p.active ? 'Turn off' : 'Turn on' }, { id: 'up', label: 'Load earlier' }, { id: 'down', label: 'Load later' }],
    `${esc(p.mod || 'The game')}${p.masters.length ? `<br>Needs: ${esc(p.masters.join(', '))}` : ''}${p.author ? `<br>By ${esc(p.author)}` : ''}`);
  if (a === 'toggle') return orgDo([p.active ? 'plugin-disable' : 'plugin-enable', name]);
  if (a === 'up' || a === 'down') { await orgDo(['plugin-move', name, String(idx + (a === 'up' ? -1 : 1))]); S.sel = 'p:' + name; drawDetails(); KIT.focusFirst(`[data-key="op:${CSS.escape(name)}"]`); }
}
async function orgInstall() {
  const src = await pick({ title: 'Choose a mod', filters: ['zip', '7z', 'rar', 'tar', 'gz', 'xz', 'zst', 'tgz'], fallback: { files: true, archives: true, folder: true } });
  if (!src) return;
  KIT.progress('Unpacking the mod');
  let r;
  try { r = await org([S.view.id, 'install', src]); } catch (e) { KIT.close(); return KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
  KIT.close();
  if (r.needs === 'fomod') return setTimeout(() => fomodDialog(r), 0);
  if (r.needs === 'bain') return setTimeout(() => bainDialog(r), 0);
  orgApply(r); KIT.toast(`${r.installed} installed`);
}
/** A FOMOD installer, one step at a time: A picks, the engine says what shows and what's allowed after each pick. */
function fomodDialog(r) {
  const f = r.fomod; let ev = r.evaluation; let at = 0;
  const visible = () => ev.steps.map((s, i) => s.visible ? i : -1).filter(i => i >= 0);
  const cancel = () => { $('#kit-layer').onclick = null; org([S.view.id, 'cancel', r.token]).catch(() => {}); KIT.close(); };
  const draw = () => {
    const vs = visible(); if (!vs.includes(at)) at = vs[0] ?? 0;
    const st = f.steps[at], se = ev.steps[at] || { groups: [] };
    const last = vs.indexOf(at) === vs.length - 1;
    const groups = (st?.groups || []).map((g, gi) => `<div class="field"><label>${esc(g.name)} <span class="msg small">${({ SelectExactlyOne: 'choose one', SelectAtMostOne: 'one at most', SelectAtLeastOne: 'one or more', SelectAll: 'all of these', SelectAny: 'any' })[g.type] || ''}</span></label>
      <div class="list">${g.plugins.map((p, pi) => { const on = se.groups[gi]?.selected.includes(pi); const t = se.groups[gi]?.types[pi] || ''; return `<button class="item f ${on ? 'sel' : ''}" data-fg="${gi}" data-fp="${pi}" ${t === 'NotUsable' ? 'disabled' : ''} style="grid-template-columns:auto 1fr auto"><span class="sw ${on ? 'on' : ''}"></span><span class="n">${esc(p.name)}</span><span class="end">${t === 'Recommended' || t === 'Required' ? esc(t) : t === 'NotUsable' ? 'Can\'t be used' : ''}</span>${p.description ? `<span class="m" style="grid-column:2/-1">${esc(p.description.slice(0, 300))}</span>` : ''}</button>`; }).join('')}</div>
      ${se.groups[gi]?.problem ? `<p class="warn small">${esc(se.groups[gi].problem)}</p>` : ''}</div>`).join('');
    KIT.dialog(`<h2>${esc(r.name)}${st ? ` · ${esc(st.name)}` : ''}</h2><p class="msg small">Step ${vs.indexOf(at) + 1} of ${vs.length}${f.info?.author ? ` · by ${esc(f.info.author)}` : ''}</p>${groups || '<p class="msg">Nothing to choose: install it.</p>'}
      ${ev.errors.length && last ? `<p class="warn small">${ev.errors.map(esc).join('<br>')}</p>` : ''}
      <div class="row"><button class="btn f" data-fd="cancel">Cancel</button>${vs.indexOf(at) > 0 ? '<button class="btn f" data-fd="back">Back</button>' : ''}<button class="btn primary f" data-fd="${last ? 'go' : 'next'}" data-autofocus>${last ? 'Install' : 'Next'}</button></div>`, { wide: true, onBack: cancel });
    $('#kit-layer').onclick = async e => {
      const b = e.target.closest('button'); if (!b || b.disabled) return;
      if (b.dataset.fg != null) {
        const gi = +b.dataset.fg, pi = +b.dataset.fp, g = f.steps[at].groups[gi];
        const cur = ev.steps[at].groups[gi].selected;
        const one = g.type === 'SelectExactlyOne' || g.type === 'SelectAtMostOne';
        const next = cur.includes(pi) ? (g.type === 'SelectExactlyOne' ? cur : cur.filter(x => x !== pi)) : one ? [pi] : [...cur, pi];
        const ch = ev.choices.map(s => s.map(x => [...x]));
        ch[at][gi] = next;
        try { ev = (await org([S.view.id, 'fomod-eval', r.token, '--choices', JSON.stringify(ch)])).evaluation; } catch (er) { KIT.toast(KIT.err(er)); }
        draw(); return document.querySelector(`[data-fg="${gi}"][data-fp="${pi}"]`)?.focus();
      }
      const d = b.dataset.fd;
      if (d === 'cancel') return cancel();
      if (d === 'back') { at = visible()[visible().indexOf(at) - 1]; return draw(); }
      if (d === 'next') { at = visible()[visible().indexOf(at) + 1]; return draw(); }
      if (d === 'go') {
        $('#kit-layer').onclick = null; KIT.progress(`Installing ${r.name}`);
        try { const res = await org([S.view.id, 'install', 'token:' + r.token, '--name', r.name, '--choices', JSON.stringify(ev.choices)]); KIT.close(); orgApply(res); KIT.toast(`${res.installed} installed`); }
        catch (er) { KIT.close(); KIT.alert('That didn\'t work', esc(KIT.err(er)), false); }
      }
    };
  };
  draw();
}
function bainDialog(r) {
  const on = new Set(r.packages.filter(p => p.default).map(p => p.name));
  const cancel = () => { $('#kit-layer').onclick = null; org([S.view.id, 'cancel', r.token]).catch(() => {}); KIT.close(); };
  const draw = () => {
    KIT.dialog(`<h2>${esc(r.name)}</h2><p class="msg small">This package has parts: choose which to install (later ones win).</p><div class="list">${r.packages.map(p => `<button class="item f ${on.has(p.name) ? 'sel' : ''}" data-bp="${esc(p.name)}" style="grid-template-columns:auto 1fr"><span class="sw ${on.has(p.name) ? 'on' : ''}"></span><span class="n">${esc(p.name)}</span></button>`).join('')}</div>
      <div class="row"><button class="btn f" data-bd="cancel">Cancel</button><button class="btn primary f" data-bd="go" data-autofocus>Install</button></div>`, { onBack: cancel });
    $('#kit-layer').onclick = async e => {
      const b = e.target.closest('button'); if (!b) return;
      if (b.dataset.bp) { on.has(b.dataset.bp) ? on.delete(b.dataset.bp) : on.add(b.dataset.bp); draw(); return document.querySelector(`[data-bp="${CSS.escape(b.dataset.bp)}"]`)?.focus(); }
      if (b.dataset.bd === 'cancel') return cancel();
      if (b.dataset.bd === 'go') {
        $('#kit-layer').onclick = null; KIT.progress(`Installing ${r.name}`);
        try { const res = await org([S.view.id, 'install', r.token, '--name', r.name, '--choices', JSON.stringify([...on])]); KIT.close(); orgApply(res); KIT.toast(`${res.installed} installed`); }
        catch (er) { KIT.close(); KIT.alert('That didn\'t work', esc(KIT.err(er)), false); }
      }
    };
  };
  draw();
}

async function orgAction(a) {
  const s = S.org, i = s?.instance;
  if (a === 'new') {
    const games = S.games.filter(g => !g.beacon);
    const id = await KIT.choose('New organizer instance', games.map(g => ({ id: g.id, label: g.title, sub: SRC[g.source] || g.source })), 'Choose the game. Its mods go in their own folders; the game\'s folder isn\'t changed.');
    if (!id) return;
    try { const r = await org(['create', id]); await loadInsts(); S.section = 'organizer'; await openInst(r.created); } catch (e) { KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
    return;
  }
  if (a === 'import') {
    const dir = await pick({ folder: true, title: 'Choose a Mod Organizer 2 folder (with ModOrganizer.ini)' });
    if (!dir) return;
    try { const r = await org(['import', dir]); await loadInsts(); S.section = 'organizer'; await openInst(r.imported); if (r.notes?.length) KIT.alert('Imported', r.notes.map(esc).join('<br>')); } catch (e) { KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
    return;
  }
  if (!s) return;
  if (a === 'add') return orgInstall();
  if (a === 'sort') return orgDo(['sort'], 'Sorted');
  if (a === 'folder') return KIT.invoke('open_folder', { path: i.dir }).catch(e => KIT.toast(KIT.err(e)));
  if (a === 'deploy') return orgDo(['deploy'], 'The mods are in the game');
  if (a === 'undeploy') return orgDo(['undeploy'], 'The game is as it was');
  if (a === 'play') {
    const ex = s.executables.filter(e => e.present);
    let pickEx = ex[0]?.title;
    if (ex.length > 1) { pickEx = await KIT.choose('Play', ex.map(e => ({ id: e.title, label: e.title, sub: e.extender ? 'Script extender' : e.from === 'mo2' ? 'From Mod Organizer 2' : '' }))); if (!pickEx) return; }
    try { await KIT.invoke('organizer_run', { id: S.view.id, exe: pickEx || null }); KIT.toast('Starting with its mods…'); setTimeout(() => reloadView(), 4000); } catch (e) { KIT.alert('It didn\'t start', `<p>${esc(KIT.err(e)).replace(/\n/g, '<br>')}</p><p class="msg small">Every step is in Hearth's log: Settings › System › Send logs.</p>`, false); }
    return;
  }
  if (a === 'launcher') {
    const w = await KIT.choose('What Play starts', [{ id: 'hearth', label: 'Hearth Mod Organizer', sub: 'Native: the game\'s folder is laid over with the mods', end: i.launcher !== 'mo2' ? 'Now' : '' }, { id: 'mo2', label: 'Mod Organizer 2', sub: 'The Windows Mod Organizer 2 under Proton (the fallback)', end: i.launcher === 'mo2' ? 'Now' : '' }]);
    if (w) { await orgDo(['launcher', w]); await reloadView(); }
    return;
  }
  if (a === 'profile') {
    const items = s.profiles.map(p => ({ id: 'p:' + p.name, label: p.name, end: p.active ? 'Now' : '', sub: [p.localSaves ? 'Own saves' : '', p.localInis ? 'Own game settings' : ''].filter(Boolean).join(' · ') }));
    items.push({ id: 'new', label: 'New profile…', sub: 'A copy of this one' }, { id: 'saves', label: (s.profiles.find(p => p.active)?.localSaves ? 'Share saves with other profiles' : 'Keep this profile\'s own saves') }, { id: 'inis', label: (s.profiles.find(p => p.active)?.localInis ? 'Use the game\'s own settings' : 'Keep this profile\'s own game settings') });
    const c = await KIT.choose('Profiles', items);
    if (!c) return;
    const cur = s.profiles.find(p => p.active) || {};
    if (c.startsWith('p:')) return orgDo(['profile', c.slice(2)], `Profile: ${c.slice(2)}`);
    if (c === 'new') { const n = await ask('New profile', 'Its name', ''); if (n) return orgDo(['profile-create', n.trim(), '--from', i.profile]); return; }
    if (c === 'saves') return orgDo(['profile-settings', '--local-saves', cur.localSaves ? 'off' : 'on']);
    if (c === 'inis') return orgDo(['profile-settings', '--local-inis', cur.localInis ? 'off' : 'on']);
  }
}
/** Add to Steam: the game's ways in as Steam shortcuts with its art (Hearth Sync makes them; adding again updates). */
async function addToSteam() {
  const gid = S.view.game || S.view.id, g = game(gid); if (!g) return;
  let st;
  try { st = await KIT.invoke('steam_entries', { id: gid }); } catch (e) { return KIT.alert('Add to Steam', esc(KIT.err(e)), false); }
  const has = new Set(st.inSteam || []), now = k => has.has(k) ? 'In Steam' : '';
  const items = [{ id: 'modded', label: `${g.title} (Modded)`, sub: st.instance ? 'Starts the game with the organizer\'s mods laid over it' : 'Starts the game with its mods (through its script extender when it has one)', end: now('modded') }];
  if (st.instance) items.push({ id: 'hearth', label: `${g.title} — Hearth Mod Organizer`, sub: 'Hearth\'s own organizer: runs natively, lays the mods over the game while it runs', end: now('hearth') });
  if (st.instance && st.mo2) items.push({ id: 'mo2', label: `${g.title} — Mod Organizer 2`, sub: 'The original Mod Organizer 2, under Proton', end: now('mo2') });
  if (items.length > 1) items.push({ id: 'all', label: 'All of these', sub: `${items.length} shortcuts` });
  if (has.size) items.push({ id: 'remove', label: 'Take them out of Steam', sub: 'Hearth Sync removes the shortcuts' });
  const note = `Hearth Sync adds them to Steam's library with the game's pictures${st.hearthSync ? ', in Steam\'s Game Mode' : ' (in Steam\'s Game Mode, with Hearth Sync)'}. Adding again updates them, never twice.${st.instance ? '' : ' Mod Organizer entries come with an organizer instance for this game.'}`;
  const a = await KIT.choose('Add to Steam', items, note);
  if (!a) return;
  try {
    if (a === 'remove') { await KIT.invoke('remove_from_steam', { id: gid }); return KIT.toast('Taken out: Steam drops them at the next sync'); }
    const kinds = a === 'all' ? items.filter(x => ['modded', 'hearth', 'mo2'].includes(x.id)).map(x => x.id) : [a];
    const r = await KIT.invoke('add_to_steam', { id: gid, kinds });
    KIT.toast(`${plural(r.added.length, 'shortcut')} for Steam: ${r.added.length === 1 ? items.find(x => x.id === kinds[0]).label : g.title}`);
  } catch (e) { KIT.alert('Add to Steam', esc(KIT.err(e)), false); }
}

/** The game page's ⋯: the things you need now and then. */
async function gameMore() {
  if (S.view.type === 'inst') {
    const i = S.org.instance, g = game(S.view.game);
    const items = [{ id: 'steam', label: 'Add to Steam…', sub: 'Modded, Mod Organizer 2 and Hearth Mod Organizer, as Steam shortcuts' }, { id: 'profile', label: `Profile: ${i.profile}`, sub: 'Switch, make a new one, own saves and game settings' }];
    if (i.wabbajack) items.push({ id: 'launcher', label: `Play with: ${i.launcher === 'mo2' ? 'Mod Organizer 2' : 'Hearth'}`, sub: 'What Play modded starts' });
    if (i.plugins) items.push({ id: 'sort', label: 'Sort plugins', sub: 'Masters first, then by what each needs' });
    items.push({ id: 'folder', label: 'Open the instance\'s folder' });
    if (g?.install || i.gameDir) items.push({ id: 'gamefolder', label: 'Open the game folder' });
    const a = await KIT.choose(i.title, items);
    if (a === 'steam') return addToSteam();
    if (a === 'gamefolder') return KIT.invoke('open_folder', { path: g?.install || i.gameDir }).catch(e => KIT.toast(KIT.err(e)));
    if (a) return orgAction(a);
    return;
  }
  const g = game(S.view.id), s = S.sum;
  const items = [{ id: 'steam', label: 'Add to Steam…', sub: 'Play it modded from Steam\'s library' }, { id: 'deploy', label: `How mods go in: ${deployLabel(s)}`, sub: 'Links, copies or the overlay' }, { id: 'nexus', label: s.nexus ? `Nexus game: ${s.nexus}` : 'Which game is it on Nexus Mods?', sub: 'For downloads and update checks' }];
  if (s.mods.some(m => m.nexus)) items.push({ id: 'updates', label: 'Check for updates', sub: 'Asks Nexus Mods for newer files of your mods' });
  items.push({ id: 'game', label: 'Open the game folder' });
  if (s.mods.length) items.push({ id: 'store', label: 'Open Hearth\'s copies of the mods' });
  const a = await KIT.choose(g.title, items);
  if (a === 'steam') return addToSteam();
  if (a === 'deploy') return deployMode();
  if (a === 'nexus') {
    const v = await ask('Nexus game', 'The name in its Nexus address: nexusmods.com/<b>this part</b>/mods', s.nexus || '');
    if (v == null) return;
    try { S.sum = await KIT.invoke('set_nexus_game', { id: g.id, domain: v.trim().toLowerCase() }); render(); KIT.toast('Saved'); } catch (e) { KIT.alert('That didn\'t work', esc(KIT.err(e)), false); }
    return;
  }
  if (a === 'updates') return checkUpdates();
  if (a === 'game') return KIT.invoke('open_folder', { path: g.install }).catch(e => KIT.toast(KIT.err(e)));
  if (a === 'store') return KIT.invoke('open_folder', { path: s.folder + '/files' }).catch(e => KIT.toast(KIT.err(e)));
}

// ---------------------------------------------------------------- accounts
const ACCT_TEXT = {
  loverslab: 'Modlists with LoversLab files download them by themselves once you\'re signed in.',
  mega: 'MEGA files already download without an account, within MEGA\'s free transfer limit; signed in, your account\'s limit applies.',
  google: 'Google Drive files download without an account until Google says “quota exceeded”; signed in, Hearth then downloads them with your account.',
};
function acctPanel(a) {
  const wait = a.pending === 'waiting';
  const err = a.pending && !wait ? `<p class="warn small" style="margin:0">${esc(a.pending)}</p>` : '';
  const state = a.signedIn ? '<span class="pill good">Signed in</span>' : !a.registered ? '<span class="pill calm">In the browser</span>' : '<span class="pill calm">Not signed in</span>';
  let body;
  if (a.signedIn) body = `<p class="msg small" style="margin:0">${esc(a.who || '')}${a.since ? `, since ${esc(when(a.since))}` : ''}</p><div class="acts"><button class="btn danger f" data-key="ao:${a.id}" data-act="acctOut" data-p="${a.id}">Sign out</button></div>`;
  else if (!a.registered) body = `<p class="msg small" style="margin:0">Hearth isn't registered with ${esc(a.name)} yet, so it can't sign in there. Until then Hearth opens each file's page in Hearth Browser for you to press Download, and carries on by itself.</p>`;
  else if (a.id === 'mega') body = `<p class="msg small" style="margin:0">Hearth keeps only MEGA's login for the engine (never your password). Accounts with two-step sign-in can't be used by the engine.</p><div class="acts"><button class="btn primary f" data-key="ai:mega" data-act="megaIn">Sign in to MEGA</button></div>`;
  else body = `<p class="msg small" style="margin:0">${wait ? 'Finish signing in in the browser window that opened…' : `Sign in yourself in the browser window that opens; ${esc(a.name)} sends you back to Hearth.`}</p>
      <div class="acts"><button class="btn primary f" data-key="ai:${a.id}" data-act="acctBegin" data-p="${a.id}" ${wait ? 'disabled' : ''}>${wait ? 'Waiting for the browser…' : `Sign in to ${esc(a.name)}`}</button></div>`;
  return `<div class="acc"><h2>${esc(a.name)}${state}</h2><p class="use">${ACCT_TEXT[a.id] || ''}</p>${err}${body}</div>`;
}
function nexusView() {
  const a = S.acct;
  if (a === undefined) return '<div class="acc main"><h2>Nexus Mods</h2><p class="msg">Reading…</p></div>';
  if (!a) return `<div class="acc main"><h2>Nexus Mods<span class="pill warn">Not signed in</span></h2>
      <p class="use">Nearly every mod and modlist comes from Nexus Mods. Hearth Mods signs in with your own API key: it stays on this PC (only you can read it) and is only sent to Nexus Mods.</p>
      <ol><li>Open your API key page (sign in to Nexus Mods there).</li><li>At the bottom, under <b>Personal API Key</b>, press Request (or copy the key you have).</li><li>Paste it here.</li></ol>
      <div class="acts"><button class="btn primary f" data-key="ki" data-act="keyIn">Paste my key</button><button class="btn f" data-key="kp" data-act="keyPage">${I.globe}Open my API key page</button></div>
      <p class="use">With a <b>free</b> account, each file is downloaded by pressing <b>Mod Manager Download</b> on the site (Nexus' rule). With <b>Premium</b>, Hearth downloads files by itself, which also makes Wabbajack modlists install without clicking.</p></div>`;
  const q = S.dls.quota || {};
  return `<div class="acc main"><h2>Nexus Mods<span class="pill ${a.premium ? 'good' : 'calm'}">${a.premium ? 'Premium' : 'Free account'}</span></h2>
      <p class="use">Signed in as <b>${esc(a.name)}</b>. ${a.premium ? 'Hearth downloads files by itself, and Wabbajack modlists install without clicking.' : 'Files come by pressing “Mod Manager Download” on the site; a Wabbajack modlist lists each file to press it for.'}</p>
      <div class="quota"><span>Checked ${esc(when(a.checked))}</span>${q.hourly != null ? `<span><b>${q.hourly}</b> requests left this hour</span><span><b>${q.daily}</b> today</span>` : ''}</div>
      <div class="acts"><button class="btn f" data-key="nr" data-act="nexusRefresh">Check again</button><button class="btn f" data-key="ns" data-act="nexusSite">${I.globe}Open Nexus Mods</button><button class="btn danger f" data-key="no" data-act="nexusOut">Sign out</button></div></div>`;
}

// ---------------------------------------------------------------- without the app (the page opened in a browser): sample games
// Pictures are drawn here (the preview has no art cache); everything else answers like the Rust side would.
const DEMO = (() => {
  const P = { 1091500: ['#0d1b3d', '#f3e600', '#ff2a6d', 'CYBERPUNK'], 489830: ['#1b2836', '#dfe8ef', '#5f7f99', 'SKYRIM'], 413150: ['#6cc4e8', '#fff3b0', '#4f8a32', 'STARDEW'], 1086940: ['#2a1436', '#e2b33c', '#80308f', "BALDUR'S GATE"],
    377160: ['#33321a', '#e5cf86', '#6c7a36', 'FALLOUT 4'], 367520: ['#0f1726', '#b8d0f0', '#41608a', 'HOLLOW KNIGHT'], 1145360: ['#2a0b0b', '#ff7a45', '#7b1c1c', 'HADES'], bfme2: ['#2a1a0e', '#e8a33d', '#5a3a1e', 'MIDDLE-EARTH'] };
  const rnd = s => { let h = 2166136261; for (const c of String(s)) { h ^= c.charCodeAt(0); h = Math.imul(h, 16777619); } return () => { h ^= h << 13; h ^= h >>> 17; h ^= h << 5; return ((h >>> 0) % 10000) / 10000; }; };
  const ridge = (r, w, h, base, amp, n) => { let d = `M0 ${h} L0 ${base}`; for (let i = 0; i <= n; i++) d += ` L${(w / n * i).toFixed(0)} ${(base - r() * amp).toFixed(0)}`; return d + ` L${w} ${h} Z`; };
  const mix = (a, b, t) => { const p = x => [1, 3, 5].map(i => parseInt(x.slice(i, i + 2), 16)); const A = p(a), B = p(b); return '#' + A.map((v, i) => Math.round(v + (B[i] - v) * t).toString(16).padStart(2, '0')).join(''); };
  function art(key, kind, salt = '') {
    const [c0, c1, c2, name] = P[key] || ['#222833', '#9fb3c8', '#47566b', 'GAME'];
    const r = rnd(key + kind + salt), cover = kind === 'cover', w = cover ? 600 : 1920, h = cover ? 900 : 620;
    const sunX = (cover ? .5 : .68 + r() * .1) * w, sunY = h * (cover ? .3 : .36);
    let s = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" preserveAspectRatio="xMidYMid slice"><defs><linearGradient id="s" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${mix(c0, '#000000', .35)}"/><stop offset=".7" stop-color="${mix(c0, c2, .55)}"/><stop offset="1" stop-color="${mix(c2, c1, .3)}"/></linearGradient>
      <radialGradient id="g"><stop offset="0" stop-color="${c1}" stop-opacity=".9"/><stop offset=".25" stop-color="${c1}" stop-opacity=".35"/><stop offset="1" stop-color="${c1}" stop-opacity="0"/></radialGradient></defs>
      <rect width="${w}" height="${h}" fill="url(#s)"/><circle cx="${sunX}" cy="${sunY}" r="${h * .55}" fill="url(#g)"/><circle cx="${sunX}" cy="${sunY}" r="${h * .07}" fill="${c1}" opacity=".95"/>`;
    for (let i = 0; i < 40; i++) s += `<circle cx="${(r() * w).toFixed(0)}" cy="${(r() * h * .45).toFixed(0)}" r="${(r() * 1.6 + .3).toFixed(1)}" fill="#fff" opacity="${(r() * .5).toFixed(2)}"/>`;
    [[.62, .22, 9, .35], [.74, .16, 14, .6], [.86, .1, 22, .85]].forEach(([b, a, n, t]) => { s += `<path d="${ridge(r, w, h, h * b, h * a, n)}" fill="${mix(mix(c0, c2, .5), '#05070a', t)}"/>`; });
    if (cover) s += `<text x="300" y="790" text-anchor="middle" font-family="Georgia, serif" font-weight="700" font-size="${name.length > 10 ? 50 : 66}" fill="#fff" letter-spacing="4">${name.replace(/&/g, '&amp;').replace(/'/g, '&#39;')}</text>`;
    return 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(s + '</svg>');
  }
  const H = '/home/you/.local/share/Steam/steamapps/common/';
  const g = (app, title, extra = {}) => ({ id: `steam:${app}`, title, source: 'steam', steamApp: app, install: H + title.replace(/[:']/g, ''), art: { cover: 'x', hero: 'x', wide: 'x' }, mods: 0, modsOn: 0, ...extra });
  const games = [g(1091500, 'Cyberpunk 2077', { mods: 10, modsOn: 9 }), g(489830, 'The Elder Scrolls V: Skyrim Special Edition'), g(413150, 'Stardew Valley', { mods: 5, modsOn: 4 }), g(1086940, "Baldur's Gate 3", { mods: 3, modsOn: 3 }),
    g(377160, 'Fallout 4'), g(367520, 'Hollow Knight'), g(1145360, 'Hades'), { id: 'beacon:bfme2', title: 'The Battle for Middle-earth II', source: 'beacon', beacon: true, install: '/home/you/Games/BFME2', art: { cover: 'x', hero: 'x', wide: 'x' } }];
  games.sort((a, b) => a.title.localeCompare(b.title));
  const day = 864e5, now = Date.now();
  const mk = (id, name, o) => Object.assign({ id, name, enabled: true, target: 'game', sub: 'archive/pc/mod', targetLabel: 'Game folder', files: 4, size: 2e6, added: now - 9 * day, overriddenBy: [], beats: [], winning: 4, source: `/home/you/Downloads/${name.replace(/ /g, '_')}.zip` }, o);
  const cp = [mk('m0', 'Cyber Engine Tweaks', { files: 140, winning: 140, size: 9.3e6, sub: 'bin/x64', nexus: { domain: 'cyberpunk2077', modId: 107, fileId: 1, version: '1.32.1' }, note: 'Needed by almost everything else here. Update before the game patches.' }),
    mk('m1', 'RED4ext', { files: 12, winning: 12, size: 1.1e6, sub: '', nexus: { domain: 'cyberpunk2077', modId: 2380, fileId: 2, version: '1.25' } }),
    mk('m2', 'ArchiveXL', { files: 6, winning: 6, size: 640e3, sub: 'red4ext/plugins', nexus: { domain: 'cyberpunk2077', modId: 4198, fileId: 3, version: '1.16' } }),
    mk('m3', 'Codeware', { files: 4, winning: 4, size: 2.4e6, sub: 'red4ext/plugins', nexus: { domain: 'cyberpunk2077', modId: 7780, fileId: 4, version: '1.12' } }),
    mk('m4', 'Nova City Lighting', { files: 310, winning: 262, size: 1.2e9, overriddenBy: ['Weathers of Night City'], nexus: { domain: 'cyberpunk2077', modId: 9112, fileId: 5, version: '2.1' } }),
    mk('m5', 'Weathers of Night City', { files: 48, winning: 48, size: 380e6, beats: ['Nova City Lighting'] }),
    mk('m6', 'Immersive First Person', { files: 12, winning: 0, size: 3.1e6, overriddenBy: ['Appearance Menu Mod'] }),
    mk('m7', 'Appearance Menu Mod', { files: 902, winning: 902, size: 610e6, beats: ['Immersive First Person'], nexus: { domain: 'cyberpunk2077', modId: 790, fileId: 6, version: '2.6' } }),
    mk('m8', 'Better Apartment Views', { enabled: false, files: 18, winning: 0, size: 44e6 }),
    mk('m9', 'Kiroshi Crowd Scanner', { files: 9, winning: 9, size: 1.8e6, added: now - 2 * day })];
  const sdv = [mk('s0', 'SMAPI', { files: 31, winning: 31, sub: '', target: 'game' }), mk('s1', 'Content Patcher', { sub: 'Mods/ContentPatcher', files: 7, winning: 7 }), mk('s2', 'Stardew Valley Expanded', { sub: 'Mods/SVE', files: 2200, winning: 2200, size: 760e6 }),
    mk('s3', 'Lookup Anything', { sub: 'Mods/LookupAnything', files: 12, winning: 12 }), mk('s4', 'Seasonal Outfits', { enabled: false, sub: 'Mods/SeasonalOutfits', files: 140, winning: 0, size: 60e6 })];
  const bg3 = [mk('b0', 'Script Extender', { files: 2, winning: 2, sub: 'bin' }), mk('b1', 'ImprovedUI', { files: 30, winning: 30, target: 'prefix-local', targetLabel: 'AppData › Local (in the game\'s Proton folder)', sub: 'Larian Studios/Baldur\'s Gate 3/Mods' }), mk('b2', 'Party Limit Begone', { files: 3, winning: 3, target: 'prefix-local', targetLabel: 'AppData › Local (in the game\'s Proton folder)', sub: 'Larian Studios/Baldur\'s Gate 3/Mods' })];
  const lists = { 'steam:1091500': cp, 'steam:413150': sdv, 'steam:1086940': bg3 };
  const sum = id => { const gm = games.find(x => x.id === id); const mods = (lists[id] || []).map((m, i) => ({ ...m, order: i, root: `${gm.install}/${m.sub}` }));
    return { game: id, title: gm.title, deploy: id === 'steam:1086940' ? 'copy' : 'hardlink', deployedAt: mods.length ? now - 3.6e6 * 2 : 0, nexus: id === 'steam:1091500' ? 'cyberpunk2077' : '', folder: `/home/you/.local/share/nimbus/mods/${id}`,
      targets: [{ id: 'game', label: 'Game folder', path: gm.install }, { id: 'prefix-documents', label: 'Documents (in the game\'s Proton folder)', path: '/pfx/drive_c/users/steamuser/Documents' }], mods, overlay: null, modes: [{ id: 'hardlink', label: 'Links' }, { id: 'copy', label: 'Copies' }, { id: 'overlay', label: 'Overlay (Beta Experimental)' }] }; };
  const om = (id, o) => Object.assign({ id, name: id, kind: 'mod', enabled: true, files: 20, version: '', plugins: [], conflicts: { winsOver: [], losesTo: [], winning: 20, losing: 0 }, installedFrom: `${id}.7z` }, o);
  const sep = n => ({ id: n + '_separator', name: n, kind: 'separator', enabled: true });
  let sky = [sep('Essentials'), om('Address Library for SKSE Plugins', { files: 2, version: '11' }), om('SkyUI', { files: 68, version: '5.2SE', plugins: ['SkyUI_SE.esp'], nexusId: 12604 }), om('Unofficial Skyrim Special Edition Patch', { files: 9, version: '4.3.2', plugins: ['unofficial skyrim special edition patch.esp'] }),
    sep('Visuals'), om('Skyrim 2020 Parallax', { files: 1240, version: '9.1', conflicts: { winsOver: [], losesTo: ['Noble Skyrim'], winning: 930, losing: 310 } }), om('Noble Skyrim', { files: 880, version: '5.1', conflicts: { winsOver: ['Skyrim 2020 Parallax'], losesTo: [], winning: 880, losing: 0 } }),
    om('Cathedral Weathers', { files: 34, version: '2.4', plugins: ['Cathedral Weathers.esp'] }), om('Obsidian Mountain Fogs', { enabled: false, files: 6 }),
    sep('Gameplay'), om('Ordinator - Perks of Skyrim', { files: 12, version: '9.31', plugins: ['Ordinator - Perks of Skyrim.esp'] }), om('Wildcat - Combat of Skyrim', { files: 4, version: '3.0', plugins: ['Wildcat - Combat of Skyrim.esp'] }), om('Immersive Armors', { enabled: false, files: 410, version: '8.1', plugins: ['Hothtrooper44_ArmorCompilation.esp'] })];
  const orgSum = () => ({ ok: true, instance: { id: 'skyrimse', title: 'Skyrim Special Edition', game: 'skyrimse', gameName: 'Skyrim Special Edition', gameDir: games.find(x => x.id === 'steam:489830').install, profile: 'Survival', plugins: true, launcher: 'hearth', dir: '/home/you/.local/share/nimbus/organizer/skyrimse', modsDir: '/home/you/.local/share/nimbus/organizer/skyrimse/mods', overwriteDir: '/x/overwrite', libraryId: 'steam:489830' },
    profiles: [{ name: 'Default', active: false }, { name: 'Survival', active: true, localSaves: true }], warnings: [], deployed: null, executables: [{ title: 'SKSE', present: true, extender: true }, { title: 'Skyrim Special Edition', present: true }],
    mods: [...sky.map((m, i) => ({ ...m, priority: i })), { id: 'Overwrite', name: 'Overwrite', kind: 'overwrite', priority: sky.length, enabled: true, files: 3, conflicts: { winsOver: [], losesTo: [], winning: 3, losing: 0 } }],
    plugins: [['Skyrim.esm', '', 1], ['Update.esm', '', 1], ['Dawnguard.esm', '', 1], ['HearthFires.esm', '', 1], ['Dragonborn.esm', '', 1], ['unofficial skyrim special edition patch.esp', 'Unofficial Skyrim Special Edition Patch'], ['SkyUI_SE.esp', 'SkyUI'], ['Cathedral Weathers.esp', 'Cathedral Weathers'], ['Ordinator - Perks of Skyrim.esp', 'Ordinator - Perks of Skyrim'], ['Wildcat - Combat of Skyrim.esp', 'Wildcat - Combat of Skyrim'], ['Hothtrooper44_ArmorCompilation.esp', 'Immersive Armors', 0, 1]]
      .map(([name, mod, imp, off], i) => ({ name, mod, implicit: !!imp, master: name.endsWith('.esm'), active: !off, loadIndex: off ? '' : i.toString(16).toUpperCase().padStart(2, '0'), masters: imp ? [] : ['Skyrim.esm', 'Update.esm'], missing: [] })) });
  const insts = () => [{ id: 'skyrimse', title: 'Skyrim Special Edition', libraryId: 'steam:489830', gameName: 'Skyrim Special Edition', gameDir: games.find(x => x.id === 'steam:489830').install, profile: 'Survival', mods: sky.filter(m => m.kind === 'mod').length, modsOn: sky.filter(m => m.kind === 'mod' && m.enabled).length },
    { id: 'tuxborn', title: 'Tuxborn', libraryId: '', wabbajack: true, gameName: 'Skyrim Special Edition', gameDir: games.find(x => x.id === 'steam:489830').install, profile: 'Tuxborn', mods: 1412, modsOn: 1398, deployed: null }];
  const gallery = [['Tuxborn', 'Omni', 'SkyrimSpecialEdition', 38e9, 61e9, 1, 'A Steam Deck friendly Skyrim: hundreds of mods tuned to run well on a handheld, with a controller-first interface.'], ['Magnum Opus', 'Tuxborn', 'Fallout4', 140e9, 220e9, 1, 'A big, beautiful Fallout 4 with reworked combat, settlements and hundreds of quality fixes.'],
    ['Viva New Vegas', 'Vivanewvegas', 'FalloutNewVegas', 15e9, 30e9], ['Living Skyrim', 'Ronnie', 'SkyrimSpecialEdition', 92e9, 140e9], ['Nordic Souls', 'Wotan', 'SkyrimSpecialEdition', 210e9, 330e9], ['Wasteland Reborn', 'Lexy', 'Fallout4', 66e9, 98e9],
    ['Elysium Remastered', 'Ely', 'SkyrimSpecialEdition', 120e9, 180e9], ['Tale of Two Wastelands', 'TTW team', 'FalloutNewVegas', 22e9, 41e9], ['Night City Nights', 'Aria', 'Cyberpunk2077', 31e9, 48e9], ['Lorerim', 'Ilyay', 'SkyrimSpecialEdition', 160e9, 250e9, 0, '', 1]]
    .map(([title, author, game, dl, ins, off, desc, down], i) => ({ id: `${author}/${title}`.replace(/ /g, ''), title, author, game, gameName: WJ_NAMES[game] || game, download: dl, install: ins, official: !!off, description: desc || '', archives: 200 + i * 87, version: `${1 + i % 4}.${i}`,
      image: art(['489830', '377160', '367520', '489830', '1145360', '377160', '1086940', '367520', '1091500', '413150'][i], 'hero', title), down: !!down }));
  const job = { id: 'w1', title: 'Tuxborn', list: 'Omni/Tuxborn', state: 'installing', running: true, phase: 'Downloading mod archives', status: '[00:21:10] Downloading Mod Archives (128/214) - 18.4MB/s', step: [3, 14], count: [128, 214], installDir: '/home/you/Modlists/Tuxborn', premiumNeeded: true,
    manual: [{ fileName: 'skse64_2_02_06.7z', modName: 'SKSE64', size: 700e3, nexus: true, here: true, url: 'https://www.nexusmods.com/skyrimspecialedition/mods/30379' },
      { fileName: 'Noble Skyrim 2K.7z', modName: 'Noble Skyrim', size: 2.1e9, nexus: true, here: false, url: 'https://www.nexusmods.com/skyrimspecialedition/mods/21423' },
      { fileName: 'ENB Series v0503.zip', modName: 'ENB binaries', size: 4e6, nexus: false, here: false, url: 'https://enbdev.com/', site: 'enbdev.com' },
      { fileName: 'Community Shaders.7z', modName: 'Community Shaders', size: 12e6, nexus: true, here: false, url: 'https://www.nexusmods.com/skyrimspecialedition/mods/86492' },
      { fileName: 'Tuxborn Textures.7z', modName: 'Tuxborn texture pack', size: 1.4e9, nexus: false, here: false, url: 'https://mega.nz/file/x', site: 'MEGA' },
      { fileName: 'Old Patch.7z', modName: 'An old patch', size: 3e6, nexus: false, here: false, skipped: true, url: 'https://example.org/patch', site: 'example.org' }],
    guide: { total: 2, done: 0, label: '2 files', item: { fileName: 'ENB Series v0503.zip', modName: 'ENB binaries', size: 4e6, site: 'enbdev.com', url: 'https://enbdev.com/' } } };
  const dls = { active: 2, quota: { hourly: 97, daily: 19890 }, notices: [], items: [
    { id: 'd1', name: 'Noble Skyrim 2K', fileName: 'Noble Skyrim 2K.7z', size: 2.1e9, done: 1.27e9, speed: 18.4e6, state: 'downloading', job: 'w1', path: '/home/you/Modlists/Downloads/Tuxborn/Noble Skyrim 2K.7z' },
    { id: 'd5', name: 'Community Shaders', fileName: 'Community Shaders.7z', size: 12e6, done: 0, state: 'needs-link', job: 'w1', path: '/x', page: 'https://www.nexusmods.com/skyrimspecialedition/mods/86492?tab=files' },
    { id: 'd2', name: 'Appearance Menu Mod 2.7', fileName: 'AppearanceMenuMod-790-2-7.zip', size: 640e6, done: 210e6, speed: 6.2e6, state: 'downloading', path: '/home/you/.local/share/nimbus/mods/downloads/cyberpunk2077/AMM.zip' },
    { id: 'd6', name: 'Weathers of Night City 1.4', fileName: 'WONC-1.4.7z', size: 380e6, done: 120e6, state: 'paused', path: '/x' },
    { id: 'd3', name: 'Kiroshi Crowd Scanner', fileName: 'KiroshiCrowdScanner.zip', size: 1.8e6, done: 1.8e6, state: 'done', path: '/home/you/.local/share/nimbus/mods/downloads/cyberpunk2077/Kiroshi.zip' },
    { id: 'd4', name: 'Unofficial Skyrim SE Patch', fileName: 'USSEP.7z', size: 81e6, done: 81e6, state: 'done', path: '/home/you/.local/share/nimbus/mods/downloads/skyrimspecialedition/USSEP.7z' }] };
  return (cmd, a) => {
    if (cmd === 'info') return { version: '1.0 preview' };
    if (cmd === 'list') return games.map(x => ({ ...x, mods: (lists[x.id] || []).length, modsOn: (lists[x.id] || []).filter(m => m.enabled).length }));
    if (cmd === 'art') return art(String(a.id).split(':')[1], a.kind);
    if (cmd === 'mods_of') { if (a.id === 'steam:489830') { const s = sum(a.id); s.organizer = { id: 'skyrimse', profile: 'Survival' }; return s; } return sum(a.id); }
    if (cmd === 'toggle') { const m = lists[a.id].find(x => x.id === a.m); m.enabled = a.on; return { report: {}, summary: sum(a.id) }; }
    if (cmd === 'move_mod') { const L = lists[a.id], i = L.findIndex(x => x.id === a.m), j = Math.max(0, Math.min(L.length - 1, i + a.by)); L.splice(j, 0, L.splice(i, 1)[0]); return { report: {}, summary: sum(a.id) }; }
    if (cmd === 'set_note') { lists[a.id].find(x => x.id === a.m).note = a.note.trim(); return sum(a.id); }
    if (cmd === 'mod_conflicts') { const m = lists[a.id].find(x => x.id === a.m); const f = [];
      for (const o of m.beats) for (let i = 0; i < 6; i++) f.push({ path: `archive/pc/mod/${o.toLowerCase().replace(/ /g, '_')}_${i}.archive`, status: 'wins', over: [o] });
      for (const o of m.overriddenBy) for (let i = 0; i < 6; i++) f.push({ path: `archive/pc/mod/${m.name.toLowerCase().replace(/ /g, '_')}_${i}.archive`, status: 'loses', to: o });
      return { files: f, wins: m.beats.length * 6, loses: m.overriddenBy.length * 6 }; }
    if (cmd === 'check_updates') return { checked: 6, premium: false, updates: [{ mod: 'm7', name: 'Appearance Menu Mod', domain: 'cyberpunk2077', modId: 790, newer: { fileId: 9, name: 'Appearance Menu Mod', version: '2.7' }, page: 'https://www.nexusmods.com/cyberpunk2077/mods/790?tab=files' }, { mod: 'm0', name: 'Cyber Engine Tweaks', domain: 'cyberpunk2077', modId: 107, newer: { fileId: 8, name: 'Cyber Engine Tweaks', version: '1.33.0' }, page: 'https://www.nexusmods.com/cyberpunk2077/mods/107?tab=files' }] };
    if (cmd === 'nexus_mod') return { name: 'x', summary: a.modId === 790 ? 'Change NPC appearances, spawn companions and props, take photos with full control of the scene. A must for photo mode.' : 'A scripting framework other mods are built on: an in-game console, hotkeys and a Lua API.', author: a.modId === 790 ? 'MxOdin and CtrlAltDeletus' : 'yamashi and the CET team', version: a.modId === 790 ? '2.7' : '1.33.0', endorsements: a.modId === 790 ? 61240 : 88310, downloads: a.modId === 790 ? 2914000 : 6120000, picture: art(a.modId === 790 ? '1086940' : '367520', 'hero', String(a.modId)) };
    if (cmd === 'steam_entries') return { inSteam: a.id === 'steam:489830' ? ['modded'] : [], instance: a.id === 'steam:489830' ? 'skyrimse' : null, mo2: a.id === 'steam:489830', hearthSync: true };
    if (cmd === 'add_to_steam') return { added: a.kinds.map(k => `hearth-mods:${k}:${a.id}`) };
    if (cmd === 'remove_from_steam') return { removed: 1 };
    if (cmd === 'play') return { started: true };
    if (cmd === 'nexus_account') return { name: 'Dragongamer9', premium: false, checked: now - 6e5 };
    if (cmd === 'downloads_list') return dls;
    if (cmd === 'accounts_list') return [{ id: 'loverslab', name: 'LoversLab', signedIn: false, registered: false }, { id: 'mega', name: 'MEGA', signedIn: true, registered: true, who: 'you@example.com', since: now - day }, { id: 'google', name: 'Google', signedIn: false, registered: true }];
    if (cmd === 'wj_engine') return { path: '/usr/lib/hearth/jackify-engine/jackify-engine', version: 'v0.5.2', system: true };
    if (cmd === 'wj_jobs') return [job];
    if (cmd === 'wj_gallery') return gallery;
    if (cmd === 'wj_defaults') return { gameDir: games.find(x => x.id === 'steam:489830').install, gameTitle: 'The Elder Scrolls V: Skyrim Special Edition', known: true, installDir: '/home/you/Modlists/' + a.title, downloadsDir: '/home/you/Modlists/Downloads/' + a.title, free: 480e9 };
    if (cmd === 'places') return [{ label: 'Home', path: '/home/you' }, { label: 'Downloads', path: '/home/you/Downloads' }];
    if (cmd === 'browse') return { path: a.path, parent: '/home/you', entries: [{ name: 'Mods', path: a.path + '/Mods', dir: true }, { name: 'SkyUI_5_2_SE.7z', path: a.path + '/SkyUI_5_2_SE.7z', dir: false, archive: true, size: 5_300_000 }] };
    if (cmd === 'organizer') {
      const [id, sub, x, , by] = a.args;
      if (id === 'instances') return { ok: true, instances: insts() };
      if (sub === 'enable' || sub === 'disable') { sky.find(m => m.id === x).enabled = sub === 'enable'; return { ok: true, summary: orgSum() }; }
      if (sub === 'move') { const i = sky.findIndex(m => m.id === x), j = Math.max(0, Math.min(sky.length - 1, i + Number(by))); sky.splice(j, 0, sky.splice(i, 1)[0]); return { ok: true, summary: orgSum() }; }
      if (sub === 'conflicts') { const m = orgSum().mods.find(q => q.id === x); const f = []; (m.conflicts?.winsOver || []).forEach(o => { for (let i = 0; i < 8; i++) f.push({ path: `textures/landscape/rock0${i}.dds`, status: 'wins', over: [o] }); }); (m.conflicts?.losesTo || []).forEach(o => { for (let i = 0; i < 8; i++) f.push({ path: `textures/landscape/mountain0${i}.dds`, status: 'loses', to: o }); }); return { ok: true, files: f }; }
      return orgSum();
    }
    if (cmd === 'stage') return { token: 's1', name: 'SkyUI_5_2_SE', files: 4, size: 5_300_000, top: ['Data/', 'readme.txt'], unwrapped: null, suggestTarget: 'game', suggestSub: '' };
    throw 'This is a preview: open Hearth Mods to do that.';
  };
})();
KIT.demo = DEMO;

(async () => {
  try { S.info = await KIT.invoke('info'); } catch (e) {}
  loadAccount(); loadAccounts();
  await load();
  // nxm:// links that opened Hearth Mods land on Downloads; the queue is watched from the start for their notices.
  const pages = { games: 'library', nexus: 'accounts', accounts: 'accounts', downloads: 'downloads', modlists: 'modlists', organizer: 'organizer', settings: 'settings' };
  const p = pages[S.info.page];
  if (p && p !== 'library') setSection(p);
  else { if (!S.view) KIT.focusFirst('.gcard'); refreshDownloads(); refreshJobs(); }
})();
