/* Search (mockup 7): the on-screen keyboard, voice search (the remote's mic), results from your PC, Theater and your
   apps, and "search for it in another app". */
(function () {
  'use strict';
  var T = window.hearthTV, E = T.esc;
  var SR = { q: '', media: [], seq: 0 };
  function recent() { return (T.state.recentSearches || []).slice(0, 5); }
  function remember(q) { q = q.trim(); if (!q) return; var r = recent().filter(function (x) { return x !== q; }); r.unshift(q); T.setState({ recentSearches: r.slice(0, 5) }); }
  function runMedia() {
    var q = SR.q.trim(), n = ++SR.seq;
    if (!q || !T.Media.ready) { SR.media = []; return; }
    T.Media.search(q).then(function (r) { if (n === SR.seq) { SR.media = r; if (T.cur === 'search') T.render(); } }).catch(function () {});
  }
  var typing;
  function changed() { clearTimeout(typing); typing = setTimeout(function () { runMedia(); }, 350); T.render(); }
  T.screen('search', {
    first: 'ka',
    enter: function () { SR.media = []; if (SR.q) runMedia(); },
    voice: function (text) { if (!text) { T.toast('Didn\'t catch that'); return; } SR.q = text; remember(text); T.toast('Heard: "' + text + '"'); runMedia(); if (T.cur !== 'search') T.go('search'); else T.render(); },
    render: function () {
      var q = SR.q.trim().toLowerCase(), apps = T.json('apps') || [];
      var groups = [];
      if (q) {
        groups.push(['My Media', SR.media.slice(0, 4).map(function (x) { return { title: x.title, src: (x.kind === 'show' ? 'Show' : x.kind === 'album' ? 'Album' : 'Movie') + ' · My PC', img: x.img, act: 'media', x: x }; })]);
        groups.push(['Theater', (T.json('theater') || []).filter(function (x) { return (x.title + ' ' + x.site).toLowerCase().indexOf(q) >= 0; }).slice(0, 4).map(function (x) { return { title: x.title, src: x.site, img: x.image, act: 'theater', x: x }; })]);
        groups.push(['Apps', apps.filter(function (a) { return a.label.toLowerCase().indexOf(q) >= 0; }).slice(0, 4).map(function (a) { return { title: a.label, src: 'App', tint: T.tileColor(a.pkg), act: 'app', x: a }; })
          .concat('settings'.indexOf(q) >= 0 && !T.kids() ? [{ title: 'Settings', src: 'Hearth TV', tint: 'var(--panel2)', act: 'settings' }] : [])]);
        groups = groups.filter(function (g) { return g[1].length; });
      }
      var total = groups.reduce(function (n, g) { return n + g[1].length; }, 0);
      SR.items = {};
      var out = '<div class="screen"><div style="flex:1;display:flex;min-height:0"><section aria-label="Type to search" style="width:720px;flex:none;background:var(--bg2);padding:48px 48px 0;box-sizing:border-box;display:flex;flex-direction:column;gap:22px">' +
        '<div style="display:flex;align-items:center;gap:14px"><button aria-label="Back to Home" data-f data-key="sBack" data-act="go" data-arg="home" style="width:52px;height:52px;border-radius:var(--r);background:var(--panel2);display:flex;align-items:center;justify-content:center;font-size:22px">↩</button><span style="font-size:34px;font-weight:700">Search</span></div>' +
        '<div style="height:72px;border-radius:var(--r);background:var(--bg);border:2px solid var(--focus);display:flex;align-items:center;gap:14px;padding:0 20px"><span class="muted">' + T.ICON.search + '</span><span style="font-size:30px;font-weight:600;flex:1;white-space:nowrap;overflow:hidden">' + E(SR.q) + '<span style="display:inline-block;width:3px;height:32px;background:var(--focus);vertical-align:middle;margin-left:2px"></span></span><span class="muted" style="font-size:16px">' + (SR.q ? '' : 'Shows, movies, links, apps') + '</span></div>' +
        '<div class="keys7">';
      'abcdefghijklmnopqrstuvwxyz1234567890'.split('').forEach(function (c) { out += '<button class="key" data-f data-key="k' + c + '" data-act="k" data-arg="' + c + '">' + c + '</button>'; });
      out += '</div><div style="display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:8px"><button class="btn small" style="height:60px" data-f data-key="kSpace" data-act="space">Space</button><button class="btn small" style="height:60px" data-f data-key="kDel" data-act="del">⌫ Delete</button><button class="btn small" style="height:60px" data-f data-key="kClear" data-act="clear">Clear</button></div>' +
        '<button class="btn small" style="height:64px" data-f data-key="kVoice" data-act="voice">' + T.ICON.mic + 'Voice search</button><div style="display:flex;flex-wrap:wrap;gap:10px;align-items:center"><span class="muted" style="font-size:16px">Recent:</span>';
      recent().forEach(function (r, i) { out += '<button class="chip" style="height:40px;padding:0 16px;border-radius:20px;font-size:16px" data-f data-key="rc' + i + '" data-act="recent" data-arg="' + E(r) + '">' + E(r) + '</button>'; });
      out += '</div></section><section aria-label="Results" data-scrolly style="flex:1;padding:48px 64px 0;display:flex;flex-direction:column;gap:26px;min-width:0;overflow:hidden"><span class="muted" style="font-size:22px">' +
        E(q ? (total ? total + (total === 1 ? ' result' : ' results') + ' for "' + SR.q + '"' : 'Nothing on your PC, in Theater or in your apps matches "' + SR.q + '"') : 'Start typing, or hold the mic button on the remote') + '</span>';
      groups.forEach(function (g, gi) {
        out += '<div style="display:flex;flex-direction:column;gap:14px"><h2 class="row">' + E(g[0]) + '</h2><div style="display:flex;gap:20px;flex-wrap:wrap">';
        g[1].forEach(function (it, i) {
          var k = 'r' + gi + '_' + i; SR.items[k] = it;
          out += '<button class="card" style="width:300px" data-f data-key="' + k + '" data-act="res" data-arg="' + k + '"><span class="pic" style="width:300px;height:169px;background:' + (it.tint || 'var(--panel2)') + '">' +
            (it.img ? '<img class="art" src="' + E(it.img) + '" alt="" loading="lazy">' : '<span style="position:absolute;inset:0;display:flex;align-items:center;justify-content:center;font-size:24px;font-weight:700;padding:16px;text-align:center;box-sizing:border-box;color:#f2f6fa">' + E(it.title) + '</span>') +
            '<span class="badge" style="left:10px;top:10px">' + E(it.src) + '</span></span><span class="t" style="font-size:18px">' + E(it.title) + '</span></button>';
        });
        out += '</div></div>';
      });
      var other = [['YouTube', '#5c2424', 'com.google.android.youtube.tv'], ['Netflix', '#5a1f22', 'com.netflix.ninja'], ['Prime Video', '#1d3d55', 'com.amazon.amazonvideo.livingroom'], ['Disney+', '#1f3160', 'com.disney.disneyplus']]
        .filter(function (o) { return !T.N || apps.some(function (a) { return a.pkg === o[2]; }); });
      if (other.length) {
        out += '<div style="display:flex;flex-direction:column;gap:12px;padding-bottom:30px"><h2 class="row">Search for "' + E(SR.q || '…') + '" in another app</h2><div style="display:flex;gap:14px">';
        other.forEach(function (o, i) { out += '<button class="btn small" style="background:' + o[1] + ';color:#f2f6fa" data-f data-key="oa' + i + '" data-act="elsewhere" data-arg="' + o[2] + '">' + o[0] + '</button>'; });
        out += '</div></div>';
      }
      out += '</section></div>' + T.footer([['OK', 'Type'], ['▶', 'Jump to results'], ['🎤', 'Hold to talk'], ['↩', 'Back to Home', 'go', 'home']], 'Searches your PC, your Theater links and your apps at once') + '</div>';
      return out;
    },
    key: function (k, e) {
      // A keyboard (or a remote with letters) types straight in.
      if (e && k && k.length === 1 && /[\w ]/.test(k)) { SR.q = (SR.q + k.toLowerCase()).slice(0, 40); changed(); return true; }
      if (e && k === 'Backspace') { SR.q = SR.q.slice(0, -1); changed(); return true; }
      return false;
    },
    act: function (act, arg) {
      if (act === 'k') { SR.q = (SR.q + arg).slice(0, 40); changed(); return true; }
      if (act === 'space') { SR.q += ' '; changed(); return true; }
      if (act === 'del') { SR.q = SR.q.slice(0, -1); changed(); return true; }
      if (act === 'clear') { SR.q = ''; SR.media = []; T.render(); return true; }
      if (act === 'recent') { SR.q = arg; runMedia(); T.render(); return true; }
      if (act === 'voice') { if (!T.n('voice')) T.toast('Voice search isn\'t available on this TV'); return true; }
      if (act === 'elsewhere') { remember(SR.q); if (!T.n('appSearch', arg, SR.q.trim())) T.toast('That app couldn\'t be opened'); return true; }
      if (act === 'res') {
        var it = SR.items[arg]; if (!it) return true; remember(SR.q);
        if (it.act === 'media') T.openItem({ item: it.x, src: it.x.src, kind: it.x.kind }, false);
        else if (it.act === 'theater') T.theaterPlay(it.x);
        else if (it.act === 'app') T.n('appOpen', it.x.pkg);
        else if (it.act === 'settings') T.go('settings');
        return true;
      }
      return false;
    }
  });
})();
