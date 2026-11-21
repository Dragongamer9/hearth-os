#!/usr/bin/python3
"""Writes hosts/hearth-ui/apps/settings/src/pages_list.rs from Mantel's page registry
(hosts/linux/desktop/shell/settings-pages.js, plus the KDE pages in kde-settings.json): every page's id, category, label, line, search words, parent, and
whether it's hidden or a landing. Run it again when settings-pages.js changes (the Rust Settings app's
test checks they agree)."""
import json, os, re, subprocess, sys
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
src = os.path.join(ROOT, 'hosts/linux/desktop/shell/settings-pages.js')
out = os.path.join(ROOT, 'hosts/hearth-ui/apps/settings/src/pages_list.rs')
js = open(src, encoding='utf-8').read().replace('.pragma library', '')
node = 'var module={};' + js + ';console.log(JSON.stringify({c:CATEGORIES,p:PAGES}))'
try:
    data = json.loads(subprocess.check_output(['node', '-e', node]))
except (OSError, subprocess.CalledProcessError):
    sys.exit('node is needed to read settings-pages.js')
def lit(s):
    return json.dumps(s or '', ensure_ascii=False)
lines = ['// Written by tools/settings-registry-rs.py from hosts/linux/desktop/shell/settings-pages.js: do not edit by hand.',
         'use crate::registry::Page;', '', '#[rustfmt::skip]', 'pub const CATEGORIES: &[(&str, &str, &str)] = &[']
for c in data['c']:
    lines.append('    (%s, %s, %s),' % (lit(c['id']), lit(c['label']), lit(c['landing'])))
lines += ['];', '', '#[rustfmt::skip]', 'pub const PAGES: &[Page] = &[']
# The Rust app's own pages for KDE's System Settings modules (cats/kde.rs; not Mantel's): after Mantel's.
kde = os.path.join(ROOT, 'hosts/linux/desktop/shell/kde-settings.json')
if os.path.exists(kde):
    data['p'] = data['p'] + [dict(p, hidden=p.get('hidden', False)) for p in json.load(open(kde, encoding='utf-8'))['pages']]
seen = set()
for p in data['p']:
    if p['id'] in seen:
        continue
    seen.add(p['id'])
    lines.append('    Page { id: %s, cat: %s, label: %s, desc: %s, words: %s, parent: %s, hidden: %s, landing: %s, live: %s },' % (
        lit(p['id']), lit(p['cat']), lit(p['label']), lit(p.get('desc')), lit(p.get('words')), lit(p.get('parent')),
        'true' if p.get('hidden') else 'false', 'true' if p.get('landing') else 'false', 'true' if p.get('live') else 'false'))
lines += ['];', '']
open(out, 'w', encoding='utf-8').write('\n'.join(lines))
print('%d categories, %d pages -> %s' % (len(data['c']), len(seen), os.path.relpath(out, ROOT)))
