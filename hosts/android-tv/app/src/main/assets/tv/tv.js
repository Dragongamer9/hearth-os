/* Hearth TV's page: the core (Android bridge, settings, focus with the remote, screens, looks, toasts).
   The screens are in their own files and register themselves with hearthTV.screen(name, {render, act, ...}). */
(function () {
  'use strict';
  var N = window.HearthTVNative || null;   // Android (Bridge.kt); null in a plain browser (CI's page check)
  var waiting = {}, seq = 0;
  var T = window.hearthTV = {
    N: N, screens: {}, cur: null, arg: null, stack: [], data: {}, toastText: '', layer: null,
    settings: {}, state: {}, profile: null
  };

  // ---------- Android ----------
  T.call = function (method) {                 // a slow one: a promise for hearthTV.done(id, result)
    var args = [].slice.call(arguments, 1);
    return new Promise(function (resolve) {
      if (!N || !N[method]) return resolve(null);
      var id = 'c' + (++seq); waiting[id] = resolve;
      try { N[method].apply(N, [id].concat(args)); } catch (e) { delete waiting[id]; resolve(null); }
    });
  };
  T.done = function (id, r) { var f = waiting[id]; delete waiting[id]; if (f) f(r); };
  T.n = function (method) {                    // a quick one: the answer now
    var args = [].slice.call(arguments, 1);
    if (!N || !N[method]) return null;
    try { return N[method].apply(N, args); } catch (e) { return null; }
  };
  T.json = function (method) { var r = T.n.apply(null, arguments); try { return r ? JSON.parse(r) : null; } catch (e) { return null; } };
  T.log = function (s) { T.n('log', String(s)); };
  window.addEventListener('error', function (e) { T.log('error: ' + e.message + ' @' + (e.filename || '').split('/').pop() + ':' + e.lineno); });

  // ---------- Settings (red flag 9: only changed keys go to Android, which merges them into the file) ----------
  var DEFAULTS = {
    setupDone: false, look: 'steam', sub: '', mode: 'dark', accent: '', wallpaper: '',
    home: { hero: true, rotate: 7, rowApps: true, rowKeep: true, rowNew: true, rowTheater: false },
    homeApps: null,                // package names in order; null = the first-start choice
    cleanup: { watch: true },
    inputs: { names: {}, hidden: {}, button: true, autoSwitch: true, backHome: true, startOn: 'hearth' },
    profiles: [{ id: 'p1', name: 'Me', color: 'steam', kids: false }], pin: '', pinOn: false,
    media: { app: 'jellyfin', quality: 'original', autoQuality: true, subs: 'off', wake: true, libraries: {} },
    link: { sendTo: true, friends: true },
    saver: { after: 10, pictures: 'media', clock: true },
    player: { frame: true, hdr: 'auto', night: false, dialog: false, out: 'tv', subSize: 'm' },
    updates: { auto: false }
  };
  T.DEFAULTS = DEFAULTS;
  function filled(s) {             // defaults per key, filled in when read; never written by reading
    var o = JSON.parse(JSON.stringify(DEFAULTS));
    Object.keys(s || {}).forEach(function (k) {
      if (s[k] && typeof s[k] === 'object' && !Array.isArray(s[k]) && o[k] && typeof o[k] === 'object' && !Array.isArray(o[k])) {
        Object.keys(s[k]).forEach(function (j) { o[k][j] = s[k][j]; });
      } else o[k] = s[k];
    });
    return o;
  }
  T.loadSettings = function () {
    var raw = T.json('settings') || (function () { try { return JSON.parse(localStorage.getItem('hearthtv.settings') || '{}'); } catch (e) { return {}; } })();
    T.raw = raw; T.settings = filled(raw);
    T.state = T.json('state') || {};
    var ev = T.n('settingsEvent'); if (ev) setTimeout(function () { T.toast(ev); }, 1500);
  };
  /** Changes one setting (a whole top-level key, or key.sub for one value inside it). */
  T.set = function (path, value) {
    var parts = path.split('.'), top = parts[0], patch = {};
    if (parts.length === 1) { patch[top] = value; }
    else {
      var obj = JSON.parse(JSON.stringify((T.raw && T.raw[top]) || {}));
      obj[parts[1]] = value; patch[top] = obj;
    }
    var merged = T.n('settingsMerge', JSON.stringify(patch));
    if (merged) { try { T.raw = JSON.parse(merged); } catch (e) {} }
    else { T.raw = T.raw || {}; T.raw[top] = patch[top]; try { localStorage.setItem('hearthtv.settings', JSON.stringify(T.raw)); } catch (e) {} }
    T.settings = filled(T.raw);
  };
  T.setState = function (patch) { var r = T.n('stateMerge', JSON.stringify(patch)); if (r) try { T.state = JSON.parse(r); } catch (e) {} else Object.assign(T.state, patch); };

  // ---------- Looks (Hearth OS's palettes.json: every look, sub-theme, Black set, accent; Gold when it lands) ----------
  T.palettes = null;
  var ANIM = { tide: 'slate-horizon', zen: 'slate-dusk', glass: 'slate-facets', synth: 'slate-nebula', neon: 'slate-contours' };
  T.wallpaperUrl = function (w) {
    if (!w) return '';
    if (w.indexOf('builtin:') === 0) return 'theme/wallpapers/' + w.slice(8) + '.jpg';
    if (w.indexOf('anim:') === 0) return 'theme/wallpapers/' + (ANIM[w.slice(5)] || 'slate-horizon') + '.jpg';
    if (w.indexOf('/usr/share/backgrounds/hearth/') === 0) return 'theme/backgrounds/' + w.split('/').pop();
    if (w.indexOf('tv:') === 0) return w.slice(3);
    return '';
  };
  T.looks = function () {
    var p = T.palettes || {}, out = [];
    Object.keys(p).forEach(function (k) { if (p[k] && p[k].label && p[k].dark) out.push({ id: k, label: p[k].label, desc: p[k].desc || '', order: p[k].order || 99, subs: p[k].subThemes || [] }); });
    out.sort(function (a, b) { return a.order - b.order; });
    return out;
  };
  T.applyLook = function () {
    var s = T.settings, p = T.palettes, root = document.documentElement.style;
    if (!p) return;
    var look = p[s.look] || p.steam || p.hearth;
    var sub = null;
    (look.subThemes || []).forEach(function (x) { if (x.id === s.sub) sub = x; });
    (p.blackSet || []).forEach(function (x) { if (x.id === s.sub) sub = x; });
    var mode = s.mode === 'light' ? 'light' : 'dark';
    var c = Object.assign({}, look[mode] || look.dark, sub ? (sub[mode] || sub.dark) : {});
    var map = { bg: '--bg', bg2: '--bg2', panel: '--panel', panel2: '--panel2', panel3: '--panel3', line: '--line', text: '--text', muted: '--muted', ember: '--acc', flame: '--focus', ink: '--ink', danger: '--danger' };
    Object.keys(map).forEach(function (k) { if (c[k]) root.setProperty(map[k], c[k]); });
    if (s.accent) { root.setProperty('--acc', s.accent); root.setProperty('--focus', s.accent); }
    root.setProperty('--soft', c.text ? 'color-mix(in srgb, ' + c.text + ' 82%, ' + (c.muted || c.text) + ')' : '#c3ccd4');
    root.setProperty('--r', Math.min(look.corner || 4, 12) + 'px');
    if (look.font) root.setProperty('--font', "'" + look.font + "', Inter, 'Noto Sans', Roboto, sans-serif");
    T.lookWallpaper = T.wallpaperUrl(s.wallpaper || (sub && sub.wallpaper) || look.wallpaper);
  };
  T.lookName = function () {
    var p = T.palettes || {}, l = p[T.settings.look];
    if (!l) return 'Steam-style';
    var sub = null; (l.subThemes || []).concat(p.blackSet || []).forEach(function (x) { if (x.id === T.settings.sub) sub = x; });
    return l.label + (sub ? ' · ' + sub.label : '');
  };

  // ---------- Profiles ----------
  T.profiles = function () { return T.settings.profiles && T.settings.profiles.length ? T.settings.profiles : DEFAULTS.profiles; };
  T.currentProfile = function () {
    var id = T.state.profile, ps = T.profiles(), p = null;
    ps.forEach(function (x) { if (x.id === id) p = x; });
    return p || ps[0];
  };
  T.kids = function () { return !!T.currentProfile().kids; };
  T.PROFILE_COLORS = ['#2a475e', '#5b3a5e', '#3d4f66', '#1f4a36', '#5a1f22', '#4a3b1d'];

  // ---------- Small helpers ----------
  T.esc = function (s) { return String(s == null ? '' : s).replace(/[&<>"']/g, function (c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]; }); };
  T.ICON = {
    flame: '<svg width="34" height="34" viewBox="0 0 24 24" fill="none" stroke="var(--focus)" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 3c1 3.5 5 5.5 5 10a5 5 0 0 1-10 0c0-2.2 1.2-3.6 2.4-4.8C10.6 7 11.6 5.5 12 3z"/><path d="M12 21a2.2 2.2 0 0 1-2.2-2.2c0-1.4 1.2-2.2 2.2-3.6 1 1.4 2.2 2.2 2.2 3.6A2.2 2.2 0 0 1 12 21z"/></svg>',
    search: '<svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="M20 20l-3.5-3.5"/></svg>',
    gear: '<svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/></svg>',
    inputs: '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3" y="5" width="18" height="12" rx="2"/><path d="M8 21h8M10 11h4M12 9l2 2-2 2"/></svg>',
    play: '<svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M7 4.5v15l13-7.5z"/></svg>',
    pause: '<svg width="30" height="30" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M6 4h4v16H6zM14 4h4v16h-4z"/></svg>',
    list: '<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h10"/></svg>',
    star: function (fill) { return '<svg width="22" height="22" viewBox="0 0 24 24" fill="' + (fill ? 'currentColor' : 'none') + '" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 17.3 6.2 20.5l1.1-6.5L2.6 9.4l6.5-.9L12 2.6l2.9 5.9 6.5.9-4.7 4.6 1.1 6.5z"/></svg>'; },
    restart: '<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 12a9 9 0 1 0 3-6.7"/><path d="M3 4v5h5"/></svg>',
    subs: '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><rect x="3" y="5" width="18" height="14" rx="2"/><path d="M7 13h4M13 13h4M7 16h7"/></svg>',
    check: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--ok)" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5"/></svg>',
    pc: '<svg width="30" height="30" viewBox="0 0 24 24" fill="none" stroke="var(--focus)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/></svg>',
    phone: '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="var(--focus)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="7" y="2" width="10" height="20" rx="2"/><path d="M11 18h2"/></svg>',
    mic: '<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3"/></svg>'
  };
  T.clockText = function () { var d = new Date(), h = d.getHours(), m = d.getMinutes(); return ((h % 12) || 12) + ':' + (m < 10 ? '0' : '') + m + ' ' + (h < 12 ? 'AM' : 'PM'); };
  T.mins = function (m) { m = Math.round(m); return m >= 60 ? Math.floor(m / 60) + ' h ' + (m % 60) + ' min' : m + ' min'; };

  /** The header of Home, Apps, My Media and Theater. */
  T.header = function (active) {
    var tabs = [['Home', 'home'], ['Apps', 'apps'], ['My Media', 'media'], ['Theater', 'theater']];
    var p = T.currentProfile(), link = T.data.linkPeer;
    var h = '<header class="top"><span class="brand">' + T.ICON.flame + '<span>Hearth TV</span></span>';
    tabs.forEach(function (t, i) { h += '<button class="tab' + (t[1] === active ? ' on' : '') + '" data-f data-key="tab' + i + '" data-act="go" data-arg="' + t[1] + '">' + t[0] + '</button>'; });
    h += '<button class="iconbtn" aria-label="Search" data-f data-key="search" data-act="go" data-arg="search">' + T.ICON.search + '</button>';
    if (T.settings.inputs.button !== false && active === 'home') h += '<button class="iconbtn" style="width:auto;padding:0 18px;gap:10px;font-size:19px;font-weight:500" data-f data-key="inputsBtn" data-act="inputs">' + T.ICON.inputs + 'Inputs</button>';
    h += '<div style="flex:1"></div>';
    h += '<span style="display:flex;align-items:center;gap:8px;font-size:17px;color:var(--muted)"><span class="dot' + (link && link.online ? '' : ' off') + '"></span>' + T.esc(link ? 'Hearth Link · ' + link.name : (T.data.mediaName ? 'My Media · ' + T.data.mediaName : 'Hearth Link · not paired')) + '</span>';
    h += '<span class="clock" data-clock>' + T.clockText() + '</span>';
    if (!T.kids()) h += '<button class="iconbtn" aria-label="Settings" data-f data-key="gear" data-act="go" data-arg="settings">' + T.ICON.gear + '</button>';
    h += '<button class="avatar" aria-label="Profile" data-f data-key="profile" data-act="profiles" style="background:' + T.esc(p.color && p.color[0] === '#' ? p.color : T.PROFILE_COLORS[0]) + '">' + T.esc((p.name || '?').slice(0, 1).toUpperCase()) + '</button>';
    return h + '</header>';
  };
  T.footer = function (keys, toastDefault) {
    var f = '<footer class="keys">';
    keys.forEach(function (k) {
      if (k[2]) f += '<button class="hint" data-f data-key="fk' + k[0] + '" data-act="' + k[2] + '"' + (k[3] ? ' data-arg="' + k[3] + '"' : '') + '><span class="k">' + k[0] + '</span>' + k[1] + '</button>';
      else f += '<span class="hint"><span class="k">' + k[0] + '</span>' + k[1] + '</span>';
    });
    return f + '<span class="toast" data-toast>' + T.esc(T.toastText || toastDefault || '') + '</span></footer>';
  };
  T.toast = function (text) {
    T.toastText = text;
    var el = document.querySelector('[data-toast]'); if (el) el.textContent = text;
    clearTimeout(T._tt); T._tt = setTimeout(function () { T.toastText = ''; var e = document.querySelector('[data-toast]'); if (e && e.textContent === text) e.textContent = (T.screens[T.cur] && T.screens[T.cur].toast) || ''; }, 6000);
  };

  // ---------- Screens ----------
  T.screen = function (name, def) { T.screens[name] = def; };
  T.go = function (name, arg, opts) {
    if (!T.screens[name]) return;
    if (name === 'settings' && T.kids()) { T.toast('Settings are off in a kids profile'); return; }
    if (name === 'settings' && T.settings.pinOn && T.settings.pin && !T._pinOk) { T.askPin(function () { T._pinOk = true; T.go(name, arg, opts); }); return; }
    if (T.cur && !(opts && opts.replace)) T.stack.push({ name: T.cur, arg: T.arg, key: T.focusKey() });
    if (T.cur && T.screens[T.cur].leave) T.screens[T.cur].leave();
    T.cur = name; T.arg = arg; T.layer = null; T.toastText = '';
    var s = T.screens[name];
    if (s.enter) s.enter(arg);
    T.render(opts && opts.focus);
  };
  T.back = function () {
    if (T.layer) { var l = T.layer; T.layer = null; if (l.onClose) l.onClose(); T.render(); return; }
    var s = T.screens[T.cur];
    if (s && s.back && s.back()) return;
    var prev = T.stack.pop();
    if (!prev) { if (T.cur !== 'home') T.go('home', null, { replace: true }); return; }
    if (s && s.leave) s.leave();
    T.cur = prev.name; T.arg = prev.arg; T.toastText = '';
    if (T.screens[T.cur].enter) T.screens[T.cur].enter(prev.arg, true);
    T.render(prev.key);
  };
  T.home = function () { T.layer = null; T.stack = []; T._pinOk = false; if (T.cur !== 'home') T.go('home', null, { replace: true }); else { T.render('resume'); } };

  var app, layerEl;
  T.render = function (focusKey) {
    var want = focusKey || T.focusKey();
    var s = T.screens[T.cur];
    app.innerHTML = s ? s.render(T.arg) : '';
    layerEl.innerHTML = T.layer ? T.layer.render() : '';
    if (s && s.after) s.after();
    if (T.layer && T.layer.after) T.layer.after();
    T.focus(want) || T.focus((T.layer && T.layer.first) || (s && s.first)) || T.focusFirst();
  };
  /** Re-draws only the pop-up layer (dialogs over a screen). */
  T.renderLayer = function (focusKey) {
    var want = focusKey || T.focusKey();
    layerEl.innerHTML = T.layer ? T.layer.render() : '';
    if (T.layer && T.layer.after) T.layer.after();
    T.focus(want) || T.focus(T.layer && T.layer.first) || T.focusFirst();
  };
  T.open = function (layer) { T.layer = layer; T.renderLayer(layer.first); };
  T.close = function () { if (T.layer) { var l = T.layer; T.layer = null; if (l.onClose) l.onClose(); layerEl.innerHTML = ''; T.focus(T._beforeLayer) || T.focusFirst(); } };

  // ---------- Focus with the remote ----------
  var focused = null;
  function scope() { return T.layer ? layerEl : app; }
  function candidates() { return [].slice.call(scope().querySelectorAll('[data-f]')).filter(function (e) { return e.offsetParent !== null || e === document.activeElement; }); }
  T.focusKey = function () { return focused && focused.isConnected ? focused.getAttribute('data-key') : null; };
  T.focusEl = function (el) {
    if (!el) return false;
    if (focused && focused !== el) focused.classList.remove('focused');
    focused = el; el.classList.add('focused');
    if (!T.layer) T._beforeLayer = el.getAttribute('data-key');
    // Rows scroll sideways to keep the focused card in view; pages scroll up and down.
    var row = el.closest('.row-scroll');
    if (row) { var r = el.getBoundingClientRect(), rr = row.getBoundingClientRect(), sc = stageScale(); if (r.right > rr.right - 40 * sc) row.scrollLeft += (r.right - rr.right) / sc + 80; if (r.left < rr.left + 10 * sc) row.scrollLeft -= (rr.left - r.left) / sc + 80; }
    var col = el.closest('[data-scrolly]');
    if (col) { var a = el.getBoundingClientRect(), c = col.getBoundingClientRect(), k = stageScale(); if (a.bottom > c.bottom - 20 * k) col.scrollTop += (a.bottom - c.bottom) / k + 60; if (a.top < c.top + 20 * k) col.scrollTop -= (c.top - a.top) / k + 60; }
    var h = el.getAttribute('data-hover'); if (h && T.hover) T.hover(h, el);
    if (el.tagName === 'INPUT') el.focus(); else if (document.activeElement && document.activeElement.tagName === 'INPUT') document.activeElement.blur();
    return true;
  };
  T.focus = function (key) {
    if (!key) return false;
    var el = scope().querySelector('[data-key="' + String(key).replace(/"/g, '') + '"]');
    return el && el.offsetParent !== null ? T.focusEl(el) : false;
  };
  T.focusFirst = function () { var c = candidates(); return c.length ? T.focusEl(c[0]) : false; };
  function stageScale() { var st = document.getElementById('stage'); return st.getBoundingClientRect().width / 1920 || 1; }
  function move(dir) {
    var c = candidates(); if (!focused || !focused.isConnected || c.indexOf(focused) < 0) return T.focusFirst();
    var a = focused.getBoundingClientRect(), ax = (a.left + a.right) / 2, ay = (a.top + a.bottom) / 2, best = null, bd = 1e12;
    c.forEach(function (e) {
      if (e === focused) return;
      var b = e.getBoundingClientRect(), bx = (b.left + b.right) / 2, by = (b.top + b.bottom) / 2, main, cross;
      if (dir === 'left') { if (b.right > a.left + 2) return; main = a.left - b.right; cross = Math.abs(by - ay); }
      else if (dir === 'right') { if (b.left < a.right - 2) return; main = b.left - a.right; cross = Math.abs(by - ay); }
      else if (dir === 'up') { if (b.bottom > a.top + 2) return; main = a.top - b.bottom; cross = Math.abs(bx - ax); }
      else { if (b.top < a.bottom - 2) return; main = b.top - a.bottom; cross = Math.abs(bx - ax); }
      // Overlapping on the other axis counts as lined up.
      var overlap = (dir === 'left' || dir === 'right') ? Math.max(0, Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top)) : Math.max(0, Math.min(a.right, b.right) - Math.max(a.left, b.left));
      var d = Math.max(main, 0) + (overlap > 0 ? 0 : cross * 2.5) + cross * 0.3;
      if (d < bd) { bd = d; best = e; }
    });
    if (best) T.focusEl(best);
    return !!best;
  }
  function activate(el) {
    if (!el) return;
    var act = el.getAttribute('data-act'), arg = el.getAttribute('data-arg');
    if (el.tagName === 'INPUT') { el.focus(); return; }
    if (!act) return;
    T.idle();
    if (T.layer && T.layer.act && T.layer.act(act, arg, el) !== false) return;
    var s = T.screens[T.cur];
    if (s && s.act && s.act(act, arg, el) !== false) return;
    T.act(act, arg, el);
  }
  /** Actions every screen has. */
  T.act = function (act, arg) {
    if (act === 'go') { if (arg === 'home') T.home(); else T.go(arg); }
    else if (act === 'back') T.back();
    else if (act === 'inputs') T.showInputs();
    else if (act === 'profiles') T.showProfiles();
    else if (act === 'screensaver') T.go('screensaver');
    else if (act === 'close') T.close();
  };
  document.addEventListener('keydown', function (e) {
    T.idle();
    if (T.cur === 'screensaver') { e.preventDefault(); T.back(); return; }
    var inInput = document.activeElement && document.activeElement.tagName === 'INPUT';
    var s = T.screens[T.cur];
    if (T.layer && T.layer.key && T.layer.key(e.key, e) === true) { e.preventDefault(); return; }
    if (!T.layer && s && s.key && s.key(e.key, e) === true) { e.preventDefault(); return; }
    var dirs = { ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down' };
    if (dirs[e.key]) {
      if (inInput && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) return;
      e.preventDefault(); move(dirs[e.key]);
    } else if (e.key === 'Enter' || e.key === ' ') {
      if (inInput) { var sub = focused && focused.getAttribute('data-enter'); if (sub) { e.preventDefault(); activate(scope().querySelector('[data-key="' + sub + '"]')); } return; }
      e.preventDefault(); activate(focused);
    } else if (e.key === 'Escape' || e.key === 'Backspace' && !inInput) { e.preventDefault(); T.back(); }
  });
  document.addEventListener('click', function (e) { var el = e.target.closest('[data-f]'); if (el) { T.focusEl(el); activate(el); } });
  document.addEventListener('mouseover', function (e) { var el = e.target.closest('[data-f]'); if (el && el !== focused) T.focusEl(el); });

  // ---------- Events from Android ----------
  T.onEvent = function (json) {
    var ev; try { ev = JSON.parse(json); } catch (e) { return; }
    T.idle();
    if (ev.type === 'key') {
      if (T.cur === 'screensaver') { T.back(); return; }
      var s = T.screens[T.cur];
      if (T.layer && T.layer.key && T.layer.key(ev.key) === true) return;
      if (!T.layer && s && s.key && s.key(ev.key) === true) return;
      if (ev.key === 'back') T.back();
      else if (ev.key === 'search') T.go('search');
      else if (ev.key === 'inputs') T.showInputs();
      else if (ev.key === 'menu' && focused) { var m = focused.getAttribute('data-menu'); if (m && s && s.menu) s.menu(m, focused); }
    } else if (ev.type === 'home') { if (T.cur === 'screensaver' || T.cur !== 'home' || T.layer) T.home(); }
    else if (ev.type === 'resume') { T.loadSettings(); T.applyLook(); if (T.screens[T.cur] && T.screens[T.cur].resume) T.screens[T.cur].resume(); }
    else if (ev.type === 'toast') T.toast(ev.text);
    else if (ev.type === 'voice') { if (T.screens.search.voice) T.screens.search.voice(ev.text); }
    else if (ev.type === 'theaterAdded') { T.toast('Saved in Theater: ' + (ev.item && ev.item.title || 'a link')); if (T.screens.theater.added) T.screens.theater.added(ev.item); }
    else if (ev.type === 'inputs') { if (T.layer && T.layer.id === 'inputs') T.showInputs(true); }
    else if (ev.type === 'cleanupFixed') T.toast('TV Cleanup: an update turned something back on, so it was switched off again (' + ev.groups.join(', ') + ')');
    else if (ev.type === 'updateProgress') { var u = document.querySelector('[data-update]'); if (u) u.textContent = 'Downloading… ' + ev.pct + '%'; }
  };

  // ---------- TV inputs (Home's Inputs button, the remote's Input button) ----------
  T.inputList = function () {
    var list = T.json('inputs') || [], s = T.settings.inputs, out = [];
    list.forEach(function (x) {
      if (s.hidden && s.hidden[x.id]) return;
      var port = x.custom ? x.label : x.label;
      var name = (s.names && s.names[x.id]) || x.custom || (x.kind === 'tuner' ? 'Live TV' : x.label);
      out.push({ id: x.id, kind: x.kind, name: name, port: x.kind === 'tuner' ? 'Antenna' : port, state: x.state });
    });
    return out;
  };
  T.showInputs = function (refresh) {
    var ins = T.inputList();
    if (!T.N) ins = [{ id: 'h1', kind: 'hdmi', name: 'HDMI 1', port: 'HDMI 1', state: 'signal' }, { id: 'h2', kind: 'hdmi', name: 'HDMI 2', port: 'HDMI 2', state: 'none' }, { id: 'tv', kind: 'tuner', name: 'Live TV', port: 'Antenna', state: 'signal' }];
    var layer = {
      id: 'inputs', first: 'in0',
      render: function () {
        var h = '<div class="dim"><div class="dialog on-panel" role="dialog" aria-label="Switch input" style="width:1500px;gap:24px"><div style="display:flex;align-items:baseline;gap:18px"><span style="font-size:36px;font-weight:700">Switch input</span><span class="muted" style="font-size:18px">Press Home on the remote any time to come back to Hearth TV</span></div>';
        h += '<div style="display:grid;grid-template-columns:repeat(5,minmax(0,1fr));gap:18px">';
        if (!ins.length) h += '<span class="muted" style="font-size:20px;grid-column:1/-1">This TV didn\'t list any inputs.</span>';
        ins.forEach(function (x, i) {
          var on = x.state === 'signal', short = x.kind === 'tuner' ? 'TV' : x.kind === 'hdmi' ? 'H' + ((x.port.match(/\d/) || [''])[0]) : x.kind === 'av' ? 'AV' : '•';
          var sig = x.kind === 'tuner' ? 'Channels' : on ? 'Signal' : x.state === 'standby' ? 'Standby' : 'No signal';
          h += '<button class="tile" data-f data-key="in' + i + '" data-act="input" data-arg="' + T.esc(x.id) + '" style="height:230px;border-radius:6px;background:var(--panel2);flex-direction:column;align-items:flex-start;justify-content:flex-end;gap:6px;padding:22px;box-sizing:border-box;text-align:left;color:var(--text)">' +
            '<span style="width:56px;height:56px;border-radius:28px;background:' + (on ? 'var(--panel3)' : 'var(--line)') + ';display:flex;align-items:center;justify-content:center;font-size:15px;font-weight:700">' + short + '</span>' +
            '<span style="font-size:22px;font-weight:700">' + T.esc(x.name) + '</span><span class="muted" style="font-size:16px;font-weight:400">' + T.esc(x.port) + '</span>' +
            '<span style="display:flex;align-items:center;gap:8px;font-size:16px;font-weight:600;color:' + (on ? 'var(--ok)' : 'var(--muted)') + '"><span class="dot' + (on ? '' : ' off') + '" style="width:9px;height:9px"></span>' + sig + '</span></button>';
        });
        h += '</div><div style="display:flex;align-items:center;gap:16px"><span class="muted" style="flex:1;font-size:17px">Rename inputs, hide the ones you don\'t use, and turn on automatic switching in Settings › TV inputs.</span>' +
          (T.kids() ? '' : '<button class="btn small" data-f data-key="inSettings" data-act="inSettings">Input settings</button>') + '<button class="btn small" data-f data-key="inBack" data-act="close">↩ Back</button></div></div></div>';
        return h;
      },
      act: function (act, arg) {
        if (act === 'input') { T.close(); if (T.n('inputSwitch', arg) === false) T.toast('The TV wouldn\'t switch to that input'); return true; }
        if (act === 'inSettings') { T.close(); T.go('settings', 'TV inputs'); return true; }
        return false;
      }
    };
    if (refresh && T.layer && T.layer.id === 'inputs') { T.layer = layer; T.renderLayer(); } else T.open(layer);
  };

  // ---------- Who's watching ----------
  T.showProfiles = function () {
    var ps = T.profiles();
    T.open({
      id: 'profiles', first: 'pp0',
      render: function () {
        var h = '<div class="dim" style="background:rgba(10,12,16,.9);flex-direction:column;gap:48px"><span style="font-size:44px;font-weight:700">Who\'s watching?</span><div style="display:flex;gap:40px">';
        ps.forEach(function (p, i) {
          h += '<button data-f data-key="pp' + i + '" data-act="pick" data-arg="' + T.esc(p.id) + '" style="display:flex;flex-direction:column;align-items:center;gap:16px;border-radius:80px"><span style="width:160px;height:160px;border-radius:80px;background:' + T.esc(p.color && p.color[0] === '#' ? p.color : T.PROFILE_COLORS[i % 6]) + ';display:flex;align-items:center;justify-content:center;font-size:60px;font-weight:700">' + T.esc((p.name || '?').slice(0, 1).toUpperCase()) + '</span><span style="font-size:22px;font-weight:600">' + T.esc(p.name) + (p.kids ? ' (kids)' : '') + '</span></button>';
        });
        if (!T.kids()) h += '<button data-f data-key="ppAdd" data-act="add" style="display:flex;flex-direction:column;align-items:center;gap:16px;border-radius:80px"><span style="width:160px;height:160px;border-radius:80px;background:var(--panel2);display:flex;align-items:center;justify-content:center;font-size:60px;font-weight:700">+</span><span style="font-size:22px;font-weight:600">Add profile</span></button>';
        return h + '</div><span class="muted" style="font-size:17px">Each profile keeps its own Keep watching row, pinned shows and apps</span></div>';
      },
      act: function (act, arg) {
        if (act === 'pick') {
          var p = null; ps.forEach(function (x) { if (x.id === arg) p = x; });
          var doPick = function () { T.setState({ profile: arg }); T.close(); T._pinOk = false; T.toast('Watching as ' + p.name); if (T.screens.home.refresh) T.screens.home.refresh(); T.render(); };
          // Leaving a kids profile asks for the PIN when there is one.
          if (T.kids() && !p.kids && T.settings.pin) T.askPin(doPick); else doPick();
          return true;
        }
        if (act === 'add') { T.close(); T.go('settings', 'Profiles'); return true; }
        return false;
      }
    });
  };

  /** The 4-digit PIN (Settings and leaving a kids profile). */
  T.askPin = function (ok, setting) {
    var typed = '';
    T.open({
      id: 'pin', first: 'pk1',
      render: function () {
        var h = '<div class="dim"><div class="dialog on-panel" style="width:620px;align-items:center;text-align:center"><span style="font-size:32px;font-weight:700">' + (setting ? 'Choose a 4-digit PIN' : 'Type your PIN') + '</span>';
        h += '<span style="font-size:60px;letter-spacing:.3em;font-weight:700;color:var(--focus)">' + '●●●●'.slice(0, typed.length) + '<span class="muted">' + '○○○○'.slice(typed.length) + '</span></span><div style="display:grid;grid-template-columns:repeat(3,110px);gap:12px">';
        ['1', '2', '3', '4', '5', '6', '7', '8', '9', '⌫', '0', '✕'].forEach(function (d) { h += '<button class="key" data-f data-key="pk' + d + '" data-act="d" data-arg="' + d + '">' + d + '</button>'; });
        return h + '</div></div></div>';
      },
      act: function (act, d) {
        if (d === '✕') { T.close(); return true; }
        if (d === '⌫') typed = typed.slice(0, -1); else if (typed.length < 4) typed += d;
        if (typed.length === 4) {
          if (setting) { T.close(); ok(typed); return true; }
          if (typed === T.settings.pin) { T.close(); ok(); return true; }
          typed = ''; T.toast('That PIN isn\'t right');
        }
        T.renderLayer(); return true;
      },
      key: function (k) { if (/^[0-9]$/.test(k)) { this.act('d', k); return true; } return false; }
    });
  };

  // ---------- Screensaver after a while with nothing happening ----------
  T.idle = function () {
    clearTimeout(T._idle);
    var m = Number(T.settings.saver && T.settings.saver.after) || 0;
    if (!m || T.playing) return;
    T._idle = setTimeout(function () { if (!T.playing && T.cur !== 'screensaver' && T.cur !== 'setup') T.go('screensaver'); }, m * 60000);
  };

  // ---------- Start ----------
  // The 1920×1080 stage scaled to whatever the TV's WebView reports (often 960×540 at 2×, or 1280×720).
  function fit() {
    var w = window.innerWidth || document.documentElement.clientWidth, h = window.innerHeight || document.documentElement.clientHeight;
    if (!w || !h) { setTimeout(fit, 100); return; }
    var st = document.getElementById('stage'), k = Math.min(w / 1920, h / 1080);
    st.style.transform = 'scale(' + k + ')';
    st.style.left = ((w - 1920 * k) / 2) + 'px'; st.style.top = ((h - 1080 * k) / 2) + 'px';
  }
  T.start = function () {
    app = document.getElementById('app'); layerEl = document.getElementById('layer');
    fit(); window.addEventListener('resize', fit); setTimeout(fit, 500);
    T.loadSettings();
    fetch('theme/palettes.json').then(function (r) { return r.json(); }).then(function (p) { T.palettes = p; T.applyLook(); if (T.cur) T.render(); }).catch(function () {});
    setInterval(function () { [].forEach.call(document.querySelectorAll('[data-clock]'), function (e) { e.textContent = T.clockText(); }); }, 15000);
    if (location.hash === '#screensaver') { T.dream = true; T.go('screensaver'); return; }
    if (T.Media && T.Media.init) T.Media.init();
    if (T.linkInit) T.linkInit();
    if (!T.settings.setupDone && T.N) T.go('setup'); else T.go('home');
    T.idle();
  };

  // Hearth Link's status for the header (your PC online or not), every 20 s.
  T.linkInit = function () {
    if (!T.n('linkAvailable')) return;
    var tick = function () {
      T.call('link', 'status', '{}').then(function (r) {
        var d = r && r.ok ? r.data : null, peers = (d && d.peers) || [], best = null;
        peers.forEach(function (p) { if (!p.friend && (!best || (p.online && !best.online))) best = p; });
        T.data.link = d; T.data.linkPeer = best;
      });
    };
    tick(); setInterval(tick, 20000);
  };
})();
