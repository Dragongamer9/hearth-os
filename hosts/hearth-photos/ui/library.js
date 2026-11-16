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
    const a = aiOf(i.path);
    const who = a ? a.faces.map(f => personName(f.g)).filter(Boolean).join(' ') : '';
    const hay = kindWords(i) + ' ' + fmt(d, { month: 'long', year: 'numeric' }).toLowerCase() + ' ' + fmt(d, { weekday: 'long' }).toLowerCase() + ' ' + (a ? a.tags.join(' ') : '').toLowerCase() + ' ' + who.toLowerCase();
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
      case 'screenshots': v = v.filter(i => i.kind === 'screenshot'); break;
      case 'ai': v = v.filter(i => i.kind === 'ai'); break;
      case 'wallpapers': v = v.filter(i => isWallpaper(i.path)); break;
      case 'person': v = v.filter(i => { const a = aiOf(i.path); return a && a.faces.some(f => f.g === L.person); }); break;
      case 'spot': { const set = new Set(L.spot || []); v = v.filter(i => set.has(i.path)); break; }
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
      <button class="lh" data-f data-view="collections" title="All your albums (Collections)">Your albums</button>
      <nav id="lalbums"></nav>
      <span class="sp"></span>
      <div id="phonebox"></div>
    </aside>
    <main id="lmain">
      <header id="padtop"></header>
      <header id="ltop">
        <label class="search"><svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="7"/><path d="m20 20-4-4"/></svg>
          <input id="lq" data-f placeholder='Search: "sunset at the lake", "ROG Ally screenshot"…' spellcheck="false" autocomplete="off">
          <span class="here">Searched on this PC</span></label>
        <span class="sp"></span>
        <button class="btn" data-f data-lact="import">Import</button>
        <button class="btn" data-f data-lact="select" id="lselect">Select</button>
        <button class="btn" data-f data-lact="slideshow">Slideshow</button>
        <label class="size">Size <input type="range" id="lsize" data-f min="110" max="340" step="10"></label>
        <button class="ib" data-f data-lact="settings" title="Photos settings"><svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/></svg></button>
      </header>
      <div id="lselbar" hidden><b id="lselcount"></b><span class="sp"></span>
        <button class="btn" data-f data-lact="selFav">♥ Favorite</button><button class="btn" data-f data-lact="selAlbum">Add to album</button>
        <button class="btn" data-f data-lact="selTrash">Trash</button><button class="btn primary" data-f data-lact="select">Done</button></div>
      <div id="lbody" class="scroll" tabindex="-1"><div id="lcontent"></div></div>
      <footer id="padfoot"><span><b>A</b>Open</span><span><b>X</b>Slideshow</span><span><b>Y</b>Search</span><span><b>☰</b>Options</span><span><b>B</b>Back</span><span class="sp"></span><small>Shows like this when a controller is in use · keyboard and mouse get the library</small></footer>
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
      people: facesOn() ? groupsShown().length || null : null, places: placeCount() || null, phone: L.items.filter(i => i.src === 'phone').length, folders: null,
      trash: L.summary.trash || null,
    };
  }
  function placeCount() { const s = new Set(); for (const i of L.items) if (i.gps) s.add(`${i.gps[0].toFixed(1)},${i.gps[1].toFixed(1)}`); return s.size; }

  function renderSide() {
    const c = counts();
    $('#lnav', root).innerHTML = NAV.map(([id, label]) =>
      `<button class="li${L.view === id || (id === 'folders' && L.view === 'folder') ? ' on' : ''}" data-f data-view="${id}"><span>${label}</span><i>${id === 'all' || c[id] ? n(c[id]) : ''}</i></button>`).join('');
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
    root.classList.toggle('pad', !!L.padLook);
    if (L.padLook) { drawPad(); return; }
    renderSide();
    const width = Math.max(300, body.clientWidth - 60);
    const v = shown();
    let h = '';
    if (L.view === 'memories') { renderMemoriesPage(); return; }
    if (L.view === 'folders' || L.view === 'people' || L.view === 'places' || L.view === 'collections') { renderCollections(); return; }
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
  // ---------------------------------------------------------------- Collections (photos-3): albums, people, places, folders
  const TABS = [['collections', 'Albums'], ['people', 'People'], ['places', 'Places'], ['folders', 'Folders']];
  const isWallpaper = p => !!L.wallDir && p.startsWith(L.wallDir + '/');
  const plural = (k, one, many) => `${n(k)} ${k === 1 ? one : many}`;
  function albumSub(list) {
    const v = list.filter(i => i.kind === 'video').length, ph = list.length - v;
    return [ph ? plural(ph, 'photo', 'photos') : '', v ? plural(v, 'video', 'videos') : ''].filter(Boolean).join(' · ') || 'Empty';
  }
  function coll(key, title, sub, items, attrs) {
    const c = pickCover(items);
    return `<button class="coll" data-f ${attrs}><span class="cv">${c ? `<img alt="" loading="lazy" src="${url('thumb', c.path, `?s=480&v=${c.mtime}`)}">` : ''}</span><b>${esc(title)}</b><small>${esc(sub)}</small></button>`;
  }
  function autoAlbums() {
    const f = favs(), by = k => L.items.filter(k);
    const caps = by(i => i.kind === 'capture'), phone = by(i => i.src === 'phone'), shots = by(i => i.kind === 'screenshot');
    const where = [...new Set(caps.map(i => i.src === 'steam' ? 'Steam' : 'Game Mode'))];
    if (L.items.some(i => i.kind === 'capture' && i.src === 'gamemode') && shots.length) where.push('Desktop');
    const shotFrom = [...new Set(shots.map(i => /ally/i.test(i.path) ? 'your Ally' : 'this PC'))];
    return [
      ['captures', 'Game captures', [n(caps.length), where.join(', ').replace(/, ([^,]*)$/, ' and $1')].filter(Boolean).join(' · '), caps],
      ['phone', 'From your phone', `${n(phone.length)} · ${phoneName()}`, phone],
      ['screenshots', 'Screenshots', [n(shots.length), shotFrom.join(' and ')].filter(Boolean).join(' · '), shots],
      ['videos', 'Videos', n(by(i => i.kind === 'video').length), by(i => i.kind === 'video')],
      ['favorites', 'Favorites', n(by(i => f.has(i.path)).length), by(i => f.has(i.path))],
      ['ai', 'Made by your AI', `${n(by(i => i.kind === 'ai').length)} · ComfyUI pictures`, by(i => i.kind === 'ai')],
    ];
  }
  function albumCards() {
    const byPath = new Map(L.items.map(i => [i.path, i]));
    let h = albums().map(a => { const list = (a.items || []).map(p => byPath.get(p)).filter(Boolean); return coll(a.id, a.name, albumSub(list), list, `data-album="${esc(a.id)}"`); }).join('');
    const walls = L.items.filter(i => isWallpaper(i.path));
    h += coll('wallpapers', 'Wallpapers', `${n(walls.length)} · used by Hearth's wallpaper picker`, walls, 'data-view="wallpapers"');
    return h + '<button class="coll add" data-f data-lact="newAlbum"><span class="cv">+ New album</span></button>';
  }
  function renderCollections() {
    const tab = L.view;
    let h = `<header class="ctop"><h1>Collections</h1><nav class="tabs">${TABS.map(([id, label]) => `<button class="tab${tab === id ? ' on' : ''}" data-f data-view="${id}">${label}</button>`).join('')}</nav><span class="sp"></span><button class="btn primary" data-f data-lact="newAlbum">+ New album</button></header>`;
    if (tab === 'collections') {
      h += `<section><h2>Kept up to date by themselves <small>photos go in as they arrive</small></h2><div class="colls">${autoAlbums().map(([v, t, sub, list]) => coll(v, t, sub, list, `data-view="${v}"`)).join('')}</div></section>`;
      h += `<section><h2>Your albums</h2><div class="colls">${albumCards()}</div></section>`;
      h += `<div class="pair"><section class="cbox" id="peopleBox">${peopleBox(6)}</section><section class="cbox" id="placesBox">${placesBox()}</section></div>`;
    } else if (tab === 'people') h += `<section class="cbox big">${peopleBox(0)}</section>`;
    else if (tab === 'places') h += `<section class="cbox big" id="placesBox">${placesBox(true)}</section>`;
    else h += foldersHtml();
    content.innerHTML = h;
    renderDates(true);
    if (tab === 'collections' || tab === 'places') drawPlaces(tab === 'places');
  }
  // ---------------------------------------------------------------- faces and what's in a photo (worked out on this PC)
  const aiOf = p => L.ai && L.ai.photos ? L.ai.photos[p] : null;
  const facesOn = () => L.store.faces !== false;
  const tagsOn = () => L.store.tags !== false;
  const personName = g => (L.store.people || {})[g] || '';
  /** Groups with at least two photos (one-offs are strangers in the background), biggest first, named ones first. */
  function groupsShown() {
    if (!L.ai || !L.ai.groups || !facesOn()) return [];
    const count = new Map(), cover = new Map(), alive = new Set(L.items.map(i => i.path));
    for (const [p, a] of Object.entries(L.ai.photos || {})) {
      if (!alive.has(p)) continue;
      for (const f of a.faces || []) { count.set(f.g, (count.get(f.g) || 0) + 1); if (!cover.has(f.g) || f.box[2] > cover.get(f.g).box[2]) cover.set(f.g, { path: p, box: f.box }); }
    }
    return L.ai.groups.map(g => ({ id: g.id, n: count.get(g.id) || 0, cover: cover.get(g.id), name: personName(g.id) }))
      .filter(g => g.n >= 2 || g.name).sort((a, b) => (!!b.name - !!a.name) || b.n - a.n);
  }
  /** A face as a circle: the photo's small copy, zoomed and moved so the face fills it. */
  function faceCircle(c, size) {
    if (!c) return `<span class="face" style="width:${size}px;height:${size}px"></span>`;
    const [x, y, w, h] = c.box, s = Math.max(w, h) * 1.5, z = 100 / s;
    const cx = (x + w / 2) * 100, cy = (y + h / 2) * 100;
    return `<span class="face" style="width:${size}px;height:${size}px"><img alt="" loading="lazy" src="${url('thumb', c.path, '?s=512')}" style="width:${z}%;height:${z}%;left:${50 - cx * z / 100}%;top:${50 - cy * z / 100}%"></span>`;
  }
  function peopleBox(max) {
    const head = `<div class="bh"><h2>People <small>faces grouped on this PC · only you see the names</small></h2>${max && groupsShown().length ? `<button class="link" data-f data-view="people">See all ${n(groupsShown().length)}</button>` : ''}</div>`;
    if (!facesOn()) return head + `<p class="muted">Faces are off. <button class="link" data-f data-lact="facesOn">Turn them on</button> to group photos by who's in them; it's all worked out on this PC.</p>`;
    const st = L.aiStatus || {};
    if (!st.ready) {
      const pr = st.progress;
      return head + (pr && pr.kind === 'setup' ? `<p class="muted">${esc(pr.text)}</p>` : `<p class="muted">Photos can group your photos by the faces in them and find what's in each one, all on this PC. It needs a one-time download (OpenCV and three small models, about 70 MB).</p><button class="btn primary" data-f data-lact="aiSetup">Set it up</button>`);
    }
    const gs = groupsShown(), list = max ? gs.slice(0, max) : gs;
    const pr = L.ai && L.ai.progress;
    return head + `<div class="people">${list.map(g => `<button class="person" data-f data-person="${esc(g.id)}">${faceCircle(g.cover, max ? 88 : 120)}<b>${esc(g.name || 'Add a name')}</b><small>${n(g.n)}</small></button>`).join('')}</div>`
      + (gs.length ? '' : `<p class="muted">${pr ? esc(pr.text) : 'No faces grouped yet.'}</p>`)
      + `<p class="muted small">Turn faces off in Settings › Photos and every group is forgotten.${pr && gs.length ? ' · ' + esc(pr.text) : ''}</p>`;
  }
  async function loadAi() {
    try { L.aiStatus = await invoke('ai_status'); } catch (_) { L.aiStatus = {}; }
    try { L.ai = await invoke('ai_results'); } catch (_) { L.ai = null; }
  }
  /** While open: look at new photos now and then; the page picks up what's found. */
  async function aiTick() {
    await loadAi();
    if (L.aiStatus && L.aiStatus.ready && (facesOn() || tagsOn()) && !(L.ai && L.ai.running)) { try { await invoke('ai_run'); } catch (_) {} }
    if (!root.hidden && ['people', 'collections'].includes(L.view)) render(); else if (!root.hidden) renderSide();
  }
  async function namePerson(id) {
    const g = groupsShown().find(x => x.id === id);
    const v = await dialog(`<h2>Who is this?</h2><div style="display:flex;justify-content:center;margin:8px 0 14px">${faceCircle(g && g.cover, 120)}</div><label class="field">Name (only you see it) <input id="pname" data-f autofocus value="${esc(personName(id))}" placeholder="Chris"></label>`,
      [{ label: 'Cancel', value: null }, { label: 'See their photos', value: 'see' }, { label: 'Save the name', kind: 'primary', value: 'save' }]);
    if (v === 'save') await change({ op: 'person.name', id, name: ($('#pname') || {}).value || '' });
    if (v === 'see') { L.view = 'person'; L.person = id; render(); body.scrollTop = 0; }
  }
  // The photo page's People and Found in it (app.js renderInfo).
  window.HPFaces = {
    html(p) {
      if (!facesOn()) return '';
      const a = aiOf(p.path);
      const faces = a ? a.faces : [];
      return `<div class="isec"><h3>People</h3><div class="chips people-row">${faces.map(f => `<button class="pface" data-f data-person="${esc(f.g)}" title="${esc(personName(f.g) || 'Name this face')}">${faceCircle({ path: p.path, box: f.box }, 48)}</button>`).join('')}`
        + `<button class="chip add" data-f ${faces.length ? `data-person="${esc((faces.find(f => !personName(f.g)) || faces[0]).g)}"` : 'data-lact="aiExplain"'}>+ Name a face</button></div>`
        + (a ? '' : `<p class="dsub">${L.aiStatus && L.aiStatus.ready ? 'Not looked at yet.' : 'Set up faces in Collections › People.'}</p>`) + '</div>';
    },
  };
  window.HPTags = {
    html(p) {
      if (!tagsOn()) return '';
      const a = aiOf(p.path);
      if (!a) return `<div class="isec"><h3>Found in it</h3><p class="dsub">${L.aiStatus && L.aiStatus.ready ? 'Not looked at yet: it\'s worked out in the background.' : 'Set it up in Collections › People: it\'s all worked out on this PC.'}</p></div>`;
      return `<div class="isec"><h3>Found in it</h3><div class="chips">${a.tags.map(t => `<button class="chip" data-f data-tag="${esc(t)}">${esc(t)}</button>`).join('')}</div>`
        + `<p class="dsub small">Worked out on this PC${a.by === 'gpu' ? ' (your graphics card)' : ''}, so you can search for it</p></div>`;
    },
  };
  // From the photo page: a face opens its name, a tag searches for it.
  document.addEventListener('click', e => {
    const f = e.target.closest('#info [data-person]');
    if (f) { e.stopPropagation(); namePerson(f.dataset.person); return; }
    const t = e.target.closest('#info [data-tag]');
    if (t) { e.stopPropagation(); L.q = t.dataset.tag; q.value = L.q; L.view = 'all'; show(); body.scrollTop = 0; return; }
    if (e.target.closest('#info [data-lact="aiExplain"]')) { e.stopPropagation(); L.view = 'people'; show(); }
  }, true);

  /** Photos' settings: faces and Found in it, on this PC. */
  async function settingsDialog() {
    const v = await dialog(`<h2>Photos settings</h2>
      <label class="sw"><input type="checkbox" id="setFaces" ${facesOn() ? 'checked' : ''}> <span><b>Faces</b><small>Group photos by who's in them. Off forgets every group and name.</small></span></label>
      <label class="sw"><input type="checkbox" id="setTags" ${tagsOn() ? 'checked' : ''}> <span><b>Found in it</b><small>What's in each photo, so you can search for it.</small></span></label>
      <p class="muted small">Both are worked out on this PC${L.aiStatus && L.aiStatus.vision ? ` (your graphics card, with ${esc(L.aiStatus.vision)} in Ollama)` : ''}. Your photos never leave it.</p>`,
      [{ label: 'Cancel', value: null }, { label: 'Save', kind: 'primary', value: 1 }],
      d => { d.querySelectorAll('input').forEach(i => { i._v = i.checked; i.addEventListener('change', () => { i._v = i.checked; }); }); L._dlg = d; });
    if (!v || !L._dlg) return;
    const faces = L._dlg.querySelector('#setFaces')._v, tags = L._dlg.querySelector('#setTags')._v;
    if (faces !== facesOn()) await setFaces(faces);
    if (tags !== tagsOn()) await change({ op: 'set', key: 'tags', value: tags });
  }
  async function setFaces(on) {
    if (!on) {
      const sure = await dialog('<h2>Turn faces off?</h2><p>Every face group and every name you gave is forgotten. You can turn faces on again later; the photos are looked at again then.</p>',
        [{ label: 'Keep faces', value: 0, focus: true }, { label: 'Turn off and forget', kind: 'danger', value: 1 }]);
      if (sure !== 1) return;
      try { await invoke('ai_faces', { on: false }); } catch (e) { toast(String(e)); }
      await change({ op: 'people.forget' });
    } else { try { await invoke('ai_faces', { on: true }); } catch (_) {} }
    await change({ op: 'set', key: 'faces', value: on });
    toast(on ? 'Faces on: your photos are looked at again' : 'Faces off: every group was forgotten');
    aiTick();
  }

  function placesBox(big) {
    return `<div class="bh"><h2>Places <small>where your photos were taken</small></h2>${big ? '' : '<button class="link" data-f data-view="places">Open the map</button>'}</div><div class="mapwrap" id="placesMap"></div>`;
  }
  function drawPlaces(big) {
    const el = $('#placesMap', root);
    if (!el || !window.HPPlace) return;
    const withGps = L.items.filter(i => i.gps);
    if (!withGps.length) { el.innerHTML = '<p class="muted pad">No photos with a place yet. Phones save where a photo was taken when their camera is allowed to.</p>'; return; }
    const w = Math.max(300, el.clientWidth), h = big ? Math.max(360, body.clientHeight - 180) : 360;
    L.mapZoom = big && L.mapZoom ? L.mapZoom : HPPlace.fitZoom(withGps.map(i => i.gps), w, h);
    const groups = HPPlace.clusters(withGps, L.mapZoom, 70);
    L.spots = groups;
    const marks = groups.map((g, k) => [g.lat, g.lon, `<button class="spot" data-f data-spot="${k}"><img alt="" loading="lazy" src="${url('thumb', pickCover(g.items).path, '?s=160')}"><b>${n(g.items.length)}</b></button>`]);
    el.innerHTML = HPPlace.map(marks, w, h, { zoom: L.mapZoom }).html + (big ? '<div class="zoom"><button class="btn" data-f data-lact="mapIn">+</button><button class="btn" data-f data-lact="mapOut">−</button></div>' : '');
  }
  function foldersHtml() {
    const by = new Map();
    for (const i of L.items) { const d = dirOf(i.path); (by.get(d) || by.set(d, []).get(d)).push(i); }
    const home = (L.home || '').replace(/\/$/, '');
    const list = [...by.entries()].sort((a, b) => a[0].localeCompare(b[0]));
    return `<section class="folders"><h2>Folders <small>${list.length} folders</small></h2><div class="grid">${list.map(([d, v]) =>
      `<button class="fold" data-f data-folder="${esc(d)}"><span class="sw"><img alt="" loading="lazy" src="${url('thumb', v[0].path, '?s=200')}"></span><b>${esc(d.split('/').pop())}</b><small>${esc(home && d.startsWith(home) ? '~' + d.slice(home.length) : d)} · ${v.length}</small></button>`).join('')}</div></section>`;
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
    try { L.store = await invoke('library_change', { change: c }); L._mem = null; render(); if (!$('#app').hidden && H.renderInfo) { H.renderInfo(); H.renderTitle && H.renderTitle(); } return true; }
    catch (e) { toast(String(e)); return false; }
  }
  async function chooseAlbum(paths) {
    const list = albums();
    const pick = await dialog(`<h2>Add ${paths.length === 1 ? 'it' : paths.length + ' items'} to an album</h2>
      <div class="albumpick">${list.map((a, k) => `<button class="btn" data-f data-b2="${k}">${esc(a.name)}</button>`).join('')}<button class="btn" data-f data-b2="walls">Wallpapers</button></div>
      <label class="field">Or a new album <input id="newAlbumName" data-f placeholder="Lake trip 2026"></label>`,
      [{ label: 'Cancel', value: null }, { label: 'Make the new album', kind: 'primary', value: 'new' }],
      (v, done) => v.addEventListener('click', e => { const b = e.target.closest('[data-b2]'); if (b) done(b.dataset.b2 === 'walls' ? 'walls' : list[+b.dataset.b2].id); }));
    if (!pick) return;
    if (pick === 'walls') { try { toast(await invoke('add_to_wallpapers', { paths })); rescan(); } catch (e) { toast(String(e)); } return; }
    if (pick === 'new') { const name = ($('#newAlbumName') || {}).value || ''; if (name.trim()) await change({ op: 'album.new', name, paths }); return; }
    await change({ op: 'album.add', id: pick, paths });
    toast('Added to the album');
  }
  window.HPLib = { chooseAlbum, change, favs, albums, get store() { return L.store; }, show, hide, visible: () => !root.hidden, key, aiOf, personName, facesOn, tagsOn, faceCircle, namePerson, settings: () => settingsDialog(), items: () => L.items, phoneName, pad };

  /** A controller in the library: D-pad / stick moves, A opens, B leaves search, X slideshow, Y search, Start Select. */
  // ---------------------------------------------------------------- the controller look (photos-4)
  const PTABS = [['foryou', 'For you'], ['all', 'All photos'], ['albums', 'Albums'], ['people', 'People'], ['places', 'Places'], ['videos', 'Videos']];
  function setPadLook(on) {
    if (!!L.padLook === on) return;
    L.padLook = on;
    if (on && !L.ptab) L.ptab = 'foryou';
    render();
    if (on) setTimeout(() => { const f = content.querySelector('[data-f]'); if (f) { f.focus(); f.classList.add('kf'); } }, 20);
  }
  // Keyboard or mouse: the library look again (as the new Settings does).
  document.addEventListener('mousemove', e => { if (L.padLook && (Math.abs(e.movementX) + Math.abs(e.movementY) > 2) && Date.now() - (L.padAt || 0) > 1500) setPadLook(false); });
  document.addEventListener('keydown', e => { if (L.padLook && !root.hidden && e.isTrusted && Date.now() - (L.padAt || 0) > 1500 && !e.key.startsWith('Arrow') && e.key !== 'Enter' && e.key !== 'Escape') setPadLook(false); }, true);
  function bigTile(attrs, cover, label) {
    return `<button class="ptile" data-f ${attrs}>${cover ? `<img alt="" loading="lazy" src="${url('thumb', cover.path, `?s=480&v=${cover.mtime}`)}">` : ''}<span class="shade"></span><b>${esc(label)}</b></button>`;
  }
  const tileName = i => i.kind === 'video' ? `Video · ${dur(i.secs)}` : i.kind === 'capture' && i.game ? i.game : i.kind === 'screenshot' ? 'Screenshot' : i.name.replace(/\.[^.]+$/, '');
  function drawPad() {
    $('#padtop', root).innerHTML = `<span class="pk">LB</span>${PTABS.map(([id, l]) => `<button class="ptab${L.ptab === id ? ' on' : ''}" data-ptab="${id}">${l}</button>`).join('')}<span class="pk">RB</span><span class="sp"></span><span class="pcount">${n(L.items.length)} photos</span>`;
    const t = L.ptab;
    let h = '';
    if (t === 'foryou') {
      const m = memories()[0] || (L.items.length ? { kind: 'All photos', title: 'Your photos', items: L.items, cover: pickCover(L.items) } : null);
      if (m) {
        const v = m.items.filter(i => i.kind === 'video').length, ph = m.items.length - v, when = fmt(asDate(m.items[0].taken), { month: 'long', year: 'numeric' });
        h += `<section class="hero" data-f data-mem="0" tabindex="0">${m.cover ? `<img alt="" src="${url('thumb', m.cover.path, `?s=1024&v=${m.cover.mtime}`)}">` : ''}<span class="shade"></span>
          <div class="ht"><span class="ck">${m === memories()[0] ? 'A memory' : 'Your photos'}</span><h1>${esc(m.title.replace(/ · \d.*$/, ''))}</h1><p>${esc([ph ? plural(ph, 'photo', 'photos') : '', v ? plural(v, 'video', 'videos') : ''].filter(Boolean).join(' and '))} · ${esc(when)}</p>
          <div class="hb"><button class="btn primary" data-f data-mem="0"><b class="pb">A</b> Play as a slideshow</button><button class="btn" data-f data-lact="browseMem"><b class="pb">X</b> Browse them</button></div></div></section>`;
      }
      const recent = L.items.filter(i => daysAgo(asDate(i.taken)) <= 1).slice(0, 12);
      if (recent.length) h += `<section><h2>Today and yesterday</h2><div class="prow">${recent.map(i => bigTile(`data-path="${esc(i.path)}" data-ptile="1"`, i, tileName(i))).join('')}</div></section>`;
      h += `<section><h2>Albums</h2><div class="prow">${padAlbums()}</div></section>`;
    } else if (t === 'albums') h += `<section><h2>Albums</h2><div class="pgrid">${padAlbums()}</div></section>`;
    else if (t === 'people') h += `<section class="cbox big">${peopleBox(0)}</section>`;
    else if (t === 'places') h += `<section class="cbox big" id="placesBox">${placesBox(true)}</section>`;
    else {
      const list = t === 'videos' ? L.items.filter(i => i.kind === 'video') : L.items;
      for (const g of groups(list)) h += `<section><h2>${esc(g.title)} <small>${esc(g.sub)}</small></h2><div class="pgrid">${g.items.map(i => bigTile(`data-path="${esc(i.path)}" data-ptile="1"`, i, tileName(i))).join('')}</div></section>`;
      groupsNow = groups(list);
    }
    content.innerHTML = h;
    if (t === 'places') drawPlaces(true);
    if (t === 'foryou') groupsNow = groups(L.items);
  }
  function padAlbums() {
    const byPath = new Map(L.items.map(i => [i.path, i]));
    const autos = autoAlbums().filter(a => a[3].length).map(([v, title, , list]) => bigTile(`data-view="${v}"`, pickCover(list), title));
    const mine = albums().map(a => { const list = (a.items || []).map(p => byPath.get(p)).filter(Boolean); return bigTile(`data-album="${esc(a.id)}"`, pickCover(list), a.name); });
    const walls = L.items.filter(i => isWallpaper(i.path));
    return [...autos.slice(0, 2), ...mine, walls.length ? bigTile('data-view="wallpapers"', pickCover(walls), 'Wallpapers') : '', ...autos.slice(2)].join('');
  }
  function padOptions() {
    H.openMenu(null, [
      { label: 'Search', run: () => { setPadLook(false); q.focus(); } },
      { label: 'Import a folder…', run: importFolder },
      { label: 'Select photos', run: () => { setPadLook(false); toggleSelect(); } },
      { label: 'New album…', run: newAlbum },
      { label: 'Photos settings…', run: settingsDialog },
      { label: 'Show the library look', run: () => setPadLook(false) },
    ]);
  }

  /** A controller in the library: the controller look comes up; D-pad moves, A opens, B back, X slideshow, Y search,
   *  LB / RB tabs, ☰ options. */
  function pad(k) {
    L.padAt = Date.now();
    if (!L.padLook) { setPadLook(true); return; }
    const dir = { up: 'up', down: 'down', left: 'left', right: 'right' }[k];
    if (dir) { if (!root.contains(document.activeElement) || document.activeElement === body) { const f = content.querySelector('[data-f]'); if (f) { f.focus(); f.classList.add('kf'); } return; } H.move(dir, root); const a = document.activeElement; if (a) a.scrollIntoView({ block: 'nearest' }); if (a && a.dataset.path) L.focus = a.dataset.path; return; }
    if (k === 'a') { const a = document.activeElement; if (a && root.contains(a) && a.click) a.click(); return; }
    if (k === 'b') { if (L.view !== 'all' && L.ptab !== 'foryou') { L.view = 'all'; } if (L.ptab !== 'foryou') { L.ptab = 'foryou'; render(); } return; }
    if (k === 'x') { if (L.ptab === 'foryou') acts.browseMem(); else acts.slideshow(); return; }
    if (k === 'y') { setPadLook(false); q.focus(); return; }
    if (k === 'start') { padOptions(); return; }
    if (k === 'lb' || k === 'rb') { const i = PTABS.findIndex(x => x[0] === L.ptab); L.ptab = PTABS[(Math.max(0, i) + (k === 'rb' ? 1 : PTABS.length - 1)) % PTABS.length][0]; render(); const f = content.querySelector('[data-f]'); if (f) { f.focus(); f.classList.add('kf'); } }
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
    browseMem: () => { const m = memories()[0]; if (m) { L.spot = m.items.map(i => i.path); L.view = 'spot'; L.ptab = 'all'; setPadLook(false); render(); body.scrollTop = 0; } },
    aiSetup: async () => { try { await invoke('ai_setup'); toast('Setting up (once)…'); } catch (e) { toast(String(e)); } const t = setInterval(async () => { await loadAi(); render(); if (L.aiStatus && (L.aiStatus.ready || !(L.aiStatus.progress))) { clearInterval(t); aiTick(); } }, 3000); },
    facesOn: () => setFaces(true),
    settings: settingsDialog,
    mapIn: () => { L.mapZoom = Math.min(16, (L.mapZoom || 4) + 1); drawPlaces(true); },
    mapOut: () => { L.mapZoom = Math.max(2, (L.mapZoom || 4) - 1); drawPlaces(true); },
  };
  root.addEventListener('click', e => {
    const pe = e.target.closest('[data-person]');
    if (pe) { namePerson(pe.dataset.person); return; }
    const pt = e.target.closest('[data-ptab]');
    if (pt) { L.ptab = pt.dataset.ptab; render(); return; }
    if (L.padLook && e.target.closest('[data-view],[data-album]') && !e.target.closest('[data-ptile]')) { const el = e.target.closest('[data-view],[data-album]'); if (el.dataset.album) { L.view = 'album'; L.album = el.dataset.album; } else L.view = el.dataset.view; L.ptab = 'all'; setPadLook(false); render(); body.scrollTop = 0; return; }
    const sp = e.target.closest('[data-spot]');
    if (sp) { const g = (L.spots || [])[+sp.dataset.spot]; if (g) { L.spot = g.items.map(i => i.path); L.view = 'spot'; render(); body.scrollTop = 0; } return; }
    const t = e.target.closest('[data-lact],[data-view],[data-album],.tile,.ptile,[data-mem],[data-folder],[data-jump]');
    if (!t) return;
    if (t.dataset.lact) { acts[t.dataset.lact](); return; }
    if (t.dataset.view) { setView(t.dataset.view); return; }
    if (t.dataset.album) { L.view = 'album'; L.album = t.dataset.album; render(); body.scrollTop = 0; return; }
    if (t.dataset.folder) { L.view = 'folder'; L.folder = t.dataset.folder; render(); body.scrollTop = 0; return; }
    if (t.dataset.mem) { const m = (L.view === 'memories' ? memories() : memories().slice(0, 4))[+t.dataset.mem]; if (m) { const v = m.items.filter(i => i.kind !== 'video'); if (v.length) open(v[0].path, { list: v, slideshow: true }); } return; }
    if (t.dataset.jump) { jump(t.dataset.jump, t.dataset.jumpyear); return; }
    if (t.classList.contains('ptile') && t.dataset.path) { open(t.dataset.path, { list: L.items.filter(i => i.kind !== 'video') }); return; }
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
  root.addEventListener('keydown', key);
  /** Keys while the library is showing (app.js sends them here when nothing in the library has focus). */
  function key(e) {
    if (e.target === q && e.key === 'Escape') { q.value = ''; L.q = ''; render(); e.stopPropagation(); return; }
    if (e.target.tagName === 'INPUT' && e.target.type !== 'range') { if (e.key === 'ArrowDown') { e.preventDefault(); firstTile(); } return; }
    const d = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' }[e.key];
    if (d && !root.contains(document.activeElement)) { e.preventDefault(); firstTile(); return; }
    if (d) { e.preventDefault(); e.stopPropagation(); H.move(d, root); const a = document.activeElement; if (a && a.classList.contains('tile')) L.focus = a.dataset.path; return; }
    if (e.key === 'Delete' && document.activeElement?.classList.contains('tile')) { L.picked = new Set([document.activeElement.dataset.path]); trashPicked(); }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') { e.preventDefault(); q.focus(); }
    if (e.key === 'Enter' && !root.contains(document.activeElement)) { e.preventDefault(); firstTile(); }
  }
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
    if (!root.hidden) { render(); if (!root.contains(document.activeElement)) { const t = content.querySelector('.tile'); if (t) t.focus({ preventScroll: true }); } } else renderSide();
  }
  function take(r) { L.items = r.items || []; L.store = r.store || {}; L.summary = r.summary || {}; L._mem = null; }

  H.startLibrary = init => boot(init);
  async function boot(init) {
    L.started = true;
    L.home = init.home || '';
    if (init.view === 'pad') { L.padLook = true; L.ptab = 'foryou'; L.view = 'all'; L.padAt = Date.now() + 60000; }
    else if (init.view) L.view = init.view;
    L.wallDir = init.pictures ? init.pictures.replace(/\/$/, '') + '/Wallpapers' : '';
    try { take(await invoke('library_cached')); } catch (_) {}
    loadAi().then(() => { if (!root.hidden) render(); });
    setTimeout(aiTick, 15000);
    setInterval(() => { if (document.visibilityState === 'visible') aiTick(); }, 60000);
    L.scanning = true;
    show();
    rescan();
    // New photos (from the phone, a game capture) while it's open: a quiet look every two minutes.
    setInterval(() => { if (!L.scanning && document.visibilityState === 'visible') rescan(); }, 120000);
  }
})();
