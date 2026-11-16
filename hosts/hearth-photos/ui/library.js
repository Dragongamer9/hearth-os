// Hearth Photos' library (1.14, docs/mockups/next-update/photos-1-library.png): every picture and video on this PC by
// day, with the lists on the left (All photos, Memories, Favorites, Videos, Game captures, People, Places, From your
// phone, Folders, Trash, your albums), the phone box, search, Import / Select / Slideshow / Size, Memories, and the
// date strip to jump by month and year. Opening a picture shows it in the viewer (app.js) with the rest of its day.
// The files are found by the app (src/library.rs); favorites and albums are your settings (photos.json), changed
// one at a time through library_change. Nothing leaves this PC.
'use strict';

(function () {
  const H = window.HP;
  const { invoke, url, esc, human, toast, dialog, store } = H;
  const $ = (s, el = document) => el.querySelector(s);
  const $$ = (s, el = document) => [...el.querySelectorAll(s)];
  const DAY = 864e5;

  const L = {
    items: [], store: {}, summary: {}, view: store.get('lib.view', 'all'), album: null, folder: null,
    q: '', size: store.get('lib.size', 200), selecting: false, picked: new Set(), focus: null, scanning: false,
  };
  const favs = () => new Set(L.store.favorites || []);
  const albums = () => (L.store.albums || []).filter(a => a && a.id);

  // ---------------------------------------------------------------- dates (taken times are as the camera wrote them)
  const asDate = ms => { const d = new Date(ms); return new Date(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate(), d.getUTCHours(), d.getUTCMinutes()); };
  const dayKey = d => `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
  const fmt = (d, o) => d.toLocaleDateString(undefined, o);
  const today0 = () => { const n = new Date(); return new Date(n.getFullYear(), n.getMonth(), n.getDate()); };
  const daysAgo = d => Math.round((today0() - new Date(d.getFullYear(), d.getMonth(), d.getDate())) / DAY);

  // ---------------------------------------------------------------- what's shown
  function kindWords(i) {
    return [i.kind, i.kind === 'capture' ? 'game capture screenshot' : '', i.kind === 'ai' ? 'ai made comfyui' : '', i.src === 'phone' ? 'phone iphone' : '',
      i.src === 'steam' ? 'steam' : '', i.src === 'gamemode' ? 'game mode' : '', i.game || '', i.camera || '', i.name, i.path].join(' ').toLowerCase();
  }
  function matches(i, q) {
    if (!q) return true;
    const d = asDate(i.taken);
    const hay = kindWords(i) + ' ' + fmt(d, { month: 'long', year: 'numeric' }).toLowerCase() + ' ' + fmt(d, { weekday: 'long' }).toLowerCase() + ' ' + (i.tags || []).join(' ').toLowerCase();
    return q.toLowerCase().split(/\s+/).filter(Boolean).every(w => hay.includes(w.replace(/s$/, '')));
  }
  function shown() {
    const f = favs();
    let v = L.items;
    switch (L.view) {
      case 'favorites': v = v.filter(i => f.has(i.path)); break;
      case 'videos': v = v.filter(i => i.kind === 'video'); break;
      case 'captures': v = v.filter(i => i.kind === 'capture'); break;
      case 'phone': v = v.filter(i => i.src === 'phone'); break;
      case 'album': { const a = albums().find(x => x.id === L.album); const set = new Set(a ? a.items : []); v = v.filter(i => set.has(i.path)); break; }
      case 'folder': v = v.filter(i => dirOf(i.path) === L.folder); break;
      case 'trash': v = L.trash || []; break;
    }
    return v.filter(i => matches(i, L.q));
  }
  const dirOf = p => p.slice(0, p.lastIndexOf('/')) || '/';

  /** Groups by day: Today, Yesterday, Earlier this week (one group), then each day this month, then by month. */
  function groups(list) {
    const out = [];
    let g = null;
    for (const i of list) {
      const d = asDate(i.taken), ago = daysAgo(d);
      const key = ago === 0 ? 'today' : ago === 1 ? 'yesterday' : ago < 7 && ago > 1 ? 'week' : ago < 31 ? dayKey(d) : `${d.getFullYear()}-${d.getMonth()}`;
      if (!g || g.key !== key) { g = { key, items: [], first: d, last: d, ago }; out.push(g); }
      g.items.push(i);
      g.last = d;
    }
    for (const g of out) {
      const n = g.items.length, word = n === 1 ? 'item' : (g.items.every(i => i.kind === 'video') ? 'videos' : 'photos');
      const day = d => fmt(d, { weekday: 'short', day: 'numeric', month: 'short' });
      g.title = g.key === 'today' ? 'Today' : g.key === 'yesterday' ? 'Yesterday' : g.key === 'week' ? 'Earlier this week'
        : g.ago < 31 ? fmt(g.first, { weekday: 'long', day: 'numeric', month: 'long' }) : fmt(g.first, { month: 'long', year: 'numeric' });
      const when = g.key === 'week' ? `${day(g.last)} – ${day(g.first)}` : g.ago < 31 ? day(g.first) : '';
      const srcs = new Set(g.items.map(i => i.src));
      const from = srcs.size === 1 ? { phone: `from ${phoneName()}`, steam: 'from Steam', gamemode: 'from Game Mode', ai: 'made by your AI', desktop: 'screenshots' }[[...srcs][0]] : '';
      g.sub = [when, `${n} ${n === 1 ? 'item' : word}`, from].filter(Boolean).join(' · ');
      g.month = `${g.first.getFullYear()}-${g.first.getMonth()}`;
    }
    return out;
  }
  const phoneName = () => (L.summary && L.summary.phoneName) || 'your phone';

  /** Justified rows: every picture at its own shape, each row filling the width. */
  function rows(items, width, target) {
    const gap = 6, out = [];
    let row = [], sum = 0;
    const ratio = i => { const r = i.w && i.h ? i.w / i.h : (i.kind === 'video' ? 16 / 9 : 4 / 3); return Math.min(3, Math.max(0.45, r)); };
    for (const i of items) {
      row.push(i); sum += ratio(i);
      const h = (width - gap * (row.length - 1)) / sum;
      if (h <= target) { out.push({ items: row, h: Math.max(80, h) }); row = []; sum = 0; }
    }
    if (row.length) out.push({ items: row, h: target, last: true });
    return out.map(r => ({ ...r, cells: r.items.map(i => ({ i, w: ratio(i) * r.h, h: r.h })) }));
  }

  // ---------------------------------------------------------------- the page
  const root = document.createElement('div');
  root.id = 'library';
  root.hidden = true;
  root.innerHTML = `
    <aside id="lside" class="scroll">
      <div class="brand"><span class="logo"><svg viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="16" rx="3"/><circle cx="9" cy="10" r="2"/><path d="m4 18 5-5 4 4 3-3 4 4"/></svg></span><b>Photos</b></div>
      <nav id="lnav"></nav>
      <div class="lh">Your albums</div>
      <nav id="lalbums"></nav>
      <span class="sp"></span>
      <div id="phonebox"></div>
    </aside>
    <main id="lmain">
      <header id="ltop">
        <label class="search"><svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="7"/><path d="m20 20-4-4"/></svg>
          <input id="lq" data-f placeholder='Search: "sunset at the lake", "ROG Ally screenshot"…' spellcheck="false" autocomplete="off">
          <span class="here">Searched on this PC</span></label>
        <span class="sp"></span>
        <button class="btn" data-f data-lact="import">Import</button>
        <button class="btn" data-f data-lact="select" id="lselect">Select</button>
        <button class="btn" data-f data-lact="slideshow">Slideshow</button>
        <label class="size">Size <input type="range" id="lsize" data-f min="110" max="340" step="10"></label>
      </header>
      <div id="lselbar" hidden><b id="lselcount"></b><span class="sp"></span>
        <button class="btn" data-f data-lact="selFav">♥ Favorite</button><button class="btn" data-f data-lact="selAlbum">Add to album</button>
        <button class="btn" data-f data-lact="selTrash">Trash</button><button class="btn primary" data-f data-lact="select">Done</button></div>
      <div id="lbody" class="scroll" tabindex="-1"><div id="lcontent"></div></div>
    </main>
    <nav id="ldates"></nav>`;
  document.body.appendChild(root);
  const body = $('#lbody', root), content = $('#lcontent', root);

  const NAV = [
    ['all', 'All photos'], ['memories', 'Memories'], ['favorites', 'Favorites'], ['videos', 'Videos'], ['captures', 'Game captures'],
    ['people', 'People'], ['places', 'Places'], ['phone', 'From your phone'], ['folders', 'Folders'], ['trash', 'Trash'],
  ];
  const n = x => x == null ? '' : x.toLocaleString();
  function counts() {
    const f = favs();
    return {
      all: L.items.length, memories: memories().length, favorites: L.items.filter(i => f.has(i.path)).length,
      videos: L.items.filter(i => i.kind === 'video').length, captures: L.items.filter(i => i.kind === 'capture').length,
      people: (L.store.people || []).length || null, places: placeCount() || null, phone: L.items.filter(i => i.src === 'phone').length, folders: null,
      trash: L.summary.trash || null,
    };
  }
  function placeCount() { const s = new Set(); for (const i of L.items) if (i.gps) s.add(`${i.gps[0].toFixed(1)},${i.gps[1].toFixed(1)}`); return s.size; }

  function renderSide() {
    const c = counts();
    $('#lnav', root).innerHTML = NAV.map(([id, label]) =>
      `<button class="li${L.view === id ? ' on' : ''}" data-f data-view="${id}"><span>${label}</span><i>${id === 'all' || c[id] ? n(c[id]) : ''}</i></button>`).join('');
    $('#lalbums', root).innerHTML = albums().map(a => {
      const cover = a.cover || (a.items || [])[0];
      return `<button class="li al${L.view === 'album' && L.album === a.id ? ' on' : ''}" data-f data-album="${esc(a.id)}"><span class="sw">${cover ? `<img alt="" loading="lazy" src="${url('thumb', cover, '?s=80')}">` : ''}</span><span>${esc(a.name)}</span></button>`;
    }).join('') + `<button class="li add" data-f data-lact="newAlbum"><span>+ New album</span></button>`;
    const s = L.summary || {};
    const last = s.phoneLast ? agoText(s.phoneLast) : null;
    $('#phonebox', root).innerHTML = `
      <div class="ph1"><i class="dot${last && Date.now() - s.phoneLast < 864e5 ? ' live' : ''}"></i><b>${esc(phoneName().replace(/^your phone$/, 'Your phone'))}</b></div>
      <p>${last ? `New photos come in by themselves · last ${last}` : 'Photos from your phone show up here when they come in (iCloud, Hearth Link or a cable).'}</p>
      <p class="small">${n(s.count || 0)} photos · ${human(s.bytes || 0)} · all on this PC, nothing leaves it${L.scanning ? ' · looking for new ones…' : ''}</p>`;
  }
  function agoText(ms) {
    const m = Math.round((Date.now() - ms) / 6e4);
    if (m < 1) return 'just now'; if (m < 60) return `${m} min ago`;
    const h = Math.round(m / 60); if (h < 24) return `${h} h ago`;
    return `${Math.round(h / 24)} days ago`;
  }

  // ---------------------------------------------------------------- Memories (worked out here, from your own photos)
  function memories() {
    if (L._mem && L._mem.n === L.items.length) return L._mem.list;
    const list = [], now = new Date(), photos = L.items.filter(i => i.kind !== 'video');
    const md = d => d.getMonth() * 32 + d.getDate();
    // A year (or more) ago today: within three days of today's date in an earlier year.
    for (let y = 1; y <= 10; y++) {
      const hit = photos.filter(i => { const d = asDate(i.taken); return d.getFullYear() === now.getFullYear() - y && Math.abs(md(d) - md(now)) <= 3; });
      if (hit.length >= 3) { list.push({ kind: `${y === 1 ? '1 year' : y + ' years'} ago today`, title: fmt(asDate(hit[0].taken), { month: 'long', year: 'numeric' }), items: hit }); break; }
    }
    // A trip: the biggest run of days in a row (2–6) with 20 or more photos in the last year.
    const byDay = new Map();
    for (const i of photos) { const d = asDate(i.taken); if ((now - d) / DAY > 366) continue; const k = dayKey(d); (byDay.get(k) || byDay.set(k, []).get(k)).push(i); }
    const days = [...byDay.values()].map(v => ({ d: asDate(v[0].taken), v })).sort((a, b) => a.d - b.d);
    let best = null;
    for (let a = 0; a < days.length; a++) {
      let run = [days[a]];
      for (let b = a + 1; b < days.length && run.length < 6; b++) { if (Math.round((new Date(days[b].d.toDateString()) - new Date(run[run.length - 1].d.toDateString())) / DAY) === 1) run.push(days[b]); else break; }
      const cnt = run.reduce((s, r) => s + r.v.length, 0);
      if (run.length >= 2 && cnt >= 20 && (!best || cnt > best.cnt)) best = { run, cnt };
    }
    if (best) {
      const d0 = best.run[0].d, weekend = best.run.every(r => [0, 5, 6].includes(r.d.getDay()));
      list.push({ kind: 'Trip', title: `${weekend ? fmt(d0, { month: 'long' }) + ' weekend' : fmt(d0, { day: 'numeric', month: 'short' }) + ' – ' + fmt(best.run[best.run.length - 1].d, { day: 'numeric', month: 'short' })} · ${best.cnt} photos`, items: best.run.flatMap(r => r.v) });
    }
    // Game captures this month.
    const caps = L.items.filter(i => { const d = asDate(i.taken); return i.kind === 'capture' && d.getFullYear() === now.getFullYear() && d.getMonth() === now.getMonth(); });
    if (caps.length >= 2) list.push({ kind: 'Game captures', title: 'Your best shots this month', items: caps });
    // This season.
    const season = m => ['Winter', 'Winter', 'Spring', 'Spring', 'Spring', 'Summer', 'Summer', 'Summer', 'Fall', 'Fall', 'Fall', 'Winter'][m];
    const s = season(now.getMonth());
    const inSeason = photos.filter(i => { const d = asDate(i.taken); return season(d.getMonth()) === s && (now - d) / DAY < 120; });
    if (inSeason.length >= 4) list.push({ kind: 'Seasons', title: `${s} ${now.getFullYear()}`, items: inSeason });
    for (const m of list) m.cover = pickCover(m.items);
    L._mem = { n: L.items.length, list };
    return list;
  }
  /** A memory's picture: a favorite when there is one, else the widest-looking photo in the middle of it. */
  function pickCover(items) {
    const f = favs();
    return items.find(i => f.has(i.path) && i.kind !== 'video') || items.filter(i => i.w >= i.h)[Math.floor(items.length / 3)] || items[0];
  }

  // ---------------------------------------------------------------- drawing the photos
  function tile(c, f) {
    const i = c.i;
    const badge = i.kind === 'video' ? `<span class="len">▶ ${dur(i.secs)}</span>` : '';
    const label = i.kind === 'capture' ? `<span class="lab">${esc(i.game ? i.game + ' · ' + (i.src === 'steam' ? 'Steam' : 'Game Mode') : i.src === 'steam' ? 'Steam' : 'Game Mode')}</span>`
      : i.kind === 'screenshot' ? '<span class="lab">Screenshot</span>' : i.kind === 'ai' ? '<span class="lab">Made by your AI</span>' : '';
    const thumb = `<img alt="" loading="lazy" decoding="async" draggable="false" src="${url('thumb', i.path, `?s=${Math.min(640, Math.round(c.h * 1.6))}&v=${i.mtime}`)}">`;
    return `<button class="tile${L.picked.has(i.path) ? ' picked' : ''}" data-f data-path="${esc(i.path)}" style="width:${c.w.toFixed(1)}px;height:${c.h.toFixed(1)}px" title="${esc(i.name)}">${thumb}${i.kind === 'video' ? '<span class="vid"></span>' : ''}${label}${badge}${f.has(i.path) ? '<span class="fav">♥</span>' : ''}${L.selecting ? '<span class="tick"></span>' : ''}</button>`;
  }
  const dur = s => { if (!s && s !== 0) return ''; s = Math.round(s); return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`; };

  let groupsNow = [];
  /** A mistake while drawing shows on the page and in the app's output (CI's screenshots and log show it). */
  function render() {
    try { draw(); } catch (e) {
      content.innerHTML = `<div class="empty"><b>Photos couldn't show this</b><p>${esc(String(e && e.stack || e))}</p></div>`;
      invoke('log', { line: 'JSERROR library: ' + (e && e.stack || e) }).catch(() => {});
    }
  }
  function draw() {
    renderSide();
    const width = Math.max(300, body.clientWidth - 60);
    const v = shown();
    let h = '';
    if (L.view === 'memories') { renderMemoriesPage(); return; }
    if (L.view === 'folders') { renderFolders(); return; }
    if (L.view === 'people' || L.view === 'places') { renderSoon(); return; }
    if (L.view === 'all' && !L.q) {
      const mem = memories().slice(0, 4);
      if (mem.length) h += `<section class="mem"><h2>Memories <small>made on this PC from your own photos</small></h2><div class="cards">${mem.map((m, k) => memCard(m, k)).join('')}</div></section>`;
    }
    groupsNow = groups(v);
    const f = favs();
    for (const g of groupsNow) {
      const rs = rows(g.items, width, L.size);
      const height = rs.reduce((s, r) => s + r.h + 6, 0) + 44;
      h += `<section class="day" data-month="${g.month}" style="contain-intrinsic-size:auto ${Math.round(height)}px"><h2>${esc(g.title)} <small>${esc(g.sub)}</small></h2>`
        + rs.map(r => `<div class="row">${r.cells.map(c => tile(c, f)).join('')}</div>`).join('') + '</section>';
    }
    if (!v.length) h += emptyText();
    content.innerHTML = h;
    renderDates();
    if (L.focus) { const t = content.querySelector(`.tile[data-path="${CSS.escape(L.focus)}"]`); if (t && root.contains(document.activeElement)) t.focus(); }
  }
  function emptyText() {
    if (L.q) return `<div class="empty"><b>Nothing found for “${esc(L.q)}”</b><p>Search looks at names, dates, games, cameras and what Photos found in your pictures.</p></div>`;
    if (L.view === 'trash') return '<div class="empty"><b>The Trash is empty</b></div>';
    if (L.scanning && !L.items.length) return '<div class="empty"><b>Looking for your photos…</b><p>Pictures, Videos, Steam screenshots and the folders you add.</p></div>';
    return '<div class="empty"><b>No photos here yet</b><p>Photos looks in your Pictures and Videos folders and Steam\'s screenshots. Import adds another folder.</p></div>';
  }
  function memCard(m, k) {
    const c = m.cover;
    return `<button class="card" data-f data-mem="${k}">${c ? `<img alt="" loading="lazy" src="${url('thumb', c.path, `?s=640&v=${c.mtime}`)}">` : ''}<span class="shade"></span><span class="ck">${esc(m.kind)}</span><b>${esc(m.title)}</b></button>`;
  }
  function renderMemoriesPage() {
    const mem = memories();
    content.innerHTML = `<section class="mem"><h2>Memories <small>made on this PC from your own photos</small></h2><div class="cards wrap">${mem.map((m, k) => memCard(m, k)).join('')}</div>${mem.length ? '' : '<div class="empty"><b>No memories yet</b><p>They\'re made from your photos as you take more: a year ago today, trips, your game captures, the seasons.</p></div>'}</section>`;
    renderDates(true);
  }
  function renderFolders() {
    const by = new Map();
    for (const i of L.items) { const d = dirOf(i.path); (by.get(d) || by.set(d, []).get(d)).push(i); }
    const home = (L.home || '').replace(/\/$/, '');
    const list = [...by.entries()].sort((a, b) => a[0].localeCompare(b[0]));
    content.innerHTML = `<section class="folders"><h2>Folders <small>${list.length} folders</small></h2><div class="grid">${list.map(([d, v]) =>
      `<button class="fold" data-f data-folder="${esc(d)}"><span class="sw"><img alt="" loading="lazy" src="${url('thumb', v[0].path, '?s=200')}"></span><b>${esc(d.split('/').pop())}</b><small>${esc(home && d.startsWith(home) ? '~' + d.slice(home.length) : d)} · ${v.length}</small></button>`).join('')}</div></section>`;
    renderDates(true);
  }
  function renderSoon() {
    const people = L.view === 'people';
    content.innerHTML = `<div class="empty"><b>${people ? 'People' : 'Places'}</b><p>${people ? 'Faces are grouped on this PC; names only you see.' : 'Photos with a place, on a map.'}</p></div>`;
    renderDates(true);
  }

  /** The date strip: this year's months, then the years before. */
  function renderDates(hide) {
    const nav = $('#ldates', root);
    if (hide || !groupsNow.length) { nav.innerHTML = ''; return; }
    const now = new Date(), seen = new Set(), out = [];
    for (const g of groupsNow) {
      const y = g.first.getFullYear(), m = g.first.getMonth();
      const k = y === now.getFullYear() || (now - g.first) / DAY < 365 ? `${y}-${m}` : `${y}`;
      if (seen.has(k)) continue;
      seen.add(k);
      out.push({ k, label: k.includes('-') ? fmt(g.first, { month: 'short' }) : String(y), year: !k.includes('-'), month: g.month });
    }
    // A year label before the months of an earlier year (2025 · Dec, Oct…).
    let lastYear = now.getFullYear(), html = '';
    for (const o of out) {
      const y = +o.month.split('-')[0];
      if (!o.year && y !== lastYear) { html += `<span class="yr">${y}</span>`; lastYear = y; }
      html += `<button class="dt${o.year ? ' yr' : ''}" data-f data-jump="${o.month}" data-jumpyear="${o.year ? o.k : ''}">${esc(o.label)}</button>`;
    }
    nav.innerHTML = html;
    markDate();
  }
  function markDate() {
    const top = body.getBoundingClientRect().top + 60;
    const cur = $$('section.day', content).find(s => s.getBoundingClientRect().bottom > top);
    if (!cur) return;
    const [y] = cur.dataset.month.split('-');
    $$('#ldates .dt', root).forEach(b => b.classList.toggle('on', b.dataset.jump === cur.dataset.month || (b.dataset.jumpyear && b.dataset.jumpyear === y)));
  }
  body.addEventListener('scroll', () => requestAnimationFrame(markDate), { passive: true });

  // ---------------------------------------------------------------- doing things
  function itemList() { return groupsNow.flatMap(g => g.items); }
  function open(path, opts = {}) {
    const g = groupsNow.find(g => g.items.some(i => i.path === path));
    const list = opts.list || (g ? g.items : itemList());
    const pics = list.filter(i => i.kind !== 'video');
    const it = list.find(i => i.path === path);
    if (it && it.kind === 'video') { invoke('open_video', { path }).catch(e => toast(String(e))); return; }
    const k = Math.max(0, pics.findIndex(i => i.path === path));
    L.focus = path;
    hide();
    H.openList(pics.map(toPic), k, { slideshow: opts.slideshow, fromLibrary: true });
  }
  const toPic = i => ({ name: i.name, path: i.path, size: i.size, mtime: i.mtime, taken: i.taken, kind: i.kind, game: i.game, src: i.src, gps: i.gps });

  async function change(c) {
    try { L.store = await invoke('library_change', { change: c }); L._mem = null; render(); return true; }
    catch (e) { toast(String(e)); return false; }
  }
  async function chooseAlbum(paths) {
    const list = albums();
    const pick = await dialog(`<h2>Add ${paths.length === 1 ? 'it' : paths.length + ' items'} to an album</h2>
      <div class="albumpick">${list.map((a, k) => `<button class="btn" data-f data-b2="${k}">${esc(a.name)}</button>`).join('')}</div>
      <label class="field">Or a new album <input id="newAlbumName" data-f placeholder="Lake trip 2026"></label>`,
      [{ label: 'Cancel', value: null }, { label: 'Make the new album', kind: 'primary', value: 'new' }],
      (v, done) => v.addEventListener('click', e => { const b = e.target.closest('[data-b2]'); if (b) done(list[+b.dataset.b2].id); }));
    if (!pick) return;
    if (pick === 'new') { const name = ($('#newAlbumName') || {}).value || ''; if (name.trim()) await change({ op: 'album.new', name, paths }); return; }
    await change({ op: 'album.add', id: pick, paths });
    toast('Added to the album');
  }
  window.HPLib = { chooseAlbum, change, favs, albums, get store() { return L.store; }, show, hide, visible: () => !root.hidden, items: () => L.items, phoneName, pad };

  /** A controller in the library: D-pad / stick moves, A opens, B leaves search, X slideshow, Y search, Start Select. */
  function pad(k) {
    const dir = { up: 'up', down: 'down', left: 'left', right: 'right' }[k];
    if (dir) { if (!root.contains(document.activeElement) || document.activeElement === body) { firstTile(); return; } H.move(dir, root); const a = document.activeElement; if (a && a.classList.contains('tile')) L.focus = a.dataset.path; return; }
    if (k === 'a') { const a = document.activeElement; if (a && root.contains(a) && a.click) a.click(); else firstTile(); return; }
    if (k === 'b') { if (document.activeElement === q) { firstTile(); return; } if (L.selecting) { toggleSelect(); return; } if (L.view !== 'all') { setView('all'); } return; }
    if (k === 'x') { acts.slideshow(); return; }
    if (k === 'y') { q.focus(); return; }
    if (k === 'lb' || k === 'rb') { const i = NAV.findIndex(x => x[0] === L.view); const nx = NAV[(Math.max(0, i) + (k === 'rb' ? 1 : NAV.length - 1)) % NAV.length]; setView(nx[0]); }
  }

  async function newAlbum() {
    const name = await dialog(`<h2>New album</h2><label class="field">Name <input id="na" data-f autofocus placeholder="Lake trip 2026"></label>`,
      [{ label: 'Cancel', value: null }, { label: 'Make it', kind: 'primary', value: 1 }]);
    const v = name && ($('#na') || {}).value;
    if (v && v.trim()) await change({ op: 'album.new', name: v, paths: [...L.picked] });
  }
  async function importFolder() {
    try {
      const p = await invoke('pick_folder');
      if (!p) return;
      if (await change({ op: 'folder.add', path: p })) { toast('Added. Looking through it…'); rescan(); }
    } catch (e) { toast(String(e)); }
  }
  function toggleSelect() {
    L.selecting = !L.selecting;
    if (!L.selecting) L.picked.clear();
    $('#lselbar', root).hidden = !L.selecting;
    $('#lselect', root).classList.toggle('on', L.selecting);
    updateSel();
    render();
  }
  function updateSel() { $('#lselcount', root).textContent = L.picked.size ? `${L.picked.size} picked` : 'Pick photos'; }
  async function trashPicked() {
    const paths = [...L.picked];
    if (!paths.length) return;
    let done = 0;
    for (const p of paths) { try { await invoke('trash', { path: p }); done++; } catch (e) { toast(String(e)); } }
    L.items = L.items.filter(i => !L.picked.has(i.path));
    L.picked.clear(); updateSel();
    L.summary.trash = (L.summary.trash || 0) + done;
    toast(`${done} moved to the Trash`);
    render();
  }

  const acts = {
    import: importFolder, select: toggleSelect, newAlbum,
    slideshow: () => { const v = itemList().filter(i => i.kind !== 'video'); if (v.length) open(v[0].path, { list: v, slideshow: true }); },
    selFav: () => { const p = [...L.picked]; if (p.length) { const all = p.every(x => favs().has(x)); change({ op: 'favorite', paths: p, on: !all }); } },
    selAlbum: () => { const p = [...L.picked]; if (p.length) chooseAlbum(p); },
    selTrash: trashPicked,
  };
  root.addEventListener('click', e => {
    const t = e.target.closest('[data-lact],[data-view],[data-album],.tile,[data-mem],[data-folder],[data-jump]');
    if (!t) return;
    if (t.dataset.lact) { acts[t.dataset.lact](); return; }
    if (t.dataset.view) { setView(t.dataset.view); return; }
    if (t.dataset.album) { L.view = 'album'; L.album = t.dataset.album; render(); body.scrollTop = 0; return; }
    if (t.dataset.folder) { L.view = 'folder'; L.folder = t.dataset.folder; render(); body.scrollTop = 0; return; }
    if (t.dataset.mem) { const m = (L.view === 'memories' ? memories() : memories().slice(0, 4))[+t.dataset.mem]; if (m) { const v = m.items.filter(i => i.kind !== 'video'); if (v.length) open(v[0].path, { list: v, slideshow: true }); } return; }
    if (t.dataset.jump) { jump(t.dataset.jump, t.dataset.jumpyear); return; }
    if (t.classList.contains('tile')) {
      const p = t.dataset.path;
      if (L.selecting || e.ctrlKey || e.shiftKey) { if (!L.selecting) toggleSelect(); L.picked.has(p) ? L.picked.delete(p) : L.picked.add(p); t.classList.toggle('picked'); updateSel(); return; }
      open(p);
    }
  });
  function jump(month, year) {
    const s = year ? $$('section.day', content).find(x => x.dataset.month.startsWith(year + '-')) : content.querySelector(`section.day[data-month="${month}"]`);
    if (s) body.scrollTo({ top: s.offsetTop - 8, behavior: 'smooth' });
  }
  async function setView(v) {
    L.view = v; L.album = null; store.set('lib.view', v === 'trash' ? 'all' : v);
    if (v === 'trash') { try { L.trash = await invoke('library_trash'); } catch (_) { L.trash = []; } }
    render(); body.scrollTop = 0;
  }
  const q = $('#lq', root);
  let qTimer = 0;
  q.addEventListener('input', () => { clearTimeout(qTimer); qTimer = setTimeout(() => { L.q = q.value.trim(); render(); body.scrollTop = 0; }, 160); });
  const size = $('#lsize', root);
  size.value = L.size;
  size.addEventListener('input', () => { L.size = +size.value; store.set('lib.size', L.size); render(); });
  let rz = 0;
  window.addEventListener('resize', () => { if (root.hidden) return; clearTimeout(rz); rz = setTimeout(render, 120); });

  // Keyboard and controller: arrows move between tiles (and everything else), Enter / A opens.
  root.addEventListener('keydown', e => {
    if (e.target === q && e.key === 'Escape') { q.value = ''; L.q = ''; render(); e.stopPropagation(); return; }
    if (e.target.tagName === 'INPUT' && e.target.type !== 'range') { if (e.key === 'ArrowDown') { e.preventDefault(); firstTile(); } return; }
    const d = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' }[e.key];
    if (d) { e.preventDefault(); e.stopPropagation(); H.move(d, root); const a = document.activeElement; if (a && a.classList.contains('tile')) L.focus = a.dataset.path; return; }
    if (e.key === 'Delete' && document.activeElement?.classList.contains('tile')) { L.picked = new Set([document.activeElement.dataset.path]); trashPicked(); }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') { e.preventDefault(); q.focus(); }
  });
  function firstTile() { const t = content.querySelector(L.focus ? `.tile[data-path="${CSS.escape(L.focus)}"]` : '.tile') || content.querySelector('.tile'); if (t) { t.focus(); t.classList.add('kf'); } }

  // ---------------------------------------------------------------- showing it, and finding the files
  function show() {
    if (!L.started) { boot({ home: H.S.home }); return; }
    root.hidden = false;
    $('#app').hidden = true;
    invoke('set_title', { title: 'Photos' }).catch(() => {});
    render();
    setTimeout(() => { const t = L.focus && content.querySelector(`.tile[data-path="${CSS.escape(L.focus)}"]`); if (t) { t.scrollIntoView({ block: 'center' }); t.focus(); if (H.S.pad) t.classList.add('kf'); } else firstTile(); }, 30);
  }
  function hide() { root.hidden = true; $('#app').hidden = false; }

  async function rescan() {
    L.scanning = true; renderSide();
    try { const r = await invoke('library_scan'); take(r); } catch (e) { toast(String(e)); }
    L.scanning = false;
    if (!root.hidden) render(); else renderSide();
  }
  function take(r) { L.items = r.items || []; L.store = r.store || {}; L.summary = r.summary || {}; L._mem = null; }

  H.startLibrary = init => boot(init);
  async function boot(init) {
    L.started = true;
    L.home = init.home || '';
    try { take(await invoke('library_cached')); } catch (_) {}
    L.scanning = true;
    show();
    rescan();
    // New photos (from the phone, a game capture) while it's open: a quiet look every two minutes.
    setInterval(() => { if (!L.scanning && document.visibilityState === 'visible') rescan(); }, 120000);
  }
})();
