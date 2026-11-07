#!/usr/bin/env python3
"""Website looks: hearthos.fun gets Hearth OS's own looks (Settings › Appearance), from the same palettes Mantel uses
(hosts/linux/desktop/theme/palettes.json). Writes site/looks.css and site/looks.js; the front page and the docs load
both. Steam-style is the site's own colours, so picking it just clears the choice. The choice is kept in this browser
only (localStorage "hearth.look"), next to light or dark ("hearth.theme").

    python3 tools/site-looks.py            # write site/looks.css and site/looks.js
    python3 tools/site-looks.py --check    # fail if they're out of date (the site test runs this)
"""
import json, os, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PAL = json.load(open(os.path.join(ROOT, 'hosts/linux/desktop/theme/palettes.json')))
SITE_DEFAULT = 'steam'


def rgb(h):
    h = h.lstrip('#'); return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def mix(a, b, t):
    a, b = rgb(a), rgb(b)
    return '#%02x%02x%02x' % tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def rgba(h, a):
    r, g, b = rgb(h); return f'rgba({r},{g},{b},{a})'


def looks():
    out = [(k, v) for k, v in PAL.items() if not k.startswith('_') and isinstance(v, dict) and v.get('dark')]
    return sorted(out, key=lambda kv: kv[1].get('order', 99))


def vars_for(p, mode):
    """The front page's and the docs' colour variables for one look in one mode (the accent ones apart)."""
    light = mode == 'light'
    text = p['text']
    soft = mix(text, p['muted'], .3)
    base = {
        # front page (site/index.html)
        '--deep': p['bg'], '--page': p['bg2'], '--panel': p['panel'], '--raise': p['panel3'], '--rule': p['line'],
        '--rule2': mix(p['line'], text, .18), '--hi': text, '--text': soft, '--muted': p['muted'],
        # docs (site/docs/docs.css)
        '--bg': p['bg2'], '--bg2': p['bg'], '--panel2': p['panel3'], '--line': p['line'], '--line2': mix(p['line'], text, .18),
        '--soft': soft, '--code': mix(p['bg'], p['panel'], .5),
    }
    # On light pages the look's accent is darkened a little so links stay readable.
    acc = mix(p['ember'], text, .22) if light else p['ember']
    accent = {
        '--acc': acc, '--acc2': acc, '--onacc': p.get('ink', '#ffffff'), '--accsoft': rgba(p['ember'], .12 if light else .14),
        '--focus': acc if light else p['flame'],
        '--glow': f'radial-gradient(60% 70% at 70% 0%,{rgba(p["ember"], .16 if light else .20)},transparent 70%)',
        '--ember': acc, '--ember2': mix(acc, text, .3), '--ink': p.get('ink', '#ffffff'),
    }
    return base, accent


def block(sel, d):
    return sel + '{' + ';'.join(f'{k}:{v}' for k, v in d.items()) + '}'


def css():
    lines = ['/* Hearth OS\'s looks on the website: made by tools/site-looks.py from hosts/linux/desktop/theme/palettes.json.',
             '   Don\'t edit by hand. Steam-style is the site\'s own colours (no data-look). */']
    for k, v in looks():
        if k == SITE_DEFAULT:
            continue
        r = f':root[data-look="{k}"]'
        dark_b, dark_a = vars_for(v['dark'], 'dark')
        light_b, light_a = vars_for(v.get('light') or v['dark'], 'light')
        # Dark (and the site's default when the system doesn't ask for light). The accent stays the look's unless
        # "Try an accent" is picked on the page.
        lines.append(block(r, dict(dark_b, **{'color-scheme': 'dark'})))
        lines.append(block(r + ':not([data-accent])', dark_a))
        lines.append('@media (prefers-color-scheme:light){' + block(r + ':not([data-theme="dark"])', dict(light_b, **{'color-scheme': 'light'}))
                     + block(r + ':not([data-theme="dark"]):not([data-accent])', light_a) + '}')
        lines.append(block(r + '[data-theme="light"]', dict(light_b, **{'color-scheme': 'light'})))
        lines.append(block(r + '[data-theme="light"]:not([data-accent])', light_a))
        # The docs' top bar stays dark in every mode; it takes the look's dark colours.
        lines.append(block(r + ' nav.top', dict({k2: v2 for k2, v2 in dict(dark_b, **dark_a).items() if not k2.startswith('--glow')},
                                                background=dark_b['--bg2'], **{'border-bottom-color': dark_b['--line']})))
    for k, v in looks():
        sel = ':root' if k == SITE_DEFAULT else f':root[data-look="{k}"]'
        stretch = {'gaming': '78%', 'zen': '100%'}.get(k)
        lines.append(block(sel, dict({'--lrad': f"{max(int(v.get('windowCorner', 4)), 3)}px"}, **({'--lstretch': stretch} if stretch else {}))))
    lines.append(LOOKS_UI_CSS)
    lines.append(WALL_CSS)
    return '\n'.join(lines) + '\n'


LOOKS_UI_CSS = """/* The look picker (a button by light/dark, and its menu). */
.looks{position:relative;display:inline-flex}
.looks>button{display:inline-grid;place-items:center;width:44px;height:44px;border-radius:3px;border:0;background:none;color:var(--text);cursor:pointer}
.looks>button:hover{background:var(--accsoft,rgba(102,192,244,.14))}
.looks>button:focus-visible,.looks-menu button:focus-visible{outline:2px solid var(--focus);outline-offset:2px}
.looks>button svg{width:20px;height:20px}
.looks-menu{position:fixed;z-index:1000;width:min(300px,calc(100vw - 32px));padding:6px;border-radius:10px;
  background:var(--panel);border:1px solid var(--rule,var(--line));box-shadow:0 18px 40px rgba(0,0,0,.35)}
.looks-menu[hidden]{display:none}
.looks-menu p{margin:6px 8px 8px;font-size:13px;color:var(--muted)}
.looks-menu button{display:flex;align-items:center;gap:10px;width:100%;padding:8px;border:0;border-radius:7px;background:transparent;color:var(--hi,var(--text));text-align:left;font:inherit;cursor:pointer}
.looks-menu button:hover{background:var(--accsoft,rgba(102,192,244,.14))}
.looks-menu button[aria-checked="true"]{background:var(--accsoft,rgba(102,192,244,.14));box-shadow:inset 0 0 0 1px var(--acc,var(--ember))}
.looks-menu .sw{display:inline-flex;flex:none;border-radius:6px;overflow:hidden;border:1px solid rgba(127,127,127,.35)}
.looks-menu .sw i{display:block;width:12px;height:28px}
.looks-menu b{display:block;font-size:14px;font-weight:600}
.looks-menu small{display:block;font-size:12px;color:var(--muted);line-height:1.3}
"""

WALL_CSS = """/* Each look's wallpaper behind the page, its main bar and its islands (looks.js sets data-bar, data-frame, has-wall). */
.lk-wall{position:fixed;inset:0;width:100vw;height:100vh;z-index:-1;pointer-events:none;background-size:cover;background-position:center}
html.has-wall{background:var(--deep,var(--bg2))}
html.has-wall body{background:transparent}
html.has-wall .hero{background:linear-gradient(90deg,color-mix(in srgb,var(--deep) 72%,transparent),color-mix(in srgb,var(--deep) 30%,transparent) 62%,transparent)}
html.has-wall section{background:color-mix(in srgb,var(--page) 80%,transparent)}
html.has-wall section.deep,html.has-wall footer{background:color-mix(in srgb,var(--deep) 84%,transparent)}
html.has-wall .layout{background:color-mix(in srgb,var(--bg) 84%,transparent);border-radius:var(--lrad);margin-top:24px;margin-bottom:24px;padding-left:28px;padding-right:28px}
html.has-wall body>footer{background:color-mix(in srgb,var(--bg2) 86%,transparent)}
@media (prefers-color-scheme:light){html.has-wall:not([data-theme="dark"]) section{background:color-mix(in srgb,var(--page) 90%,transparent)}
  html.has-wall:not([data-theme="dark"]) .hero{background:linear-gradient(90deg,color-mix(in srgb,var(--deep) 86%,transparent),color-mix(in srgb,var(--deep) 45%,transparent) 62%,transparent)}
  html.has-wall:not([data-theme="dark"]) .layout{background:color-mix(in srgb,var(--bg) 92%,transparent)}}
html.has-wall[data-theme="light"] section{background:color-mix(in srgb,var(--page) 90%,transparent)}
html.has-wall[data-theme="light"] .hero{background:linear-gradient(90deg,color-mix(in srgb,var(--deep) 86%,transparent),color-mix(in srgb,var(--deep) 45%,transparent) 62%,transparent)}
html.has-wall[data-theme="light"] .layout{background:color-mix(in srgb,var(--bg) 92%,transparent)}
html.has-wall .h1,html.has-wall .h2,html.has-wall main h1{font-stretch:var(--lstretch,inherit)}
.lk-fx{position:fixed;inset:0;pointer-events:none;z-index:50}
.lk-scan{background:repeating-linear-gradient(0deg,rgba(0,0,0,.10) 0 1px,transparent 1px 4px)}
.lk-edge{box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--acc,var(--ember)) 50%,transparent),inset 0 0 48px color-mix(in srgb,var(--acc,var(--ember)) 20%,transparent)}
/* The main bar, as the look draws it: a full-width strip (bar, thin), floating islands, or Console's shelf of tiles. */
.lk-isl{display:flex;align-items:center;gap:6px;min-width:0}
.lk-isl.tools{margin-left:auto}
.lk-isl.brandi,.lk-isl.tools{flex:none}
.lk-isl.navi{min-width:0}
/* Every link shows in every look: floating islands and Console's shelf use the whole width, as Mantel's bar does, and get
   more compact (the brand becomes its flame) on narrower screens. */
html[data-bar="floating"] header.top .wrap,html[data-bar="shelf"] header.top .wrap{max-width:none;padding:0 16px}
@media (max-width:1240px){html[data-bar="floating"] header nav a{padding:7px 7px;font-size:14px}
  html[data-bar="floating"] .lk-isl.brandi .brand,html[data-bar="shelf"] .lk-isl.brandi .brand{font-size:0;gap:0}}
@media (max-width:1480px){html[data-bar="shelf"] .lk-hint{display:none!important}}
@media (max-width:560px){.lk-isl.tools .looks,.lk-isl.tools .theme{width:auto}.lk-isl.brandi{flex:0 1 auto;min-width:0;overflow:hidden}.lk-isl.brandi .brand{min-width:0;overflow:hidden;text-overflow:ellipsis}html[data-bar] header .dl{padding:9px 11px}html[data-bar="floating"] header.top .lk-isl{padding:0 4px}}
.lk-clock,.lk-perf{white-space:nowrap;font:600 14px/1 var(--body);color:var(--hi,var(--text));padding:0 10px}
.lk-perf{font:600 12px/1.25 var(--mono);color:var(--acc,var(--ember));display:flex;gap:12px}
.lk-perf b{color:var(--hi,var(--text))}
html[data-bar] header.top,html[data-bar] nav.top{backdrop-filter:blur(18px);-webkit-backdrop-filter:blur(18px)}
html[data-bar="bar"] header.top{background:color-mix(in srgb,var(--panel) 86%,transparent)}
html[data-bar="bar"][data-thin] header.top .wrap{height:50px}
html[data-bar="bar"][data-thin] header nav a{padding:6px 10px;font-size:14px}
html[data-bar="floating"] header.top,html[data-bar="shelf"] header.top{background:transparent;border-bottom:0;padding-top:10px;backdrop-filter:none;-webkit-backdrop-filter:none}
html[data-bar="floating"] header.top .wrap{height:58px;gap:10px}
html[data-bar="floating"] header.top .lk-isl{height:100%;padding:0 8px}
html[data-bar="floating"] header.top .brand{margin-right:6px}
html[data-bar] header nav a{white-space:nowrap}
html[data-bar="floating"] header nav a{padding:8px 9px;font-size:14.5px}
html[data-bar="bar"] header nav a{padding:8px 9px}
@media (max-width:1320px){.lk-clock{display:none!important}}
html[data-bar="floating"] nav.top,html[data-bar="shelf"] nav.top{background:transparent;border-bottom:0;padding-top:10px;backdrop-filter:none;-webkit-backdrop-filter:none}
html[data-bar="floating"] nav.top .wrap,html[data-bar="shelf"] nav.top .wrap{gap:10px}
html[data-bar="floating"] nav.top .lk-isl{height:52px;padding:0 10px}
html[data-bar="shelf"] header.top .wrap{height:84px;gap:12px}
html[data-bar="shelf"] header nav{gap:10px}
html[data-bar="shelf"] header nav{gap:8px}
html[data-bar="shelf"] header nav a{font-size:14px;font-weight:600;padding:11px 12px;border-radius:12px;white-space:nowrap;
  background:color-mix(in srgb,var(--panel) 82%,transparent);border:1px solid var(--rule);backdrop-filter:blur(14px)}
html[data-bar="shelf"] header nav a:hover,html[data-bar="shelf"] header nav a:focus-visible{outline:3px solid var(--acc);outline-offset:2px;background:var(--raise)}
html[data-bar="shelf"] nav.top .links a{padding:12px 16px;border-radius:12px;background:color-mix(in srgb,var(--panel) 82%,transparent)}
.lk-hint{display:none;align-items:center;gap:8px;font-size:13px;color:var(--muted);white-space:nowrap}
.lk-hint i{display:inline-grid;place-items:center;width:22px;height:22px;border-radius:50%;border:1.5px solid var(--muted);font-style:normal;font-size:11px;font-weight:700}
html[data-bar="shelf"] .lk-hint{display:inline-flex}
@media (max-width:1020px){html[data-bar] .lk-isl.navi,.lk-perf,.lk-clock,.lk-hint{display:none!important}}
/* Islands and panels in the look's frame (Frame.qml): plain, frost (Glass), neon (Synthwave), chamfer (Gaming), wood (Cabin). */
html[data-bar="floating"] .lk-isl.fr{border-radius:var(--lrad)}
html.has-wall :is(.flow,.shelf,.safe>div,.box,.stat,.cmd,.archived,.mock,.lk-isl.fr,main .card,main .tile){border-radius:var(--lrad)}
html[data-frame="plain"] :is(.flow,.shelf,.box,.stat,.cmd,.lk-isl.fr,main .card){background:color-mix(in srgb,var(--panel) 90%,transparent);box-shadow:0 0 0 1px var(--rule,var(--line));backdrop-filter:blur(16px);-webkit-backdrop-filter:blur(16px)}
html[data-frame="frost"] :is(.flow,.shelf,.box,.stat,.cmd,.lk-isl.fr,main .card){background:color-mix(in srgb,var(--panel) 62%,transparent);backdrop-filter:blur(26px) saturate(1.35);-webkit-backdrop-filter:blur(26px) saturate(1.35);
  box-shadow:inset 0 1px 0 rgba(255,255,255,.35),0 0 0 1px rgba(255,255,255,.16),0 10px 30px rgba(0,0,0,.18)}
html[data-frame="neon"] :is(.flow,.shelf,.box,.stat,.cmd,.lk-isl.fr,main .card){background:color-mix(in srgb,var(--panel) 88%,transparent);backdrop-filter:blur(10px);-webkit-backdrop-filter:blur(10px);
  box-shadow:0 0 0 1.5px color-mix(in srgb,var(--acc,var(--ember)) 80%,transparent),0 0 0 4px color-mix(in srgb,var(--acc,var(--ember)) 18%,transparent),0 0 0 6px color-mix(in srgb,var(--acc,var(--ember)) 10%,transparent),0 0 22px color-mix(in srgb,var(--acc,var(--ember)) 28%,transparent)}
html[data-frame="chamfer"] :is(.flow,.shelf,.box,.stat,.cmd,.lk-isl.fr,main .card){border-radius:0!important;background:color-mix(in srgb,var(--panel) 92%,transparent);
  clip-path:polygon(0 0,calc(100% - 16px) 0,100% 16px,100% 100%,16px 100%,0 calc(100% - 16px));box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--acc,var(--ember)) 55%,transparent)}
html[data-frame="wood"] .lk-isl.fr{box-shadow:0 0 0 1px var(--woodedge);background:repeating-linear-gradient(0deg,transparent 0 6px,var(--grain) 6px 7px),linear-gradient(180deg,var(--wood1),var(--wood2) 55%,var(--wood3))}
html[data-frame="wood"] :is(.flow,.shelf,.box,.stat,.cmd,main .card){box-shadow:0 0 0 1px var(--woodedge);
  background:linear-gradient(color-mix(in srgb,var(--panel) 66%,transparent),color-mix(in srgb,var(--panel) 66%,transparent)),repeating-linear-gradient(0deg,transparent 0 6px,var(--grain) 6px 7px),linear-gradient(180deg,var(--wood1),var(--wood2) 55%,var(--wood3))}
html[data-frame="wood"]{--wood1:#4a2f1b;--wood2:#3a2414;--wood3:#2b1a0e;--woodedge:#1a0f08;--grain:rgba(0,0,0,.24)}
@media (prefers-color-scheme:light){html[data-frame="wood"]:not([data-theme="dark"]){--wood1:#d8b48a;--wood2:#c99f72;--wood3:#b98b5d;--woodedge:#8a6440;--grain:rgba(90,51,20,.18)}}
html[data-frame="wood"][data-theme="light"]{--wood1:#d8b48a;--wood2:#c99f72;--wood3:#b98b5d;--woodedge:#8a6440;--grain:rgba(90,51,20,.18)}
html:not([data-bar="floating"]) .lk-isl.fr{background:none!important;box-shadow:none!important;backdrop-filter:none!important;-webkit-backdrop-filter:none!important;clip-path:none!important}
"""


def js():
    data = []
    for k, v in looks():
        d = v['dark']
        wall = v.get('wallpaper', '')
        wall = wall[5:] if wall.startswith('anim:') else (os.path.splitext(os.path.basename(wall))[0].replace('hearth-', '') if wall else 'fireside')
        data.append({'id': k, 'name': v.get('label', k.title()), 'desc': v.get('desc', ''),
                     'sw': [d['bg2'], d['panel3'], d['ember'], d['flame']],
                     # How Mantel draws this look: its wallpaper, its main bar and its islands' frame.
                     'wall': wall, 'bar': v.get('islandStyle', 'floating'), 'thin': bool(v.get('thin')), 'frame': v.get('frame', 'plain'),
                     'clock': bool(v.get('deskClock')), 'perf': bool(v.get('perfIsland')), 'edge': bool(v.get('edgeGlow')),
                     'scan': bool(v.get('scanlines')), 'ember': d['ember'], 'flame': d['flame'],
                     'lember': (v.get('light') or d)['ember'], 'lflame': (v.get('light') or d)['flame']})
    return ('// Hearth OS\'s looks on the website: made by tools/site-looks.py (don\'t edit by hand). The picker sits in any\n'
            '// element with class "looks"; the choice is kept in this browser only.\n'
            '(function(){\n'
            '  var LOOKS = ' + json.dumps(data, separators=(',', ':')) + ', DEF = ' + json.dumps(SITE_DEFAULT) + ';\n'
            + LOOKS_JS + WALL_JS + '})();\n')


LOOKS_JS = r"""  var root = document.documentElement;
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
      b.addEventListener('click', function(){ set(l.id); sync(); close(false); });
      menu.appendChild(b); return b;
    });
    function sync(){ items.forEach(function(b){ b.setAttribute('aria-checked', b.dataset.id === cur()); }); }
    // The menu lives at the end of the page, not inside the bar: a look's island can clip what's inside it (Gaming's cut
    // corners) or blur it (frosted islands), which hid the menu and made the looks impossible to switch.
    function place(){ var r = btn.getBoundingClientRect(); menu.style.top = Math.round(r.bottom + 8) + 'px'; menu.style.right = Math.max(8, Math.round(innerWidth - r.right)) + 'px'; }
    function open(){ sync(); place(); menu.hidden = false; btn.setAttribute('aria-expanded', 'true');
      var c = items.filter(function(b){ return b.dataset.id === cur(); })[0] || items[0]; c.focus(); }
    function close(focus){ menu.hidden = true; btn.setAttribute('aria-expanded', 'false'); if (focus) btn.focus(); }
    btn.addEventListener('click', function(){ menu.hidden ? open() : close(false); });
    menu.addEventListener('keydown', function(e){
      var i = items.indexOf(document.activeElement);
      if (e.key === 'Escape') { e.preventDefault(); close(true); }
      else if (e.key === 'ArrowDown') { e.preventDefault(); items[(i + 1) % items.length].focus(); }
      else if (e.key === 'ArrowUp') { e.preventDefault(); items[(i - 1 + items.length) % items.length].focus(); }
    });
    document.addEventListener('click', function(e){ if (!menu.hidden && !box.contains(e.target) && !menu.contains(e.target)) close(false); });
    addEventListener('resize', function(){ if (!menu.hidden) place(); });
    addEventListener('scroll', function(){ if (!menu.hidden) place(); }, { passive: true });
    box.appendChild(btn); document.body.appendChild(menu);
  });
"""


# The look's wallpaper (Mantel's Wallpaper.qml and Scene.qml, drawn the same way), its main bar and its islands.
WALL_JS = r"""
  var BASE = (function(){ var s = document.currentScript && document.currentScript.src; try { return new URL('img/looks/', s || location.href).href; } catch (e) { return 'img/looks/'; } })();
  var still = matchMedia('(prefers-reduced-motion: reduce)');
  var lightQ = matchMedia('(prefers-color-scheme: light)');
  function byId(id){ return LOOKS.filter(function(l){ return l.id === id; })[0] || LOOKS.filter(function(l){ return l.id === DEF; })[0]; }
  function isDark(){ var t = root.dataset.theme; return t ? t === 'dark' : !lightQ.matches; }
  var imgs = {};
  function img(n){ if (!imgs[n]) { var i = new Image(); i.src = BASE + n; imgs[n] = i; i.onload = function(){ paint(true); }; } return imgs[n]; }
  function ready(i){ return i.complete && i.naturalWidth > 0; }
  // The page's own pieces, put in islands so a look with floating islands can draw them apart.
  function wrap(bar, items, cls){
    items = items.filter(Boolean); if (!items.length) return null;
    var d = el('div', 'lk-isl fr ' + cls); items[0].parentNode.insertBefore(d, items[0]); items.forEach(function(n){ d.appendChild(n); }); return d;
  }
  var head = document.querySelector('header.top .wrap'), dnav = document.querySelector('nav.top .wrap'), clockEl = null, perfEl = null, hint = null;
  if (head) {
    wrap(head, [head.querySelector('.brand')], 'brandi');
    wrap(head, [head.querySelector('nav')], 'navi');
    hint = el('span', 'lk-hint'); hint.innerHTML = '<i>A</i>open <i>B</i>back';
    var tools = [head.querySelector('.looks'), head.querySelector('.theme'), head.querySelector('.dl')].filter(Boolean);
    if (tools.length) tools[0].parentNode.insertBefore(hint, tools[0]);
    var t = wrap(head, [hint].concat(tools), 'tools');
  } else if (dnav) {
    wrap(dnav, [dnav.querySelector('.brand')], 'brandi');
    wrap(dnav, [dnav.querySelector('.links'), dnav.querySelector('.looks')], 'tools');
  }
  var cv = el('canvas', 'lk-wall'), pic = el('div', 'lk-wall'), scan = el('div', 'lk-fx lk-scan'), edge = el('div', 'lk-fx lk-edge');
  cv.setAttribute('aria-hidden', 'true'); pic.setAttribute('aria-hidden', 'true');
  document.body.insertBefore(pic, document.body.firstChild); document.body.insertBefore(cv, document.body.firstChild);
  document.body.appendChild(scan); document.body.appendChild(edge);
  var c = cv.getContext('2d'), W = 0, H = 0, L = null, dark = true, t0 = performance.now(), raf = 0, sparks = [];
  function lin(y0, y1, stops){ var g = c.createLinearGradient(0, y0, 0, y1); stops.forEach(function(s){ g.addColorStop(s[0], s[1]); }); return g; }
  function blob(n, x, y, size, o){ var i = img(n); if (!ready(i)) return; c.globalAlpha = o; c.drawImage(i, x - size / 2, y - size / 2, size, size); c.globalAlpha = 1; }
  var DRAW = {
    fireside: function(t){   // the Hearth look: warm darkness lit from below by a slowly breathing fire, embers drifting up
      c.fillStyle = lin(0, H, dark ? [[0, '#0b0807'], [.6, '#17110e'], [1, '#1f140e']] : [[0, '#f6efe8'], [.6, '#efe5db'], [1, '#f1dcc8']]); c.fillRect(0, 0, W, H);
      var g = img('ember-glow.png'); if (!ready(g)) return;
      var br = (Math.sin(t / 1300) + 1) / 2, fl = (Math.sin(t / 310) + Math.sin(t / 190)) / 4 + .5, dr = Math.sin(t / 1800) * W * .02;
      c.globalAlpha = (dark ? .5 : .3) + br * .1; c.drawImage(g, (W - W * 1.5) / 2, H - H * 1.1 * .45, W * 1.5, H * 1.1);
      c.globalAlpha = (dark ? .34 : .2) + fl * .1; c.drawImage(g, (W - W * .7) / 2 + dr, H - H * .6 * .42, W * .7, H * .6); c.globalAlpha = 1;
      if (moving()) { if (Math.random() < .05) sparks.push({ x: W * (.25 + Math.random() * .5), y: H + 4, v: H * (.0006 + Math.random() * .0006), w: Math.random() * 6.28, c: Math.random() > .5 }); }
      else if (!sparks.length) for (var i = 0; i < 14; i++) sparks.push({ x: W * .25 + ((i * .618) % 1) * W * .5, y: H * (.55 + ((i * .754) % 1) * .42), v: 0, w: i, c: i % 2 });
      sparks = sparks.filter(function(s){ s.y -= s.v * 16; s.x += Math.sin(t / 900 + s.w) * .3; return s.y > H * .2; });
      sparks.forEach(function(s){ c.globalAlpha = .55 * Math.min(1, (s.y - H * .2) / (H * .3)); c.fillStyle = s.c ? (dark ? L.ember : L.lember) : (dark ? L.flame : L.lflame); c.beginPath(); c.arc(s.x, s.y, 2.2, 0, 6.3); c.fill(); });
      c.globalAlpha = 1;
    },
    tide: function(t){   // Steam-style: soft blue light drifting slowly over blue-slate
      c.fillStyle = lin(0, H, dark ? [[0, '#0e131b'], [1, '#1b2838']] : [[0, '#e6ebf0'], [1, '#dbe2e9']]); c.fillRect(0, 0, W, H);
      [['blob-blue.png', .22, .64, .62, .10, .06, 31, .55], ['blob-slate.png', .52, .32, .70, -.12, .08, 37, .7], ['blob-sky.png', .76, .62, .46, -.08, -.10, 41, .35], ['blob-blue.png', .84, .20, .40, -.10, .05, 34, .45]].forEach(function(b){
        var k = Math.sin(t / (b[6] * 1000 / 6.283)); blob(b[0], W * (b[1] + b[4] * k), H * (b[2] + b[5] * k), H * b[3] * 1.7, b[7] * (dark ? 1 : .45)); });
      c.fillStyle = lin(H * .55, H, [[0, 'rgba(9,12,18,0)'], [1, dark ? 'rgba(9,12,18,.5)' : 'rgba(9,12,18,.04)']]); c.fillRect(0, 0, W, H);
    },
    neon: function(){   // Gaming: near-black, cyan lines, a magenta HUD frame
      c.fillStyle = lin(0, H, [[0, dark ? '#06080c' : '#e9f0f5'], [1, dark ? '#0b0f18' : '#dbe5ee']]); c.fillRect(0, 0, W, H);
      c.strokeStyle = dark ? 'rgba(34,228,255,0.08)' : 'rgba(0,152,184,0.10)'; c.lineWidth = Math.max(1, W / 1920);
      for (var x = -H; x < W; x += W / 12) { c.beginPath(); c.moveTo(x, H); c.lineTo(x + H * .48, H * .52); c.stroke(); }
      var g = c.createRadialGradient(W * .74, H * .34, 0, W * .74, H * .34, H * .45); g.addColorStop(0, dark ? 'rgba(255,61,200,0.20)' : 'rgba(192,24,154,0.12)'); g.addColorStop(1, 'rgba(0,0,0,0)'); c.fillStyle = g; c.fillRect(0, 0, W, H);
      var fx = W * .615, fy = H * .185, fw = W * .24, fh = H * .31, cut = H * .08, i = H * .04;
      c.strokeStyle = dark ? 'rgba(255,61,200,0.75)' : 'rgba(192,24,154,0.55)'; c.lineWidth = Math.max(2, W / 800);
      c.beginPath(); c.moveTo(fx, fy); c.lineTo(fx + fw - cut, fy); c.lineTo(fx + fw, fy + cut); c.lineTo(fx + fw, fy + fh); c.lineTo(fx + cut, fy + fh); c.lineTo(fx, fy + fh - cut); c.closePath(); c.stroke();
      c.strokeStyle = dark ? 'rgba(34,228,255,0.35)' : 'rgba(0,152,184,0.35)'; c.lineWidth = Math.max(1, W / 1600);
      c.beginPath(); c.moveTo(fx + i, fy + i); c.lineTo(fx + fw - cut - i * .5, fy + i); c.lineTo(fx + fw - i, fy + cut + i * .5); c.lineTo(fx + fw - i, fy + fh - i); c.lineTo(fx + cut + i * .5, fy + fh - i); c.lineTo(fx + i, fy + fh - cut - i * .5); c.closePath(); c.stroke();
      c.fillStyle = dark ? '#2a2550' : '#c9d6e2'; c.font = 'bold ' + Math.round(H * .085) + 'px "DejaVu Sans Condensed","Arial Narrow",sans-serif'; c.fillText('HEARTH', W * .655, H * .37);
    },
    zen: function(){   // Zen: warm grey, one brush circle
      c.fillStyle = lin(0, H, [[0, dark ? '#1d1c1a' : '#f1eee8'], [1, dark ? '#171614' : '#e6e2da']]); c.fillRect(0, 0, W, H);
      var cx = W > 900 ? W * .72 : W / 2, cy = H * .47, r = H * .2; c.lineCap = 'round';
      for (var s = 0; s < 120; s++) { var a0 = -.9 + (s / 120) * 5.6, a1 = -.9 + ((s + 1) / 120) * 5.6, k = s / 120;
        c.strokeStyle = dark ? 'rgba(201,216,182,' + (.5 - k * .3) + ')' : 'rgba(60,58,52,' + (.55 - k * .35) + ')'; c.lineWidth = r * (.16 - k * .12); c.beginPath(); c.arc(cx, cy, r, a0, a1); c.stroke(); }
    },
    glass: function(){   // Glass: deep night with a soft aurora
      c.fillStyle = lin(0, H, [[0, dark ? '#081022' : '#dfe8f7'], [1, dark ? '#120d2a' : '#efe6f6']]); c.fillRect(0, 0, W, H);
      [['blob-sky.png', .22, .32, .9, .75], ['blob-plum.png', .68, .42, 1, .8], ['blob-blue.png', .45, .78, .8, .55], ['blob-sky.png', .86, .12, .5, .4]].forEach(function(b){ blob(b[0], W * b[1], H * b[2], H * b[3] * 1.3, b[4] * (dark ? 1 : .6)); });
      c.save(); c.globalAlpha = dark ? .35 : .25; c.translate(W / 2, H * .38); c.rotate(-8 * Math.PI / 180);
      var g = c.createLinearGradient(0, -H * .08, 0, H * .08); g.addColorStop(0, 'rgba(92,242,192,0)'); g.addColorStop(.5, '#5cf2c0'); g.addColorStop(1, 'rgba(92,242,192,0)');
      c.fillStyle = g; c.fillRect(-W * .6, -H * .08, W * 1.2, H * .16); c.restore();
    },
    synth: function(){   // Synthwave: striped sun over a magenta grid, scanlines
      var hz = H * .6; c.fillStyle = lin(0, hz, [[0, dark ? '#12062a' : '#ffe3f3'], [.65, dark ? '#5a1460' : '#ffb3d9'], [1, dark ? '#ff5a8a' : '#ff8fb8']]); c.fillRect(0, 0, W, hz);
      c.fillStyle = dark ? 'rgba(255,240,255,0.7)' : 'rgba(255,255,255,0.8)';
      for (var i = 0; i < 70; i++) c.fillRect(((i * .6180339) % 1) * W, ((i * .7548776) % 1) * hz * .6, 2, 2);
      var cx = W > 900 ? W * .72 : W / 2, cy = hz - H * .02, r = H * .2, g = c.createLinearGradient(0, cy - r, 0, cy + r); g.addColorStop(0, '#ffe66d'); g.addColorStop(1, '#ff3fa4');
      c.save(); c.beginPath(); c.arc(cx, cy, r, Math.PI, 0); c.closePath(); c.clip(); c.fillStyle = g; c.fillRect(cx - r, cy - r, r * 2, r * 2);
      c.fillStyle = dark ? '#3a0f4a' : '#ffb3d9'; for (var k = 0; k < 6; k++) c.fillRect(cx - r, cy - r * .55 + k * r * .11, r * 2, 2 + k * 1.6); c.restore();
      c.fillStyle = lin(hz, H, [[0, dark ? '#1a0630' : '#f7d6ef'], [1, dark ? '#07020f' : '#f1c5e6']]); c.fillRect(0, hz, W, H - hz);
      c.strokeStyle = dark ? 'rgba(255,63,164,0.8)' : 'rgba(216,27,132,0.55)'; c.lineWidth = Math.max(1, W / 1400);
      for (var a = -14; a <= 14; a++) { c.beginPath(); c.moveTo(cx + a * W * .01, hz); c.lineTo(cx + a * W * .12, H); c.stroke(); }
      for (var j = 1; j < 14; j++) { var y = hz + (H - hz) * Math.pow(j / 13, 2.2); c.beginPath(); c.moveTo(0, y); c.lineTo(W, y); c.stroke(); }
      g = c.createRadialGradient(W / 2, H / 2, H * .3, W / 2, H / 2, W * .7); g.addColorStop(0, 'rgba(0,0,0,0)'); g.addColorStop(1, 'rgba(0,0,0,0.4)'); c.fillStyle = g; c.fillRect(0, 0, W, H);
    }
  };
  var MOVING = { fireside: 1, tide: 1 };
  function moving(){ return !still.matches && !document.hidden && L && MOVING[L.wall]; }
  function paint(force){
    if (!L) return;
    if (!DRAW[L.wall]) { cv.hidden = true; pic.hidden = false;
      pic.style.backgroundImage = (dark ? '' : 'linear-gradient(rgba(255,250,244,.74),rgba(255,250,244,.74)),') + 'url(' + BASE + L.wall + '.webp)'; return; }
    cv.hidden = false; pic.hidden = true;
    var w = innerWidth, h = innerHeight;
    if (force || w !== W || h !== H) { W = w; H = h; cv.width = W; cv.height = H; }
    DRAW[L.wall](performance.now() - t0);
  }
  var last = 0, frames = 0, fpsT = 0;
  function loop(now){
    raf = 0;
    frames++; if (now - fpsT > 1000) { if (perfEl && L && L.perf) perfEl.innerHTML = '<span>FPS <b>' + Math.round(frames * 1000 / (now - fpsT)) + '</b></span><span>Page <b>' + (document.documentElement.scrollHeight / 1000).toFixed(1) + 'k px</b></span>'; frames = 0; fpsT = now; }
    if (now - last > 33) { last = now; paint(false); }
    if (moving() || (perfEl && L && L.perf && !document.hidden)) raf = requestAnimationFrame(loop);
  }
  function tick(){ if (clockEl && L && L.clock) clockEl.textContent = new Date().toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }); }
  function apply(){
    L = byId(cur()); dark = isDark(); sparks = [];
    root.classList.add('has-wall'); root.dataset.bar = L.bar; root.dataset.frame = L.frame;
    if (L.thin) root.dataset.thin = ''; else delete root.dataset.thin;
    scan.hidden = !L.scan; edge.hidden = !L.edge;
    if (perfEl) perfEl.hidden = !L.perf || L.bar !== 'floating'; if (clockEl) clockEl.hidden = !L.clock || L.bar !== 'floating';
    tick(); paint(true);
    if (!raf && (moving() || (perfEl && L.perf))) raf = requestAnimationFrame(loop);
  }
  apply();
  setInterval(tick, 15000);
  addEventListener('resize', function(){ paint(true); });
  document.addEventListener('visibilitychange', function(){ if (!document.hidden && !raf) raf = requestAnimationFrame(loop); });
  [still, lightQ].forEach(function(q){ (q.addEventListener ? q.addEventListener('change', apply) : q.addListener(apply)); });
  new MutationObserver(apply).observe(root, { attributes: true, attributeFilter: ['data-look', 'data-theme'] });
"""


def versioned(want):
    """site/index.html and the docs template ask for looks.css / looks.js?v=<hash of both>, so a browser never keeps an old copy."""
    import hashlib, re
    v = hashlib.sha1((want['site/looks.css'] + want['site/looks.js']).encode()).hexdigest()[:10]
    for p in ('site/index.html', 'tools/build-docs.py'):
        t = open(os.path.join(ROOT, p)).read()
        want[p] = re.sub(r'(looks\.(?:css|js))(\?v=[0-9a-f]+)?"', lambda m: m.group(1) + '?v=' + v + '"', t)
    return want


def main():
    want = versioned({'site/looks.css': css(), 'site/looks.js': js()})
    if '--check' in sys.argv:
        stale = [p for p, t in want.items() if not os.path.exists(os.path.join(ROOT, p)) or open(os.path.join(ROOT, p)).read() != t]
        if stale:
            print('Out of date (run python3 tools/site-looks.py): ' + ', '.join(stale)); sys.exit(1)
        return
    for p, t in want.items():
        open(os.path.join(ROOT, p), 'w').write(t)


if __name__ == '__main__':
    main()
