/* Apps (mockup 3): on your Home screen (in order, move left/right, add/remove), other apps on this TV, what TV
   Cleanup switched off, and the options panel (Open, Add to/Remove from Home, Move, Clear the app's cache). */
(function () {
  'use strict';
  var T = window.hearthTV, E = T.esc;
  var A = { sel: null, all: [], toast: 'Apps you add to Home show up there in this order' };
  var HEARTH = { media: { label: 'My Media', bg: 'var(--panel3)', info: 'Hearth · shows, movies and music from your PC', go: 'media' }, theater: { label: 'Theater', bg: '#3d4f66', info: 'Hearth · your saved video links', go: 'theater' } };
  function homeList() { var h = T.homeApps(); return h.home.map(function (a) { return a.pkg; }); }
  function save(list) {
    var p = T.currentProfile();
    if (p.apps && p.apps.length) {
      var ps = T.profiles().map(function (x) { return x.id === p.id ? Object.assign({}, x, { apps: list }) : x; });
      T.set('profiles', ps);
    } else T.set('homeApps', list);
  }
  function offList() {
    var st = T.json('cleanupStatus') || {}, out = [], names = { tcl: 'TCL Channel and extras', ads: 'Sponsored apps and rows', acr: 'TCL screen tracking', home: 'Google TV home (backup)' };
    Object.keys(st).forEach(function (g) { (st[g].packages || []).forEach(function (p) { if (!p.on) out.push(p.pkg); }); if (st[g].present && !st[g].on) out.push(names[g] || g); });
    var seen = {}; return out.filter(function (x) { if (seen[x]) return false; seen[x] = 1; return names[x] || x.indexOf('.') < 0; }).slice(0, 6);
  }
  T.screen('apps', {
    first: 'h0',
    toast: A.toast,
    enter: function () { A.all = T.json('apps') || []; var h = homeList(); A.sel = A.sel || h[0] || (A.all[0] && A.all[0].pkg); },
    render: function () {
      var home = homeList(), by = {};
      A.all.forEach(function (a) { by[a.pkg] = a; });
      var others = A.all.filter(function (a) { return home.indexOf(a.pkg) < 0; });
      var tile = function (a, i, onHome) {
        var sel = A.sel === a.pkg;
        return '<button class="tile" data-f data-key="' + (onHome ? 'h' : 'o') + i + '" data-act="pick" data-arg="' + E(a.pkg) + '" style="height:112px;background:' + T.tileColor(a.pkg) + ';flex-direction:column;gap:4px;padding:0 10px;text-align:center' + (sel ? ';box-shadow:0 0 0 3px var(--bg),0 0 0 6px var(--focus)' : '') + '"><span style="font-size:20px;font-weight:700">' + E(a.label) + '</span><span style="font-size:14px;color:#c3ccd4;font-weight:400">' + (onHome ? (i + 1) + ' on Home' : '') + '</span></button>';
      };
      var out = '<div class="screen">' + T.header('apps') + '<div style="flex:1;display:flex;gap:48px;padding:16px 64px 0;min-height:0">';
      out += '<section data-scrolly style="flex:1;display:flex;flex-direction:column;gap:16px;min-width:0;overflow:hidden;padding:8px;margin:-8px">';
      out += '<div style="display:flex;align-items:baseline;gap:16px"><h2 style="font-size:26px;font-weight:700">On your Home screen</h2><span class="muted" style="font-size:17px">In this order · pick one to move it</span></div><div style="display:grid;grid-template-columns:repeat(6,minmax(0,1fr));gap:18px">';
      home.forEach(function (k, i) { if (by[k]) out += tile(by[k], i, true); });
      out += '</div><h2 style="margin:18px 0 0;font-size:26px;font-weight:700">Other apps on this TV</h2><div style="display:grid;grid-template-columns:repeat(6,minmax(0,1fr));gap:18px">';
      others.forEach(function (a, i) { out += tile(a, i, false); });
      out += '</div><div style="display:flex;align-items:baseline;gap:16px;margin-top:18px"><h2 class="muted" style="font-size:26px;font-weight:700">Switched off by TV Cleanup</h2><button data-f data-key="cleanupLink" data-act="cleanup" style="font-size:17px;color:var(--focus)">Change in Settings</button></div><div style="display:flex;gap:18px;flex-wrap:wrap">';
      var off = offList();
      if (!off.length) out += '<span class="muted" style="font-size:18px">Nothing yet</span>';
      off.forEach(function (o) { out += '<span style="height:64px;padding:0 22px;border-radius:var(--r);border:1.5px dashed var(--line);display:flex;align-items:center;font-size:18px;color:var(--muted)">' + E(o) + '</span>'; });
      out += '</div><div style="height:30px;flex:none"></div></section>';
      var a = by[A.sel] || A.all[0] || { label: '', pkg: '' }, idx = home.indexOf(a.pkg);
      out += '<aside aria-label="App options" class="panel on-panel" style="width:460px;flex:none;align-self:flex-start;padding:30px;box-sizing:border-box;display:flex;flex-direction:column;gap:12px">' +
        '<span class="tile" style="width:120px;height:68px;background:' + T.tileColor(a.pkg) + ';font-size:15px">' + E(a.label) + '</span>' +
        '<span style="font-size:30px;font-weight:700;padding-top:4px">' + E(a.label) + '</span><span class="muted" style="font-size:17px;padding-bottom:8px">Installed app' + (a.version ? ' · version ' + E(a.version) : '') + '</span>' +
        '<button class="btn small primary" style="height:58px" data-f data-key="aOpen" data-act="open">Open</button>' +
        '<button class="btn small" data-f data-key="aHome" data-act="toggleHome">' + (idx >= 0 ? 'Remove from Home' : 'Add to Home') + '</button>' +
        (idx >= 0 ? '<div style="display:flex;gap:10px"><button class="btn small" style="flex:1" data-f data-key="aLeft" data-act="move" data-arg="-1">◀ Move left</button><button class="btn small" style="flex:1" data-f data-key="aRight" data-act="move" data-arg="1">Move right ▶</button></div>' : '') +
        '<button class="btn small" data-f data-key="aCache" data-act="cache">Clear the app\'s cache</button>' +
        '<span class="muted" style="font-size:15px;line-height:1.5;padding-top:6px">Uninstalling and app permissions stay in the TV\'s own settings, so nothing here can break the TV.</span></aside>';
      out += '</div>' + T.footer([['OK', 'Pick'], ['☰', 'Options'], ['↩', 'Back to Home', 'go', 'home']], A.toast) + '</div>';
      return out;
    },
    act: function (act, arg) {
      var home = homeList(), a = null;
      A.all.forEach(function (x) { if (x.pkg === A.sel) a = x; });
      if (act === 'pick') { A.sel = arg; T.render(); return true; }
      if (act === 'cleanup') { T.go('settings', 'TV Cleanup'); return true; }
      if (!a) return false;
      if (act === 'open') { if (!T.n('appOpen', a.pkg)) T.toast('That app couldn\'t be opened'); return true; }
      if (act === 'toggleHome') {
        var i = home.indexOf(a.pkg);
        if (i >= 0) { home.splice(i, 1); save(home); T.toast(a.label + ' removed from Home (still installed)'); }
        else { home.push(a.pkg); save(home); T.toast(a.label + ' added to the end of Home'); }
        T.render('aHome'); return true;
      }
      if (act === 'move') {
        var j = home.indexOf(a.pkg); if (j < 0) return true;
        var to = Math.max(0, Math.min(home.length - 1, j + Number(arg)));
        home.splice(j, 1); home.splice(to, 0, a.pkg); save(home);
        T.toast(a.label + ' is now number ' + (to + 1) + ' on Home'); T.render(Number(arg) < 0 ? 'aLeft' : 'aRight'); return true;
      }
      if (act === 'cache') {
        T.call('appClearCache', a.pkg).then(function (r) { T.toast(r && r.ok ? a.label + '\'s cache cleared' : 'The TV\'s page for ' + a.label + ' is open: Clear cache is there'); });
        return true;
      }
      return false;
    }
  });
})();
