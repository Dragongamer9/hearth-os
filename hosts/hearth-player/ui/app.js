// Hearth Player's page. Plays from the app's local server (the Rust side, src/main.rs); without the app (opened in
// a browser, or in the UI tests) it runs on sample data (demoInvoke at the end).
'use strict';
const T = window.__TAURI__;
const invoke = T ? T.core.invoke : (c, a) => demoInvoke(c, a || {});
const listen = T ? T.event.listen : (n, f) => { (window.__demoListeners ||= {})[n] = f; };
const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const clamp = (v, a, b) => Math.min(b, Math.max(a, v));
const call = (cmd, args) => Promise.resolve(invoke(cmd, args)).catch(e => { throw new Error(typeof e === 'string' ? e : (e?.message || String(e))); });
function fmt(t) {
  if (!isFinite(t) || t < 0) return '--:--';
  t = Math.floor(t); const h = Math.floor(t / 3600), m = Math.floor(t / 60) % 60, s = t % 60;
  return h ? `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}` : `${m}:${String(s).padStart(2, '0')}`;
}
function fmtPrecise(t) { const base = fmt(t); return isFinite(t) ? `${base}.${String(Math.floor((t % 1) * 10))}` : base; }
const baseName = p => String(p).split(/[\\/]/).pop();
const dirName = p => { const s = String(p); const i = Math.max(s.lastIndexOf('/'), s.lastIndexOf('\\')); return i > 0 ? s.slice(0, i) : s; };
const isUrl = p => /^https?:\/\//i.test(p);

// ---------------------------------------------------------------- icons (24px strokes)
const ICON = {
  play: '<path d="M7 4.5v15l12.5-7.5z" fill="currentColor" stroke="none"/>',
  pause: '<path d="M7 4.5h3.5v15H7zM13.5 4.5H17v15h-3.5z" fill="currentColor" stroke="none"/>',
  prev: '<path d="M6 5v14M19 5l-10 7 10 7z"/>', next: '<path d="M18 5v14M5 5l10 7-10 7z"/>',
  back10: '<path d="M4 12a8 8 0 1 0 2.3-5.6M4 4v4h4"/><text x="12" y="15.5" font-size="7.5" text-anchor="middle" fill="currentColor" stroke="none" font-family="sans-serif">10</text>',
  fwd10: '<path d="M20 12a8 8 0 1 1-2.3-5.6M20 4v4h-4"/><text x="12" y="15.5" font-size="7.5" text-anchor="middle" fill="currentColor" stroke="none" font-family="sans-serif">10</text>',
  vol: '<path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4z"/><path d="M15.5 9a4 4 0 0 1 0 6M18 6.5a7.5 7.5 0 0 1 0 11"/>',
  mute: '<path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4z"/><path d="M16 9.5l5 5M21 9.5l-5 5"/>',
  shuffle: '<path d="M4 7h3.5c4.5 0 5.5 10 10 10H20M4 17h3.5c1.8 0 3-1.6 4-3.5M14 9.5c1-1.4 2-2.5 3.5-2.5H20M17.5 4.5 20 7l-2.5 2.5M17.5 14.5 20 17l-2.5 2.5"/>',
  repeat: '<path d="M5 11V9a2 2 0 0 1 2-2h11.5M16 4.5 18.5 7 16 9.5M19 13v2a2 2 0 0 1-2 2H5.5M8 14.5 5.5 17 8 19.5"/>',
  repeat1: '<path d="M5 11V9a2 2 0 0 1 2-2h11.5M16 4.5 18.5 7 16 9.5M19 13v2a2 2 0 0 1-2 2H5.5M8 14.5 5.5 17 8 19.5"/><text x="12" y="14.6" font-size="6.5" text-anchor="middle" fill="currentColor" stroke="none" font-family="sans-serif">1</text>',
  cc: '<rect x="3" y="5.5" width="18" height="13" rx="2.5"/><path d="M10.5 10.3a2.3 2.3 0 1 0 0 3.4M17 10.3a2.3 2.3 0 1 0 0 3.4"/>',
  lang: '<path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4z"/><path d="M15 8h6M15 12h6M15 16h4"/>',
  list: '<path d="M8 6h12M8 12h12M8 18h12M4 6h.01M4 12h.01M4 18h.01"/>',
  queue: '<path d="M4 6h11M4 11h11M4 16h6M16 14v6l5-3z"/>',
  camera: '<path d="M4 8h3l1.5-2h7L17 8h3v11H4z"/><circle cx="12" cy="13" r="3.3"/>',
  cut: '<circle cx="6.5" cy="7" r="2.5"/><circle cx="6.5" cy="17" r="2.5"/><path d="M8.6 8.4 20 17M8.6 15.6 20 7"/>',
  full: '<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>',
  unfull: '<path d="M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5"/>',
  folder: '<path d="M3.5 6.5a1.5 1.5 0 0 1 1.5-1.5h4l2 2h8a1.5 1.5 0 0 1 1.5 1.5v9A1.5 1.5 0 0 1 19 19H5a1.5 1.5 0 0 1-1.5-1.5z"/>',
  radio: '<rect x="3" y="8" width="18" height="12" rx="2.5"/><path d="M7 8 16 3.5"/><circle cx="15.5" cy="14" r="2.5"/><path d="M6.5 12.5h3M6.5 15.5h3"/>',
  note: '<path d="M9 18V5.5l11-2V16"/><circle cx="6.5" cy="18" r="2.5"/><circle cx="17.5" cy="16" r="2.5"/>',
  film: '<rect x="3.5" y="4" width="17" height="16" rx="2"/><path d="M7.5 4v16M16.5 4v16M3.5 8.5h4M3.5 15.5h4M16.5 8.5h4M16.5 15.5h4"/>',
  up: '<path d="M12 19V5M6 11l6-6 6 6"/>', home: '<path d="M4 11 12 4l8 7v9h-5v-6H9v6H4z"/>',
  drive: '<rect x="3.5" y="8" width="17" height="9" rx="2"/><path d="M7 12.5h.01M11 12.5h6"/>',
  download: '<path d="M12 4v11M7 10.5l5 5 5-5M5 19.5h14"/>', check: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
  star: '<path d="m12 4 2.5 5.2 5.6.7-4.1 3.9 1 5.6-5-2.8-5 2.8 1-5.6-4.1-3.9 5.6-.7z"/>',
  starOn: '<path d="m12 4 2.5 5.2 5.6.7-4.1 3.9 1 5.6-5-2.8-5 2.8 1-5.6-4.1-3.9 5.6-.7z" fill="currentColor"/>',
  trash: '<path d="M5 7h14M10 7V5h4v2M7 7l1 12h8l1-12"/>', plus: '<path d="M12 5v14M5 12h14"/>',
  rotL: '<path d="M8 4.5 4.5 8 8 11.5M4.5 8H14a5.5 5.5 0 0 1 0 11h-3"/>', rotR: '<path d="M16 4.5 19.5 8 16 11.5M19.5 8H10a5.5 5.5 0 0 0 0 11h3"/>',
  flipH: '<path d="M12 3v18M9 7 4 17h5zM15 7l5 10h-5z"/>', flipV: '<path d="M3 12h18M7 9 17 4v5zM7 15l10 5v-5z"/>',
  undo: '<path d="M9 14 4 9l5-5M4 9h10a6 6 0 0 1 0 12h-3"/>', redo: '<path d="m15 14 5-5-5-5M20 9H10a6 6 0 0 0 0 12h3"/>',
  refresh: '<path d="M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7"/>', x: '<path d="M6 6l12 12M18 6 6 18"/>',
  disc: '<circle cx="12" cy="12" r="8.5"/><circle cx="12" cy="12" r="2.2"/><path d="M12 5.8a6.2 6.2 0 0 1 6.2 6.2"/>',
  eject: '<path d="M12 5 5 13h14zM5 17.5h14"/>', lock: '<rect x="5" y="10.5" width="14" height="9.5" rx="2"/><path d="M8 10.5V8a4 4 0 0 1 8 0v2.5M12 14v2.5"/>',
  pad: '<path d="M7.5 8h9a4.5 4.5 0 0 1 4.3 5.8l-1 3.4a2.2 2.2 0 0 1-3.7 1L14 16h-4l-2.1 2.2a2.2 2.2 0 0 1-3.7-1l-1-3.4A4.5 4.5 0 0 1 7.5 8z"/><path d="M8 11v3M6.5 12.5h3M15.5 11.5h.01M17.5 13.5h.01"/>',
};
const icon = n => `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${ICON[n] || ''}</svg>`;
function paintIcons(root = document) { for (const el of $$('[data-i]', root)) el.innerHTML = icon(el.dataset.i); }
function setIcon(btn, n) { const s = $('span', btn) || btn; s.innerHTML = icon(n); }

// ---------------------------------------------------------------- state
const video = $('#video');
const S = {
  view: 'play', queue: [], qi: -1, item: null, probe: null, tools: {}, platform: 'linux', settings: {},
  shuffle: false, repeat: 'off', order: [], prepared: null, subs: [], subOn: -1, resume: {}, pad: false,
  full: false, pick: null, dir: '', places: [], podTab: 'shows', pods: { shows: [], state: {} }, radio: { favs: [] },
  track: 0, art: '', jobs: {}, loadingSrc: false,
  // Discs: the open disc (its titles or tracks), the playing stream's start time, the drives, DVD decoding.
  discInfo: null, disc: null, drives: [], decoding: null, discPanel: false,
};

// ---------------------------------------------------------------- toasts, sheets, menus, on-screen keyboard
let toastTimer;
function toast(text, action) {
  const t = $('#toast');
  t.innerHTML = `<span>${esc(text)}</span>${action ? `<button class="btn f" id="toastAct">${esc(action.label)}</button>` : ''}`;
  t.hidden = false;
  if (action) $('#toastAct').onclick = () => { t.hidden = true; action.run(); };
  clearTimeout(toastTimer); toastTimer = setTimeout(() => { t.hidden = true; }, action ? 7000 : 3200);
}
let sheetStack = [];
function sheet(html, { onClose, wide } = {}) {
  const layer = $('#layer');
  const v = document.createElement('div');
  v.className = 'veil';
  v.innerHTML = `<div class="dlg" role="dialog" ${wide ? 'style="width:min(52em,100%)"' : ''}>${html}</div>`;
  v.addEventListener('click', e => { if (e.target === v) closeSheet(); });
  const prevFocus = document.activeElement;
  layer.appendChild(v);
  paintIcons(v);
  sheetStack.push({ v, onClose, prevFocus });
  setTimeout(() => ($('[data-autofocus]', v) || $('.f', v))?.focus(), 0);
  return v;
}
function closeSheet() {
  const s = sheetStack.pop(); if (!s) return false;
  s.v.remove(); s.onClose?.();
  if (s.prevFocus && document.contains(s.prevFocus)) s.prevFocus.focus();
  return true;
}
const topSheet = () => sheetStack.length ? sheetStack[sheetStack.length - 1].v : null;
// A list of choices; each {label, sub, on, run}. Returns the sheet.
function menu(title, items) {
  const v = sheet(`<h2>${esc(title)}</h2><div class="menu">${items.map((it, i) => it.sep ? '<hr style="border:0;border-top:1px solid var(--line);margin:.3em 0">' :
    `<button class="mi f" data-mi="${i}" ${it.on ? 'data-autofocus' : ''} ${it.disabled ? 'disabled' : ''}><span class="chk">${it.on ? icon('check') : ''}</span><span>${esc(it.label)}</span>${it.sub ? `<small>${esc(it.sub)}</small>` : ''}</button>`).join('')}</div>`);
  v.addEventListener('click', e => { const b = e.target.closest('[data-mi]'); if (!b) return; const it = items[+b.dataset.mi]; if (!it.keep) closeSheet(); it.run?.(); });
  return v;
}
function confirmSheet(title, body, okLabel, { danger } = {}) {
  return new Promise(res => {
    const v = sheet(`<h2>${esc(title)}</h2><div>${body}</div><div class="acts"><button class="btn f" data-c="no">Cancel</button><button class="btn ${danger ? 'danger' : 'primary'} f" data-c="yes" data-autofocus>${esc(okLabel)}</button></div>`, { onClose: () => res(false) });
    v.addEventListener('click', e => { const b = e.target.closest('[data-c]'); if (!b) return; const s = sheetStack.pop(); s.v.remove(); if (s.prevFocus && document.contains(s.prevFocus)) s.prevFocus.focus(); res(b.dataset.c === 'yes'); });
  });
}
// The on-screen keyboard, for typing with a controller (inputs with data-osk).
let oskTarget = null;
function openOsk(input) {
  closeOsk(); oskTarget = input;
  const keys = '1234567890qwertyuiopasdfghjkl:zxcvbnm./-_?&='.split('');
  input.insertAdjacentHTML('afterend', `<div class="osk" id="osk">${keys.map(k => `<button class="f" data-key="${esc(k)}">${esc(k)}</button>`).join('')}<button class="f wide" data-key="space">Space</button><button class="f wide" data-key="back">⌫</button><button class="f wide" data-key="shift">ABC</button><button class="f wide" data-key="done">Done</button></div>`);
  $('#osk').addEventListener('click', e => {
    const b = e.target.closest('[data-key]'); if (!b) return;
    const k = b.dataset.key, i = oskTarget;
    if (k === 'done') { closeOsk(); i.focus(); i.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })); return; }
    if (k === 'back') i.value = i.value.slice(0, -1);
    else if (k === 'space') i.value += ' ';
    else if (k === 'shift') { for (const kb of $$('#osk [data-key]')) if (kb.dataset.key.length === 1) { kb.dataset.key = kb.dataset.key === kb.dataset.key.toLowerCase() ? kb.dataset.key.toUpperCase() : kb.dataset.key.toLowerCase(); kb.textContent = kb.dataset.key; } }
    else i.value += k;
    i.dispatchEvent(new Event('input', { bubbles: true }));
  });
  $('#osk .f').focus();
}
function closeOsk() { $('#osk')?.remove(); oskTarget = null; }

// ---------------------------------------------------------------- views
function setView(v) {
  S.view = v;
  for (const t of $$('.tab')) t.classList.toggle('on', t.dataset.view === v);
  $('#v-play').hidden = v !== 'play'; $('#v-open').hidden = v !== 'open'; $('#v-pods').hidden = v !== 'pods';
  document.body.classList.toggle('view-play', v === 'play');
  if (v === 'open') { renderPlaces(); if (!S.dir) openDir(S.places[0]?.path || S.home); else setTimeout(() => ($('#files .row') || $('#places .place.on'))?.focus(), 0); }
  if (v === 'pods') renderPods();
  hints();
}
function hints() {
  const h = S.view === 'play' && menuOn() ? menuHints() : S.view === 'play' && discPanelOn() ? discHints() : S.view === 'play' && S.item?.disc ? discPlayHints()
    : S.view === 'play'
    ? (S.item ? '<span><b>A</b>Play/pause</span><span><b>◀ ▶</b>Seek</span><span><b>LB RB</b>Previous/next</span><span><b>X</b>Controls</span><span><b>Y</b>Edit</span><span><b>B</b>Back</span>' : '<span><b>A</b>Choose</span>')
    : '<span><b>A</b>Choose</span><span><b>B</b>Back</span><span><b>LB RB</b>Switch tabs</span>';
  $('#hints').innerHTML = h + `<span class="dim" id="hint"></span>`;
}

// ---------------------------------------------------------------- playing
const isVideo = () => S.item && S.item.kind === 'video';
// A DVD title or Video CD part plays as a stream that starts at S.disc.offset: the time in it is that plus the video's.
const restarts = () => !!(S.item?.disc && S.disc?.restart);
const ct = () => (restarts() ? (S.disc.pending ?? (S.disc.offset + (video.currentTime || 0))) : video.currentTime);
function setBodyMode() {
  document.body.classList.toggle('audio', !!S.item && !isVideo());
  document.body.classList.toggle('empty', !S.item);
  $('#emptyStage').hidden = !!S.item;
  $('#music').hidden = !S.item || isVideo();
  $('#shotBtn').hidden = !isVideo(); $('#fullBtn').hidden = !isVideo();
  $('#subsBtn').hidden = !isVideo();
  $('#disc').hidden = !S.discPanel || !S.discInfo;
  document.body.classList.toggle('disc-open', !$('#disc').hidden);
}
function makeOrder() {
  const n = S.queue.length; S.order = [...Array(n).keys()];
  if (S.shuffle) { for (let i = n - 1; i > 0; i--) { const j = Math.floor(Math.random() * (i + 1)); [S.order[i], S.order[j]] = [S.order[j], S.order[i]]; } }
}
async function playQueue(items, start = 0, opts = {}) {
  if (!items.length) { toast('There was nothing to play there.'); return; }
  S.queue = items; makeOrder();
  if (S.shuffle && start) { const k = S.order.indexOf(start); [S.order[0], S.order[k]] = [S.order[k], S.order[0]]; }
  setView('play');
  await playIndex(start, opts);
}
async function openAndPlay(paths, opts) { try { const q = await call('open_paths', { paths }); await playQueue(q, 0, opts); } catch (e) { toast(e.message); } }

async function playIndex(i, { at = null, noResume = false } = {}) {
  if (i < 0 || i >= S.queue.length) return;
  saveResume();
  S.qi = i; S.item = S.queue[i]; S.track++; S.probe = null; S.subs = []; S.subOn = -1; S.prepared = null;
  closeNotice();
  setBodyMode(); hints();
  shownCue = null;
  $('#title').textContent = S.item.name;
  call('set_title', { title: `${S.item.title || S.item.name} - Hearth Player` }).catch(() => {});
  if (S.item.disc) { await playDiscItem(S.item, { at, noResume }); return; }
  S.disc = null;
  let url;
  try { url = await call('media_url', { path: S.item.path }); } catch (e) { notice(e.message); return; }
  await setSource(url);
  const r = S.resume[S.item.path];
  const pod = S.item.podcast && S.pods.state[S.item.podcast.guid];
  const resumeAt = at ?? (!noResume ? (pod?.pos || r?.t || 0) : 0);
  const startPlay = () => {
    if (resumeAt > 5) {
      video.currentTime = resumeAt;
      toast(`Carrying on from ${fmt(resumeAt)}`, { label: 'Start over', run: () => { video.currentTime = 0; video.play(); } });
    }
    video.play().catch(() => {});
  };
  if (video.readyState >= 1) startPlay(); else video.addEventListener('loadedmetadata', startPlay, { once: true });
  if (!isVideo()) renderMusic();
  loadProbe(S.item, S.track);
  updateButtons();
  mpris(true);
}
async function setSource(url) {
  // Frames are drawn into canvases (screenshots, thumbnails) only from Hearth Player's own server, which allows it.
  if (/^http:\/\/127\.0\.0\.1:\d+\/[a-z0-9]+\/f\//.test(url)) video.crossOrigin = 'anonymous'; else video.removeAttribute('crossorigin');
  S.loadingSrc = true;
  video.src = url;
  video.playbackRate = S.speed || 1;
  video.load();
  thumbs.src = null;
}
async function loadProbe(item, track) {
  if (isUrl(item.path) || item.disc) return;
  let p;
  try { p = await call('probe', { path: item.path }); } catch (_) { return; }
  if (track !== S.track) return;
  S.probe = p.info || null;
  S.subs = (p.subs || []).map(s => ({ label: s.label, file: s.path, lang: s.lang }));
  for (const st of S.probe?.streams || []) if (st.kind === 'subtitle') S.subs.push({ label: st.label + (st.text ? '' : ' (picture subtitles: not shown)'), stream: st.index, text: st.text, lang: st.lang, def: st.default || st.forced });
  if (p.tags) { item.tags = p.tags; if (!isVideo()) renderNowPlaying(); }
  renderMarks();
  updateButtons();
  // Subtitles on by default when the file says so (or a forced track), else off; your last choice wins next time.
  const want = S.settings.subLang;
  let pick = -1;
  if (want) pick = S.subs.findIndex(s => s.lang && s.lang.toLowerCase().startsWith(want));
  if (pick < 0) pick = S.subs.findIndex(s => s.def && s.text !== false);
  if (pick >= 0 && S.settings.subsOff !== true) selectSub(pick, true);
}
function renderMarks() {
  const d = duration(); const m = $('#marks'); let h = '';
  if (d > 0 && S.probe?.chapters?.length > 1) for (const c of S.probe.chapters.slice(1)) h += `<i style="left:${(c.start / d) * 100}%" title="${esc(c.title)}"></i>`;
  if (E.on && d > 0) { h += `<i class="in" style="left:${(E.s.start / d) * 100}%"></i><i class="out" style="left:${((E.s.end || d) / d) * 100}%"></i>`; }
  m.innerHTML = h;
  const keep = $('#keep'), cut = $('#cutBar');
  keep.hidden = !E.on; cut.hidden = !(E.on && E.s.cut);
  if (E.on && d > 0) {
    const a = E.s.start / d, b = (E.s.end || d) / d; keep.style.left = `${a * 100}%`; keep.style.width = `${(b - a) * 100}%`;
    if (E.s.cut) { cut.style.left = `${(E.s.cut[0] / d) * 100}%`; cut.style.width = `${((E.s.cut[1] - E.s.cut[0]) / d) * 100}%`; }
  }
}
const duration = () => (S.item?.disc && S.disc ? S.disc.duration : isFinite(video.duration) && video.duration > 0 ? video.duration : (S.probe?.duration || 0));
const live = () => !!S.item && !S.item.disc && video.duration === Infinity;

function updateButtons() {
  setIcon($('#ppBtn'), video.paused ? 'play' : 'pause');
  setIcon($('#muteBtn'), video.muted || video.volume === 0 ? 'mute' : 'vol');
  $('#vol').value = Math.round((video.muted ? 0 : video.volume) * 100);
  $('#shufBtn').classList.toggle('on', S.shuffle);
  $('#repBtn').classList.toggle('on', S.repeat !== 'off'); setIcon($('#repBtn'), S.repeat === 'one' ? 'repeat1' : 'repeat');
  $('#speedBtn').textContent = `${+(video.playbackRate || 1).toFixed(2)}×`;
  const audioN = (S.probe?.streams || []).filter(s => s.kind === 'audio').length;
  $('#audioBtn').hidden = audioN < 2;
  $('#chapBtn').hidden = !(S.probe?.chapters?.length > 1);
  $('#subsBtn').classList.toggle('on', S.subOn >= 0);
  setIcon($('#fullBtn'), S.full ? 'unfull' : 'full');
  $('[data-act="prev"]').disabled = !S.item; $('[data-act="next"]').disabled = !S.item || (S.queue.length < 2 && S.repeat === 'off');
  $('#editBtn').disabled = !S.item || isUrl(S.item.path) || !!S.item.disc;
  $('#discBtn').hidden = !S.discInfo || !S.item?.disc;
  $('#ejectBtn').hidden = !S.discInfo?.device;
  if (S.item?.disc) $('#shotBtn').hidden = !isVideo();
}
function tick() {
  const d = duration(), t = ct();
  $('#tCur').textContent = fmt(t);
  $('#tDur').textContent = live() ? 'LIVE' : fmt(d);
  const pct = d > 0 && !live() ? clamp(t / d, 0, 1) : 0;
  $('#done').style.width = `${pct * 100}%`; $('#knob').style.left = `${pct * 100}%`;
  if (restarts()) $('#buf').style.width = '0%';
  else if (video.buffered.length && d > 0) $('#buf').style.width = `${clamp(video.buffered.end(video.buffered.length - 1) / d, 0, 1) * 100}%`;
  $('#seek').setAttribute('aria-valuetext', `${fmt(t)} of ${fmt(d)}`);
  if (!isVideo()) for (const r of $$('.trk')) r.classList.toggle('on', r.dataset.path === S.item?.path);
}
video.addEventListener('timeupdate', () => { tick(); periodic(); });
video.addEventListener('durationchange', () => { tick(); renderMarks(); });
video.addEventListener('progress', tick);
video.addEventListener('play', () => { updateButtons(); call('keep_awake', { mode: isVideo() ? 'video' : 'audio' }).catch(() => {}); mpris(); });
video.addEventListener('pause', () => { updateButtons(); call('keep_awake', { mode: '' }).catch(() => {}); saveResume(); mpris(); wake(); });
video.addEventListener('volumechange', () => { const snd = $('#menuSound'); if (snd) { snd.volume = video.volume; snd.muted = video.muted; } updateButtons(); S.settings.volume = video.volume; S.settings.muted = video.muted; saveSettings(); mpris(); });
video.addEventListener('ratechange', () => { updateButtons(); mpris(); });
video.addEventListener('seeked', () => { mpris(false, true); });
video.addEventListener('loadedmetadata', () => { S.loadingSrc = false; tick(); renderMarks(); updateButtons(); });
video.addEventListener('ended', () => {
  call('keep_awake', { mode: '' }).catch(() => {});
  if (discEnded()) return;
  if (S.item) { delete S.resume[S.item.path]; saveResumeStore(); if (S.item.podcast) podMark(S.item.podcast.guid, { played: true, pos: 0 }); }
  if (S.repeat === 'one') { video.currentTime = 0; video.play(); return; }
  next(true);
});
video.addEventListener('error', () => {
  if (!S.item || !video.src) return;
  const code = video.error?.code;
  if (S.item.disc) { discError(code); return; }
  // A file the web view can't play: make a copy it can (ffmpeg), once.
  if (!isUrl(S.item.path) && S.tools.ffmpeg && !S.prepared && (code === 4 || code === 3)) { prepare(S.item, null); return; }
  const why = code === 4 || code === 3 ? (S.tools.ffmpeg ? 'This file couldn\'t be played, even after converting it.' : 'This kind of file needs codecs that aren\'t installed (and ffmpeg isn\'t there to convert it).')
    : code === 2 ? 'The connection broke off.' : 'This couldn\'t be played.';
  notice(why);
});

function notice(text, { progress, stop, actions } = {}) {
  const n = $('#notice'); n.hidden = false;
  n.innerHTML = `<span>${esc(text)}</span>${progress != null ? `<div class="prog"><i style="width:${Math.round(progress * 100)}%"></i></div>` : ''}${stop ? '<button class="btn f" data-act="noticeStop">Stop</button>' : ''}${(actions || []).map((a, i) => `<button class="btn f" data-na="${i}">${esc(a.label)}</button>`).join('')}`;
  n.onclick = e => { const b = e.target.closest('[data-na]'); if (b) actions[+b.dataset.na].run(); if (e.target.closest('[data-act="noticeStop"]')) stop(); };
}
function closeNotice() { $('#notice').hidden = true; }

// Getting a file ready to play (or another sound track) with ffmpeg.
async function prepare(item, audio) {
  const keepAt = video.currentTime || 0;
  S.prepared = { audio };
  try {
    const id = await call('prepare_start', { path: item.path, audio });
    const track = S.track;
    S.jobs[id] = {
      progress: (pct, text) => { if (track === S.track) notice(text, { progress: pct, stop: () => call('job_cancel', { id }) }); },
      done: async r => {
        if (track !== S.track) return;
        if (!r.ok) { notice(r.error); return; }
        closeNotice();
        const url = await call('media_url', { path: r.result.path });
        await setSource(url);
        video.addEventListener('loadedmetadata', () => { if (keepAt > 1) video.currentTime = keepAt; video.play().catch(() => {}); }, { once: true });
      },
    };
  } catch (e) { notice(e.message); }
}

function playPause() {
  if (!S.item) { setView('open'); return; }
  if (video.paused) video.play().catch(e => toast(e.message)); else video.pause();
}
function seekTo(t) { if (!S.item || live()) return; if (restarts()) { discSeek(t); return; } const d = duration(); video.currentTime = clamp(t, 0, d > 0 ? d - 0.05 : t); osd(`${fmt(video.currentTime)} / ${fmt(d)}`); tick(); }
const seekBy = dt => seekTo(ct() + dt);
function next(auto = false) {
  if (!S.queue.length) return;
  if (!auto && S.item?.disc?.kind === 'dvd' && S.queue.length < 2) { chapterStep(1); return; }
  const k = S.order.indexOf(S.qi);
  if (k < S.order.length - 1) playIndex(S.order[k + 1]);
  else if (S.repeat === 'all') { if (S.shuffle) makeOrder(); playIndex(S.order[0]); }
  else if (!auto) toast('That was the last one.');
}
function prev() {
  if (!S.queue.length) return;
  if (S.item?.disc?.kind === 'dvd' && S.queue.length < 2) { chapterStep(-1); return; }
  if (ct() > 4) { seekTo(0); return; }
  const k = S.order.indexOf(S.qi);
  if (k > 0) playIndex(S.order[k - 1]); else seekTo(0);
}
function setVolume(v) { video.muted = false; video.volume = clamp(v, 0, 1); osd(`Volume ${Math.round(video.volume * 100)}%`); }
const SPEEDS = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 3];
function setSpeed(s) { S.speed = s; video.playbackRate = s; osd(`Speed ${s}×`); }
function speedStep(dir) { const c = video.playbackRate || 1; const i = SPEEDS.findIndex(s => s >= c - 1e-6); setSpeed(SPEEDS[clamp((i < 0 ? 3 : i) + dir, 0, SPEEDS.length - 1)]); }
let osdTimer;
function osd(text) { const o = $('#osd'); o.textContent = text; o.classList.add('show'); clearTimeout(osdTimer); osdTimer = setTimeout(() => o.classList.remove('show'), 1100); }

// Subtitles: the app turns them into WebVTT (files beside the video, or text streams inside it, via ffmpeg) and the
// page draws the cues itself over the video, the same in every web view.
function parseVtt(text) {
  const t = s => { const m = /(?:(\d+):)?(\d{1,2}):(\d{2})[.,](\d{1,3})/.exec(s); return m ? (+(m[1] || 0)) * 3600 + (+m[2]) * 60 + (+m[3]) + (+m[4].padEnd(3, '0')) / 1000 : NaN; };
  const cues = [];
  for (const block of String(text).replace(/\r/g, '').split(/\n{2,}/)) {
    const lines = block.split('\n');
    const i = lines.findIndex(l => l.includes('-->'));
    if (i < 0) continue;
    const [a, b] = lines[i].split('-->');
    const start = t(a), end = t(b);
    if (!isFinite(start) || !isFinite(end)) continue;
    // Only <i>, <b> and <u> are kept; everything else is shown as text.
    const html = lines.slice(i + 1).join('\n').split(/(<\/?[ibu]>)/).map(p => (/^<\/?[ibu]>$/.test(p) ? p : esc(p.replace(/<[^>]*>/g, '').replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>')))).join('').replace(/\n/g, '<br>');
    cues.push({ start, end, html });
  }
  return cues.sort((x, y) => x.start - y.start);
}
let shownCue = '';
function drawCues() {
  const el = $('#cues');
  const s = S.subOn >= 0 ? S.subs[S.subOn] : null;
  const t = video.currentTime;
  const html = s && s.cues ? s.cues.filter(c => t >= c.start && t < c.end).map(c => `<span>${c.html}</span>`).join('<br>') : '';
  if (html !== shownCue) { el.innerHTML = html; shownCue = html; }
}
video.addEventListener('timeupdate', drawCues);
video.addEventListener('seeked', drawCues);
async function selectSub(i, quiet = false) {
  S.subOn = -1;
  if (i < 0) { S.settings.subsOff = true; saveSettings(); if (S.item?.disc?.sub != null) discSub(-1); updateButtons(); drawCues(); if (!quiet) osd('Subtitles off'); return; }
  const s = S.subs[i]; if (!s) return;
  if (s.disc != null) { discSub(i); return; }
  if (s.text === false) { toast('Picture subtitles (Blu-ray and DVD) can\'t be shown yet. Try a text track or a subtitle file.'); return; }
  try {
    if (!s.cues) {
      const vtt = s.file ? await call('subtitle_vtt', { path: s.file }) : s.extFile ? await call('subtitle_file_vtt', { path: s.extFile }) : await call('subtitle_vtt', { path: S.item.path, stream: s.stream });
      s.cues = parseVtt(vtt);
      if (!s.cues.length) { toast('There are no subtitles in that track.'); return; }
    }
    S.subOn = i;
    S.settings.subsOff = false; if (s.lang) S.settings.subLang = s.lang.toLowerCase().slice(0, 2); saveSettings();
    if (!quiet) osd(s.label);
  } catch (e) { toast(e.message); }
  drawCues();
  updateButtons();
}
function subsMenu() {
  const items = [{ label: 'Off', on: S.subOn < 0, run: () => selectSub(-1) }, ...S.subs.map((s, i) => ({ label: s.label, on: S.subOn === i, run: () => selectSub(i) }))];
  items.push({ sep: true }, { label: 'Load a subtitle file…', run: () => pickFile('subs', p => { S.subs.push({ label: `${baseName(p)}`, extFile: p }); selectSub(S.subs.length - 1); }) });
  const sizes = [['Small', 0.8], ['Normal', 1], ['Large', 1.3], ['Huge', 1.7]];
  items.push({ sep: true }, ...sizes.map(([l, v]) => ({ label: `Text size: ${l}`, on: (S.settings.cue || 1) === v, run: () => { S.settings.cue = v; applyCue(); saveSettings(); } })));
  menu('Subtitles', items);
}
const applyCue = () => document.documentElement.style.setProperty('--cue', `${S.settings.cue || 1}`);
function cycleSubs() { if (!S.subs.length) { osd('No subtitles'); return; } selectSub(S.subOn + 1 >= S.subs.length ? -1 : S.subOn + 1); }

function audioTracks() { return (S.probe?.streams || []).filter(s => s.kind === 'audio'); }
function currentAudio() {
  if (S.item?.disc) { const i = audioTracks().findIndex(a => a.id === S.item.disc.audio); return i < 0 ? 0 : i; }
  if (video.audioTracks && video.audioTracks.length > 1) { for (let i = 0; i < video.audioTracks.length; i++) if (video.audioTracks[i].enabled) return i; }
  return S.prepared?.audio ?? 0;
}
function selectAudio(n) {
  if (S.item?.disc) { discAudio(n); return; }
  const at = video.audioTracks;
  if (at && at.length > 1) { for (let i = 0; i < at.length; i++) at[i].enabled = i === n; osd(audioTracks()[n]?.label || `Sound ${n + 1}`); return; }
  if (!S.tools.ffmpeg) { toast('Switching sound tracks in this file needs ffmpeg.'); return; }
  prepare(S.item, n);
}
function audioMenu() { const cur = currentAudio(); menu('Sound track', audioTracks().map((s, i) => ({ label: s.label, on: i === cur, run: () => selectAudio(i) }))); }
function chaptersMenu() {
  const t = ct();
  const ch = S.probe?.chapters || [];
  menu('Chapters', ch.map(c => ({ label: c.title, sub: fmt(c.start), on: t >= c.start && t < c.end, run: () => seekTo(c.start + 0.01) })));
}
function chapterStep(dir) {
  const ch = S.probe?.chapters || []; if (ch.length < 2) { seekBy(dir * 60); return; }
  const t = ct();
  const i = ch.findIndex(c => t >= c.start - 0.5 && t < c.end);
  const j = clamp((i < 0 ? 0 : i) + dir, 0, ch.length - 1);
  seekTo(ch[j].start + 0.01); osd(ch[j].title);
}
function speedMenu() { const c = video.playbackRate || 1; menu('Speed', SPEEDS.map(s => ({ label: `${s}×${s === 1 ? ' (normal)' : ''}`, on: Math.abs(s - c) < 1e-6, run: () => setSpeed(s) }))); }
function queueMenu() {
  const items = S.order.map(i => ({ label: S.queue[i].title || S.queue[i].name, on: i === S.qi, run: () => playIndex(i) }));
  items.push({ sep: true }, { label: 'Save as a playlist…', run: savePlaylist });
  menu(`Play queue (${S.queue.length})`, items.length > 2 ? items : [{ label: 'Nothing in the queue', disabled: true }]);
}
async function savePlaylist() {
  const dir = S.item && !isUrl(S.item.path) ? dirName(S.item.path) : S.home;
  const name = await askText('Save the queue as a playlist', 'Name', 'Playlist');
  if (!name) return;
  try { const p = await call('save_playlist', { path: `${dir}/${name}.m3u8`, items: S.queue.map(q => q.path) }); toast(`Saved ${baseName(p)}`); } catch (e) { toast(e.message); }
}
function askText(title, label, value) {
  return new Promise(res => {
    const v = sheet(`<h2>${esc(title)}</h2><label class="dim">${esc(label)}</label><input class="inp f" id="askIn" value="${esc(value)}" data-osk="text" spellcheck="false" data-autofocus><div class="acts"><button class="btn f" data-c="no">Cancel</button><button class="btn primary f" data-c="yes">OK</button></div>`, { onClose: () => res(null) });
    const done = ok => { const val = $('#askIn', v).value.trim(); const s = sheetStack.pop(); s.v.remove(); res(ok ? val : null); };
    v.addEventListener('click', e => { const b = e.target.closest('[data-c]'); if (b) done(b.dataset.c === 'yes'); });
    $('#askIn', v).addEventListener('keydown', e => { if (e.key === 'Enter') { e.preventDefault(); e.stopPropagation(); done(true); } });
  });
}

// Full screen: the window goes full screen; the controls hide when nothing moves.
async function setFull(on) {
  try { S.full = await call('fullscreen', { on }); } catch (_) { S.full = on; if (!T) { /* browser preview */ } }
  document.body.classList.toggle('full', S.full);
  updateButtons(); wake();
}
let idleTimer;
function wake() {
  document.body.classList.remove('idle');
  clearTimeout(idleTimer);
  if (S.full && isVideo() && !video.paused && S.view === 'play') idleTimer = setTimeout(() => { if (!topSheet() && E.on !== true) document.body.classList.add('idle'); }, 2800);
}
document.addEventListener('mousemove', wake);

// Screenshot of the current frame.
async function screenshot() {
  if (!isVideo()) return;
  const at = ct();
  let png = null;
  try {
    const c = document.createElement('canvas'); c.width = video.videoWidth; c.height = video.videoHeight;
    c.getContext('2d').drawImage(video, 0, 0); png = c.toDataURL('image/png');
  } catch (_) { png = null; }
  try { const p = await call('save_frame', { path: S.item.path, at, png }); toast(`Saved ${baseName(p)}`); osd('Picture saved'); } catch (e) { toast(e.message); }
}

// Seek bar: click, drag, hover thumbnails, keys when focused.
const seekEl = $('#seek');
const thumbs = { vid: null, src: null, busy: false, want: null };
function seekFrac(e) { const r = seekEl.getBoundingClientRect(); return clamp((e.clientX - r.left) / r.width, 0, 1); }
let dragging = false;
seekEl.addEventListener('pointerdown', e => { if (!S.item) return; dragging = true; seekEl.setPointerCapture(e.pointerId); seekTo(seekFrac(e) * duration()); });
seekEl.addEventListener('pointermove', e => { if (dragging) seekTo(seekFrac(e) * duration()); showThumb(seekFrac(e)); });
seekEl.addEventListener('pointerup', () => { dragging = false; });
seekEl.addEventListener('pointerleave', () => { $('#thumb').hidden = true; });
seekEl.addEventListener('focus', () => showThumb(duration() ? ct() / duration() : 0));
seekEl.addEventListener('blur', () => { $('#thumb').hidden = true; });
function showThumb(frac) {
  const d = duration(); if (!d || live()) return;
  const t = frac * d, th = $('#thumb');
  th.hidden = false; th.style.left = `${clamp(frac * 100, 8, 92)}%`;
  const ch = (S.probe?.chapters || []).find(c => t >= c.start && t < c.end);
  $('#thumbTime').textContent = `${fmt(t)}${ch && S.probe.chapters.length > 1 ? ` · ${ch.title}` : ''}`;
  // Not for disc streams: a picture from elsewhere in the title would start a second stream from the disc.
  const canDraw = isVideo() && video.crossOrigin === 'anonymous' && !S.item?.disc;
  th.classList.toggle('nopic', !canDraw);
  if (!canDraw) return;
  if (!thumbs.vid) { thumbs.vid = document.createElement('video'); thumbs.vid.muted = true; thumbs.vid.preload = 'auto'; thumbs.vid.crossOrigin = 'anonymous';
    thumbs.vid.addEventListener('seeked', () => { try { $('#thumbCanvas').getContext('2d').drawImage(thumbs.vid, 0, 0, 192, 108); } catch (_) {} thumbs.busy = false; if (thumbs.want != null) { const w = thumbs.want; thumbs.want = null; thumbAt(w); } }); }
  if (thumbs.src !== video.currentSrc) { thumbs.src = video.currentSrc; thumbs.vid.src = video.currentSrc; }
  thumbAt(t);
}
function thumbAt(t) { if (thumbs.busy) { thumbs.want = t; return; } thumbs.busy = true; try { thumbs.vid.currentTime = t; } catch (_) { thumbs.busy = false; } setTimeout(() => { thumbs.busy = false; }, 800); }

// Remembering where you stopped (videos, long songs, podcasts).
let lastSave = 0;
function periodic() {
  const now = Date.now();
  if (now - lastSave > 5000) { lastSave = now; saveResume(); mpris(); }
}
function saveResume() {
  if (!S.item || live()) return;
  const t = ct(), d = duration();
  if (S.item.podcast) { if (t > 3) podMark(S.item.podcast.guid, { pos: t, dur: d, played: d > 0 && d - t < 30 }); return; }
  if (!(d > 120) || isUrl(S.item.path)) return;
  if (t > 5 && d - t > 15) S.resume[S.item.path] = { t: Math.round(t), d: Math.round(d), at: Date.now() };
  else delete S.resume[S.item.path];
  saveResumeStore();
}
let resumeTimer;
function saveResumeStore() {
  clearTimeout(resumeTimer);
  resumeTimer = setTimeout(() => {
    const keys = Object.keys(S.resume);
    if (keys.length > 500) keys.sort((a, b) => S.resume[a].at - S.resume[b].at).slice(0, keys.length - 500).forEach(k => delete S.resume[k]);
    call('store_set', { name: 'resume', value: S.resume }).catch(() => {});
  }, 400);
}
let settingsTimer;
function saveSettings() { clearTimeout(settingsTimer); settingsTimer = setTimeout(() => call('store_set', { name: 'settings', value: S.settings }).catch(() => {}), 500); }

// Media keys and Mantel's lock screen (MPRIS), and the web view's own Media Session.
let lastMpris = '';
function mpris(newTrack = false, seeked = false) {
  if (!S.item) return;
  const tg = S.item.tags || {};
  const st = {
    status: video.paused ? (ct() > 0 ? 'Paused' : 'Stopped') : 'Playing',
    title: tg.title || S.item.title || S.item.name, artist: tg.artist || S.item.artist || '', album: tg.album || S.item.album || '',
    artUrl: S.art || S.item.image || '', url: isUrl(S.item.path) ? S.item.path : `file://${S.item.path}`,
    length: live() ? 0 : duration(), position: ct() || 0, canNext: S.queue.length > 1 || S.repeat === 'all', canPrev: S.queue.length > 0,
    rate: video.playbackRate || 1, volume: video.muted ? 0 : video.volume, track: S.track, seeked,
  };
  const key = JSON.stringify({ ...st, position: Math.round(st.position / 5) });
  if (key === lastMpris && !seeked && !newTrack) return;
  lastMpris = key;
  call('now_playing', { state: st }).catch(() => {});
  if ('mediaSession' in navigator) {
    try {
      navigator.mediaSession.metadata = new MediaMetadata({ title: st.title, artist: st.artist, album: st.album, artwork: st.artUrl ? [{ src: st.artUrl }] : [] });
      navigator.mediaSession.playbackState = video.paused ? 'paused' : 'playing';
    } catch (_) {}
  }
}
if ('mediaSession' in navigator) {
  const ms = navigator.mediaSession;
  const h = (a, f) => { try { ms.setActionHandler(a, f); } catch (_) {} };
  h('play', () => video.play()); h('pause', () => video.pause()); h('nexttrack', () => next()); h('previoustrack', () => prev());
  h('seekto', d => seekTo(d.seekTime)); h('seekbackward', () => seekBy(-10)); h('seekforward', () => seekBy(10));
}
listen('player://remote', ev => {
  const { cmd, value, uri } = ev.payload || {};
  switch (cmd) {
    case 'playpause': playPause(); break;
    case 'play': if (S.item) video.play(); break;
    case 'pause': video.pause(); break;
    case 'stop': video.pause(); seekTo(0); break;
    case 'next': next(); break;
    case 'previous': prev(); break;
    case 'seekby': seekBy(value || 0); break;
    case 'seekto': seekTo(value || 0); break;
    case 'rate': setSpeed(clamp(value || 1, 0.25, 4)); break;
    case 'volume': setVolume(value || 0); break;
    case 'open': if (uri) openAndPlay([uri]); break;
  }
});
listen('player://job', ev => { const j = S.jobs[ev.payload.id]; j?.progress?.(ev.payload.pct, ev.payload.text); });
listen('player://job-done', ev => { const j = S.jobs[ev.payload.id]; delete S.jobs[ev.payload.id]; j?.done?.(ev.payload); });

// ---------------------------------------------------------------- music view
async function renderMusic() {
  renderNowPlaying();
  const item = S.item;
  S.art = '';
  $('#cover').hidden = true; $('#coverNone').hidden = false;
  if (item.image) { $('#cover').src = item.image; $('#cover').hidden = false; $('#coverNone').hidden = true; S.art = item.image; }
  else if (!isUrl(item.path) && !item.disc) call('art_url', { path: item.path }).then(u => { if (u && S.item === item) { $('#cover').src = u; $('#cover').hidden = false; $('#coverNone').hidden = true; S.art = u; mpris(true); } }).catch(() => {});
  // The list: the play queue when it's a playlist or podcast, else the songs in this song's folder.
  let list = S.queue;
  $('#coverNone').innerHTML = icon(item.disc ? 'disc' : 'note');
  if (item.disc) { /* a CD's songs: the play queue */ }
  else if (!isUrl(item.path) && S.queue.length <= 1) {
    try { const t = await call('folder_tracks', { path: item.path }); if (t.length > 1) { list = t; S.queue = t; S.qi = t.findIndex(x => x.path === item.path); makeOrder(); } } catch (_) {}
  }
  if (S.item !== item) return;
  $('#tracks').innerHTML = list.map((t, i) => `<button class="trk f" data-path="${esc(t.path)}" data-q="${i}"><span class="n">${i + 1}</span><span>${esc(t.title || t.name)}</span><span class="d">${t.disc ? fmt(t.duration) : t.kind === 'video' ? 'video' : ''}</span></button>`).join('');
  tick(); updateButtons();
}
function renderNowPlaying() {
  const it = S.item, tg = it.tags || {};
  $('#mTitle').textContent = tg.title || it.title || it.name.replace(/\.[^.]+$/, '');
  $('#mArtist').textContent = tg.artist || it.artist || (it.radio ? 'Internet radio' : '');
  $('#mAlbum').textContent = [tg.album || it.album, tg.year].filter(Boolean).join(' · ');
}
$('#tracks').addEventListener('click', e => { const b = e.target.closest('.trk'); if (b) playIndex(+b.dataset.q); });

// ---------------------------------------------------------------- open (and picking a file for the edit panel)
function renderPlaces() {
  const ico = { videos: 'film', music: 'note', downloads: 'download', home: 'home', podcasts: 'radio', drive: 'drive' };
  $('#places').innerHTML = S.places.map(p => `<button class="place f ${p.path === S.placeOn ? 'on' : ''}" data-place="${esc(p.path)}">${icon(ico[p.kind] || 'folder')}<span>${esc(p.name)}</span></button>`).join('')
    + S.drives.map(d => `<button class="place f" data-drive="${esc(d.device)}">${icon(d.media ? 'disc' : 'drive')}<span>${esc(d.media && d.label ? d.label : d.name)}<small class="dim" style="display:block;font-size:.78em">${esc(driveLine(d))}</small></span></button>`).join('');
}
async function openDir(path, focusName) {
  if (!path) return;
  try {
    const list = await call('list_dir', { path, filter: S.pick?.filter || 'media' });
    S.dir = path; S.entries = list;
    S.placeOn = S.places.filter(p => path.startsWith(p.path)).sort((a, b) => b.path.length - a.path.length)[0]?.path;
    renderPlaces();
    $('#path').textContent = path;
    $('#playFolder').textContent = S.pick ? (S.pick.filter === 'folder' ? 'Save here' : 'Cancel') : 'Play this folder';
    $('#playFolder').disabled = !S.pick && !list.some(e => !e.dir);
    const icoFor = e => e.dir ? 'folder' : e.kind === 'video' ? 'film' : e.kind === 'image' ? 'camera' : e.kind === 'subtitle' ? 'cc' : e.kind === 'playlist' ? 'queue' : 'note';
    $('#files').innerHTML = list.length ? list.map((e, i) => `<button class="row f ${e.dir ? 'folder' : ''}" data-e="${i}"><span class="ico">${icon(icoFor(e))}</span><span>${esc(e.name)}</span><span class="meta">${e.dir ? '' : human(e.size)}</span></button>`).join('')
      : `<div class="empty">${S.pick ? 'Nothing to pick in this folder.' : 'No videos or music in this folder.'}</div>`;
    const target = focusName ? $$('#files .row').find(r => list[+r.dataset.e]?.name === focusName) : $('#files .row');
    (target || $('#places .place.on'))?.focus();
  } catch (e) { toast(e.message); }
}
function human(b) { const u = ['bytes', 'KB', 'MB', 'GB', 'TB']; let v = b, i = 0; while (v >= 1000 && i < u.length - 1) { v /= 1000; i++; } return `${v.toFixed(v < 10 && i ? 1 : 0)} ${u[i]}`; }
function upDir() { const p = dirName(S.dir); if (p && p !== S.dir) openDir(p, baseName(S.dir)); else if (S.dir !== '/') openDir('/'); }
$('#places').addEventListener('click', e => { const dv = e.target.closest('[data-drive]'); if (dv) { openDisc(dv.dataset.drive); return; } const b = e.target.closest('[data-place]'); if (b) openDir(b.dataset.place); });
$('#files').addEventListener('click', e => {
  const b = e.target.closest('[data-e]'); if (!b) return;
  const en = S.entries[+b.dataset.e];
  if (en.dir) { openDir(en.path); return; }
  if (S.pick) { const p = S.pick; S.pick = null; S.dir = ''; setView(p.back); p.done(en.path); return; }
  // The queue: this folder's files of the same sort (songs after a song, videos after a video).
  const media = S.entries.filter(x => !x.dir && x.kind !== 'playlist' && (x.kind === 'video') === (en.kind === 'video'));
  if (en.kind === 'playlist') openAndPlay([en.path]);
  else { const items = media.map(x => ({ path: x.path, name: x.name, kind: x.kind === 'video' ? 'video' : 'audio' })); playQueue(items, items.findIndex(x => x.path === en.path)); }
});
// Picks a file in the Open view (a cover picture, a subtitle file), then comes back.
function pickFile(filter, done) {
  const back = S.view; const editOpen = E.on;
  S.pick = { filter, done: p => { if (editOpen) $('#editPanel').hidden = false; done(p); }, back };
  if (editOpen) $('#editPanel').hidden = true;
  const start = S.item && !isUrl(S.item.path) ? dirName(S.item.path) : S.places[0]?.path;
  S.dir = '';
  setView('open'); openDir(start);
  toast(filter === 'images' ? 'Pick a JPEG or PNG picture' : 'Pick a subtitle file');
}
function cancelPick() { if (!S.pick) return false; const p = S.pick; S.pick = null; S.dir = ''; if (E.on) $('#editPanel').hidden = false; setView(p.back); return true; }

// ---------------------------------------------------------------- edit panel (trim, cut out, rotate, mute, fades, tags)
const E = { on: false, s: null, undo: [], redo: [], tags: null, tagsOrig: null, cover: { file: null, remove: false }, busy: false };
function editDefaults() { return { start: 0, end: 0, cut: null, rotate: 0, hflip: false, vflip: false, mute: false, fadeIn: 0, fadeOut: 0, normalize: false, exact: false }; }
async function openEdit() {
  if (S.item?.disc) { toast('Discs can\'t be edited. Copy the film to a file first.'); return; }
  if (!S.item || isUrl(S.item.path)) { toast('Only files on this computer can be edited.'); return; }
  if (!S.tools.ffmpeg && isVideo()) { toast('Editing videos needs ffmpeg, which isn\'t installed.'); }
  E.on = true; E.s = editDefaults(); E.undo = []; E.redo = []; E.cover = { file: null, remove: false };
  E.tags = null;
  if (!isVideo()) { try { E.tags = await call('tags_read', { path: S.item.path }); } catch (_) { E.tags = { title: '', artist: '', album: '', genre: '', track: null, year: null }; } E.tagsOrig = JSON.stringify(E.tags); }
  $('#editPanel').hidden = false;
  renderEdit(); renderMarks(); applyPreview();
  setTimeout(() => $('#editPanel .f')?.focus(), 0);
}
function closeEdit(force = false) {
  if (!E.on) return;
  if (!force && (E.undo.length || tagsChanged())) {
    confirmSheet('Leave without saving?', '<p>Your edits haven\'t been saved.</p>', 'Leave', { danger: true }).then(ok => { if (ok) closeEdit(true); });
    return;
  }
  E.on = false; $('#editPanel').hidden = true; renderMarks(); applyPreview(); $('#editBtn').focus();
}
const tagsChanged = () => E.tags && (JSON.stringify(E.tags) !== E.tagsOrig || E.cover.file || E.cover.remove);
function change(fn) { E.undo.push(JSON.stringify(E.s)); if (E.undo.length > 200) E.undo.shift(); E.redo = []; fn(E.s); fixMarks(); renderEdit(); renderMarks(); applyPreview(); }
function undo() { if (!E.undo.length) return; E.redo.push(JSON.stringify(E.s)); E.s = JSON.parse(E.undo.pop()); renderEdit(); renderMarks(); applyPreview(); osd('Undone'); }
function redo() { if (!E.redo.length) return; E.undo.push(JSON.stringify(E.s)); E.s = JSON.parse(E.redo.pop()); renderEdit(); renderMarks(); applyPreview(); osd('Redone'); }
function fixMarks() {
  const d = duration(), s = E.s;
  s.start = clamp(s.start, 0, d); if (s.end) s.end = clamp(s.end, 0, d);
  if (s.end && s.end <= s.start + 0.1) s.end = Math.min(d, s.start + 0.1);
  if (s.cut) { s.cut = [clamp(s.cut[0], s.start, s.end || d), clamp(s.cut[1], s.start, s.end || d)].sort((a, b) => a - b); if (s.cut[1] - s.cut[0] < 0.05) s.cut = null; }
}
function applyPreview() {
  const s = E.on ? E.s : editDefaults();
  const r = s.rotate % 360, side = r === 90 || r === 270;
  const scale = side && video.videoWidth ? Math.min(video.clientHeight / video.clientWidth, video.clientWidth / video.clientHeight) : 1;
  video.style.transform = `rotate(${r}deg) scale(${(s.hflip ? -1 : 1) * scale}, ${(s.vflip ? -1 : 1) * scale})`;
  if (E.on && isVideo()) video.muted = s.mute || S.settings.muted; else if (!E.on) video.muted = !!S.settings.muted;
}
const tog = (key, label, sub, on) => `<button class="tog f ${on ? 'on' : ''}" data-tog="${key}"><span class="box"></span><span>${esc(label)}${sub ? `<small>${esc(sub)}</small>` : ''}</span></button>`;
const numRow = (key, label, v, unit) => `<div class="num"><span>${esc(label)}</span><button class="ic f" data-num="${key}" data-d="-0.5" title="Less">−</button><span class="v">${v ? `${v.toFixed(1)} ${unit}` : 'off'}</span><button class="ic f" data-num="${key}" data-d="0.5" title="More">+</button></div>`;
function renderEdit() {
  const s = E.s, d = duration(), vid = isVideo();
  const markRow = (label, t, setAct, clearAct) => `<div class="markrow"><span>${label}</span><span class="tv">${fmtPrecise(t)}</span><span class="btns"><button class="btn f" data-ed="${setAct}" title="Set to where it's playing">Set here</button>${clearAct ? `<button class="ic f" data-ed="${clearAct}" title="Reset">${icon('x')}</button>` : ''}</span></div>`;
  const cutRows = s.cut ? markRow('From', s.cut[0], 'cutA', null) + markRow('To', s.cut[1], 'cutB', null) + `<button class="btn f" data-ed="cutOff">Don't cut anything out</button>` : `<button class="btn f" data-ed="cutOn">Cut out a part…</button>`;
  const tg = E.tags;
  const coverImg = E.cover.remove ? '' : E.cover.file ? `<span class="noimg">${icon('camera')}</span>` : (S.art ? `<img src="${esc(S.art)}" alt="">` : '');
  const tagsHtml = tg ? `<fieldset><legend>Song details (tags)</legend><div class="tags">
      ${[['title', 'Title'], ['artist', 'Artist'], ['album', 'Album'], ['genre', 'Genre']].map(([k, l]) => `<label for="tg-${k}">${l}</label><input class="inp f" id="tg-${k}" data-tag="${k}" value="${esc(tg[k] || '')}" data-osk="tag" spellcheck="false">`).join('')}
      <label for="tg-track">Track</label><input class="inp f" id="tg-track" data-tag="track" inputmode="numeric" value="${tg.track ?? ''}" data-osk="tag">
      <label for="tg-year">Year</label><input class="inp f" id="tg-year" data-tag="year" inputmode="numeric" value="${tg.year ?? ''}" data-osk="tag">
    </div>
    <div class="coverpick">${coverImg || `<span class="noimg">${icon('note')}</span>`}<div class="btns"><button class="btn f" data-ed="coverPick">${E.cover.file ? baseName(E.cover.file) : 'Choose a cover…'}</button><button class="btn f" data-ed="coverRemove">Remove cover</button></div></div>
    <button class="btn primary f" data-ed="saveTags" ${tagsChanged() ? '' : 'disabled'}>Save details</button><small class="dim">Only the tags change; the sound isn't touched.</small></fieldset>` : '';
  $('#editPanel').innerHTML = `<header style="display:flex;align-items:center;gap:.5em"><h2>${vid ? 'Edit video' : 'Edit song'}</h2><span class="spacer"></span>
      <button class="ic f" data-ed="undo" title="Undo (Ctrl+Z)" ${E.undo.length ? '' : 'disabled'}>${icon('undo')}</button><button class="ic f" data-ed="redo" title="Redo (Ctrl+Y)" ${E.redo.length ? '' : 'disabled'}>${icon('redo')}</button>
      <button class="ic f" data-ed="close" title="Close (Esc)">${icon('x')}</button></header>
    <div class="body">
      <fieldset><legend>Keep from … to … (I and O set them)</legend>${markRow('Start', s.start, 'setStart', 'resetStart')}${markRow('End', s.end || d, 'setEnd', 'resetEnd')}</fieldset>
      <fieldset><legend>Cut out a part in the middle</legend>${cutRows}</fieldset>
      ${vid ? `<fieldset><legend>Turn and flip</legend><div class="btns">
          <button class="btn f" data-ed="rotL">${icon('rotL')}</button><button class="btn f" data-ed="rotR">${icon('rotR')}</button>
          <button class="btn f ${s.hflip ? 'primary' : ''}" data-ed="flipH" title="Mirror">${icon('flipH')}</button><button class="btn f ${s.vflip ? 'primary' : ''}" data-ed="flipV" title="Upside down">${icon('flipV')}</button>
          <span class="dim" style="align-self:center">${s.rotate ? `Turned ${s.rotate}°` : ''}</span></div></fieldset>
        <fieldset><legend>Sound</legend>${tog('mute', 'Remove the sound', '', s.mute)}</fieldset>
        <fieldset><legend>Picture</legend><button class="btn f" data-ed="shot">${icon('camera')} Save this frame as a picture</button></fieldset>`
      : `<fieldset><legend>Sound</legend>${numRow('fadeIn', 'Fade in', s.fadeIn, 's')}${numRow('fadeOut', 'Fade out', s.fadeOut, 's')}${tog('normalize', 'Even out the loudness', 'Normalise to a standard loudness', s.normalize)}</fieldset>`}
      ${tog('exact', 'Cut exactly where marked', vid ? 'Re-encodes the video (slower). Off: trims copy the video and start at the nearest keyframe, with no quality loss.' : 'Re-encodes the sound. Off: copies the sound as it is when nothing else needs re-encoding.', s.exact)}
      ${tagsHtml}
    </div>
    <div class="foot"><button class="btn primary f" data-ed="save" ${S.tools.ffmpeg ? '' : 'disabled'}>Save as a copy…</button><button class="btn f" data-ed="preview">Play the kept part</button>${S.tools.ffmpeg ? '' : '<small class="dim">Trimming needs ffmpeg.</small>'}</div>`;
}
$('#editPanel').addEventListener('input', e => {
  const k = e.target.dataset.tag; if (!k || !E.tags) return;
  const v = e.target.value;
  E.tags[k] = k === 'track' || k === 'year' ? (parseInt(v, 10) || null) : v;
  const b = $('[data-ed="saveTags"]'); if (b) b.disabled = !tagsChanged();
});
$('#editPanel').addEventListener('click', async e => {
  const t = e.target.closest('[data-ed],[data-tog],[data-num]'); if (!t) return;
  const now = video.currentTime;
  if (t.dataset.tog) { change(s => { s[t.dataset.tog] = !s[t.dataset.tog]; }); $(`[data-tog="${t.dataset.tog}"]`)?.focus(); return; }
  if (t.dataset.num) { const k = t.dataset.num; change(s => { s[k] = clamp(+(s[k] + +t.dataset.d).toFixed(1), 0, 30); }); $(`[data-num="${k}"][data-d="${t.dataset.d}"]`)?.focus(); return; }
  const a = t.dataset.ed;
  const keepFocus = () => setTimeout(() => $(`[data-ed="${a}"]`)?.focus(), 0);
  switch (a) {
    case 'close': closeEdit(); return;
    case 'undo': undo(); keepFocus(); return;
    case 'redo': redo(); keepFocus(); return;
    case 'setStart': change(s => { s.start = now; }); break;
    case 'setEnd': change(s => { s.end = now; }); break;
    case 'resetStart': change(s => { s.start = 0; }); break;
    case 'resetEnd': change(s => { s.end = 0; }); break;
    case 'cutOn': change(s => {
      // From where it's playing when that's inside the kept part, else the middle of it; a tenth of it long.
      const end = s.end || duration(), len = Math.max(0.5, Math.min(30, (end - s.start) * 0.2));
      const a = now > s.start && now + len < end ? now : s.start + (end - s.start - len) / 2;
      s.cut = [a, a + len];
    }); break;
    case 'cutA': change(s => { s.cut[0] = now; }); break;
    case 'cutB': change(s => { s.cut[1] = now; }); break;
    case 'cutOff': change(s => { s.cut = null; }); break;
    case 'rotL': change(s => { s.rotate = (s.rotate + 270) % 360; }); break;
    case 'rotR': change(s => { s.rotate = (s.rotate + 90) % 360; }); break;
    case 'flipH': change(s => { s.hflip = !s.hflip; }); break;
    case 'flipV': change(s => { s.vflip = !s.vflip; }); break;
    case 'shot': screenshot(); return;
    case 'preview': seekTo(E.s.start); video.play(); return;
    case 'coverPick': pickFile('images', p => { E.cover = { file: p, remove: false }; renderEdit(); }); return;
    case 'coverRemove': E.cover = { file: null, remove: true }; renderEdit(); keepFocus(); return;
    case 'saveTags': saveTags(); return;
    case 'save': saveEdit(); return;
  }
  keepFocus();
});
// Stop at the end mark while previewing the edit.
video.addEventListener('timeupdate', () => {
  if (!E.on || video.paused) return;
  const t = video.currentTime, s = E.s;
  if (s.cut && t >= s.cut[0] && t < s.cut[1] - 0.05) video.currentTime = s.cut[1];
  if (s.end && t >= s.end) { video.pause(); video.currentTime = s.end; }
});
async function saveTags() {
  try {
    const t = await call('tags_write', { path: S.item.path, tags: E.tags, cover: E.cover.file || E.cover.remove ? E.cover : null });
    E.tags = t; E.tagsOrig = JSON.stringify(t); E.cover = { file: null, remove: false };
    S.item.tags = t; renderNowPlaying(); renderEdit(); toast('Saved the song\'s details'); mpris(true);
    if (!isVideo()) renderMusic();
  } catch (e) { toast(e.message); }
}
function editSpec(output = '') {
  const s = E.s;
  return { input: S.item.path, output, start: s.start, end: s.end, cutStart: s.cut ? s.cut[0] : null, cutEnd: s.cut ? s.cut[1] : null,
    rotate: s.rotate, hflip: s.hflip, vflip: s.vflip, mute: s.mute, fadeIn: s.fadeIn, fadeOut: s.fadeOut, normalize: s.normalize, exact: s.exact };
}
async function saveEdit() {
  if (E.busy) return;
  let plan;
  try { plan = await call('edit_plan', { edit: editSpec() }); } catch (e) { toast(e.message); return; }
  const v = sheet(`<h2>Save as a copy</h2>
    <label class="dim" for="saveName">File name (in the same folder as the original)</label><input class="inp f" id="saveName" value="${esc(baseName(plan.output))}" data-osk="name" spellcheck="false">
    <p>${esc(plan.how)}. The result is ${fmt(plan.duration)} long.</p><p class="dim">The original isn't changed.</p>
    <div class="acts"><button class="btn f" data-c="no">Cancel</button><button class="btn primary f" data-c="yes" data-autofocus>Save</button></div>`);
  v.addEventListener('click', async e => {
    const b = e.target.closest('[data-c]'); if (!b) return;
    const name = $('#saveName', v).value.trim();
    closeSheet();
    if (b.dataset.c !== 'yes' || !name) return;
    const out = `${dirName(S.item.path)}/${name}`;
    try {
      E.busy = true;
      const id = await call('edit_start', { edit: editSpec(out) });
      const pv = sheet(`<h2>Saving…</h2><p id="jobText" class="dim">Getting ready</p><div class="prog"><i id="jobBar"></i></div><div class="acts"><button class="btn f" data-c="stop" data-autofocus>Stop</button></div>`, { onClose: () => call('job_cancel', { id }) });
      pv.addEventListener('click', ev => { if (ev.target.closest('[data-c="stop"]')) { call('job_cancel', { id }); } });
      S.jobs[id] = {
        progress: (pct, text) => { const bar = $('#jobBar', pv); if (bar) { bar.style.width = `${Math.round(pct * 100)}%`; $('#jobText', pv).textContent = `${text} · ${Math.round(pct * 100)}%`; } },
        done: r => {
          E.busy = false;
          if (topSheet() === pv) { sheetStack.pop(); pv.remove(); }
          if (!r.ok) { toast(r.error); return; }
          const p = r.result.path;
          E.undo = []; E.redo = [];
          toast(`Saved ${baseName(p)}`, { label: 'Play it', run: () => { closeEdit(true); openAndPlay([p]); } });
        },
      };
    } catch (err) { E.busy = false; toast(err.message); }
  });
}

// ---------------------------------------------------------------- podcasts & radio
const RADIO = 'https://all.api.radio-browser.info';
async function loadPods() {
  const p = await call('store_get', { name: 'podcasts' }).catch(() => null);
  if (p && p.shows) S.pods = { shows: p.shows || [], state: p.state || {} };
  const r = await call('store_get', { name: 'radio' }).catch(() => null);
  if (r && r.favs) S.radio = r;
}
let podSaveTimer;
function savePods() { clearTimeout(podSaveTimer); podSaveTimer = setTimeout(() => call('store_set', { name: 'podcasts', value: S.pods }).catch(() => {}), 400); }
const saveRadio = () => call('store_set', { name: 'radio', value: S.radio }).catch(() => {});
function podMark(guid, patch) { S.pods.state[guid] = { ...(S.pods.state[guid] || {}), ...patch }; savePods(); }
const stripHtml = h => { const d = document.createElement('div'); d.innerHTML = String(h || '').replace(/<(script|style)[\s\S]*?<\/\1>/gi, ''); return (d.textContent || '').replace(/\s+/g, ' ').trim(); };
function parseDuration(s) { if (!s) return 0; if (/^\d+$/.test(s)) return +s; return s.split(':').reduce((a, x) => a * 60 + (+x || 0), 0); }
// An RSS podcast feed (with the iTunes tags most feeds have).
function parseFeed(xml, url) {
  const doc = new DOMParser().parseFromString(xml, 'text/xml');
  if (doc.querySelector('parsererror') && !doc.querySelector('channel')) throw new Error('That address isn\'t a podcast feed.');
  const ch = doc.querySelector('channel'); if (!ch) throw new Error('That address isn\'t a podcast feed.');
  const kid = (el, name) => [...el.children].find(c => c.nodeName === name);
  const text = (el, name) => (kid(el, name)?.textContent || '').trim();
  const img = (kid(ch, 'itunes:image')?.getAttribute('href')) || text(kid(ch, 'image') || ch, 'url') || '';
  const episodes = [...ch.children].filter(c => c.nodeName === 'item').map(it => {
    const enc = kid(it, 'enclosure');
    const eurl = enc?.getAttribute('url') || '';
    return { guid: text(it, 'guid') || eurl || text(it, 'title'), title: text(it, 'title') || 'Episode', date: Date.parse(text(it, 'pubDate')) || 0, url: eurl, type: enc?.getAttribute('type') || '',
      duration: parseDuration(text(it, 'itunes:duration')), desc: stripHtml(text(it, 'description') || text(it, 'itunes:summary')).slice(0, 600), image: kid(it, 'itunes:image')?.getAttribute('href') || '' };
  }).filter(e => e.url);
  return { url, title: text(ch, 'title') || url, author: text(ch, 'itunes:author') || text(ch, 'managingEditor'), image: img, desc: stripHtml(text(ch, 'description')).slice(0, 600), episodes, fetched: Date.now() };
}
async function subscribe(url) {
  try {
    toast('Reading the feed…');
    const show = parseFeed(await call('net_get', { url }), url);
    const i = S.pods.shows.findIndex(s => s.url === url);
    if (i >= 0) S.pods.shows[i] = show; else S.pods.shows.unshift(show);
    savePods(); S.podShow = url; renderPods(); toast(`Subscribed to ${show.title}`);
  } catch (e) { toast(e.message); }
}
async function refreshShows(force = false) {
  for (const s of S.pods.shows) {
    if (!force && Date.now() - (s.fetched || 0) < 6 * 3600e3) continue;
    try { const n = parseFeed(await call('net_get', { url: s.url }), s.url); Object.assign(s, n); } catch (_) { /* keep what we had */ }
  }
  savePods(); if (S.view === 'pods') renderPods();
}
async function podSearch(q) {
  q = q.trim(); if (!q) return;
  if (isUrl(q)) { if (S.podTab === 'radio') { playQueue([{ path: q, name: q, kind: 'audio', radio: {} }]); return; } subscribe(q); return; }
  const body = $('#podBody');
  body.innerHTML = '<div class="empty">Searching…</div>';
  try {
    if (S.podTab === 'radio') {
      const list = JSON.parse(await call('net_get', { url: `${RADIO}/json/stations/search?name=${encodeURIComponent(q)}&limit=60&hidebroken=true&order=clickcount&reverse=true` }));
      S.radioResults = list; S.radioTitle = `Stations for “${q}”`; renderPods();
    } else {
      const r = JSON.parse(await call('net_get', { url: `https://itunes.apple.com/search?media=podcast&entity=podcast&limit=30&term=${encodeURIComponent(q)}` }));
      S.podResults = (r.results || []).filter(x => x.feedUrl).map(x => ({ url: x.feedUrl, title: x.collectionName, author: x.artistName, image: x.artworkUrl600 || x.artworkUrl100 }));
      S.podShow = null; renderPods();
    }
  } catch (e) { body.innerHTML = `<div class="empty">${esc(e.message)}</div>`; }
}
function cardImg(src) { return src ? `<img src="${esc(src)}" alt="" loading="lazy" onerror="this.replaceWith(Object.assign(document.createElement('span'),{className:'noimg'}))">` : `<span class="noimg">${icon('radio')}</span>`; }
function renderPods() {
  for (const c of $$('[data-pod]')) c.classList.toggle('on', c.dataset.pod === S.podTab);
  $('#podSearch').placeholder = S.podTab === 'radio' ? 'Search stations (name, genre, country) or paste a stream address' : 'Search podcasts, or paste a feed address';
  const b = $('#podBody');
  if (S.podTab === 'radio') { renderRadio(b); return; }
  const show = S.podShow && S.pods.shows.find(s => s.url === S.podShow);
  if (show) {
    const st = S.pods.state;
    b.innerHTML = `<div class="show-head">${show.image ? `<img src="${esc(show.image)}" alt="">` : '<span></span>'}<div><h2>${esc(show.title)}</h2><p class="dim">${esc(show.author || '')}</p>
        <div class="btns"><button class="btn f" data-pa="back">All podcasts</button><button class="btn f" data-pa="refresh">${icon('refresh')}</button><button class="btn danger f" data-pa="unsub">Unsubscribe</button></div></div></div>
      <div class="eps">${show.episodes.slice(0, 300).map((e, i) => { const s = st[e.guid] || {}; const pct = s.dur ? clamp(s.pos / s.dur, 0, 1) : 0; return `<div class="ep ${s.played ? 'played' : ''}">
        <span class="t">${esc(e.title)}</span>
        <span class="acts"><button class="ic f" data-ep="${i}" data-pa="play" title="${s.file ? 'Play' : 'Stream'}">${icon('play')}</button>${s.file ? `<button class="ic f" data-ep="${i}" data-pa="deldl" title="Remove the download">${icon('trash')}</button>` : `<button class="ic f" data-ep="${i}" data-pa="dl" title="Download">${s.dl != null ? `${Math.round(s.dl * 100)}%` : icon('download')}</button>`}<button class="ic f" data-ep="${i}" data-pa="played" title="${s.played ? 'Mark as not played' : 'Mark as played'}">${icon('check')}</button></span>
        <span class="s">${e.date ? new Date(e.date).toLocaleDateString() : ''}${e.duration ? ` · ${fmt(e.duration)}` : ''}${s.file ? ' · downloaded' : ''}${s.played ? ' · played' : s.pos ? ` · ${fmt(s.pos)} in` : ''}</span>
        ${pct && !s.played ? `<span class="bar"><i style="width:${pct * 100}%"></i></span>` : ''}</div>`; }).join('') || '<div class="empty">No episodes in this feed.</div>'}</div>`;
    return;
  }
  const res = S.podResults;
  b.innerHTML = (res ? `<div class="section-h"><h3>Search results</h3><button class="btn f" data-pa="clear">Clear</button></div><div class="grid">${res.map((r, i) => `<button class="card f" data-res="${i}">${cardImg(r.image)}<b>${esc(r.title)}</b><small>${esc(r.author || '')}</small></button>`).join('') || '<div class="empty">Nothing found.</div>'}</div>` : '')
    + `<div class="section-h"><h3>Your podcasts</h3>${S.pods.shows.length ? `<button class="btn f" data-pa="refreshAll">${icon('refresh')}</button>` : ''}</div>`
    + (S.pods.shows.length ? `<div class="grid">${S.pods.shows.map((s, i) => { const n = s.episodes.filter(e => !(S.pods.state[e.guid] || {}).played).length; return `<button class="card f" data-show="${i}">${cardImg(s.image)}<b>${esc(s.title)}</b><small>${n} not played</small></button>`; }).join('')}</div>`
      : '<div class="empty">No podcasts yet. Search for one above, or paste its feed address.</div>');
}
function renderRadio(b) {
  const favs = S.radio.favs || [];
  const isFav = s => favs.some(f => f.stationuuid === s.stationuuid);
  const row = (s, i, src) => `<div class="station"><button class="f" style="display:contents" data-st="${i}" data-src="${src}">${cardImg(s.favicon)}<span><b>${esc(s.name.trim())}</b><small>${esc([s.country, s.tags?.split(',').slice(0, 3).join(', '), s.codec && `${s.codec}${s.bitrate ? ` ${s.bitrate}k` : ''}`].filter(Boolean).join(' · '))}</small></span></button><button class="ic f ${isFav(s) ? 'on' : ''}" data-fav="${i}" data-src="${src}" title="${isFav(s) ? 'Remove from favourites' : 'Add to favourites'}">${icon(isFav(s) ? 'starOn' : 'star')}</button></div>`;
  let h = `<div class="section-h"><h3>Favourites</h3></div>` + (favs.length ? `<div class="stations">${favs.map((s, i) => row(s, i, 'fav')).join('')}</div>` : '<div class="empty">Star a station to keep it here.</div>');
  h += `<div class="section-h"><h3>${esc(S.radioTitle || 'Popular stations')}</h3></div><div class="stations" id="radioList">${(S.radioResults || []).map((s, i) => row(s, i, 'res')).join('') || '<div class="empty">Loading stations…</div>'}</div>`;
  h += '<p class="dim" style="font-size:.8em">Stations come from radio-browser.info, a free community directory.</p>';
  b.innerHTML = h;
  if (!S.radioResults) call('net_get', { url: `${RADIO}/json/stations/topclick/60?hidebroken=true` }).then(t => { S.radioResults = JSON.parse(t); if (S.view === 'pods' && S.podTab === 'radio') renderPods(); }).catch(e => { const l = $('#radioList'); if (l) l.innerHTML = `<div class="empty">${esc(e.message)}</div>`; });
}
function playStation(s) {
  const url = s.url_resolved || s.url;
  playQueue([{ path: url, name: s.name.trim(), title: s.name.trim(), artist: 'Internet radio', kind: 'audio', image: s.favicon || '', radio: { uuid: s.stationuuid } }]);
  if (s.stationuuid) call('net_get', { url: `${RADIO}/json/url/${s.stationuuid}` }).catch(() => {});
}
$('#v-pods').addEventListener('click', async e => {
  const t = e.target.closest('[data-pod],[data-pa],[data-show],[data-res],[data-st],[data-fav]'); if (!t) return;
  if (t.dataset.pod) { S.podTab = t.dataset.pod; renderPods(); return; }
  if (t.dataset.show) { S.podShow = S.pods.shows[+t.dataset.show].url; renderPods(); setTimeout(() => $('#podBody .f')?.focus(), 0); return; }
  if (t.dataset.res) { subscribe(S.podResults[+t.dataset.res].url); return; }
  if (t.dataset.st) { playStation((t.dataset.src === 'fav' ? S.radio.favs : S.radioResults)[+t.dataset.st]); return; }
  if (t.dataset.fav) {
    const s = (t.dataset.src === 'fav' ? S.radio.favs : S.radioResults)[+t.dataset.fav];
    const i = S.radio.favs.findIndex(f => f.stationuuid === s.stationuuid);
    if (i >= 0) S.radio.favs.splice(i, 1); else S.radio.favs.push({ stationuuid: s.stationuuid, name: s.name, url: s.url, url_resolved: s.url_resolved, favicon: s.favicon, country: s.country, tags: s.tags, codec: s.codec, bitrate: s.bitrate });
    saveRadio(); renderPods(); return;
  }
  const show = S.podShow && S.pods.shows.find(s => s.url === S.podShow);
  const ep = t.dataset.ep != null && show ? show.episodes[+t.dataset.ep] : null;
  switch (t.dataset.pa) {
    case 'back': S.podShow = null; renderPods(); break;
    case 'clear': S.podResults = null; renderPods(); break;
    case 'refresh': try { Object.assign(show, parseFeed(await call('net_get', { url: show.url }), show.url)); savePods(); renderPods(); toast('Up to date'); } catch (err) { toast(err.message); } break;
    case 'refreshAll': toast('Looking for new episodes…'); refreshShows(true); break;
    case 'unsub': if (await confirmSheet(`Unsubscribe from ${show.title}?`, '<p>Downloaded episodes stay in Music › Podcasts.</p>', 'Unsubscribe', { danger: true })) { S.pods.shows = S.pods.shows.filter(s => s !== show); S.podShow = null; savePods(); renderPods(); } break;
    case 'play': {
      const st = S.pods.state[ep.guid] || {};
      let path = ep.url;
      if (st.file && await call('file_exists', { path: st.file }).catch(() => false)) path = st.file;
      playQueue([{ path, name: ep.title, title: ep.title, artist: show.title, album: show.author, kind: /video/.test(ep.type) ? 'video' : 'audio', image: ep.image || show.image, podcast: { guid: ep.guid, show: show.title } }]);
      break;
    }
    case 'played': { const s = S.pods.state[ep.guid] || {}; podMark(ep.guid, { played: !s.played, pos: 0 }); renderPods(); break; }
    case 'deldl': { const s = S.pods.state[ep.guid]; try { await call('delete_download', { path: s.file }); } catch (err) { toast(err.message); } podMark(ep.guid, { file: null }); renderPods(); break; }
    case 'dl': {
      if ((S.pods.state[ep.guid] || {}).dl != null) break;
      try {
        const id = await call('download_start', { url: ep.url, show: show.title, title: ep.title });
        podMark(ep.guid, { dl: 0 }); renderPods();
        S.jobs[id] = {
          progress: pct => { S.pods.state[ep.guid].dl = pct; const bt = $(`[data-pa="dl"][data-ep="${t.dataset.ep}"]`); if (bt) bt.textContent = `${Math.round(pct * 100)}%`; },
          done: r => { const s = S.pods.state[ep.guid]; delete s.dl; if (r.ok) { s.file = r.result.path; toast(`Downloaded ${ep.title}`); } else toast(r.error); savePods(); if (S.view === 'pods') renderPods(); },
        };
      } catch (err) { toast(err.message); }
      break;
    }
  }
});
$('#podSearch').addEventListener('keydown', e => { if (e.key === 'Enter') { e.preventDefault(); podSearch(e.target.value); } });

// ---------------------------------------------------------------- discs: DVDs, Video CDs, music CDs (src/disc*.rs, dvd.rs)
// A disc opens on its own screen over Now playing: a DVD's titles and chapters (the disc's own menu isn't run: this is
// the menu), a Video CD's parts, a music CD's songs; or why it can't play (locked DVD, Blu-ray, empty). Select (or a
// remote's Menu) opens it again while the film plays. Discs going in and out come from udev (player://disc).
const DISC_KIND = { dvd: 'DVD-Video', vcd: 'Video CD', svcd: 'Super Video CD', 'audio-cd': 'Music CD', data: 'Data disc', bluray: 'Blu-ray disc', blank: 'Empty disc', empty: 'No disc' };
const playableKind = k => ['dvd', 'vcd', 'svcd', 'audio-cd', 'data'].includes(k);
const discPanelOn = () => !!(S.discPanel && S.discInfo);
function driveLine(d) { return d.media ? `${d.name} · ${DISC_KIND[d.kind] || 'Disc'}` : `${d.name} · no disc`; }
function discHints() {
  return '<span><b>A</b>Play</span><span><b>✥</b>Move</span>' + (S.discInfo?.device ? '<span><b>X</b>Eject</span>' : '') + '<span><b>Y</b>Controller buttons</span>' + (S.item?.disc ? '<span><b>B</b>Back to the film</span>' : '<span><b>B</b>Back</span>');
}
function discPlayHints() {
  const dvd = S.item?.disc?.kind === 'dvd';
  return `<span><b>A</b>Play/pause</span><span><b>◀ ▶</b>Seek</span><span><b>LB RB</b>${dvd ? 'Chapters' : 'Previous/next'}</span><span><b>Y</b>Sound${dvd ? ' and subtitles' : ''}</span><span><b>Select</b>Disc menu</span><span><b>B</b>Back</span>`;
}
function showDiscPanel(on) {
  S.discPanel = !!on && !!S.discInfo;
  if (S.discPanel) renderDisc();
  setBodyMode(); updateButtons(); hints();
  if (S.discPanel) setTimeout(() => ($('#disc [data-autofocus]') || $('#disc .f'))?.focus(), 0);
  else if (document.activeElement?.closest?.('#disc')) document.activeElement.blur();
}
function toggleDiscPanel() { if (S.view !== 'play') setView('play'); showDiscPanel(!discPanelOn()); }

async function refreshDrives(announce, device) {
  const before = S.drives;
  try { S.drives = (await call('disc_drives')) || []; } catch (_) { S.drives = []; }
  const playable = S.drives.find(d => d.media && playableKind(d.kind));
  $('#playDiscBtn').hidden = !playable;
  $('#discHere').hidden = !playable;
  if (playable) $('#discHere').textContent = `In the ${playable.name.toLowerCase().includes('drive') ? playable.name : 'drive'}: ${playable.label || DISC_KIND[playable.kind]}`;
  if (S.view === 'open') renderPlaces();
  if (!announce || !device) return;
  const now = S.drives.find(d => d.device === device), was = before.find(d => d.device === device);
  if (now?.media && !(was?.media && was.label === now.label && was.kind === now.kind)) {
    const what = now.label || DISC_KIND[now.kind] || 'A disc';
    if (playableKind(now.kind)) toast(`A disc was inserted: ${what}`, { label: 'Play disc', run: () => openDisc(device) });
    else toast(`A disc was inserted: ${what}. ${now.kind === 'bluray' ? 'Blu-ray discs don\'t play in Hearth Player.' : ''}`.trim());
  } else if (!now?.media && S.discInfo?.device === device) {
    discGone();
    toast('The disc was taken out.');
  }
}
function discEvent(device) { refreshDrives(true, device); }

function stopPlaying() {
  video.pause(); video.removeAttribute('src'); video.load();
  S.item = null; S.queue = []; S.qi = -1; S.disc = null; S.probe = null; S.subs = []; S.subOn = -1;
  call('keep_awake', { mode: '' }).catch(() => {});
  $('#title').textContent = '';
  setBodyMode(); updateButtons(); hints(); tick();
}
function discGone() {
  stopMenu();
  if (S.item?.disc) { saveResume(); stopPlaying(); }
  S.discInfo = null; S.discPanel = false; setBodyMode(); updateButtons(); hints();
}

async function openDisc(path, { play = false } = {}) {
  notice('Reading the disc…');
  let info;
  try { info = await call('disc_open', { path: path || null }); } catch (e) { closeNotice(); toast(e.message); return; }
  closeNotice();
  if (S.item?.disc && S.discInfo?.id !== info.id) stopPlaying();
  stopMenu();
  S.discInfo = info;
  S.discPrefs = { audio: undefined, sub: undefined };
  if (info.kind === 'data' && info.folder) {
    toast(`${info.label || 'The disc'}: its files`);
    S.discPanel = false; setView('open'); openDir(info.folder); return;
  }
  if (S.decoding == null) S.decoding = await call('dvd_decoding').catch(() => ({ installed: false, canAdd: false }));
  setView('play');
  showDiscPanel(true);
  // A DVD starts in its own menu, as on a DVD player (the titles screen is under it, and stands in when it can't run).
  if (info.kind === 'dvd' && info.menu && !play) startMenu();
  if (info.kind === 'audio-cd' && info.tracks.every(t => /^Track \d+$/.test(t.title))) lookupNames(false);
  if (play && info.kind === 'dvd' && !info.encrypted && info.main) playTitle(info.main);
}

const discPath = (info, n, name) => `/disc/${info.key || info.id}/${n}/${name}`;
function titleItem(info, n) {
  const name = `${info.label || 'DVD'} - Title ${n}`;
  return { path: discPath(info, n, name), name, title: info.label || 'DVD', kind: 'video', disc: { id: info.id, kind: info.kind, n, audio: S.discPrefs?.audio, sub: S.discPrefs?.sub } };
}
function trackItems(info) {
  return info.tracks.map(t => info.kind === 'audio-cd'
    ? { path: discPath(info, t.n, t.title), name: t.title, title: t.title, artist: t.artist || info.artist, album: info.album || info.label, kind: 'audio', duration: t.duration, disc: { id: info.id, kind: info.kind, n: t.n } }
    : { path: discPath(info, t.n, t.title), name: `${info.label} - ${t.title}`, title: info.label, kind: 'video', duration: t.duration, disc: { id: info.id, kind: info.kind, n: t.n } });
}
function playTitle(n, at = null) { const it = titleItem(S.discInfo, n); playQueue([it], 0, at != null ? { at } : {}); }
function playTrack(n, at = null) { const items = trackItems(S.discInfo); playQueue(items, Math.max(0, items.findIndex(i => i.disc.n === n)), at != null ? { at } : {}); }

// What the page plays for a disc item (playIndex hands it here).
async function playDiscItem(item, { at = null, noResume = false } = {}) {
  const info = S.discInfo, d = item.disc;
  if (!info || info.id !== d.id) { notice('The disc isn\'t in the drive any more.'); return; }
  const t = d.kind === 'dvd' ? info.titles.find(x => x.n === d.n) : null;
  S.disc = null;
  S.probe = t ? { duration: t.duration, chapters: t.chapters, streams: t.audio.map(a => ({ kind: 'audio', label: a.label, id: a.id, lang: a.lang })) } : { duration: item.duration || 0, chapters: [], streams: [] };
  S.subs = t ? t.subs.map(x => ({ label: x.label, lang: x.lang, disc: x.id })) : [];
  if (t) {
    // Sound: your pick on the disc screen, else your usual language, else the disc's first.
    if (d.audio == null) { const want = S.settings.discAudioLang; d.audio = (t.audio.find(a => want && a.lang === want) || t.audio[0])?.id ?? null; }
    if (d.sub === undefined) { const want = S.settings.subLang; d.sub = S.settings.subsOff === false && want ? (t.subs.find(x => x.lang === want)?.id ?? null) : null; }
    S.subOn = S.subs.findIndex(x => x.disc === d.sub);
  }
  const r = S.resume[item.path];
  const resumeAt = at ?? (!noResume ? (r?.t || 0) : 0);
  showDiscPanel(false);
  setBodyMode();
  try { await discStream(resumeAt > 5 || at != null ? resumeAt : 0); } catch (e) { notice(e.message, { actions: [{ label: 'Disc menu', run: () => { closeNotice(); showDiscPanel(true); } }] }); return; }
  if (resumeAt > 5 && at == null) toast(`Carrying on from ${fmt(resumeAt)}`, { label: 'Start over', run: () => seekTo(0) });
  if (!isVideo()) renderMusic();
  renderMarks(); updateButtons(); mpris(true); hints();
}
async function discStream(at, { paused = false } = {}) {
  const d = S.item.disc, track = S.track;
  const p = await call('disc_play', { id: d.id, n: d.n, at: Math.max(0, at || 0), audio: d.audio ?? null, sub: d.sub ?? null, reencode: !!d.reencode });
  if (track !== S.track) return;
  const retries = S.disc?.retries || 0;
  S.disc = { offset: p.restart ? p.start : 0, duration: p.duration, restart: p.restart, pending: null, retries };
  await setSource(p.url);
  video.addEventListener('loadedmetadata', () => { if (!p.restart && at > 1) video.currentTime = at; if (!paused) video.play().catch(() => {}); }, { once: true });
  tick(); renderMarks();
}
let discSeekTimer;
function discSeek(t) {
  const d = duration();
  t = clamp(t, 0, d > 1 ? d - 1 : t);
  S.disc.pending = t;
  osd(`${fmt(t)} / ${fmt(d)}`); tick();
  clearTimeout(discSeekTimer);
  const paused = video.paused && !!video.src;
  discSeekTimer = setTimeout(() => discStream(t, { paused }).catch(e => toast(e.message)), 450);
}
function discError(code) {
  const d = S.item.disc;
  if (d.kind !== 'audio-cd' && !d.reencode) {
    // The picture as it is didn't play: once more, made again (H.264).
    d.reencode = true; discStream(ct()).catch(e => notice(e.message)); return;
  }
  notice(d.kind === 'audio-cd' ? 'This song couldn\'t be read from the CD. The disc may be scratched or dirty.' : 'This couldn\'t be played from the disc. The disc may be scratched or dirty.',
    { actions: [{ label: 'Disc menu', run: () => { closeNotice(); showDiscPanel(true); } }] });
}
function discEnded() {
  if (!S.item?.disc) return false;
  // A stream that stopped well before the end (a hard-to-read spot): carry on just after it, twice at most.
  if (restarts() && S.disc && duration() - ct() > 20 && S.disc.retries < 2) { S.disc.retries++; discStream(ct() + 2).catch(() => {}); return true; }
  delete S.resume[S.item.path]; saveResumeStore();
  if (S.item.kind === 'audio') return false;
  if (S.discInfo?.menu) startMenu().then(ok => { if (!ok) showDiscPanel(true); }); else showDiscPanel(true);
  return true;
}
function discSub(i) {
  const d = S.item?.disc; if (!d) return;
  const s = i >= 0 ? S.subs[i] : null;
  d.sub = s ? s.disc : null; S.subOn = s ? i : -1;
  S.settings.subsOff = !s; if (s?.lang) S.settings.subLang = s.lang; saveSettings();
  osd(s ? `${s.label} (drawn into the picture)` : 'Subtitles off');
  discStream(ct()).catch(e => toast(e.message));
  updateButtons();
}
function discAudio(n) {
  const d = S.item?.disc; const a = audioTracks()[n]; if (!d || !a) return;
  d.audio = a.id; if (a.lang) { S.settings.discAudioLang = a.lang; saveSettings(); }
  osd(a.label);
  discStream(ct()).catch(e => toast(e.message));
}
function discTracksMenu() {
  const d = S.item?.disc; if (!d) return;
  if (d.kind !== 'dvd') { queueMenu(); return; }
  const au = audioTracks(), cur = currentAudio();
  const items = au.map((a, i) => ({ label: a.label, sub: 'Sound', on: i === cur, run: () => discAudio(i) }));
  items.push({ sep: true }, { label: 'Subtitles off', on: S.subOn < 0, run: () => discSub(-1) }, ...S.subs.map((s, i) => ({ label: s.label, sub: 'Subtitles', on: S.subOn === i, run: () => discSub(i) })));
  menu('Sound and subtitles', items);
}
function prefLabel(list, id, none) { const x = list.find(a => a.id === id); return x ? x.label : none; }
function pickPref(kind) {
  const info = S.discInfo, t = info.titles.find(x => x.n === info.main) || info.titles[0];
  if (kind === 'audio') menu('Sound', t.audio.map(a => ({ label: a.label, on: (S.discPrefs.audio ?? t.audio[0]?.id) === a.id, run: () => { S.discPrefs.audio = a.id; if (a.lang) { S.settings.discAudioLang = a.lang; saveSettings(); } renderDisc(); } })));
  else menu('Subtitles', [{ label: 'Off', on: S.discPrefs.sub == null, run: () => { S.discPrefs.sub = null; renderDisc(); } }, ...t.subs.map(x => ({ label: x.label, sub: 'Drawn into the picture', on: S.discPrefs.sub === x.id, run: () => { S.discPrefs.sub = x.id; renderDisc(); } }))]);
}

function noteHtml(info) {
  const dec = S.decoding || {};
  if (info.encrypted) {
    const acts = [];
    if (!dec.installed && dec.canAdd) acts.push(`<button class="btn primary f" data-da="decoding" data-autofocus>${icon('plus')} Add DVD decoding</button>`);
    if (dec.installed) acts.push(`<button class="btn primary f" data-da="again" data-autofocus>${icon('refresh')} Try again</button>`);
    if (info.device) acts.push(`<button class="btn f" data-da="eject">${icon('eject')} Eject</button>`);
    const extra = !dec.installed && !dec.canAdd ? '<p class="dim">Adding DVD decoding isn\'t offered on this edition of Hearth OS yet.</p>'
      : !dec.installed ? '<p class="dim">Adding it asks for an administrator\'s password, needs the internet and takes about a minute. Nothing is added unless you say yes.</p>' : '';
    return `<h3>${icon('lock')} This DVD is locked</h3><p>${esc(info.note)}</p>${extra}<div class="acts">${acts.join('')}</div>`;
  }
  const head = { bluray: 'Blu-ray discs don\'t play here', blank: 'This disc is empty', empty: 'No disc in the drive', data: 'Nothing to play here' }[info.kind] || 'This disc can\'t be played';
  return `<h3>${icon('disc')} ${esc(head)}</h3><p>${esc(info.note)}</p><div class="acts">${info.device ? `<button class="btn primary f" data-da="eject" data-autofocus>${icon('eject')} Eject</button>` : ''}</div>`;
}
function renderDisc() {
  const info = S.discInfo; if (!info) return;
  $('#discLabel').textContent = info.label || DISC_KIND[info.kind] || 'Disc';
  const total = info.kind === 'audio-cd' ? info.tracks.reduce((a, t) => a + t.duration, 0) : 0;
  $('#discKind').textContent = [DISC_KIND[info.kind] || 'Disc',
    info.kind === 'dvd' ? `${info.titles.length} ${info.titles.length === 1 ? 'title' : 'titles'}` : '',
    info.kind === 'audio-cd' ? `${info.tracks.length} songs · ${fmt(total)}` : '',
    (info.kind === 'vcd' || info.kind === 'svcd') ? `${info.tracks.length} ${info.tracks.length === 1 ? 'part' : 'parts'}` : '',
    info.kind === 'audio-cd' && info.artist ? info.artist : ''].filter(Boolean).join(' · ');
  $('#discArt').classList.toggle('locked', !!info.encrypted || !!info.note);
  const acts = [];
  const playing = S.item?.disc?.id === info.id;
  if (playing) acts.push(`<button class="btn primary f" data-da="back" data-autofocus>${icon('play')} Back to ${isVideo() ? 'the film' : 'the music'}<small>${fmt(ct())}</small></button>`);
  let choices = '';
  if (info.kind === 'dvd' && !info.encrypted) {
    const main = info.titles.find(t => t.n === info.main) || info.titles[0];
    const r = S.resume[titleItem(info, main.n).path];
    if (info.menu) acts.push(`<button class="btn f" data-da="menu">${icon('list')} The disc's own menu</button>`);
    acts.push(`<button class="btn ${playing ? '' : 'primary'} f" data-da="title" data-n="${main.n}" ${playing ? '' : 'data-autofocus'}>${icon('play')} Play the film<small>${fmt(main.duration)}</small></button>`);
    if (r?.t > 5) acts.push(`<button class="btn f" data-da="resume" data-n="${main.n}" data-t="${r.t}">${icon('refresh')} Carry on<small>${fmt(r.t)}</small></button>`);
    if (main.audio.length > 1) choices += `<button class="chip f" data-da="prefAudio">${icon('lang')}<span>Sound</span><small>${esc(prefLabel(main.audio, S.discPrefs.audio ?? main.audio[0]?.id, 'The disc\'s own'))}</small></button>`;
    if (main.subs.length) choices += `<button class="chip f" data-da="prefSub">${icon('cc')}<span>Subtitles</span><small>${esc(prefLabel(main.subs, S.discPrefs.sub, 'Off'))}</small></button>`;
  }
  if (info.kind === 'audio-cd') {
    acts.push(`<button class="btn ${playing ? '' : 'primary'} f" data-da="all" ${playing ? '' : 'data-autofocus'}>${icon('play')} Play the CD<small>${fmt(total)}</small></button>`);
    acts.push(`<button class="btn f" data-da="names">${icon('refresh')} Look up names online</button>`);
  }
  if ((info.kind === 'vcd' || info.kind === 'svcd') && info.tracks.length) acts.push(`<button class="btn ${playing ? '' : 'primary'} f" data-da="track" data-n="${info.tracks[0].n}" ${playing ? '' : 'data-autofocus'}>${icon('play')} Play<small>${fmt(info.tracks.reduce((a, t) => a + t.duration, 0))}</small></button>`);
  if (info.device && !info.note) acts.push(`<button class="btn f" data-da="eject">${icon('eject')} Eject</button>`);
  acts.push(`<button class="btn f" data-da="pad">${icon('pad')} Controller buttons</button>`);
  if (S.decoding?.installed && info.kind === 'dvd') acts.push(`<button class="btn f" data-da="undecoding">${icon('trash')} Remove DVD decoding</button>`);
  $('#discActs').innerHTML = acts.join('');
  $('#discChoices').innerHTML = choices;
  const note = $('#discNote');
  note.hidden = !(info.encrypted || info.note);
  if (!note.hidden) note.innerHTML = noteHtml(info);
  const list = $('#discList');
  if (info.kind === 'dvd' && !info.encrypted) {
    const cur = playing ? S.item.disc.n : null;
    list.innerHTML = `<div class="disc-head">${icon('list')}<span>${info.menu ? 'Titles and chapters: pick one here, or use the disc\'s own menu.' : 'Titles and chapters. This disc\'s own menu can\'t run here: pick from them instead.'}</span></div>` + info.titles.map(t => {
      const bits = [`${t.chapters.length} ${t.chapters.length === 1 ? 'chapter' : 'chapters'}`, t.audio.length > 1 ? `${t.audio.length} sound tracks` : '', t.subs.length ? `${t.subs.length} subtitles` : '', t.angles > 1 ? `${t.angles} camera angles (the first plays)` : ''].filter(Boolean).join(' · ');
      const ch = t.chapters.length > 1 ? `<div class="disc-sub" data-for="${t.n}" ${t.n === (cur ?? info.main) ? '' : 'hidden'}>${t.chapters.map((c, i) => `<button class="chip f" data-ch="${t.n}" data-t="${c.start}">${i + 1} · ${fmt(c.start)}</button>`).join('')}</div>` : '';
      return `<button class="disc-row f ${t.n === info.main ? 'main' : ''} ${t.n === cur ? 'on' : ''}" data-title="${t.n}"><span class="n">${t.n}</span><span>Title ${t.n}${t.n === info.main ? '<span class="tagm">The film</span>' : ''}<small>${esc(bits)}</small></span><span class="d">${fmt(t.duration)}</span></button>${ch}`;
    }).join('');
  } else if (info.kind === 'audio-cd' || info.kind === 'vcd' || info.kind === 'svcd') {
    const cur = playing ? S.item.disc.n : null;
    const from = info.namesFrom ? `Names from ${info.namesFrom}.` : info.tracks.some(t => !/^Track \d+$/.test(t.title)) ? 'Names from the disc (CD-Text).' : 'This disc has no names on it.';
    list.innerHTML = (info.kind === 'audio-cd' ? `<div class="disc-head">${icon('note')}<span>${esc(from)}</span></div>` : '') + info.tracks.map(t => `<button class="disc-row f ${t.n === cur ? 'on' : ''}" data-trk="${t.n}"><span class="n">${t.n}</span><span>${esc(t.title)}${t.artist && t.artist !== info.artist ? `<small>${esc(t.artist)}</small>` : ''}</span><span class="d">${fmt(t.duration)}</span></button>`).join('');
  } else list.innerHTML = '';
  paintIcons($('#disc'));
}
$('#disc').addEventListener('focusin', e => {
  const r = e.target.closest('[data-title]'); if (!r) return;
  for (const s of $$('#disc .disc-sub')) s.hidden = s.dataset.for !== r.dataset.title;
});
$('#disc').addEventListener('click', e => {
  const info = S.discInfo; if (!info) return;
  const a = e.target.closest('[data-da]');
  if (a) {
    const n = +a.dataset.n;
    switch (a.dataset.da) {
      case 'title': playTitle(n); break;
      case 'resume': playTitle(n, +a.dataset.t); break;
      case 'all': playTrack(info.tracks[0].n); break;
      case 'track': playTrack(n); break;
      case 'back': showDiscPanel(false); if (video.paused) video.play().catch(() => {}); break;
      case 'eject': ejectDisc(); break;
      case 'menu': startMenu(); break;
      case 'pad': controllerSheet(); break;
      case 'names': lookupNames(true); break;
      case 'decoding': addDecoding(); break;
      case 'undecoding': removeDecoding(); break;
      case 'again': openDisc(info.source); break;
      case 'prefAudio': pickPref('audio'); break;
      case 'prefSub': pickPref('sub'); break;
    }
    return;
  }
  const t = e.target.closest('[data-title]'); if (t) { playTitle(+t.dataset.title); return; }
  const c = e.target.closest('[data-ch]'); if (c) { playTitle(+c.dataset.ch, +c.dataset.t + 0.01); return; }
  const k = e.target.closest('[data-trk]'); if (k) playTrack(+k.dataset.trk);
});

async function lookupNames(loud) {
  const info = S.discInfo; if (!info || info.kind !== 'audio-cd') return;
  if (loud) toast('Looking the CD up…');
  try {
    const n = await call('disc_names', { id: info.id });
    if (S.discInfo !== info) return;
    if (n.album) { info.album = n.album; info.label = n.album; }
    if (n.artist) info.artist = n.artist;
    n.tracks.forEach(([title, artist], i) => { if (info.tracks[i]) { if (title) info.tracks[i].title = title; if (artist) info.tracks[i].artist = artist; } });
    info.namesFrom = n.from;
    // The songs already in the play queue get their names too.
    for (const q of S.queue) { const t = q.disc && info.tracks.find(x => x.n === q.disc.n); if (t) { q.title = q.name = t.title; q.artist = t.artist; q.album = info.album; } }
    if (S.item?.disc && !isVideo()) renderMusic();
    renderDisc();
    if (loud) toast(`Names from ${n.from}`);
  } catch (e) { if (loud) toast(e.message); }
}

async function ejectDisc() {
  stopMenu();
  const dev = S.discInfo?.device || S.drives.find(d => d.media)?.device;
  if (!dev) return;
  if (S.item?.disc) { saveResume(); stopPlaying(); }
  try { await call('disc_eject', { device: dev }); S.discInfo = null; S.discPanel = false; setBodyMode(); updateButtons(); hints(); osd('Ejected'); }
  catch (e) { toast(e.message); }
  refreshDrives(false);
}

async function decodingJob(add) {
  const id = await call('dvd_decoding_change', { add });
  S.jobs[id] = {
    progress: (pct, text) => notice(text || (add ? 'Adding DVD decoding' : 'Removing DVD decoding'), { progress: pct }),
    done: async r => {
      closeNotice();
      if (!r.ok) { toast(r.error); return; }
      S.decoding = await call('dvd_decoding').catch(() => S.decoding);
      if (add) toast(r.result && r.result.live === false ? 'DVD decoding is added. Restart the PC to use it.' : 'DVD decoding is added.');
      else toast('DVD decoding is removed (from the next restart).');
      if (S.discInfo) openDisc(S.discInfo.source);
    },
  };
}
async function addDecoding() {
  if (S.decoding == null) S.decoding = await call('dvd_decoding').catch(() => ({}));
  if (!S.decoding.canAdd) { toast('Adding DVD decoding isn\'t offered on this edition of Hearth OS yet.'); return; }
  const ok = await confirmSheet('Add DVD decoding?',
    `<p>Most DVDs from a shop are locked against copying. To play them, Hearth Player needs <b>libdvdcss</b>, a small free library that unlocks them.</p>
     <p>Hearth OS can't include it for legal reasons. It comes from <b>RPM Fusion</b> (its "free-tainted" part), a well-known add-on source for Fedora. Whether you may use it where you live is up to you.</p>
     <p>You'll be asked for an administrator's password. It needs the internet and takes about a minute. You can remove it again from the disc screen.</p>`, 'Add DVD decoding');
  if (ok) decodingJob(true).catch(e => toast(e.message));
}
async function removeDecoding() {
  const ok = await confirmSheet('Remove DVD decoding?', '<p>Locked DVDs won\'t play any more. You can add it again from the disc screen.</p>', 'Remove', { danger: true });
  if (ok) decodingJob(false).catch(e => toast(e.message));
}

function controllerSheet() {
  const row = (b, t) => `<b>${b}</b><span>${esc(t)}</span>`;
  sheet(`<h2>Controller and remote</h2><div class="padmap">
    <h4>Playing a disc</h4>${row('A', 'Play or pause')}${row('◀ ▶', 'Back or forward 10 seconds')}${row('LT RT', 'Back or forward 30 seconds')}${row('LB RB', 'Chapter (DVD) or song (CD) before, after')}${row('▲ ▼', 'Louder, or the buttons')}${row('Y', 'Sound and subtitles')}${row('Select', 'The disc menu')}${row('Start', 'Full screen')}${row('X', 'Show the buttons')}${row('B', 'Back')}
    <h4>In the disc's own menu</h4>${row('✥', 'Pick a button')}${row('A', 'Press it')}${row('B', 'The titles list')}${row('X', 'Eject the disc')}
    <h4>On the titles list</h4>${row('✥', 'Move')}${row('A', 'Play what\'s picked')}${row('X', 'Eject the disc')}${row('Y', 'This list')}${row('LB RB', 'Switch tabs')}${row('B', 'Back to the film')}
    <h4>A remote or keyboard</h4>${row('← →', 'Seek, or move')}${row('OK', 'Choose, or play and pause')}${row('Back', 'Back')}${row('Menu', 'The disc\'s own menu')}${row('D', 'The titles list')}${row('⏯ ⏹', 'Play, pause, stop')}${row('⏏', 'Eject')}${row('⏮ ⏭', 'Chapter or song')}${row('?', 'This list')}
  </div><div class="acts"><button class="btn primary f" data-autofocus onclick="closeSheet()">Done</button></div>`, { wide: true });
}

// ---------------------------------------------------------------- a DVD's own menu (src/dvdnav.rs)
// libdvdnav runs the disc's menu; its picture comes as JPEG frames (an <img>), its buttons as rectangles drawn here.
// D-pad, arrows or a remote move between buttons, A / Enter / OK press one, the mouse points and clicks. B or Back goes
// to the titles screen (which stands in when a disc's menu can't run). A button that plays a title hands it to the
// usual player (seeking, sound, subtitles).
const M = { on: false, state: null };
const menuOn = () => M.on && !!S.discInfo;
function menuHints() { return '<span><b>✥</b>Pick a button</span><span><b>A</b>Press it</span><span><b>B</b>The titles list</span><span><b>Y</b>Controller buttons</span>'; }
async function startMenu(again = false) {
  const info = S.discInfo; if (!info?.menu) return false;
  try {
    const r = await call('disc_menu_start', { id: info.id, again });
    if (S.item?.disc) { saveResume(); video.pause(); }
    M.on = true; M.state = r.menu;
    $('#menuPic').src = r.url;
    // The menu's own music and sounds, at the player's volume; stopped with the menu.
    const snd = $('#menuSound');
    if (r.audio) { snd.src = r.audio; snd.volume = video.volume; snd.muted = video.muted; snd.play().catch(() => {}); }
    S.discPanel = false;
    $('#dvdmenu').hidden = false;
    setBodyMode(); updateButtons(); hints(); renderMenuBtns();
    if (document.activeElement && document.activeElement !== document.body) document.activeElement.blur();
    return true;
  } catch (e) { toast(`${e.message} Pick from its titles here.`); return false; }
}
function stopMenu() {
  if (!M.on) return;
  M.on = false;
  $('#menuPic').removeAttribute('src');
  const snd = $('#menuSound'); snd.pause(); snd.removeAttribute('src'); snd.load();
  $('#dvdmenu').hidden = true;
  call('disc_menu_stop').catch(() => {});
  hints();
}
function menuSize() { const img = $('#menuPic'); return { w: 720, h: img.naturalHeight === 576 ? 576 : 480 }; }
function renderMenuBtns() {
  const st = M.state || { buttons: [], selected: 0 }, { w, h } = menuSize();
  $('#menuBtns').innerHTML = st.buttons.map((b, i) => `<i data-mb="${i + 1}" class="${i + 1 === st.selected ? 'sel' : ''}" style="left:${b.x0 / w * 100}%;top:${b.y0 / h * 100}%;width:${(b.x1 - b.x0) / w * 100}%;height:${(b.y1 - b.y0) / h * 100}%"></i>`).join('');
}
$('#menuPic').addEventListener('load', renderMenuBtns);
async function menuCmd(cmd, x = 0, y = 0) {
  try { M.state = await call('disc_menu_cmd', { cmd, x: Math.round(x), y: Math.round(y) }); renderMenuBtns(); } catch (_) { /* the menu ended meanwhile */ }
}
function menuXY(e) { const r = $('#menuBox').getBoundingClientRect(), { w, h } = menuSize(); return [clamp((e.clientX - r.left) / r.width, 0, 1) * w, clamp((e.clientY - r.top) / r.height, 0, 1) * h]; }
$('#menuBox').addEventListener('click', e => { const [x, y] = menuXY(e); menuCmd('click', x, y); });
let menuHover = 0;
$('#menuBox').addEventListener('mousemove', e => {
  const b = e.target.closest('[data-mb]'); const n = b ? +b.dataset.mb : 0;
  if (n && n !== menuHover && n !== M.state?.selected) { const [x, y] = menuXY(e); menuCmd('hover', x, y); }
  menuHover = n;
});
listen('player://dvdmenu', ev => {
  const p = ev.payload || {};
  if (!M.on) return;
  if (p.menu) { M.state = p.menu; renderMenuBtns(); }
  if (p.restart) startMenu(true);
  if (p.play) {
    // A button played a title: the usual player takes it from its chapter.
    stopMenu();
    const t = S.discInfo?.titles.find(x => x.n === p.play.title);
    const ch = t?.chapters[(p.play.part || 1) - 1];
    if (t) playTitle(t.n, ch && ch.start > 0.5 ? ch.start + 0.01 : null); else showDiscPanel(true);
  }
  if (p.end) { stopMenu(); showDiscPanel(true); }
});
// Keys and controller buttons while the disc's menu is on (true: taken).
function menuKey(k) {
  const dirs = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right', up: 'up', down: 'down', left: 'left', right: 'right' };
  if (dirs[k]) { menuCmd(dirs[k]); return true; }
  if (k === 'Enter' || k === ' ' || k === 'a') { menuCmd('activate'); return true; }
  if (['Escape', 'Backspace', 'BrowserBack', 'GoBack', 'b', 'select'].includes(k)) { stopMenu(); showDiscPanel(true); return true; }
  if (k === 'ContextMenu' || k === 'MediaTopMenu' || k === 'TV') { menuCmd('root'); return true; }
  if (k === 'y' || k === '?') { controllerSheet(); return true; }
  if (k === 'Eject' || k === 'x') { stopMenu(); ejectDisc(); return true; }
  return false;
}

// ---------------------------------------------------------------- buttons
const ACT = {
  goOpen: () => setView('open'), playPause, prev, next, back10: () => seekBy(-10), fwd10: () => seekBy(10),
  mute: () => { video.muted = !video.muted; osd(video.muted ? 'Muted' : 'Sound on'); },
  shuffle: () => { S.shuffle = !S.shuffle; makeOrder(); if (S.shuffle && S.qi >= 0) { const k = S.order.indexOf(S.qi); [S.order[0], S.order[k]] = [S.order[k], S.order[0]]; } S.settings.shuffle = S.shuffle; saveSettings(); updateButtons(); osd(S.shuffle ? 'Shuffle on' : 'Shuffle off'); },
  repeat: () => { S.repeat = { off: 'all', all: 'one', one: 'off' }[S.repeat]; S.settings.repeat = S.repeat; saveSettings(); updateButtons(); osd({ off: 'Repeat off', all: 'Repeat all', one: 'Repeat this one' }[S.repeat]); },
  speed: speedMenu, subs: subsMenu, audio: audioMenu, chapters: chaptersMenu, queue: queueMenu, shot: screenshot,
  discMenu: () => toggleDiscPanel(), eject: () => ejectDisc(), playDisc: () => openDisc(S.drives.find(d => d.media)?.device || null),
  edit: () => (E.on ? closeEdit() : openEdit()), full: () => setFull(!S.full),
  up: upDir, playFolder: () => { if (S.pick) { cancelPick(); return; } openAndPlay([S.dir]); }, podGo: () => podSearch($('#podSearch').value),
};
document.addEventListener('click', e => {
  const a = e.target.closest('[data-act]'); if (a && ACT[a.dataset.act]) { ACT[a.dataset.act](); return; }
  const tab = e.target.closest('[data-view]'); if (tab) { if (S.pick) cancelPick(); setView(tab.dataset.view); }
});
$('#vol').addEventListener('input', e => setVolume(e.target.value / 100));
$('#stage').addEventListener('dblclick', () => { if (isVideo()) setFull(!S.full); });
$('#stage').addEventListener('click', e => { if (e.target === video) playPause(); });
// Files dropped on the window.
document.addEventListener('dragover', e => e.preventDefault());

// ---------------------------------------------------------------- keyboard
function inField(e) { const t = e.target; return (t.tagName === 'INPUT' && t.type !== 'range') || t.tagName === 'TEXTAREA'; }
function back() {
  if (oskTarget) { const i = oskTarget; closeOsk(); i.focus(); return true; }
  if (closeSheet()) return true;
  if (S.pick) return cancelPick();
  if (discPanelOn() && S.item) { showDiscPanel(false); return true; }
  if (E.on) { closeEdit(); return true; }
  if (S.full) { setFull(false); return true; }
  if (S.view === 'pods' && S.podShow) { S.podShow = null; renderPods(); return true; }
  if (S.view === 'open' && S.dir) { upDir(); return true; }
  if (S.view !== 'play' && S.item) { setView('play'); return true; }
  if (S.view === 'play' && document.activeElement && document.activeElement !== document.body && controlsFocused()) { document.activeElement.blur(); return true; }
  return false;
}
const controlsFocused = () => { const a = document.activeElement; return !!a && (a.closest('#controls') || a.closest('#tabs') || a.closest('#tracks')); };
// In Now playing with nothing focused (or the seek bar), arrows and letters drive playback; otherwise arrows move around.
function playKeys() { return S.view === 'play' && !topSheet() && !E.on && (!document.activeElement || document.activeElement === document.body || document.activeElement === video || document.activeElement === seekEl); }
document.addEventListener('keydown', e => {
  S.pad = false; document.body.classList.remove('pad'); wake();
  const k = e.key, ctrl = e.ctrlKey || e.metaKey;
  if (inField(e)) {
    if (k === 'Escape') { e.preventDefault(); e.target.blur(); }
    return;
  }
  if (S.view === 'play' && menuOn() && !topSheet() && !ctrl && menuKey(k.length === 1 ? k.toLowerCase() : k)) { e.preventDefault(); return; }
  if (k === 'Escape' || k === 'BrowserBack' || k === 'GoBack' || (k === 'Backspace' && !inField(e))) { if (back()) e.preventDefault(); return; }
  // Remote controls: Menu (or the context menu key) for the disc menu, Eject, Stop.
  if ((k === 'ContextMenu' || k === 'TV' || k === 'MediaTopMenu') && S.discInfo) { e.preventDefault(); if (S.discInfo.menu) startMenu().then(ok => { if (!ok) toggleDiscPanel(); }); else toggleDiscPanel(); return; }
  if (k === 'Eject' && S.discInfo?.device) { e.preventDefault(); ejectDisc(); return; }
  if (k === 'MediaStop' && S.item) { e.preventDefault(); video.pause(); if (S.discInfo) showDiscPanel(true); return; }
  if (k === '?' && !inField(e)) { e.preventDefault(); controllerSheet(); return; }
  if (ctrl && k.toLowerCase() === 'o') { e.preventDefault(); setView('open'); return; }
  if (ctrl && k.toLowerCase() === 'z') { e.preventDefault(); undo(); return; }
  if (ctrl && (k.toLowerCase() === 'y' || (e.shiftKey && k.toLowerCase() === 'z'))) { e.preventDefault(); redo(); return; }
  if (ctrl && k.toLowerCase() === 's' && E.on) { e.preventDefault(); saveEdit(); return; }
  if (ctrl && k.toLowerCase() === 'q') { e.preventDefault(); call('quit').catch(() => {}); return; }
  if (k === 'F11') { e.preventDefault(); if (isVideo()) setFull(!S.full); return; }
  if (S.view === 'play' && !topSheet() && discPanelOn()) {
    if (k === 'Enter' && document.activeElement === document.body) { e.preventDefault(); focusables()[0]?.focus(); return; }
    if (k.toLowerCase() === 'd' && S.item) { e.preventDefault(); toggleDiscPanel(); return; }
    if (k === 'MediaPlayPause' && S.item) { playPause(); return; }
  } else if (S.view === 'play' && !topSheet()) {
    const letter = k.length === 1 ? k.toLowerCase() : '';
    if (letter === 'd' && S.discInfo) { e.preventDefault(); toggleDiscPanel(); return; }
    const media = { ' ': playPause, k: playPause, j: () => seekBy(-10), l: () => seekBy(10), m: ACT.mute, f: () => isVideo() && setFull(!S.full), n: next, p: prev, v: cycleSubs, s: screenshot, e: ACT.edit, q: queueMenu, a: () => audioTracks().length > 1 && audioMenu(), '[': () => speedStep(-1), ']': () => speedStep(1), '=': () => setSpeed(1), ',': () => { video.pause(); seekBy(-1 / 25); }, '.': () => { video.pause(); seekBy(1 / 25); } };
    if (E.on) { media.i = () => change(s => { s.start = video.currentTime; }); media.o = () => change(s => { s.end = video.currentTime; }); }
    const onButton = document.activeElement && document.activeElement.tagName === 'BUTTON';
    if (letter && media[letter] && !(onButton && letter === ' ') && !ctrl) { e.preventDefault(); media[letter](); return; }
    if (k === ' ' && !onButton) { e.preventDefault(); playPause(); return; }
    if (k === 'MediaPlayPause') { playPause(); return; }
    if (k === 'MediaTrackNext') { next(); return; }
    if (k === 'MediaTrackPrevious') { prev(); return; }
    if (k === 'PageUp') { e.preventDefault(); chapterStep(-1); return; }
    if (k === 'PageDown') { e.preventDefault(); chapterStep(1); return; }
    if (/^[0-9]$/.test(k) && !E.on) { seekTo(duration() * (+k / 10)); return; }
    if (playKeys() || (E.on && document.activeElement === seekEl)) {
      if (k === 'ArrowLeft') { e.preventDefault(); seekBy(e.shiftKey ? -60 : -5); return; }
      if (k === 'ArrowRight') { e.preventDefault(); seekBy(e.shiftKey ? 60 : 5); return; }
      if (k === 'ArrowUp' && document.activeElement !== seekEl) { e.preventDefault(); setVolume(video.volume + 0.05); return; }
      if (k === 'ArrowDown' && document.activeElement !== seekEl) { e.preventDefault(); $('#ppBtn').focus(); return; }
      if (k === 'Home') { seekTo(0); return; }
      if (k === 'End') { seekTo(duration() - 1); return; }
    }
  }
  const dirs = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
  if (dirs[k]) { e.preventDefault(); move(dirs[k]); return; }
  if (k === 'Enter' && document.activeElement?.dataset?.osk) { e.preventDefault(); openOsk(document.activeElement); }
});

// ---------------------------------------------------------------- moving around: D-pad, stick, arrow keys
function focusables() {
  const root = topSheet() || (E.on && !S.pick ? $('#editPanel') : document);
  return [...root.querySelectorAll('.f')].filter(el => !el.disabled && el.offsetParent !== null && !el.closest('[hidden]') && (root !== document || !el.closest('#editPanel')));
}
function move(dir) {
  const cur = document.activeElement;
  if (cur?.type === 'range' && (dir === 'left' || dir === 'right')) { cur.value = Number(cur.value) + (dir === 'right' ? 5 : -5); cur.dispatchEvent(new Event('input', { bubbles: true })); return; }
  if (cur === seekEl && (dir === 'left' || dir === 'right')) { seekBy(dir === 'right' ? 5 : -5); showThumb(video.currentTime / (duration() || 1)); return; }
  const all = focusables(); if (!all.length) return;
  if (!cur || !all.includes(cur)) { (all.find(el => el.closest('#controls')) && S.view === 'play' ? $('#ppBtn') : all[0]).focus(); return; }
  const a = cur.getBoundingClientRect(); const ax = a.left + a.width / 2, ay = a.top + a.height / 2;
  let best = null, bestScore = Infinity;
  for (const el of all) {
    if (el === cur) continue;
    const b = el.getBoundingClientRect(); const bx = b.left + b.width / 2, by = b.top + b.height / 2;
    const dx = bx - ax, dy = by - ay;
    const ok = dir === 'left' ? b.right <= a.left + 4 : dir === 'right' ? b.left >= a.right - 4 : dir === 'up' ? b.bottom <= a.top + 4 : b.top >= a.bottom - 4;
    if (!ok) continue;
    const main = dir === 'left' || dir === 'right' ? Math.abs(dx) : Math.abs(dy);
    const cross = dir === 'left' || dir === 'right' ? Math.abs(dy) : Math.abs(dx);
    const score = main + cross * 2.2;
    if (score < bestScore) { bestScore = score; best = el; }
  }
  if (best) { best.focus(); best.scrollIntoView({ block: 'nearest', inline: 'nearest' }); }
  else if (dir === 'up' && S.view === 'play' && cur.closest('#controls')) { cur.blur(); }
}
function choose() {
  const el = document.activeElement;
  if (!el || el === document.body || el === video) { if (S.view === 'play' && S.item) playPause(); else focusables()[0]?.focus(); return; }
  if (el === seekEl) { playPause(); return; }
  if (el.dataset.osk) { if (!oskTarget) openOsk(el); return; }
  if (el.tagName === 'BUTTON') el.click();
}
// Controllers (the standard mapping). In Now playing with the controls hidden: A play/pause, D-pad left/right
// seek, up/down volume, LB/RB previous/next, LT/RT seek 30 s, X shows the controls, Y edit, Start full screen.
// Elsewhere the D-pad moves, A chooses, B goes back, LB/RB switch tabs.
const held = {}; let lastPadMove = 0;
function pad() {
  const pads = HearthGamepad.pads();
  const now = performance.now();
  const st = { up: 0, down: 0, left: 0, right: 0, a: 0, b: 0, x: 0, y: 0, lb: 0, rb: 0, lt: 0, rt: 0, start: 0, select: 0 };
  for (const g of pads) {
    const bt = i => g.buttons[i]?.pressed;
    const ax = g.axes[0] || 0, ay = g.axes[1] || 0;
    st.up ||= bt(12) || ay < -.6; st.down ||= bt(13) || ay > .6; st.left ||= bt(14) || ax < -.6; st.right ||= bt(15) || ax > .6;
    st.a ||= bt(0); st.b ||= bt(1); st.x ||= bt(2); st.y ||= bt(3); st.lb ||= bt(4); st.rb ||= bt(5); st.lt ||= (g.buttons[6]?.value || 0) > .5; st.rt ||= (g.buttons[7]?.value || 0) > .5;
    st.select ||= bt(8); st.start ||= bt(9);
  }
  for (const [k, on] of Object.entries(st)) {
    if (!on) { held[k] = 0; continue; }
    const first = !held[k];
    const repeatable = ['up', 'down', 'left', 'right', 'lt', 'rt'].includes(k);
    const repeat = held[k] && now - held[k] > 380 && now - lastPadMove > 110;
    if (first) held[k] = now;
    if (!(first || (repeat && repeatable))) continue;
    lastPadMove = now; S.pad = true; document.body.classList.add('pad'); wake();
    padPress(k);
  }
}
function padPress(k) {
  const bare = playKeys();
  if (k === 'b') { back(); return; }
  if (S.view === 'play' && menuOn() && !topSheet()) { menuKey(k); return; }
  // A disc: Select opens and closes the disc menu (a remote's Menu button), Y the sound and subtitles.
  if (k === 'select' && S.view === 'play' && S.discInfo && !topSheet()) { toggleDiscPanel(); return; }
  if (S.view === 'play' && discPanelOn() && !topSheet()) {
    if (k === 'a') { choose(); return; }
    if (k === 'x' && S.discInfo?.device) { ejectDisc(); return; }
    if (k === 'y') { controllerSheet(); return; }
    if (k === 'lb' || k === 'rb') { cycleView(k === 'lb' ? -1 : 1); return; }
    if (['up', 'down', 'left', 'right'].includes(k)) { move(k); return; }
    return;
  }
  if (k === 'y' && S.view === 'play' && S.item?.disc && !topSheet()) { discTracksMenu(); return; }
  if (k === 'lb' && (S.view !== 'play' || !S.item)) { cycleView(-1); return; }
  if (k === 'rb' && (S.view !== 'play' || !S.item)) { cycleView(1); return; }
  if (S.view === 'play' && S.item && !topSheet()) {
    if (k === 'lb') { prev(); return; } if (k === 'rb') { next(); return; }
    if (k === 'lt') { seekBy(-30); return; } if (k === 'rt') { seekBy(30); return; }
    if (k === 'y') { ACT.edit(); return; }
    if (k === 'start') { if (isVideo()) setFull(!S.full); return; }
    if (k === 'select') { cycleView(1); return; }
    if (k === 'x') { if (controlsFocused()) document.activeElement.blur(); else $('#ppBtn').focus(); return; }
    if (bare) {
      if (k === 'a') { playPause(); return; }
      if (k === 'left') { seekBy(-10); return; } if (k === 'right') { seekBy(10); return; }
      if (k === 'up') { setVolume(video.volume + 0.05); return; } if (k === 'down') { if (document.activeElement === seekEl) setVolume(video.volume - 0.05); else $('#ppBtn').focus(); return; }
    }
  }
  if (k === 'a') choose();
  else if (['up', 'down', 'left', 'right'].includes(k)) move(k);
  else if (k === 'x' && document.activeElement?.dataset?.osk) openOsk(document.activeElement);
}
function cycleView(d) { const v = ['play', 'open', 'pods']; if (S.pick) cancelPick(); setView(v[(v.indexOf(S.view) + d + v.length) % v.length]); setTimeout(() => (S.view === 'play' ? null : focusables()[0])?.focus(), 30); }
window.addEventListener('gamepadconnected', () => { const h = $('#hint'); if (h) h.textContent = 'Controller connected'; });

// ---------------------------------------------------------------- start
(async function start() {
  paintIcons();
  const init = await call('init');
  S.platform = init.platform; S.tools = init.tools || {}; S.places = init.places || []; S.home = init.home;
  S.settings = init.settings && typeof init.settings === 'object' ? init.settings : {};
  S.resume = (await call('store_get', { name: 'resume' }).catch(() => null)) || {};
  if (typeof S.resume !== 'object') S.resume = {};
  video.volume = typeof S.settings.volume === 'number' ? S.settings.volume : 1;
  video.muted = !!S.settings.muted;
  S.shuffle = !!S.settings.shuffle; S.repeat = S.settings.repeat || 'off';
  applyCue();
  await loadPods();
  setBodyMode(); updateButtons(); hints();
  const l = init.launch || {};
  if (l.view === 'podcasts') { S.podTab = 'shows'; setView('pods'); } else setView('play');
  listen('player://disc', ev => discEvent(ev.payload?.device));
  refreshDrives(false);
  if (l.disc != null) await openDisc(l.disc || null, { play: false });
  else if (l.files?.length) await openAndPlay(l.files);
  else if (l.view === 'music') { const m = S.places.find(p => p.kind === 'music'); setView('open'); if (m) openDir(m.path); }
  if (l.fullscreen) setFull(true);
  HearthGamepad.loop(pad);
  if (l.show === 'controller') controllerSheet();
  else if (l.show === 'decoding') addDecoding();
  refreshShows(false);
  document.body.dataset.ready = '1';
})().catch(e => { document.body.dataset.ready = 'error'; console.error(e); toast(String(e.message || e)); });

// ---------------------------------------------------------------- without the app: sample data
function demoInvoke(cmd, a) {
  const D = window.__demo || {};
  const base = D.base || 'media/';
  const files = D.files || [{ name: 'Sample clip.webm', kind: 'video', size: 2.1e6 }, { name: 'Song one.ogg', kind: 'audio', size: 3e5 }, { name: 'Song two.ogg', kind: 'audio', size: 3e5 }];
  const store = (window.__demoStore ||= {});
  const r = v => Promise.resolve(v);
  switch (cmd) {
    case 'init': return r({ platform: 'linux', version: 'preview', tools: D.tools || { ffmpeg: '/usr/bin/ffmpeg', ffprobe: '/usr/bin/ffprobe', encoders: ['libx264', 'aac'] }, settings: {}, launch: D.launch || { files: [], view: 'player' },
      places: [{ name: 'Videos', path: '/home/you/Videos', kind: 'videos' }, { name: 'Music', path: '/home/you/Music', kind: 'music' }, { name: 'Home', path: '/home/you', kind: 'home' }], podcastDir: '/home/you/Music/Podcasts', home: '/home/you' });
    case 'list_dir': return r(a.filter === 'images' ? [{ name: 'cover.jpg', path: `${a.path}/cover.jpg`, dir: false, kind: 'image', size: 1e5 }]
      : [{ name: 'Holiday', path: `${a.path}/Holiday`, dir: true, kind: 'folder', size: 0 }, ...files.map(f => ({ name: f.name, path: `${a.path}/${f.name}`, dir: false, kind: f.kind, size: f.size }))]);
    case 'open_paths': return r(a.paths.map(p => ({ path: p, name: baseName(p), kind: /\.(webm|mp4|mkv)$/i.test(p) ? 'video' : 'audio' })));
    case 'folder_tracks': return r(files.filter(f => f.kind === 'audio').map(f => ({ path: `${dirName(a.path)}/${f.name}`, name: f.name, kind: 'audio' })));
    case 'media_url': return r(isUrl(a.path) ? a.path : base + encodeURIComponent(baseName(a.path)));
    case 'probe': return r(D.probe ? D.probe(a.path) : { info: { duration: 0, streams: [], chapters: [] }, subs: [], tags: null });
    case 'subtitle_vtt': case 'subtitle_file_vtt': return r(D.vtt || 'WEBVTT\n\n00:00:00.000 --> 00:00:30.000\nHello from the <i>subtitles</i>\n');
    case 'art_url': return r(null);
    case 'tags_read': return r({ title: 'Song one', artist: 'Hearth Band', album: 'Embers', genre: 'Folk', track: 1, year: 2026, duration: 30, has_cover: false });
    case 'tags_write': (window.__demoCalls ||= []).push([cmd, a]); return r({ ...a.tags });
    case 'store_get': return r(store[a.name] ?? null);
    case 'store_set': store[a.name] = a.value; return r(null);
    case 'keep_awake': (window.__demoAwake ||= []).push(a.mode); return r(a.mode);
    case 'now_playing': window.__demoNowPlaying = a.state; return r(null);
    case 'fullscreen': return r(a.on);
    case 'edit_plan': (window.__demoCalls ||= []).push([cmd, a]); return r({ output: a.edit.input.replace(/(\.[^.]+)$/, ' (edited)$1'), reencode: a.edit.exact, how: a.edit.exact ? 'Re-encoding (video with libx264)' : 'Copying the streams (no quality loss)', duration: (a.edit.end || 10) - a.edit.start });
    case 'edit_start': {
      (window.__demoCalls ||= []).push([cmd, a]);
      const id = 7; const L = window.__demoListeners || {};
      setTimeout(() => L['player://job']?.({ payload: { id, pct: 0.5, text: 'Copying' } }), 50);
      setTimeout(() => L['player://job-done']?.({ payload: { id, ok: true, result: { path: a.edit.output } } }), 150);
      return r(id);
    }
    case 'net_get': return D.net ? r(D.net(a.url)) : Promise.reject('No internet in the preview');
    case 'save_frame': (window.__demoCalls ||= []).push([cmd, { path: a.path, at: a.at, png: a.png ? a.png.slice(0, 22) : null }]); return r('/home/you/Videos/Sample clip 00-00-02.png');
    case 'file_exists': return r(false);
    case 'disc_drives': return r(D.drives || []);
    case 'disc_open': return D.disc ? r(D.disc) : Promise.reject(new Error('There\'s no disc in the drive.'));
    case 'disc_play': (window.__demoCalls ||= []).push([cmd, a]); return r({ url: base + encodeURIComponent(D.discClip || 'Sample clip.webm'), start: a.at || 0, duration: (D.disc?.titles?.find(t => t.n === a.n)?.duration) || (D.disc?.tracks?.find(t => t.n === a.n)?.duration) || 12, restart: D.disc?.kind !== 'audio-cd' });
    case 'disc_names': return Promise.reject(new Error('No internet in the preview'));
    case 'dvd_decoding': return r(D.decoding || { installed: false, canAdd: true });
    case 'disc_menu_start': return D.menu ? r({ url: base + 'menu.jpg', menu: D.menu }) : Promise.reject(new Error('No menu in the preview.'));
    case 'disc_menu_cmd': (window.__demoCalls ||= []).push([cmd, a]); return r(D.menu || { buttons: [], selected: 0 });
    case 'disc_eject': (window.__demoCalls ||= []).push([cmd, a]); return r(null);
    default: (window.__demoCalls ||= []).push([cmd, a]); return r(null);
  }
}
