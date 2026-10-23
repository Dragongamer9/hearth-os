'use strict';
// Hearth Maps' page: MapLibre GL draws OpenFreeMap's vector map; every request to a service (map pieces, search,
// directions) goes through the Rust side, which keeps each service's rules (src/services.rs). Search runs only when
// Enter (or Search) is pressed, never while typing (Nominatim's policy). The person's location is only asked for
// when they press "Where am I".
// Keyboard: arrows pan and + / - zoom when the map has focus, / or Ctrl+F search, Ctrl+D directions, L where am I,
// S saved places, M map style, Enter "what's here" at the middle of the map, Esc back.
// Controller: left stick pans, LT/RT (or the right stick) zoom, A what's here, Y search (on-screen keyboard),
// X directions, LB switches between the map and the panel, B back, Menu map style, View saved places.
const T = window.__TAURI__;
const invoke = T ? T.core.invoke : demoInvoke;
const listen = T ? T.event.listen : () => {};
const $ = (s, el = document) => el.querySelector(s);
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const LANG = (navigator.languages && navigator.languages[0]) || navigator.language || 'en';

const S = { services: null, view: null, saved: [], stack: [], panel: 'home', results: [], place: null, from: null, to: null, mode: 'car', route: null, me: null, pad: false, cacheBytes: 0, query: '' };
let map, marker = null, fromMarker = null, meMarker = null;

// ---------------------------------------------------------------- map pieces through the Rust side
maplibregl.addProtocol('hearth', async (params) => {
  const raw = await invoke('tile', { url: params.url });
  const buf = raw instanceof ArrayBuffer ? raw : ArrayBuffer.isView(raw) ? raw.buffer : new Uint8Array(raw).buffer;
  if (params.type === 'json') return { data: JSON.parse(new TextDecoder().decode(buf)) };
  if (params.type === 'string') return { data: new TextDecoder().decode(buf) };
  return { data: buf };
});
const styleUrl = id => `hearth://${new URL(S.services.tiles).host}/styles/${id}`;

// ---------------------------------------------------------------- start
(async () => {
  const c = await invoke('config');
  Object.assign(S, { services: c.services, view: c.view, saved: c.saved, cacheBytes: c.cacheBytes });
  if (!S.services.styles.find(s => s.id === S.view.style)) S.view.style = S.services.styles[0].id;
  try { map = new maplibregl.Map({
    container: 'map', style: styleUrl(S.view.style), center: [S.view.lon, S.view.lat], zoom: S.view.zoom,
    attributionControl: false, maxPitch: 60, dragRotate: true, keyboard: true, fadeDuration: 150,
  }); } catch (e) {
    // MapLibre needs WebGL; without it the panel still works for search, but there's no map to draw on.
    $('#pbody').innerHTML = `<p class="bad">This computer's graphics couldn't start the map (WebGL: ${esc(e.message || e)}).</p>`;
    throw e;
  }
  // The map data's attribution comes with the style (OpenFreeMap, OpenMapTiles, OpenStreetMap); search and
  // directions are credited here, with the link to fix the map that FOSSGIS asks for.
  map.addControl(new maplibregl.AttributionControl({ compact: false, customAttribution: 'Search <a href="https://nominatim.org/">Nominatim</a> · Directions <a href="https://project-osrm.org/">OSRM</a> by <a href="https://routing.openstreetmap.de/">FOSSGIS</a> · <a href="https://www.openstreetmap.org/fixthemap">Fix the map</a>' }), 'bottom-right');
  map.addControl(new maplibregl.ScaleControl({ unit: /^en-US/.test(LANG) ? 'imperial' : 'metric' }), 'bottom-left');
  map.on('style.load', drawRoute);
  map.on('moveend', saveViewSoon);
  map.on('contextmenu', e => whatsHere(e.lngLat.lat, e.lngLat.lng));
  map.on('error', e => { if (String(e.error?.message || e.error).includes('Offline')) offline(true); });
  map.getCanvas().setAttribute('tabindex', '0');
  map.getCanvas().addEventListener('focus', () => $('#map').classList.add('focus'));
  map.getCanvas().addEventListener('blur', () => $('#map').classList.remove('focus'));
  if (S.services.notice) toast(S.services.notice);
  const l = c.launch;
  if (l?.search) { $('#q').value = l.search; runSearch(l.search); }
  else if (l?.geo) {
    const g = l.geo;
    if (g.lat != null) { const p = { name: g.label || 'Dropped pin', address: '', lat: g.lat, lon: g.lon }; showPlace(p, g.zoom || 16); }
    else if (g.query) { $('#q').value = g.query; runSearch(g.query); }
  }
  render();
  if (!l) $('#q').focus();
})().catch(e => toast(String(e), true));

listen('maps://offline', () => offline(true));
window.addEventListener('error', e => toast(`Something went wrong: ${e.message}`, true));
window.addEventListener('unhandledrejection', e => toast(String(e.reason?.message || e.reason), true));
let offlineTimer;
function offline(on) {
  $('#banner').hidden = !on;
  $('#banner').textContent = 'Offline: showing the parts of the map seen before';
  clearTimeout(offlineTimer); offlineTimer = setTimeout(() => { $('#banner').hidden = true; }, 8000);
}

let viewTimer;
function saveViewSoon() {
  clearTimeout(viewTimer);
  viewTimer = setTimeout(() => {
    const c = map.getCenter();
    S.view = { lat: c.lat, lon: c.lng, zoom: map.getZoom(), style: S.view.style };
    invoke('set_view', { view: S.view }).catch(() => {});
  }, 1500);
}

// ---------------------------------------------------------------- the panel
function go(panel, push = true) { if (push && S.panel !== panel) S.stack.push(S.panel); S.panel = panel; render(); }
function back() {
  if (oskTarget) { closeOsk(); return true; }
  if (S.stack.length) { S.panel = S.stack.pop(); render(); focusPanel(); return true; }
  if (S.panel !== 'home') { S.panel = 'home'; clearMarker(); render(); return true; }
  if (document.activeElement !== map.getCanvas()) { map.getCanvas().focus(); return true; }
  return false;
}
function focusPanel() { setTimeout(() => ($('#pbody .f') || $('#q')).focus(), 0); }
const fmtDist = m => m >= 1000 ? `${(m / 1000).toFixed(m >= 10000 ? 0 : 1)} km` : `${Math.round(m / 10) * 10} m`;
const fmtTime = s => { const m = Math.round(s / 60); return m < 60 ? `${Math.max(1, m)} min` : `${Math.floor(m / 60)} h ${m % 60} min`; };
const coords = p => `${p.lat.toFixed(5)}, ${p.lon.toFixed(5)}`;
const isSaved = p => p && S.saved.some(x => Math.abs(x.lat - p.lat) < 1e-6 && Math.abs(x.lon - p.lon) < 1e-6);

function render() {
  const b = $('#pbody');
  const P = S.panel;
  if (P === 'home') b.innerHTML = '';
  else if (P === 'busy') b.innerHTML = `<p class="muted"><span class="spin"></span> ${esc(S.busyText || 'Searching…')}</p>`;
  else if (P === 'results') {
    b.innerHTML = S.results.length
      ? S.results.map((p, i) => `<button class="res f" data-res="${i}"><small>${esc(p.kind)}</small><b>${esc(p.name)}</b><span>${esc(p.address)}</span></button>`).join('') + note('Search by <a href="https://nominatim.org/">Nominatim</a>, data © <a href="https://www.openstreetmap.org/copyright">OpenStreetMap contributors</a>.')
      : `<p class="muted">Nothing found for “${esc(S.query)}”.</p>`;
  } else if (P === 'place') {
    const p = S.place;
    b.innerHTML = `<h2>${esc(p.name)}</h2><div class="sub">${esc(p.address || '')}</div><div class="sub">${coords(p)}</div>
      <div class="acts"><button class="btn primary f" data-act="to-here">Directions</button><button class="btn f" data-act="from-here">From here</button>
      <button class="btn f" data-act="save">${isSaved(p) ? 'Saved ✓' : 'Save'}</button><button class="btn f" data-act="copy">Copy</button><button class="btn f" data-act="osm">On OpenStreetMap</button></div>`;
  } else if (P === 'route') {
    const slot = (k, label, p) => `<div class="slot"><label>${label}</label><button class="res f" style="margin:0" data-slot="${k}">${p ? `<b>${esc(p.name)}</b><span>${esc(p.address || coords(p))}</span>` : `<span>Search for a place, or press here</span>`}</button></div>`;
    const r = S.route;
    b.innerHTML = `${slot('from', 'From', S.from)}${slot('to', 'To', S.to)}
      <div class="chips">${[['car', 'Car'], ['bike', 'Bike'], ['foot', 'On foot']].map(([m, t]) => `<button class="chip f ${S.mode === m ? 'on' : ''}" data-mode="${m}">${t}</button>`).join('')}</div>
      <div class="acts"><button class="btn primary f" data-act="route-go" ${S.from && S.to ? '' : 'disabled'}>Go</button><button class="btn f" data-act="swap">Swap</button><button class="btn f" data-act="from-me">From where I am</button><button class="btn f" data-act="route-web" ${S.from && S.to ? '' : 'disabled'}>Open in browser</button></div>
      ${S.routeError ? `<p class="bad">${esc(S.routeError)}</p>` : ''}
      ${r ? `<div class="sum"><b>${fmtTime(r.duration)}</b> · ${fmtDist(r.distance)}</div>${r.steps.map((s, i) => `<button class="step f" data-step="${i}"><span>${esc(s.text)}</span><span>${s.distance ? fmtDist(s.distance) : ''}</span></button>`).join('')}` : ''}
      ${note('Directions by <a href="https://project-osrm.org/">OSRM</a> on <a href="https://routing.openstreetmap.de/">FOSSGIS</a>’s server, data © <a href="https://www.openstreetmap.org/copyright">OpenStreetMap contributors</a>. Something wrong? <a href="https://www.openstreetmap.org/fixthemap">Fix the map</a>.')}`;
  } else if (P === 'pick') {
    b.innerHTML = `<p class="sub">Search for the ${S.picking === 'from' ? 'start' : 'destination'} above, or choose a saved place:</p>` + (S.saved.map((p, i) => `<button class="res f" data-pickSaved="${i}"><b>${esc(p.name)}</b><span>${esc(p.address || coords(p))}</span></button>`).join('') || '<p class="muted">No saved places yet.</p>')
      + `<div class="acts"><button class="btn f" data-act="pick-center">The middle of the map</button>${S.picking === 'from' ? '<button class="btn f" data-act="from-me">Where I am</button>' : ''}</div>`;
  } else if (P === 'saved') {
    b.innerHTML = `<h2>Saved places</h2>` + (S.saved.map((p, i) => `<div class="slot"><button class="res f" style="margin:0" data-saved="${i}"><b>${esc(p.name)}</b><span>${esc(p.address || coords(p))}</span></button><button class="btn small f" data-forget="${i}" title="Forget">✕</button></div>`).join('') || '<p class="muted">Places you save show up here. They stay on this computer.</p>');
  } else if (P === 'layers') {
    const mb = (S.cacheBytes / 1048576).toFixed(1);
    b.innerHTML = `<h2>Map style</h2><div class="chips">${S.services.styles.map(s => `<button class="chip f ${s.id === S.view.style ? 'on' : ''}" data-style="${esc(s.id)}">${esc(s.name)}</button>`).join('')}</div>
      <h2>Offline</h2><p class="sub">Parts of the map you've looked at are kept so they still show without internet: ${mb} MB of ${S.services.cacheMb} MB.</p>
      <div class="acts"><button class="btn f" data-act="clear-cache">Clear them</button></div>
      ${note('Map <a href="https://openfreemap.org/">OpenFreeMap</a> · <a href="https://www.openmaptiles.org/">© OpenMapTiles</a> · Data © <a href="https://www.openstreetmap.org/copyright">OpenStreetMap contributors</a>. Search <a href="https://nominatim.org/">Nominatim</a>. Directions <a href="https://project-osrm.org/">OSRM</a> by FOSSGIS. Your location is only looked up when you press Where am I, and isn\'t sent anywhere.')}`;
  }
}
const note = html => `<p class="note">${html}</p>`;

// ---------------------------------------------------------------- search
$('#searchrow').addEventListener('submit', e => { e.preventDefault(); runSearch($('#q').value); });
async function runSearch(q) {
  q = q.trim(); if (!q) return;
  // "52.52, 13.40" goes straight there.
  const m = /^\s*(-?\d{1,2}(?:\.\d+)?)\s*[, ]\s*(-?\d{1,3}(?:\.\d+)?)\s*$/.exec(q);
  if (m && Math.abs(+m[1]) <= 90 && Math.abs(+m[2]) <= 180) { showPlace({ name: 'Coordinates', address: '', lat: +m[1], lon: +m[2] }, 15); return; }
  S.query = q; S.busyText = 'Searching…';
  const back = S.panel === 'pick' ? 'pick' : null;
  go('busy', false);
  try {
    const b = map.getBounds();
    S.results = await invoke('find', { query: q, near: [b.getWest(), b.getSouth(), b.getEast(), b.getNorth()], lang: LANG });
    if (back) { S.panel = 'pick'; S.pickResults = true; }
    S.panel = 'results'; render();
    if (S.results.length === 1) pickResult(0); else focusPanel();
  } catch (e) { S.panel = back || 'home'; render(); toast(String(e), true); }
}
function pickResult(i) {
  const p = S.results[i];
  if (S.picking) { S[S.picking] = p; S.picking = null; S.route = null; S.routeError = ''; S.stack = S.stack.filter(x => x !== 'results' && x !== 'pick'); go('route', false); placeRouteMarkers(); focusPanel(); return; }
  showPlace(p);
}
function showPlace(p, zoom) {
  S.place = p;
  setMarker(p);
  if (p.bbox && !zoom) {
    const [s, n, w, e] = p.bbox;
    if (Math.abs(n - s) > 0.0005 || Math.abs(e - w) > 0.0005) map.fitBounds([[w, s], [e, n]], { padding: 80, maxZoom: 17, duration: 900 });
    else map.flyTo({ center: [p.lon, p.lat], zoom: 17, duration: 900 });
  } else map.flyTo({ center: [p.lon, p.lat], zoom: zoom || Math.max(map.getZoom(), 15), duration: 900 });
  go('place');
  focusPanel();
}
function setMarker(p) {
  clearMarker();
  const el = document.createElement('div'); el.className = 'pin';
  marker = new maplibregl.Marker({ element: el, anchor: 'bottom' }).setLngLat([p.lon, p.lat]).addTo(map);
}
function clearMarker() { if (marker) { marker.remove(); marker = null; } }

async function whatsHere(lat, lon) {
  const p = { name: 'Dropped pin', address: '', lat, lon };
  setMarker(p);
  S.place = p; go('place');
  try {
    const r = await invoke('whats_here', { lat, lon, zoom: Math.round(Math.min(18, Math.max(10, map.getZoom() + 1))), lang: LANG });
    if (r && S.place === p) { p.name = r.name || 'Dropped pin'; p.address = r.address; p.osm = r.osm; render(); }
  } catch (e) { toast(String(e), true); }
  focusPanel();
}

// ---------------------------------------------------------------- directions
function placeRouteMarkers() {
  if (fromMarker) { fromMarker.remove(); fromMarker = null; }
  clearMarker();
  if (S.from) { const el = document.createElement('div'); el.className = 'pin from'; fromMarker = new maplibregl.Marker({ element: el, anchor: 'bottom' }).setLngLat([S.from.lon, S.from.lat]).addTo(map); }
  if (S.to) setMarker(S.to);
  if (S.from && S.to) map.fitBounds([[Math.min(S.from.lon, S.to.lon), Math.min(S.from.lat, S.to.lat)], [Math.max(S.from.lon, S.to.lon), Math.max(S.from.lat, S.to.lat)]], { padding: 100, maxZoom: 16, duration: 800 });
}
async function runRoute() {
  if (!S.from || !S.to) return;
  S.routeError = ''; S.busyText = 'Finding the way…'; go('busy', false);
  try {
    S.route = await invoke('directions', { mode: S.mode, from: [S.from.lat, S.from.lon], to: [S.to.lat, S.to.lon] });
  } catch (e) { S.route = null; S.routeError = `${e} You can open the directions in your browser instead.`; }
  S.panel = 'route'; render(); drawRoute();
  if (S.route) { const b = S.route.line.reduce((a, [x, y]) => [Math.min(a[0], x), Math.min(a[1], y), Math.max(a[2], x), Math.max(a[3], y)], [180, 90, -180, -90]); map.fitBounds([[b[0], b[1]], [b[2], b[3]]], { padding: 90, duration: 800 }); }
  focusPanel();
}
function drawRoute() {
  if (!map || !map.isStyleLoaded()) return;
  const data = { type: 'Feature', properties: {}, geometry: { type: 'LineString', coordinates: S.route ? S.route.line : [] } };
  if (map.getSource('route')) { map.getSource('route').setData(data); return; }
  map.addSource('route', { type: 'geojson', data });
  map.addLayer({ id: 'route-casing', type: 'line', source: 'route', layout: { 'line-join': 'round', 'line-cap': 'round' }, paint: { 'line-color': '#2a1206', 'line-width': 9, 'line-opacity': .7 } });
  map.addLayer({ id: 'route', type: 'line', source: 'route', layout: { 'line-join': 'round', 'line-cap': 'round' }, paint: { 'line-color': '#ff8a3d', 'line-width': 5 } });
}
function clearRoute() { S.route = null; drawRoute(); if (fromMarker) { fromMarker.remove(); fromMarker = null; } }

async function whereAmI() {
  const btn = $('[data-act="locate"]'); btn.classList.add('busy');
  try {
    const f = await invoke('where_am_i');
    S.me = { name: 'Where I am', address: `Within about ${fmtDist(Math.max(10, f.accuracy || 0))}`, lat: f.lat, lon: f.lon };
    if (meMarker) meMarker.remove();
    const el = document.createElement('div'); el.className = 'me';
    meMarker = new maplibregl.Marker({ element: el }).setLngLat([f.lon, f.lat]).addTo(map);
    map.flyTo({ center: [f.lon, f.lat], zoom: f.accuracy > 2000 ? 12 : f.accuracy > 300 ? 14 : 16, duration: 1000 });
    return S.me;
  } catch (e) { toast(String(e), true); return null; }
  finally { btn.classList.remove('busy'); }
}

// ---------------------------------------------------------------- clicks
document.addEventListener('click', async e => {
  const a = e.target.closest('a[href]');
  if (a) { e.preventDefault(); const h = a.getAttribute('href'); if (/^https:\/\//.test(h)) invoke('open_web', { url: h }).catch(err => toast(String(err), true)); return; }
  const b = e.target.closest('button'); if (!b) return;
  const d = b.dataset;
  if (d.key && b.closest('#osk')) { oskPress(d.key); return; }
  if (d.res !== undefined) { pickResult(+d.res); return; }
  if (d.saved !== undefined) { showPlace(S.saved[+d.saved], 16); return; }
  if (d.picksaved !== undefined) { S[S.picking] = S.saved[+d.picksaved]; S.picking = null; S.route = null; go('route', false); placeRouteMarkers(); focusPanel(); return; }
  if (d.forget !== undefined) { const p = S.saved[+d.forget]; S.saved = await invoke('forget_place', { lat: p.lat, lon: p.lon }); render(); focusPanel(); return; }
  if (d.style) { S.view.style = d.style; map.setStyle(styleUrl(d.style)); saveViewSoon(); render(); $(`[data-style="${CSS.escape(d.style)}"]`)?.focus(); return; }
  if (d.mode) { S.mode = d.mode; S.route = null; S.routeError = ''; drawRoute(); render(); $(`[data-mode="${d.mode}"]`)?.focus(); return; }
  if (d.slot) { S.picking = d.slot; go('pick'); $('#q').value = ''; $('#q').focus(); return; }
  if (d.step !== undefined) { const s = S.route.steps[+d.step]; map.flyTo({ center: [s.lon, s.lat], zoom: 17, duration: 700 }); return; }
  switch (d.act) {
    case 'locate': { const me = await whereAmI(); if (me) { S.place = me; setMarker(me); go('place'); } return; }
    case 'route': S.stack = []; if (!S.to && S.place) S.to = S.place; go('route'); placeRouteMarkers(); focusPanel(); return;
    case 'saved': go('saved'); focusPanel(); return;
    case 'layers': S.cacheBytes = (await invoke('config').catch(() => ({}))).cacheBytes ?? S.cacheBytes; go('layers'); focusPanel(); return;
    case 'zoomin': map.zoomIn(); return;
    case 'zoomout': map.zoomOut(); return;
    case 'to-here': S.to = S.place; S.route = null; S.routeError = ''; go('route'); placeRouteMarkers(); focusPanel(); return;
    case 'from-here': S.from = S.place; S.route = null; S.routeError = ''; go('route'); placeRouteMarkers(); focusPanel(); return;
    case 'from-me': { const me = await whereAmI(); if (me) { S.from = me; S.picking = null; S.route = null; S.panel = 'route'; render(); placeRouteMarkers(); focusPanel(); } return; }
    case 'pick-center': { const c = map.getCenter(); S[S.picking] = { name: 'Point on the map', address: '', lat: c.lat, lon: c.lng }; S.picking = null; S.route = null; S.panel = 'route'; render(); placeRouteMarkers(); focusPanel(); return; }
    case 'swap': [S.from, S.to] = [S.to, S.from]; S.route = null; render(); placeRouteMarkers(); return;
    case 'route-go': runRoute(); return;
    case 'route-web': invoke('directions_in_browser', { mode: S.mode, from: [S.from.lat, S.from.lon], to: [S.to.lat, S.to.lon] }).catch(err => toast(String(err), true)); return;
    case 'save': { const p = S.place; S.saved = isSaved(p) ? await invoke('forget_place', { lat: p.lat, lon: p.lon }) : await invoke('save_place', { place: { name: p.name, address: p.address || '', lat: p.lat, lon: p.lon } }); render(); $('[data-act="save"]')?.focus(); return; }
    case 'copy': copy(coords(S.place)); return;
    case 'osm': invoke('open_web', { url: S.place.osm ? `https://www.openstreetmap.org/${S.place.osm}` : `https://www.openstreetmap.org/?mlat=${S.place.lat.toFixed(5)}&mlon=${S.place.lon.toFixed(5)}#map=17/${S.place.lat.toFixed(5)}/${S.place.lon.toFixed(5)}` }).catch(err => toast(String(err), true)); return;
    case 'clear-cache': S.cacheBytes = await invoke('clear_cache'); render(); toast('Cleared'); return;
  }
});
function copy(text) {
  const ok = () => toast('Copied');
  if (navigator.clipboard?.writeText) navigator.clipboard.writeText(text).then(ok, fb); else fb();
  function fb() { const t = document.createElement('textarea'); t.value = text; document.body.appendChild(t); t.select(); document.execCommand('copy'); t.remove(); ok(); }
}
let toastTimer;
function toast(text, bad) {
  $('.toast')?.remove();
  document.body.insertAdjacentHTML('beforeend', `<div class="toast ${bad ? 'bad' : ''}">${esc(text)}</div>`);
  clearTimeout(toastTimer); toastTimer = setTimeout(() => $('.toast')?.remove(), bad ? 6000 : 2500);
}

// ---------------------------------------------------------------- keyboard
document.addEventListener('keydown', e => {
  S.pad = false; showPadHints(false);
  const t = e.target, k = e.key, ctrl = e.ctrlKey || e.metaKey;
  const typing = t.tagName === 'INPUT';
  const onMap = t === map?.getCanvas();
  if (k === 'Escape') { e.preventDefault(); if (typing && t.value && t.id === 'q') { t.value = ''; return; } back(); return; }
  if (ctrl && (k === 'f' || k === 'F')) { e.preventDefault(); $('#q').focus(); $('#q').select(); return; }
  if (ctrl && (k === 'd' || k === 'D')) { e.preventDefault(); $('[data-act="route"]').click(); return; }
  if (typing) {
    if (k === 'ArrowDown') { e.preventDefault(); $('#pbody .f')?.focus(); }
    return;
  }
  if (k === '/') { e.preventDefault(); $('#q').focus(); return; }
  if (onMap) {
    if (k === 'Enter') { e.preventDefault(); const c = map.getCenter(); whatsHere(c.lat, c.lng); return; }
    if (k === 'l' || k === 'L') { $('[data-act="locate"]').click(); return; }
    if (k === 's' || k === 'S') { $('[data-act="saved"]').click(); return; }
    if (k === 'm' || k === 'M') { $('[data-act="layers"]').click(); return; }
    return; // arrows, + and - are MapLibre's own (pan and zoom)
  }
  const dirs = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
  if (dirs[k]) { e.preventDefault(); move(dirs[k]); }
});

// ---------------------------------------------------------------- moving through the panel and tools
function focusables() {
  if (oskTarget) return [...document.querySelectorAll('#osk .f')];
  return [...document.querySelectorAll('#panel .f, #tools .f')].filter(el => !el.disabled && el.offsetParent !== null);
}
function move(dir) {
  const cur = document.activeElement;
  const all = focusables(); if (!all.length) return;
  if (!cur || !all.includes(cur)) { all[0].focus(); return; }
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
  if (best) { best.focus(); best.scrollIntoView({ block: 'nearest' }); }
}

// The on-screen keyboard, for typing a search with a controller.
let oskTarget = null, shift = false;
const OSK = ['1234567890', 'qwertyuiop', 'asdfghjkl\'', 'zxcvbnm,.-'];
function openOsk(input) {
  oskTarget = input;
  const rows = OSK.map(r => [...r].map(c => `<button class="f" data-key="${esc(c)}">${esc(c)}</button>`).join('')).join('');
  $('#searchrow').insertAdjacentHTML('afterend', `<div class="osk" id="osk">${rows}<button class="f wide" data-key="shift">Shift</button><button class="f wide" data-key="space">Space</button><button class="f wide" data-key="back">⌫</button><button class="f wide" data-key="clear">Clear</button><button class="f wide" data-key="done">Search</button></div>`);
  $('#osk [data-key="q"]').focus();
}
function closeOsk() { const t = oskTarget; $('#osk')?.remove(); oskTarget = null; t?.focus(); }
function oskPress(k) {
  const t = oskTarget; if (!t) return;
  if (k === 'done') { closeOsk(); runSearch(t.value); return; }
  if (k === 'shift') { shift = !shift; document.querySelectorAll('#osk [data-key]').forEach(b => { if (b.dataset.key.length === 1) b.textContent = shift ? b.dataset.key.toUpperCase() : b.dataset.key; }); return; }
  if (k === 'back') t.value = t.value.slice(0, -1);
  else if (k === 'clear') t.value = '';
  else t.value += k === 'space' ? ' ' : (shift ? k.toUpperCase() : k);
}

// ---------------------------------------------------------------- controllers (the standard mapping)
function showPadHints(on) { $('#hints').hidden = !on; $('#cross').hidden = !on; }
const held = {}; let lastPadMove = 0, lastFrame = performance.now();
function pad() {
  const now = performance.now(), dt = Math.min(50, now - lastFrame); lastFrame = now;
  const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : [];
  const st = { up: false, down: false, left: false, right: false, a: false, b: false, x: false, y: false, lb: false, menu: false, view: false };
  let lx = 0, ly = 0, zoom = 0;
  for (const g of pads) {
    const bt = i => g.buttons[i]?.pressed, val = i => g.buttons[i]?.value || 0;
    const ax = g.axes[0] || 0, ay = g.axes[1] || 0, ry = g.axes[3] || 0;
    if (Math.abs(ax) > .18) lx = ax; if (Math.abs(ay) > .18) ly = ay;
    zoom += (val(7) - val(6)) + (Math.abs(ry) > .25 ? -ry : 0);
    st.up ||= bt(12); st.down ||= bt(13); st.left ||= bt(14); st.right ||= bt(15);
    st.a ||= bt(0); st.b ||= bt(1); st.x ||= bt(2); st.y ||= bt(3); st.lb ||= bt(4); st.view ||= bt(8); st.menu ||= bt(9);
  }
  const onMap = map && (document.activeElement === map.getCanvas() || document.activeElement === document.body);
  if (map && (lx || ly || Math.abs(zoom) > .05)) {
    S.pad = true; showPadHints(true);
    if (onMap && (lx || ly)) map.panBy([lx * dt * 0.9, ly * dt * 0.9], { duration: 0 });
    else if (!onMap) { if (ly < -.6) st.up = true; if (ly > .6) st.down = true; if (lx < -.6) st.left = true; if (lx > .6) st.right = true; }
    if (Math.abs(zoom) > .05) map.setZoom(map.getZoom() + zoom * dt * 0.0022);
  }
  for (const [k, on] of Object.entries(st)) {
    if (!on) { held[k] = 0; continue; }
    const first = !held[k];
    const repeat = held[k] && now - held[k] > 380 && now - lastPadMove > 110;
    if (first) held[k] = now;
    if (!(first || (repeat && ['up', 'down', 'left', 'right'].includes(k)))) continue;
    S.pad = true; lastPadMove = now; showPadHints(true);
    const focusedMap = document.activeElement === map?.getCanvas() || document.activeElement === document.body;
    if (['up', 'down', 'left', 'right'].includes(k)) {
      if (focusedMap && !oskTarget) { const s = 140; map.panBy({ up: [0, -s], down: [0, s], left: [-s, 0], right: [s, 0] }[k], { duration: 200 }); }
      else move(k);
    } else if (k === 'a') {
      if (focusedMap) { const c = map.getCenter(); whatsHere(c.lat, c.lng); }
      else { const el = document.activeElement; if (el?.id === 'q') { if (!oskTarget) openOsk(el); } else if (el?.tagName === 'BUTTON') el.click(); }
    } else if (k === 'b') { back(); }
    else if (k === 'y') { $('#q').focus(); if (!oskTarget) openOsk($('#q')); }
    else if (k === 'x') { $('[data-act="route"]').click(); }
    else if (k === 'lb') { if (focusedMap) focusPanel(); else map.getCanvas().focus(); }
    else if (k === 'menu') { $('[data-act="layers"]').click(); }
    else if (k === 'view') { $('[data-act="saved"]').click(); }
  }
  requestAnimationFrame(pad);
}
requestAnimationFrame(pad);
window.addEventListener('gamepadconnected', () => { showPadHints(true); });

// ---------------------------------------------------------------- without the app (opening the page in a browser)
function demoInvoke(cmd, args) {
  switch (cmd) {
    case 'config': return Promise.resolve({ services: { tiles: 'https://tiles.openfreemap.org', tilesEnabled: true, styles: [{ id: 'liberty', name: 'Liberty' }, { id: 'positron', name: 'Positron' }, { id: 'dark', name: 'Dark' }], searchEnabled: false, routingEnabled: false, notice: 'Preview: search and directions work in the app.', cacheMb: 50 }, view: { lat: 52.516, lon: 13.378, zoom: 13, style: 'liberty' }, saved: [], launch: null, cacheBytes: 0 });
    case 'tile': return fetch(args.url.replace(/^hearth:\/\//, 'https://')).then(async r => { const ct = r.headers.get('content-type') || ''; if (ct.includes('json')) return new TextEncoder().encode((await r.text()).replaceAll('https://tiles.openfreemap.org', 'hearth://tiles.openfreemap.org')).buffer; return r.arrayBuffer(); });
    case 'set_view': return Promise.resolve();
    default: return Promise.reject('This is a preview: open Maps for that.');
  }
}
