/* My Media (mockup 4), a show (mockup 5), a movie's details, the player (with subtitles and audio) and music. */
(function () {
  'use strict';
  var T = window.hearthTV, E = T.esc, M = T.Media;
  var L = { kind: 'All', sort: 'Recently added', items: [], loading: false, error: '', detail: null, toast: '' };
  var TINTS = ['#2b3443', '#2a2f38', '#3b3326', '#26333b', '#33263b', '#263b2e'];

  function loadLibrary() {
    if (!M.ready) { L.items = []; return; }
    L.loading = true; L.error = '';
    M.wake();
    M.library().then(function (list) { L.items = list; L.loading = false; if (T.cur === 'media') T.render(); })
      .catch(function (e) { L.loading = false; L.error = M.error === 'signedOut' ? 'My Media signed out: sign in again in Settings › My Media' : 'Your PC didn\'t answer (' + e.message + '). Is it on?'; if (T.cur === 'media') T.render(); });
  }
  function shown() {
    var list = L.items.filter(function (x) { return L.kind === 'All' || (L.kind === 'Shows' && x.kind === 'show') || (L.kind === 'Movies' && x.kind === 'movie') || (L.kind === 'Music' && x.kind === 'album'); });
    if (L.sort === 'A to Z') list = list.slice().sort(function (a, b) { return a.title.localeCompare(b.title); });
    else if (L.sort === 'Unwatched') list = list.filter(function (x) { return !x.played && (x.kind !== 'show' || x.unplayed > 0); });
    return list;
  }
  function sub(x) {
    if (x.kind === 'show') return 'Show' + (x.year ? ' · ' + x.year : '') + (x.childCount ? ' · ' + x.childCount + ' episodes' : '');
    if (x.kind === 'movie') return 'Movie' + (x.year ? ' · ' + x.year : '') + (x.mins ? ' · ' + T.mins(x.mins) : '');
    if (x.kind === 'album') return (x.artist || 'Album') + (x.year ? ' · ' + x.year : '') + (x.childCount ? ' · ' + x.childCount + ' tracks' : '');
    return '';
  }
  function badge(x) {
    if (x.played) return 'Watched';
    if (x.kind === 'show') return x.unplayed ? x.unplayed + ' new' : 'Show';
    if (x.kind === 'album') return 'Album';
    return (x.tags || [])[0] || 'HD';
  }
  var grid = {};
  T.screen('media', {
    first: 'k0',
    enter: function (arg) {
      L.detail = arg && arg.detail ? arg.detail : null;
      if (!L.items.length || !(arg && arg.detail)) loadLibrary();
    },
    back: function () { if (L.detail) { L.detail = null; T.render(); return true; } return false; },
    render: function () {
      grid = {};
      if (L.detail) return detailHtml(L.detail);
      var out = '<div class="screen">' + T.header('media');
      out += '<div style="flex:1;display:flex;flex-direction:column;gap:22px;padding:16px 64px 0;min-height:0"><div style="display:flex;align-items:center;gap:10px">';
      ['All', 'Shows', 'Movies', 'Music'].forEach(function (k, i) { out += '<button class="chip' + (L.kind === k ? ' on' : '') + '" style="height:48px;padding:0 24px;border-radius:24px;font-size:19px" data-f data-key="k' + i + '" data-act="kind" data-arg="' + k + '">' + k + '</button>'; });
      out += '<div style="flex:1"></div><span class="muted" style="font-size:18px;padding-right:8px">Sort</span>';
      ['Recently added', 'A to Z', 'Unwatched'].forEach(function (k, i) { out += '<button data-f data-key="s' + i + '" data-act="sort" data-arg="' + k + '" style="height:44px;padding:0 18px;border-radius:var(--r);font-size:17px;' + (L.sort === k ? 'background:var(--panel2);font-weight:600' : 'color:var(--muted)') + '">' + k + '</button>'; });
      out += '</div>';
      if (!M.ready) out += emptyHtml('My Media isn\'t connected yet', 'Sign in to your PC\'s Jellyfin (Hearth Server) or Plex to see your shows, movies and music here.', 'Connect My Media', 'setup');
      else if (L.error) out += emptyHtml('Can\'t reach your library', L.error, 'Try again', 'retry');
      else if (L.loading && !L.items.length) out += '<span class="muted" style="font-size:22px;padding-top:40px">Loading your library…</span>';
      else {
        out += '<div data-scrolly style="flex:1;overflow:hidden;min-height:0;padding:8px;margin:-8px"><div style="display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:30px 24px;padding-bottom:40px">';
        var list = shown();
        if (!list.length) out += '<span class="muted" style="font-size:20px;grid-column:1/-1">Nothing here' + (L.sort === 'Unwatched' ? ' that you haven\'t watched' : '') + '.</span>';
        list.forEach(function (x, i) {
          var k = 'i' + i; grid[k] = x;
          out += '<button class="card" style="min-width:0" data-f data-key="' + k + '" data-act="open" data-arg="' + k + '" data-menu="' + k + '"><span class="pic" style="width:100%;aspect-ratio:16/9">' +
            (x.img ? '<img class="art" src="' + E(x.img) + '" alt="" loading="lazy">' : '<span class="notart" style="background:' + TINTS[i % 6] + '"><b>' + E(x.title) + '</b><i>' + (x.kind === 'album' ? 'No album art in the files' : 'No artwork in your library yet') + '</i></span>') +
            ((x.pct > 0 || x.played) && x.kind !== 'album' ? '<span class="bar"><span style="width:' + (x.played ? 100 : Math.round(x.pct)) + '%"></span></span>' : '') +
            '<span class="badge r" style="font-size:14px">' + E(badge(x)) + '</span></span><span class="t" style="font-size:20px">' + E(x.title) + '</span><span class="s">' + E(sub(x)) + '</span></button>';
        });
        out += '</div></div>';
      }
      out += '</div>' + musicBar();
      out += T.footer([['OK', 'Open'], ['☰', 'Mark watched or pin'], ['↩', 'Back to Home', 'go', 'home']], L.items.length ? L.items.length + ' items on your PC · new files show up here by themselves' : '') + '</div>';
      return out;
    },
    act: function (act, arg) {
      if (act === 'kind') { L.kind = arg; T.render(); return true; }
      if (act === 'sort') { L.sort = arg; T.render(); return true; }
      if (act === 'setup') { T.go('settings', 'My Media'); return true; }
      if (act === 'retry') { loadLibrary(); T.render(); return true; }
      if (act === 'open' && grid[arg]) {
        var x = grid[arg];
        if (x.kind === 'show') T.go('show', x.id);
        else if (x.kind === 'album') playAlbum(x);
        else { L.detail = x; T.render('dPlay'); }
        return true;
      }
      if (act === 'dPlay' && L.detail) { T.play(L.detail); return true; }
      if (act === 'dWatched' && L.detail) {
        var d = L.detail, on = !d.played;
        M.setPlayed(d.id, on).then(function () { d.played = on; d.pct = on ? 100 : 0; T.toast(on ? 'Marked watched' : 'Marked unwatched'); T.render('dWatched'); });
        return true;
      }
      if (act === 'dBack') { L.detail = null; T.render(); return true; }
      if (musicAct(act)) return true;
      return false;
    },
    menu: function (key) {
      var x = grid[key]; if (!x || x.kind === 'album') return;
      M.setPlayed(x.id, !x.played).then(function () { x.played = !x.played; T.toast(x.played ? x.title + ' marked watched' : x.title + ' marked unwatched'); T.render(key); });
    }
  });
  function emptyHtml(title, text, btn, act) {
    return '<div style="display:flex;flex-direction:column;gap:14px;padding-top:40px;max-width:900px"><span style="font-size:34px;font-weight:700">' + E(title) + '</span><span class="soft" style="font-size:21px;line-height:1.5">' + E(text) + '</span><div><button class="btn primary" data-f data-key="emptyBtn" data-act="' + act + '">' + E(btn) + '</button></div></div>';
  }
  function detailHtml(d) {
    var credit = d.overview ? '' : '';
    return '<div class="screen" style="background:#10131a">' + (d.backdrop ? '<img class="art" src="' + E(d.backdrop) + '" alt="" style="opacity:.75">' : '') +
      '<div style="position:absolute;inset:0;background:linear-gradient(90deg,rgba(14,18,26,.96) 0%,rgba(14,18,26,.7) 45%,rgba(14,18,26,.1) 100%)"></div>' +
      '<div style="position:absolute;left:96px;top:140px;width:900px;display:flex;flex-direction:column;gap:18px">' +
      '<span style="font-size:18px;font-weight:600;color:var(--focus);text-transform:uppercase;letter-spacing:.08em">' + (d.kind === 'show' ? 'Show' : 'Movie') + ' · From your PC</span>' +
      '<span style="font-size:76px;font-weight:700;line-height:1.02;letter-spacing:-.02em;color:#f2f6fa">' + E(d.title) + '</span>' +
      '<span style="font-size:22px;color:#c3ccd4">' + E([d.year, d.genre, d.mins ? T.mins(d.mins) : '', (d.tags || []).join(' ')].filter(Boolean).join(' · ')) + '</span>' +
      '<p style="margin:0;font-size:21px;line-height:1.5;color:#c3ccd4;display:-webkit-box;-webkit-line-clamp:5;-webkit-box-orient:vertical;overflow:hidden">' + E(d.overview || 'No summary in your library.') + '</p>' +
      '<div style="display:flex;gap:14px;padding-top:12px"><button class="btn primary" style="height:62px" data-f data-key="dPlay" data-act="dPlay">' + T.ICON.play + (d.pct > 0 && !d.played ? 'Resume' : 'Play') + '</button>' +
      '<button class="btn" style="height:62px" data-f data-key="dWatched" data-act="dWatched">' + (d.played ? 'Mark unwatched' : 'Mark watched') + '</button>' +
      '<button class="btn" style="height:62px" data-f data-key="dBack" data-act="dBack">↩ Back</button></div>' + credit + '</div></div>';
  }

  // ---------------- A show (mockup 5) ----------------
  var S = { id: null, show: null, seasons: [], tab: 0, eps: [], ep: 0, more: [], pinned: false, loading: true };
  function loadShow(id) {
    S.id = id; S.show = null; S.seasons = []; S.eps = []; S.more = []; S.tab = 0; S.ep = 0; S.loading = true;
    M.wake();
    M.get(id).then(function (x) {
      S.show = x;
      return M.seasons(id);
    }).then(function (ss) {
      S.seasons = ss.filter(function (x) { return x.kind === 'season'; });
      return M.nextUp(id).catch(function () { return []; });
    }).then(function (nu) {
      var next = nu[0], si = 0;
      if (next) S.seasons.forEach(function (x, i) { if (x.id === next.seasonId || x.index === next.parentIndex) si = i; });
      S.tab = si;
      return loadEps(next && next.id);
    }).then(function () { return M.similar(id).catch(function () { return []; }); }).then(function (m) { S.more = m; S.loading = false; if (T.cur === 'show') T.render(); })
      .catch(function (e) { S.loading = false; S.error = e.message; if (T.cur === 'show') T.render(); });
  }
  function loadEps(focusId) {
    var se = S.seasons[S.tab]; if (!se) { S.eps = []; return Promise.resolve(); }
    return M.episodes(S.id, se.id).then(function (eps) {
      S.eps = eps; S.ep = 0;
      eps.forEach(function (e, i) { if (focusId ? e.id === focusId : (!e.played && S.ep === 0 && i > 0 && eps[0].played)) S.ep = i; });
      if (!focusId) { var firstOpen = eps.findIndex(function (e) { return !e.played; }); if (firstOpen >= 0) S.ep = firstOpen; }
    });
  }
  T.screen('show', {
    first: 'resume',
    enter: function (id, back) { if (!back || S.id !== id) loadShow(id); },
    render: function () {
      var x = S.show || { title: S.loading ? 'Loading…' : (S.error || ''), backdrop: '' }, cur = S.eps[S.ep];
      var pinnedNow = (function () { var all = (T.settings.pins || {})[T.currentProfile().id] || []; return all.some(function (p) { return p.id === S.id; }); })();
      var out = '<div class="screen" style="display:block">' +
        '<div style="position:absolute;right:0;top:0;width:1300px;height:730px;overflow:hidden">' + (x.backdrop ? '<img class="art" src="' + E(x.backdrop) + '" alt="" style="object-position:center 30%">' : '') + '</div>' +
        '<div style="position:absolute;right:0;top:0;width:1300px;height:730px;background:linear-gradient(90deg,var(--bg) 0%,rgba(23,26,33,.7) 30%,rgba(23,26,33,0) 65%)"></div>' +
        '<div style="position:absolute;left:0;right:0;top:450px;height:280px;background:linear-gradient(180deg,rgba(23,26,33,0) 0%,var(--bg) 100%)"></div>' +
        '<div style="position:relative;display:flex;flex-direction:column;height:1080px;box-sizing:border-box">' +
        '<header style="height:100px;flex:none;display:flex;align-items:center;gap:14px;padding:0 64px;font-size:19px;color:var(--muted)"><button data-act="go" data-arg="home">Home</button><span>›</span><button data-act="go" data-arg="media">My Media</button><span>›</span><span>Shows</span><span>›</span><span style="color:var(--text);font-weight:600">' + E(x.title) + '</span><div style="flex:1"></div>' +
        '<span style="display:flex;align-items:center;gap:8px;font-size:17px"><span class="dot"></span>Streaming from Nvidia PC · ' + E(M.kind === 'plex' ? 'Plex' : 'Hearth Server') + '</span></header>' +
        '<section style="padding:36px 64px 0;display:flex;flex-direction:column;gap:16px;width:980px;flex:none">' +
        '<span style="font-size:76px;font-weight:700;letter-spacing:-.02em;line-height:1;color:var(--text)">' + E(x.title) + '</span>' +
        '<div style="display:flex;gap:18px;align-items:center;font-size:20px;color:var(--soft)">' + [x.year, x.genre, S.seasons.length ? (S.seasons.length === 1 ? S.eps.length + ' episodes on your PC' : S.seasons.length + ' seasons') : ''].filter(Boolean).map(E).join('<span>·</span>') +
        (x.tags || []).map(function (t) { return '<span style="border:1.5px solid var(--muted);border-radius:3px;padding:1px 8px;font-size:16px;font-weight:600">' + E(t) + '</span>'; }).join('') + '</div>' +
        '<p style="margin:0;font-size:21px;line-height:1.5;color:var(--soft);max-width:780px;display:-webkit-box;-webkit-line-clamp:3;-webkit-box-orient:vertical;overflow:hidden">' + E(x.overview || '') + '</p>' +
        '<div style="display:flex;gap:14px;padding-top:14px">' +
        '<button class="btn primary" style="height:62px" data-f data-key="resume" data-act="play">' + T.ICON.play + E(resumeLabel(cur)) + '</button>' +
        '<button class="btn" style="height:62px" data-f data-key="over" data-act="startOver">' + T.ICON.restart + 'Start over</button>' +
        '<button class="btn square" aria-label="' + (pinnedNow ? 'Unpin from Home' : 'Pin to Home') + '" data-f data-key="pin" data-act="pin">' + T.ICON.star(pinnedNow) + '</button>' +
        '<button class="btn square" aria-label="Subtitles and audio" data-f data-key="subs" data-act="subs">' + T.ICON.subs + '</button></div></section>' +
        '<section style="padding:50px 64px 0;display:flex;flex-direction:column;gap:18px;flex:none"><div style="display:flex;gap:10px">';
      S.seasons.forEach(function (se, i) { out += '<button data-f data-key="t' + i + '" data-act="season" data-arg="' + i + '" style="height:46px;padding:0 22px;border-radius:var(--r);font-size:19px;' + (i === S.tab ? 'background:var(--text);color:var(--bg);font-weight:700' : 'color:var(--muted)') + '">' + E(se.title) + '</button>'; });
      out += '<button data-f data-key="tMore" data-act="season" data-arg="more" style="height:46px;padding:0 22px;border-radius:var(--r);font-size:19px;' + (S.tab === 'more' ? 'background:var(--text);color:var(--bg);font-weight:700' : 'color:var(--muted)') + '">More like this</button></div><div class="row-scroll" style="gap:22px">';
      var cards = S.tab === 'more' ? S.more : S.eps;
      cards.forEach(function (e, i) {
        var isEp = S.tab !== 'more', left = Math.round(e.mins * (100 - e.pct) / 100);
        out += '<button class="card" style="width:400px" data-f data-key="e' + i + '" data-act="' + (isEp ? 'ep' : 'more') + '" data-arg="' + i + '"><span class="pic" style="width:400px;height:225px' + (isEp && i === S.ep ? ';box-shadow:0 0 0 3px var(--bg),0 0 0 6px var(--focus)' : '') + '">' +
          (e.img ? '<img class="art" src="' + E(e.img) + '" alt="" loading="lazy">' : '<span class="notart" style="background:var(--panel3)"><b>' + E(e.title) + '</b></span>') +
          (e.played ? '<span class="badge r" style="width:32px;height:32px;padding:0;border-radius:16px;display:flex;align-items:center;justify-content:center">' + T.ICON.check + '</span>' : '') +
          '<span class="bar" style="background:rgba(220,229,237,.2)"><span style="width:' + (e.played ? 100 : Math.round(e.pct)) + '%"></span></span></span>' +
          '<span class="t" style="font-size:20px">' + E(isEp ? 'E' + (e.index || i + 1) + ' · ' + e.title : e.title) + '</span><span class="s">' + E(isEp ? Math.round(e.mins) + ' min · ' + (e.played ? 'Watched' : e.pct > 0 ? left + ' min left' : 'Not started') : sub(e)) + '</span></button>';
      });
      if (!cards.length) out += '<span class="muted" style="font-size:19px;padding:40px 0">' + (S.loading ? 'Loading…' : 'Nothing here.') + '</span>';
      out += '</div></section><div style="flex:1"></div>' + T.footer([['OK', 'Play'], ['◀▶', 'Episodes'], ['↩', 'Back to My Media', 'go', 'media']], 'Watched progress syncs with your PC and every Hearth device') + '</div></div>';
      return out;
    },
    act: function (act, arg) {
      var cur = S.eps[S.ep];
      if (act === 'season') {
        if (arg === 'more') { S.tab = 'more'; T.render('tMore'); return true; }
        S.tab = Number(arg); loadEps().then(function () { T.render('t' + arg); }); return true;
      }
      if (act === 'ep') { var i = Number(arg); if (i === S.ep) { T.play(S.eps[i], { list: S.eps, at: i }); } else { S.ep = i; T.toast('Selected E' + (S.eps[i].index || i + 1) + ' · OK again plays it'); T.render('e' + i); } return true; }
      if (act === 'more') { var m = S.more[Number(arg)]; if (m) T.openItem({ item: m, src: m.src, kind: m.kind }, false); return true; }
      if (act === 'play' && cur) { T.play(cur, { list: S.eps, at: S.ep }); return true; }
      if (act === 'startOver' && cur) { T.play(cur, { list: S.eps, at: S.ep, from: 0 }); return true; }
      if (act === 'subs') { T.subtitlesPanel(); return true; }
      if (act === 'pin' && S.show) {
        var all = Object.assign({}, T.settings.pins || {}), pid = T.currentProfile().id, list = (all[pid] || []).slice(), was = list.some(function (p) { return p.id === S.id; });
        list = was ? list.filter(function (p) { return p.id !== S.id; }) : [{ src: S.show.src, id: S.id, title: S.show.title, sub: 'Show', img: S.show.img, backdrop: S.show.backdrop, kind: 'show', item: S.show }].concat(list).slice(0, 6);
        all[pid] = list; T.set('pins', all); T.toast(was ? 'Removed from the top of Home' : 'Pinned to the top of Home'); T.render('pin'); return true;
      }
      return false;
    }
  });
  function resumeLabel(e) {
    if (!e) return 'Play';
    var tag = 'S' + (e.parentIndex || 1) + ' E' + (e.index || 1);
    return e.pct > 0 && !e.played ? 'Resume ' + tag + ' · ' + Math.round(e.mins * (100 - e.pct) / 100) + ' min left' : 'Play ' + tag;
  }

  // ---------------- Subtitles and audio (remembered per show, and the default in Settings › My Media) ----------------
  T.subtitlesPanel = function (forPlayer) {
    var key = S.id || 'movie', prefs = (T.settings.showPrefs || {})[key] || {};
    var P = T.player || {}, subsList = (P.pb && P.pb.subs) || [];
    var subOpts = ['Off'].concat(subsList.length ? subsList.map(function (s) { return s.label; }) : ['English', 'English (SDH)', 'Spanish']);
    var audOpts = (P.pb && P.pb.audio && P.pb.audio.length) ? P.pb.audio.map(function (a) { return a.label; }) : ['Original', 'Night mode (quieter loud parts)'];
    var curSub = prefs.sub || (T.settings.media.subs === 'off' ? 'Off' : T.settings.media.subs), curAud = prefs.aud || audOpts[0], size = T.settings.player.subSize || 'm';
    T.open({
      id: 'subs', first: 'so0',
      render: function () {
        var h = '<div style="position:absolute;inset:0;background:rgba(10,12,16,.55)"></div><aside class="on-panel" aria-label="Subtitles and audio" style="position:absolute;right:0;top:0;bottom:0;width:560px;background:var(--panel);border-left:1px solid var(--line);padding:48px 40px;box-sizing:border-box;display:flex;flex-direction:column;gap:14px">' +
          '<span style="font-size:30px;font-weight:700;padding-bottom:10px">Subtitles and audio</span><span class="h3">Subtitles</span>';
        subOpts.forEach(function (o, i) { h += opt('so' + i, 'sub', o, o.toLowerCase() === String(curSub).toLowerCase()); });
        h += '<span class="h3" style="padding-top:12px">Audio</span>';
        audOpts.forEach(function (o, i) { h += opt('ao' + i, 'aud', o, o === curAud); });
        h += '<span class="h3" style="padding-top:12px">Subtitle size</span><div style="display:flex;gap:10px">';
        [['s', 'Small'], ['m', 'Medium'], ['l', 'Large']].forEach(function (z) { h += '<button data-f data-key="z' + z[0] + '" data-act="size" data-arg="' + z[0] + '" style="flex:1;height:52px;border-radius:var(--r);font-size:18px;' + (size === z[0] ? 'background:var(--panel3);font-weight:700' : 'background:var(--panel2)') + '">' + z[1] + '</button>'; });
        return h + '</div><div style="flex:1"></div><button class="btn primary" style="height:58px" data-f data-key="subsDone" data-act="done">Done</button></aside>';
      },
      act: function (act, arg) {
        if (act === 'sub') curSub = arg; else if (act === 'aud') curAud = arg; else if (act === 'size') { size = arg; T.set('player.subSize', arg); }
        else if (act === 'done') {
          var all = Object.assign({}, T.settings.showPrefs || {}); all[key] = { sub: curSub, aud: curAud }; T.set('showPrefs', all);
          T.close(); T.toast('Subtitles: ' + curSub + ' · Audio: ' + curAud + ' · remembered for this ' + (S.id ? 'show' : 'movie'));
          if (T.player && T.player.applyPrefs) T.player.applyPrefs();
          return true;
        }
        T.renderLayer(); return true;
      }
    });
    function opt(k, act, label, sel) { return '<button data-f data-key="' + k + '" data-act="' + act + '" data-arg="' + E(label) + '" style="height:56px;padding:0 20px;border-radius:var(--r);display:flex;align-items:center;justify-content:space-between;font-size:20px;' + (sel ? 'background:var(--panel3);font-weight:600' : 'background:var(--panel2)') + '"><span>' + E(label) + '</span><span style="color:var(--focus)">' + (sel ? '✓' : '') + '</span></button>'; }
  };

  // ---------------- The player ----------------
  var PL = T.player = { item: null, list: null, at: 0, pb: null, timer: null, chrome: true };
  T.play = function (item, opts) {
    opts = opts || {};
    PL.item = item; PL.list = opts.list || null; PL.at = opts.at || 0; PL.pb = null; PL.from = opts.from;
    M.wake();
    T.go('player');
  };
  function loadPlayback() {
    var it = PL.item, startSec = PL.from === 0 ? 0 : (it.pct > 0 && it.pct < 97 ? it.mins * 60 * it.pct / 100 : 0);
    PL.startSec = startSec;
    return M.playback(it.id, Math.round(startSec * 10000000)).then(function (pb) {
      PL.pb = pb;
      if (T.settings.player.frame && pb.fps) T.n('frameRate', pb.fps);
      var v = document.getElementById('pv'); if (!v) return;
      v.src = pb.url;
      v.addEventListener('loadedmetadata', function once() { v.removeEventListener('loadedmetadata', once); if (startSec && M.kind !== 'plex') v.currentTime = startSec; }, false);
      PL.applyPrefs();
      v.play().catch(function () {});
      M.report('start', it.id, pb, startSec, false);
      clearInterval(PL.timer);
      PL.timer = setInterval(function () { var vv = document.getElementById('pv'); if (vv && PL.pb) M.report('progress', it.id, PL.pb, PL.pos(), vv.paused, vv.duration); }, 10000);
    }).catch(function (e) { T.toast('Couldn\'t play: ' + e.message); T.log('play: ' + e.message); });
  }
  PL.pos = function () { var v = document.getElementById('pv'); return v ? v.currentTime : 0; };
  PL.applyPrefs = function () {
    var v = document.getElementById('pv'); if (!v || !PL.pb) return;
    var key = S.id || 'movie', prefs = (T.settings.showPrefs || {})[key] || {}, want = (prefs.sub || T.settings.media.subs || 'off').toLowerCase();
    [].slice.call(v.querySelectorAll('track')).forEach(function (t) { t.remove(); });
    (PL.pb.subs || []).forEach(function (s) {
      var tr = document.createElement('track'); tr.kind = 'subtitles'; tr.label = s.label; tr.srclang = s.lang || 'en'; tr.src = s.url;
      if (want !== 'off' && (s.label.toLowerCase() === want || (s.lang || '').toLowerCase().indexOf(want.slice(0, 2)) === 0)) tr.default = true;
      v.appendChild(tr);
    });
    setTimeout(function () { [].forEach.call(v.textTracks, function (t) { t.mode = (want !== 'off' && (t.label.toLowerCase() === want || (t.language || '').indexOf(want.slice(0, 2)) === 0)) ? 'showing' : 'disabled'; }); }, 300);
    var size = { s: '26px', m: '34px', l: '44px' }[T.settings.player.subSize || 'm'];
    document.documentElement.style.setProperty('--cue', size);
    PL.audioFx(v);
  };
  // Night mode (quieter loud parts) and Clearer voices: the browser's own compressor and EQ on the video's sound.
  PL.audioFx = function (v) {
    var p = T.settings.player;
    if (!p.night && !p.dialog) { if (PL.ac) { try { PL.comp.disconnect(); PL.eq.disconnect(); PL.srcNode.connect(PL.ac.destination); } catch (e) {} } return; }
    try {
      if (!PL.ac) { PL.ac = new (window.AudioContext || window.webkitAudioContext)(); PL.srcNode = PL.ac.createMediaElementSource(v); PL.comp = PL.ac.createDynamicsCompressor(); PL.eq = PL.ac.createBiquadFilter(); }
      PL.srcNode.disconnect();
      PL.eq.type = 'peaking'; PL.eq.frequency.value = 2500; PL.eq.Q.value = 0.9; PL.eq.gain.value = p.dialog ? 6 : 0;
      PL.comp.threshold.value = p.night ? -30 : -12; PL.comp.ratio.value = p.night ? 6 : 2;
      PL.srcNode.connect(PL.eq); PL.eq.connect(PL.comp); PL.comp.connect(PL.ac.destination);
    } catch (e) { T.log('audio fx: ' + e.message); }
  };
  function fmt(s) { s = Math.max(0, Math.floor(s || 0)); var h = Math.floor(s / 3600), m = Math.floor(s % 3600 / 60), x = s % 60; return (h ? h + ':' + (m < 10 ? '0' : '') : '') + m + ':' + (x < 10 ? '0' : '') + x; }
  function stopPlayback() {
    var v = document.getElementById('pv'); clearInterval(PL.timer); clearTimeout(PL.hide);
    if (v && PL.pb && PL.item) { M.report('stop', PL.item.id, PL.pb, PL.pos(), true, v.duration); T.toast('Stopped at ' + fmt(PL.pos()) + ' · saved to your PC'); try { v.pause(); v.removeAttribute('src'); v.load(); } catch (e) {} }
    T.playing = false; T.n('keepOn', false); T.n('frameRate', 0); T.idle();
  }
  function showChrome() {
    var c = document.querySelector('.player .chrome'); if (c) c.classList.remove('hidden');
    clearTimeout(PL.hide); PL.hide = setTimeout(function () { var v = document.getElementById('pv'); var cc = document.querySelector('.player .chrome'); if (cc && v && !v.paused && !T.layer) cc.classList.add('hidden'); }, 4500);
  }
  T.screen('player', {
    first: 'pp',
    enter: function () { T.playing = true; T.n('keepOn', true); },
    leave: function () { stopPlayback(); },
    render: function () {
      var it = PL.item || {}, ep = it.kind === 'episode';
      return '<div class="player"><video id="pv" playsinline crossorigin="anonymous"></video><style>video::cue{font-size:var(--cue,34px)}</style><div class="chrome">' +
        '<div class="topfade"></div><div class="botfade"></div>' +
        '<div style="position:absolute;left:80px;top:56px;display:flex;flex-direction:column;gap:6px"><span style="font-size:32px;font-weight:700">' + E(ep ? it.seriesName : it.title) + '</span><span style="font-size:20px;color:#c3ccd4">' + E(ep ? 'Season ' + (it.parentIndex || 1) + ' · Episode ' + (it.index || 1) + ' · ' + it.title : [it.year, it.tags && it.tags.join(' ')].filter(Boolean).join(' · ')) + '</span></div>' +
        '<div style="position:absolute;left:80px;right:80px;bottom:64px;display:flex;flex-direction:column;gap:22px">' +
        '<div style="display:flex;align-items:center;gap:22px;font-size:20px;font-variant-numeric:tabular-nums"><span data-pos>0:00</span><div style="flex:1;height:8px;border-radius:4px;background:rgba(220,229,237,.25);position:relative"><div data-prog style="width:0%;height:8px;border-radius:4px;background:var(--acc)"></div></div><span data-len style="color:#c3ccd4">' + fmt(it.mins * 60) + '</span></div>' +
        '<div style="display:flex;align-items:center;gap:16px">' +
        '<button class="round" aria-label="Back 1 minute" data-f data-key="rew" data-act="seek" data-arg="-60" style="width:64px;height:64px;background:rgba(36,48,63,.9);font-size:15px;font-weight:700">−1m</button>' +
        '<button class="round" aria-label="Pause" data-f data-key="pp" data-act="pp" style="width:80px;height:80px;background:#dce5ed;color:#171a21" data-pp>' + T.ICON.pause + '</button>' +
        '<button class="round" aria-label="Forward 1 minute" data-f data-key="fwd" data-act="seek" data-arg="60" style="width:64px;height:64px;background:rgba(36,48,63,.9);font-size:15px;font-weight:700">+1m</button>' +
        '<div style="flex:1"></div><button class="btn small" style="background:rgba(36,48,63,.9)" data-f data-key="psubs" data-act="subs">Subtitles and audio</button>' +
        (PL.list && PL.at < PL.list.length - 1 ? '<button class="btn small" style="background:rgba(36,48,63,.9)" data-f data-key="pnext" data-act="next">Next episode</button>' : '') +
        '<button class="btn small" style="background:rgba(36,48,63,.9)" data-f data-key="pback" data-act="back">↩ Back</button></div></div></div></div>';
    },
    after: function () {
      var v = document.getElementById('pv');
      v.addEventListener('timeupdate', function () {
        var d = v.duration && isFinite(v.duration) ? v.duration : PL.item.mins * 60, p = v.currentTime;
        var a = document.querySelector('[data-pos]'), b = document.querySelector('[data-prog]'), c = document.querySelector('[data-len]');
        if (a) a.textContent = fmt(p); if (b) b.style.width = Math.min(100, p / d * 100) + '%'; if (c) c.textContent = fmt(d);
      });
      v.addEventListener('pause', function () { var b = document.querySelector('[data-pp]'); if (b) { b.innerHTML = T.ICON.play.replace('22', '30').replace('22', '30'); b.setAttribute('aria-label', 'Play'); } showChrome(); });
      v.addEventListener('play', function () { var b = document.querySelector('[data-pp]'); if (b) { b.innerHTML = T.ICON.pause; b.setAttribute('aria-label', 'Pause'); } showChrome(); });
      v.addEventListener('ended', function () { if (PL.list && PL.at < PL.list.length - 1) next(); else T.back(); });
      v.addEventListener('error', function () { T.toast('This file didn\'t play on the TV. Try a lower quality in Settings › My Media.'); });
      showChrome();
      loadPlayback();
    },
    key: function (k) {
      var v = document.getElementById('pv'); showChrome();
      if (!v) return false;
      if (k === 'playpause') { v.paused ? v.play() : v.pause(); return true; }
      if (k === 'ff') { v.currentTime += 30; return true; }
      if (k === 'rew') { v.currentTime = Math.max(0, v.currentTime - 10); return true; }
      return false;
    },
    act: function (act, arg) {
      var v = document.getElementById('pv'); showChrome();
      if (act === 'pp' && v) { v.paused ? v.play() : v.pause(); return true; }
      if (act === 'seek' && v) { v.currentTime = Math.max(0, v.currentTime + Number(arg)); return true; }
      if (act === 'subs') { if (v) v.pause(); T.subtitlesPanel(true); return true; }
      if (act === 'next') { next(); return true; }
      return false;
    }
  });
  function next() {
    stopPlayback();
    PL.at++; PL.item = PL.list[PL.at]; PL.from = 0; PL.pb = null; T.playing = true; T.n('keepOn', true);
    T.render('pp');
  }

  // ---------------- Music (an album plays here, a small player stays while you browse) ----------------
  var MU = T.music = { album: null, tracks: [], i: 0, audio: null };
  function playAlbum(x) {
    M.tracks(x.id).then(function (ts) {
      if (!ts.length) { T.toast('No tracks in ' + x.title); return; }
      MU.album = x; MU.tracks = ts; MU.i = 0; startTrack(); T.toast('Playing ' + x.title); T.render();
    }).catch(function (e) { T.toast('Couldn\'t play: ' + e.message); });
  }
  function startTrack() {
    if (!MU.audio) { MU.audio = new Audio(); MU.audio.addEventListener('ended', function () { if (MU.i < MU.tracks.length - 1) { MU.i++; startTrack(); if (T.cur === 'media') T.render(); } }); }
    MU.audio.src = M.audioUrl(MU.tracks[MU.i].id); MU.audio.play().catch(function () {});
  }
  function musicBar() {
    if (!MU.album) return '';
    var t = MU.tracks[MU.i] || {}, paused = MU.audio && MU.audio.paused;
    return '<div class="panel" style="height:92px;flex:none;margin:0 64px 16px;display:flex;align-items:center;gap:22px;padding:0 24px">' +
      '<span style="width:60px;height:60px;border-radius:var(--r);background:var(--panel3);flex:none;position:relative;overflow:hidden">' + (MU.album.poster ? '<img class="art" src="' + E(MU.album.poster) + '" alt="">' : '') + '</span>' +
      '<span style="display:flex;flex-direction:column;gap:4px;flex:1;min-width:0"><span style="font-size:20px;font-weight:600">' + E(t.title) + '</span><span class="muted" style="font-size:16px">' + E(MU.album.title + (MU.album.artist ? ' · ' + MU.album.artist : '')) + '</span></span>' +
      '<button class="round" aria-label="Previous track" data-f data-key="mPrev" data-act="mPrev" style="width:52px;height:52px;background:var(--panel2);font-size:18px">⏮</button>' +
      '<button class="round" aria-label="' + (paused ? 'Play' : 'Pause') + '" data-f data-key="mPP" data-act="mPP" style="width:60px;height:60px;background:#dce5ed;color:#171a21;font-size:22px;font-weight:700">' + (paused ? '▶' : 'II') + '</button>' +
      '<button class="round" aria-label="Next track" data-f data-key="mNext" data-act="mNext" style="width:52px;height:52px;background:var(--panel2);font-size:18px">⏭</button>' +
      '<span class="muted" style="font-size:16px;width:260px;text-align:right">Keeps playing while you browse</span></div>';
  }
  function musicAct(act) {
    if (!MU.album) return false;
    if (act === 'mPP') { MU.audio.paused ? MU.audio.play() : MU.audio.pause(); T.render('mPP'); return true; }
    if (act === 'mPrev') { MU.i = (MU.i - 1 + MU.tracks.length) % MU.tracks.length; startTrack(); T.render('mPrev'); return true; }
    if (act === 'mNext') { MU.i = (MU.i + 1) % MU.tracks.length; startTrack(); T.render('mNext'); return true; }
    return false;
  }
})();
