/* The screensaver (mockup 9): pictures fading one into the next, the clock and the date. Any button wakes it.
   Also the TV's own screensaver (HearthDream.kt loads this page with #screensaver). */
(function () {
  'use strict';
  var T = window.hearthTV, E = T.esc;
  var SS = { list: [], i: 0, iv: null };
  var SPACE = [{ img: 'img/space-hubble-deep-field.jpg', title: 'Hubble Deep Field', src: 'Space photos · NASA, public domain' }, { img: 'img/space-rocket.jpg', title: 'Launch pad at night', src: 'Space photos · public domain' }];
  function wallpapers() {
    var w = ['slate-horizon', 'slate-dusk', 'slate-ridges', 'slate-nebula', 'slate-facets', 'slate-contours'].map(function (n) { return { img: 'theme/wallpapers/' + n + '.jpg', title: n.replace('slate-', 'Slate ').replace(/^\w/, function (c) { return c.toUpperCase(); }), src: 'Hearth OS wallpaper' }; });
    ['hearth-embers', 'hearth-cabin', 'hearth-peaks', 'hearth-fireside', 'hearth-lanterns', 'hearth-ridge'].forEach(function (n) { w.push({ img: 'theme/backgrounds/' + n + '.jpg', title: n.replace('hearth-', 'Hearth ').replace(/\b\w/g, function (c) { return c.toUpperCase(); }), src: 'Hearth OS wallpaper' }); });
    return w;
  }
  function load() {
    var kind = T.settings.saver.pictures;
    if (kind === 'space') { SS.list = SPACE; return Promise.resolve(); }
    if (kind === 'wallpapers' || !T.Media || !T.Media.ready) { SS.list = wallpapers(); return Promise.resolve(); }
    return T.Media.latest().then(function (r) {
      SS.list = r.filter(function (x) { return x.backdrop; }).slice(0, 12).map(function (x) { return { img: x.backdrop, title: x.kind === 'episode' ? x.seriesName : x.title, src: 'From My Media' }; });
      if (!SS.list.length) SS.list = wallpapers();
    }).catch(function () { SS.list = wallpapers(); });
  }
  T.screen('screensaver', {
    enter: function () { SS.i = 0; SS.list = []; load().then(function () { if (T.cur === 'screensaver') T.render(); }); },
    leave: function () { clearInterval(SS.iv); },
    back: function () { if (T.dream) return true; return false; },
    render: function () {
      var d = new Date(), h = d.getHours(), m = d.getMinutes(), cur = SS.list[SS.i] || {};
      var out = '<div class="screen" style="background:#000;display:block">';
      SS.list.forEach(function (x, j) { out += '<img class="art" data-slide="' + j + '" src="' + E(x.img) + '" alt="" style="opacity:' + (j === SS.i ? 1 : 0) + ';transform:scale(' + (j === SS.i ? 1.06 : 1) + ');transition:opacity 1.2s ease,transform 8s linear">'; });
      out += '<div style="position:absolute;left:0;right:0;bottom:0;height:320px;background:linear-gradient(0deg,rgba(0,0,0,.7),rgba(0,0,0,0))"></div>';
      if (T.settings.saver.clock !== false) out += '<div style="position:absolute;left:80px;bottom:70px;display:flex;flex-direction:column;gap:4px;color:#f2f6fa"><span data-ssclock style="font-size:120px;font-weight:300;letter-spacing:-.03em;line-height:1;font-variant-numeric:tabular-nums">' + ((h % 12) || 12) + ':' + (m < 10 ? '0' : '') + m + '</span><span data-ssdate style="font-size:28px;color:#dce5ed">' + d.toLocaleDateString('en-US', { weekday: 'long', month: 'long', day: 'numeric' }) + '</span></div>';
      out += '<div style="position:absolute;right:80px;bottom:76px;display:flex;flex-direction:column;align-items:flex-end;gap:6px;text-align:right;color:#f2f6fa"><span data-sstitle style="font-size:26px;font-weight:600">' + E(cur.title || '') + '</span><span data-sssrc style="font-size:19px;color:#c3ccd4">' + E(cur.src || '') + '</span><div data-ssdots style="display:flex;gap:6px;padding-top:10px">';
      SS.list.forEach(function (x, j) { out += '<span style="display:block;height:6px;border-radius:3px;width:' + (j === SS.i ? 28 : 10) + 'px;background:' + (j === SS.i ? '#f2f6fa' : 'rgba(242,246,250,.4)') + ';transition:width .3s"></span>'; });
      out += '</div></div>' + (T.dream ? '' : '<div style="position:absolute;left:0;right:0;top:48px;display:flex;justify-content:center"><span style="font-size:18px;color:rgba(242,246,250,.75);background:rgba(0,0,0,.35);border-radius:20px;padding:8px 18px">Press any button to wake up</span></div>') + '</div>';
      return out;
    },
    after: function () {
      clearInterval(SS.iv);
      SS.iv = setInterval(function () {
        if (!SS.list.length) return;
        SS.i = (SS.i + 1) % SS.list.length;
        [].forEach.call(document.querySelectorAll('[data-slide]'), function (el) { var j = Number(el.getAttribute('data-slide')); el.style.opacity = j === SS.i ? 1 : 0; el.style.transform = 'scale(' + (j === SS.i ? 1.06 : 1) + ')'; });
        var cur = SS.list[SS.i], t = document.querySelector('[data-sstitle]'), s = document.querySelector('[data-sssrc]'), c = document.querySelector('[data-ssclock]');
        if (t) t.textContent = cur.title || ''; if (s) s.textContent = cur.src || '';
        var d = new Date(); if (c) c.textContent = ((d.getHours() % 12) || 12) + ':' + (d.getMinutes() < 10 ? '0' : '') + d.getMinutes();
        var dots = document.querySelector('[data-ssdots]'); if (dots) [].forEach.call(dots.children, function (el, j) { el.style.width = (j === SS.i ? 28 : 10) + 'px'; el.style.background = j === SS.i ? '#f2f6fa' : 'rgba(242,246,250,.4)'; });
      }, 6000);
    }
  });
})();
