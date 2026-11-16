/* Home (mockup 2 · Home): the featured banner, Your apps, Keep watching, Recently added, Theater; Inputs and
   Who's watching from the header. */
(function () {
  'use strict';
  var T = window.hearthTV, E = T.esc;
  // The colours of the mockup's name tiles, by app.
  var TILE = {
    'com.netflix.ninja': '#5a1f22', 'com.netflix.mediaclient': '#5a1f22', 'com.google.android.youtube.tv': '#5c2424', 'com.google.android.youtube.tvkids': '#5c2424',
    'com.disney.disneyplus': '#1f3160', 'com.amazon.amazonvideo.livingroom': '#1d3d55', 'com.hulu.livingroomplus': '#1f4a36', 'com.spotify.tv.android': '#1d4430',
    'com.google.android.youtube.tvmusic': '#4a2020', 'org.jellyfin.androidtv': '#2b2f5a', 'com.plexapp.android': '#4a3b1d', 'org.videolan.vlc': '#4d2f16',
    'com.wbd.stream': '#2a2f6a', 'com.hbo.hbonow': '#2a2f6a', 'com.peacocktv.peacockandroid': '#1c1c2a', 'com.cbs.ott': '#1b2f60', 'tv.pluto.android': '#3a3a12',
    'com.apple.atve.androidtv.appletv': '#202020', 'com.crunchyroll.crunchyroid': '#5a3010', 'com.valvesoftware.steamlink': '#1b2838', 'com.google.stadia.androidtv': '#3a1f3a'
  };
  T.tileColor = function (pkg) { return TILE[pkg] || 'var(--panel2)'; };
  /** What goes inside an app's tile: the app's own banner from the TV, or its icon with the name, or just the name. */
  T.tileInner = function (a, name, size) {
    if (!a || !a.pkg || !T.N) return name;
    var src = '/appicon/' + encodeURIComponent(a.pkg);
    if (a.art === 'banner') return '<img class="tile-art" src="' + src + '" alt="" onerror="this.remove()"><span class="tile-name">' + name + '</span>';
    return '<span style="display:flex;flex-direction:column;align-items:center;gap:6px"><img src="' + src + '" alt="" style="width:' + (size || 44) + 'px;height:' + (size || 44) + 'px;object-fit:contain" onerror="this.remove()">' + name + '</span>';
  };
  var DEFAULT_HOME = ['com.netflix.ninja', 'com.google.android.youtube.tv', 'com.disney.disneyplus', 'com.amazon.amazonvideo.livingroom', 'com.hulu.livingroomplus', 'com.spotify.tv.android'];

  var H = { hero: 0, keep: [], latest: [], heroes: [], apps: [], loading: false, first: 'resume', toast: 'TV remote or controller · the Home button always comes back here' };
  T.homeApps = function () {
    var all = T.json('apps') || [], by = {};
    all.forEach(function (a) { by[a.pkg] = a; });
    var list = T.settings.homeApps || DEFAULT_HOME;
    var p = T.currentProfile();
    if (p.apps && p.apps.length) list = p.apps;
    return { all: all, home: list.filter(function (k) { return by[k]; }).map(function (k) { return by[k]; }) };
  };
  function pins() { var all = T.settings.pins || {}; return all[T.currentProfile().id] || []; }
  function setPins(list) { var all = Object.assign({}, T.settings.pins || {}); all[T.currentProfile().id] = list; T.set('pins', all); }
  function isPinned(it) { return pins().some(function (p) { return p.id === it.id && p.src === it.src; }); }

  function theaterProgress() {
    return (T.json('theater') || []).filter(function (x) { return x.pct > 0 && x.pct < 97; }).slice(0, 4).map(function (x) {
      return { src: 'theater', id: x.id, kind: 'link', title: x.title, sub: 'Saved link · ' + x.site, img: x.image, backdrop: x.image, pct: x.pct, badge: 'Theater', link: x };
    });
  }
  function rowItem(it) {
    if (it.src === 'theater') return it;
    var sub = it.kind === 'episode' ? 'S' + (it.parentIndex || 1) + ' E' + (it.index || 1) + ' · ' + it.title + (it.pct ? ' · ' + T.mins(it.mins * (100 - it.pct) / 100) + ' left' : '')
      : (it.kind === 'movie' ? 'Movie' + (it.year ? ' · ' + it.year : '') : it.kind === 'show' ? 'Show' : 'Album') + (it.pct ? ' · ' + T.mins(it.mins * (100 - it.pct) / 100) + ' left' : it.created && Date.now() - it.created < 3 * 86400000 ? ' · Just added' : '');
    return { src: it.src, id: it.id, kind: it.kind, title: it.kind === 'episode' ? (it.seriesName || it.title) : it.title, sub: sub, img: it.img, backdrop: it.backdrop, pct: it.pct, badge: 'My PC', item: it };
  }
  function heroFor(x) {
    var it = x.item || x;
    if (x.src === 'theater') return { kicker: 'Saved in Hearth Theater', title: x.title, sub: 'Saved link · ' + x.link.site + (x.link.len ? ' · ' + x.link.len : ''), pct: x.pct, play: 'Play full screen', more: 'All saved links', img: x.backdrop, x: x };
    var resume = it.pct > 0 && it.pct < 97;
    return {
      kicker: resume ? 'Pick up where you left off · From your PC' : 'New on your PC' + (it.created ? ' · Added ' + ago(it.created) : ''),
      title: x.title, sub: (x.sub || '').replace(/^S(\d+) E(\d+) · /, 'Season $1 · Episode $2, ').replace(' · Just added', '') + (it.tags && it.tags.length ? ' · ' + it.tags.join(' ') : ''),
      pct: it.pct || 0, play: resume ? 'Resume' : 'Play', more: it.kind === 'movie' ? 'Details' : 'All episodes', img: x.backdrop || x.img, x: x
    };
  }
  function ago(t) { var d = Math.floor((Date.now() - t) / 86400000); return d <= 0 ? 'today' : d === 1 ? 'yesterday' : d + ' days ago'; }

  H.refresh = function () {
    H.loading = true;
    var M = T.Media;
    Promise.all([M.keepWatching().catch(function () { return []; }), M.ready ? M.latest().catch(function () { return []; }) : Promise.resolve([])]).then(function (r) {
      H.keep = r[0].map(rowItem).concat(theaterProgress());
      H.latest = r[1].map(rowItem);
      var hs = pins().map(function (p) { return p; }).concat(H.keep.slice(0, 2)).concat(H.latest.slice(0, 1));
      var t = (T.json('theater') || [])[0]; if (t) hs.push({ src: 'theater', id: t.id, title: t.title, img: t.image, backdrop: t.image, pct: t.pct || 0, link: t, sub: '' });
      var seen = {}; H.heroes = hs.filter(function (x) { var k = x.src + x.id; if (seen[k]) return false; seen[k] = 1; return true; }).slice(0, 5).map(heroFor);
      if (H.hero >= H.heroes.length) H.hero = 0;
      H.loading = false;
      if (T.cur === 'home' && !T.layer) T.render();
    });
  };

  function heroHtml() {
    var h = H.heroes[H.hero];
    if (!h) {
      var M = T.Media, wp = T.lookWallpaper;
      return '<section aria-label="Featured" style="margin:6px 64px 0;height:420px;flex:none;border-radius:6px;position:relative;overflow:hidden;background:var(--bg2)">' + (wp ? '<img class="art" src="' + E(wp) + '" alt="">' : '') +
        '<div style="position:absolute;inset:0;background:linear-gradient(90deg,rgba(14,18,26,.94) 0%,rgba(14,18,26,.55) 42%,rgba(14,18,26,0) 75%)"></div><div style="position:absolute;left:56px;bottom:50px;display:flex;flex-direction:column;gap:14px;width:860px">' +
        '<span style="font-size:18px;font-weight:600;color:var(--focus);text-transform:uppercase;letter-spacing:.08em">' + (M.ready ? 'Your PC\'s library' : 'My Media') + '</span><span style="font-size:60px;font-weight:700;line-height:1.05">' + (H.loading ? 'Loading…' : M.ready ? 'Nothing in progress yet' : 'Connect your PC to see your shows here') + '</span>' +
        '<div style="display:flex;gap:14px;padding-top:10px">' + (M.ready ? '<button class="btn primary" data-f data-key="resume" data-act="go" data-arg="media">' + T.ICON.play + 'Browse My Media</button>' : '<button class="btn primary" data-f data-key="resume" data-act="setupMedia">' + T.ICON.pc + 'Connect your PC</button>') + '</div></div></section>';
    }
    var pinned = isPinned(h.x.item || h.x);
    var s = '<section aria-label="Featured" style="margin:6px 64px 0;height:420px;flex:none;border-radius:6px;position:relative;overflow:hidden;background:#10131a">' +
      (h.img ? '<img class="art" src="' + E(h.img) + '" alt="">' : '') +
      '<div style="position:absolute;inset:0;background:linear-gradient(90deg,rgba(14,18,26,.94) 0%,rgba(14,18,26,.55) 42%,rgba(14,18,26,0) 75%)"></div>' +
      '<div style="position:absolute;left:56px;top:50px;bottom:50px;display:flex;flex-direction:column;justify-content:flex-end;gap:12px;width:820px">' +
      '<span style="font-size:18px;font-weight:600;color:var(--focus);text-transform:uppercase;letter-spacing:.08em">' + E(h.kicker) + '</span>' +
      '<span style="font-size:66px;font-weight:700;line-height:1.02;letter-spacing:-.02em;color:#f2f6fa;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden">' + E(h.title) + '</span>' +
      '<span style="font-size:22px;color:#c3ccd4">' + E(h.sub) + '</span>' +
      '<div style="width:420px;height:6px;border-radius:3px;background:rgba(220,229,237,.22)"><div style="width:' + Math.round(h.pct || 0) + '%;height:6px;border-radius:3px;background:var(--acc)"></div></div>' +
      '<div style="display:flex;gap:14px;padding-top:12px">' +
      '<button class="btn primary" data-f data-key="resume" data-act="heroPlay">' + T.ICON.play + E(h.play) + '</button>' +
      '<button class="btn" data-f data-key="episodes" data-act="heroMore">' + T.ICON.list + E(h.more) + '</button>' +
      '<button class="btn" data-f data-key="pin" data-act="pin">' + T.ICON.star(pinned) + (pinned ? 'Pinned' : 'Pin') + '</button></div></div>' +
      '<div style="position:absolute;right:40px;bottom:30px;display:flex;gap:6px">';
    H.heroes.forEach(function (x, i) { s += '<button aria-label="Show ' + E(x.title) + '" data-act="hero" data-arg="' + i + '" style="height:22px;padding:8px 4px;box-sizing:border-box;display:flex;align-items:center"><span style="display:block;height:6px;border-radius:3px;width:' + (i === H.hero ? 34 : 12) + 'px;background:' + (i === H.hero ? '#dce5ed' : 'rgba(220,229,237,.35)') + ';transition:width .2s"></span></button>'; });
    return s + '</div></section>';
  }
  function card(x, key, w, hgt) {
    w = w || 318; hgt = hgt || 179;
    return '<button class="card" style="width:' + w + 'px" data-f data-key="' + key + '" data-act="open" data-arg="' + key + '" data-menu="' + key + '">' +
      '<span class="pic" style="width:' + w + 'px;height:' + hgt + 'px">' + (x.img ? '<img class="art" src="' + E(x.img) + '" alt="" loading="lazy">' : '<span class="notart" style="background:var(--panel3)"><b>' + E(x.title) + '</b></span>') +
      '<span class="badge">' + E(x.badge) + '</span>' + (x.pct ? '<span class="bar"><span style="width:' + Math.round(x.pct) + '%"></span></span>' : '') + '</span>' +
      '<span class="t">' + E(x.title) + '</span><span class="s">' + E(x.sub) + '</span></button>';
  }
  var cards = {};
  T.screen('home', {
    first: 'resume',
    toast: H.toast,
    enter: function (arg, back) { H.apps = T.homeApps(); if (!back || !H.heroes.length) H.refresh(); },
    resume: function () { H.apps = T.homeApps(); H.refresh(); },
    refresh: function () { H.refresh(); },
    render: function () {
      var s = T.settings.home, out = '<div class="screen">' + T.header('home');
      cards = {};
      if (s.hero !== false) out += heroHtml(); else out += '<div style="height:20px"></div>';
      if (s.rowApps !== false) {
        out += '<section style="padding:24px 64px 0;display:flex;flex-direction:column;gap:14px;flex:none"><div style="display:flex;align-items:baseline;gap:16px"><h2 class="row">Your apps</h2><button data-act="go" data-arg="apps" style="font-size:17px;color:var(--focus)">See all apps</button></div><div class="row-scroll">';
        var tiles = [{ name: 'My Media', bg: 'var(--panel3)', go: 'media' }, { name: 'Theater', bg: '#3d4f66', go: 'theater' }].concat(H.apps.home.map(function (a) { return { name: a.label, bg: T.tileColor(a.pkg), pkg: a.pkg, app: a }; }));
        if (T.kids()) tiles = tiles.filter(function (t) { return !t.pkg || /kids/i.test(t.pkg); });
        tiles.push({ name: 'All apps', bg: 'var(--panel2)', go: 'apps' });
        tiles.forEach(function (t, i) { out += '<button class="tile" style="width:186px;height:104px;flex:none;background:' + t.bg + ';font-size:21px" data-f data-key="app' + i + '" data-act="' + (t.go ? 'go' : 'app') + '" data-arg="' + E(t.go || t.pkg) + '" aria-label="' + E(t.name) + '">' + (t.app ? T.tileInner(t.app, E(t.name), 40) : E(t.name)) + '</button>'; });
        out += '</div></section>';
      }
      var rows = [];
      if (s.rowKeep !== false) rows.push(['Keep watching', H.keep]);
      if (s.rowNew !== false && H.latest.length) rows.push(['Recently added on your PC', H.latest]);
      if (s.rowTheater) rows.push(['Theater', (T.json('theater') || []).slice(0, 8).map(function (x) { return { src: 'theater', id: x.id, title: x.title, sub: x.site, img: x.image, pct: x.pct, badge: 'Theater', link: x }; })]);
      out += '<div data-scrolly style="flex:1;overflow:hidden;min-height:0">';
      rows.forEach(function (r, ri) {
        out += '<section style="padding:26px 64px 0;display:flex;flex-direction:column;gap:14px"><h2 class="row">' + E(r[0]) + '</h2><div class="row-scroll">';
        if (!r[1].length) out += '<span class="muted" style="font-size:19px;padding:30px 0">' + (T.Media.ready ? 'Start something on My Media or Theater and it shows up here.' : 'Connect your PC (My Media) or save a link in Theater to see it here.') + '</span>';
        r[1].forEach(function (x, i) { var k = 'r' + ri + '_' + i; cards[k] = x; out += card(x, k); });
        out += '</div></section>';
      });
      out += '</div>';
      out += T.footer([['OK', 'Open'], ['☰', 'Options'], ['◀▶', 'Move'], ['⇄', 'Inputs', 'inputs'], ['☾', 'Screensaver', 'screensaver']], H.toast);
      return out + '</div>';
    },
    after: function () {
      clearInterval(H.rot);
      var secs = Number(T.settings.home.rotate) || 0;
      if (secs > 0) H.rot = setInterval(function () {
        var k = T.focusKey();
        if (T.cur !== 'home' || T.layer || H.heroes.length < 2 || k === 'resume' || k === 'episodes' || k === 'pin') return;
        H.hero = (H.hero + 1) % H.heroes.length; T.render();
      }, secs * 1000);
    },
    leave: function () { clearInterval(H.rot); },
    act: function (act, arg) {
      var h = H.heroes[H.hero];
      if (act === 'hero') { H.hero = Number(arg); T.render('resume'); return true; }
      if (act === 'app') { if (!T.n('appOpen', arg)) T.toast('That app couldn\'t be opened'); return true; }
      if (act === 'setupMedia') { T.go('settings', 'My Media'); return true; }
      if (act === 'heroPlay' && h) { open(h.x, true); return true; }
      if (act === 'heroMore' && h) { open(h.x, false); return true; }
      if (act === 'pin' && h) {
        var it = h.x.item || h.x, list = pins(), was = isPinned(it);
        if (was) list = list.filter(function (p) { return !(p.id === it.id && p.src === it.src); });
        else list = [{ src: h.x.src, id: it.id, title: h.x.title, sub: h.x.sub, img: h.x.img, backdrop: h.x.backdrop, kind: h.x.kind, link: h.x.link, item: h.x.item }].concat(list).slice(0, 6);
        setPins(list); T.toast((was ? 'Unpinned ' : 'Pinned ') + h.title + (was ? '' : ' to the top of Home')); T.render('pin'); return true;
      }
      if (act === 'open' && cards[arg]) { open(cards[arg], true); return true; }
      return false;
    },
    menu: function (key) {
      var x = cards[key]; if (!x) return;
      var it = x.item || x, was = isPinned(it);
      var list = pins();
      if (was) list = list.filter(function (p) { return !(p.id === it.id && p.src === it.src); }); else list = [{ src: x.src, id: it.id, title: x.title, sub: x.sub, img: x.img, backdrop: x.backdrop, kind: x.kind, link: x.link, item: x.item }].concat(list).slice(0, 6);
      setPins(list); T.toast((was ? 'Unpinned ' : 'Pinned ') + x.title); H.refresh();
    }
  });
  function open(x, play) {
    if (x.src === 'theater') { if (play) T.theaterPlay(x.link); else T.go('theater'); return; }
    var it = x.item || x;
    if (it.kind === 'episode') { if (play) T.play(it); else T.go('show', it.seriesId); }
    else if (it.kind === 'show') T.go('show', it.id);
    else if (it.kind === 'movie') { if (play) T.play(it); else T.go('media', { detail: it }); }
    else T.go('media', { detail: it });
  }
  T.openItem = open;
})();
