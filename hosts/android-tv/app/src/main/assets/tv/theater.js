/* Hearth Theater (mockup 6): saved links, filters, play full screen, remove, Add a link (QR for your phone, a Hearth
   PC, or type it). YouTube plays in Hearth TV's own full-screen player; other sites open full screen in the viewer. */
(function () {
  'use strict';
  var T = window.hearthTV, E = T.esc;
  var TH = { chip: 'All', typing: false, draft: '', add: null, playing: null };
  function links() { return T.json('theater') || []; }
  function save(list) { T.n('theaterSave', JSON.stringify(list)); }
  function shown() {
    return links().filter(function (x) { return TH.chip === 'All' || (TH.chip === 'Watched' ? x.pct >= 97 || x.watched : x.kind === TH.chip); });
  }
  T.theaterPlay = function (x) {
    if (!x) return;
    var list = links(); list.forEach(function (y) { if (y.id === x.id) { y.opened = Date.now(); if (!y.pct) y.pct = 1; } }); save(list);
    if (x.yt || x.list) { TH.playing = x; T.go('theaterPlayer', x); }
    else T.n('openViewer', x.url, x.title || '');
  };
  T.screen('theater', {
    first: 't0',
    toast: 'The same saved links as Hearth Theater on your PC',
    enter: function () { TH.typing = false; TH.add = T.json('addPage'); },
    added: function () { if (T.cur === 'theater' && !T.layer) T.render(); },
    render: function () {
      var out = '<div class="screen">' + T.header('theater') + '<div style="flex:1;display:flex;gap:48px;padding:20px 64px 0;min-height:0">';
      out += '<section data-scrolly style="flex:1;display:flex;flex-direction:column;gap:22px;min-width:0;overflow:hidden;padding:8px;margin:-8px">' +
        '<div style="display:flex;flex-direction:column;gap:8px"><h1 style="font-size:44px;font-weight:700;letter-spacing:-.01em">Hearth Theater</h1><span class="muted" style="font-size:20px">Links you\'ve saved, played full screen with no page around them</span></div><div style="display:flex;gap:10px">';
      ['All', 'Videos', 'Live streams', 'Playlists', 'Watched'].forEach(function (c, i) { out += '<button class="chip' + (TH.chip === c ? ' on' : '') + '" data-f data-key="c' + i + '" data-act="chip" data-arg="' + c + '">' + c + '</button>'; });
      out += '</div><div style="display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:26px 22px;padding-bottom:40px">';
      var list = shown();
      if (!list.length) out += '<span class="muted" style="font-size:20px;padding-top:30px;grid-column:1/-1">Nothing saved here yet.</span>';
      list.forEach(function (x, i) {
        var live = x.len === 'LIVE';
        out += '<button class="card" style="min-width:0" data-f data-key="t' + i + '" data-act="play" data-arg="' + E(x.id) + '" data-menu="' + E(x.id) + '"><span class="pic" style="width:100%;aspect-ratio:16/9">' +
          (x.image ? '<img class="art" src="' + E(x.image) + '" alt="" loading="lazy">' : '<span class="notart" style="background:var(--panel3)"><b>' + E(x.title) + '</b></span>') +
          (x.len ? '<span class="badge br" style="font-weight:700;' + (live ? 'background:var(--danger);color:#fff' : '') + '">' + E(x.len) + '</span>' : '') +
          (x.pct > 0 && x.pct < 97 ? '<span class="bar"><span style="width:' + Math.round(x.pct) + '%"></span></span>' : '') +
          '</span><span class="t">' + E(x.title) + '</span><span class="s">' + E(x.site + (x.by ? ' · ' + x.by : '')) + '</span></button>';
      });
      out += '</div></section>';
      var a = TH.add || {};
      out += '<aside aria-label="Add a link" class="panel on-panel" style="width:440px;flex:none;align-self:flex-start;margin-top:8px;padding:32px;box-sizing:border-box;display:flex;flex-direction:column;gap:20px">' +
        '<span style="font-size:26px;font-weight:700">Add a link</span><span class="soft" style="font-size:18px;line-height:1.5">Typing links with a remote is slow, so send them from your phone or PC instead.</span>' +
        '<div style="display:flex;gap:20px;align-items:center">' + (a.qr ? '<img src="' + E(a.qr) + '" alt="QR code to add a link" style="width:150px;height:150px;border-radius:var(--r);flex:none">' : '<div style="width:150px;height:150px;border-radius:var(--r);background:var(--panel2);flex:none;display:flex;align-items:center;justify-content:center;font-size:15px;text-align:center" class="muted">Connect the TV to your network</div>') +
        '<span class="soft" style="font-size:17px;line-height:1.5">Scan with your phone and paste the link there</span></div><div style="height:1px;background:var(--line)"></div>' +
        '<span class="soft" style="display:flex;gap:14px;align-items:flex-start;font-size:17px;line-height:1.5">' + T.ICON.pc.replace(/30/g, '24') + 'On any Hearth PC: right-click a link › Send to TV, through Hearth Link</span>' +
        '<span class="soft" style="display:flex;gap:14px;align-items:flex-start;font-size:17px;line-height:1.5">' + T.ICON.phone + 'On your phone: share any video to Hearth TV</span>';
      if (TH.typing) {
        out += '<label for="tv-link" class="muted" style="font-size:16px">Video link</label><input id="tv-link" data-f data-key="linkIn" data-enter="linkSave" value="' + E(TH.draft) + '" placeholder="https://" inputmode="url" style="height:56px;border-radius:var(--r);border:2px solid var(--focus);background:var(--bg);color:var(--text);padding:0 16px;font-size:19px;box-sizing:border-box;margin-top:-10px;font-family:inherit">' +
          '<div style="display:flex;gap:12px"><button class="btn small primary" style="flex:1" data-f data-key="linkSave" data-act="save">Save to Theater</button><button class="btn small" data-f data-key="linkCancel" data-act="cancel">Cancel</button></div>';
      } else out += '<button class="btn small" data-f data-key="type" data-act="type">Type a link instead</button>';
      out += '</aside></div>' + T.footer([['OK', 'Play full screen'], ['☰', 'Remove or rename'], ['↩', 'Back to Home', 'go', 'home']], 'The same saved links as Hearth Theater on your PC') + '</div>';
      return out;
    },
    after: function () {
      var inp = document.getElementById('tv-link');
      if (inp) inp.addEventListener('input', function () { TH.draft = inp.value; });
    },
    act: function (act, arg) {
      if (act === 'chip') { TH.chip = arg; T.render(); return true; }
      if (act === 'play') { var x = links().filter(function (y) { return y.id === arg; })[0]; T.theaterPlay(x); return true; }
      if (act === 'type') { TH.typing = true; TH.draft = ''; T.render('linkIn'); return true; }
      if (act === 'cancel') { TH.typing = false; T.render('type'); return true; }
      if (act === 'save') {
        var url = (TH.draft || '').trim();
        if (!/^https?:\/\//i.test(url)) url = url ? 'https://' + url : '';
        if (!url) { T.toast('Paste or type a link first'); return true; }
        T.n('theaterAdd', url); TH.typing = false; TH.chip = 'All'; T.toast('Saving ' + url.replace(/^https?:\/\/(www\.)?/, '').split('/')[0] + ' to Theater…'); T.render('type'); return true;
      }
      return false;
    },
    menu: function (id) {
      var x = links().filter(function (y) { return y.id === id; })[0]; if (!x) return;
      T.open({
        id: 'tmenu', first: 'tmPlay',
        render: function () {
          return '<div class="dim"><div class="dialog on-panel" style="width:640px"><span style="font-size:28px;font-weight:700">' + E(x.title) + '</span><span class="muted" style="font-size:18px">' + E(x.site) + '</span>' +
            '<button class="btn small primary" data-f data-key="tmPlay" data-act="tp">Play full screen</button>' +
            '<button class="btn small" data-f data-key="tmWatched" data-act="tw">' + (x.pct >= 97 || x.watched ? 'Mark not watched' : 'Mark watched') + '</button>' +
            '<button class="btn small danger" data-f data-key="tmRemove" data-act="tr">Remove from Theater</button>' +
            '<button class="btn small" data-f data-key="tmBack" data-act="close">↩ Back</button></div></div>';
        },
        act: function (act) {
          var list = links();
          if (act === 'tp') { T.close(); T.theaterPlay(x); return true; }
          if (act === 'tw') { list.forEach(function (y) { if (y.id === x.id) { var w = y.pct >= 97 || y.watched; y.watched = !w; y.pct = w ? 0 : 100; } }); save(list); T.close(); T.render(); return true; }
          if (act === 'tr') { save(list.filter(function (y) { return y.id !== x.id; })); T.close(); T.toast('Removed "' + x.title + '" from Theater'); T.render(); return true; }
          return false;
        }
      });
    }
  });

  // YouTube links: their own player, full screen, with the remote (OK play/pause, ◀ ▶ 10 s).
  T.screen('theaterPlayer', {
    first: 'tpp',
    enter: function () { T.playing = true; T.n('keepOn', true); },
    leave: function () {
      T.playing = false; T.n('keepOn', false);
      var x = TH.playing; if (!x) return;
      var list = links(); list.forEach(function (y) { if (y.id === x.id && TH.pos && TH.dur) y.pct = Math.min(100, TH.pos / TH.dur * 100); }); save(list);
    },
    render: function (x) {
      var src = 'https://www.youtube-nocookie.com/embed/' + (x.yt ? encodeURIComponent(x.yt) : 'videoseries') + '?autoplay=1&controls=1&rel=0&modestbranding=1&enablejsapi=1&playsinline=1' + (x.list ? '&list=' + encodeURIComponent(x.list) : '') + (x.pct > 0 && x.pct < 97 && x.durSec ? '&start=' + Math.floor(x.durSec * x.pct / 100) : '');
      return '<div class="player"><iframe id="yt" src="' + E(src) + '" allow="autoplay; encrypted-media; fullscreen" allowfullscreen style="position:absolute;inset:0;width:100%;height:100%;border:0"></iframe>' +
        '<div class="chrome" style="pointer-events:none"><div class="topfade"></div><div style="position:absolute;left:80px;top:56px;display:flex;flex-direction:column;gap:6px"><span style="font-size:32px;font-weight:700">' + E(x.title) + '</span><span style="font-size:20px;color:#c3ccd4">' + E(x.site) + ' · full screen, no page around it</span></div></div>' +
        '<button data-f data-key="tpp" data-act="pp" style="position:absolute;left:-9999px">Play/pause</button></div>';
    },
    after: function () {
      setTimeout(function () { var c = document.querySelector('.player .chrome'); if (c) c.classList.add('hidden'); }, 4000);
      window.addEventListener('message', ytMsg);
      var f = document.getElementById('yt');
      f.addEventListener('load', function () { f.contentWindow.postMessage(JSON.stringify({ event: 'listening' }), '*'); });
    },
    key: function (k) { if (k === 'playpause') { yt(TH.paused ? 'playVideo' : 'pauseVideo'); TH.paused = !TH.paused; return true; } if (k === 'ff') { yt('seekTo', [(TH.pos || 0) + 30, true]); return true; } if (k === 'rew') { yt('seekTo', [Math.max(0, (TH.pos || 0) - 10), true]); return true; } return false; },
    act: function (act) { if (act === 'pp') { yt(TH.paused ? 'playVideo' : 'pauseVideo'); TH.paused = !TH.paused; return true; } return false; }
  });
  T.screens.theaterPlayer.keyNav = true;
  function yt(fn, args) { var f = document.getElementById('yt'); if (f) f.contentWindow.postMessage(JSON.stringify({ event: 'command', func: fn, args: args || [] }), '*'); }
  function ytMsg(e) {
    try { var d = JSON.parse(e.data); if (d.info && typeof d.info.currentTime === 'number') { TH.pos = d.info.currentTime; if (d.info.duration) TH.dur = d.info.duration; } } catch (x) {}
  }
  // ◀ ▶ skip 10 s in YouTube's player.
  document.addEventListener('keydown', function (e) {
    if (T.cur !== 'theaterPlayer' || T.layer) return;
    if (e.key === 'ArrowLeft') { e.stopPropagation(); e.preventDefault(); yt('seekTo', [Math.max(0, (TH.pos || 0) - 10), true]); }
    if (e.key === 'ArrowRight') { e.stopPropagation(); e.preventDefault(); yt('seekTo', [(TH.pos || 0) + 10, true]); }
  }, true);
})();
