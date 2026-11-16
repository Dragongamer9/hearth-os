// Places (1.14): a small map from OpenStreetMap on a photo's page (photos-2: the pin and the place's name), and the
// map behind the Places card and page (photos-3: groups of photos with their counts). The map pictures and names come
// through the app (src/place.rs), which keeps them, so a place seen once shows again offline.
'use strict';

(function () {
  const H = window.HP;
  const { invoke, url, esc } = H;
  const T = window.__TAURI__;
  const tileUrl = (z, x, y) => T ? url('tile', `${z}/${x}/${y}`) : `https://tile.openstreetmap.org/${z}/${x}/${y}.png`;
  const X = (lon, z) => (lon + 180) / 360 * 2 ** z * 256;
  const Y = (lat, z) => { const r = lat * Math.PI / 180; return (1 - Math.log(Math.tan(r) + 1 / Math.cos(r)) / Math.PI) / 2 * 2 ** z * 256; };

  /** The zoom that fits these points in w × h (with a margin), 2…16. */
  function fitZoom(pts, w, h) {
    if (pts.length < 2) return 14;
    for (let z = 16; z >= 2; z--) {
      const xs = pts.map(p => X(p[1], z)), ys = pts.map(p => Y(p[0], z));
      if (Math.max(...xs) - Math.min(...xs) < w * 0.75 && Math.max(...ys) - Math.min(...ys) < h * 0.7) return z;
    }
    return 2;
  }

  /** A map w × h centred on the points: tiles as pictures, then `marks` (HTML at each point: [lat, lon, html]). */
  function map(marks, w, h, opts = {}) {
    const pts = marks.map(m => [m[0], m[1]]);
    const z = opts.zoom || fitZoom(pts, w, h);
    const cx = pts.reduce((s, p) => s + X(p[1], z), 0) / pts.length, cy = pts.reduce((s, p) => s + Y(p[0], z), 0) / pts.length;
    const left = cx - w / 2, top = cy - h / 2, n = 2 ** z;
    let html = '';
    for (let ty = Math.floor(top / 256); ty <= Math.floor((top + h) / 256); ty++) {
      if (ty < 0 || ty >= n) continue;
      for (let tx = Math.floor(left / 256); tx <= Math.floor((left + w) / 256); tx++) {
        const wx = ((tx % n) + n) % n;
        html += `<img class="mt" alt="" loading="lazy" draggable="false" src="${tileUrl(z, wx, ty)}" style="left:${tx * 256 - left}px;top:${ty * 256 - top}px" onerror="this.remove()">`;
      }
    }
    for (const m of marks) html += `<span class="mk" style="left:${X(m[1], z) - left}px;top:${Y(m[0], z) - top}px">${m[2]}</span>`;
    return { html: `<div class="map" style="width:${w}px;height:${h}px">${html}<span class="osm">© OpenStreetMap</span></div>`, zoom: z };
  }

  /** Photos with a place, grouped into spots about `px` apart at this zoom: [{lat, lon, items}]. */
  function clusters(items, z, px = 70) {
    const out = [];
    for (const i of items) {
      if (!i.gps) continue;
      const x = X(i.gps[1], z), y = Y(i.gps[0], z);
      let c = out.find(c => Math.abs(c.x - x) < px && Math.abs(c.y - y) < px);
      if (!c) { c = { x, y, lat: i.gps[0], lon: i.gps[1], items: [] }; out.push(c); }
      c.items.push(i);
    }
    return out;
  }

  const names = new Map();
  async function placeName(lat, lon) {
    const k = `${lat.toFixed(3)},${lon.toFixed(3)}`;
    if (!names.has(k)) names.set(k, invoke('place_name', { lat, lon }).catch(() => ''));
    return names.get(k);
  }

  window.HPPlace = {
    map, clusters, fitZoom, placeName,
    /** The photo page's Place: the map with its pin, then the place's name (filled in by after()). */
    html(gps) {
      const m = map([[gps[0], gps[1], '<i class="pin"></i>']], 352, 150, { zoom: 14 });
      return `<div class="placebox">${m.html}</div><div class="dsub" id="placeName">${gps[0].toFixed(4)}, ${gps[1].toFixed(4)} · map from OpenStreetMap</div>`;
    },
    async after(gps) {
      const n = await placeName(gps[0], gps[1]);
      const el = document.getElementById('placeName');
      if (el && n) el.textContent = `Near ${n} · map from OpenStreetMap`;
    },
  };
})();
