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
    lines.append(LOOKS_UI_CSS)
    return '\n'.join(lines) + '\n'


LOOKS_UI_CSS = """/* The look picker (a button by light/dark, and its menu). */
.looks{position:relative;display:inline-flex}
.looks>button{display:inline-grid;place-items:center;width:44px;height:44px;border-radius:3px;border:0;background:none;color:var(--text);cursor:pointer}
.looks>button:hover{background:var(--accsoft,rgba(102,192,244,.14))}
.looks>button:focus-visible,.looks-menu button:focus-visible{outline:2px solid var(--focus);outline-offset:2px}
.looks>button svg{width:20px;height:20px}
.looks-menu{position:absolute;right:0;top:calc(100% + 8px);z-index:60;width:min(300px,calc(100vw - 32px));padding:6px;border-radius:10px;
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


def js():
    data = []
    for k, v in looks():
        d = v['dark']
        data.append({'id': k, 'name': v.get('label', k.title()), 'desc': v.get('desc', ''),
                     'sw': [d['bg2'], d['panel3'], d['ember'], d['flame']]})
    return ('// Hearth OS\'s looks on the website: made by tools/site-looks.py (don\'t edit by hand). The picker sits in any\n'
            '// element with class "looks"; the choice is kept in this browser only.\n'
            '(function(){\n'
            '  var LOOKS = ' + json.dumps(data, separators=(',', ':')) + ', DEF = ' + json.dumps(SITE_DEFAULT) + ';\n'
            + LOOKS_JS + '})();\n')


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
"""


def main():
    want = {'site/looks.css': css(), 'site/looks.js': js()}
    if '--check' in sys.argv:
        stale = [p for p, t in want.items() if not os.path.exists(os.path.join(ROOT, p)) or open(os.path.join(ROOT, p)).read() != t]
        if stale:
            print('Out of date (run python3 tools/site-looks.py): ' + ', '.join(stale)); sys.exit(1)
        return
    for p, t in want.items():
        open(os.path.join(ROOT, p), 'w').write(t)


if __name__ == '__main__':
    main()
