// AppImage Maker's page: packages dropped or picked are converted one at a time (convert_one in gui.rs), with
// their steps, log and warnings; My AppImages lists what was made (open, folder, app menu, delete).
'use strict';
const T = window.__TAURI__;
const invoke = T ? T.core.invoke : (c, a) => demoInvoke(c, a || {});
const listen = T ? T.event.listen : (n, f) => { (window.__demoListeners ||= {})[n] = f; return Promise.resolve(() => {}); };
const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const ICON_PKG = '<svg viewBox="0 0 24 24"><path d="M12 3l8 4v10l-8 4-8-4V7z"/><path d="M4 7l8 4 8-4M12 11v10"/></svg>';
const ICON_WARN = '<svg viewBox="0 0 24 24"><path d="M12 4l9 16H3zM12 10v4M12 17v.5"/></svg>';
const STEPS = ['Unpacking', 'Fixing paths', 'Finding the app', 'Gathering libraries', "Writing the AppImage's launcher", 'Packing', 'Adding it to the app menu'];
const KINDS = /\.(deb|rpm|zip|tar|tgz|txz|tzst|tbz2?|tar\.(gz|xz|zst|bz2))$/i;

const st = { tab: 'make', jobs: [], nextId: 1, busy: false, settings: { outDir: '', fetch: true, menu: true }, defaultOut: '', mine: [], icons: {} };

function toast(t) { const el = $('#toast'); el.textContent = t; el.hidden = false; clearTimeout(toast.t); toast.t = setTimeout(() => (el.hidden = true), 2600); }
const base = p => String(p).split('/').pop();
const mb = n => n >= 1e9 ? (n / 1e9).toFixed(1) + ' GB' : Math.max(0.1, n / 1e6).toFixed(n < 1e7 ? 1 : 0) + ' MB';

// ---------------------------------------------------------------- settings
let saveTimer;
function save() { clearTimeout(saveTimer); saveTimer = setTimeout(() => invoke('store_set', { value: { ...st.settings } }).catch(() => {}), 200); }
function drawSettings() {
  const out = st.settings.outDir || st.defaultOut || '~/Applications';
  // The box trims long paths from the left (direction: rtl); <bdi> keeps the path itself the right way round.
  $('#outDir').innerHTML = `<bdi>${esc(out)}</bdi>`; $('#outDir').title = out;
  $('#tgFetch').classList.toggle('on', !!st.settings.fetch); $('#tgFetch').setAttribute('aria-checked', String(!!st.settings.fetch));
  $('#tgMenu').classList.toggle('on', !!st.settings.menu); $('#tgMenu').setAttribute('aria-checked', String(!!st.settings.menu));
}
$('#tgFetch').onclick = () => { st.settings.fetch = !st.settings.fetch; drawSettings(); save(); };
$('#tgMenu').onclick = () => { st.settings.menu = !st.settings.menu; drawSettings(); save(); };
$('#outBtn').onclick = async () => { const d = await invoke('pick_folder').catch(() => null); if (d) { st.settings.outDir = d; drawSettings(); save(); } };

// ---------------------------------------------------------------- tabs
function setTab(t) {
  st.tab = t;
  $$('.tab').forEach(b => b.classList.toggle('on', b.dataset.tab === t));
  $('#make').hidden = t !== 'make'; $('#mine').hidden = t !== 'mine';
  if (t === 'mine') loadMine();
}
$$('.tab').forEach(b => (b.onclick = () => setTab(b.dataset.tab)));

// ---------------------------------------------------------------- the queue
function add(paths) {
  let n = 0;
  for (const p of paths) {
    if (!KINDS.test(p)) { toast(`${base(p)} isn't a .deb, .rpm, tarball or .zip`); continue; }
    if (st.jobs.some(j => j.path === p && (j.state === 'wait' || j.state === 'work'))) continue;
    st.jobs.unshift({ id: st.nextId++, path: p, state: 'wait', step: '', log: [], report: null, error: '', open: false });
    n++;
  }
  if (n) { setTab('make'); drawJobs(); run(); }
}
async function pick() { const files = await invoke('pick_files').catch(() => []); add(files || []); }
$('#drop').onclick = pick;

async function run() {
  if (st.busy) return;
  const j = st.jobs.slice().reverse().find(x => x.state === 'wait');
  if (!j) return;
  st.busy = true; j.state = 'work'; j.step = STEPS[0]; drawJobs();
  try {
    j.report = await invoke('convert_one', { job: j.id, path: j.path, outDir: st.settings.outDir || '', fetch: !!st.settings.fetch, menu: !!st.settings.menu });
    j.state = 'done';
    if (j.report.icon) iconFor(j.report.icon).then(drawJobs);
    toast(`${j.report.name} is ready`);
    loadMine(true);
  } catch (e) {
    j.state = 'fail'; j.error = String(e); j.open = true;
  }
  st.busy = false; drawJobs(); run();
}

listen('progress', ev => {
  const p = ev.payload || {}; const j = st.jobs.find(x => x.id === p.job); if (!j) return;
  if (p.step) j.step = p.step;
  if (p.log) j.log.push(p.log);
  drawJob(j);
});
listen('drop', ev => {
  const p = ev.payload || {};
  document.body.classList.toggle('over', !!p.over);
  if (p.files) { document.body.classList.remove('over'); add(p.files); }
});

async function iconFor(path) {
  if (!path) return '';
  if (st.icons[path] === undefined) st.icons[path] = (await invoke('icon_data', { path }).catch(() => null)) || '';
  return st.icons[path];
}
function iconHtml(path) { const d = st.icons[path]; return d ? `<img src="${esc(d)}" alt="">` : ICON_PKG; }

function jobHtml(j) {
  const r = j.report;
  const pct = j.state === 'done' ? 100 : j.state === 'work' ? Math.round(100 * (Math.max(0, STEPS.indexOf(j.step)) + .5) / (STEPS.length - (st.settings.menu ? 0 : 1))) : 0;
  const label = { wait: 'Waiting', work: j.step || 'Working', done: 'Done', fail: "Didn't work" }[j.state];
  const name = r ? `${r.name}${r.version ? ' ' + r.version : ''}` : base(j.path);
  const sub = r ? `${base(r.output)} · ${mb(r.size)}` : j.path;
  let h = `<div class="top"><div class="ic">${r ? iconHtml(r.icon) : ICON_PKG}</div><div class="nm"><b>${esc(name)}</b><span>${esc(sub)}</span></div><span class="state ${j.state}">${esc(label)}</span></div>`;
  if (j.state === 'work') h += `<div class="bar"><i style="width:${pct}%"></i></div>`;
  if (j.error) h += `<div class="error">${esc(j.error)}</div>`;
  if (r) {
    for (const w of r.warnings || []) h += `<div class="warn">${ICON_WARN}<span>${esc(w)}</span></div>`;
    const host = (r.libs_from_host || []).length, fed = (r.libs_from_fedora || []).length;
    if (host || fed) h += `<div class="libs">Libraries put inside: ${host} from this computer${fed ? `, ${fed} downloaded from Fedora` : ''}</div>`;
    h += `<div class="row"><button class="btn pri f" data-act="open">Open</button><button class="btn f" data-act="folder">Show in folder</button>${r.menu_entry ? '<span class="tag menu">In the app menu</span>' : '<button class="btn f" data-act="menu">Add to app menu</button>'}<span class="spacer"></span><button class="btn link f" data-act="log">${j.open ? 'Hide details' : 'Details'}</button></div>`;
  } else if (j.state === 'fail') {
    h += `<div class="row"><button class="btn f" data-act="retry">Try again</button><button class="btn link f" data-act="dismiss">Dismiss</button><span class="spacer"></span><button class="btn link f" data-act="log">${j.open ? 'Hide details' : 'Details'}</button></div>`;
  }
  if (j.open && j.log.length) h += `<div class="log">${esc(j.log.join('\n'))}</div>`;
  return h;
}
function drawJob(j) {
  const el = $(`.job[data-id="${j.id}"]`); if (!el) return drawJobs();
  const focused = document.activeElement && el.contains(document.activeElement) ? document.activeElement.dataset.act : null;
  el.innerHTML = jobHtml(j);
  if (focused) $(`[data-act="${focused}"]`, el)?.focus();
  const log = $('.log', el); if (log) log.scrollTop = log.scrollHeight;
}
function drawJobs() {
  const box = $('#jobs');
  const focusedJob = document.activeElement?.closest?.('.job')?.dataset.id; const focusedAct = document.activeElement?.dataset?.act;
  if (!st.jobs.length) { box.innerHTML = '<div class="empty">Apps you drop here are turned into AppImages, one at a time.</div>'; return; }
  box.innerHTML = st.jobs.map(j => `<div class="job" data-id="${j.id}">${jobHtml(j)}</div>`).join('');
  if (focusedJob) $(`.job[data-id="${focusedJob}"] [data-act="${focusedAct}"]`)?.focus();
}
$('#jobs').addEventListener('click', async e => {
  const b = e.target.closest('[data-act]'); if (!b) return;
  const j = st.jobs.find(x => x.id === +b.closest('.job').dataset.id); if (!j) return;
  const act = b.dataset.act;
  if (act === 'log') { j.open = !j.open; return drawJob(j); }
  if (act === 'retry') { j.state = 'wait'; j.error = ''; j.log = []; drawJobs(); return run(); }
  if (act === 'dismiss') { st.jobs = st.jobs.filter(x => x !== j); return drawJobs(); }
  await itemAction(act, j.report);
  if (act === 'menu') { const m = st.mine.find(x => x.output === j.report.output); if (m) j.report.menu_entry = m.menu_entry; drawJob(j); }
});

// ---------------------------------------------------------------- My AppImages
async function loadMine(quiet) {
  st.mine = (await invoke('made_list').catch(() => [])) || [];
  await Promise.all(st.mine.map(m => iconFor(m.icon)));
  $('#mineCount').textContent = st.mine.length ? `(${st.mine.length})` : '';
  if (!quiet || st.tab === 'mine') drawMine();
}
function drawMine() {
  const box = $('#mineList');
  if (!st.mine.length) { box.innerHTML = '<div class="empty">No AppImages made yet. Drop an app on the Make tab.</div>'; return; }
  box.innerHTML = st.mine.map((m, i) => {
    const when = m.made ? new Date(m.made * 1000).toLocaleDateString() : '';
    const tags = (m.in_menu ? '<span class="tag menu">In the app menu</span>' : '') + (m.exists ? '' : '<span class="tag">File missing</span>');
    const btns = m.exists
      ? `<button class="btn pri f" data-act="open">Open</button><button class="btn f" data-act="folder">Show in folder</button>${m.in_menu ? '<button class="btn f" data-act="unmenu">Take out of menu</button>' : '<button class="btn f" data-act="menu">Add to app menu</button>'}<span class="spacer"></span><button class="btn link f" data-act="forget">Remove from list</button><button class="btn danger f" data-act="delete">Delete</button>`
      : `<span class="spacer"></span><button class="btn link f" data-act="forget">Remove from list</button>`;
    return `<div class="item${m.exists ? '' : ' gone'}" data-i="${i}"><div class="top"><div class="ic">${iconHtml(m.icon)}</div><div class="nm"><b>${esc(m.name)}${m.version ? ' ' + esc(m.version) : ''}</b><span>${esc(base(m.output))} · ${mb(m.size || 0)} · made ${esc(when)} from ${esc(base(m.input))}</span></div>${tags}</div><div class="row">${btns}</div></div>`;
  }).join('');
}
$('#mineList').addEventListener('click', async e => {
  const b = e.target.closest('[data-act]'); if (!b) return;
  const m = st.mine[+b.closest('.item').dataset.i]; if (!m) return;
  await itemAction(b.dataset.act, m);
});

async function itemAction(act, m) {
  try {
    if (act === 'open') { await invoke('open_app', { path: m.output }); toast(`Starting ${m.name}`); }
    else if (act === 'folder') await invoke('show_in_folder', { path: m.output });
    else if (act === 'menu') { await invoke('add_menu', { output: m.output }); toast(`${m.name} is in the app menu`); await loadMine(); }
    else if (act === 'unmenu') { await invoke('remove_menu', { output: m.output }); toast(`${m.name} was taken out of the menu`); await loadMine(); }
    else if (act === 'forget') { await invoke('forget', { output: m.output, deleteFile: false }); await loadMine(); }
    else if (act === 'delete') {
      if (await confirmBox(`Delete ${base(m.output)}? The AppImage file and its menu entry go; the package you made it from stays.`)) {
        await invoke('forget', { output: m.output, deleteFile: true }); toast('Deleted'); await loadMine();
      }
    }
  } catch (e) { toast(String(e)); }
}

function confirmBox(text) {
  return new Promise(res => {
    const box = $('#confirm'); $('#confirmText').textContent = text; box.hidden = false; $('#confirmNo').focus();
    const done = v => { box.hidden = true; $('#confirmYes').onclick = $('#confirmNo').onclick = null; confirmBox.close = null; res(v); };
    $('#confirmYes').onclick = () => done(true); $('#confirmNo').onclick = () => done(false);
    confirmBox.close = () => done(false);
  });
}

// ---------------------------------------------------------------- keyboard, controller
document.addEventListener('keydown', e => {
  if (e.ctrlKey && e.key.toLowerCase() === 'o') { e.preventDefault(); pick(); return; }
  if (e.key === 'Escape' && confirmBox.close) { confirmBox.close(); return; }
  const dirs = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
  if (dirs[e.key]) { e.preventDefault(); move(dirs[e.key]); }
});
function focusables() { const root = $('#confirm:not([hidden])') || document; return [...root.querySelectorAll('.f')].filter(el => !el.disabled && el.offsetParent !== null); }
function move(dir) {
  const all = focusables(); if (!all.length) return;
  const cur = document.activeElement;
  if (!cur || !all.includes(cur)) { all[0].focus(); return; }
  const a = cur.getBoundingClientRect(); const ax = a.left + a.width / 2, ay = a.top + a.height / 2;
  let best = null, bestScore = Infinity;
  for (const el of all) {
    if (el === cur) continue;
    const b = el.getBoundingClientRect(); const bx = b.left + b.width / 2, by = b.top + b.height / 2;
    const ok = dir === 'left' ? b.right <= a.left + 4 : dir === 'right' ? b.left >= a.right - 4 : dir === 'up' ? b.bottom <= a.top + 4 : b.top >= a.bottom - 4;
    if (!ok) continue;
    const main = dir === 'left' || dir === 'right' ? Math.abs(bx - ax) : Math.abs(by - ay);
    const cross = dir === 'left' || dir === 'right' ? Math.abs(by - ay) : Math.abs(bx - ax);
    const s = main + cross * 2.2;
    if (s < bestScore) { bestScore = s; best = el; }
  }
  if (best) { best.focus(); best.scrollIntoView({ block: 'nearest' }); }
}
// D-pad/stick moves, A presses, B goes back (closes the question, or to the Make tab), X picks apps, Y opens the
// newest AppImage, LB/RB switch tabs.
const held = {}; let lastPadMove = 0;
function pad() {
  const now = performance.now();
  const s = { up: 0, down: 0, left: 0, right: 0, a: 0, b: 0, x: 0, y: 0, lb: 0, rb: 0 };
  for (const g of HearthGamepad.pads()) {
    const bt = i => g.buttons[i]?.pressed; const ax = g.axes[0] || 0, ay = g.axes[1] || 0;
    s.up ||= bt(12) || ay < -.6; s.down ||= bt(13) || ay > .6; s.left ||= bt(14) || ax < -.6; s.right ||= bt(15) || ax > .6;
    s.a ||= bt(0); s.b ||= bt(1); s.x ||= bt(2); s.y ||= bt(3); s.lb ||= bt(4); s.rb ||= bt(5);
  }
  for (const [k, on] of Object.entries(s)) {
    if (!on) { held[k] = 0; continue; }
    const first = !held[k];
    const rep = held[k] && now - held[k] > 380 && now - lastPadMove > 110 && ['up', 'down', 'left', 'right'].includes(k);
    if (first) held[k] = now;
    if (!first && !rep) continue;
    lastPadMove = now; document.body.classList.add('pad');
    if (['up', 'down', 'left', 'right'].includes(k)) move(k);
    else if (k === 'a') { const el = document.activeElement; if (el && el.matches('button')) el.click(); else focusables()[0]?.focus(); }
    else if (k === 'b') { if (confirmBox.close) confirmBox.close(); else if (st.tab !== 'make') setTab('make'); }
    else if (k === 'x') pick();
    else if (k === 'y') { const done = st.jobs.find(j => j.state === 'done') || null; const m = done ? done.report : st.mine.find(x => x.exists); if (m) itemAction('open', m); }
    else if (k === 'lb' || k === 'rb') setTab(st.tab === 'make' ? 'mine' : 'make');
  }
}
document.addEventListener('mousemove', () => document.body.classList.remove('pad'));

(async function start() {
  let saved = null;
  try { saved = await invoke('store_get'); } catch (_) {}
  if (saved && typeof saved === 'object') {
    st.defaultOut = saved.defaultOut || '';
    if (typeof saved.outDir === 'string') st.settings.outDir = saved.outDir;
    if (typeof saved.fetch === 'boolean') st.settings.fetch = saved.fetch;
    if (typeof saved.menu === 'boolean') st.settings.menu = saved.menu;
  }
  drawSettings(); drawJobs(); loadMine(true);
  const files = await invoke('start_files').catch(() => []);
  if (files && files.length) add(files);
  HearthGamepad.loop(pad);
  document.body.dataset.ready = '1';
})();

// Without the app (tests/ui_test.py in Chromium): a pretend conversion.
function demoInvoke(cmd, a) {
  const s = (window.__demoStore ||= { settings: null, made: [] });
  if (cmd === 'store_get') return Promise.resolve({ ...(s.settings || {}), defaultOut: '/home/you/Applications' });
  if (cmd === 'store_set') { s.settings = a.value; return Promise.resolve(null); }
  if (cmd === 'made_list') return Promise.resolve(s.made.map(m => ({ ...m, exists: true, in_menu: !!m.menu_entry })));
  if (cmd === 'start_files') return Promise.resolve(window.__demoStart || []);
  if (cmd === 'pick_files') return Promise.resolve(window.__demoPick || []);
  if (cmd === 'icon_data') return Promise.resolve(null);
  if (cmd === 'convert_one') {
    return new Promise((res, rej) => {
      const fire = (p) => window.__demoListeners?.progress?.({ payload: { job: a.job, ...p } });
      let i = 0;
      const t = setInterval(() => {
        if (i < 6) { fire({ step: STEPS[i], log: `${STEPS[i]}…` }); i++; return; }
        clearInterval(t);
        if (/broken/.test(a.path)) return rej('couldn\'t find a program to start in this package');
        const name = base(a.path).replace(/[-_].*$/, '');
        const r = { input: a.path, output: `${a.outDir || '/home/you/Applications'}/${name}-1.0-x86_64.AppImage`, name, version: '1.0', size: 84e6, icon: '',
          warnings: /python/.test(a.path) ? ['This app runs on Python, which isn\'t put inside: it uses this computer\'s Python and its modules.'] : [],
          libs_from_host: ['libgtk-3.so.0', 'libnotify.so.4'], libs_from_fedora: ['libyaml-0.so.2 (libyaml)'], libs_missing: [], menu_entry: a.menu ? '/home/you/.local/share/applications/x.desktop' : '', made: 1763452800 };
        s.made = [r, ...s.made.filter(m => m.output !== r.output)];
        res(r);
      }, 30);
    });
  }
  if (cmd === 'forget') { s.made = s.made.filter(m => m.output !== a.output); return Promise.resolve(null); }
  if (cmd === 'add_menu') { s.made.forEach(m => { if (m.output === a.output) m.menu_entry = 'x'; }); return Promise.resolve('x'); }
  if (cmd === 'remove_menu') { s.made.forEach(m => { if (m.output === a.output) m.menu_entry = ''; }); return Promise.resolve(null); }
  return Promise.resolve(null);
}
