// Hearth OS docs: a Copy button and "Did this work?" votes on every command (.cmd[data-cmd]).
// Votes go to the Hearth OS counter (counter/src/index.js: /v1/cmdvote, /v1/cmdvotes): one per browser and
// command, changeable, with a random id kept in this browser. Nothing else is sent.
(function(){
  var API = 'https://hearth-counter.game305-cl.workers.dev';
  var THUMB_UP = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 10v11H3V10h4zm0 0l4-8a3 3 0 0 1 3 3v4h5.5a2 2 0 0 1 2 2.3l-1.3 8A2 2 0 0 1 18.2 21H7"/></svg>';
  var THUMB_DOWN = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M17 14V3h4v11h-4zm0 0l-4 8a3 3 0 0 1-3-3v-4H4.5a2 2 0 0 1-2-2.3l1.3-8A2 2 0 0 1 5.8 3H17"/></svg>';
  var COPY = '<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="9" y="9" width="12" height="12" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h10"/></svg>';

  function store(k, v){ try{ if(v === undefined) return localStorage.getItem(k); if(v === null) localStorage.removeItem(k); else localStorage.setItem(k, v); }catch(e){ return null; } }
  function deviceId(){
    var id = store('hearth.docs.id');
    if(!id || !/^[a-f0-9]{32}$/.test(id)){
      var b = new Uint8Array(16); (window.crypto || window.msCrypto).getRandomValues(b);
      id = Array.prototype.map.call(b, function(x){ return ('0' + x.toString(16)).slice(-2); }).join('');
      store('hearth.docs.id', id);
    }
    return id;
  }

  var blocks = Array.prototype.slice.call(document.querySelectorAll('.cmd[data-cmd]'));
  blocks.forEach(function(el){
    var name = el.getAttribute('data-cmd');
    var bar = document.createElement('div'); bar.className = 'bar';
    bar.innerHTML = '<span class="q">Did this work for you?</span>'
      + '<button type="button" class="up" aria-pressed="false" aria-label="Yes, it worked">' + THUMB_UP + '<span class="n">·</span></button>'
      + '<button type="button" class="down" aria-pressed="false" aria-label="No, it didn\'t work">' + THUMB_DOWN + '<span class="n">·</span></button>'
      + '<button type="button" class="copy">' + COPY + '<span>Copy</span></button>';
    el.appendChild(bar);
    var mine = Number(store('hearth.docs.v.' + name) || 0);
    el._mine = mine; paintMine(el);
    bar.querySelector('.copy').addEventListener('click', function(){
      var text = el.querySelector('code').innerText.replace(/\n$/, '');
      var btn = this, done = function(){ btn.classList.add('done'); btn.lastChild.textContent = 'Copied'; setTimeout(function(){ btn.classList.remove('done'); btn.lastChild.textContent = 'Copy'; }, 1600); };
      if(navigator.clipboard && navigator.clipboard.writeText) navigator.clipboard.writeText(text).then(done, function(){});
      else { var t = document.createElement('textarea'); t.value = text; document.body.appendChild(t); t.select(); try{ document.execCommand('copy'); done(); }catch(e){} t.remove(); }
    });
    bar.querySelector('.up').addEventListener('click', function(){ vote(el, name, el._mine === 1 ? 0 : 1); });
    bar.querySelector('.down').addEventListener('click', function(){ vote(el, name, el._mine === -1 ? 0 : -1); });
  });

  function paintMine(el){
    el.querySelector('.up').setAttribute('aria-pressed', el._mine === 1 ? 'true' : 'false');
    el.querySelector('.down').setAttribute('aria-pressed', el._mine === -1 ? 'true' : 'false');
  }
  function paintCounts(counts){
    blocks.forEach(function(el){
      var c = counts[el.getAttribute('data-cmd')] || {up: 0, down: 0};
      el.querySelector('.up .n').textContent = c.up;
      el.querySelector('.down .n').textContent = c.down;
    });
  }
  function vote(el, name, v){
    var before = el._mine;
    el._mine = v; paintMine(el);
    store('hearth.docs.v.' + name, v ? String(v) : null);
    // Show it straight away; the counter's answer corrects it.
    var up = el.querySelector('.up .n'), down = el.querySelector('.down .n');
    var nu = Number(up.textContent) || 0, nd = Number(down.textContent) || 0;
    if(before === 1) nu--; if(before === -1) nd--; if(v === 1) nu++; if(v === -1) nd++;
    up.textContent = Math.max(0, nu); down.textContent = Math.max(0, nd);
    fetch(API + '/v1/cmdvote', {method: 'POST', headers: {'content-type': 'application/json'}, body: JSON.stringify({id: deviceId(), cmd: name, v: v})})
      .then(function(r){ return r.json(); }).then(function(j){ if(j && j.counts) paintCounts(j.counts); }).catch(function(){});
  }
  if(blocks.length){
    fetch(API + '/v1/cmdvotes').then(function(r){ return r.json(); }).then(function(j){ paintCounts(j.counts || {}); })
      .catch(function(){ blocks.forEach(function(el){ el.querySelectorAll('.n').forEach(function(n){ n.textContent = '0'; }); }); });
  }

  // The edition picker: what doesn't apply to the chosen edition is hidden on every page (remembered; ?ed= in a link
  // picks it too). data-ed="all" parts only show with All editions.
  var NAMES = {anatase: 'Anatase', ubuntu: 'Ubuntu', cachyos: 'CachyOS', peppermint: 'Peppermint'};
  function chosen(){
    var q = (location.search.match(/[?&]ed=([a-z]+)/) || [])[1];
    if(q && (NAMES[q] || q === 'all')){ store('hearth.docs.ed', q); return q; }
    var s = store('hearth.docs.ed'); return NAMES[s] ? s : 'all';
  }
  function applyEdition(ed){
    var hidden = 0;
    Array.prototype.forEach.call(document.querySelectorAll('[data-ed]'), function(el){
      var list = el.getAttribute('data-ed').split(/\s+/);
      var off = list[0] === 'all' ? ed !== 'all' : (ed !== 'all' && list.indexOf(ed) < 0);
      el.classList.toggle('ed-off', off);
      if(off && el.tagName === 'SECTION') hidden++;
    });
    var sel = document.querySelector('.picker select'); if(sel){ sel.value = ed; sel.parentNode.classList.toggle('on', ed !== 'all'); }
    var show = document.querySelector('.edshow');
    if(show){
      show.hidden = ed === 'all';
      show.hidden = ed === 'all' || !hidden;
      if(ed !== 'all' && hidden) show.innerHTML = hidden + ' section' + (hidden > 1 ? 's' : '') + ' for other editions hidden · <button type="button">show all</button>';
      var b = show.querySelector('button'); if(b) b.onclick = function(){ pick('all'); };
    }
  }
  function pick(ed){ store('hearth.docs.ed', ed === 'all' ? null : ed); applyEdition(ed); if(typeof follow === 'function') follow(); }
  var selEl = document.querySelector('.picker select'); if(selEl) selEl.addEventListener('change', function(){ pick(selEl.value); });
  applyEdition(chosen());

  // The section list follows the reading position: the last heading above the top of the screen.
  var links = Array.prototype.slice.call(document.querySelectorAll('.side .sub a[href^="#"]'));
  var heads = links.map(function(a){ return document.getElementById(a.getAttribute('href').slice(1)); });
  var ticking = false;
  function follow(){
    ticking = false;
    var cur = 0;
    for(var i = 0; i < heads.length; i++){ if(heads[i] && heads[i].offsetParent && heads[i].getBoundingClientRect().top < 130) cur = i; }
    if(window.innerHeight + window.scrollY >= document.documentElement.scrollHeight - 4){ for(var j = heads.length - 1; j >= 0; j--){ if(heads[j] && heads[j].offsetParent){ cur = j; break; } } }
    links.forEach(function(a, i){ a.classList.toggle('cur', i === cur); });
  }
  if(links.length){ window.addEventListener('scroll', function(){ if(!ticking){ ticking = true; requestAnimationFrame(follow); } }, {passive: true}); follow(); }
})();

// "Still stuck? Tell us" (troubleshooting): sent to the Hearth OS counter's /v1/logs as kind "docs", so it lands with
// the device logs (a file and an issue in the private logs repository). Answers a code to quote.
(function(){
  var f = document.getElementById('ask'); if(!f) return;
  var API = 'https://hearth-counter.game305-cl.workers.dev';
  f.addEventListener('submit', function(e){
    e.preventDefault();
    var msg = f.querySelector('.msg'), btn = f.querySelector('button'), v = function(n){ return (f.elements[n].value || '').trim(); };
    msg.className = 'msg';
    if(v('what').length < 10){ msg.className = 'msg err'; msg.textContent = 'Say a bit more about what went wrong.'; f.elements.what.focus(); return; }
    var body = 'Hearth logs: a problem sent from hearthos.fun/docs\n\n'
      + 'Page: ' + location.pathname + '\n'
      + 'Edition: ' + (v('edition') || 'not sure') + '\n'
      + 'Version: ' + (v('version') || '?') + '\n'
      + 'Device: ' + (v('device') || '?') + '\n'
      + 'Log code: ' + (v('code') || 'none') + '\n'
      + 'Contact: ' + (v('contact') || 'none') + '\n'
      + 'Browser: ' + navigator.userAgent + '\n\n'
      + '===== What went wrong =====\n' + v('what') + '\n\n'
      + '===== What they tried =====\n' + (v('tried') || '(nothing said)') + '\n';
    var note = (v('edition') ? v('edition') + ': ' : '') + v('what').replace(/\s+/g, ' ').slice(0, 300);
    btn.disabled = true; msg.textContent = 'Sending…';
    fetch(API + '/v1/logs', {method: 'POST', body: body, headers: {'content-type': 'text/plain; charset=utf-8',
        'x-hearth-kind': 'docs', 'x-hearth-version': (v('version') || 'website').replace(/[^\w.+-]/g, '').slice(0, 20) || 'website',
        'x-hearth-note': encodeURIComponent(note)}})
      .then(function(r){ return r.json().then(function(j){ return {ok: r.ok, j: j}; }); })
      .then(function(x){
        if(x.ok && x.j.code){ msg.innerHTML = 'Sent. Your code: <b></b>. Quote it if you follow up.'; msg.querySelector('b').textContent = x.j.code; f.reset(); }
        else { msg.className = 'msg err'; msg.textContent = (x.j && x.j.error === 'busy, try again later') ? 'Lots of reports right now: try again in an hour.' : 'That didn\'t send. Try again in a moment.'; }
      })
      .catch(function(){ msg.className = 'msg err'; msg.textContent = 'That didn\'t send (no connection?). Try again in a moment.'; })
      .then(function(){ btn.disabled = false; });
  });
})();
