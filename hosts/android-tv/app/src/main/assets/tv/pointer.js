/* Hearth TV: the remote's mouse pointer on websites (Theater's viewer, ViewerActivity.kt).
   This page (pointer.html) is a see-through layer over the website: it draws the pointer, the hint and the pointer's
   menu, and works out where the pointer is. Nothing of it goes into the website: the clicks, long-presses, hovering and
   scrolling are real touch and mouse events that ViewerActivity puts into the website's own view
   (HearthPointerNative: move, tap, longPress, rightClick, scroll, back, setMode).
   The remote: ▲▼◀▶ move (slow at first, faster when held), OK clicks, hold OK for the menu (long-press, right-click,
   page mode), Back closes the menu or goes back a page, CH +/− or the number keys scroll, ☰ or a colour key switches
   between pointer and page mode. Nothing is drawn while the pointer stands still.
   The same file is the pointer's logic for the unit tests (hosts/android-tv/test/pointer.test.js). */
(function (root, factory) {
  var P = factory();
  if (typeof module === 'object' && module.exports) module.exports = P;
  else root.HearthPointer = P;
})(this, function () {
  'use strict';
  var SPEEDS = { slow: 0.6, normal: 1, fast: 1.6 };
  var SIZES = { s: 30, m: 42, l: 58 };           // the pointer's height in pixels on a 1080p screen
  var LONG_MS = 600;                             // holding OK this long opens the menu
  var MOVE_MS = 33;                              // hovering reaches the website at most 30 times a second
  var BASE = 0.22;                               // screen heights a second at first
  var MAXMUL = 5, RAMP_DELAY = 150, RAMP_MS = 1100;
  var NUDGE = 0.006;                             // one short press: a small, precise step
  var DIRS = { up: [0, -1], down: [0, 1], left: [-1, 0], right: [1, 0] };
  var MENU = [
    { id: 'long', label: 'Long-press here', why: 'Like holding a finger on it: opens the site\'s own menu' },
    { id: 'right', label: 'Right-click here', why: 'For sites that use a mouse\'s right button' },
    { id: 'page', label: 'Switch to page mode', why: 'The remote works the page the way the site built it' },
    { id: 'close', label: 'Close', why: '' }
  ];

  /** How fast the pointer goes (screen heights a second) after a button has been held [heldMs]. */
  function speedAt(heldMs, speed) {
    var f = SPEEDS[speed] || 1;
    var t = Math.max(0, Math.min(1, (heldMs - RAMP_DELAY) / RAMP_MS));
    return BASE * f * (1 + (MAXMUL - 1) * t * t * (3 - 2 * t));
  }

  function hintText(mode) {
    return mode === 'pointer'
      ? { title: 'Pointer', keys: [['▲▼◀▶', 'Move'], ['OK', 'Click'], ['Hold OK', 'Long-press, right-click, page mode'], ['CH +/−', 'Scroll'], ['↩', 'Back a page']] }
      : { title: 'Page mode', keys: [['▲▼◀▶ OK', 'Work the page the way the site built it'], ['Hold OK', 'Pointer'], ['↩', 'Back a page']] };
  }

  function Pointer(o) {
    o = o || {};
    this.native = o.native || null;
    this.W = o.width || 1920; this.H = o.height || 1080;
    this.speed = SPEEDS[o.speed] ? o.speed : 'normal';
    this.size = SIZES[o.size] ? o.size : 'm';
    this.mode = o.startOn === false ? 'page' : 'pointer';
    this.x = this.W / 2; this.y = this.H / 2;
    this.held = {}; this.last = 0; this.lastMove = -1e9;
    this.scrollX = 0; this.scrollY = 0;
    this.okAt = 0; this.okLong = false; this.okTimer = null;
    this.menu = null;                              // the menu's chosen row while it's open
    this.menuHeld = {};
    this.view = o.view || function () {};
    this.later = o.later || function (fn, ms) { return setTimeout(fn, ms); };
    this.cancel = o.cancel || function (t) { clearTimeout(t); };
    this.frame = o.frame || null;                  // requestAnimationFrame
    this.running = false;
  }
  var P = Pointer.prototype;
  P.call = function (fn) {
    var n = this.native;
    if (!n || typeof n[fn] !== 'function') return null;
    try { return n[fn].apply(n, [].slice.call(arguments, 1)); } catch (e) { return null; }
  };
  P.fx = function () { return this.x / this.W; };
  P.fy = function () { return this.y / this.H; };
  P.holding = function () { var h = this.held; return Object.keys(h).some(function (k) { return h[k]; }); };
  /** The screen changed size: the pointer stays at the same place on it. */
  P.resize = function (w, h) {
    if (!w || !h) return;
    var fx = this.fx(), fy = this.fy();
    this.W = w; this.H = h; this.x = fx * w; this.y = fy * h;
    this.view(this, 'move');
  };
  /** Puts the pointer at a place on the screen (fractions of its width and height). */
  P.place = function (fx, fy, now) {
    this.x = Math.max(0, Math.min(this.W - 1, fx * this.W)); this.y = Math.max(0, Math.min(this.H - 1, fy * this.H));
    this.view(this, 'move'); this.sendMove(now || 0, true);
  };

  P.key = function (dir, down, now) {
    if (!DIRS[dir] || this.mode !== 'pointer') return false;
    if (this.menu !== null) {
      // One step a press in the menu (a held button doesn't run through it).
      var was = this.menuHeld[dir]; this.menuHeld[dir] = down;
      if (down && !was && (dir === 'up' || dir === 'down')) {
        this.menu = (this.menu + (dir === 'down' ? 1 : MENU.length - 1)) % MENU.length;
        this.view(this, 'menu');
      }
      return true;
    }
    if (down) {
      if (this.held[dir]) return true;
      this.held[dir] = now || 1;
      this.last = now;
      this.moveBy(DIRS[dir][0] * NUDGE * this.H, DIRS[dir][1] * NUDGE * this.H, now, true);
      this.start();
    } else {
      delete this.held[dir];
      if (!this.holding()) this.sendMove(now, true);
    }
    return true;
  };
  /** One frame while a button is held. Returns whether to keep going. */
  P.tick = function (now) {
    var dt = Math.min(0.1, Math.max(0, (now - this.last) / 1000)), self = this, vx = 0, vy = 0;
    this.last = now;
    Object.keys(this.held).forEach(function (d) {
      var v = speedAt(now - self.held[d], self.speed) * self.H;
      vx += DIRS[d][0] * v; vy += DIRS[d][1] * v;
    });
    if (vx || vy) this.moveBy(vx * dt, vy * dt, now, false);
    return this.holding();
  };
  P.start = function () {
    if (this.running || !this.frame) return;
    var self = this;
    this.running = true;
    var loop = function (t) {
      if (!self.tick(t)) { self.running = false; self.sendMove(t, true); return; }
      self.frame(loop);
    };
    this.frame(loop);
  };
  P.moveBy = function (dx, dy, now, force) {
    var nx = this.x + dx, ny = this.y + dy;
    var cx = Math.max(0, Math.min(this.W - 1, nx)), cy = Math.max(0, Math.min(this.H - 1, ny));
    // Pushing past an edge scrolls the page by what didn't fit.
    this.scrollX += (nx - cx) / this.W; this.scrollY += (ny - cy) / this.H;
    this.x = cx; this.y = cy;
    this.view(this, 'move');
    this.sendMove(now, force);
  };
  P.sendMove = function (now, force) {
    if (!force && now - this.lastMove < MOVE_MS) return;
    this.lastMove = now;
    this.call('move', this.fx(), this.fy());
    if (this.scrollX || this.scrollY) {
      this.call('scroll', this.fx(), this.fy(), this.scrollX, this.scrollY);
      this.scrollX = 0; this.scrollY = 0;
    }
  };
  /** Every button let go (Hearth TV went to the background). */
  P.release = function () {
    this.held = {};
    if (this.okTimer !== null) this.cancel(this.okTimer);
    this.okTimer = null; this.okAt = 0; this.okLong = false;
  };

  P.ok = function (down, now) {
    if (this.mode !== 'pointer') return false;
    var self = this;
    if (down) {
      if (this.okAt) return true;
      this.okAt = now || 1; this.okLong = false;
      this.okTimer = this.later(function () {
        self.okTimer = null; self.okLong = true;
        if (self.menu === null) self.openMenu();
      }, LONG_MS);
      return true;
    }
    if (!this.okAt) return true;
    if (this.okTimer !== null) this.cancel(this.okTimer);
    var wasLong = this.okLong;
    this.okTimer = null; this.okAt = 0; this.okLong = false;
    if (wasLong) return true;                      // the menu just opened; OK again picks from it
    if (this.menu !== null) { this.pick(MENU[this.menu].id); return true; }
    this.click();
    return true;
  };
  P.click = function () { this.call('tap', this.fx(), this.fy()); this.view(this, 'click'); };
  P.openMenu = function () { this.held = {}; this.menuHeld = {}; this.menu = 0; this.view(this, 'menu'); };
  P.closeMenu = function () { this.menu = null; this.view(this, 'menu'); };
  P.pick = function (id) {
    this.closeMenu();
    if (id === 'long') { this.call('longPress', this.fx(), this.fy()); this.view(this, 'click'); }
    else if (id === 'right') { this.call('rightClick', this.fx(), this.fy()); this.view(this, 'click'); }
    else if (id === 'page') { this.call('setMode', 'page'); this.setMode('page'); }
  };
  /** Back: closes the menu first, otherwise the website goes back a page (ViewerActivity: or leaves full screen). */
  P.back = function () {
    if (this.menu !== null) { this.closeMenu(); return 'menu'; }
    this.call('back');
    return 'page';
  };
  P.setMode = function (m) {
    m = m === 'page' ? 'page' : 'pointer';
    this.release(); this.menu = null; this.mode = m;
    this.view(this, 'mode');
  };
  /** The scroll keys: [dy] -1 up, 1 down (half a screen); [dx] the same sideways. */
  P.scrollKey = function (dy, dx) {
    if (this.mode !== 'pointer') return false;
    this.call('scroll', this.fx(), this.fy(), (dx || 0) * 0.5, (dy || 0) * 0.5);
    return true;
  };

  // ---------- On the TV: the layer over the website (pointer.html) ----------
  function mount(doc, win) {
    var cfg = {};
    try { cfg = JSON.parse(decodeURIComponent((win.location.hash || '').slice(1)) || '{}') || {}; } catch (e) { cfg = {}; }
    var look = cfg.look || {}, css = doc.documentElement.style;
    if (look.accent) css.setProperty('--acc', look.accent);
    if (look.panel) css.setProperty('--panel', look.panel);
    if (look.text) css.setProperty('--text', look.text);
    if (look.font) css.setProperty('--font', look.font);
    var $ = function (id) { return doc.getElementById(id); };
    var ptr = $('hp-ptr'), ring = $('hp-ring'), hint = $('hp-hint'), menu = $('hp-menu');
    var hintTimer = null;
    function scale(p) { return p.H / 1080; }
    function showHint(p) {
      var h = hintText(p.mode), s = '<b class="t">' + h.title + '</b>';
      h.keys.forEach(function (k) { s += '<span class="k"><span class="kb">' + k[0] + '</span>' + k[1] + '</span>'; });
      if (p.mode === 'pointer') s += '<span class="k soft">☰ or a colour key: page mode</span>';
      hint.innerHTML = s;
      hint.className = 'on ' + p.mode;
      if (hintTimer) win.clearTimeout(hintTimer);
      hintTimer = win.setTimeout(function () { hint.className = p.mode; }, 7000);
    }
    function drawMenu(p) {
      if (p.menu === null) { menu.className = ''; menu.innerHTML = ''; return; }
      var s = '<span class="mt">At the pointer</span>';
      MENU.forEach(function (m, i) { s += '<span class="mi' + (i === p.menu ? ' on' : '') + '"><b>' + m.label + '</b>' + (m.why ? '<i>' + m.why + '</i>' : '') + '</span>'; });
      s += '<span class="mf">OK choose · ▲▼ move · ↩ close</span>';
      menu.innerHTML = s;
      menu.className = 'on';
      var k = scale(p), w = menu.offsetWidth || 520 * k, h = menu.offsetHeight || 360 * k, gap = 28 * k;
      var left = p.x + gap + w > p.W ? p.x - gap - w : p.x + gap;
      var top = Math.max(16 * k, Math.min(p.H - h - 16 * k, p.y - 20 * k));
      menu.style.left = Math.max(16 * k, left) + 'px'; menu.style.top = top + 'px';
    }
    function draw(p, what) {
      var k = scale(p), sz = SIZES[p.size] * k;
      ptr.style.width = ptr.style.height = sz + 'px';
      // The tip of the arrow (3,2 in its 24×24 drawing) is the pointer's place.
      ptr.style.transform = 'translate3d(' + (p.x - sz * 3 / 24) + 'px,' + (p.y - sz * 2 / 24) + 'px,0)';
      ptr.style.display = p.mode === 'pointer' ? 'block' : 'none';
      if (what === 'click') {
        ring.style.left = p.x + 'px'; ring.style.top = p.y + 'px';
        ring.className = ''; void ring.offsetWidth; ring.className = 'go';
      }
      if (what === 'menu' || what === 'mode') drawMenu(p);
      if (what === 'mode' || what === 'start') showHint(p);
    }
    var p = new Pointer({
      native: win.HearthPointerNative || null, width: win.innerWidth, height: win.innerHeight,
      speed: cfg.speed, size: cfg.size, startOn: cfg.startOn, view: draw,
      frame: win.requestAnimationFrame ? win.requestAnimationFrame.bind(win) : null
    });
    var now = function () { return win.performance.now(); };
    win.addEventListener('resize', function () { p.resize(win.innerWidth, win.innerHeight); });
    // What ViewerActivity calls (evaluateJavascript) for the remote's buttons.
    win.hp = {
      pointer: p,
      key: function (dir, down) { return p.key(dir, down, now()); },
      ok: function (down) { return p.ok(down, now()); },
      back: function () { return p.back(); },
      mode: function (m) { p.setMode(m); return p.mode; },
      scroll: function (dy, dx) { return p.scrollKey(dy, dx); },
      release: function () { p.release(); },
      place: function (fx, fy) { p.place(fx, fy, now()); }
    };
    draw(p, 'start');
    p.call('ready');
    return p;
  }

  if (typeof document !== 'undefined' && document.getElementById && document.getElementById('hp-root')) mount(document, window);

  return { Pointer: Pointer, speedAt: speedAt, hintText: hintText, SPEEDS: SPEEDS, SIZES: SIZES, MENU: MENU, LONG_MS: LONG_MS, MOVE_MS: MOVE_MS, BASE: BASE, MAXMUL: MAXMUL, NUDGE: NUDGE };
});
