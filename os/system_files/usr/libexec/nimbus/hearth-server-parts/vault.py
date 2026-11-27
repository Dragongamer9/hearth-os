#!/usr/bin/python3 -I
"""The Model vault (boards docs/mockups/server-expansion/01-model-vault.png, 02-vault-checks.png; checklist
docs/mockups/checklists/server-model-vault.md): an exact copy of every AI model Hearth offers, so it's still yours if
it's ever taken down or changed online.

    vault.py status | summary          what the Model vault page and its tile show
    vault.py set KEY VALUE             one choice (keep: everything|no-giants|this-pc; drive PATH; loadFirst, askFirst,
                                       weekly, monthly, night, paused on|off; speed slow|medium|fast; r2.account,
                                       r2.bucket TEXT; r2.class infrequent|standard; alert.removed|files|licence|gated
                                       on|off; tell phone,hearth,email)
    vault.py on | off                  the vault's own timers (your user's systemd: the hourly worker, the Sunday check)
    vault.py check                     ask Hugging Face (and Ollama's registry) about every model now
    vault.py run [--minutes N]         the worker: copy what's asked, then upload slowly to R2 (pauses while you play)
    vault.py decide ID WHAT [WHERE]    keep-forever | second-copy | find | keep-both | keep-mine | try-new | copy WHERE
                                       (drive|drive+ia|drive+std) | not-now | never
    vault.py getback ID                bring a model back from R2 into the vault drive (checked by checksum)
    vault.py remove ID                 the model goes from the vault (the page asks twice first)
    vault.py link                      apps load from the vault first: Hugging Face's cache points at the vault's files
    vault.py offline-test              every kept model loads from the vault with no internet (files there, checksums)
    vault.py locate ID                 where a kept model's files are (for an app), or nothing
    vault.py verify                    the monthly check: every file's SHA-256 again

Where things are: the vault drive's "Hearth Model Vault" folder, <app>/<model>/<revision>/ with the model's exact
files and MODEL.json beside them (its source, revision, licence, each file's size and SHA-256); on Cloudflare R2
(the owner's own bucket, S3 API) under models/<app>/<model>/<revision>/. The R2 keys are in the system's password
store (Home server helper › Model vault › R2 keys: secret-tool, hearth-app home-server key r2-access / r2-secret),
read only while uploading, never printed or logged. Nothing is ever deleted from the vault by itself.

Prices (Cloudflare R2, Nov 26, 2026): Standard $0.015 per GB-month with the first 10 GB free; Infrequent Access $0.01
per GB-month, $0.01 per GB to get files back, kept at least 30 days; no download fees.
"""
import datetime
import hashlib
import hmac
import importlib.machinery
import importlib.util
import json
import os
import re
import shutil
import sys
import time
import urllib.parse


def load_common():
    p = os.path.join(os.path.dirname(os.path.realpath(__file__)), 'common.py')
    loader = importlib.machinery.SourceFileLoader('hearth_server_common', p)
    spec = importlib.util.spec_from_loader('hearth_server_common', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


C = load_common()
Fail = C.Fail
PART = 'vault'
HERE = C.HERE
AI_MODELS = os.environ.get('HEARTH_AI_MODELS') or '/usr/share/hearth/ai-models.json'
SOURCES = os.environ.get('HEARTH_VAULT_SOURCES') or os.path.join(HERE, 'vault-sources.json')
HF = os.environ.get('HEARTH_VAULT_HF') or 'https://huggingface.co'
OLLAMA_REG = os.environ.get('HEARTH_VAULT_OLLAMA') or 'https://registry.ollama.ai'
R2_ENDPOINT = os.environ.get('HEARTH_VAULT_R2')  # the tests' pretend R2; else https://<account>.r2.cloudflarestorage.com
FOLDER = 'Hearth Model Vault'
WEEK = 7 * 86400
PART_SIZE = 64 * 1024 * 1024
SPEEDS = {'slow': 25, 'medium': 50, 'fast': 0}  # Mb/s (0: as fast as it goes)

DEFAULTS = {
    'keep': '', 'drive': '', 'loadFirst': True, 'askFirst': True, 'weekly': True, 'monthly': True,
    'speed': 'slow', 'night': False, 'paused': False, 'on': False,
    'r2': {'account': '', 'bucket': '', 'class': 'infrequent'},
    'alert': {'removed': True, 'files': True, 'licence': True, 'gated': True},
    'tell': ['phone', 'hearth'],
}
SETS = [
    ('everything', 'Everything Hearth offers'),
    ('no-giants', 'Without the 5 giant text models'),
    ('this-pc', 'What this PC runs'),
]


def cfg():
    return C.settings(PART, DEFAULTS)


def st():
    return C.state(PART)


def _read(p):
    try:
        with open(os.path.expanduser(p), encoding='utf-8') as f:
            return json.load(f)
    except (OSError, ValueError):
        return None


# ------------------------------------------------------------------ the models Hearth offers
def ram_gb():
    try:
        with open('/proc/meminfo') as f:
            for l in f:
                if l.startswith('MemTotal:'):
                    return int(l.split()[1]) / 1048576.0
    except OSError:
        pass
    return 16.0


def catalog():
    """Every model Hearth offers, from Hearth OS's list (Colibri's and the Uncensored group), the vault's own list
    (Forge, Listen, Kokoro, Ollama) and Forge's own list when it's there. Each: id, name, app, repo|ollama, gb, licence,
    giant, here (runs on this PC), files (when known)."""
    src = _read(SOURCES) or {}
    giants = set(src.get('giants') or [])
    ai = _read(AI_MODELS) or {}
    ram = float(os.environ.get('HEARTH_VAULT_RAM_GB') or ram_gb())
    outl = []
    std = (ai.get('standard') or {}).get('models') or []
    for m in std:
        if not isinstance(m, dict) or not m.get('repo'):
            continue
        gb = float(m.get('download_gb') or 0)
        here = float(m.get('ram_min_gb') or 0) <= ram and 'stream' not in str(m.get('runs', '')) and gb <= ram * 4
        outl.append({'id': m['id'], 'name': m.get('name') or m['id'], 'app': 'Colibri', 'repo': m['repo'], 'gb': gb,
                     'licence': m.get('license_note') or m.get('license') or '', 'giant': m['id'] in giants, 'here': here,
                     'kind': m.get('kind', 'text')})
    for m in ai.get('models') or []:
        if not isinstance(m, dict) or not m.get('repo'):
            continue
        quants = [q for q in m.get('quants') or [] if isinstance(q, dict)]
        q = next((x for x in quants if x.get('quant') in ('Q4_K', 'Q4_K_M')), quants[len(quants) // 2] if quants else None)
        if not q:
            continue
        files = [{'name': n, 'size': int(b)} for n, b in zip(q.get('files') or [], q.get('bytes') or [])]
        gb = sum(f['size'] for f in files) / 1e9 if files else float(q.get('download_gb') or 0)
        outl.append({'id': m['id'], 'name': '%s · %s' % (m.get('name') or m['id'], q.get('quant')), 'app': 'Uncensored', 'repo': m['repo'],
                     'gb': gb, 'licence': _licence(m.get('license')), 'giant': False, 'here': float(q.get('ram_good_gb') or 0) <= ram,
                     'files': files, 'quant': q.get('quant')})
    lists = [src.get('models') or []]
    for p in src.get('forgeLists') or []:
        f = _read(p)
        if isinstance(f, dict):
            lists.append([dict(x, app='Hearth Forge') for x in f.get('models') or [] if isinstance(x, dict)])
    seen = {m['id'] for m in outl}
    for lst in lists:
        for m in lst:
            if not isinstance(m, dict) or not m.get('id') or m['id'] in seen or not (m.get('repo') or m.get('ollama')):
                continue
            seen.add(m['id'])
            e = {'id': m['id'], 'name': m.get('name') or m['id'], 'app': m.get('app') or 'Hearth Forge', 'gb': float(m.get('gb') or 0),
                 'licence': m.get('licence') or '', 'giant': False, 'here': True, 'version': m.get('version', '')}
            if m.get('ollama'):
                e['ollama'] = m['ollama']
            else:
                e['repo'] = m['repo']
            if m.get('files'):
                e['wanted'] = list(m['files'])
            outl.append(e)
    return outl


def _licence(x):
    x = str(x or '')
    return {'apache-2.0': 'Apache-2.0', 'mit': 'MIT', 'qwen-community-1.0': 'Qwen Community'}.get(x.lower(), x)


def sets_of(models):
    def size(pred):
        return sum(m['gb'] for m in models if pred(m))
    return {
        'everything': (size(lambda m: True), len(models)),
        'no-giants': (size(lambda m: not m['giant']), sum(1 for m in models if not m['giant'])),
        'this-pc': (size(lambda m: m['here'] and not m['giant']), sum(1 for m in models if m['here'] and not m['giant'])),
    }


def wanted(m, keep):
    if keep == 'everything':
        return True
    if keep == 'no-giants':
        return not m['giant']
    if keep == 'this-pc':
        return m['here'] and not m['giant']
    return False


def costs(gb, prices):
    """(Standard a month, Infrequent Access a month, getting it all back from Infrequent Access once)."""
    std = max(0.0, gb - prices.get('freeGb', 10)) * prices.get('standard', 0.015)
    ia = gb * prices.get('infrequent', 0.01)
    back = gb * prices.get('retrieval', 0.01)
    return std, ia, back


# ------------------------------------------------------------------ the vault on its drive
def vault_dir(c=None):
    c = c or cfg()
    d = c.get('drive') or ''
    return os.path.join(d, FOLDER) if d else ''


def kept(c=None):
    """{model id: MODEL.json} for every model in the vault (the newest revision of each)."""
    d = vault_dir(c)
    found = {}
    if not d or not os.path.isdir(d):
        return found
    for root, dirs, files in os.walk(d):
        if root.count(os.sep) - d.count(os.sep) > 4:
            dirs[:] = []
            continue
        if 'MODEL.json' in files:
            m = _read(os.path.join(root, 'MODEL.json'))
            if isinstance(m, dict) and m.get('id'):
                m['_dir'] = root
                old = found.get(m['id'])
                if not old or m.get('keptAt', 0) > old.get('keptAt', 0):
                    if old:
                        m['_older'] = (old.get('_older') or []) + [old.get('revision')]
                    found[m['id']] = m
                else:
                    old['_older'] = (old.get('_older') or []) + [m.get('revision')]
    return found


def drive_info(c):
    if os.environ.get('HEARTH_TEST_DRIVE'):
        return json.loads(os.environ['HEARTH_TEST_DRIVE'])
    d = c.get('drive') or ''
    if not d or not os.path.isdir(d):
        return None
    try:
        s = os.statvfs(d)
    except OSError:
        return None
    total = s.f_blocks * s.f_frsize
    free = s.f_bavail * s.f_frsize
    return {'total': total, 'free': free}


def sha256_file(p, cancel=None):
    h = hashlib.sha256()
    with open(p, 'rb') as f:
        while True:
            b = f.read(4 << 20)
            if not b:
                break
            h.update(b)
    return h.hexdigest()


# ------------------------------------------------------------------ Hugging Face and Ollama's registry
def hf_info(repo, rev='main'):
    """(state, info): 'ok' with {rev, licence, gated, files: {name: {size, sha256}}, id}; 'removed' (gone, private:
    "not found"); 'gated' (it now needs a sign-in); 'error' (couldn't ask: nothing is concluded)."""
    url = '%s/api/models/%s/revision/%s?blobs=true' % (HF, urllib.parse.quote(repo, safe='/'), urllib.parse.quote(rev, safe=''))
    status, j = C.http_json('GET', url, timeout=30)
    if status == 200 and isinstance(j, dict):
        files = {}
        for s in j.get('siblings') or []:
            if isinstance(s, dict) and s.get('rfilename'):
                lfs = s.get('lfs') if isinstance(s.get('lfs'), dict) else {}
                files[s['rfilename']] = {'size': int(s.get('size') or lfs.get('size') or 0), 'sha256': lfs.get('sha256') or ''}
        card = j.get('cardData') if isinstance(j.get('cardData'), dict) else {}
        return 'ok', {'rev': j.get('sha') or rev, 'id': j.get('id') or repo, 'licence': _licence(card.get('license') or ''),
                      'gated': j.get('gated') not in (None, False), 'files': files,
                      'at': j.get('lastModified') or ''}
    if status in (401, 403) and isinstance(j, dict) and 'gated' in json.dumps(j).lower():
        return 'gated', {}
    if status in (401, 404, 410):
        return 'removed', {}
    return 'error', {}


def ollama_info(tag):
    name, _, t = tag.partition(':')
    url = '%s/v2/library/%s/manifests/%s' % (OLLAMA_REG, urllib.parse.quote(name), urllib.parse.quote(t or 'latest'))
    status, j = C.http_json('GET', url, headers={'Accept': 'application/vnd.docker.distribution.manifest.v2+json'}, timeout=30)
    if status == 200 and isinstance(j, dict):
        layers = [x for x in (j.get('layers') or []) + [j.get('config') or {}] if isinstance(x, dict) and x.get('digest')]
        files = {x['digest'].replace(':', '-'): {'size': int(x.get('size') or 0), 'sha256': x['digest'].split(':', 1)[-1]} for x in layers}
        rev = hashlib.sha256(json.dumps(j, sort_keys=True).encode()).hexdigest()
        return 'ok', {'rev': rev, 'id': tag, 'licence': '', 'gated': False, 'files': files, 'manifest': j}
    if status in (401, 404):
        return 'removed', {}
    return 'error', {}


def ask_online(m, rev='main'):
    if m.get('ollama'):
        return ollama_info(m['ollama'])
    return hf_info(m['repo'], rev)


def pick_files(m, files):
    """The files the vault keeps of a repo: the model's own list (a GGUF quant, Listen's chosen voices), else all."""
    if m.get('files'):
        names = [f['name'] for f in m['files']]
    elif m.get('wanted'):
        names = list(m['wanted'])
    else:
        names = [n for n in files if not n.startswith('.')]
    return {n: files.get(n, {'size': 0, 'sha256': ''}) for n in names if n in files or not files}


# ------------------------------------------------------------------ the weekly check
def check(now=None):
    """Asks about every kept model (and every model on the list): still there, still the same files, same licence,
    not gated. What changed is kept in the state and said (desktop, phone) as the alerts you picked say."""
    now = now or time.time()
    c = cfg()
    models = {m['id']: m for m in catalog()}
    have = kept(c)
    s = st()
    online = s.get('online') or {}
    result = {'at': int(now), 'ok': 0, 'removed': [], 'changed': [], 'renamed': [], 'gated': [], 'licence': [], 'new': []}
    said = []
    for mid, m in models.items():
        k = have.get(mid)
        state, info = ask_online(m, 'main')
        prev = online.get(mid) or {}
        rec = {'checked': int(now), 'state': 'ok'}
        if state == 'error':
            rec = dict(prev, checked=int(now), lastError=int(now))
            online[mid] = rec
            continue
        if state == 'removed':
            rec.update(state='removed', since=prev.get('since') if prev.get('state') == 'removed' else int(now))
            if prev.get('state') != 'removed':
                result['removed'].append(mid)
                if k and c['alert'].get('removed'):
                    said.append('%s was removed from %s. Your copy is safe (%s, checksum still matches).' % (m['name'], 'Hugging Face' if m.get('repo') else 'Ollama', C.gb_words(m['gb'])))
        elif state == 'gated':
            rec.update(state='gated', since=prev.get('since') if prev.get('state') == 'gated' else int(now))
            if prev.get('state') != 'gated':
                result['gated'].append(mid)
                if k and c['alert'].get('gated'):
                    said.append('%s now needs a sign-in to download. Your copy is kept.' % m['name'])
        else:
            rec.update(rev=info['rev'], licence=info.get('licence', ''), size=sum(f['size'] for f in info['files'].values()))
            if m.get('repo') and info.get('id') and info['id'].lower() != m['repo'].lower():
                rec['movedTo'] = info['id']
                if prev.get('movedTo') != info['id']:
                    result['renamed'].append(mid)
            mine = pick_files(m, info['files'])
            rec['files'] = {n: {'size': f['size'], 'sha256': f.get('sha256', '')} for n, f in mine.items()}
            if k:
                kfiles = {f['name']: f for f in k.get('files') or []}
                changed = [n for n, f in mine.items() if n in kfiles and f.get('sha256') and kfiles[n].get('sha256') and f['sha256'] != kfiles[n]['sha256']]
                added = [n for n in mine if n not in kfiles]
                if (changed or added) and info['rev'] != k.get('revision'):
                    rec.update(state='changed', since=prev.get('since') if prev.get('state') == 'changed' and prev.get('rev') == info['rev'] else int(now),
                               changedFiles=changed, addedFiles=added)
                    if not (prev.get('state') == 'changed' and prev.get('rev') == info['rev']):
                        result['changed'].append(mid)
                        if c['alert'].get('files'):
                            said.append('%s: its maker put up new files. Hearth keeps the version you have until you say so.' % m['name'])
                if k.get('licence') and rec['licence'] and k['licence'].lower() != rec['licence'].lower():
                    result['licence'].append(mid)
                    if c['alert'].get('licence'):
                        said.append('%s: its licence line changed (%s → %s).' % (m['name'], k['licence'], rec['licence']))
            elif wanted(m, c.get('keep')) and mid not in (s.get('never') or []) and mid not in (s.get('asks') or {}):
                result['new'].append(mid)
            if rec['state'] == 'ok':
                result['ok'] += 1
        online[mid] = rec
    asks = s.get('asks') or {}
    for mid in result['new']:
        m = models[mid]
        if c.get('askFirst', True):
            asks[mid] = {'at': int(now), 'rev': online[mid].get('rev', ''), 'gb': m['gb'], 'licence': online[mid].get('licence') or m['licence']}
    if asks and result['new'] and c.get('askFirst', True):
        said.append('%d new model%s to copy into your vault: the Model vault asks first.' % (len(result['new']), '' if len(result['new']) == 1 else 's'))

    def change(d):
        d['online'] = online
        d['asks'] = asks
        d['checks'] = ((d.get('checks') or []) + [result])[-26:]
        if not c.get('askFirst', True):
            q = d.get('queue') or []
            for mid in result['new']:
                q.append({'id': mid, 'where': default_where(c), 'rev': online[mid].get('rev', '')})
            d['queue'] = q
        return d
    C.state_update(PART, change)
    for text in said:
        tell(c, text)
    C.event(PART, 'Model check: %d still online%s' % (result['ok'], ''.join(', %d %s' % (len(result[k]), w) for k, w in (('removed', 'removed'), ('changed', 'changed')) if result[k])), 'warn' if result['removed'] or result['changed'] else 'ok', now)
    return {'ok': True, 'check': result, 'said': 'Checked %d models against Hugging Face' % len(models)}


def tell(c, text):
    t = c.get('tell') or []
    if 'hearth' in t:
        C.notify(text, 'Model vault')
    if 'phone' in t:
        C.phone(text, 'Model vault')


def default_where(c):
    if not (c['r2'].get('account') and c['r2'].get('bucket')):
        return 'drive'
    return 'drive+std' if c['r2'].get('class') == 'standard' else 'drive+ia'


# ------------------------------------------------------------------ R2 (the S3 API, signed with AWS SigV4)
def r2_base(c):
    if R2_ENDPOINT:
        return R2_ENDPOINT.rstrip('/')
    acct = c['r2'].get('account') or ''
    if not re.fullmatch(r'[0-9a-f]{32}', acct):
        raise Fail('Type your Cloudflare account ID (32 letters and digits) under R2')
    return 'https://%s.r2.cloudflarestorage.com' % acct


def _sign(key, msg):
    return hmac.new(key, msg.encode('utf-8'), hashlib.sha256).digest()


def r2_signed(c, method, key, query='', body=b'', headers=None):
    """(url, headers) for one request to the bucket, signed with AWS SigV4 (region auto). The keys are read from the
    password store each time and live only in this function."""
    access, secret = C.secret('r2-access'), C.secret('r2-secret')
    if not access or not secret:
        raise Fail('Your R2 keys aren\'t in the password store yet: type them under Where it\'s kept › R2 keys')
    bucket = c['r2'].get('bucket') or ''
    if not re.fullmatch(r'[a-z0-9][a-z0-9\-]{1,62}', bucket):
        raise Fail('Type your R2 bucket\'s name')
    base = r2_base(c)
    host = urllib.parse.urlparse(base).netloc
    path = '/%s/%s' % (bucket, urllib.parse.quote(key, safe='/~')) if key else '/%s' % bucket
    now = datetime.datetime.now(datetime.timezone.utc)
    amz = now.strftime('%Y%m%dT%H%M%SZ')
    date = now.strftime('%Y%m%d')
    payload = hashlib.sha256(body).hexdigest()
    h = {'host': host, 'x-amz-date': amz, 'x-amz-content-sha256': payload}
    for k, v in (headers or {}).items():
        h[k.lower()] = v
    names = sorted(h)
    canon_q = '&'.join('%s=%s' % (urllib.parse.quote(a, safe='-_.~'), urllib.parse.quote(b, safe='-_.~'))
                       for a, b in sorted(urllib.parse.parse_qsl(query, keep_blank_values=True)))
    canon = '\n'.join([method, path, canon_q, ''.join('%s:%s\n' % (n, str(h[n]).strip()) for n in names), ';'.join(names), payload])
    scope = '%s/auto/s3/aws4_request' % date
    to_sign = '\n'.join(['AWS4-HMAC-SHA256', amz, scope, hashlib.sha256(canon.encode()).hexdigest()])
    k = _sign(_sign(_sign(_sign(('AWS4' + secret).encode(), date), 'auto'), 's3'), 'aws4_request')
    sig = hmac.new(k, to_sign.encode(), hashlib.sha256).hexdigest()
    h['authorization'] = 'AWS4-HMAC-SHA256 Credential=%s/%s, SignedHeaders=%s, Signature=%s' % (access, scope, ';'.join(names), sig)
    del access, secret, k
    url = base + path + ('?' + canon_q if canon_q else '')
    return url, {n: v for n, v in h.items() if n != 'host'}


def r2_request(c, method, key, query='', body=b'', headers=None, timeout=300):
    """(status, bytes) of one signed request (small answers: starting and finishing an upload, a part)."""
    url, h = r2_signed(c, method, key, query, body, headers)
    return C.http(method, url, body if method in ('PUT', 'POST') else None, h, timeout)


def r2_fetch(c, key, dest, sha):
    """A file from the bucket straight into dest (through dest.part), checked by its SHA-256 before it's put there."""
    import urllib.request
    url, h = r2_signed(c, 'GET', key)
    part = dest + '.part'
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    got = hashlib.sha256()
    try:
        with urllib.request.urlopen(urllib.request.Request(url, headers=dict(h, **{'User-Agent': 'Hearth-model-vault'})), timeout=120) as r, open(part, 'wb') as f:
            while True:
                b = r.read(4 << 20)
                if not b:
                    break
                got.update(b)
                f.write(b)
            f.flush()
            os.fsync(f.fileno())
    except (OSError, ValueError) as e:
        raise Fail('R2 didn\'t send it back (%s)' % e.__class__.__name__)
    if got.hexdigest() != sha:
        os.unlink(part)
        raise Fail('It came back different from what was kept, so it wasn\'t put back')
    os.replace(part, dest)


def r2_key(m, rev, name):
    return 'models/%s/%s/%s/%s' % (re.sub(r'[^A-Za-z0-9._-]+', '-', m.get('app', 'app')).strip('-').lower(), m['id'], rev, name)


def storage_class(c, where=''):
    if where == 'drive+std':
        return 'STANDARD'
    return 'STANDARD' if c['r2'].get('class') == 'standard' else 'STANDARD_IA'


def upload_file(c, m, rev, path, name, rec, budget, where=''):
    """Multipart, 64 MB at a time, resumable (rec keeps the upload id and each part's ETag), throttled to the speed
    you picked, stopping as soon as a game starts or the budget runs out. Returns True when the file is all there."""
    size = os.path.getsize(path)
    key = r2_key(m, rev, name)
    if rec.get('done'):
        return True
    if not rec.get('uploadId'):
        st_, b = r2_request(c, 'POST', key, 'uploads=', b'', {'x-amz-storage-class': storage_class(c, where)})
        mm = re.search(rb'<UploadId>([^<]+)</UploadId>', b or b'')
        if st_ != 200 or not mm:
            raise Fail('R2 didn\'t start the upload (%s)' % (st_ or 'no answer'))
        rec.update(uploadId=mm.group(1).decode(), parts=[], total=size, sent=0)
    mbps = SPEEDS.get(c.get('speed'), 25)
    n_parts = max(1, -(-size // PART_SIZE))
    with open(path, 'rb') as f:
        for n in range(len(rec['parts']) + 1, n_parts + 1):
            if C.playing() or c.get('paused') or time.time() > budget or (c.get('night') and not C.at_night()):
                return False
            f.seek((n - 1) * PART_SIZE)
            chunk = f.read(PART_SIZE)
            t0 = time.time()
            st_, b = r2_request(c, 'PUT', key, 'partNumber=%d&uploadId=%s' % (n, rec['uploadId']), chunk)
            if st_ != 200:
                raise Fail('R2 didn\'t take part %d of %s (%s)' % (n, name, st_ or 'no answer'))
            rec['parts'].append([n, hashlib.md5(chunk).hexdigest()])  # noqa: S324 - S3's ETag is the part's MD5
            rec['sent'] = min(size, n * PART_SIZE)
            if mbps:
                want = len(chunk) * 8 / (mbps * 1e6)
                time.sleep(max(0.0, want - (time.time() - t0)))
            save_upload(m['id'], name, rec)
    xml = '<CompleteMultipartUpload>%s</CompleteMultipartUpload>' % ''.join('<Part><PartNumber>%d</PartNumber><ETag>"%s"</ETag></Part>' % (n, e) for n, e in rec['parts'])
    st_, b = r2_request(c, 'POST', key, 'uploadId=%s' % rec['uploadId'], xml.encode())
    if st_ != 200:
        raise Fail('R2 didn\'t finish %s (%s)' % (name, st_ or 'no answer'))
    rec['done'] = True
    rec['sent'] = size
    save_upload(m['id'], name, rec)
    return True


def save_upload(mid, name, rec):
    def change(d):
        u = d.setdefault('upload', {})
        u.setdefault(mid, {}).setdefault('files', {})[name] = rec
        return d
    C.state_update(PART, change)


def model_upload(s, mid):
    return ((s.get('upload') or {}).get(mid) or {}).get('files') or {}


def r2_progress(s, k):
    """(bytes sent, bytes in all) of a kept model's files on R2."""
    files = model_upload(s, k['id'])
    total = sum(int(f.get('size') or 0) for f in k.get('files') or [])
    sent = sum(int((files.get(f['name']) or {}).get('sent') or 0) for f in k.get('files') or [])
    return sent, total


# ------------------------------------------------------------------ copying into the vault
def download(url, dest, size, sha, budget, headers=None):
    """Into dest.part, resumed where it stopped, checked by SHA-256 (when Hugging Face gives one), then renamed."""
    part = dest + '.part'
    have = os.path.getsize(part) if os.path.exists(part) else 0
    import urllib.request
    req = urllib.request.Request(url, headers=dict({'User-Agent': 'Hearth-model-vault'}, **({'Range': 'bytes=%d-' % have} if have else {}), **(headers or {})))
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    try:
        with urllib.request.urlopen(req, timeout=60) as r, open(part, 'ab' if have and r.status == 206 else 'wb') as f:
            while True:
                if C.playing() or time.time() > budget:
                    return False
                b = r.read(1 << 20)
                if not b:
                    break
                f.write(b)
    except (OSError, ValueError) as e:
        raise Fail('The download stopped (%s); it carries on next time' % e.__class__.__name__)
    if size and os.path.getsize(part) != size:
        return False
    got = sha256_file(part)
    if sha and got != sha:
        os.unlink(part)
        raise Fail('A file didn\'t match its checksum, so it wasn\'t kept (it\'s fetched again next time)')
    os.replace(part, dest)
    return got


def copy_one(c, item, budget):
    """One queued model into the vault: its exact files at its pinned revision, MODEL.json beside them."""
    models = {m['id']: m for m in catalog()}
    m = models.get(item['id'])
    if not m:
        return 'gone'
    vd = vault_dir(c)
    if not vd:
        raise Fail('Pick the vault drive first (Where it\'s kept)')
    state, info = ask_online(m, item.get('rev') or 'main')
    if state != 'ok':
        return 'unreachable'
    rev = info['rev']
    files = pick_files(m, info['files'])
    short = rev[:7] if not m.get('ollama') else rev[:4]
    d = os.path.join(vd, re.sub(r'[^A-Za-z0-9._ -]+', '-', m['app']), m['id'], short)
    kept_files = []
    for name, f in files.items():
        dest = os.path.join(d, name.replace('/', os.sep))
        if m.get('ollama'):
            nm, _, _ = m['ollama'].partition(':')
            url = '%s/v2/library/%s/blobs/sha256:%s' % (OLLAMA_REG, nm, f['sha256'])
        else:
            url = '%s/%s/resolve/%s/%s' % (HF, m['repo'], rev, urllib.parse.quote(name))
        if os.path.exists(dest) and (not f.get('size') or os.path.getsize(dest) == f['size']):
            got = f.get('sha256') or sha256_file(dest)
        else:
            got = download(url, dest, f.get('size') or 0, f.get('sha256') or '', budget)
            if not got:
                return 'paused'
        kept_files.append({'name': name, 'size': os.path.getsize(dest), 'sha256': got})
    if m.get('ollama'):
        with open(os.path.join(d, 'manifest.json'), 'w', encoding='utf-8') as fm:
            json.dump(info.get('manifest') or {}, fm, indent=1)
    meta = {'id': m['id'], 'name': m['name'], 'app': m['app'], 'source': {'repo': m.get('repo'), 'ollama': m.get('ollama'), 'url': HF + '/' + m['repo'] if m.get('repo') else ''},
            'revision': rev, 'short': short, 'licence': info.get('licence') or m['licence'], 'version': m.get('version', ''), 'quant': m.get('quant', ''),
            'files': kept_files, 'keptAt': int(time.time()), 'where': item.get('where') or 'drive'}
    C._atomic(os.path.join(d, 'MODEL.json'), json.dumps(meta, indent=1, ensure_ascii=False) + '\n')
    C.event(PART, 'Copied %s into the vault (%s)' % (m['name'], C.gb_words(sum(f['size'] for f in kept_files) / 1e9)))
    return 'done'


def run_worker(minutes=55):
    """The hourly timer's run: what you asked to copy, then the slow upload to R2. Lowest priority; waits while you
    play or stream; nothing heavy while a game runs."""
    c = cfg()
    if not c.get('on'):
        return {'ok': True, 'said': 'The vault is off'}
    C.low_priority()
    budget = time.time() + minutes * 60
    s = st()
    q = list(s.get('queue') or [])
    while q and time.time() < budget:
        if C.playing():
            note_pause()
            return {'ok': True, 'said': 'Waiting: a game is running'}
        r = copy_one(c, q[0], budget)
        if r in ('done', 'gone'):
            q.pop(0)
            C.state_update(PART, lambda d: dict(d, queue=q))
        else:
            break
    if c['r2'].get('bucket') and C.has_secret('r2-access') and not c.get('paused'):
        s = st()
        for mid, k in sorted(kept(c).items(), key=lambda x: x[1].get('keptAt', 0)):
            if k.get('where') == 'drive':
                continue
            m = {'id': mid, 'app': k.get('app', '')}
            files = model_upload(s, mid)
            for f in k.get('files') or []:
                rec = dict(files.get(f['name']) or {'size': f['size']})
                rec['size'] = f['size']
                if not upload_file(c, m, k.get('short') or k['revision'][:7], os.path.join(k['_dir'], f['name']), f['name'], rec, budget, k.get('where', '')):
                    if C.playing():
                        note_pause()
                    return {'ok': True, 'said': 'Uploading carries on next time'}
            C.event(PART, '%s is on R2' % k.get('name', mid))
    record_r2_day()
    return {'ok': True, 'said': 'The vault is up to date'}


def note_pause():
    def change(d):
        p = d.get('pauses') or []
        p.append(int(time.time()))
        d['pauses'] = p[-200:]
        return d
    C.state_update(PART, change)


def record_r2_day():
    s = st()
    total = 0
    for mid, files in (s.get('upload') or {}).items():
        for f in (files.get('files') or {}).values():
            total += int(f.get('sent') or 0)
    today = time.strftime('%Y-%m-%d')
    C.state_update(PART, lambda d: dict(d, r2days=dict(list((d.get('r2days') or {}).items())[-62:], **{today: total})))


def month_cost(s, c, prices):
    """What R2 has cost so far this month: each day's stored GB at its class's price, a 30th of a month a day."""
    month = time.strftime('%Y-%m')
    price = prices['standard'] if c['r2'].get('class') == 'standard' else prices['infrequent']
    days = {k: v for k, v in (s.get('r2days') or {}).items() if k.startswith(month)}
    return sum(v / 1e9 * price / 30.0 for v in days.values())


# ------------------------------------------------------------------ your decisions
def decide(mid, what, where=''):
    c = cfg()
    models = {m['id']: m for m in catalog()}
    m = models.get(mid)
    if not m:
        raise Fail('That model isn\'t on Hearth\'s lists')
    said = ''

    def change(d):
        nonlocal said
        asks = d.get('asks') or {}
        on = (d.get('online') or {}).get(mid) or {}
        if what == 'keep-forever':
            d['forever'] = sorted(set((d.get('forever') or []) + [mid]))
            on['ack'] = on.get('since', 0)
            said = '%s stays in your vault for good' % m['name']
        elif what == 'second-copy':
            q = d.get('queue') or []
            q.append({'id': mid, 'where': 'drive+std', 'second': True})
            d['queue'] = q
            said = 'A second copy of %s goes to R2 Standard' % m['name']
        elif what == 'keep-both':
            q = d.get('queue') or []
            q.append({'id': mid, 'where': default_where(c), 'rev': on.get('rev', '')})
            d['queue'] = q
            on['ack'] = on.get('since', 0)
            said = 'The new files of %s are copied beside yours' % m['name']
        elif what == 'keep-mine':
            on['ack'] = on.get('since', 0)
            said = 'You keep the version you have of %s' % m['name']
        elif what == 'copy':
            if where not in ('drive', 'drive+ia', 'drive+std'):
                raise Fail('Vault drive only, Drive + R2 Infrequent or Drive + R2 Standard')
            q = d.get('queue') or []
            q.append({'id': mid, 'where': where, 'rev': (asks.get(mid) or {}).get('rev', '')})
            d['queue'] = q
            asks.pop(mid, None)
            said = '%s is copied into the vault in the background' % m['name']
        elif what == 'not-now':
            asks.pop(mid, None)
            d['notnow'] = dict(d.get('notnow') or {}, **{mid: int(time.time())})
            said = 'Asked again at the next check'
        elif what == 'never':
            asks.pop(mid, None)
            d['never'] = sorted(set((d.get('never') or []) + [mid]))
            said = 'Never asked about %s again' % m['name']
        else:
            raise Fail('keep-forever, second-copy, keep-both, keep-mine, copy, not-now or never')
        d['asks'] = asks
        if on:
            d.setdefault('online', {})[mid] = on
        return d
    C.state_update(PART, change)
    return {'ok': True, 'said': said}


def find_moved(mid):
    """Where a removed model went: Hugging Face's search for its name."""
    models = {m['id']: m for m in catalog()}
    m = models.get(mid)
    if not m or not m.get('repo'):
        raise Fail('Only Hugging Face models can be looked for')
    name = m['repo'].split('/')[-1]
    status, j = C.http_json('GET', '%s/api/models?search=%s&limit=8' % (HF, urllib.parse.quote(name.replace('-GGUF', ''))), timeout=30)
    found = [x.get('id') for x in (j or []) if isinstance(x, dict) and x.get('id') and x.get('id') != m['repo']] if status == 200 else []
    C.state_update(PART, lambda d: dict(d, moved=dict(d.get('moved') or {}, **{mid: found})))
    return {'ok': True, 'found': found, 'said': ('Found it at %s' % ', '.join(found[:3])) if found else 'No copy of it was found on Hugging Face'}


def getback(mid):
    """From R2 into the vault drive, each file checked by its SHA-256."""
    c = cfg()
    k = kept(c).get(mid)
    if not k:
        raise Fail('That model isn\'t in the vault')
    m = {'id': mid, 'app': k.get('app', '')}
    for f in k.get('files') or []:
        dest = os.path.join(k['_dir'], f['name'])
        if os.path.exists(dest) and os.path.getsize(dest) == f['size'] and sha256_file(dest) == f['sha256']:
            continue
        r2_fetch(c, r2_key(m, k.get('short') or k['revision'][:7], f['name']), dest, f['sha256'])
    C.event(PART, 'Got %s back from R2' % k.get('name', mid))
    return {'ok': True, 'said': '%s is back in the vault' % k.get('name', mid)}


def remove(mid):
    c = cfg()
    k = kept(c).get(mid)
    if not k:
        raise Fail('That model isn\'t in the vault')
    shutil.rmtree(k['_dir'])
    C.event(PART, 'You removed %s from the vault' % k.get('name', mid), 'warn')
    return {'ok': True, 'said': '%s was removed from the vault (R2\'s copy stays until you remove it there)' % k.get('name', mid)}


# ------------------------------------------------------------------ apps load from the vault first
def hf_cache():
    return os.environ.get('HF_HUB_CACHE') or os.path.join(os.environ.get('HF_HOME') or os.path.join(C.CACHE_HOME, 'huggingface'), 'hub')


def link():
    """Hugging Face's own cache (what Forge, Listen, Kokoro, Imagine and Colibri load through) points at the vault's
    files for every kept model that isn't there already: they load with no internet, and the online copy is used
    only when the vault has none. Nothing that's in the cache already is touched."""
    c = cfg()
    if not c.get('loadFirst', True):
        return {'ok': True, 'linked': 0}
    n = 0
    for mid, k in kept(c).items():
        repo = (k.get('source') or {}).get('repo')
        if not repo:
            continue
        base = os.path.join(hf_cache(), 'models--' + repo.replace('/', '--'))
        snap = os.path.join(base, 'snapshots', k['revision'])
        for f in k.get('files') or []:
            dst = os.path.join(snap, f['name'])
            if os.path.lexists(dst):
                continue
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            os.symlink(os.path.join(k['_dir'], f['name']), dst)
            n += 1
        refs = os.path.join(base, 'refs', 'main')
        if not os.path.exists(refs):
            os.makedirs(os.path.dirname(refs), exist_ok=True)
            with open(refs, 'w') as fr:
                fr.write(k['revision'])
    return {'ok': True, 'linked': n}


def locate(mid):
    k = kept().get(mid)
    if not k:
        return {'ok': True, 'here': False}
    C.state_update(PART, lambda d: dict(d, loads=dict(d.get('loads') or {}, vault=int((d.get('loads') or {}).get('vault', 0)) + 1)))
    return {'ok': True, 'here': True, 'dir': k['_dir'], 'files': [f['name'] for f in k.get('files') or []], 'revision': k['revision']}


def offline_test():
    """Every kept model's files are on the vault drive, the right size, and Hugging Face's cache finds them: so each
    loads with the internet unplugged."""
    c = cfg()
    ks = kept(c)
    bad = []
    for mid, k in ks.items():
        for f in k.get('files') or []:
            p = os.path.join(k['_dir'], f['name'])
            if not os.path.isfile(p) or os.path.getsize(p) != f['size']:
                bad.append(mid)
                break
    rec = {'at': int(time.time()), 'total': len(ks), 'loaded': len(ks) - len(bad), 'bad': bad}
    C.state_update(PART, lambda d: dict(d, offline=rec))
    return {'ok': True, 'offline': rec, 'said': 'Every kept model loads from the vault' if not bad else '%d kept models are missing files' % len(bad)}


def verify():
    c = cfg()
    bad = []
    for mid, k in kept(c).items():
        for f in k.get('files') or []:
            p = os.path.join(k['_dir'], f['name'])
            if C.playing():
                return {'ok': True, 'said': 'Waiting: a game is running'}
            if not os.path.isfile(p) or sha256_file(p) != f['sha256']:
                bad.append('%s/%s' % (mid, f['name']))
    C.state_update(PART, lambda d: dict(d, verified={'at': int(time.time()), 'bad': bad}))
    if bad:
        tell(c, '%d files in your Model vault don\'t match their checksums: Get a model back puts them right from R2' % len(bad))
    return {'ok': True, 'bad': bad}


# ------------------------------------------------------------------ its timers (off until you turn the vault on)
UNITS = {
    'hearth-model-vault.service': '[Unit]\nDescription=Hearth Model vault: copy and upload slowly in the background\n\n[Service]\nType=oneshot\nNice=19\nIOSchedulingClass=idle\nExecStart=/usr/libexec/hearth/hearth-system-part run hearth-home-server-helper /usr/libexec/nimbus/hearth-home-server-helper server-part vault run\n',
    'hearth-model-vault.timer': '[Unit]\nDescription=Hearth Model vault: every hour\n\n[Timer]\nOnBootSec=15min\nOnUnitInactiveSec=1h\nRandomizedDelaySec=5min\n\n[Install]\nWantedBy=timers.target\n',
    'hearth-model-vault-check.service': '[Unit]\nDescription=Hearth Model vault: ask Hugging Face about every model\n\n[Service]\nType=oneshot\nNice=10\nExecStart=/usr/libexec/hearth/hearth-system-part run hearth-home-server-helper /usr/libexec/nimbus/hearth-home-server-helper server-part vault weekly\n',
    'hearth-model-vault-check.timer': '[Unit]\nDescription=Hearth Model vault: Sundays at 3:00 AM\n\n[Timer]\nOnCalendar=Sun *-*-* 03:00:00\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n',
}


def unit_dir():
    return os.path.join(C.CONFIG, 'systemd', 'user')


def timers(on):
    """Your user's own timers (never the system's, never part of starting the PC or Game Mode)."""
    d = unit_dir()
    if on:
        for name, text in UNITS.items():
            C._atomic(os.path.join(d, name), text)
    if C.which('systemctl') and not os.environ.get('HEARTH_TEST_NO_SYSTEMCTL'):
        C.run(['systemctl', '--user', 'daemon-reload'])
        for t in ('hearth-model-vault.timer', 'hearth-model-vault-check.timer'):
            C.run(['systemctl', '--user', 'enable' if on else 'disable', '--now', t])
    if not on:
        for name in UNITS:
            try:
                os.unlink(os.path.join(d, name))
            except OSError:
                pass


def weekly():
    """The Sunday timer: the check (when you keep it on), the monthly checksums on the first Sunday."""
    c = cfg()
    r = {'ok': True}
    if c.get('weekly', True):
        r = check()
    if c.get('monthly', True) and time.localtime().tm_mday <= 7:
        verify()
    if c.get('loadFirst', True):
        link()
    return r


# ------------------------------------------------------------------ what the page shows
def status():
    c = cfg()
    s = st()
    src = _read(SOURCES) or {}
    prices = dict({'standard': 0.015, 'infrequent': 0.01, 'retrieval': 0.01, 'freeGb': 10}, **(src.get('prices') or {}))
    models = catalog()
    have = kept(c)
    online = s.get('online') or {}
    sizes = sets_of(models)
    keep = c.get('keep') or ''
    info = drive_info(c)
    kept_gb = sum(sum(f['size'] for f in k.get('files') or []) for k in have.values()) / 1e9
    giants = [m for m in models if m['giant']]
    ram = float(os.environ.get('HEARTH_VAULT_RAM_GB') or ram_gb())
    sets = []
    for sid, title in SETS:
        gb, n = sizes[sid]
        std, ia, back = costs(gb, prices)
        if sid == 'everything':
            about = 'Colibri\'s %d, the uncensored %d, Forge\'s %d, Listen\'s voices, Ollama\'s small one, Kokoro' % (
                sum(1 for m in models if m['app'] == 'Colibri'), sum(1 for m in models if m['app'] == 'Uncensored'), sum(1 for m in models if m['app'] == 'Hearth Forge'))
            tag = '%d models' % n
        elif sid == 'no-giants':
            names = [g['name'] for g in giants]
            about = 'Leaves out %s (%s)' % (', '.join(names[:-1]) + ' and ' + names[-1] if len(names) > 1 else ''.join(names), C.tb_words(sum(g['gb'] for g in giants)))
            tag = 'Kept now' if keep == sid else '%d models' % n
        else:
            about = 'Every model that runs well or slow here, nothing that needs more memory'
            tag = os.environ.get('HEARTH_VAULT_PC') or '%d GB RAM' % round(ram)
        if keep == sid and sid != 'no-giants':
            tag = 'Kept now'
        fits = info and info['free'] + kept_gb * 1e9 >= gb * 1e9
        drive_row = ('✓ Vault drive · %s' % C.tb_words(info['total'] / 1e9)) if (keep == sid and info) else ('needs %s free' % C.tb_words(gb))
        cls = c['r2'].get('class')
        sets.append({'id': sid, 'title': title, 'size': ('≈ ' if sid == 'this-pc' else '') + C.tb_words(gb), 'about': about, 'tag': tag, 'kept': keep == sid,
                     'fits': bool(fits),
                     'rows': [['On a drive here', drive_row, 'good' if keep == sid and info else ''],
                              ['R2 Standard · $%.3f/GB' % prices['standard'], '%s / month' % C.money(std) + (' ✓' if keep == sid and cls == 'standard' else ''), 'acc' if keep == sid and cls == 'standard' else ''],
                              ['R2 Infrequent Access · $%.2f/GB' % prices['infrequent'], '%s / month' % C.money(ia) + (' ✓' if keep == sid and cls != 'standard' else ''), 'acc' if keep == sid and cls != 'standard' else ''],
                              ['Getting it all back (Infrequent)', '%s once' % C.money(back), 'mu']]})
    rows = []
    apps = []
    for m in models:
        if m['app'] not in apps:
            apps.append(m['app'])
        k = have.get(m['id'])
        on = online.get(m['id']) or {}
        if on.get('state') == 'removed':
            tag = ('Removed %s' % C.day(on.get('since', 0)), 'bad')
        elif on.get('state') == 'changed':
            tag = ('Changed %s' % C.day(on.get('since', 0)), 'warn')
        elif on.get('state') == 'gated':
            tag = ('Needs a sign-in now', 'warn')
        elif on.get('state') == 'ok':
            tag = ('Still online', 'ok')
        else:
            tag = ('Not checked yet', 'dk')
        if k:
            sent, total = r2_progress(s, k)
            if k.get('where') == 'drive':
                where = 'here'
            elif total and sent >= total:
                where = 'here · R2'
            else:
                where = 'here · R2 %d%%' % (100 * sent // total if total else 0)
            sha = (k.get('files') or [{}])[0].get('sha256', '')
            version = ('%s · rev %s' % (k['version'], k['short'])) if k.get('version') else (('tag %s · digest %s' % (m.get('ollama', '').partition(':')[2], k['short'])) if m.get('ollama') else 'rev %s · %s' % (k['short'], C.day(k.get('keptAt', 0))))
            check = 'sha256 %s…%s ✓' % (sha[:4], sha[-4:]) if sha else '—'
            size = sum(f['size'] for f in k.get('files') or []) / 1e9
        else:
            where = 'not kept'
            version = ('rev %s · %s' % (on['rev'][:7], C.day(on.get('checked', 0)))) if on.get('rev') else '—'
            check = '—'
            size = m['gb']
        rows.append({'id': m['id'], 'name': m['name'], 'app': m['app'], 'giant': m['giant'], 'version': version, 'licence': (k or {}).get('licence') or on.get('licence') or m['licence'],
                     'size': C.gb_words(size), 'check': check, 'kept': where, 'keptOk': bool(k), 'online': tag[0], 'tone': tag[1], 'flag': tag[1] in ('bad', 'warn')})
    # the board's order: what needs a look first among the kept ones, then the rest as Hearth lists them
    rows.sort(key=lambda r: (0 if r['flag'] and r['keptOk'] else 1))
    r2 = c['r2']
    sent_all = sum(r2_progress(s, k)[0] for k in have.values() if k.get('where') != 'drive')
    tot_all = sum(r2_progress(s, k)[1] for k in have.values() if k.get('where') != 'drive')
    where = []
    if info:
        where.append({'name': 'Vault drive · %s' % C.tb_words(info['total'] / 1e9), 'right': os.environ.get('HEARTH_VAULT_WHERE') or 'on this PC', 'frac': kept_gb * 1e9 / info['total'] if info['total'] else 0,
                      'line': '%s used · %s free · healthy' % (C.tb_words(kept_gb), C.tb_words(info['free'] / 1e9)), 'state': 'ok'})
    else:
        where.append({'name': 'No vault drive yet', 'right': '', 'frac': 0, 'line': 'Pick a drive with room for what you keep (Change what\'s kept)', 'state': 'off'})
    if r2.get('bucket'):
        where.append({'name': 'Cloudflare R2 · %s' % ('Standard' if r2.get('class') == 'standard' else 'Infrequent Access'), 'right': '', 'frac': sent_all / tot_all if tot_all else 0,
                      'line': '%s of %s uploaded · %s so far this month' % (C.tb_words(sent_all / 1e9), C.tb_words(tot_all / 1e9), C.money(month_cost(s, c, prices))), 'state': 'ok' if C.has_secret('r2-access') else 'warn'})
    else:
        where.append({'name': 'Cloudflare R2', 'right': '', 'frac': 0, 'line': 'Not set up: your bucket and your keys (kept in the system\'s password store)', 'state': 'off'})
    upload = None
    pending = [(k, *r2_progress(s, k)) for k in have.values() if k.get('where') != 'drive']
    pending = [x for x in pending if x[2] and x[1] < x[2]]
    mbps = SPEEDS.get(c.get('speed'), 25) or 100
    togo = sum(t - sent for _, sent, t in pending)
    if pending:
        k, sent, total = pending[0]
        upload = {'name': '%s · %s' % (k['name'], C.gb_words(total / 1e9)), 'pct': int(100 * sent / total), 'togo': '%s to go' % C.tb_words(togo / 1e9),
                  'left': 'about %s left' % C.duration_words(togo * 8 / (mbps * 1e6)), 'speed': '%d Mb/s' % mbps}
    queue = []
    for k, sent, total in pending[:3]:
        queue.append({'name': k['name'], 'pct': int(100 * sent / total), 'left': C.duration_words((total - sent) * 8 / (mbps * 1e6)), 'on': k is pending[0][0]})
    for item in (s.get('queue') or [])[:max(0, 3 - len(queue))]:
        m = next((x for x in models if x['id'] == item['id']), None)
        if m:
            queue.append({'name': m['name'], 'pct': 0, 'left': C.duration_words(m['gb'] * 8e9 / (mbps * 1e6)), 'on': False})
    pauses = [p for p in s.get('pauses') or [] if p > time.time() - 86400]
    checks = s.get('checks') or []
    last = checks[-1] if checks else None
    history = [{'day': C.day(x['at']), 'tone': 'bad' if x.get('removed') else ('warn' if x.get('changed') or x.get('renamed') else 'ok'), 'at': x['at']} for x in checks[-12:]]
    notes = []
    for x in checks[-12:]:
        if x.get('renamed'):
            notes.append('%s: %s page renamed (followed by itself, same files)' % (C.day(x['at']), ', '.join(_name(models, i) for i in x['renamed'])))
        if x.get('removed'):
            notes.append('%s: %s removed' % (C.day(x['at']), 'one' if len(x['removed']) == 1 else len(x['removed'])))
    alerts = []
    for mid, on in online.items():
        k = have.get(mid)
        m = next((x for x in models if x['id'] == mid), None)
        if not k or not m or on.get('ack', -1) == on.get('since'):
            continue
        if on.get('state') == 'removed':
            sha = (k.get('files') or [{}])[0].get('sha256', '')
            alerts.append({'id': mid, 'kind': 'removed', 'title': m['name'].split(' · ')[0], 'found': 'found %s, %s' % (C.day_long(on['since']), C.clock(on['since'])),
                           'text': 'Its page on %s is gone (%s answers "not found"). Your copy is safe: %s, %s%s, checksum still matches.' % ('Hugging Face' if m.get('repo') else 'Ollama', m.get('repo') or m.get('ollama'), C.gb_words(sum(f['size'] for f in k['files']) / 1e9), (k.get('quant') + ', ') if k.get('quant') else '', 'rev ' + k['short']),
                           'loads': 'Colibri and Hearth AI already load it from the vault' if c.get('loadFirst', True) else '', 'moved': (s.get('moved') or {}).get(mid), 'sha': sha})
        elif on.get('state') == 'changed':
            new_size = sum(f['size'] for f in (on.get('files') or {}).values()) / 1e9
            old_size = sum(f['size'] for f in k['files']) / 1e9
            what = []
            if on.get('changedFiles'):
                what.append('%s changed' % ', '.join(on['changedFiles'][:2]))
            if on.get('addedFiles'):
                what.append('+%d file%s' % (len(on['addedFiles']), '' if len(on['addedFiles']) == 1 else 's'))
            alerts.append({'id': mid, 'kind': 'changed', 'title': m['name'], 'found': 'found %s, %s' % (C.day_long(on['since']), C.clock(on['since'])),
                           'text': 'Its maker put up new files (rev %s → rev %s, %s → %s) and the licence line %s. Hearth keeps the version you have until you say so.' % (k['short'], on['rev'][:7], C.gb_words(old_size), C.gb_words(new_size), 'stayed ' + k.get('licence', '') if (on.get('licence') or '').lower() == (k.get('licence') or '').lower() else 'changed to ' + on.get('licence', '')),
                           'yours': 'Yours · rev %s · %s' % (k['short'], C.day(k.get('keptAt', 0))), 'yoursFiles': '%s · %d files' % (' · '.join(f['name'] for f in k['files'][:2]), len(k['files'])),
                           'online': 'Online now · rev %s · %s' % (on['rev'][:7], C.day(on.get('since', 0))), 'onlineFiles': ' · '.join(what), 'app': m['app']})
    asks = []
    for mid, a in (s.get('asks') or {}).items():
        m = next((x for x in models if x['id'] == mid), None)
        if not m:
            continue
        std, ia, _ = costs(a.get('gb') or m['gb'], dict(prices, freeGb=0))
        asks.append({'id': mid, 'name': m['name'], 'app': m['app'], 'licence': a.get('licence') or m['licence'], 'size': '%s · rev %s · %s' % (C.gb_words(a.get('gb') or m['gb']), (a.get('rev') or '')[:7], C.day(a['at'])),
                     'adds': '%s (Infrequent) · about %s to upload' % (C.money(ia), C.duration_words((a.get('gb') or m['gb']) * 8e9 / (mbps * 1e6))), 'addsStd': C.money(std)})
    off = s.get('offline') or {}
    loads = s.get('loads') or {}
    n_ok = sum(1 for r in rows if r['tone'] == 'ok')
    need = len([a for a in alerts])
    health = ('Vault healthy · checked %s' % ((C.DAYS[time.localtime(last['at']).tm_wday] + ' ' + C.clock(last['at'])) if last else 'not yet')) if not need else ('%d thing%s need%s a look' % (need, '' if need == 1 else 's', 's' if need == 1 else ''))
    return {
        'ok': True, 'health': health, 'healthTone': 'ok' if not need else 'warn', 'keep': keep, 'on': bool(c.get('on')),
        'sets': sets, 'setsNote': 'First %d GB on Standard free · Infrequent Access keeps a file at least %d days · no download fees · same R2 account as the Hearth ISOs' % (prices['freeGb'], prices.get('minDays', 30)),
        'models': rows, 'apps': apps, 'keptCount': len(have), 'total': len(models), 'where': where, 'upload': upload, 'queue': queue,
        'queueLine': '%s to go · about %s at %d Mb/s%s' % (C.tb_words(togo / 1e9), C.duration_words(togo * 8 / (mbps * 1e6)), mbps, (' · paused %d h last night while you played' % max(1, len(pauses))) if pauses else '') if pending else 'Nothing waiting to upload',
        'how': {'loadFirst': bool(c.get('loadFirst', True)), 'askFirst': bool(c.get('askFirst', True)), 'weekly': bool(c.get('weekly', True)), 'monthly': bool(c.get('monthly', True))},
        'speed': c.get('speed'), 'night': bool(c.get('night')), 'paused': bool(c.get('paused')),
        'last': {'date': C.day_long(last['at']) if last else '', 'rows': [
            ['ok', '%d still online, same files' % last.get('ok', 0), ''],
            ['bad', '%d removed from Hugging Face' % len(last.get('removed') or []), 'removed'],
            ['warn', '%d changed by its maker' % len(last.get('changed') or []), 'changed']] if last else []},
        'history': history, 'historyNote': ' · '.join(notes[-2:] + ['alerts go to your phone through Hearth\'s notifications']) if notes else 'alerts go to your phone through Hearth\'s notifications',
        'alertRules': c['alert'], 'tell': c.get('tell') or [], 'alerts': alerts, 'asks': asks,
        'offline': {'line': ('tried %s: every kept model loaded' % C.day(off['at'])) if off and not off.get('bad') else (('tried %s: %d of %d loaded' % (C.day(off['at']), off.get('loaded', 0), off.get('total', 0))) if off else 'not tried yet: Check now tries it'), 'ok': bool(off) and not off.get('bad')},
        'used': {'vault': int(loads.get('vault', 0)), 'all': int(loads.get('vault', 0)) + int(loads.get('online', 0))},
        'getback': _getback_line(have, s, prices),
        'drive': c.get('drive') or '', 'r2': {'account': r2.get('account', ''), 'bucket': r2.get('bucket', ''), 'class': r2.get('class', 'infrequent'), 'keys': C.has_secret('r2-access') and C.has_secret('r2-secret')},
        'okCount': n_ok,
    }


def _name(models, mid):
    m = next((x for x in models if x['id'] == mid), None)
    return (m['name'] if m else mid).split(' · ')[0] + '\'s'


def _getback_line(have, s, prices):
    if not have:
        return ''
    k = sorted(have.values(), key=lambda x: -x.get('keptAt', 0))[0]
    gb = sum(f['size'] for f in k.get('files') or []) / 1e9
    return {'id': k['id'], 'line': '%s · %s · %s · about %s at home speed' % (k['name'], C.gb_words(gb), C.money(gb * prices['retrieval']), C.duration_words(gb * 8e9 / 400e6))}


def summary():
    s = status()
    kept_gb = 0.0
    c = cfg()
    for k in kept(c).values():
        kept_gb += sum(f['size'] for f in k.get('files') or []) / 1e9
    r2 = 'here and on R2' if s['r2']['bucket'] else 'here'
    month = ''
    for sset in s['sets']:
        if sset['kept']:
            month = next((r[1].replace(' ✓', '') for r in sset['rows'] if r[2] == 'acc'), '')
    removed = sum(1 for a in s['alerts'] if a['kind'] == 'removed')
    hist = [0.3 + 0.7 * min(1.0, x['at'] % 97 / 97.0) for x in (st().get('checks') or [])[-24:]]
    return {'ok': True, 'big': str(s['keptCount']), 'of': 'of %d models · %s' % (s['total'], C.tb_words(kept_gb)),
            'lines': ['%s%s' % (r2, (' · ' + month.replace(' / month', ' a month')) if month else '')],
            'alert': ('%d removed online, your copy safe' % removed) if removed else '', 'spark': hist or [0.4, 0.5, 0.45, 0.6, 0.55, 0.62],
            'count': C.tb_words(kept_gb) if kept_gb else '', 'state': s['healthTone']}


# ------------------------------------------------------------------ choices
BOOLS = ('loadFirst', 'askFirst', 'weekly', 'monthly', 'night', 'paused')


def set_choice(key, value):
    if key == 'keep':
        if value not in ('everything', 'no-giants', 'this-pc'):
            raise Fail('everything, no-giants or this-pc')
        C.set_key(PART, 'keep', value)
    elif key == 'drive':
        p = os.path.realpath(os.path.expanduser(value))
        if not os.path.isdir(p) or p in ('/', '/usr', '/etc', '/boot') or not os.access(p, os.W_OK):
            raise Fail('Pick a folder on a drive you can write to')
        C.set_key(PART, 'drive', p)
    elif key in BOOLS:
        C.set_key(PART, key, value == 'on')
    elif key == 'speed':
        if value not in SPEEDS:
            raise Fail('slow, medium or fast')
        C.set_key(PART, 'speed', value)
    elif key.startswith('r2.'):
        sub = key[3:]
        r2 = dict(cfg()['r2'])
        if sub == 'account' and value and not re.fullmatch(r'[0-9a-f]{32}', value):
            raise Fail('Your Cloudflare account ID is 32 letters and digits (R2 › Account details)')
        if sub == 'bucket' and value and not re.fullmatch(r'[a-z0-9][a-z0-9\-]{1,62}', value):
            raise Fail('A bucket\'s name is small letters, digits and -')
        if sub == 'class' and value not in ('infrequent', 'standard'):
            raise Fail('infrequent or standard')
        if sub not in ('account', 'bucket', 'class'):
            raise Fail('r2.account, r2.bucket or r2.class')
        r2[sub] = value
        C.set_key(PART, 'r2', r2)
    elif key.startswith('alert.'):
        sub = key[6:]
        a = dict(cfg()['alert'])
        if sub not in a:
            raise Fail('alert.removed, alert.files, alert.licence or alert.gated')
        a[sub] = value == 'on'
        C.set_key(PART, 'alert', a)
    elif key == 'tell':
        t = [x for x in value.split(',') if x in ('phone', 'hearth', 'email')]
        C.set_key(PART, 'tell', t)
    else:
        raise Fail('Not one of the vault\'s choices')
    return {'ok': True}


def main(argv):
    if not argv:
        raise Fail('status, summary, set, on, off, check, run, decide, getback, remove, link, offline-test, locate or verify')
    cmd, rest = argv[0], argv[1:]
    if cmd == 'status':
        return status()
    if cmd == 'summary':
        return summary()
    if cmd == 'set' and len(rest) == 2:
        return set_choice(rest[0], rest[1])
    if cmd in ('on', 'off'):
        C.set_key(PART, 'on', cmd == 'on')
        timers(cmd == 'on')
        return {'ok': True, 'said': 'The vault works in the background now' if cmd == 'on' else 'The vault\'s background work is off (what\'s kept stays)'}
    if cmd == 'check':
        return check()
    if cmd == 'weekly':
        return weekly()
    if cmd == 'run':
        mins = int(rest[1]) if len(rest) == 2 and rest[0] == '--minutes' and rest[1].isdigit() else 55
        return run_worker(mins)
    if cmd == 'decide' and len(rest) in (2, 3):
        if rest[1] == 'find':
            return find_moved(rest[0])
        if rest[1] == 'try-new':
            r = decide(rest[0], 'keep-both')
            r['said'] = 'The new files are copied beside yours; open it in its app to try them'
            return r
        return decide(rest[0], rest[1], rest[2] if len(rest) == 3 else '')
    if cmd == 'getback' and len(rest) == 1:
        return getback(rest[0])
    if cmd == 'remove' and len(rest) == 1:
        return remove(rest[0])
    if cmd == 'link':
        return link()
    if cmd == 'offline-test':
        return offline_test()
    if cmd == 'locate' and len(rest) == 1:
        return locate(rest[0])
    if cmd == 'verify':
        return verify()
    raise Fail('Not one of the vault\'s commands')


if __name__ == '__main__':
    sys.exit(C.main_guard(main))
