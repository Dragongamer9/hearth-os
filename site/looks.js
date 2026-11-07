// Hearth OS's looks on the website: made by tools/site-looks.py (don't edit by hand). The picker sits in any
// element with class "looks"; the choice is kept in this browser only.
(function(){
  var LOOKS = [{"id":"hearth","name":"Hearth","desc":"Warm firelit darks, frosted islands","sw":["#17110e","#33271f","#ff8a3d","#ffd166"]},{"id":"steam","name":"Steam-style","desc":"Blue-slate, flat, crisp corners","sw":["#1b2838","#2a475e","#1a9fff","#66c0f4"]},{"id":"gaming","name":"Gaming","desc":"Neon edges, HUD shapes, performance island","sw":["#0b0f16","#1a2433","#22e4ff","#ff3dc8"]},{"id":"zen","name":"Zen","desc":"One thin bar, quiet colours","sw":["#22211e","#3a3833","#8fb39a","#c9d8b6"]},{"id":"glass","name":"Glass","desc":"Frosted islands over aurora","sw":["#0e1630","#2e3a60","#5cf2c0","#b8f7e4"]},{"id":"synthwave","name":"Synthwave","desc":"Sunset grid, chrome type, scanlines","sw":["#1a0a2e","#36175a","#ff3fa4","#29e3ff"]},{"id":"cabin","name":"Cabin","desc":"Wood, embers, the most Hearth","sw":["#1b120c","#3d2819","#ff9440","#ffc46b"]},{"id":"console","name":"Console","desc":"Big tiles for the TV, 10-foot","sw":["#15100d","#33271f","#ff8a3d","#ffd166"]}], DEF = "steam";
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
      b.addEventListener('click', function(){ set(l.id); sync(); close(true); });
      menu.appendChild(b); return b;
    });
    function sync(){ items.forEach(function(b){ b.setAttribute('aria-checked', b.dataset.id === cur()); }); }
    function open(){ sync(); menu.hidden = false; btn.setAttribute('aria-expanded', 'true');
      var c = items.filter(function(b){ return b.dataset.id === cur(); })[0] || items[0]; c.focus(); }
    function close(focus){ menu.hidden = true; btn.setAttribute('aria-expanded', 'false'); if (focus) btn.focus(); }
    btn.addEventListener('click', function(){ menu.hidden ? open() : close(false); });
    menu.addEventListener('keydown', function(e){
      var i = items.indexOf(document.activeElement);
      if (e.key === 'Escape') { e.preventDefault(); close(true); }
      else if (e.key === 'ArrowDown') { e.preventDefault(); items[(i + 1) % items.length].focus(); }
      else if (e.key === 'ArrowUp') { e.preventDefault(); items[(i - 1 + items.length) % items.length].focus(); }
    });
    document.addEventListener('click', function(e){ if (!menu.hidden && !box.contains(e.target)) close(false); });
    box.appendChild(btn); box.appendChild(menu);
  });
})();
