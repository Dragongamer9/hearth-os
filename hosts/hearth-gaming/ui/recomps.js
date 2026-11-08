// Hearth Games › Recomps (docs/RECOMPS.md, docs/HEARTH-GAMES.md): native PC ports of console games. The engine is
// Hearth Recomps in hearth-core (recomps.rs), reached through the window's one Bridge (its recomps.* calls, allowed in
// src/gamestore.rs CALLS): the catalogue, installs from each project's GitHub releases, checking your copy of the game,
// updates and uninstall. Installed ports are in Hearth's library as recomp:<id>, so Hearth Sync keeps them in Steam
// Game Mode and Play starts them the same way. Hearth never downloads, hosts or links to games, ROMs or disc images.
// games.js draws the tab with recompsHtml() and hands it clicks (rcClick) and the engine's "recomps" events (rcEvent).
'use strict';
// call(method, args) is games.js': one of the Bridge calls src/gamestore.rs allows.

const RC = {
  data: null, err: '', loading: false, con: 'all', show: 'all', q: '',
  det: {}, derr: {}, dbusy: {}, page: null, more: false,
};
const RC_KIND = { recomp: 'Recompilation', decomp: 'Decompilation port', port: 'Fan port', custom: 'Added by you' };
const rcAll = () => (RC.data && RC.data.recomps) || [];
const rcGet = id => rcAll().find(e => e.id === id);
const rcCon = k => ((RC.data && RC.data.consoles) || {})[k] || (k === 'other' ? 'Other' : k || '');
const rcEsc = s => KIT.esc(s);
/** Which build this PC gets: 'linux', 'windows' (through Proton), 'none', or null (known once its page has asked GitHub). */
function rcBuild(e) {
  if (e.platform) return e.platform;
  const d = RC.det[e.id]; if (d) return d.pick ? d.pick.platform : 'none';
  return e.linux === true ? 'linux' : e.linux === false ? 'windows' : null;
}
/** The port's page is what the dialog layer shows (not a choice or alert that took its place for a moment). */
const rcPageOpen = () => !!KIT.$('#kit-layer #rc-page');
const rcBuildLabel = b => b === 'linux' ? 'The Linux build' : b === 'windows' ? 'The Windows build, through Proton' : b === 'none' ? 'No build for this PC in the newest release' : 'Shown once Hearth has asked GitHub';

// ---------------------------------------------------------------- loading
async function rcLoad() {
  if (RC.loading) return;
  RC.loading = true;
  try { RC.data = await call('recomps.list'); RC.err = ''; } catch (e) { RC.err = KIT.err(e); }
  RC.loading = false;
  if (S.tab === 'recomps') { if (rcPageOpen()) rcRedraw(); else if (!KIT.dialogOpen()) render(); }
}
async function rcDetails(id, fresh = false) {
  if (RC.dbusy[id] || (RC.det[id] && !fresh)) return;
  RC.dbusy[id] = true; RC.derr[id] = '';
  try { RC.det[id] = await call('recomps.details', { id, fresh }); } catch (e) { RC.derr[id] = KIT.err(e); }
  RC.dbusy[id] = false;
  if (RC.page === id && rcPageOpen()) rcRedraw();
}
/** The engine's "recomps" events: a job's stage and progress (installing, checking your copy), or updates found. */
function rcEvent(d) {
  if (!d) return;
  if (d.updates) { rcLoad(); return; }
  const e = rcGet(d.id); if (!e) return;
  const before = e.job || {};
  e.job = d.job;
  const j = d.job || {};
  if (!j.busy && before.busy) {
    if (j.error) KIT.toast(`${e.title}: ${j.error}`);
    else if (j.done) KIT.toast(`${e.title} ${j.version ? j.version + ' ' : ''}is installed`);
    else if (j.game) KIT.toast(j.game.verified === true ? 'Your copy is the right version' : j.game.verified === false ? 'Using your copy anyway' : 'Your copy is set up (the project gives no hash to check it against)');
    rcLoad();
  }
  if (RC.page === d.id && rcPageOpen()) {
    // Only the progress moved: the bar and its words change in place (the page isn't drawn again under the person).
    if (j.busy && before.busy && j.stage === before.stage) {
      const bar = KIT.$('#rc-bar i'); if (bar && j.progress != null) bar.style.width = `${Math.round(j.progress * 100)}%`;
      const pc = KIT.$('#rc-pct'); if (pc) pc.textContent = j.progress != null ? `${Math.round(j.progress * 100)}%` : '';
    } else rcRedraw();
  } else if (S.tab === 'recomps' && !KIT.dialogOpen()) {
    const t = document.querySelector(`.tile[data-rc="${CSS.escape(d.id)}"] .s`);
    if (t) t.innerHTML = rcTileState(e); else render();
  }
}

// ---------------------------------------------------------------- the list
function rcShown() {
  const q = RC.q.trim().toLowerCase();
  let L = rcAll().filter(e => !e.hidden);
  if (RC.con !== 'all') L = L.filter(e => e.console === RC.con);
  if (RC.show === 'installed') L = L.filter(e => e.installed || (e.job && e.job.busy));
  if (RC.show === 'updates') L = L.filter(e => e.update);
  if (q) L = L.filter(e => [e.title, e.game, rcCon(e.console), e.repo].join(' ').toLowerCase().includes(q));
  const az = (a, b) => a.title.localeCompare(b.title);
  return L.sort((a, b) => (!!b.update - !!a.update) || (!!b.installed - !!a.installed) || (!!b.featured - !!a.featured) || (!!a.experimental - !!b.experimental) || az(a, b));
}
function rcTileState(e) {
  const j = e.job || {};
  const st = j.busy ? `<span class="badge">${j.progress != null ? Math.round(j.progress * 100) + '%' : 'Working…'}</span>`
    : e.update ? '<span class="badge sky">Update</span>' : e.installed ? '<span class="badge good">Installed</span>' : '';
  return `<span>${rcEsc(rcCon(e.console))}</span>${st}`;
}
function rcTile(e) {
  const j = e.job || {};
  return `<button class="tile rc f ${e.installed || j.busy ? '' : 'dim'}" data-rc="${rcEsc(e.id)}">
    <span class="cover">${e.icon ? `<img loading="lazy" src="${rcEsc(e.icon)}" alt="">` : `<span class="letter">${rcEsc((e.title || '?')[0])}</span>`}
      ${j.busy ? `<span class="pbar"><i style="width:${Math.round((j.progress || 0) * 100)}%"></i></span>` : ''}</span>
    <span class="t">${rcEsc(e.title)}</span><span class="s">${rcTileState(e)}</span></button>`;
}
function rcGrid() {
  const L = rcShown();
  return L.map(rcTile).join('') || `<div class="empty" style="grid-column:1/-1">${RC.q ? 'No port by that name.' : RC.show === 'updates' ? 'Every installed port is up to date.' : RC.show === 'installed' ? 'No ports installed yet.' : 'Nothing here.'}</div>`;
}
function recompsHtml() {
  if (!RC.data) {
    if (!RC.loading && !RC.err) rcLoad();
    return RC.err ? `<div class="empty"><p class="bad">Couldn't load Hearth Recomps: ${rcEsc(RC.err)}</p><button class="btn f" data-rca="retry">Try again</button></div>`
      : '<div class="empty"><div class="prog spin"><i></i></div>Loading Recomps…</div>';
  }
  const all = rcAll().filter(e => !e.hidden), n = f => all.filter(f).length;
  const ups = n(e => e.update);
  const shows = [['all', 'All'], ['installed', `Installed (${n(e => e.installed)})`], ['updates', `Updates (${ups})`]];
  const limited = RC.data.limitedUntil ? `<p class="note">GitHub only answers so many questions an hour without a sign-in. Until ${new Date(RC.data.limitedUntil).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}, Hearth shows what it asked earlier.</p>` : '';
  return `<div class="bar">${shows.map(([k, l]) => `<button class="chip f ${RC.show === k ? 'on' : ''}" data-rcs="${k}">${rcEsc(l)}</button>`).join('')}
      <button class="chip f" data-rca="console">${RC.con === 'all' ? 'Every console' : rcEsc(rcCon(RC.con))} ▾</button>
      <span class="spacer"></span>
      <input class="inp f" id="rq" placeholder="Search ports and games" data-osk value="${rcEsc(RC.q)}"><button class="btn f" data-rca="check">Check for updates</button></div>
    <div class="scroll">${limited}<div class="grid">${rcGrid()}</div>
      <p class="note" style="margin-top:1.2em">${all.length} native PC versions of console games, each installed from its own project's GitHub releases. You bring your own copy of the game (a ROM or disc image you made from a game you own): Hearth checks it and puts it where the port looks. Hearth never downloads games. Installed ports show up in Steam's Game Mode too.</p></div>`;
}
function rcAfterRender() {
  const q = KIT.$('#rq');
  if (q) q.oninput = () => { RC.q = q.value; const g = KIT.$('#main .grid'); if (g) g.innerHTML = rcGrid(); };
}

// ---------------------------------------------------------------- a port's page
function rcPageHtml(e) {
  const d = RC.det[e.id], err = RC.derr[e.id], j = e.job || {}, b = rcBuild(e), latest = d && d.latest;
  const pick = d && d.pick;
  const acts = [];
  const btn = (a, label, cls = '', extra = '') => `<button class="btn f ${cls}" data-r="${a}" ${extra}>${label}</button>`;
  const copyLabel = e.needs ? (e.copy && e.copy.path ? 'Change your copy' : `Needs your copy of ${rcEsc(e.game || e.title)}`) : '';
  if (j.busy) {
    if (/Download/.test(j.stage || '')) acts.push(btn('cancel', 'Cancel', 'danger', 'data-autofocus'));
  } else if (!e.installed) {
    const none = d && d.releases && !pick;
    acts.push(btn('install', none ? 'No build for this PC' : 'Install', none ? '' : 'primary', 'data-autofocus'));
    if (e.needs) acts.push(btn('copy', copyLabel));
  } else {
    acts.push(btn('play', '▶ Play', 'primary', 'data-autofocus'));
    if (e.update) acts.push(btn('update', `Update to ${rcEsc(e.update)}`), btn('skip', 'Skip this update'));
    if (e.needs) acts.push(btn('copy', copyLabel));
    acts.push(btn('uninstall', 'Uninstall', 'danger'));
  }
  acts.push(btn('close', 'Back'));
  const facts = [
    ['Console', rcCon(e.console)],
    ['Kind', RC_KIND[e.kind] || 'Port'],
    ['This PC gets', pick ? `${rcBuildLabel(pick.platform)} (${pick.asset.name}${pick.asset.size ? `, ${KIT.human(pick.asset.size)}` : ''})` : rcBuildLabel(b)],
    ['Installed', e.installed ? (e.version || 'Your own copy') : 'No'],
    ['Newest', latest ? `${latest.tag}${latest.published ? `, ${new Date(latest.published).toLocaleDateString()}` : ''}${latest.prerelease ? ' (pre-release)' : ''}` : err ? 'Couldn\'t ask GitHub' : RC.dbusy[e.id] ? 'Asking GitHub…' : '…'],
    e.installed && e.dir ? ['Installed in', `<span class="path">${rcEsc(e.dir)}</span>`] : null,
    e.saves ? ['Saves', e.saves] : null,
  ].filter(Boolean);
  const tags = [e.featured ? '<span class="badge">Featured</span>' : '', e.experimental ? '<span class="badge muted">Early: expect rough edges</span>' : '',
    e.custom ? '<span class="badge muted">Added by you: not checked by Hearth</span>' : '', e.arm64 ? '<span class="badge muted">Arm builds too</span>' : ''].join('');
  const owner = (e.repo || '').split('/')[0];
  const about = `${e.game ? `A native PC version of ${rcEsc(e.game)}` : rcEsc(e.title)}${e.console && e.console !== 'other' ? ` (${rcEsc(rcCon(e.console))})` : ''}: ${rcEsc((RC_KIND[e.kind] || 'port').toLowerCase())} made by the <b>${rcEsc(owner)}</b> project, installed from its own GitHub releases (<span class="path">github.com/${rcEsc(e.repo)}</span>).${b === 'windows' ? ' It has no Linux build, so Hearth runs its Windows build with Proton, like any Windows game.' : ''}`;
  const work = j.busy ? `<div class="panel rc-job"><b id="rc-stage">${rcEsc(j.stage || 'Working…')}</b> <span id="rc-pct" class="msg">${j.progress != null ? Math.round(j.progress * 100) + '%' : ''}</span>
      <div class="prog ${j.progress == null ? 'spin' : ''}" id="rc-bar"><i style="width:${j.progress != null ? Math.round(j.progress * 100) : 30}%"></i></div></div>` : '';
  const failed = j.error && !j.busy && !j.mismatch ? `<p class="bad">${rcEsc(j.error)}</p>` : '';
  // Your copy of the game: what it needs, how Hearth uses it, and the check's verdict (or "Use it anyway").
  let copy = '';
  if (e.needs) {
    const g = e.copy && e.copy.path ? e.copy : null;
    const check = e.hash ? 'Hearth checks it against the hash the project documents, so you know it\'s the version the port needs.' : 'The project doesn\'t document a hash, so Hearth can\'t check it: use the version it asks for.';
    const how = e.setup === 'n64recomp' ? 'Hearth checks your ROM the way the port does and puts it where the port keeps it, so the game starts straight away.'
      : e.setup === 'beside' ? `Hearth puts a link to your file in the port's folder${e.besideName ? ` (as <span class="path">${rcEsc(e.besideName)}</span>)` : ''}. Nothing is copied.`
      : 'The port asks for your file the first time it starts: choose the same file there.';
    const verdict = !g ? '' : `<p><span class="path">${rcEsc(g.path)}</span><br>${g.verified === true ? '<span class="ok">✓ Checked: the right version</span>' : g.verified === false ? '<span class="warn">Used anyway: it doesn\'t match the project\'s hash</span>' : '<span class="msg">Not checked (there\'s no hash to check it against)</span>'}</p>`;
    const mismatch = j.mismatch && !j.busy ? `<p class="bad">${rcEsc(j.error)}. Make sure it's ${rcEsc(e.needs)}.</p><div class="acts">${btn('anyway', 'Use it anyway')}${btn('copy', 'Choose another file')}</div>` : '';
    copy = `<h3>Your copy of the game</h3><p class="msg">It needs <b>${rcEsc(e.needs)}</b>${(e.files || []).length ? ` (${rcEsc(e.files.join(', '))})` : ''}, made from a game you own. ${check} ${how}</p>${verdict}${mismatch}`;
  }
  const full = (latest && latest.notes) || '';
  const notes = RC.more ? full : full.slice(0, 1400);
  const notesHtml = err ? `<p class="msg">Couldn't ask GitHub: ${rcEsc(err)}</p>${btn('refresh', 'Try again')}`
    : !d ? '<div class="prog spin"><i></i></div>'
    : !latest ? '<p class="msg">No releases yet.</p>'
    : `<div class="rc-notes">${rcEsc(notes) || 'The project wrote no notes for this release.'}${full.length > notes.length ? '…' : ''}</div>${full.length > 1400 ? btn('more', RC.more ? 'Show less' : 'Read more') : ''}${d.stale ? '<p class="note">Shown as GitHub answered earlier.</p>' : ''}`;
  return `<div id="rc-page"></div><div class="rc-head">${e.icon ? `<img src="${rcEsc(e.icon)}" alt="">` : `<span class="letter">${rcEsc((e.title || '?')[0])}</span>`}<div><h2>${rcEsc(e.title)}</h2>${tags ? `<div class="chips">${tags}</div>` : ''}</div></div>
    <p class="msg">${about}</p>
    ${work}${failed}
    <div class="row" style="justify-content:flex-start">${acts.join('')}</div>
    <dl class="kv" style="margin-top:1em">${facts.map(([k, v]) => `<dt>${rcEsc(k)}</dt><dd>${typeof v === 'string' && v.startsWith('<') ? v : rcEsc(v)}</dd>`).join('')}</dl>
    ${copy}
    <h3>Patch notes${latest ? `: ${rcEsc(latest.name || latest.tag)}` : ''}</h3>${notesHtml}
    <p class="note">${rcEsc(e.title)} is made by the ${rcEsc(owner)} project and its contributors, not by Hearth. Updates are copied over the old version: your saves, mods, settings and game file stay.</p>`;
}
function rcOpen(id, focusKey) {
  const e = rcGet(id); if (!e) return;
  if (RC.page !== id) RC.more = false;
  RC.page = id;
  rcDetails(id);
  const d = KIT.dialog(rcPageHtml(e), { wide: true, onBack: rcClosePage });
  if (focusKey) d.querySelector(`[data-r="${focusKey}"]`)?.focus();
  d.onclick = ev => { const b = ev.target.closest('[data-r]'); if (b) rcAct(b.dataset.r, e.id); };
}
function rcClosePage() { RC.page = null; KIT.close(); rcRefreshTiles(); }
/** The tiles' states without drawing the tab again (so the tile that had focus keeps it). */
function rcRefreshTiles() {
  for (const el of document.querySelectorAll('#main .tile[data-rc]')) {
    const e = rcGet(el.dataset.rc); if (!e) continue;
    el.querySelector('.s').innerHTML = rcTileState(e);
    el.classList.toggle('dim', !(e.installed || (e.job && e.job.busy)));
  }
}
/** Draws the open page again, keeping the button that had focus and where it was scrolled to. */
function rcRedraw() {
  const e = RC.page && rcGet(RC.page); if (!e) return;
  const dlg = KIT.$('#kit-layer .dlg'); if (!dlg || !rcPageOpen()) return;
  const top = dlg.scrollTop, had = document.activeElement?.dataset?.r;
  rcOpen(e.id, had);
  const n = KIT.$('#kit-layer .dlg'); if (n) n.scrollTop = top;
}
/** After a choice dialog took the page's place: the page comes back (once KIT.close has put focus back on the tile). */
const rcBack = (id, focusKey) => setTimeout(() => rcOpen(id, focusKey), 0);

// ---------------------------------------------------------------- what the page's buttons do
async function rcInstall(e) {
  const d = RC.det[e.id];
  if (d && d.releases && !d.pick) return KIT.toast(`${e.title} has no build for this PC in its newest release.`);
  let place = '';
  try {
    const P = await call('recomps.places');
    if (P && P.length > 1) {
      const pickd = await KIT.choose(`Install ${e.title}`, P.map((p, i) => ({ id: String(i), label: p.label, end: p.free ? `${KIT.human(p.free)} free` : '', sub: p.path || 'Hearth\'s own folder on this device' })), 'Choose the drive.');
      if (pickd == null) return rcBack(e.id, 'install');
      place = P[+pickd].path || '';
      rcBack(e.id);
    }
  } catch (err) { KIT.toast(KIT.err(err)); }
  try { await call('recomps.install', place ? { id: e.id, place } : { id: e.id }); KIT.toast('Getting it from GitHub…'); }
  catch (err) { await KIT.alert(`Couldn't install ${e.title}`, KIT.esc(KIT.err(err)), false); rcBack(e.id, 'install'); }
  rcLoad();
}
async function rcUninstall(e) {
  const pickd = await KIT.choose(`Uninstall ${e.title}?`, [
    { id: 'keep', label: 'Uninstall, back up my saves first', sub: 'Your saves and mods from its folder go in a backup first' },
    { id: 'all', label: 'Uninstall', sub: 'The port is removed' },
  ], `Your own copy of the game is never deleted.${e.adopted ? ' This port is in a folder you chose, so Hearth only forgets it.' : ''}${e.setup === 'n64recomp' ? ' Its saves in ~/.config stay either way.' : ''}`);
  if (!pickd) return rcBack(e.id, 'uninstall');
  try {
    const r = await call('recomps.uninstall', { id: e.id, keep: pickd === 'keep' });
    KIT.toast(r && r.backup ? `Uninstalled. Saves backed up to ${r.backup}` : `${e.title} is uninstalled`);
  } catch (err) { KIT.alert(`Couldn't uninstall ${e.title}`, KIT.esc(KIT.err(err)), false); }
  RC.page = null;
  rcLoad();
}
async function rcAct(a, id) {
  const e = rcGet(id); if (!e) return;
  const j = e.job || {};
  switch (a) {
    case 'close': return rcClosePage();
    case 'play': RC.page = null; KIT.close(); return play({ id: `recomp:${id}`, title: e.title });
    case 'install': return rcInstall(e);
    case 'update':
      try { await call('recomps.install', { id }); KIT.toast(`Updating ${e.title}…`); } catch (err) { KIT.toast(KIT.err(err)); }
      return rcLoad();
    case 'skip':
      try { await call('recomps.skip', { id, tag: e.update }); KIT.toast('Skipped: Hearth tells you about the next one'); } catch (err) { KIT.toast(KIT.err(err)); }
      return rcLoad();
    case 'cancel': return call('recomps.cancel', { id }).catch(err => KIT.toast(KIT.err(err)));
    case 'uninstall': return rcUninstall(e);
    case 'copy':
      // The desktop's file chooser opens; Hearth checks the file in the background ("recomps" events).
      KIT.toast('Choose your copy of the game in the file chooser');
      try { const r = await call('recomps.pickGame', { id }); if (r) KIT.toast('Checking your copy…'); } catch (err) { await KIT.alert('That file can\'t be used', KIT.esc(KIT.err(err)), false); rcBack(id, 'copy'); }
      return;
    case 'anyway':
      try { await call('recomps.useGame', { id, path: j.file, anyway: true }); } catch (err) { KIT.toast(KIT.err(err)); }
      return;
    case 'more': RC.more = !RC.more; return rcRedraw();
    case 'refresh': RC.det[id] = null; rcDetails(id, true); return rcRedraw();
  }
}

/** Clicks on the Recomps tab (games.js passes them on). True when it was one of ours. */
function rcClick(ev) {
  const t = ev.target.closest('[data-rc]'); if (t) { rcOpen(t.dataset.rc); return true; }
  const s = ev.target.closest('[data-rcs]'); if (s) { RC.show = s.dataset.rcs; render(); return true; }
  const b = ev.target.closest('[data-rca]'); if (!b) return false;
  if (b.dataset.rca === 'retry') { RC.err = ''; rcLoad(); render(); }
  if (b.dataset.rca === 'check') call('recomps.checkUpdates').then(() => KIT.toast('Looking for updates…')).catch(err => KIT.toast(KIT.err(err)));
  if (b.dataset.rca === 'console') {
    const cons = [...new Set(rcAll().filter(e => !e.hidden).map(e => e.console))].sort((x, y) => rcCon(x).localeCompare(rcCon(y)));
    KIT.choose('Show ports for', [{ id: 'all', label: 'Every console' }, ...cons.map(c => ({ id: c, label: rcCon(c), end: String(rcAll().filter(e => e.console === c && !e.hidden).length) }))])
      .then(c => { if (c) { RC.con = c; render(); } });
  }
  return true;
}

// ---------------------------------------------------------------- preview without the app (a plain browser)
function rcDemo(m, a) {
  const notes = 'What\'s new\n\n- Widescreen and high frame rates\n- Fixed a crash when pausing\n- Mods: the new mod menu\n';
  if (m === 'recomps.list') return {
    updated: '2026-10', consoles: { n64: 'Nintendo 64', x360: 'Xbox 360', ps1: 'PlayStation' }, limitedUntil: null, arch: 'x86_64',
    recomps: [
      { id: 'zelda64', title: 'Zelda 64: Recompiled', game: 'The Legend of Zelda: Majora\'s Mask', console: 'n64', kind: 'recomp', repo: 'Zelda64Recomp/Zelda64Recomp', linux: true, needs: 'Majora\'s Mask (USA) ROM', files: ['.z64', '.n64', '.v64'], setup: 'n64recomp', hash: { xxh3: ['x'] }, featured: true, installed: true, version: 'v1.2.0', update: 'v1.2.1', platform: 'linux', dir: '/home/you/.local/share/nimbus/recomps/zelda64/app', copy: { path: '/home/you/Games/mm.z64', verified: true }, saves: '~/.config/Zelda64Recompiled/saves' },
      { id: 'unleashed', title: 'Unleashed Recompiled', game: 'Sonic Unleashed', console: 'x360', kind: 'recomp', repo: 'hedge-dev/UnleashedRecomp', linux: true, needs: 'Sonic Unleashed (Xbox 360) disc image', files: ['.iso'], setup: 'in-app', installed: false, featured: true },
      { id: 'soh', title: 'Ship of Harkinian', game: 'The Legend of Zelda: Ocarina of Time', console: 'n64', kind: 'decomp', repo: 'HarbourMasters/Shipwright', linux: true, needs: 'Ocarina of Time ROM', files: ['.z64'], setup: 'beside', besideName: 'oot.z64', hash: { sha1: ['y'] }, installed: false, job: { busy: true, stage: 'Downloading 9.0.0…', progress: .35 } },
      { id: 'banjo', title: 'Banjo: Recompiled', game: 'Banjo-Kazooie', console: 'n64', kind: 'recomp', repo: 'BanjoRecomp/BanjoRecomp', linux: true, needs: 'Banjo-Kazooie (USA) ROM', files: ['.z64'], setup: 'n64recomp', hash: { xxh3: ['z'] }, installed: false, job: { busy: false, mismatch: true, error: 'It doesn\'t match the version the project checks for', file: '/home/you/Games/banjo.z64' } },
      { id: 'cdx', title: 'Crash Decomp', game: 'Crash Bandicoot', console: 'ps1', kind: 'decomp', repo: 'example/crash', linux: false, needs: 'Crash Bandicoot (USA) disc image', files: ['.cue', '.bin'], setup: 'beside', installed: false, experimental: true },
    ],
  };
  if (m === 'recomps.details') return { id: a.id, latest: { tag: 'v1.2.1', name: 'v1.2.1', notes, published: '2026-10-20T10:00:00Z' }, releases: [{ tag: 'v1.2.1' }], pick: { platform: a.id === 'cdx' ? 'windows' : 'linux', asset: { name: a.id === 'cdx' ? 'Crash-Windows.zip' : 'Port-Linux-x64.AppImage', size: 62e6 } } };
  if (m === 'recomps.places') return [{ label: 'This device', path: '', free: 120e9 }, { label: 'SD card', path: '/run/media/you/SD/Hearth Recomps', free: 400e9 }];
  return {};
}
