#!/usr/bin/env python3
"""hearth-forge: Hearth Forge's helper (docs/mockups/checklists/hearth-forge.md; hosts/hearth-ui/apps/forge).

Built into the app (include_str!, so a Hearth Store update of the app updates this too) and written by it to
~/.local/share/hearth/forge/hearth-forge.py, next to models.json, the list of every model.

On the Nvidia PC it is Hearth Forge's own user service (hearth-forge.service, as you, written by the app into
~/.config/systemd/user only when you set it up; no [Install], started by its timer only while it's on, never part of
starting the PC, the session or Game Mode; low priority; waits while you play). It listens on 127.0.0.1 and this PC's
Tailscale address only (port 8191), and it:
  - gets a model only when you press Get: each file from the model's own Hugging Face repo at its pinned revision,
    checked against the SHA-256 Hugging Face lists for it (git's own hash for the small files), kept in
    ~/Hearth/Forge/models (Hugging Face's own folder layout, so each model's code finds it with nothing downloaded);
  - installs a model's own code (its runner: a pinned git commit, checked) in its own Python environment, with
    PyTorch's CUDA 12.8 builds, only when you Get a model that needs it;
  - makes each model you ask for, one at a time, each in a program of its own that ends when it's done (so the
    graphics card's memory is given back), and keeps every one in ~/Hearth/Forge/<name> with its photo, words and
    settings.

On any device the app also runs it for Tripo3D online (`tripo`: your own key, read from the system's password store
by this program itself, never printed or written anywhere), for the small preview of a model (`preview`) and for the
exports (`export`, through Blender: .glb .obj .fbx .stl .usdz, fewer faces, smaller textures, solid / flat bottom /
hollow for printing).

  hearth-forge.py on | off | status            the service (on: written, its timer enabled, started)
  hearth-forge.py serve                         the service itself
  hearth-forge.py run JOBDIR                    one model being made (inside its runner's Python)
  hearth-forge.py preview MADEDIR               preview.json and stats.json from model.glb
  hearth-forge.py export MADEDIR FORMAT OUT [--faces N] [--texture PX] [--solid] [--flat] [--hollow MM] [--height M]
  hearth-forge.py tripo balance | make JOBDIR
"""
import base64
import fnmatch
import hashlib
import http.server
import json
import os
import re
import shutil
import signal
import socket
import socketserver
import struct
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
import zlib

HERE = os.path.dirname(os.path.realpath(__file__))
PORT = int(os.environ.get('HEARTH_FORGE_PORT', '8191'))
HOME = os.path.expanduser('~')
FORGE = os.environ.get('HEARTH_FORGE_DIR') or os.path.join(HOME, 'Hearth', 'Forge')
MODELS = os.path.join(FORGE, 'models')
HUB = os.path.join(MODELS, 'hub')
LOCK = os.path.join(MODELS, 'forge-lock.json')
DATA = os.environ.get('HEARTH_FORGE_DATA') or os.path.join(os.environ.get('XDG_DATA_HOME') or os.path.join(HOME, '.local', 'share'), 'hearth', 'forge')
STATE = os.path.join(os.environ.get('XDG_STATE_HOME') or os.path.join(HOME, '.local', 'state'), 'hearth', 'forge.json')
RUNNERS = os.path.join(DATA, 'runners')
HF = os.environ.get('HEARTH_FORGE_HF', 'https://huggingface.co')
TORCH_INDEX = 'https://download.pytorch.org/whl/cu128'
COMFY = os.environ.get('HEARTH_FORGE_COMFY', 'http://127.0.0.1:8188')
TRIPO = os.environ.get('HEARTH_FORGE_TRIPO', 'https://api.tripo3d.ai/v2/openapi')
UNIT = 'hearth-forge.service'
TIMER = 'hearth-forge.timer'
NAME_RE = re.compile(r'^[a-z0-9][a-z0-9.-]{0,63}$')
REPO_RE = re.compile(r'^[A-Za-z0-9][A-Za-z0-9_.-]{0,95}/[A-Za-z0-9][A-Za-z0-9_.-]{0,95}$')
COMMIT_RE = re.compile(r'^[0-9a-f]{40}$')
TS_IP_RE = re.compile(r'^100\.\d{1,3}\.\d{1,3}\.\d{1,3}$')


class Fail(Exception):
    pass


# ------------------------------------------------------------------ small things

def catalog():
    p = os.environ.get('HEARTH_FORGE_MODELS') or os.path.join(HERE, 'models.json')
    with open(p, encoding='utf-8') as f:
        return json.load(f)


def model_of(mid, cat=None):
    for m in (cat or catalog())['models']:
        if m['id'] == mid:
            return m
    raise Fail('Hearth Forge doesn\'t know a model called %s' % mid)


def read_json(p, default):
    try:
        with open(p, encoding='utf-8') as f:
            v = json.load(f)
        return v if isinstance(v, type(default)) else default
    except (OSError, ValueError):
        return default


def write_json(p, v):
    """Written whole and atomically (a temporary file, fsynced, renamed); the old one kept as .bak."""
    d = os.path.dirname(p)
    os.makedirs(d, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=d, prefix='.' + os.path.basename(p) + '.')
    try:
        with os.fdopen(fd, 'w', encoding='utf-8') as f:
            json.dump(v, f, indent=1, ensure_ascii=False)
            f.write('\n')
            f.flush()
            os.fsync(f.fileno())
        if os.path.exists(p):
            try:
                shutil.copy2(p, p + '.bak')
            except OSError:
                pass
        os.replace(tmp, p)
    except BaseException:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


_state_lock = threading.Lock()


def state():
    return read_json(STATE, {})


def state_set(fn):
    """Install and download progress, the queue: state, never a setting (merged under one lock)."""
    with _state_lock:
        d = state()
        fn(d)
        write_json(STATE, d)
        return d


def cmd(args, timeout=60, cwd=None, env=None):
    """(exit code, output) of a program; never a shell."""
    try:
        p = subprocess.run(args, capture_output=True, text=True, timeout=timeout, cwd=cwd, env=env, stdin=subprocess.DEVNULL)
        return p.returncode, (p.stdout or '') + (p.stderr or '')
    except (OSError, subprocess.SubprocessError) as e:
        return 127, str(e)


def secret(key):
    """A key from the system's password store (secret-tool, the app's own entry), or ''. Never printed or kept."""
    if not shutil.which('secret-tool'):
        return ''
    code, out = cmd(['secret-tool', 'lookup', 'hearth-app', 'forge', 'key', key], 20)
    return out.strip() if code == 0 else ''


def game_running():
    """Game Mode or a game is running: gamescope, or Hearth's hearth-playing file (as the home server's)."""
    try:
        for pid in os.listdir('/proc'):
            if pid.isdigit():
                try:
                    with open('/proc/%s/comm' % pid) as f:
                        if f.read().strip() in ('gamescope', 'gamescope-wl'):
                            return True
                except OSError:
                    pass
    except OSError:
        pass
    rt = os.environ.get('XDG_RUNTIME_DIR') or '/run/user/%d' % os.getuid()
    return os.path.exists(os.path.join(rt, 'hearth-playing'))


def slug(s):
    s = re.sub(r'[^a-z0-9]+', '-', (s or '').lower()).strip('-')
    return (s[:40].strip('-') or 'model')


def free_gb(path):
    try:
        st = os.statvfs(path if os.path.exists(path) else HOME)
        return st.f_bavail * st.f_frsize / 1e9
    except OSError:
        return 0.0


def folder_bytes(p):
    total = 0
    for root, _dirs, files in os.walk(p):
        for f in files:
            try:
                total += os.lstat(os.path.join(root, f)).st_size
            except OSError:
                pass
    return total


# ------------------------------------------------------------------ getting a model (only when you press Get)

def hf_get(url, token='', timeout=60):
    req = urllib.request.Request(url, headers={'User-Agent': 'Hearth-Forge'})
    if token:
        req.add_header('Authorization', 'Bearer ' + token)
    return urllib.request.urlopen(req, timeout=timeout)


def hf_files(repo, rev, token=''):
    """The repo's files at this revision with their hashes: (commit, [{path, size, sha256 | blob}])."""
    if not REPO_RE.fullmatch(repo):
        raise Fail('not a Hugging Face repo: %s' % repo)
    url = '%s/api/models/%s/revision/%s?blobs=true' % (HF, repo, urllib.parse.quote(rev or 'main', safe=''))
    try:
        with hf_get(url, token) as r:
            info = json.load(r)
    except urllib.error.HTTPError as e:
        if e.code in (401, 403):
            raise Fail('%s asks you to accept its licence on huggingface.co first (and your Hugging Face key in Hearth Forge › Settings)' % repo)
        if e.code == 404:
            raise Fail('%s isn\'t on Hugging Face (or isn\'t open to you)' % repo)
        raise Fail('Hugging Face said %s for %s' % (e.code, repo))
    except (urllib.error.URLError, OSError, ValueError) as e:
        raise Fail('Hugging Face can\'t be reached: %s' % e)
    commit = info.get('sha') or ''
    if not COMMIT_RE.fullmatch(commit):
        raise Fail('Hugging Face didn\'t say which commit %s is' % repo)
    if rev and COMMIT_RE.fullmatch(rev) and commit != rev:
        raise Fail('%s at %s isn\'t the revision Hearth pinned' % (repo, rev))
    out = []
    for s in info.get('siblings') or []:
        path = s.get('rfilename') or ''
        if not path or path.startswith('/') or '..' in path.split('/'):
            continue
        lfs = s.get('lfs') or {}
        out.append({'path': path, 'size': int(lfs.get('size') or s.get('size') or 0),
                    'sha256': (lfs.get('sha256') or '').lower(), 'blob': (s.get('blobId') or '').lower()})
    return commit, out


def wanted(files, include):
    return [f for f in files if any(fnmatch.fnmatchcase(f['path'], pat) for pat in include)]


def snap_dir(repo, commit):
    return os.path.join(HUB, 'models--' + repo.replace('/', '--'), 'snapshots', commit)


def file_ok(path, f):
    """The file is there and is what Hugging Face says (its SHA-256, or git's blob hash for a small file)."""
    try:
        if os.path.getsize(path) != f['size'] and f['size']:
            return False
    except OSError:
        return False
    if f['sha256']:
        h = hashlib.sha256()
    elif f['blob']:
        h = hashlib.sha1(b'blob %d\0' % os.path.getsize(path))
    else:
        return False
    with open(path, 'rb') as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest() == (f['sha256'] or f['blob'])


def download(repo, commit, f, dest, token, progress):
    """One file, to a .part beside it, its hash worked out as it comes; kept only when it matches."""
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    part = dest + '.part'
    url = '%s/%s/resolve/%s/%s' % (HF, repo, commit, urllib.parse.quote(f['path']))
    h = hashlib.sha256() if f['sha256'] else hashlib.sha1(b'blob %d\0' % f['size'])
    got = 0
    with hf_get(url, token, timeout=120) as r, open(part, 'wb') as out:
        for chunk in iter(lambda: r.read(1 << 20), b''):
            out.write(chunk)
            h.update(chunk)
            got += len(chunk)
            progress(len(chunk))
    if h.hexdigest() != (f['sha256'] or f['blob']):
        os.unlink(part)
        raise Fail('%s from %s didn\'t match its SHA-256: not kept' % (f['path'], repo))
    os.replace(part, dest)


def lock():
    return read_json(LOCK, {})


def installed(m, lk=None):
    lk = lk if lk is not None else lock()
    e = lk.get(m['id']) or {}
    if not e.get('done') or not m.get('files'):
        return False
    return all(os.path.isdir(snap_dir(s['repo'], s['commit'])) for s in e.get('sources', []))


def get_model(mid, cat=None):
    """Get: every file the model needs, checked, then its code (runner). Runs in the service's own thread."""
    cat = cat or catalog()
    m = model_of(mid, cat)
    if m.get('noSource') or not m.get('files'):
        raise Fail(m.get('noSource') or 'Hearth doesn\'t know where its files are')
    token = secret('huggingface')
    lk = lock()
    prev = (lk.get(mid) or {}).get('sources') or []
    plan = []
    for i, src in enumerate(m['files']):
        rev = src.get('revision') or next((p['commit'] for p in prev if p.get('repo') == src['repo']), '')
        commit, files = hf_files(src['repo'], rev, token)
        need = wanted(files, src['include'])
        if not need:
            raise Fail('%s has none of the files Hearth expects' % src['repo'])
        plan.append((src['repo'], commit, need))
    total = sum(f['size'] for _, _, need in plan for f in need)
    if free_gb(MODELS) * 1e9 < total * 1.05 + 2e9:
        raise Fail('Not enough free space: it needs %.1f GB' % (total / 1e9))
    done = [0]
    last = [0.0]

    def progress(n):
        done[0] += n
        if time.time() - last[0] > 1:
            last[0] = time.time()
            set_job(mid, progress=round(100.0 * done[0] / max(total, 1), 1), step='downloading %.1f of %.1f GB' % (done[0] / 1e9, total / 1e9))

    for repo, commit, need in plan:
        d = snap_dir(repo, commit)
        for f in need:
            dest = os.path.join(d, *f['path'].split('/'))
            if file_ok(dest, f):
                progress(f['size'])
                continue
            while game_running():
                set_job(mid, step='waiting while you play')
                time.sleep(20)
            download(repo, commit, f, dest, token, progress)
        refs = os.path.join(HUB, 'models--' + repo.replace('/', '--'), 'refs')
        os.makedirs(refs, exist_ok=True)
        with open(os.path.join(refs, 'main'), 'w') as fh:
            fh.write(commit)
    runner = m.get('runner')
    if runner:
        set_job(mid, step='getting its own code', progress=100)
        install_runner(runner, cat, lambda s: set_job(mid, step=s))

    def keep(d):
        d[mid] = {'done': True, 'at': int(time.time()), 'bytes': total,
                  'sources': [{'repo': r, 'commit': c, 'files': len(n)} for r, c, n in plan]}
    with _state_lock:
        lk = lock()
        keep(lk)
        write_json(LOCK, lk)


def remove_model(mid):
    """The model's own files go (a file another installed model still uses stays). Made models always stay."""
    lk = lock()
    e = lk.pop(mid, None)
    if not e:
        raise Fail('It isn\'t installed')
    used = {(s['repo'], s['commit']) for k, v in lk.items() for s in (v.get('sources') or [])}
    for s in e.get('sources', []):
        if (s['repo'], s['commit']) in used:
            continue
        d = snap_dir(s['repo'], s['commit'])
        if os.path.isdir(d) and not os.path.islink(d) and os.path.realpath(d).startswith(os.path.realpath(HUB) + os.sep):
            shutil.rmtree(d)
    write_json(LOCK, lk)


# ------------------------------------------------------------------ a model's own code (its runner)

def runner_dir(rid):
    if not NAME_RE.fullmatch(rid):
        raise Fail('bad runner name')
    return os.path.join(RUNNERS, rid)


def runner_python(rid):
    return os.path.join(runner_dir(rid), 'venv', 'bin', 'python')


def runner_ready(rid):
    return os.path.isfile(os.path.join(runner_dir(rid), 'ready.json'))


def install_runner(rid, cat, step):
    """Its pinned commit (checked) into its own folder, a Python environment, PyTorch for CUDA 12.8, what it needs,
    its own parts built. Anything wrong: you're told why, and nothing that was working is touched."""
    r = cat['runners'][rid]
    d = runner_dir(rid)
    if runner_ready(rid) and read_json(os.path.join(d, 'ready.json'), {}).get('commit') == r['commit']:
        return
    if not shutil.which('git'):
        raise Fail('git isn\'t on this PC')
    os.makedirs(d, exist_ok=True)
    src = os.path.join(d, 'src')
    if not os.path.isdir(os.path.join(src, '.git')):
        step('getting %s' % r['name'])
        tmp = src + '.new'
        shutil.rmtree(tmp, ignore_errors=True)
        code, out = cmd(['git', 'init', '-q', tmp], 60)
        code, out = cmd(['git', '-C', tmp, 'fetch', '-q', '--depth', '1', r['git'], r['commit']], 1800)
        if code != 0:
            shutil.rmtree(tmp, ignore_errors=True)
            raise Fail('Couldn\'t get %s: %s' % (r['name'], out.strip()[-200:]))
        cmd(['git', '-C', tmp, 'checkout', '-q', '--detach', 'FETCH_HEAD'], 300)
        code, head = cmd(['git', '-C', tmp, 'rev-parse', 'HEAD'], 20)
        if head.strip() != r['commit']:
            shutil.rmtree(tmp, ignore_errors=True)
            raise Fail('%s isn\'t the commit Hearth pinned: not installed' % r['name'])
        os.rename(tmp, src)
    py = runner_python(rid)
    if not os.path.isfile(py):
        step('making its Python environment')
        code, out = cmd(['/usr/bin/python3', '-m', 'venv', os.path.join(d, 'venv')], 600)
        if code != 0:
            raise Fail('Couldn\'t make its Python environment (python3-venv): ' + out.strip()[-200:])

    def pip(args, timeout):
        return cmd([py, '-m', 'pip', '--disable-pip-version-check', '--no-input', '--no-cache-dir', '--progress-bar', 'off'] + args, timeout, cwd=src)
    if pip(['show', 'torch'], 120)[0] != 0:
        step('downloading PyTorch for CUDA 12.8 (a few GB)')
        code, out = pip(['install', '--index-url', TORCH_INDEX, 'torch', 'torchvision'], 3 * 3600)
        if code != 0:
            raise Fail('PyTorch didn\'t install: ' + out.strip()[-200:])
    step('installing what %s needs' % r['name'])
    code, out = pip(['install'] + r['pip'], 3600)
    if code != 0:
        raise Fail('What it needs didn\'t install: ' + out.strip()[-200:])
    if os.path.isfile(os.path.join(src, 'setup.py')) or os.path.isfile(os.path.join(src, 'pyproject.toml')):
        pip(['install', '--no-deps', '-e', '.'], 1800)
    built = []
    for part in r.get('build') or []:
        step('building %s' % part)
        code, out = pip(['install', '--no-build-isolation', './' + part], 3600)
        if code == 0:
            built.append(part)
    code, out = cmd([py, '-c', 'import json, torch; c = torch.cuda.is_available(); print(json.dumps({"torch": torch.__version__, "cuda": c, "gpu": torch.cuda.get_device_name(0) if c else ""}))'], 300)
    info = {}
    try:
        info = json.loads(out.strip().splitlines()[-1]) if code == 0 else {}
    except (ValueError, IndexError):
        pass
    # rembg's own cut-out model (it checks its MD5 itself), now rather than at the first make
    env = dict(os.environ, U2NET_HOME=os.path.join(MODELS, 'u2net'))
    cmd([py, '-c', 'import rembg; rembg.new_session()'], 900, env=env)
    write_json(os.path.join(d, 'ready.json'), {'commit': r['commit'], 'built': built, 'missing': [b for b in r.get('build') or [] if b not in built],
                                                'torch': str(info.get('torch', ''))[:40], 'cuda': info.get('cuda') is True, 'at': int(time.time())})


# ------------------------------------------------------------------ the queue and the one being made

_jobs_lock = threading.Lock()
JOBS = {}      # model id -> {step, progress, error} (getting)
WAKE = threading.Event()


def set_job(mid, **kw):
    with _jobs_lock:
        JOBS.setdefault(mid, {}).update(kw)


def queue():
    return state().get('queue') or []


def made_dirs():
    out = []
    try:
        names = os.listdir(FORGE)
    except OSError:
        return out
    for n in names:
        d = os.path.join(FORGE, n)
        if n in ('models', 'exports') or n.startswith('.') or not os.path.isfile(os.path.join(d, 'ask.json')):
            continue
        out.append(d)
    return out


def made_item(d):
    ask = read_json(os.path.join(d, 'ask.json'), {})
    st = read_json(os.path.join(d, 'stats.json'), {})
    done = read_json(os.path.join(d, 'done.json'), {})
    files = sorted(f for f in os.listdir(d) if not f.startswith('.')) if os.path.isdir(d) else []
    return {'id': os.path.basename(d), 'title': ask.get('title') or os.path.basename(d), 'ask': ask, 'stats': st, 'done': done,
            'files': files, 'at': done.get('at') or ask.get('at') or 0}


def add_job(ask, photos):
    """A new one: its folder in ~/Hearth/Forge with the photos, the words and the settings, then into the queue."""
    title = (ask.get('title') or ask.get('words') or 'A model').strip()[:80]
    base = slug(title)
    name = base
    n = 2
    while os.path.exists(os.path.join(FORGE, name)):
        name = '%s-%d' % (base, n)
        n += 1
    d = os.path.join(FORGE, name)
    os.makedirs(d)
    names = []
    for i, b in enumerate(photos[:6]):
        fn = 'photo.png' if i == 0 else 'photo-%d.png' % (i + 1)
        with open(os.path.join(d, fn), 'wb') as f:
            f.write(b)
        names.append(fn)
    ask = dict(ask, title=title, photos=names, at=int(time.time()))
    write_json(os.path.join(d, 'ask.json'), ask)
    state_set(lambda s: s.setdefault('queue', []).append(name))
    WAKE.set()
    return name


def worker():
    """One at a time, in a program of its own at low priority; waits while you play (it would take the graphics card)."""
    while True:
        q = queue()
        if not q:
            WAKE.wait(30)
            WAKE.clear()
            continue
        name = q[0]
        d = os.path.join(FORGE, name)
        if not os.path.isdir(d):
            state_set(lambda s: s.__setitem__('queue', [x for x in s.get('queue', []) if x != name]))
            continue
        while game_running():
            write_json(os.path.join(d, 'progress.json'), {'stage': 'waiting while you play', 'progress': 0})
            time.sleep(20)
        run_one(d)
        state_set(lambda s: s.__setitem__('queue', [x for x in s.get('queue', []) if x != name]))


def run_one(d):
    ask = read_json(os.path.join(d, 'ask.json'), {})
    t0 = time.time()
    try:
        cat = catalog()
        m = model_of(ask.get('model') or '', cat)
        if not m.get('runner'):
            raise Fail('Hearth Forge can\'t make with %s yet: its own code isn\'t in Hearth Forge (it can be got and kept)' % m['name'])
        if not installed(m):
            raise Fail('Get %s first (Models)' % m['name'])
        if ask.get('mode') in ('words', 'picture') and not os.path.isfile(os.path.join(d, 'photo.png')):
            write_json(os.path.join(d, 'progress.json'), {'stage': 'making a picture from your words', 'progress': 2})
            picture_from_words(ask, d)
        elif ask.get('change') and os.path.isfile(os.path.join(d, 'photo.png')):
            write_json(os.path.join(d, 'progress.json'), {'stage': 'changing the picture with your words', 'progress': 2})
            picture_from_words(ask, d, start=os.path.join(d, 'photo.png'))
        env = dict(os.environ, HF_HUB_CACHE=HUB, HF_HUB_OFFLINE='1', TRANSFORMERS_OFFLINE='1', HY3DGEN_MODELS=os.path.join(MODELS, '.none'),
                   U2NET_HOME=os.path.join(MODELS, 'u2net'), PYTHONUNBUFFERED='1')
        py = runner_python(m['runner'])
        src = os.path.join(runner_dir(m['runner']), 'src')
        env['PYTHONPATH'] = src
        p = subprocess.Popen(['nice', '-n', '10', py, os.path.realpath(__file__), 'run', d], cwd=src, env=env,
                             stdin=subprocess.DEVNULL, stdout=open(os.path.join(d, 'log.txt'), 'w'), stderr=subprocess.STDOUT)
        while p.poll() is None:
            time.sleep(1)
            if os.path.exists(os.path.join(d, 'stop')):
                p.terminate()
        if p.returncode != 0 or not os.path.isfile(os.path.join(d, 'model.glb')):
            tail = ''
            try:
                with open(os.path.join(d, 'log.txt'), errors='replace') as f:
                    tail = f.read()[-400:]
            except OSError:
                pass
            raise Fail('It stopped: ' + (tail.strip().splitlines() or ['no reason given'])[-1][:300])
        make_preview(d)
        timing = read_json(os.path.join(d, 'progress.json'), {}).get('timing') or {}
        write_json(os.path.join(d, 'done.json'), {'ok': True, 'at': int(time.time()), 'secs': int(time.time() - t0), 'timing': timing, 'model': m['name']})
    except Exception as e:  # noqa: BLE001 - every reason is told to the person
        write_json(os.path.join(d, 'done.json'), {'ok': False, 'at': int(time.time()), 'error': str(e)[:400]})
    try:
        os.unlink(os.path.join(d, 'progress.json'))
    except OSError:
        pass


def picture_from_words(ask, d, start=None):
    """Words → picture with Hearth Imagine's ComfyUI on this PC (its own API): one object on a plain background,
    or (Change it with words…) your photo changed by the words."""
    def call(method, path, body=None):
        req = urllib.request.Request(COMFY + path, data=json.dumps(body).encode() if body is not None else None, method=method,
                                     headers={'Content-Type': 'application/json'})
        with urllib.request.urlopen(req, timeout=30) as r:
            return json.load(r)
    try:
        info = call('GET', '/object_info/CheckpointLoaderSimple')
        ckpts = info['CheckpointLoaderSimple']['input']['required']['ckpt_name'][0]
    except Exception:
        raise Fail('Words need Hearth Imagine\'s ComfyUI on this PC (turn it on in Hearth Imagine)')
    if not ckpts:
        raise Fail('ComfyUI has no picture model yet')
    words = '%s, %s, a single object, centred, whole, plain light grey background, product photo, sharp, real materials' % (ask.get('words', ''), ask.get('change', ''))
    g = {'1': {'class_type': 'CheckpointLoaderSimple', 'inputs': {'ckpt_name': ckpts[0]}},
         '2': {'class_type': 'CLIPTextEncode', 'inputs': {'text': words.strip(', '), 'clip': ['1', 1]}},
         '3': {'class_type': 'CLIPTextEncode', 'inputs': {'text': 'cropped, several objects, text, blurry, people', 'clip': ['1', 1]}},
         '5': {'class_type': 'KSampler', 'inputs': {'seed': int(ask.get('seed') or 1), 'steps': 28, 'cfg': 6.5, 'sampler_name': 'dpmpp_2m', 'scheduler': 'karras',
                                                    'denoise': 1.0, 'model': ['1', 0], 'positive': ['2', 0], 'negative': ['3', 0], 'latent_image': ['4', 0]}},
         '6': {'class_type': 'VAEDecode', 'inputs': {'samples': ['5', 0], 'vae': ['1', 2]}},
         '7': {'class_type': 'SaveImage', 'inputs': {'images': ['6', 0], 'filename_prefix': 'Forge/picture'}}}
    if start:
        with open(start, 'rb') as f:
            img = f.read()
        b = '----hearthforge%d' % int(time.time())
        body = ('--%s\r\nContent-Disposition: form-data; name="image"; filename="forge-%s.png"\r\nContent-Type: image/png\r\n\r\n' % (b, os.path.basename(d))).encode() + img + ('\r\n--%s--\r\n' % b).encode()
        req = urllib.request.Request(COMFY + '/upload/image', data=body, method='POST', headers={'Content-Type': 'multipart/form-data; boundary=' + b})
        with urllib.request.urlopen(req, timeout=60) as r:
            up = json.load(r)['name']
        g['8'] = {'class_type': 'LoadImage', 'inputs': {'image': up}}
        g['4'] = {'class_type': 'VAEEncode', 'inputs': {'pixels': ['8', 0], 'vae': ['1', 2]}}
        g['5']['inputs']['denoise'] = 0.55
    else:
        g['4'] = {'class_type': 'EmptyLatentImage', 'inputs': {'width': 1024, 'height': 1024, 'batch_size': 1}}
    pid = call('POST', '/prompt', {'prompt': g, 'client_id': 'hearth-forge'})['prompt_id']
    for _ in range(900):
        time.sleep(2)
        h = call('GET', '/history/' + pid).get(pid)
        if h and h.get('outputs'):
            im = next(iter(o['images'][0] for o in h['outputs'].values() if o.get('images')))
            q = urllib.parse.urlencode({'filename': im['filename'], 'subfolder': im.get('subfolder', ''), 'type': im.get('type', 'output')})
            with urllib.request.urlopen(COMFY + '/view?' + q, timeout=60) as r:
                data = r.read()
            if start:
                os.replace(start, os.path.join(d, 'photo-before.png'))
            with open(os.path.join(d, 'photo.png'), 'wb') as f:
                f.write(data)
            ask2 = read_json(os.path.join(d, 'ask.json'), {})
            ask2['photos'] = ['photo.png']
            write_json(os.path.join(d, 'ask.json'), ask2)
            return
    raise Fail('ComfyUI didn\'t make the picture')


# ------------------------------------------------------------------ inside a runner's own Python: one model made

DETAIL = {'draft': (192, 20), 'good': (256, 30), 'high': (320, 40), 'highest': (384, 50)}
TEXSIZE = {'1k': 1024, '2k': 2048, '4k': 4096}


def job_run(d):
    ask = read_json(os.path.join(d, 'ask.json'), {})
    cat = catalog()
    m = model_of(ask['model'], cat)
    timing = {}

    def stage(s, pct):
        write_json(os.path.join(d, 'progress.json'), {'stage': s, 'progress': pct, 'timing': timing})
    import torch
    from PIL import Image
    seed = int(ask.get('seed') or 1)
    torch.manual_seed(seed)
    vram = torch.cuda.get_device_properties(0).total_memory / 1e9 if torch.cuda.is_available() else 0
    photo = Image.open(os.path.join(d, 'photo.png'))
    res, steps = DETAIL.get(ask.get('detail', 'high'), DETAIL['high'])
    tex = ask.get('textures', 'pbr')
    tsize = TEXSIZE.get(str(ask.get('textureSize', '2k')).lower(), 2048)
    out = os.path.join(d, 'model.glb')
    t = time.time()
    if m['runner'] == 'hunyuan3d2':
        from hy3dgen.rembg import BackgroundRemover
        from hy3dgen.shapegen import Hunyuan3DDiTFlowMatchingPipeline, FloaterRemover, DegenerateFaceRemover
        run = m.get('run') or {}
        stage('taking out the background', 4)
        img = photo.convert('RGBA')
        img = BackgroundRemover()(img.convert('RGB')) if img.getextrema()[3][0] == 255 else img
        img.save(os.path.join(d, 'photo-cut.png'))
        stage('making the shape', 10)
        shape = Hunyuan3DDiTFlowMatchingPipeline.from_pretrained(run.get('shapeRepo', 'tencent/Hunyuan3D-2'), subfolder=run['shape'], variant='fp16')
        if run.get('flashvdm'):
            shape.enable_flashvdm()
        g = torch.Generator().manual_seed(seed)
        mesh = shape(image=img, num_inference_steps=int(run.get('steps') or steps), octree_resolution=res, guidance_scale=5.0, generator=g, output_type='trimesh')[0]
        mesh = DegenerateFaceRemover()(FloaterRemover()(mesh))
        del shape
        torch.cuda.empty_cache()
        timing['shape'] = int(time.time() - t)
        if tex != 'none':
            t = time.time()
            stage('painting its textures', 55)
            try:
                from hy3dgen.texgen import Hunyuan3DPaintPipeline
                paint = Hunyuan3DPaintPipeline.from_pretrained('tencent/Hunyuan3D-2', subfolder=run.get('paint', 'hunyuan3d-paint-v2-0'))
                if tsize != paint.config.texture_size:
                    from hy3dgen.texgen.differentiable_renderer.mesh_render import MeshRender
                    paint.config.texture_size = tsize
                    paint.render = MeshRender(default_resolution=paint.config.render_size, texture_size=tsize)
                if vram < 20:
                    paint.enable_model_cpu_offload()
                mesh = paint(mesh, image=img)
                timing['textures'] = int(time.time() - t)
            except Exception as e:  # noqa: BLE001 - the shape is kept, and why it has no textures is said
                timing['texturesFailed'] = ('Its textures couldn\'t be painted (%s): the shape is kept' % e)[:300]
        mesh.export(out)
    elif m['runner'] == 'triposr':
        import numpy as np
        import rembg
        from tsr.system import TSR
        from tsr.utils import remove_background, resize_foreground
        stage('taking out the background', 5)
        img = resize_foreground(remove_background(photo, rembg.new_session()), 0.85)
        img.save(os.path.join(d, 'photo-cut.png'))
        a = np.array(img).astype(np.float32) / 255.0
        a = a[:, :, :3] * a[:, :, 3:4] + (1 - a[:, :, 3:4]) * 0.5
        img = Image.fromarray((a * 255.0).astype(np.uint8))
        stage('making the shape', 20)
        dev = 'cuda:0' if torch.cuda.is_available() else 'cpu'
        mdl = TSR.from_pretrained(snapshot('stabilityai/TripoSR'), config_name='config.yaml', weight_name='model.ckpt')
        mdl.renderer.set_chunk_size(8192)
        mdl.to(dev)
        with torch.no_grad():
            codes = mdl([img], device=dev)
        mesh = mdl.extract_mesh(codes, True, resolution=res)[0]
        timing['shape'] = int(time.time() - t)
        mesh.export(out)
    elif m['runner'] == 'sf3d':
        import rembg
        from sf3d.system import SF3D
        from sf3d.utils import remove_background, resize_foreground
        stage('taking out the background', 5)
        img = resize_foreground(remove_background(photo.convert('RGBA'), rembg.new_session()), 0.85)
        img.save(os.path.join(d, 'photo-cut.png'))
        stage('making the shape and its textures', 20)
        dev = 'cuda' if torch.cuda.is_available() else 'cpu'
        mdl = SF3D.from_pretrained(snapshot('stabilityai/stable-fast-3d'), config_name='config.yaml', weight_name='model.safetensors')
        mdl.to(dev)
        mdl.eval()
        with torch.no_grad():
            with torch.autocast(device_type=dev, dtype=torch.bfloat16) if dev == 'cuda' else _nothing():
                mesh, _ = mdl.run_image(img, bake_resolution=min(tsize, 2048), remesh='none', vertex_count=-1)
        timing['shape'] = int(time.time() - t)
        mesh.export(out, include_normals=True)
    else:
        raise Fail('no runner for %s' % m['id'])
    stage('done', 100)


class _nothing:
    def __enter__(self):
        return self

    def __exit__(self, *a):
        return False


def snapshot(repo):
    """The folder Hearth got this repo into (the commit refs/main names)."""
    base = os.path.join(HUB, 'models--' + repo.replace('/', '--'))
    try:
        with open(os.path.join(base, 'refs', 'main')) as f:
            return os.path.join(base, 'snapshots', f.read().strip())
    except OSError:
        raise Fail('%s isn\'t got yet' % repo)


# ------------------------------------------------------------------ a .glb read with Python alone: the preview

def glb_read(path):
    """(json, bin) of a binary glTF."""
    with open(path, 'rb') as f:
        data = f.read()
    if data[:4] != b'glTF':
        raise Fail('not a .glb')
    js, bn = None, b''
    off = 12
    while off + 8 <= len(data):
        ln, kind = struct.unpack_from('<II', data, off)
        chunk = data[off + 8: off + 8 + ln]
        if kind == 0x4E4F534A:
            js = json.loads(chunk.decode('utf-8'))
        elif kind == 0x004E4942:
            bn = chunk
        off += 8 + ln
    if js is None:
        raise Fail('a .glb without its JSON')
    return js, bn


CTYPES = {5120: 'b', 5121: 'B', 5122: 'h', 5123: 'H', 5125: 'I', 5126: 'f'}
NCOMP = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4}


def accessor(js, bn, i):
    a = js['accessors'][i]
    bv = js['bufferViews'][a['bufferView']]
    n, c = a['count'], NCOMP[a['type']]
    fmt = CTYPES[a['componentType']]
    size = struct.calcsize(fmt)
    stride = bv.get('byteStride') or size * c
    base = bv.get('byteOffset', 0) + a.get('byteOffset', 0)
    out = []
    for k in range(n):
        vals = struct.unpack_from('<' + fmt * c, bn, base + k * stride)
        if a.get('normalized') and fmt in 'BH':
            vals = tuple(v / (255.0 if fmt == 'B' else 65535.0) for v in vals)
        out.append(vals)
    return out


def mat_mul(a, b):
    return [sum(a[r + 4 * k] * b[k + 4 * c] for k in range(4)) for c in range(4) for r in range(4)]


def node_matrix(n):
    if 'matrix' in n:
        return list(n['matrix'])
    t = n.get('translation', [0, 0, 0])
    q = n.get('rotation', [0, 0, 0, 1])
    s = n.get('scale', [1, 1, 1])
    x, y, z, w = q
    r = [1 - 2 * (y * y + z * z), 2 * (x * y + z * w), 2 * (x * z - y * w), 0,
         2 * (x * y - z * w), 1 - 2 * (x * x + z * z), 2 * (y * z + x * w), 0,
         2 * (x * z + y * w), 2 * (y * z - x * w), 1 - 2 * (x * x + y * y), 0, 0, 0, 0, 1]
    for c in range(3):
        for k in range(3):
            r[c * 4 + k] *= s[c]
    r[12], r[13], r[14] = t
    return r


def texture_image(js, bn, glb_dir, tex):
    """A texture as a PIL picture (when Pillow is here), or None."""
    try:
        from PIL import Image
        import io
    except ImportError:
        return None
    try:
        src = js['images'][js['textures'][tex['index']]['source']]
        if 'bufferView' in src:
            bv = js['bufferViews'][src['bufferView']]
            raw = bn[bv.get('byteOffset', 0): bv.get('byteOffset', 0) + bv['byteLength']]
            return Image.open(io.BytesIO(raw)).convert('RGBA')
        uri = src.get('uri', '')
        if uri.startswith('data:'):
            return Image.open(io.BytesIO(base64.b64decode(uri.split(',', 1)[1]))).convert('RGBA')
        return Image.open(os.path.join(glb_dir, uri)).convert('RGBA')
    except Exception:  # noqa: BLE001 - a texture that can't be read: the preview goes without it
        return None


def make_preview(d, max_faces=14000):
    """preview.json (at most 14 000 faces with their colours, for the app's own viewport), stats.json (faces, points,
    size, closed), and the materials' small pictures (map-colour.png, map-roughness.png, map-metal.png, map-bumps.png:
    the model's own maps; a model with only a colour map gets the other three worked out from it, said so in stats)."""
    js, bn = glb_read(os.path.join(d, 'model.glb'))
    verts, faces, cols = [], [], []
    maps = {}
    derived = False

    def walk(ni, m):
        n = js['nodes'][ni]
        m = mat_mul(m, node_matrix(n))
        if 'mesh' in n:
            for prim in js['meshes'][n['mesh']]['primitives']:
                if prim.get('mode', 4) != 4:
                    continue
                pos = accessor(js, bn, prim['attributes']['POSITION'])
                base = len(verts)
                for p in pos:
                    verts.append((m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12], m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13], m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14]))
                idx = [i[0] for i in accessor(js, bn, prim['indices'])] if 'indices' in prim else list(range(len(pos)))
                mat = js.get('materials', [{}])[prim['material']] if 'material' in prim else {}
                pbr = mat.get('pbrMetallicRoughness', {})
                factor = pbr.get('baseColorFactor', [0.8, 0.8, 0.8, 1])
                vc = accessor(js, bn, prim['attributes']['COLOR_0']) if 'COLOR_0' in prim['attributes'] else None
                uv = accessor(js, bn, prim['attributes']['TEXCOORD_0']) if 'TEXCOORD_0' in prim['attributes'] else None
                img = texture_image(js, bn, d, pbr['baseColorTexture']) if 'baseColorTexture' in pbr and uv else None
                for key, tex in (('colour', pbr.get('baseColorTexture')), ('roughness', pbr.get('metallicRoughnessTexture')), ('bumps', mat.get('normalTexture'))):
                    if tex and key not in maps:
                        im = texture_image(js, bn, d, tex)
                        if im is not None:
                            maps[key] = im
                for k in range(0, len(idx) - 2, 3):
                    a, b, c = idx[k], idx[k + 1], idx[k + 2]
                    faces.append((base + a, base + b, base + c))
                    if img is not None:
                        u = (uv[a][0] + uv[b][0] + uv[c][0]) / 3.0
                        v = (uv[a][1] + uv[b][1] + uv[c][1]) / 3.0
                        px = img.getpixel((min(img.width - 1, max(0, int(u % 1.0 * img.width))), min(img.height - 1, max(0, int(v % 1.0 * img.height)))))
                        cols.append((px[0] / 255.0 * factor[0], px[1] / 255.0 * factor[1], px[2] / 255.0 * factor[2]))
                    elif vc is not None:
                        cols.append(tuple(sum(vc[i][j] for i in (a, b, c)) / 3.0 for j in range(3)))
                    else:
                        cols.append(tuple(factor[:3]))
        for ch in n.get('children', []):
            walk(ch, m)
    ident = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]
    scene = js.get('scenes', [{'nodes': list(range(len(js.get('nodes', []))))}])[js.get('scene', 0)]
    for ni in scene.get('nodes', []):
        walk(ni, ident)
    if not faces:
        raise Fail('the model has no faces')
    # closed (every edge in exactly two faces): it prints as a solid
    edges = {}
    for f in faces:
        for e in ((f[0], f[1]), (f[1], f[2]), (f[2], f[0])):
            k = (min(e), max(e))
            edges[k] = edges.get(k, 0) + 1
    closed = all(v == 2 for v in edges.values())
    lo = [min(v[i] for v in verts) for i in range(3)]
    hi = [max(v[i] for v in verts) for i in range(3)]
    keep = faces if len(faces) <= max_faces else [faces[int(i * len(faces) / max_faces)] for i in range(max_faces)]
    kcol = cols if len(faces) <= max_faces else [cols[int(i * len(faces) / max_faces)] for i in range(max_faces)]
    used = sorted({i for f in keep for i in f})
    remap = {o: n for n, o in enumerate(used)}
    pv = [round(c, 5) for i in used for c in verts[i]]
    pf = [remap[i] for f in keep for i in f]
    pc = [int(max(0.0, min(1.0, c)) * 255) for col in kcol for c in col]
    write_json(os.path.join(d, 'preview.json'), {'v': pv, 'f': pf, 'c': pc, 'up': 'y'})
    if 'colour' in maps:
        derived = 'roughness' not in maps
        maps.update(derive_maps(maps, derived_from='colour'))
    for k, im in maps.items():
        try:
            im.copy().resize((256, 256)).save(os.path.join(d, 'map-%s.png' % k))
        except Exception:  # noqa: BLE001
            pass
    write_json(os.path.join(d, 'stats.json'), {'faces': len(faces), 'points': len(verts), 'size': [hi[i] - lo[i] for i in range(3)], 'closed': closed,
                                               'maps': sorted(maps), 'mapsWorkedOut': derived})


def derive_maps(maps, derived_from='colour'):
    """Roughness, metal and bumps from the model's own maps when it has them (glTF keeps roughness in green and metal
    in blue of one picture), else worked out from its colour: darker and less coloured reads as metal, the colour's
    fine detail as bumps."""
    from PIL import Image, ImageFilter, ImageOps, ImageChops
    out = {}
    col = maps[derived_from].convert('RGB').resize((256, 256))
    if 'roughness' in maps:
        r, g, b = maps['roughness'].convert('RGB').resize((256, 256)).split()
        out['roughness'] = Image.merge('RGB', (g, g, g))
        out['metal'] = Image.merge('RGB', (b, b, b))
    else:
        grey = ImageOps.grayscale(col)
        out['roughness'] = Image.merge('RGB', [ImageOps.invert(grey)] * 3)
        sat = col.convert('HSV').split()[1]
        metal = ImageOps.invert(ImageChops.lighter(sat, grey.point(lambda v: 255 - v)))
        out['metal'] = Image.merge('RGB', [metal] * 3)
    if 'bumps' not in maps:
        grey = ImageOps.grayscale(col).filter(ImageFilter.GaussianBlur(1))
        dx = grey.filter(ImageFilter.Kernel((3, 3), [-1, 0, 1, -2, 0, 2, -1, 0, 1], 1, 128))
        dy = grey.filter(ImageFilter.Kernel((3, 3), [-1, -2, -1, 0, 0, 0, 1, 2, 1], 1, 128))
        out['bumps'] = Image.merge('RGB', (dx, dy, Image.new('L', grey.size, 255)))
    return out


# ------------------------------------------------------------------ exports (through Blender)

BLENDER_SCRIPT = r'''
import bpy, bmesh, sys, json, math
a = json.loads(sys.argv[sys.argv.index("--") + 1])
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=a["src"])
objs = [o for o in bpy.context.scene.objects if o.type == "MESH"]
bpy.ops.object.select_all(action="DESELECT")
for o in objs:
    o.select_set(True)
bpy.context.view_layer.objects.active = objs[0]
if len(objs) > 1:
    bpy.ops.object.join()
o = bpy.context.view_layer.objects.active
bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
h = max(o.dimensions) or 1.0
if a.get("height"):
    s = a["height"] / (o.dimensions.z or h)
    o.scale = (s, s, s)
    bpy.ops.object.transform_apply(scale=True)
def mod(kind, **kw):
    m = o.modifiers.new(kind.lower(), kind)
    for k, v in kw.items():
        setattr(m, k, v)
    bpy.ops.object.modifier_apply(modifier=m.name)
if a.get("solid"):
    mod("REMESH", mode="VOXEL", voxel_size=max(o.dimensions) / 300.0)
if a.get("flat"):
    bm = bmesh.new(); bm.from_mesh(o.data)
    zmin = min(v.co.z for v in bm.verts); cut = zmin + (max(v.co.z for v in bm.verts) - zmin) * 0.01
    r = bmesh.ops.bisect_plane(bm, geom=bm.verts[:] + bm.edges[:] + bm.faces[:], plane_co=(0, 0, cut), plane_no=(0, 0, 1), clear_inner=True)
    edges = [e for e in r["geom_cut"] if isinstance(e, bmesh.types.BMEdge)]
    if edges:
        bmesh.ops.holes_fill(bm, edges=edges)
    bm.to_mesh(o.data); bm.free()
if a.get("hollow"):
    mod("SOLIDIFY", thickness=a["hollow"] / 1000.0 * (1.0 if a.get("height") else h / 0.3), offset=-1.0, use_even_offset=True)
if a.get("faces") and len(o.data.polygons) > a["faces"]:
    mod("DECIMATE", ratio=max(0.001, a["faces"] / len(o.data.polygons)))
if a.get("texture"):
    for img in bpy.data.images:
        if img.size[0] > a["texture"]:
            img.scale(a["texture"], a["texture"])
            img.pack()
f, out = a["format"], a["out"]
if f == "glb":
    bpy.ops.export_scene.gltf(filepath=out, export_format="GLB")
elif f == "obj":
    bpy.ops.wm.obj_export(filepath=out, path_mode="COPY")
elif f == "fbx":
    bpy.ops.export_scene.fbx(filepath=out, path_mode="COPY", embed_textures=True)
elif f == "stl":
    try:
        bpy.ops.wm.stl_export(filepath=out, global_scale=1000.0 if a.get("height") else 1.0)
    except AttributeError:
        bpy.ops.export_mesh.stl(filepath=out, global_scale=1000.0 if a.get("height") else 1.0)
elif f == "usdz":
    bpy.ops.wm.usd_export(filepath=out)
elif f == "blend":
    bpy.ops.wm.save_as_mainfile(filepath=out)
'''

FORMATS = ('glb', 'obj', 'fbx', 'stl', 'usdz', 'blend')


def blender():
    b = shutil.which('blender')
    if b:
        return [b]
    if shutil.which('flatpak') and cmd(['flatpak', 'info', 'org.blender.Blender'], 20)[0] == 0:
        return ['flatpak', 'run', '--filesystem=home', 'org.blender.Blender']
    return []


def export(d, fmt, out, faces=0, texture=0, solid=False, flat=False, hollow=0.0, height=0.0):
    if fmt not in FORMATS:
        raise Fail('Hearth Forge saves .glb .obj .fbx .stl .usdz')
    src = os.path.join(d, 'model.glb')
    if not os.path.isfile(src):
        raise Fail('It isn\'t made yet')
    simple = fmt == 'glb' and not (faces or texture or solid or flat or hollow or height)
    if simple:
        os.makedirs(os.path.dirname(out), exist_ok=True)
        shutil.copyfile(src, out)
        return out
    b = blender()
    if not b:
        raise Fail('Saving as .%s (and fewer faces, smaller textures, printing) needs Blender: Hearth 3D gets it for you' % fmt)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    args = json.dumps({'src': src, 'format': fmt, 'out': out, 'faces': int(faces), 'texture': int(texture), 'solid': bool(solid), 'flat': bool(flat), 'hollow': float(hollow), 'height': float(height)})
    with tempfile.NamedTemporaryFile('w', suffix='.py', delete=False) as f:
        f.write(BLENDER_SCRIPT)
        script = f.name
    try:
        code, log = cmd(b + ['-b', '--factory-startup', '--python', script, '--', args], 1800)
    finally:
        os.unlink(script)
    if code != 0 or not os.path.isfile(out):
        raise Fail('Blender couldn\'t save it: ' + (log.strip().splitlines() or ['no reason'])[-1][:300])
    return out


# ------------------------------------------------------------------ Tripo3D online (your own key)

def tripo_call(method, path, key, body=None, ctype='application/json'):
    data = None
    if body is not None:
        data = body if isinstance(body, bytes) else json.dumps(body).encode()
    req = urllib.request.Request(TRIPO + path, data=data, method=method, headers={'Authorization': 'Bearer ' + key, 'Content-Type': ctype, 'User-Agent': 'Hearth-Forge'})
    try:
        with urllib.request.urlopen(req, timeout=60) as r:
            v = json.load(r)
    except urllib.error.HTTPError as e:
        try:
            msg = json.load(e).get('message', '')
        except Exception:  # noqa: BLE001
            msg = ''
        raise Fail('Tripo3D said %s%s' % (e.code, ': ' + msg if msg else ''))
    except (urllib.error.URLError, OSError, ValueError) as e:
        raise Fail('Tripo3D can\'t be reached: %s' % e)
    if v.get('code') not in (0, None):
        raise Fail('Tripo3D said: %s' % (v.get('message') or v.get('code')))
    return v.get('data') or {}


def tripo_key():
    k = secret('tripo3d')
    if not k:
        raise Fail('No Tripo3D key yet: add yours in Hearth Forge (it goes into the system\'s password store)')
    return k


def tripo_balance():
    data = tripo_call('GET', '/user/balance', tripo_key())
    return {'balance': data.get('balance'), 'frozen': data.get('frozen')}


def tripo_make(d):
    """A model made by Tripo3D with your key: never asked twice (its task id is kept in the folder and picked up again),
    then downloaded into the folder like any other."""
    key = tripo_key()
    ask = read_json(os.path.join(d, 'ask.json'), {})
    task = read_json(os.path.join(d, 'tripo.json'), {})
    tid = task.get('task')
    if not tid:
        tex = ask.get('textures', 'pbr')
        body = {'texture': tex != 'none', 'pbr': tex == 'pbr', 'model_version': 'v2.5-20250123'}
        if ask.get('seed'):
            body['model_seed'] = int(ask['seed'])
        if os.path.isfile(os.path.join(d, 'photo.png')) and ask.get('mode') != 'words':
            with open(os.path.join(d, 'photo.png'), 'rb') as f:
                img = f.read()
            b = '----hearthforge%d' % int(time.time())
            mp = ('--%s\r\nContent-Disposition: form-data; name="file"; filename="photo.png"\r\nContent-Type: image/png\r\n\r\n' % b).encode() + img + ('\r\n--%s--\r\n' % b).encode()
            tok = tripo_call('POST', '/upload', key, mp, 'multipart/form-data; boundary=' + b).get('image_token')
            body.update({'type': 'image_to_model', 'file': {'type': 'png', 'file_token': tok}})
        else:
            body.update({'type': 'text_to_model', 'prompt': ask.get('words', '')[:1000]})
        tid = tripo_call('POST', '/task', key, body).get('task_id')
        if not tid:
            raise Fail('Tripo3D didn\'t start it')
        write_json(os.path.join(d, 'tripo.json'), {'task': tid, 'at': int(time.time())})
    t0 = time.time()
    while time.time() - t0 < 1800:
        data = tripo_call('GET', '/task/' + tid, key)
        st = data.get('status')
        write_json(os.path.join(d, 'progress.json'), {'stage': 'Tripo3D: %s' % st, 'progress': data.get('progress') or 0})
        if st == 'success':
            o = data.get('output') or {}
            url = o.get('pbr_model') or o.get('model') or o.get('base_model')
            if not url or not url.startswith('https://'):
                raise Fail('Tripo3D gave no model')
            with urllib.request.urlopen(urllib.request.Request(url, headers={'User-Agent': 'Hearth-Forge'}), timeout=300) as r, open(os.path.join(d, 'model.glb'), 'wb') as f:
                shutil.copyfileobj(r, f)
            make_preview(d)
            write_json(os.path.join(d, 'done.json'), {'ok': True, 'at': int(time.time()), 'secs': int(time.time() - t0), 'model': 'Tripo3D', 'credits': data.get('consumed_credit')})
            try:
                os.unlink(os.path.join(d, 'progress.json'))
            except OSError:
                pass
            return
        if st in ('failed', 'cancelled', 'banned', 'expired', 'unknown'):
            raise Fail('Tripo3D: %s' % st)
        time.sleep(5)
    raise Fail('Tripo3D is taking too long: it carries on, and Hearth picks it up next time')


# ------------------------------------------------------------------ the service

def gpu():
    code, out = cmd(['nvidia-smi', '--query-gpu=name,memory.total,memory.used', '--format=csv,noheader,nounits'], 10)
    if code != 0 or not out.strip():
        return {}
    parts = [x.strip() for x in out.strip().splitlines()[0].split(',')]
    try:
        return {'name': parts[0], 'vramTotal': float(parts[1]) * 1048576, 'vramUsed': float(parts[2]) * 1048576}
    except (IndexError, ValueError):
        return {'name': parts[0]}


_gpu_cache = {'at': 0, 'v': {}}


def status():
    if time.time() - _gpu_cache['at'] > 30:
        _gpu_cache['v'] = gpu()
        _gpu_cache['at'] = time.time()
    cat = catalog()
    lk = lock()
    q = queue()
    running = None
    if q:
        pr = read_json(os.path.join(FORGE, q[0], 'progress.json'), {})
        # waiting while you play is waiting, not being made
        if pr and pr.get('stage') != 'waiting while you play':
            running = {'id': q[0], 'stage': pr.get('stage', 'starting'), 'progress': pr.get('progress', 0), 'timing': pr.get('timing', {})}
    with _jobs_lock:
        getting = dict(JOBS)
    models = {}
    for m in cat['models']:
        models[m['id']] = {'installed': installed(m, lk), 'bytes': (lk.get(m['id']) or {}).get('bytes', 0),
                           'runner': bool(m.get('runner')) and runner_ready(m['runner']), 'getting': getting.get(m['id'])}
    return {'protocol': 1, 'gpu': _gpu_cache['v'], 'queue': [{'id': x, 'title': read_json(os.path.join(FORGE, x, 'ask.json'), {}).get('title', x)} for x in q],
            'running': running, 'models': models, 'playing': game_running(), 'kept': {'used': folder_bytes(MODELS), 'free': free_gb(MODELS) * 1e9, 'dir': MODELS}}


def start_get(mid):
    with _jobs_lock:
        j = JOBS.get(mid)
        if j and j.get('busy'):
            return
        JOBS[mid] = {'busy': True, 'step': 'starting', 'progress': 0, 'error': ''}

    def go():
        try:
            get_model(mid)
            set_job(mid, busy=False, step='', progress=100)
        except Exception as e:  # noqa: BLE001
            set_job(mid, busy=False, error=str(e)[:300])
    threading.Thread(target=go, daemon=True, name='get-' + mid).start()


MAX_BODY = 64 << 20


class Handler(http.server.BaseHTTPRequestHandler):
    server_version = 'HearthForge/1'

    def log_message(self, *a):
        pass

    def allowed(self):
        ip = self.client_address[0]
        return ip.startswith('127.') or ip == '::1' or TS_IP_RE.fullmatch(ip) is not None

    def send(self, code, body, ctype='application/json'):
        b = body if isinstance(body, bytes) else json.dumps(body).encode()
        self.send_response(code)
        self.send_header('Content-Type', ctype)
        self.send_header('Content-Length', str(len(b)))
        self.end_headers()
        self.wfile.write(b)

    def body(self):
        n = int(self.headers.get('Content-Length') or 0)
        if n > MAX_BODY:
            raise Fail('too big')
        return self.rfile.read(n) if n else b''

    def made_dir(self, name):
        if not re.fullmatch(r'[a-z0-9][a-z0-9-]{0,48}', name or ''):
            raise Fail('no such model')
        d = os.path.join(FORGE, name)
        if not os.path.isfile(os.path.join(d, 'ask.json')):
            raise Fail('no such model')
        return d

    def do_GET(self):
        if not self.allowed():
            return self.send(403, {'error': 'only this PC and your Tailscale devices'})
        try:
            parts = [urllib.parse.unquote(x) for x in urllib.parse.urlparse(self.path).path.strip('/').split('/')]
            if parts == ['status']:
                return self.send(200, status())
            if parts == ['made']:
                items = sorted((made_item(d) for d in made_dirs()), key=lambda x: -x['at'])
                return self.send(200, items)
            if len(parts) == 4 and parts[0] == 'made' and parts[2] == 'file':
                d = self.made_dir(parts[1])
                fn = parts[3]
                if fn not in os.listdir(d) or fn.startswith('.') or '/' in fn:
                    return self.send(404, {'error': 'no such file'})
                with open(os.path.join(d, fn), 'rb') as f:
                    data = f.read()
                ct = 'image/png' if fn.endswith('.png') else 'application/json' if fn.endswith('.json') else 'model/gltf-binary' if fn.endswith('.glb') else 'application/octet-stream'
                return self.send(200, data, ct)
            return self.send(404, {'error': 'no such thing'})
        except Fail as e:
            return self.send(400, {'error': str(e)})

    def do_POST(self):
        if not self.allowed():
            return self.send(403, {'error': 'only this PC and your Tailscale devices'})
        try:
            parts = [urllib.parse.unquote(x) for x in urllib.parse.urlparse(self.path).path.strip('/').split('/')]
            if len(parts) == 3 and parts[0] == 'models' and parts[2] in ('get', 'remove'):
                model_of(parts[1])
                if parts[2] == 'get':
                    start_get(parts[1])
                else:
                    remove_model(parts[1])
                return self.send(200, {'ok': True})
            if parts == ['make']:
                v = json.loads(self.body() or b'{}')
                photos = [base64.b64decode(p) for p in v.pop('photos', []) if isinstance(p, str)]
                model_of(v.get('model', ''))
                return self.send(200, {'id': add_job(v, photos)})
            if len(parts) == 3 and parts[0] == 'made' and parts[2] == 'stop':
                d = self.made_dir(parts[1])
                if parts[1] in queue()[1:]:
                    state_set(lambda s: s.__setitem__('queue', [x for x in s.get('queue', []) if x != parts[1]]))
                else:
                    open(os.path.join(d, 'stop'), 'w').close()
                return self.send(200, {'ok': True})
            return self.send(404, {'error': 'no such thing'})
        except (Fail, ValueError) as e:
            return self.send(400, {'error': str(e)})


class Server(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True
    allow_reuse_address = True


def listen_addrs():
    out = ['127.0.0.1']
    if shutil.which('tailscale'):
        code, o = cmd(['tailscale', 'ip', '-4'], 8)
        ip = o.strip().splitlines()[0].strip() if code == 0 and o.strip() else ''
        if TS_IP_RE.fullmatch(ip):
            out.append(ip)
    return out


def serve():
    os.makedirs(FORGE, exist_ok=True)
    try:
        os.nice(5)
    except OSError:
        pass
    servers = []
    for a in listen_addrs():
        try:
            servers.append(Server((a, PORT), Handler))
        except OSError as e:
            print('can\'t listen on %s:%d: %s' % (a, PORT, e), file=sys.stderr)
    if not servers:
        raise Fail('nothing to listen on')
    for s in servers[1:]:
        threading.Thread(target=s.serve_forever, daemon=True).start()
    threading.Thread(target=worker, daemon=True, name='worker').start()
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
    servers[0].serve_forever()


UNIT_TEXT = '''# Hearth Forge (written by the app when you set it up; docs/mockups/checklists/hearth-forge.md): it makes 3D models
# on this PC's graphics card, as you. No install section: started only by its timer (a minute after you sign in,
# enabled only while it's on) or by the app. Never part of starting the PC, the session or Game Mode; low priority;
# it waits while you play. Listens on 127.0.0.1 and this PC's Tailscale address only.
[Unit]
Description=Hearth Forge: 3D models on this PC
ConditionPathExists=%h/.local/share/hearth/forge/hearth-forge.py
After=network.target

[Service]
ExecStart=/usr/bin/python3 %h/.local/share/hearth/forge/hearth-forge.py serve
Restart=on-failure
RestartSec=30
Nice=10
CPUWeight=20
IOWeight=20
IOSchedulingClass=idle
TimeoutStopSec=30
'''
TIMER_TEXT = '''# Hearth Forge: started a minute after you sign in, only while it's on (the app enables this for you).
[Unit]
Description=Hearth Forge, a minute after sign-in

[Timer]
OnStartupSec=1min

[Install]
WantedBy=timers.target
'''


def units_dir():
    return os.path.join(os.environ.get('XDG_CONFIG_HOME') or os.path.join(HOME, '.config'), 'systemd', 'user')


def switch(on):
    d = units_dir()
    if on:
        os.makedirs(d, exist_ok=True)
        for name, text in ((UNIT, UNIT_TEXT), (TIMER, TIMER_TEXT)):
            p = os.path.join(d, name)
            cur = ''
            try:
                with open(p) as f:
                    cur = f.read()
            except OSError:
                pass
            if cur != text:
                with open(p + '.tmp', 'w') as f:
                    f.write(text)
                os.replace(p + '.tmp', p)
        cmd(['systemctl', '--user', 'daemon-reload'], 30)
        cmd(['systemctl', '--user', 'enable', TIMER], 30)
        code, out = cmd(['systemctl', '--user', 'restart', UNIT], 30)
        if code != 0:
            raise Fail('It didn\'t start: ' + out.strip()[-200:])
        return {'on': True}
    cmd(['systemctl', '--user', 'disable', TIMER], 30)
    cmd(['systemctl', '--user', 'stop', UNIT], 60)
    return {'on': False}


def local_status():
    active = cmd(['systemctl', '--user', 'is-active', '--quiet', UNIT], 10)[0] == 0
    timer = cmd(['systemctl', '--user', 'is-enabled', '--quiet', TIMER], 10)[0] == 0
    return {'active': active, 'on': timer, 'unit': os.path.isfile(os.path.join(units_dir(), UNIT)), 'nvidia': bool(shutil.which('nvidia-smi')),
            'git': bool(shutil.which('git')), 'blender': bool(blender()), 'secretTool': bool(shutil.which('secret-tool')), 'free': free_gb(MODELS) * 1e9}


def main(argv):
    if not argv:
        print(__doc__)
        return 0
    try:
        c = argv[0]
        if c == 'serve':
            serve()
        elif c in ('on', 'off'):
            print(json.dumps(switch(c == 'on')))
        elif c == 'status':
            print(json.dumps(local_status()))
        elif c == 'run':
            job_run(argv[1])
        elif c == 'preview':
            make_preview(argv[1])
            print(json.dumps({'ok': True}))
        elif c == 'export':
            opts = {'faces': 0, 'texture': 0, 'solid': False, 'flat': False, 'hollow': 0.0, 'height': 0.0}
            rest = argv[4:]
            i = 0
            while i < len(rest):
                k = rest[i].lstrip('-')
                if k in ('solid', 'flat'):
                    opts[k] = True
                    i += 1
                elif k in ('faces', 'texture', 'hollow', 'height') and i + 1 < len(rest):
                    opts[k] = float(rest[i + 1]) if k in ('hollow', 'height') else int(rest[i + 1])
                    i += 2
                else:
                    raise Fail('unknown: ' + rest[i])
            print(json.dumps({'ok': True, 'file': export(argv[1], argv[2], argv[3], **opts)}))
        elif c == 'tripo' and argv[1:2] == ['balance']:
            print(json.dumps(tripo_balance()))
        elif c == 'tripo' and argv[1:2] == ['make']:
            try:
                tripo_make(argv[2])
            except Fail as e:
                write_json(os.path.join(argv[2], 'done.json'), {'ok': False, 'at': int(time.time()), 'error': str(e)[:400]})
                raise
            print(json.dumps({'ok': True}))
        else:
            print(__doc__)
            return 2
        return 0
    except Fail as e:
        print(json.dumps({'ok': False, 'error': str(e)}))
        return 1


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
