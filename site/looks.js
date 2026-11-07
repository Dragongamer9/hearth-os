// Hearth OS's looks on the website: made by tools/site-looks.py (don't edit by hand). The picker sits in any
// element with class "looks"; the choice is kept in this browser only.
(function(){
  var LOOKS = [{"id":"hearth","name":"Hearth","desc":"Warm firelit darks, frosted islands","sw":["#17110e","#33271f","#ff8a3d","#ffd166"],"wall":"fireside","bar":"floating","thin":false,"frame":"plain","clock":false,"perf":false,"edge":false,"scan":false,"ember":"#ff8a3d","flame":"#ffd166","lember":"#e8702a","lflame":"#c98a00"},{"id":"steam","name":"Steam-style","desc":"Blue-slate, flat, crisp corners","sw":["#1b2838","#2a475e","#1a9fff","#66c0f4"],"wall":"tide","bar":"bar","thin":false,"frame":"plain","clock":false,"perf":false,"edge":false,"scan":false,"ember":"#1a9fff","flame":"#66c0f4","lember":"#1a7fd4","lflame":"#1a9fff"},{"id":"gaming","name":"Gaming","desc":"Neon edges, HUD shapes, performance island","sw":["#0b0f16","#1a2433","#22e4ff","#ff3dc8"],"wall":"neon","bar":"floating","thin":false,"frame":"chamfer","clock":true,"perf":true,"edge":true,"scan":false,"ember":"#22e4ff","flame":"#ff3dc8","lember":"#0098b8","lflame":"#c0189a"},{"id":"zen","name":"Zen","desc":"One thin bar, quiet colours","sw":["#22211e","#3a3833","#8fb39a","#c9d8b6"],"wall":"zen","bar":"bar","thin":true,"frame":"plain","clock":false,"perf":false,"edge":false,"scan":false,"ember":"#8fb39a","flame":"#c9d8b6","lember":"#5f8a6c","lflame":"#7fa070"},{"id":"glass","name":"Glass","desc":"Frosted islands over aurora","sw":["#0e1630","#2e3a60","#5cf2c0","#b8f7e4"],"wall":"glass","bar":"floating","thin":false,"frame":"frost","clock":true,"perf":false,"edge":false,"scan":false,"ember":"#5cf2c0","flame":"#b8f7e4","lember":"#1aa57a","lflame":"#4fd1a5"},{"id":"synthwave","name":"Synthwave","desc":"Sunset grid, chrome type, scanlines","sw":["#1a0a2e","#36175a","#ff3fa4","#29e3ff"],"wall":"synth","bar":"floating","thin":false,"frame":"neon","clock":true,"perf":false,"edge":false,"scan":true,"ember":"#ff3fa4","flame":"#29e3ff","lember":"#d81b84","lflame":"#0a9fc0"},{"id":"cabin","name":"Cabin","desc":"Wood, embers, the most Hearth","sw":["#1b120c","#3d2819","#ff9440","#ffc46b"],"wall":"cabin","bar":"floating","thin":false,"frame":"wood","clock":true,"perf":false,"edge":false,"scan":false,"ember":"#ff9440","flame":"#ffc46b","lember":"#c8631c","lflame":"#b5800f"},{"id":"console","name":"Console","desc":"Big tiles for the TV, 10-foot","sw":["#15100d","#33271f","#ff8a3d","#ffd166"],"wall":"embers","bar":"shelf","thin":false,"frame":"plain","clock":false,"perf":false,"edge":false,"scan":false,"ember":"#ff8a3d","flame":"#ffd166","lember":"#e8702a","lflame":"#c98a00"}], DEF = "steam";
  var root = document.documentElement;
  function cur(){ return root.dataset.look || DEF; }
  function set(id){
    if (id === DEF) delete root.dataset.look; else root.dataset.look = id;
    try { if (id === DEF) localStorage.removeItem('hearth.look'); else localStorage.setItem('hearth.look', id); } catch (e) {}
  }
  function el(tag, cls, text){ var e = document.createElement(tag); if (cls) e.className = cls; if (text) e.textContent = text; return e; }
  [].slice.call(document.querySelectorAll('.looks')).forEach(function(box, n){
    var btn = el('button'); btn.type = 'button'; btn.setAttribute('aria-haspopup', 'true'); btn.setAttribute('aria-expanded', 'false');
    btn.setAttribute('aria-label', 'Pick a Hearth OS look for this site'); btn.title = 'Look';
    btn.innerHTML = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 3a9 9 0 1 0 0 18c1.1 0 1.7-.8 1.7-1.7 0-.5-.2-.9-.5-1.2-.3-.3-.5-.7-.5-1.2 0-.9.8-1.7 1.7-1.7H16a5 5 0 0 0 5-5C21 6.6 17 3 12 3z"/><circle cx="7.5" cy="11" r="1"/><circle cx="10" cy="7" r="1"/><circle cx="14.5" cy="7" r="1"/></svg>';
    var menu = el('div', 'looks-menu'); menu.hidden = true; menu.id = 'looks-menu-' + n; menu.setAttribute('role', 'menu');
    btn.setAttribute('aria-controls', menu.id);
    menu.appendChild(el('p', '', 'Hearth OS\'s looks, on this site:'));
    var items = LOOKS.map(function(l){
      var b = el('button'); b.type = 'button'; b.setAttribute('role', 'menuitemradio'); b.dataset.id = l.id;
      var sw = el('span', 'sw'); l.sw.forEach(function(c){ var i = el('i'); i.style.background = c; sw.appendChild(i); });
      var t = el('span'); t.appendChild(el('b', '', l.name)); if (l.desc) t.appendChild(el('small', '', l.desc));
      b.appendChild(sw); b.appendChild(t);
      b.addEventListener('click', function(){ set(l.id); sync(); close(false); });
      menu.appendChild(b); return b;
    });
    function sync(){ items.forEach(function(b){ b.setAttribute('aria-checked', b.dataset.id === cur()); }); }
    // The menu lives at the end of the page, not inside the bar: a look's island can clip what's inside it (Gaming's cut
    // corners) or blur it (frosted islands), which hid the menu and made the looks impossible to switch.
    function place(){ var r = btn.getBoundingClientRect(); menu.style.top = Math.round(r.bottom + 8) + 'px'; menu.style.right = Math.max(8, Math.round(innerWidth - r.right)) + 'px'; }
    function open(){ sync(); place(); menu.hidden = false; btn.setAttribute('aria-expanded', 'true');
      var c = items.filter(function(b){ return b.dataset.id === cur(); })[0] || items[0]; c.focus(); }
    function close(focus){ menu.hidden = true; btn.setAttribute('aria-expanded', 'false'); if (focus) btn.focus(); }
    btn.addEventListener('click', function(){ menu.hidden ? open() : close(false); });
    menu.addEventListener('keydown', function(e){
      var i = items.indexOf(document.activeElement);
      if (e.key === 'Escape') { e.preventDefault(); close(true); }
      else if (e.key === 'ArrowDown') { e.preventDefault(); items[(i + 1) % items.length].focus(); }
      else if (e.key === 'ArrowUp') { e.preventDefault(); items[(i - 1 + items.length) % items.length].focus(); }
    });
    document.addEventListener('click', function(e){ if (!menu.hidden && !box.contains(e.target) && !menu.contains(e.target)) close(false); });
    addEventListener('resize', function(){ if (!menu.hidden) place(); });
    addEventListener('scroll', function(){ if (!menu.hidden) place(); }, { passive: true });
    box.appendChild(btn); document.body.appendChild(menu);
  });

  var BASE = (function(){ var s = document.currentScript && document.currentScript.src; try { return new URL('img/looks/', s || location.href).href; } catch (e) { return 'img/looks/'; } })();
  var still = matchMedia('(prefers-reduced-motion: reduce)');
  var lightQ = matchMedia('(prefers-color-scheme: light)');
  function byId(id){ return LOOKS.filter(function(l){ return l.id === id; })[0] || LOOKS.filter(function(l){ return l.id === DEF; })[0]; }
  function isDark(){ var t = root.dataset.theme; return t ? t === 'dark' : !lightQ.matches; }
  var imgs = {};
  function img(n){ if (!imgs[n]) { var i = new Image(); i.src = BASE + n; imgs[n] = i; i.onload = function(){ paint(true); }; } return imgs[n]; }
  function ready(i){ return i.complete && i.naturalWidth > 0; }
  // The page's own pieces, put in islands so a look with floating islands can draw them apart.
  function wrap(bar, items, cls){
    items = items.filter(Boolean); if (!items.length) return null;
    var d = el('div', 'lk-isl fr ' + cls); items[0].parentNode.insertBefore(d, items[0]); items.forEach(function(n){ d.appendChild(n); }); return d;
  }
  var head = document.querySelector('header.top .wrap'), dnav = document.querySelector('nav.top .wrap'), clockEl = null, perfEl = null, hint = null;
  if (head) {
    wrap(head, [head.querySelector('.brand')], 'brandi');
    wrap(head, [head.querySelector('nav')], 'navi');
    hint = el('span', 'lk-hint'); hint.innerHTML = '<i>A</i>open <i>B</i>back';
    var tools = [head.querySelector('.looks'), head.querySelector('.theme'), head.querySelector('.dl')].filter(Boolean);
    if (tools.length) tools[0].parentNode.insertBefore(hint, tools[0]);
    var t = wrap(head, [hint].concat(tools), 'tools');
  } else if (dnav) {
    wrap(dnav, [dnav.querySelector('.brand')], 'brandi');
    wrap(dnav, [dnav.querySelector('.links'), dnav.querySelector('.looks')], 'tools');
  }
  var cv = el('canvas', 'lk-wall'), pic = el('div', 'lk-wall'), scan = el('div', 'lk-fx lk-scan'), edge = el('div', 'lk-fx lk-edge');
  cv.setAttribute('aria-hidden', 'true'); pic.setAttribute('aria-hidden', 'true');
  document.body.insertBefore(pic, document.body.firstChild); document.body.insertBefore(cv, document.body.firstChild);
  document.body.appendChild(scan); document.body.appendChild(edge);
  var c = cv.getContext('2d'), W = 0, H = 0, L = null, dark = true, t0 = performance.now(), raf = 0, sparks = [];
  function lin(y0, y1, stops){ var g = c.createLinearGradient(0, y0, 0, y1); stops.forEach(function(s){ g.addColorStop(s[0], s[1]); }); return g; }
  function blob(n, x, y, size, o){ var i = img(n); if (!ready(i)) return; c.globalAlpha = o; c.drawImage(i, x - size / 2, y - size / 2, size, size); c.globalAlpha = 1; }
  var DRAW = {
    fireside: function(t){   // the Hearth look: warm darkness lit from below by a slowly breathing fire, embers drifting up
      c.fillStyle = lin(0, H, dark ? [[0, '#0b0807'], [.6, '#17110e'], [1, '#1f140e']] : [[0, '#f6efe8'], [.6, '#efe5db'], [1, '#f1dcc8']]); c.fillRect(0, 0, W, H);
      var g = img('ember-glow.png'); if (!ready(g)) return;
      var br = (Math.sin(t / 1300) + 1) / 2, fl = (Math.sin(t / 310) + Math.sin(t / 190)) / 4 + .5, dr = Math.sin(t / 1800) * W * .02;
      c.globalAlpha = (dark ? .5 : .3) + br * .1; c.drawImage(g, (W - W * 1.5) / 2, H - H * 1.1 * .45, W * 1.5, H * 1.1);
      c.globalAlpha = (dark ? .34 : .2) + fl * .1; c.drawImage(g, (W - W * .7) / 2 + dr, H - H * .6 * .42, W * .7, H * .6); c.globalAlpha = 1;
      if (moving()) { if (Math.random() < .05) sparks.push({ x: W * (.25 + Math.random() * .5), y: H + 4, v: H * (.0006 + Math.random() * .0006), w: Math.random() * 6.28, c: Math.random() > .5 }); }
      else if (!sparks.length) for (var i = 0; i < 14; i++) sparks.push({ x: W * .25 + ((i * .618) % 1) * W * .5, y: H * (.55 + ((i * .754) % 1) * .42), v: 0, w: i, c: i % 2 });
      sparks = sparks.filter(function(s){ s.y -= s.v * 16; s.x += Math.sin(t / 900 + s.w) * .3; return s.y > H * .2; });
      sparks.forEach(function(s){ c.globalAlpha = .55 * Math.min(1, (s.y - H * .2) / (H * .3)); c.fillStyle = s.c ? (dark ? L.ember : L.lember) : (dark ? L.flame : L.lflame); c.beginPath(); c.arc(s.x, s.y, 2.2, 0, 6.3); c.fill(); });
      c.globalAlpha = 1;
    },
    tide: function(t){   // Steam-style: soft blue light drifting slowly over blue-slate
      c.fillStyle = lin(0, H, dark ? [[0, '#0e131b'], [1, '#1b2838']] : [[0, '#e6ebf0'], [1, '#dbe2e9']]); c.fillRect(0, 0, W, H);
      [['blob-blue.png', .22, .64, .62, .10, .06, 31, .55], ['blob-slate.png', .52, .32, .70, -.12, .08, 37, .7], ['blob-sky.png', .76, .62, .46, -.08, -.10, 41, .35], ['blob-blue.png', .84, .20, .40, -.10, .05, 34, .45]].forEach(function(b){
        var k = Math.sin(t / (b[6] * 1000 / 6.283)); blob(b[0], W * (b[1] + b[4] * k), H * (b[2] + b[5] * k), H * b[3] * 1.7, b[7] * (dark ? 1 : .45)); });
      c.fillStyle = lin(H * .55, H, [[0, 'rgba(9,12,18,0)'], [1, dark ? 'rgba(9,12,18,.5)' : 'rgba(9,12,18,.04)']]); c.fillRect(0, 0, W, H);
    },
    neon: function(){   // Gaming: near-black, cyan lines, a magenta HUD frame
      c.fillStyle = lin(0, H, [[0, dark ? '#06080c' : '#e9f0f5'], [1, dark ? '#0b0f18' : '#dbe5ee']]); c.fillRect(0, 0, W, H);
      c.strokeStyle = dark ? 'rgba(34,228,255,0.08)' : 'rgba(0,152,184,0.10)'; c.lineWidth = Math.max(1, W / 1920);
      for (var x = -H; x < W; x += W / 12) { c.beginPath(); c.moveTo(x, H); c.lineTo(x + H * .48, H * .52); c.stroke(); }
      var g = c.createRadialGradient(W * .74, H * .34, 0, W * .74, H * .34, H * .45); g.addColorStop(0, dark ? 'rgba(255,61,200,0.20)' : 'rgba(192,24,154,0.12)'); g.addColorStop(1, 'rgba(0,0,0,0)'); c.fillStyle = g; c.fillRect(0, 0, W, H);
      var fx = W * .615, fy = H * .185, fw = W * .24, fh = H * .31, cut = H * .08, i = H * .04;
      c.strokeStyle = dark ? 'rgba(255,61,200,0.75)' : 'rgba(192,24,154,0.55)'; c.lineWidth = Math.max(2, W / 800);
      c.beginPath(); c.moveTo(fx, fy); c.lineTo(fx + fw - cut, fy); c.lineTo(fx + fw, fy + cut); c.lineTo(fx + fw, fy + fh); c.lineTo(fx + cut, fy + fh); c.lineTo(fx, fy + fh - cut); c.closePath(); c.stroke();
      c.strokeStyle = dark ? 'rgba(34,228,255,0.35)' : 'rgba(0,152,184,0.35)'; c.lineWidth = Math.max(1, W / 1600);
      c.beginPath(); c.moveTo(fx + i, fy + i); c.lineTo(fx + fw - cut - i * .5, fy + i); c.lineTo(fx + fw - i, fy + cut + i * .5); c.lineTo(fx + fw - i, fy + fh - i); c.lineTo(fx + cut + i * .5, fy + fh - i); c.lineTo(fx + i, fy + fh - cut - i * .5); c.closePath(); c.stroke();
      c.fillStyle = dark ? '#2a2550' : '#c9d6e2'; c.font = 'bold ' + Math.round(H * .085) + 'px "DejaVu Sans Condensed","Arial Narrow",sans-serif'; c.fillText('HEARTH', W * .655, H * .37);
    },
    zen: function(){   // Zen: warm grey, one brush circle
      c.fillStyle = lin(0, H, [[0, dark ? '#1d1c1a' : '#f1eee8'], [1, dark ? '#171614' : '#e6e2da']]); c.fillRect(0, 0, W, H);
      var cx = W > 900 ? W * .72 : W / 2, cy = H * .47, r = H * .2; c.lineCap = 'round';
      for (var s = 0; s < 120; s++) { var a0 = -.9 + (s / 120) * 5.6, a1 = -.9 + ((s + 1) / 120) * 5.6, k = s / 120;
        c.strokeStyle = dark ? 'rgba(201,216,182,' + (.5 - k * .3) + ')' : 'rgba(60,58,52,' + (.55 - k * .35) + ')'; c.lineWidth = r * (.16 - k * .12); c.beginPath(); c.arc(cx, cy, r, a0, a1); c.stroke(); }
    },
    glass: function(){   // Glass: deep night with a soft aurora
      c.fillStyle = lin(0, H, [[0, dark ? '#081022' : '#dfe8f7'], [1, dark ? '#120d2a' : '#efe6f6']]); c.fillRect(0, 0, W, H);
      [['blob-sky.png', .22, .32, .9, .75], ['blob-plum.png', .68, .42, 1, .8], ['blob-blue.png', .45, .78, .8, .55], ['blob-sky.png', .86, .12, .5, .4]].forEach(function(b){ blob(b[0], W * b[1], H * b[2], H * b[3] * 1.3, b[4] * (dark ? 1 : .6)); });
      c.save(); c.globalAlpha = dark ? .35 : .25; c.translate(W / 2, H * .38); c.rotate(-8 * Math.PI / 180);
      var g = c.createLinearGradient(0, -H * .08, 0, H * .08); g.addColorStop(0, 'rgba(92,242,192,0)'); g.addColorStop(.5, '#5cf2c0'); g.addColorStop(1, 'rgba(92,242,192,0)');
      c.fillStyle = g; c.fillRect(-W * .6, -H * .08, W * 1.2, H * .16); c.restore();
    },
    synth: function(){   // Synthwave: striped sun over a magenta grid, scanlines
      var hz = H * .6; c.fillStyle = lin(0, hz, [[0, dark ? '#12062a' : '#ffe3f3'], [.65, dark ? '#5a1460' : '#ffb3d9'], [1, dark ? '#ff5a8a' : '#ff8fb8']]); c.fillRect(0, 0, W, hz);
      c.fillStyle = dark ? 'rgba(255,240,255,0.7)' : 'rgba(255,255,255,0.8)';
      for (var i = 0; i < 70; i++) c.fillRect(((i * .6180339) % 1) * W, ((i * .7548776) % 1) * hz * .6, 2, 2);
      var cx = W > 900 ? W * .72 : W / 2, cy = hz - H * .02, r = H * .2, g = c.createLinearGradient(0, cy - r, 0, cy + r); g.addColorStop(0, '#ffe66d'); g.addColorStop(1, '#ff3fa4');
      c.save(); c.beginPath(); c.arc(cx, cy, r, Math.PI, 0); c.closePath(); c.clip(); c.fillStyle = g; c.fillRect(cx - r, cy - r, r * 2, r * 2);
      c.fillStyle = dark ? '#3a0f4a' : '#ffb3d9'; for (var k = 0; k < 6; k++) c.fillRect(cx - r, cy - r * .55 + k * r * .11, r * 2, 2 + k * 1.6); c.restore();
      c.fillStyle = lin(hz, H, [[0, dark ? '#1a0630' : '#f7d6ef'], [1, dark ? '#07020f' : '#f1c5e6']]); c.fillRect(0, hz, W, H - hz);
      c.strokeStyle = dark ? 'rgba(255,63,164,0.8)' : 'rgba(216,27,132,0.55)'; c.lineWidth = Math.max(1, W / 1400);
      for (var a = -14; a <= 14; a++) { c.beginPath(); c.moveTo(cx + a * W * .01, hz); c.lineTo(cx + a * W * .12, H); c.stroke(); }
      for (var j = 1; j < 14; j++) { var y = hz + (H - hz) * Math.pow(j / 13, 2.2); c.beginPath(); c.moveTo(0, y); c.lineTo(W, y); c.stroke(); }
      g = c.createRadialGradient(W / 2, H / 2, H * .3, W / 2, H / 2, W * .7); g.addColorStop(0, 'rgba(0,0,0,0)'); g.addColorStop(1, 'rgba(0,0,0,0.4)'); c.fillStyle = g; c.fillRect(0, 0, W, H);
    }
  };
  var MOVING = { fireside: 1, tide: 1 };
  function moving(){ return !still.matches && !document.hidden && L && MOVING[L.wall]; }
  function paint(force){
    if (!L) return;
    if (!DRAW[L.wall]) { cv.hidden = true; pic.hidden = false;
      pic.style.backgroundImage = (dark ? '' : 'linear-gradient(rgba(255,250,244,.74),rgba(255,250,244,.74)),') + 'url(' + BASE + L.wall + '.webp)'; return; }
    cv.hidden = false; pic.hidden = true;
    var w = innerWidth, h = innerHeight;
    if (force || w !== W || h !== H) { W = w; H = h; cv.width = W; cv.height = H; }
    DRAW[L.wall](performance.now() - t0);
  }
  var last = 0, frames = 0, fpsT = 0;
  function loop(now){
    raf = 0;
    frames++; if (now - fpsT > 1000) { if (perfEl && L && L.perf) perfEl.innerHTML = '<span>FPS <b>' + Math.round(frames * 1000 / (now - fpsT)) + '</b></span><span>Page <b>' + (document.documentElement.scrollHeight / 1000).toFixed(1) + 'k px</b></span>'; frames = 0; fpsT = now; }
    if (now - last > 33) { last = now; paint(false); }
    if (moving() || (perfEl && L && L.perf && !document.hidden)) raf = requestAnimationFrame(loop);
  }
  function tick(){ if (clockEl && L && L.clock) clockEl.textContent = new Date().toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }); }
  function apply(){
    L = byId(cur()); dark = isDark(); sparks = [];
    root.classList.add('has-wall'); root.dataset.bar = L.bar; root.dataset.frame = L.frame;
    if (L.thin) root.dataset.thin = ''; else delete root.dataset.thin;
    scan.hidden = !L.scan; edge.hidden = !L.edge;
    if (perfEl) perfEl.hidden = !L.perf || L.bar !== 'floating'; if (clockEl) clockEl.hidden = !L.clock || L.bar !== 'floating';
    tick(); paint(true);
    if (!raf && (moving() || (perfEl && L.perf))) raf = requestAnimationFrame(loop);
  }
  apply();
  setInterval(tick, 15000);
  addEventListener('resize', function(){ paint(true); });
  document.addEventListener('visibilitychange', function(){ if (!document.hidden && !raf) raf = requestAnimationFrame(loop); });
  [still, lightQ].forEach(function(q){ (q.addEventListener ? q.addEventListener('change', apply) : q.addListener(apply)); });
  new MutationObserver(apply).observe(root, { attributes: true, attributeFilter: ['data-look', 'data-theme'] });
})();
