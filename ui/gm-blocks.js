// Hearth's Game Mode made of blocks (docs/mockups/checklists/build-anything.md board 6): Home, Library and Quick Access
// built from the same blocks as the desktop (hearth-layout's screen format: a grid, blocks with x, y, w, h, what they
// show and their look), and edited with the controller:
//   A pick up / put down · X its look · Y Describe it · B done · LB / RB Home, Library, Quick Access ·
//   the left stick (or the D-pad) moves the picked block, or picks the next one · the right stick resizes it · + adds one
// Describe it: the on-screen keyboard, or Say it (your voice, heard by an AI engine of yours), then Make it (your AI
// changes the page: hearth-core gmblocks.describe → desktop/shell/hearth-gm-blocks → hearth-layout). Undo.
// The 15 Home layouts are ready-made starting points. Settings › You › Appearance › Home layout › "Your own (blocks)".
// Kept like the rest of Game Mode's look: localStorage and the uiprefs file (merged, atomic, a .bak kept; one key,
// nimbus.gmblocks), and only when you change something. Loaded after index.html; its hooks there check it's here, so
// if this file can't load Game Mode is exactly as before.
"use strict";

const GMB_PAGES = [['home', 'Home'], ['library', 'Library'], ['quick', 'Quick Access']];
const GMB_NAMES = {hero: 'Hero tile', tilerow: 'Tile row', iconrow: 'Icon row', widget: 'Widget', friends: 'Friends', downloads: 'Downloads',
  clock: 'Clock', text: 'Words', play: 'Play', list: 'List', qa: 'Quick Access'};
const GMB_QA = {perf: ['Performance', 'perf'], audio: ['Sound', 'quick'], display: ['Display', 'quick'], power: ['Power', 'quick'],
  wifi: ['Wi-Fi and Bluetooth', 'quick'], friends: ['Friends', 'friends'], resume: ['Quick Resume', 'quick'], music: ['Now playing', 'now']};
const GMB_LOOKS = [{}, {colour: '#2a1f3d', corners: 16}, {colour: '#1f3a2e', corners: 8}, {colour: '#3a2a12', corners: 24}, {colour: '#12263a', corners: 4}, {colour: '#00000000', corners: 0}];
const B = (id, block, x, y, w, h, more = {}) => Object.assign({id, block, x, y, w, h}, more);
const GMB_DEFAULT = {
  home: [B('hero', 'hero', 0, 0, 9, 6), B('recent', 'tilerow', 9, 0, 10, 3, {shows: 'recent', skip: 1, count: 2}),
    B('next', 'tilerow', 9, 3, 5, 3, {shows: 'recent', skip: 3, count: 1}), B('widget', 'widget', 14, 3, 5, 3),
    B('games', 'tilerow', 0, 7, 13, 3, {shows: 'games', skip: 4, count: 4})],
  library: [B('az', 'list', 0, 0, 7, 14), B('hero', 'hero', 7, 0, 17, 6), B('all', 'tilerow', 7, 7, 17, 4, {shows: 'games'})],
  quick: [B('perf', 'qa', 0, 0, 12, 2, {shows: 'perf'}), B('resume', 'qa', 12, 0, 12, 2, {shows: 'resume'}), B('audio', 'qa', 0, 2, 12, 2, {shows: 'audio'}),
    B('display', 'qa', 12, 2, 12, 2, {shows: 'display'}), B('friends', 'qa', 0, 4, 12, 2, {shows: 'friends'}), B('power', 'qa', 12, 4, 12, 2, {shows: 'power'})],
};
// The 15 Home layouts as starting points (the same idea in blocks).
const GMB_FROM = {
  shelf: [B('hero', 'hero', 0, 0, 24, 6), B('recent', 'tilerow', 0, 7, 24, 3, {shows: 'recent'}), B('games', 'tilerow', 0, 10, 24, 3, {shows: 'games'})],
  tiles: [B('hero', 'hero', 0, 0, 10, 6), B('recent', 'tilerow', 10, 0, 14, 3, {shows: 'recent', skip: 1, count: 3}), B('widget', 'widget', 10, 3, 7, 3),
    B('friends', 'friends', 17, 3, 7, 3), B('games', 'tilerow', 0, 7, 24, 3, {shows: 'games'})],
  spotlight: [B('row', 'iconrow', 0, 0, 16, 3), B('play', 'play', 0, 4, 14, 4), B('widget', 'widget', 0, 9, 10, 3)],
  'x-carousel': [B('row', 'iconrow', 2, 3, 20, 4), B('play', 'play', 6, 8, 12, 4)],
  'x-rows': [B('hero', 'hero', 0, 0, 24, 5), B('recent', 'tilerow', 0, 6, 24, 3, {shows: 'recent'}), B('games', 'tilerow', 0, 10, 24, 3, {shows: 'games'})],
  'x-list': [B('az', 'list', 0, 0, 8, 14), B('hero', 'hero', 8, 0, 16, 8), B('play', 'play', 8, 9, 12, 4)],
  'x-wall': [B('w1', 'tilerow', 0, 0, 24, 4, {shows: 'games'}), B('w2', 'tilerow', 0, 5, 24, 4, {shows: 'games', skip: 6}), B('w3', 'tilerow', 0, 10, 24, 4, {shows: 'games', skip: 12})],
  'x-focus': [B('clock', 'clock', 8, 0, 8, 3), B('hero', 'hero', 6, 4, 12, 6), B('games', 'tilerow', 2, 11, 20, 3, {shows: 'recent', count: 5})],
  'x-widgets': [B('hero', 'hero', 0, 0, 11, 7), B('friends', 'friends', 11, 0, 6, 3), B('downloads', 'downloads', 17, 0, 7, 3), B('widget', 'widget', 11, 3, 13, 4),
    B('games', 'tilerow', 0, 8, 24, 3, {shows: 'games'})],
  'x-orbit': [B('row', 'iconrow', 2, 1, 20, 3), B('hero', 'hero', 7, 5, 10, 7)],
  'x-spines': [B('games', 'tilerow', 0, 2, 24, 5, {shows: 'games'}), B('play', 'play', 0, 8, 14, 4)],
  'x-magazine': [B('hero', 'hero', 0, 0, 15, 9), B('recent', 'list', 16, 0, 8, 14, {shows: 'recent'}), B('title', 'text', 0, 10, 15, 2, {value: 'The Hearth'})],
  'x-dock': [B('hero', 'hero', 0, 0, 24, 10), B('row', 'iconrow', 2, 11, 20, 3)],
  'x-collections': [B('fav', 'tilerow', 0, 0, 24, 4, {shows: 'favs', title: 'Favorites'}), B('recent', 'tilerow', 0, 5, 24, 4, {shows: 'recent', title: 'Recent'}),
    B('games', 'tilerow', 0, 10, 24, 4, {shows: 'games', title: 'Installed'})],
  'x-journal': [B('recent', 'list', 0, 0, 8, 14, {shows: 'recent'}), B('hero', 'hero', 9, 0, 15, 7), B('widget', 'widget', 9, 8, 7, 3), B('clock', 'clock', 17, 8, 7, 3)],
};
const GMB = {pages: null, edit: false, page: 'home', pick: 0, held: false, undo: [], sayOpen: false, say: '', busy: false, err: '', rowPick: 0};
const gmbCopy = o => JSON.parse(JSON.stringify(o));

function gmbLoad(){
  let o = null;
  try{ o = JSON.parse(localStorage.getItem('nimbus.gmblocks') || 'null'); }catch(e){ o = null; }
  const p = {};
  for(const [k] of GMB_PAGES) p[k] = o && Array.isArray(o[k]) ? o[k] : gmbCopy(GMB_DEFAULT[k]);
  GMB.pages = p;
}
function gmbSave(){
  const v = JSON.stringify(GMB.pages);
  try{ localStorage.setItem('nimbus.gmblocks', v); }catch(e){}
  if(typeof uiPrefSave === 'function') uiPrefSave('nimbus.gmblocks', v);
}
function gmbBlocks(page){ if(!GMB.pages) gmbLoad(); return GMB.pages[page] || []; }
function gmbLibrary(){ return LOOK.layout === 'blocks' && gmbBlocks('library').length > 0; }
function gmbChange(f){ GMB.undo.push(JSON.stringify(GMB.pages)); if(GMB.undo.length > 60) GMB.undo.shift(); f(GMB.pages[GMB.page]); gmbSave(); }
function gmbUndo(){ const u = GMB.undo.pop(); if(!u) return toast('Nothing to undo'); GMB.pages = JSON.parse(u); gmbSave(); GMB.pick = Math.min(GMB.pick, gmbBlocks(GMB.page).length - 1); gmbRedraw(); }

// ---------- the games the blocks show
function gmbGames(shows){
  const V = visible();
  if(shows === 'recent') return V.filter(g => g.last).sort((a, b) => b.last - a.last).concat(V.filter(g => !g.last));
  if(shows === 'favs') return V.filter(g => g.fav);
  return [...V].filter(g => g.inst || g.cloud || isActive(g)).sort((a, b) => (b.last || 0) - (a.last || 0) || String(a.title).localeCompare(String(b.title)));
}
function gmbStyle(b){
  const s = b.style || {};
  return (s.colour ? `background:${s.colour};` : '') + (s.corners !== undefined ? `border-radius:${s.corners}px;` : '') + (s.seeThrough ? `opacity:${1 - s.seeThrough / 160};` : '');
}
function gmbArt(g){ return g ? `<div class="art" style="${heroStyle(g)}"></div>${heroImg(g)}` : ''; }

// One block's drawing. Outside the editor its parts are Game Mode's own buttons (Play, a game's page, Quick Access).
function gmbBlock(b, i, page){
  const ed = GMB.edit, key = `gmb-${page}-${b.id || i}`;
  const pos = `grid-column:${(b.x || 0) + 1} / span ${Math.max(1, b.w || 1)};grid-row:${(b.y || 0) + 1} / span ${Math.max(1, b.h || 1)};`;
  const picked = ed && GMB.page === page && GMB.pick === i;
  const cls = `gmb-b gmb-${esc(b.block)}${picked ? ' picked' : ''}${picked && GMB.held ? ' held' : ''}`;
  const chip = picked ? `<span class="gmb-chip">${GMB.held ? 'picked · move with the left stick' : 'A to pick up'}</span>` : '';
  const shell = (inner, act = '', data = '') => ed
    ? `<div class="${cls}" style="${pos}${gmbStyle(b)}" data-gmbi="${i}">${inner}${chip}</div>`
    : (act ? `<button class="${cls}" style="${pos}${gmbStyle(b)}" data-f data-k="${key}" data-act="${act}" ${data}>${inner}</button>`
           : `<div class="${cls}" style="${pos}${gmbStyle(b)}">${inner}</div>`);
  const list = gmbGames(b.shows).slice(b.skip || 0, (b.skip || 0) + (b.count || 12));
  switch(b.block){
    case 'hero': { const g = gmbGames('recent')[0]; return shell(`${gmbArt(g)}<div class="gmb-cap"><b>${esc(g ? g.title : 'Your games show here')}</b>${g && g.last ? `<small>${esc(ago(g.last))}</small>` : ''}</div>`, g ? 'game' : '', g ? `data-id="${esc(g.id)}"` : ''); }
    case 'tilerow': {
      const inner = `${b.title ? `<div class="gmb-t">${esc(b.title)}</div>` : ''}<div class="gmb-row">${list.map((g, j) => ed
        ? `<div class="gmb-tile">${gmbArt(g)}</div>` : `<button class="gmb-tile" data-f data-k="${key}-${j}" data-act="game" data-id="${esc(g.id)}">${gmbArt(g)}<span>${esc(g.title)}</span></button>`).join('') || '<div class="muted">No games yet</div>'}</div>`;
      return ed ? shell(inner) : `<div class="${cls} gmb-flat" style="${pos}">${inner}</div>`;
    }
    case 'iconrow': {
      const L = gmbGames('games').slice(0, b.count || 7), p = Math.min(GMB.rowPick, Math.max(0, L.length - 1));
      const inner = `<div class="gmb-row gmb-icons">${L.map((g, j) => ed ? `<div class="gmb-ic${j === p ? ' on' : ''}">${gmbArt(g)}</div>`
        : `<button class="gmb-ic${j === p ? ' on' : ''}" data-f data-k="${key}-${j}" data-act="gmbRow" data-j="${j}" data-id="${esc(g.id)}">${gmbArt(g)}</button>`).join('')}</div>`;
      return ed ? shell(inner) : `<div class="${cls} gmb-flat" style="${pos}">${inner}</div>`;
    }
    case 'play': {
      const L = gmbGames('games'), g = L[Math.min(GMB.rowPick, Math.max(0, L.length - 1))];
      return ed ? shell(`<div class="gmb-big">${esc(g ? g.title : 'Pick a game')}</div><div class="gmb-playb">Play</div>`)
        : `<div class="${cls} gmb-flat" style="${pos}"><div class="gmb-big">${esc(g ? g.title : 'Pick a game')}</div>${g ? `<button class="gmb-playb" data-f data-k="${key}" data-act="game" data-id="${esc(g.id)}">Play</button>` : ''}</div>`;
    }
    case 'widget': { const bt = (S.sys && S.sys.battery) || {}; return shell(`<div class="gmb-t">Widget · battery and Quick Resume</div><div class="gmb-num">${bt.present ? Math.round(bt.pct || 0) + '%' : 'Plugged in'} · ${QR.list.length} ${QR.list.length === 1 ? 'game' : 'games'}</div>`, 'gmbQa', 'data-v="resume"'); }
    case 'friends': { const fr = FR.s && FR.s.friends ? FR.s.friends.filter(f => f.state === 'ingame' || f.state === 'online').slice(0, 3) : null;
      return shell(`<div class="gmb-t">Friends</div>${fr ? (fr.length ? fr.map(f => `<div>${esc(f.name)}${f.game ? ' · ' + esc(f.game) : ''}</div>`).join('') : '<div class="muted">No one online right now</div>') : '<div class="muted">Friends show here</div>'}`, 'nav', 'data-view="friends"'); }
    case 'downloads': { const g = GAMES.find(x => isActive(x));
      return shell(`<div class="gmb-t">Downloads</div>${g ? `<div>${esc(g.title)}</div><div class="bar"><i style="width:${dlInfo(g).pct || 0}%"></i></div>` : '<div class="muted">Nothing downloading</div>'}`, 'nav', 'data-view="downloads"'); }
    case 'clock': return shell(`<div class="gmb-clock">${esc(clockText(new Date()))}</div>`);
    case 'text': return shell(`<div class="gmb-words">${esc(b.value || '')}</div>`);
    case 'list': {
      const L = (b.shows === 'recent' ? gmbGames('recent') : [...visible()].sort((a, c) => String(a.title).localeCompare(String(c.title)))).slice(0, 40);
      const inner = `<div class="gmb-t">${b.shows === 'recent' ? 'Recently played' : 'A–Z'}</div>${L.map((g, j) => ed ? `<div class="gmb-li">${esc(g.title)}</div>` : `<button class="gmb-li" data-f data-k="${key}-${j}" data-act="game" data-id="${esc(g.id)}">${esc(g.title)}</button>`).join('')}`;
      return ed ? shell(inner) : `<div class="${cls} gmb-flat gmb-scroll" style="${pos}">${inner}</div>`;
    }
    case 'qa': { const q = GMB_QA[b.shows] || GMB_QA.perf; return shell(`<div class="gmb-t">Quick Access</div><div class="gmb-qa">${esc(q[0])}</div>`, 'gmbQa', `data-v="${esc(b.shows || 'perf')}"`); }
    default: return '';
  }
}
// the + tile: in the first free 3×3 spot
function gmbFree(page, w, h){
  const L = gmbBlocks(page);
  for(let y = 0; y + h <= 14; y++) for(let x = 0; x + w <= 24; x++) if(!L.some(b => x < b.x + b.w && b.x < x + w && y < b.y + b.h && b.y < y + h)) return [x, y];
  return [21, 11];
}
function gmbGrid(page){
  const L = gmbBlocks(page);
  let h = L.map((b, i) => gmbBlock(b, i, page)).join('');
  if(GMB.edit && GMB.page === page){
    const [x, y] = gmbFree(page, 3, 3), on = GMB.pick === L.length;
    h += `<div class="gmb-b gmb-add${on ? ' picked' : ''}" style="grid-column:${x + 1} / span 3;grid-row:${y + 1} / span 3">+</div>`;
  }
  return `<div class="gmb${GMB.edit ? ' gmb-editing' : ''}">${h}</div>`;
}
function gmbView(page){
  if(!GMB.pages) gmbLoad();
  if(!GMB.edit) return gmbGrid(page) + `<button class="gmb-editbtn" data-f data-k="gmb-edit" data-act="gmbEdit">✎ Edit Game Mode</button>`;
  return gmbEditor();
}
// Quick Access (its quick tab) drawn from the Quick Access page's blocks.
function gmbQuick(){
  const L = gmbBlocks('quick').slice().sort((a, b) => a.y - b.y || a.x - b.x);
  if(!L.length) return '';
  return `<h3 style="margin-top:4px">Your Quick Access</h3><div class="gmb-qgrid">${L.map((b, i) => b.block === 'qa'
    ? `<button class="btn" data-f data-k="gmbq-${i}" data-act="gmbQa" data-v="${esc(b.shows || 'perf')}">${esc((GMB_QA[b.shows] || GMB_QA.perf)[0])}</button>` : '').join('')}</div>`;
}

// ---------- the editor (board 6)
function gmbEditor(){
  const pg = GMB_PAGES.find(p => p[0] === GMB.page) || GMB_PAGES[0];
  const say = GMB.sayOpen ? `<div class="gmb-say">
      <div class="gmb-saytext">${GMB.say ? '"' + esc(GMB.say) + '"' : '<span class="muted">Describe it: "make the big tile fill the screen and put my friends on the right"</span>'}<i class="gmb-caret"></i></div>
      <div class="gmb-keys">${'qwertyuiop'.split('').map(c => `<span>${c}</span>`).join('')}</div>
      <div class="gmb-saybtns"><button class="btn" data-f data-k="gmb-type" data-act="gmbType">⌨ Type it</button><button class="btn" data-f data-k="gmb-listen" data-act="gmbListen">🎤 Say it</button>
      <button class="btn" data-f data-k="gmb-undo" data-act="gmbUndo">Undo</button><button class="btn pri" data-f data-k="gmb-make" data-act="gmbMake">${GMB.busy ? 'Making it…' : 'Make it'}</button></div>
      ${GMB.err ? `<div class="gmb-err">${esc(GMB.err)}</div>` : ''}</div>` : '';
  return `<div class="gmb-edhead"><b>Editing Game Mode</b><span class="gmb-tag">EDITING</span><span class="gmb-pages">${GMB_PAGES.map(([k, l]) => `<span class="${k === GMB.page ? 'on' : ''}">${l}</span>`).join(' · ')}</span><span class="gmb-same">Same blocks as your desktop</span></div>
    ${gmbGrid(GMB.page)}${say}
    <div class="gmb-hints"><span><i class="gb a">A</i>Pick up / put down</span><span><i class="gb x">X</i>Its look</span><span><i class="gb y">Y</i>Describe it</span><span><i class="gb b">B</i>Done</span>
      <span><i class="gb lr">LB RB</i>Home · Library · Quick Access</span><span class="gmb-r">Hold the right stick: resize</span></div>`;
}
function gmbRedraw(){ if(typeof render === 'function') render(GMB.sayOpen ? 'gmb-make' : null, true); }
function gmbEdit(on = true){
  if(!GMB.pages) gmbLoad();
  GMB.edit = on; GMB.held = false; GMB.sayOpen = false; GMB.err = ''; GMB.pick = 0;
  if(on){ GMB.page = S.view === 'library' && gmbLibrary() ? 'library' : 'home'; GMB.undo = []; if(S.view !== 'home' && S.view !== 'library') navigate('home'); }
  if(!on && GMB.page === 'library') { GMB.page = 'home'; }
  gmbRedraw();
}
function gmbMove(dx, dy){
  const L = gmbBlocks(GMB.page);
  if(GMB.held && GMB.pick < L.length){
    gmbChange(P => { const b = P[GMB.pick]; b.x = Math.max(0, Math.min(24 - b.w, b.x + dx)); b.y = Math.max(0, Math.min(14 - b.h, b.y + dy)); });
  } else {
    // the nearest block that way (the + tile last)
    const cur = L[GMB.pick] || {x: 0, y: 0, w: 1, h: 1}, cx = cur.x + cur.w / 2, cy = cur.y + cur.h / 2;
    let best = -1, bd = 1e9;
    L.forEach((b, i) => { if(i === GMB.pick) return; const bx = b.x + b.w / 2 - cx, by = b.y + b.h / 2 - cy;
      if((dx && Math.sign(bx) === dx && Math.abs(bx) >= Math.abs(by) / 2) || (dy && Math.sign(by) === dy && Math.abs(by) >= Math.abs(bx) / 2)){ const d = Math.abs(bx) + Math.abs(by); if(d < bd){ bd = d; best = i; } } });
    GMB.pick = best >= 0 ? best : (dx > 0 || dy > 0 ? L.length : Math.max(0, GMB.pick === L.length ? L.length - 1 : GMB.pick));
  }
  gmbRedraw();
}
function gmbResize(d){
  const L = gmbBlocks(GMB.page); if(GMB.pick >= L.length) return;
  gmbChange(P => { const b = P[GMB.pick]; b.w = Math.max(1, Math.min(24 - b.x, b.w + d)); b.h = Math.max(1, Math.min(14 - b.y, b.h + d)); });
  gmbRedraw();
}
function gmbLook(){
  const L = gmbBlocks(GMB.page); if(GMB.pick >= L.length) return;
  gmbChange(P => { const b = P[GMB.pick], cur = JSON.stringify(b.style || {}); const i = GMB_LOOKS.findIndex(s => JSON.stringify(s) === cur); const n = GMB_LOOKS[(i + 1) % GMB_LOOKS.length];
    if(Object.keys(n).length) b.style = gmbCopy(n); else delete b.style; });
  gmbRedraw();
}
function gmbAdd(){
  const items = Object.keys(GMB_NAMES).filter(k => GMB.page === 'quick' ? ['qa', 'clock', 'widget', 'text', 'friends', 'downloads'].includes(k) : k !== 'qa').map(k => ({label: 'Add: ' + GMB_NAMES[k], icon: 'plus', run: () => {
    const size = {hero: [8, 5], tilerow: [12, 3], iconrow: [14, 3], widget: [5, 3], friends: [5, 3], downloads: [5, 3], clock: [5, 2], text: [8, 1], play: [12, 4], list: [6, 10], qa: [12, 2]}[k] || [4, 2];
    const [x, y] = gmbFree(GMB.page, size[0], size[1]);
    gmbChange(P => { let id = k, n = 2; while(P.some(b => b.id === id)) id = k + '-' + n++; P.push(Object.assign({id, block: k, x, y, w: size[0], h: size[1]}, k === 'tilerow' ? {shows: 'games'} : k === 'qa' ? {shows: 'perf'} : k === 'text' ? {value: 'Your words'} : {})); });
    GMB.pick = gmbBlocks(GMB.page).length - 1; gmbRedraw(); }}));
  if(GMB.page === 'home') for(const l of LAYOUTS.concat(XLAYOUTS).filter(l => GMB_FROM[l.id])) items.push({label: 'Start from: ' + l.name, icon: 'layout', run: () => { gmbChange(P => { P.splice(0, P.length, ...gmbCopy(GMB_FROM[l.id])); }); GMB.pick = 0; gmbRedraw(); }});
  if(GMB.pick < gmbBlocks(GMB.page).length) items.unshift({label: 'Remove this block', icon: 'x', run: () => { const i = GMB.pick; gmbChange(P => P.splice(i, 1)); GMB.pick = Math.max(0, i - 1); gmbRedraw(); }});
  openSheet({title: 'Add a block to ' + (GMB_PAGES.find(p => p[0] === GMB.page) || [0, 'Home'])[1], sub: 'The same blocks as your desktop; or start from one of the Home layouts', items});
}
async function gmbType(){
  const t = await openPrompt({title: 'Describe it', value: GMB.say, osk: true, mode: 'text'});
  if(typeof t === 'string'){ GMB.say = t.trim(); GMB.err = ''; gmbRedraw(); }
}
async function gmbListen(){
  GMB.err = 'Listening… say what you want'; gmbRedraw();
  try{ const r = await call('gmblocks.listen', {}, 60000); GMB.say = (r && r.text) || GMB.say; GMB.err = ''; }
  catch(e){ GMB.err = (e && e.message) || 'Nothing was heard'; }
  gmbRedraw();
}
async function gmbMake(){
  if(!GMB.say || GMB.busy) return gmbType();
  if(!host){ GMB.err = 'Your AI answers on Hearth OS'; return gmbRedraw(); }
  GMB.busy = true; GMB.err = ''; gmbRedraw();
  try{
    const r = await call('gmblocks.describe', {page: GMB.page, text: GMB.say, screen: {grid: {cols: 24, rows: 14}, blocks: gmbBlocks(GMB.page)}}, 300000);
    if(r && r.screen && Array.isArray(r.screen.blocks)){ gmbChange(P => { P.splice(0, P.length, ...r.screen.blocks); }); GMB.pick = 0; toast('Made ✓ (Undo puts it back)'); }
  }catch(e){ GMB.err = (e && e.message) || "Your AI didn't answer"; }
  GMB.busy = false; gmbRedraw();
}
function gmbAct(a, d, el){
  switch(a){
    case 'gmbEdit': return gmbEdit(true);
    case 'gmbType': return gmbType();
    case 'gmbListen': return gmbListen();
    case 'gmbUndo': return gmbUndo();
    case 'gmbMake': return gmbMake();
    case 'gmbRow': { const j = +d.j || 0; if(GMB.rowPick === j && d.id) return navigate('game', {game: d.id}); GMB.rowPick = j; return render(el && el.dataset.k, true); }
    case 'gmbQa': { const q = GMB_QA[d.v] || GMB_QA.perf; closeAll(); S.qtab = q[1]; return toggleLayer('qam'); }
  }
}
// The controller in the editor (index.html's input() asks first). True: handled here.
function gmbInput(k, top){
  if(!GMB.edit || top) return false;
  if(S.view !== 'home' && S.view !== 'library'){ GMB.edit = false; return false; }
  if(GMB.sayOpen){
    if(k === 'b'){ GMB.sayOpen = false; gmbRedraw(); return true; }
    if(k === 'y'){ gmbType(); return true; }
    return false;     // the box's buttons: Game Mode's own focus and A
  }
  const L = gmbBlocks(GMB.page);
  switch(k){
    case 'left': case 'right': case 'up': case 'down': gmbMove(k === 'left' ? -1 : k === 'right' ? 1 : 0, k === 'up' ? -1 : k === 'down' ? 1 : 0); return true;
    case 'a': if(GMB.pick >= L.length){ gmbAdd(); return true; } GMB.held = !GMB.held; gmbRedraw(); return true;
    case 'x': gmbLook(); return true;
    case 'y': GMB.sayOpen = true; GMB.err = ''; gmbRedraw(); if(!GMB.say) gmbType(); return true;
    case 'b': if(GMB.held){ GMB.held = false; gmbRedraw(); } else { gmbEdit(false); toast('Game Mode kept ✓'); } return true;
    case 'lb': case 'rb': { const i = GMB_PAGES.findIndex(p => p[0] === GMB.page); GMB.page = GMB_PAGES[(i + (k === 'rb' ? 1 : -1) + 3) % 3][0]; GMB.pick = 0; GMB.held = false; gmbRedraw(); return true; }
    case 'rup': case 'rdown': gmbResize(k === 'rup' ? 1 : -1); return true;
    case 'start': gmbAdd(); return true;
  }
  return false;
}
// the mouse and touch: a press picks a block (and picks it up)
document.addEventListener('click', e => {
  if(!GMB.edit) return;
  const b = e.target.closest && e.target.closest('.gmb-editing [data-gmbi], .gmb-editing .gmb-add');
  if(!b) return;
  const L = gmbBlocks(GMB.page);
  if(b.classList.contains('gmb-add')){ GMB.pick = L.length; gmbAdd(); return; }
  const i = +b.dataset.gmbi; if(GMB.pick === i) GMB.held = !GMB.held; else { GMB.pick = i; GMB.held = false; }
  gmbRedraw();
}, true);

(function gmbCss(){
  const s = document.createElement('style');
  s.textContent = `
.gmb{display:grid;grid-template-columns:repeat(24,1fr);grid-template-rows:repeat(14,1fr);gap:1.2vh;height:calc(100vh - 150px);min-height:420px;padding:2vh 3vw;box-sizing:border-box;position:relative}
.gmb-b{position:relative;overflow:hidden;border-radius:14px;background:var(--panel);color:var(--text);border:0;text-align:left;padding:14px 16px;font:inherit;box-sizing:border-box;min-width:0;min-height:0}
.gmb-b .art{position:absolute;inset:0;background-size:cover;background-position:center}
.gmb-b > img,.gmb-tile > img,.gmb-ic > img{position:absolute;inset:0;width:100%;height:100%;object-fit:cover}
.gmb-flat{background:transparent;padding:0;overflow:visible}
.gmb-cap{position:absolute;left:18px;bottom:14px;right:18px;text-shadow:0 2px 10px #000;display:flex;align-items:baseline;justify-content:space-between;gap:10px}
.gmb-cap b{font-size:clamp(18px,2.4vw,34px)}.gmb-cap small{opacity:.85}
.gmb-row{display:flex;gap:1vw;height:100%;min-height:0}.gmb-flat .gmb-t{margin-bottom:6px}
.gmb-tile{position:relative;flex:1;min-width:0;border-radius:12px;overflow:hidden;background:var(--panel2);border:0;padding:0;color:var(--text);font:inherit}
.gmb-tile span{position:absolute;left:10px;bottom:8px;right:10px;font-weight:700;text-shadow:0 2px 8px #000;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.gmb-icons{align-items:center}.gmb-ic{position:relative;height:62%;aspect-ratio:1;border-radius:18%;overflow:hidden;border:0;padding:0;background:var(--panel2)}
.gmb-ic.on{height:96%;outline:4px solid #fff;outline-offset:-4px}
.gmb-t{font-size:13px;opacity:.75;margin-bottom:6px}.gmb-num{font-size:clamp(20px,2.2vw,34px)}.gmb-clock{font-size:clamp(40px,6vw,96px);font-weight:300}
.gmb-big{font-size:clamp(28px,4vw,64px);font-weight:300;margin-bottom:14px}.gmb-playb{display:inline-block;background:#fff;color:#111;border-radius:999px;padding:12px 44px;font-weight:700;border:0;font:inherit;font-weight:700}
.gmb-li{display:block;width:100%;text-align:left;background:transparent;border:0;color:var(--text);padding:7px 4px;font:inherit;border-radius:8px}
.gmb-scroll{overflow:auto}.gmb-qa{font-size:20px;font-weight:600}.gmb-words{font-size:clamp(18px,2vw,30px)}
.gmb-qgrid{display:grid;grid-template-columns:1fr 1fr;gap:8px;margin-bottom:8px}
.gmb-editbtn{position:fixed;right:24px;bottom:60px;z-index:3;background:var(--panel2);color:var(--text);border:1px solid var(--line);border-radius:999px;padding:8px 16px;font:inherit;font-size:13px}
.gmb-edhead{display:flex;align-items:center;gap:14px;padding:12px 3vw;border-bottom:1px solid var(--line)}
.gmb-edhead b{font-size:18px}.gmb-tag{border:1px solid #ff7a2f;color:#ff9a52;border-radius:999px;padding:2px 10px;font-size:12px}
.gmb-pages{opacity:.7;font-size:14px}.gmb-pages .on{opacity:1;font-weight:700;color:var(--text)}.gmb-same{margin-left:auto;opacity:.8;font-size:14px}
.gmb-editing .gmb-b{cursor:pointer}
.gmb-editing .picked{outline:3px dashed #ff7a2f;outline-offset:4px;overflow:visible}
.gmb-chip{position:absolute;top:12px;right:12px;border:1px solid #ff7a2f;color:#ffb27a;border-radius:999px;padding:3px 12px;font-size:12px;background:rgba(0,0,0,.35);z-index:2}
.gmb-add{display:flex;align-items:center;justify-content:center;font-size:36px;background:transparent;border:2px dashed rgba(255,255,255,.35)}
.gmb-say{position:fixed;right:3vw;top:46vh;width:min(720px,40vw);background:#1b1420;border:2px solid #ff7a2f;border-radius:16px;padding:16px;z-index:4}
.gmb-saytext{font-size:18px;margin-bottom:12px}.gmb-caret{display:inline-block;width:2px;height:20px;background:#ff7a2f;vertical-align:middle;margin-left:2px}
.gmb-keys{display:grid;grid-template-columns:repeat(10,1fr);gap:6px;margin-bottom:12px}.gmb-keys span{background:var(--panel2);border-radius:6px;text-align:center;padding:6px 0;font-size:14px}
.gmb-saybtns{display:flex;gap:10px}.gmb-saybtns .pri{margin-left:auto;background:#ff7a2f;color:#1b1006}.gmb-err{margin-top:10px;color:#ff9a52;font-size:13px}
.gmb-hints{position:fixed;left:0;right:0;bottom:0;display:flex;gap:26px;align-items:center;padding:12px 3vw;background:rgba(0,0,0,.55);font-size:15px;z-index:3}
.gmb-hints .gb{display:inline-flex;align-items:center;justify-content:center;min-width:22px;height:22px;border-radius:11px;margin-right:6px;font-style:normal;font-size:11px;font-weight:700;color:#111;background:#ddd;padding:0 4px}
.gmb-hints .a{background:#3fae62;color:#fff}.gmb-hints .x{background:#4c7dff;color:#fff}.gmb-hints .y{background:#f2c14e}.gmb-hints .b{background:#e05a6a;color:#fff}.gmb-r{margin-left:auto;opacity:.85}`;
  document.head.appendChild(s);
})();
gmbLoad();
