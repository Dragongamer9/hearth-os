#!/bin/sh
# Puts CodeMirror 6 (the editor component), its language packages and the JetBrains Mono font into ui/vendor.
# They're downloaded from the npm registry as released (no npm, no bundler, no build step), each one checked
# against the registry's sha512, and the import lines between the packages are pointed at the files next to them,
# so the page loads them as plain ES modules. The versions are the newest that fit the ranges below (and what
# the packages ask of each other); ui/vendor/VERSIONS lists exactly what was taken. The Hearth Text workflow runs
# this with "vendor" ticked (or on a push when ui/vendor is missing) and commits the result: nothing downloads
# anything when the app is built or run.
set -eu
cd "$(dirname "$0")"
exec python3 - "$@" <<'PY'
import base64, hashlib, io, json, os, re, shutil, sys, tarfile, urllib.request

OUT = 'ui/vendor'
REGISTRY = 'https://registry.npmjs.org/'
ROOTS = {
    '@codemirror/state': '^6.4.0', '@codemirror/view': '^6.26.0', '@codemirror/commands': '^6.3.0',
    '@codemirror/language': '^6.10.0', '@codemirror/search': '^6.5.0', '@codemirror/autocomplete': '^6.12.0',
    '@codemirror/lang-javascript': '^6.2.0', '@codemirror/lang-python': '^6.1.0', '@codemirror/lang-json': '^6.0.0',
    '@codemirror/lang-yaml': '^6.0.0', '@codemirror/lang-xml': '^6.0.0', '@codemirror/lang-html': '^6.4.0',
    '@codemirror/lang-css': '^6.2.0', '@codemirror/lang-markdown': '^6.2.0', '@codemirror/lang-cpp': '^6.0.0',
    '@codemirror/lang-rust': '^6.0.0', '@codemirror/legacy-modes': '^6.4.0', '@lezer/highlight': '^1.2.0',
}
# Old-style modes from @codemirror/legacy-modes (shell, TOML, INI/.desktop/.conf, C#, diffs, nginx, Dockerfile...).
LEGACY = ['simple-mode', 'shell', 'toml', 'properties', 'clike', 'diff', 'nginx', 'dockerfile', 'powershell', 'lua', 'go', 'sql', 'cmake', 'perl', 'ruby']
FONT = ('@fontsource/jetbrains-mono', '5.1.1', ['files/jetbrains-mono-latin-400-normal.woff2', 'files/jetbrains-mono-latin-ext-400-normal.woff2', 'LICENSE'])


def get(url):
    with urllib.request.urlopen(url, timeout=60) as r:
        return r.read()


def packument(name, cache={}):
    if name not in cache:
        cache[name] = json.loads(get(REGISTRY + name.replace('/', '%2f')))
    return cache[name]


def ver(v):
    m = re.match(r'^(\d+)\.(\d+)\.(\d+)(-.+)?$', v)
    return (int(m[1]), int(m[2]), int(m[3]), m[4] or '') if m else None


def satisfies(v, rng):
    pv = ver(v)
    if not pv or pv[3]:
        return False                      # no pre-releases
    for alt in rng.split('||'):
        ok = True
        for part in alt.split():
            part = part.strip()
            if part in ('*', 'x', 'latest', ''):
                continue
            m = re.match(r'^(\^|~|>=|<=|>|<|=)?v?(\d+)(?:\.(\d+|x|\*))?(?:\.(\d+|x|\*))?', part)
            if not m:
                ok = False; break
            op = m[1] or '='
            nums = [int(x) if x and x.isdigit() else None for x in (m[2], m[3], m[4])]
            base = tuple(n or 0 for n in nums)
            cur = pv[:3]
            if op == '^':
                if base[0] > 0: ok &= cur >= base and cur[0] == base[0]
                elif base[1] > 0: ok &= cur >= base and cur[:2] == base[:2]
                else: ok &= cur == base
            elif op == '~': ok &= cur >= base and cur[:2] == base[:2]
            elif op == '>=': ok &= cur >= base
            elif op == '>': ok &= cur > base
            elif op == '<=': ok &= cur <= base
            elif op == '<': ok &= cur < base
            else:
                known = [n for n in nums if n is not None]
                ok &= list(cur[:len(known)]) == known
        if ok:
            return True
    return False


# Every package and every range asked of it, until nothing new turns up; then the newest version fitting all.
ranges = {k: {v} for k, v in ROOTS.items()}
chosen = {}
changed = True
while changed:
    changed = False
    for name in sorted(ranges):
        versions = packument(name)['versions']
        fit = [v for v in versions if all(satisfies(v, r) for r in ranges[name])]
        if not fit:
            sys.exit('No version of %s fits %s' % (name, ' and '.join(sorted(ranges[name]))))
        best = max(fit, key=ver)
        if chosen.get(name) != best:
            chosen[name] = best
            changed = True
        for dep, r in (versions[best].get('dependencies') or {}).items():
            if r not in ranges.setdefault(dep, set()):
                ranges[dep].add(r)
                changed = True


def flat(name):
    return name.lstrip('@').replace('/', '-') + '.js'


def tarball(name, version):
    meta = packument(name)['versions'][version]
    data = get(meta['dist']['tarball'])
    sri = meta['dist']['integrity']
    got = 'sha512-' + base64.b64encode(hashlib.sha512(data).digest()).decode()
    if got != sri:
        sys.exit("%s@%s: checksum doesn't match the registry's" % (name, version))
    return tarfile.open(fileobj=io.BytesIO(data)), sri, meta


def entry(meta):
    exp = meta.get('exports')
    if isinstance(exp, dict):
        dot = exp.get('.', exp)
        if isinstance(dot, dict):
            for k in ('import', 'module', 'default'):
                v = dot.get(k)
                if isinstance(v, dict):
                    v = v.get('default')
                if isinstance(v, str):
                    return v
        elif isinstance(dot, str):
            return dot
    return meta.get('module') or meta.get('main') or 'index.js'


IMPORT = re.compile(r'''((?:\bfrom|\bimport)\s*\(?\s*)(['"])([@\w.][\w@/.\-]*)\2''')


def relink(src, up):
    def sub(m):
        spec = m[3]
        if spec.startswith('.') or spec.startswith('/'):
            return m[0]
        if spec not in chosen:
            print('note: left alone (not a vendored package): %s' % spec)
            return m[0]
        return m[1] + m[2] + up + flat(spec) + m[2]
    return IMPORT.sub(sub, src)


def member(tf, path):
    path = path.lstrip('./') if path.startswith('./') else path
    f = tf.extractfile('package/' + path)
    if f is None:
        sys.exit('missing ' + path)
    return f.read()


if os.path.isdir(OUT):
    shutil.rmtree(OUT)
os.makedirs(OUT + '/legacy-modes')
lines = []
for name in sorted(chosen):
    v = chosen[name]
    tf, sri, meta = tarball(name, v)
    lines.append('%s@%s %s' % (name, v, sri))
    if name == '@codemirror/legacy-modes':
        for mode in LEGACY:
            src = member(tf, 'mode/%s.js' % mode).decode('utf-8')
            with open('%s/legacy-modes/%s.js' % (OUT, mode), 'w', encoding='utf-8') as f:
                f.write(relink(src, '../'))
    else:
        src = member(tf, entry(meta)).decode('utf-8')
        if re.search(r'''from\s*['"]\./''', src):
            sys.exit('%s has more than one file; vendor.sh only takes single-file packages' % name)
        with open(os.path.join(OUT, flat(name)), 'w', encoding='utf-8') as f:
            f.write(relink(src, './'))
    try:
        lic = next(m for m in tf.getnames() if re.match(r'package/LICEN[CS]E', m, re.I))
        with open(os.path.join(OUT, 'LICENSE-' + name.lstrip('@').replace('/', '-')), 'wb') as f:
            f.write(tf.extractfile(lic).read())
    except StopIteration:
        pass

name, v, keep = FONT
tf, sri, _ = tarball(name, v)
lines.append('%s@%s %s' % (name, v, sri))
for path in keep:
    base = os.path.basename(path)
    if base == 'LICENSE':
        base = 'LICENSE-jetbrains-mono'
    with open(os.path.join(OUT, base), 'wb') as f:
        f.write(member(tf, path))

with open(os.path.join(OUT, 'VERSIONS'), 'w') as f:
    f.write('\n'.join(lines) + '\n')
print('\n'.join(lines))
print('%d files, %d KB' % (sum(len(fs) for _, _, fs in os.walk(OUT)), sum(os.path.getsize(os.path.join(d, x)) for d, _, fs in os.walk(OUT) for x in fs) // 1024))
PY
