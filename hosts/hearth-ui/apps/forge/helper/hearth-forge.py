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
  - installs each model's own code (its runner: the maker's repo at a pinned git commit, checked, with the maker's
    own pinned requirements and the Python its maker names) in its own environment in ~/Hearth/Forge/runtimes/<runner>,
    with PyTorch's CUDA 12.8 build (the RTX 50s need it) and its parts built for this card, only when you press Get;
  - makes each model you ask for, one at a time, each in a program of its own that ends when it's done (so the
    graphics card's memory is given back), through the model's own code (forge_runners.py, next to this file), with
    the maker's low-memory or offload settings on a card smaller than the maker's figure; a refiner's first shape
    made by another model first, a shape-only model painted by Hunyuan3D 2.0 after, a splat kept and a mesh made from
    it; and keeps every one in ~/Hearth/Forge/<name> with its photo, words and settings.

On any device the app also runs it for Tripo3D online (`tripo`: your own key, read from the system's password store
by this program itself, never printed or written anywhere), for the small preview of a model (`preview`) and for the
exports (`export`, through Blender: .glb .obj .fbx .stl .usdz, fewer faces, smaller textures, solid / flat bottom /
hollow for printing).

  hearth-forge.py on | off | status            the service (on: written, its timer enabled, started)
  hearth-forge.py serve                         the service itself
  hearth-forge.py run JOBDIR [--as MODEL --out coarse.glb --textures none]
                                                one model being made (inside its runner's Python)
  hearth-forge.py paint JOBDIR                  its shape painted by Hunyuan3D 2.0 (inside Hunyuan3D 2.0's Python)
  hearth-forge.py preview MADEDIR               preview.json and stats.json from model.glb
  hearth-forge.py export MADEDIR FORMAT OUT [--faces N] [--texture PX] [--solid] [--flat] [--hollow MM] [--height M]
  hearth-forge.py tripo balance | make JOBDIR
"""
import base64
import fnmatch
import hashlib
import http.server
import json
import math
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
RUNTIMES = os.path.join(FORGE, 'runtimes')
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


SCAN_INCLUDE = ['*.json', '*.safetensors', '*.txt', '*.model', '*.py', '*.pt', '*.yaml']
SCAN_KEY = re.compile(r'["\']?(?:[a-z_]*(?:model|name|path|repo|pretrained)[a-z_]*)["\']?\s*[:=]\s*["\']?([A-Za-z0-9][A-Za-z0-9_.-]{0,95}/[A-Za-z0-9][A-Za-z0-9_.-]{0,95})["\']?', re.I)


def scan_repos(m, plan):
    """The Hugging Face repos a model's own config files name ("scan" in models.json): keys like model_name,
    pretrained_model_name_or_path, repo; never a path that's a file or folder in the model itself."""
    out = []
    for repo, commit, need in plan:
        d = snap_dir(repo, commit)
        for f in need:
            if not any(fnmatch.fnmatchcase(f['path'], pat) for pat in m.get('scan') or []) or f['size'] > 2 << 20:
                continue
            try:
                with open(os.path.join(d, *f['path'].split('/')), encoding='utf-8', errors='replace') as fh:
                    text = fh.read()
            except OSError:
                continue
            for hit in SCAN_KEY.findall(text):
                first = hit.split('/')[0]
                if REPO_RE.fullmatch(hit) and not os.path.exists(os.path.join(d, first)) and not hit.endswith(('.json', '.yaml', '.safetensors', '.pt', '.ckpt', '.png')) and hit not in out:
                    out.append(hit)
    return out


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

    def fetch(items):
        for repo, commit, need in items:
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
    fetch(plan)
    extra = []
    # the other Hugging Face repos the model's own config files name (TRELLIS.2's DINOv3 and BiRefNet, …): got too,
    # each checked the same way; one Hugging Face doesn't have is skipped, one that asks for a licence stops the Get
    seen = {r for r, _, _ in plan}
    for repo in scan_repos(m, plan):
        if repo in seen:
            continue
        seen.add(repo)
        rev = next((p['commit'] for p in prev if p.get('repo') == repo), '')
        try:
            commit, files = hf_files(repo, rev, token)
        except Fail as e:
            if 'isn\'t on Hugging Face' in str(e):
                continue
            raise
        need = wanted(files, SCAN_INCLUDE)
        if need:
            extra.append((repo, commit, need))
    if extra:
        fetch(extra)
        plan += extra
    for rid in [m.get('runner')] + list(m.get('alsoRunners') or []):
        if rid:
            set_job(mid, step='getting its own code', progress=100)
            install_runner(rid, cat, lambda s: set_job(mid, step=s))

    total += sum(f['size'] for _, _, need in extra for f in need)

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

UV_PIN = 'uv==0.13.5'
SKIP_ALWAYS = ('torch', 'torchvision', 'torchaudio', 'triton', 'xformers')
NAME_OF = re.compile(r'^\s*([A-Za-z0-9][A-Za-z0-9._-]*)')
GIT_PINNED = re.compile(r'git\+\S+@[0-9a-f]{40}(?:#\S*)?(?:\s|$)')


def runner_dir(rid):
    if not NAME_RE.fullmatch(rid):
        raise Fail('bad runner name')
    return os.path.join(RUNTIMES, rid)


def runner_python(rid):
    return os.path.join(runner_dir(rid), 'venv', 'bin', 'python')


def runner_src(rid, r=None):
    """Where the model's own code runs from: its repo, or the folder in it the maker names (Lyra 2.0: Lyra-2)."""
    r = r if r is not None else (catalog()['runners'].get(rid) or {})
    return os.path.join(runner_dir(rid), 'src', *[x for x in (r.get('root') or '').split('/') if x and x != '..'])


def runner_ready(rid, cat=None):
    r = ((cat or catalog())['runners'].get(rid)) or {}
    return read_json(os.path.join(runner_dir(rid), 'ready.json'), {}).get('commit') == r.get('commit') and os.path.isfile(runner_python(rid))


def req_name(line):
    m = NAME_OF.match(line)
    return m.group(1).lower().replace('_', '-').split('[')[0] if m else ''


def filter_requirements(text, r):
    """The maker's own requirements, as Hearth installs them: (now, built after, dropped). Index and option lines go
    (only PyPI and PyTorch's own index are used), PyTorch's own packages go (the CUDA 12.8 build is installed first and
    kept by a constraint), "skip" drops the maker's demo-only parts, "replace" pins a line the maker left unpinned, and
    a git requirement that doesn't name a commit is refused: nothing installs that could change under it."""
    skip = {s.lower().replace('_', '-') for s in list(SKIP_ALWAYS) + list(r.get('skip') or [])}
    skip -= {req_name(x) for x in r.get('torch') or []}
    late_names = {s.lower().replace('_', '-') for s in r.get('late') or [] if not s.startswith(('git+', 'http'))}
    rep = r.get('replace') or {}
    now, late, dropped = [], [], []
    for raw in text.splitlines():
        line = raw.split(' #', 1)[0].strip() if not raw.strip().startswith('#') else ''
        if not line:
            continue
        line = rep.get(line, line)
        if line.startswith('-'):
            dropped.append(line)
            continue
        if line.startswith(('./', '../')):
            late.append(line)
            continue
        if 'git+' in line:
            if not GIT_PINNED.search(line + ' '):
                raise Fail('%s isn\'t pinned to a commit: Hearth won\'t install it' % line)
            (late if r.get('lateGit') or req_name(line.split('@')[0]) in late_names else now).append(line)
            continue
        name = req_name(line)
        if name in skip or name.startswith('nvidia-'):
            dropped.append(line)
        elif name in late_names:
            late.append(line)
        else:
            now.append(line)
    return now, late, dropped


def nvcc():
    """NVIDIA's CUDA compiler for the parts a model builds: (path, version) or ('', '')."""
    for c in (shutil.which('nvcc') or '', '/usr/local/cuda/bin/nvcc', '/usr/lib/nvidia-cuda-toolkit/bin/nvcc'):
        if c and os.path.isfile(c):
            code, out = cmd([c, '--version'], 30)
            m = re.search(r'release (\d+)\.(\d+)', out)
            if code == 0 and m:
                return c, '%s.%s' % m.groups()
    return '', ''


def cuda_arch(version):
    """What to build for: the RTX 50s' own sm_120 with CUDA 12.8 or newer; with an older toolkit, 8.9 and its PTX
    (which the card's driver compiles for itself)."""
    try:
        major, minor = (int(x) for x in version.split('.')[:2])
    except ValueError:
        return '8.9+PTX'
    return '12.0' if (major, minor) >= (12, 8) else '8.9+PTX'


def ensure_uv(step):
    """uv (pinned) in a folder of its own: it makes each model's environment with the Python its maker names."""
    d = os.path.join(RUNTIMES, '.uv')
    uv = os.path.join(d, 'bin', 'uv')
    if os.path.isfile(uv):
        return uv
    step('getting uv (Python environments)')
    code, out = cmd(['/usr/bin/python3', '-m', 'venv', d], 600)
    if code != 0:
        raise Fail('Couldn\'t make a Python environment (python3-venv): ' + out.strip()[-200:])
    code, out = cmd([os.path.join(d, 'bin', 'python'), '-m', 'pip', 'install', '--disable-pip-version-check', '--no-input', '--no-cache-dir', UV_PIN], 900)
    if code != 0 or not os.path.isfile(uv):
        raise Fail('uv didn\'t install: ' + out.strip()[-200:])
    return uv


def pip_install_cmd(py, cons, links=()):
    """The pip install command line for a model's runtime. --progress-bar is an install option: it goes after
    'install' (before it, pip stops with "no such option: --progress-bar")."""
    return [py, '-m', 'pip', '--disable-pip-version-check', '--no-input', '--no-cache-dir', 'install', '--progress-bar', 'off',
            '-c', cons] + list(links)


def install_runner(rid, cat, step):
    """The model's own code at its pinned commit (checked) into ~/Hearth/Forge/runtimes/<runner>, a Python environment
    with the Python its maker names, PyTorch's CUDA 12.8 build, the maker's own pinned requirements, then the parts it
    builds (with this PC's CUDA compiler, for this card). Anything wrong: you're told why, and nothing that was working
    is touched (a runtime is only ready when ready.json says so)."""
    r = cat['runners'][rid]
    d = runner_dir(rid)
    if runner_ready(rid, cat):
        return
    if not shutil.which('git'):
        raise Fail('git isn\'t on this PC')
    builds = bool(r.get('late') or r.get('build'))
    cc, ccv = nvcc()
    if builds and not cc and not r.get('optionalBuild'):
        raise Fail('%s builds parts of its own for the graphics card: it needs NVIDIA\'s CUDA toolkit (nvcc) on this PC first' % r['name'])
    os.makedirs(d, exist_ok=True)
    src = os.path.join(d, 'src')
    if os.path.isdir(os.path.join(src, '.git')) and cmd(['git', '-C', src, 'rev-parse', 'HEAD'], 20)[1].strip() != r['commit']:
        shutil.rmtree(src, ignore_errors=True)
    if not os.path.isdir(os.path.join(src, '.git')):
        step('getting %s' % r['name'])
        tmp = src + '.new'
        shutil.rmtree(tmp, ignore_errors=True)
        cmd(['git', 'init', '-q', tmp], 60)
        code, out = cmd(['git', '-C', tmp, 'fetch', '-q', '--depth', '1', r['git'], r['commit']], 1800)
        if code != 0:
            shutil.rmtree(tmp, ignore_errors=True)
            raise Fail('Couldn\'t get %s: %s' % (r['name'], out.strip()[-200:]))
        cmd(['git', '-C', tmp, 'checkout', '-q', '--detach', 'FETCH_HEAD'], 300)
        code, got = cmd(['git', '-C', tmp, 'rev-parse', 'HEAD'], 20)
        if got.strip() != r['commit']:
            shutil.rmtree(tmp, ignore_errors=True)
            raise Fail('%s isn\'t the commit Hearth pinned: not installed' % r['name'])
        if r.get('submodules'):
            step('getting the parts it includes (submodules)')
            code, out = cmd(['git', '-C', tmp, 'submodule', 'update', '--init', '--recursive', '--depth', '1'], 3600)
            if code != 0:
                shutil.rmtree(tmp, ignore_errors=True)
                raise Fail('Couldn\'t get %s\'s submodules: %s' % (r['name'], out.strip()[-200:]))
        os.rename(tmp, src)
    root = runner_src(rid, r)
    venv = os.path.join(d, 'venv')
    py = runner_python(rid)
    if not os.path.isfile(py):
        step('making its Python environment (Python %s)' % r['python'] if r.get('python') else 'making its Python environment')
        if r.get('python'):
            code, out = cmd([ensure_uv(step), 'venv', '--seed', '--python', r['python'], venv], 1800, env=dict(os.environ, UV_PYTHON_INSTALL_DIR=os.path.join(RUNTIMES, '.python')))
        else:
            code, out = cmd(['/usr/bin/python3', '-m', 'venv', venv], 600)
        if code != 0 or not os.path.isfile(py):
            raise Fail('Couldn\'t make its Python environment: ' + out.strip()[-200:])
    env = dict(os.environ, PATH=os.path.join(venv, 'bin') + os.pathsep + os.environ.get('PATH', ''), VIRTUAL_ENV=venv, MAX_JOBS='4', NATTEN_N_WORKERS='4', FORCE_CUDA='1')
    if cc:
        arch = cuda_arch(ccv)
        env.update(CUDA_HOME=os.path.dirname(os.path.dirname(cc)), TORCH_CUDA_ARCH_LIST=arch, NATTEN_CUDA_ARCH=arch.split('+')[0])
    env.update({k: str(v) for k, v in (r.get('env') or {}).items()})
    torch = r.get('torch') or ['torch==2.7.1', 'torchvision==0.22.1']
    cons = os.path.join(d, 'constraints.txt')
    with open(cons, 'w') as f:
        f.write('\n'.join(x for x in torch if req_name(x) in ('torch', 'torchvision', 'torchaudio')) + '\n')
    links = [a for u in r.get('findLinks') or [] for a in ('--find-links', u)]

    def pip(args, timeout, label, isolated=True):
        step(label)
        base = pip_install_cmd(py, cons, links)
        code, out = cmd(base + ([] if isolated else ['--no-build-isolation']) + args, timeout, cwd=root, env=env)
        if code != 0:
            raise Fail('%s didn\'t work: %s' % (label[0].upper() + label[1:], out.strip()[-240:]))
    if cmd([py, '-m', 'pip', 'show', 'torch'], 120)[0] != 0:
        pip(['--index-url', TORCH_INDEX] + torch, 3 * 3600, 'downloading PyTorch for CUDA 12.8 (a few GB)')
    now, late, dropped = [], [], []
    for rf in r.get('requirements') or []:
        p = os.path.join(root, *rf.split('/'))
        if os.path.realpath(p).startswith(os.path.realpath(src) + os.sep) and os.path.isfile(p):
            with open(p, encoding='utf-8', errors='replace') as f:
                a, b, c = filter_requirements(f.read(), r)
            now += a
            late += b
            dropped += c
    if any(x.startswith(('./', '../')) for x in late) and not cc:
        raise Fail('%s builds parts of its own for the graphics card: it needs NVIDIA\'s CUDA toolkit (nvcc) on this PC first' % r['name'])
    if now:
        req = os.path.join(d, 'requirements-hearth.txt')
        with open(req, 'w') as f:
            f.write('\n'.join(now) + '\n')
        pip(['--no-deps', '-r', req] if r.get('noDeps') else ['-r', req], 3 * 3600, 'installing what %s needs (its own list)' % r['name'])
    if r.get('pip'):
        pip(list(r['pip']), 3600, 'installing what %s needs' % r['name'])
    deferred = [x for x in late if not x.startswith(('./', '../'))]
    names = {req_name(x) for x in deferred if not x.startswith(('git+', 'http'))}
    for item in deferred + [x for x in r.get('late') or [] if x.startswith(('git+', 'http')) or req_name(x) not in names]:
        label = item.split('@')[0].rstrip('/').split('/')[-1].replace('.git', '') if '/' in item else item
        pip((['--no-deps'] if r.get('lateNoDeps') else []) + [item], 3 * 3600, 'building %s' % label, isolated=False)
    for h in r.get('hub') or []:
        owner, name = h['repo'].split('/')
        hd = os.path.join(d, 'torch', 'hub', '%s_%s_main' % (owner, name))
        if not os.path.isdir(hd):
            step('getting %s (pinned)' % h['repo'])
            tmp = hd + '.new'
            shutil.rmtree(tmp, ignore_errors=True)
            cmd(['git', 'init', '-q', tmp], 60)
            code, out = cmd(['git', '-C', tmp, 'fetch', '-q', '--depth', '1', 'https://github.com/%s.git' % h['repo'], h['commit']], 1800)
            cmd(['git', '-C', tmp, 'checkout', '-q', '--detach', 'FETCH_HEAD'], 300)
            if code != 0 or cmd(['git', '-C', tmp, 'rev-parse', 'HEAD'], 20)[1].strip() != h['commit']:
                shutil.rmtree(tmp, ignore_errors=True)
                raise Fail('Couldn\'t get %s at the commit Hearth pinned' % h['repo'])
            os.rename(tmp, hd)
    built, missing = [], []
    for part in [x for x in late if x.startswith(('./', '../'))] + list(r.get('build') or []):
        part = part[2:].rstrip('/') if part.startswith('./') else part
        try:
            pip(['./' + part], 3600, 'building %s' % part, isolated=False)
            built.append(part)
        except Fail:
            if not r.get('optionalBuild'):
                raise
            missing.append(part)
    if r.get('editable'):
        spec = r['editable']
        extra = []
        if r.get('constraints'):
            cp = os.path.join(root, *r['constraints'].split('/'))
            if os.path.isfile(cp):
                keep = os.path.join(d, 'constraints-maker.txt')
                with open(cp, encoding='utf-8', errors='replace') as f, open(keep, 'w') as o:
                    o.write('\n'.join(x for x in f.read().splitlines() if req_name(x) not in SKIP_ALWAYS and not req_name(x).startswith('nvidia-') and not x.strip().startswith('-')) + '\n')
                extra = ['-c', keep]
        pip(extra + ([] if r.get('editableDeps') else ['--no-deps']) + ['-e', spec], 3 * 3600, 'installing %s itself' % r['name'], isolated=False)
    for argv in r.get('sh') or []:
        step('running its own setup step (%s)' % ' '.join(argv))
        code, out = cmd(list(argv), 1800, cwd=root, env=env)
        if code != 0:
            raise Fail('Its setup step (%s) didn\'t work: %s' % (' '.join(argv), out.strip()[-200:]))
    code, out = cmd([py, '-c', 'import json, torch; c = torch.cuda.is_available(); print(json.dumps({"torch": torch.__version__, "cuda": c, "gpu": torch.cuda.get_device_name(0) if c else ""}))'], 300)
    info = {}
    try:
        info = json.loads(out.strip().splitlines()[-1]) if code == 0 else {}
    except (ValueError, IndexError):
        pass
    # rembg's own cut-out model (it checks its MD5 itself), now rather than at the first make, where a model uses it
    env2 = dict(os.environ, U2NET_HOME=os.path.join(MODELS, 'u2net'))
    cmd([py, '-c', 'import rembg; rembg.new_session()'], 900, env=env2)
    write_json(os.path.join(d, 'ready.json'), {'commit': r['commit'], 'built': built, 'missing': missing, 'dropped': dropped[:80], 'python': r.get('python', 'system'),
                                                'nvcc': ccv, 'arch': env.get('TORCH_CUDA_ARCH_LIST', ''), 'torch': str(info.get('torch', ''))[:40],
                                                'cuda': info.get('cuda') is True, 'at': int(time.time())})


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
        if n in ('models', 'exports', 'runtimes') or n.startswith('.') or not os.path.isfile(os.path.join(d, 'ask.json')):
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


def vram_gb():
    """This PC's graphics card memory in GB (nvidia-smi), 0 without one; HEARTH_FORGE_VRAM in the tests."""
    if os.environ.get('HEARTH_FORGE_VRAM'):
        return float(os.environ['HEARTH_FORGE_VRAM'])
    return float(gpu().get('vramTotal') or 0) / 1e9


def too_big(m, vram):
    return ('%s needs a graphics card with %d GB (%s\'s own figure) and has no low-memory mode, so it can\'t be made on this PC\'s %d GB: '
            'make it on a bigger PC of yours (Settings › Where models are made)' % (m['name'], m['vramMin'], m.get('vramWho') or m['maker'], round(vram)))


def runner_env(rid, cat):
    """What a model's own program runs with: only the files Get checked (Hugging Face offline, Hearth's own folder),
    the runtime's own torch hub (its pinned dinov2, StableNormal), the maker's own settings, its code on the path."""
    r = cat['runners'][rid]
    src = runner_src(rid, r)
    env = dict(os.environ, HF_HUB_CACHE=HUB, HF_HOME=os.path.join(MODELS, '.hf-home'), HF_HUB_OFFLINE='1', TRANSFORMERS_OFFLINE='1', HF_HUB_DISABLE_TELEMETRY='1',
               HY3DGEN_MODELS=os.path.join(MODELS, '.none'), U2NET_HOME=os.path.join(MODELS, 'u2net'), TORCH_HOME=os.path.join(runner_dir(rid), 'torch'),
               PYTHONUNBUFFERED='1', PATH=os.path.dirname(runner_python(rid)) + os.pathsep + os.environ.get('PATH', ''))
    env.update({k: str(v) for k, v in (r.get('env') or {}).items()})
    env['PYTHONPATH'] = os.pathsep.join([src] + [os.path.join(src, x) for x in r.get('path') or []])
    return env, src


def spawn(d, rid, args, cat, log='log.txt'):
    """One step in the model's own Python, in a program (and process group) of its own at low priority; Stop ends it
    and everything it started."""
    env, src = runner_env(rid, cat)
    with open(os.path.join(d, log), 'a') as lf:
        p = subprocess.Popen(['nice', '-n', '10', runner_python(rid), os.path.realpath(__file__)] + args, cwd=src, env=env, stdin=subprocess.DEVNULL,
                             stdout=lf, stderr=subprocess.STDOUT, start_new_session=True)
        while p.poll() is None:
            time.sleep(1)
            if os.path.exists(os.path.join(d, 'stop')):
                try:
                    os.killpg(p.pid, signal.SIGTERM)
                except OSError:
                    p.terminate()
    return p.returncode


def log_tail(d, log='log.txt'):
    try:
        with open(os.path.join(d, log), errors='replace') as f:
            tail = f.read()[-600:]
    except OSError:
        tail = ''
    return (tail.strip().splitlines() or ['no reason given'])[-1][:300]


def ready_to_make(m, cat, lk=None):
    return installed(m, lk) and all(runner_ready(x, cat) for x in [m['runner']] + list(m.get('alsoRunners') or []))


def painter_ready(cat, lk=None):
    """Hunyuan3D 2.0's painter: its runtime and its paint files (Hunyuan3D 2.0, 2.0 mini or 2.1 bring them)."""
    if not runner_ready('hunyuan3d2', cat):
        return False
    try:
        return os.path.isdir(os.path.join(snapshot('tencent/Hunyuan3D-2'), 'hunyuan3d-paint-v2-0'))
    except Fail:
        return False


def first_shape(d, ask, m, cat):
    """A refiner's first shape (Hi3D 1.0, UltraShape): the made model you picked (ask "from"), or made now by the
    first of its "coarse" models that's got, shape only, into coarse.glb."""
    src = ask.get('from') or ''
    if src and re.fullmatch(r'[a-z0-9][a-z0-9-]{0,48}', src) and os.path.isfile(os.path.join(FORGE, src, 'model.glb')):
        shutil.copyfile(os.path.join(FORGE, src, 'model.glb'), os.path.join(d, 'coarse.glb'))
        return
    lk = lock()
    for cid in m.get('coarse') or []:
        c = model_of(cid, cat)
        if ready_to_make(c, cat, lk):
            write_json(os.path.join(d, 'progress.json'), {'stage': 'first shape with %s' % c['name'], 'progress': 3})
            code = spawn(d, c['runner'], ['run', d, '--as', cid, '--out', 'coarse.glb', '--textures', 'none'], cat)
            if code != 0 or not os.path.isfile(os.path.join(d, 'coarse.glb')):
                raise Fail('Its first shape (%s) stopped: %s' % (c['name'], log_tail(d)))
            return
    names = ' or '.join(model_of(x, cat)['name'] for x in (m.get('coarse') or [])[:2])
    raise Fail('%s makes another model\'s shape finer: Get %s first, for the shape it starts from' % (m['name'], names))


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
        if not ready_to_make(m, cat):
            raise Fail('%s\'s own code isn\'t installed yet: press Get again (Models)' % m['name'])
        vram = vram_gb()
        if m.get('vramMin') and vram < float(m['vramMin']):
            raise Fail(too_big(m, vram))
        native = (m.get('run') or {}).get('words') == 'native'
        if ask.get('mode') in ('words', 'picture') and not os.path.isfile(os.path.join(d, 'photo.png')) and not native:
            write_json(os.path.join(d, 'progress.json'), {'stage': 'making a picture from your words', 'progress': 2})
            picture_from_words(ask, d)
        elif ask.get('change') and os.path.isfile(os.path.join(d, 'photo.png')):
            write_json(os.path.join(d, 'progress.json'), {'stage': 'changing the picture with your words', 'progress': 2})
            picture_from_words(ask, d, start=os.path.join(d, 'photo.png'))
        open(os.path.join(d, 'log.txt'), 'w').close()
        if m.get('coarse'):
            first_shape(d, ask, m, cat)
        code = spawn(d, m['runner'], ['run', d], cat)
        glb, splat = os.path.join(d, 'model.glb'), os.path.join(d, 'splat.ply')
        if code != 0 or not (os.path.isfile(glb) or os.path.isfile(splat)):
            raise Fail('It stopped: ' + log_tail(d))
        res = read_json(os.path.join(d, 'result.json'), {})
        timing = read_json(os.path.join(d, 'progress.json'), {}).get('timing') or {}
        if not os.path.isfile(glb):
            write_json(os.path.join(d, 'progress.json'), {'stage': 'making a mesh from its splat', 'progress': 92, 'timing': timing})
            splat_to_mesh(splat, glb, up=(m.get('run') or {}).get('up', 'y'), detail=ask.get('detail', 'high'))
            res['meshFromSplat'] = True
            write_json(os.path.join(d, 'result.json'), res)
        if res.get('note'):
            timing['note'] = res['note'][:200]
        if res.get('shapeOnly') and ask.get('textures', 'pbr') != 'none' and m.get('paint'):
            if painter_ready(cat):
                t = time.time()
                write_json(os.path.join(d, 'progress.json'), {'stage': 'painting its textures (Hunyuan3D 2.0)', 'progress': 60, 'timing': timing})
                shutil.copyfile(glb, os.path.join(d, 'model-shape.glb'))
                if spawn(d, m['paint'], ['paint', d], cat) == 0 and os.path.isfile(glb):
                    timing['textures'] = int(time.time() - t)
                else:
                    shutil.copyfile(os.path.join(d, 'model-shape.glb'), glb)
                    timing['texturesFailed'] = ('Its textures couldn\'t be painted (%s): the shape is kept' % log_tail(d))[:300]
            else:
                timing['note'] = ((timing.get('note', '') + ' · ') if timing.get('note') else '') + 'Shape only: Get Hunyuan3D 2.0 and its painter paints this one'
        make_preview(d)
        write_json(os.path.join(d, 'done.json'), {'ok': True, 'at': int(time.time()), 'secs': int(time.time() - t0), 'timing': timing, 'model': m['name']})
        shutil.rmtree(os.path.join(d, '.work'), ignore_errors=True)
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


class Ctx:
    """One model being made, as forge_runners sees it (its folder, the ask, the model's run settings, its code, this
    card's memory, the photo, how far it is)."""
    Fail = Fail

    def __init__(self, d, m, cat, ask, out='model.glb', textures=None):
        rid = m['runner']
        self.d, self.m, self.ask, self.cat = d, m, ask, cat
        self.run = m.get('run') or {}
        self.src = runner_src(rid, cat['runners'][rid])
        self.py = runner_python(rid) if os.path.isfile(runner_python(rid)) else sys.executable
        self.bin = os.path.dirname(self.py)
        self.work = os.path.join(d, '.work')
        os.makedirs(self.work, exist_ok=True)
        self.seed = int(ask.get('seed') or 1)
        self.detail = ask.get('detail', 'high') if ask.get('detail') in DETAIL else 'high'
        self.res, self.steps = DETAIL[self.detail]
        self.tex = textures or ask.get('textures', 'pbr')
        self.tsize = TEXSIZE.get(str(ask.get('textureSize', '2k')).lower(), 2048)
        self.vram = vram_gb()
        self.photo = os.path.join(d, 'photo.png')
        self.photos = [os.path.join(d, x) for x in ask.get('photos') or [] if os.path.isfile(os.path.join(d, x)) and '/' not in x] or ([self.photo] if os.path.isfile(self.photo) else [])
        self.words = (ask.get('words') or '').strip()
        self.out = os.path.join(d, out)
        self.coarse = os.path.join(d, 'coarse.glb') if out != 'coarse.glb' and os.path.isfile(os.path.join(d, 'coarse.glb')) else ''
        self.timing = {}
        self.prefix = 'first shape: ' if out == 'coarse.glb' else ''

    def stage(self, s, pct):
        write_json(os.path.join(self.d, 'progress.json'), {'stage': self.prefix + s, 'progress': pct, 'timing': self.timing})

    def snapshot(self, repo):
        return snapshot(repo)

    def links(self, mapping, folder):
        """A folder of links to Hearth's checked files, laid out as the model's own code looks for them."""
        base = folder if os.path.isabs(folder) else os.path.join(self.work, folder)
        for k, v in mapping.items():
            dest = os.path.join(base, *k.split('/'))
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            if os.path.islink(dest):
                os.unlink(dest)
            if not os.path.exists(dest):
                os.symlink(v, dest)
        return base

    def cli(self, argv, cwd=None, env=None, timeout=12 * 3600):
        """The model's own command line (its README's), in this runtime; its output into the model's log."""
        sys.stdout.flush()
        e = dict(os.environ)
        e.update(env or {})
        try:
            p = subprocess.run(argv, cwd=cwd or self.src, env=e, stdin=subprocess.DEVNULL, timeout=timeout)
        except (OSError, subprocess.SubprocessError) as x:
            raise Fail('%s couldn\'t run: %s' % (os.path.basename(argv[1] if len(argv) > 1 else argv[0]), x))
        if p.returncode != 0:
            raise Fail('%s stopped (%d)' % (self.m['name'], p.returncode))

    def cutout(self, remover=None):
        """The photo with its background taken out (kept as photo-cut.png): kept as it is when it already has its own
        see-through background, else the model's own remover, else rembg."""
        from PIL import Image
        img = Image.open(self.photo).convert('RGBA')
        if img.getextrema()[3][0] == 255:
            if remover is not None:
                img = remover(img).convert('RGBA')
            else:
                import rembg
                img = rembg.remove(img.convert('RGB'), session=rembg.new_session()).convert('RGBA')
        img.save(os.path.join(self.d, 'photo-cut.png'))
        return img


def load_runners():
    """forge_runners.py, next to this file (written by the app with it)."""
    import importlib.util
    spec = importlib.util.spec_from_file_location('forge_runners', os.path.join(HERE, 'forge_runners.py'))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def job_run(d, as_model=None, out='model.glb', textures=None):
    if out not in ('model.glb', 'coarse.glb'):
        raise Fail('bad output name')
    ask = read_json(os.path.join(d, 'ask.json'), {})
    cat = catalog()
    m = model_of(as_model or ask['model'], cat)
    ctx = Ctx(d, m, cat, ask, out=out, textures=textures)
    timing = ctx.timing
    stage = ctx.stage
    t = time.time()
    if m['runner'] not in ('hunyuan3d2', 'triposr', 'sf3d'):
        res = load_runners().make(ctx)
        timing['shape'] = int(time.time() - t)
        if out == 'model.glb':
            write_json(os.path.join(d, 'result.json'), res)
        stage('done', 100)
        return res
    import torch
    from PIL import Image
    seed = ctx.seed
    torch.manual_seed(seed)
    vram = torch.cuda.get_device_properties(0).total_memory / 1e9 if torch.cuda.is_available() else 0
    photo = Image.open(ctx.photo)
    res, steps = ctx.res, ctx.steps
    tex = ctx.tex
    tsize = ctx.tsize
    out = ctx.out
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
                mesh = paint_mesh(mesh, img, tsize, vram, run.get('paint', 'hunyuan3d-paint-v2-0'))
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
    res = {'mesh': True, 'splat': False, 'shapeOnly': tex == 'none', 'note': ''}
    if ctx.out.endswith('model.glb'):
        write_json(os.path.join(d, 'result.json'), res)
    stage('done', 100)
    return res


def paint_mesh(mesh, img, tsize, vram, sub='hunyuan3d-paint-v2-0'):
    """Hunyuan3D 2.0's painter (hy3dgen.texgen): the shape's textures from the photo; on a card under 20 GB its parts
    moved to memory while the others work (its own enable_model_cpu_offload)."""
    from hy3dgen.texgen import Hunyuan3DPaintPipeline
    paint = Hunyuan3DPaintPipeline.from_pretrained('tencent/Hunyuan3D-2', subfolder=sub)
    if tsize != paint.config.texture_size:
        from hy3dgen.texgen.differentiable_renderer.mesh_render import MeshRender
        paint.config.texture_size = tsize
        paint.render = MeshRender(default_resolution=paint.config.render_size, texture_size=tsize)
    if vram < 20:
        paint.enable_model_cpu_offload()
    return paint(mesh, image=img)


def paint_run(d):
    """A shape-only model's shape painted by Hunyuan3D 2.0 (inside Hunyuan3D 2.0's own runtime): model.glb, from the
    cut-out photo (the background taken out by Hunyuan3D's own remover when the model didn't keep one)."""
    import torch
    import trimesh
    from PIL import Image
    ask = read_json(os.path.join(d, 'ask.json'), {})
    vram = torch.cuda.get_device_properties(0).total_memory / 1e9 if torch.cuda.is_available() else 0
    cut = os.path.join(d, 'photo-cut.png')
    if os.path.isfile(cut):
        img = Image.open(cut).convert('RGBA')
    else:
        from hy3dgen.rembg import BackgroundRemover
        img = Image.open(os.path.join(d, 'photo.png')).convert('RGBA')
        img = BackgroundRemover()(img.convert('RGB')) if img.getextrema()[3][0] == 255 else img
        img.save(cut)
    mesh = trimesh.load(os.path.join(d, 'model.glb'), force='mesh')
    mesh = paint_mesh(mesh, img, TEXSIZE.get(str(ask.get('textureSize', '2k')).lower(), 2048), vram)
    mesh.export(os.path.join(d, 'model.glb'))


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
    pj = {'v': pv, 'f': pf, 'c': pc, 'up': 'y'}
    splat = splat_preview(d)
    if splat:
        pj['s'], pj['sc'] = splat
    write_json(os.path.join(d, 'preview.json'), pj)
    if 'colour' in maps:
        derived = 'roughness' not in maps
        maps.update(derive_maps(maps, derived_from='colour'))
    for k, im in maps.items():
        try:
            im.copy().resize((256, 256)).save(os.path.join(d, 'map-%s.png' % k))
        except Exception:  # noqa: BLE001
            pass
    write_json(os.path.join(d, 'stats.json'), {'faces': len(faces), 'points': len(verts), 'size': [hi[i] - lo[i] for i in range(3)], 'closed': closed,
                                               'maps': sorted(maps), 'mapsWorkedOut': derived, 'splat': bool(splat), 'splatPoints': len(splat[0]) // 3 if splat else 0,
                                               'meshFromSplat': read_json(os.path.join(d, 'result.json'), {}).get('meshFromSplat') is True})


def splat_preview(d, most=20000):
    """The splat's points for the viewport's Splat view (at most 20 000, its solid ones, the right way up), or None."""
    sp = os.path.join(d, 'splat.ply')
    if not os.path.isfile(sp):
        return None
    up = 'y'
    try:
        m = model_of(read_json(os.path.join(d, 'ask.json'), {}).get('model', ''))
        up = (m.get('run') or {}).get('up', 'y')
    except Fail:
        pass
    try:
        pts = [p for p in ply_points(sp, limit=most * 2) if p[6] > 0.3][:most]
    except (Fail, OSError, struct.error, ValueError):
        return None
    if not pts:
        return None
    s = [round(c, 4) for p in pts for c in turn_up(p[:3], up)]
    sc = [int(max(0.0, min(1.0, c)) * 255) for p in pts for c in p[3:6]]
    return s, sc


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


# ------------------------------------------------------------------ splats (.ply): read with Python alone, and a mesh made from one

PLY_TYPES = {'char': 'b', 'int8': 'b', 'uchar': 'B', 'uint8': 'B', 'short': 'h', 'int16': 'h', 'ushort': 'H', 'uint16': 'H',
             'int': 'i', 'int32': 'i', 'uint': 'I', 'uint32': 'I', 'float': 'f', 'float32': 'f', 'double': 'd', 'float64': 'd'}
SH_C0 = 0.28209479177387814


def ply_points(path, limit=0):
    """A splat's (or a point cloud's) points: [(x, y, z, r, g, b, opacity)], colours 0..1. Gaussian splats keep their
    colour as the SH DC term (f_dc_0..2) and opacity as a logit; plain point clouds as red, green, blue (0..255).
    Binary little-endian or ASCII; only the vertex element is read. limit: every n-th point so at most that many."""
    with open(path, 'rb') as f:
        if f.readline().strip() != b'ply':
            raise Fail('not a .ply')
        fmt, props, count, el = '', [], 0, None
        while True:
            line = f.readline()
            if not line:
                raise Fail('a .ply without its header end')
            w = line.decode('ascii', 'replace').split()
            if not w:
                continue
            if w[0] == 'format':
                fmt = w[1]
            elif w[0] == 'element':
                el = w[1]
                if el == 'vertex':
                    count = int(w[2])
            elif w[0] == 'property' and el == 'vertex':
                if w[1] == 'list':
                    raise Fail('a .ply whose points have lists')
                props.append((w[2], PLY_TYPES.get(w[1], 'f')))
            elif w[0] == 'end_header':
                break
        names = [n for n, _ in props]
        if not {'x', 'y', 'z'} <= set(names):
            raise Fail('a .ply without points')
        step = max(1, count // limit) if limit else 1
        rows = []
        if fmt == 'binary_little_endian':
            st = struct.Struct('<' + ''.join(c for _, c in props))
            data = f.read(st.size * count)
            for k in range(0, min(count, len(data) // st.size), step):
                rows.append(st.unpack_from(data, k * st.size))
        elif fmt == 'ascii':
            for k, line in enumerate(f.read().decode('ascii', 'replace').splitlines()[:count]):
                if k % step == 0 and line.strip():
                    rows.append(tuple(float(x) for x in line.split()[:len(props)]))
        else:
            raise Fail('a .ply Hearth Forge can\'t read (%s)' % fmt)
    ix = {n: i for i, n in enumerate(names)}
    out = []
    for r in rows:
        if 'f_dc_0' in ix:
            col = tuple(min(1.0, max(0.0, 0.5 + SH_C0 * r[ix['f_dc_%d' % i]])) for i in range(3))
        elif 'red' in ix:
            col = tuple(r[ix[c]] / (255.0 if r[ix[c]] > 1.0 or props[ix[c]][1] in 'BbHh' else 1.0) for c in ('red', 'green', 'blue'))
        else:
            col = (0.8, 0.8, 0.8)
        op = 1.0 / (1.0 + math.exp(-max(-30.0, min(30.0, r[ix['opacity']])))) if 'opacity' in ix else 1.0
        out.append((r[ix['x']], r[ix['y']], r[ix['z']], col[0], col[1], col[2], op))
    return out


def turn_up(p, up):
    """Y up, as Forge shows and exports: a splat made in a camera's own frame (y down, looking along +z) is turned
    the right way up ("up": "-y")."""
    return (p[0], -p[1], -p[2]) if up == '-y' else (p[0], p[1], p[2])


SPLAT_GRID = {'draft': 64, 'good': 96, 'high': 128, 'highest': 160}


def splat_to_mesh(ply, glb, up='y', detail='high'):
    """A closed mesh with the splat's colours, so a splat can be sent to Godot, O3DE, Hearth CAD and saved like any
    model (the splat itself is kept beside it as splat.ply). The splat's solid points (opacity over 0.3, the farthest
    1 % on each side left out) are put in a grid, its pinholes closed (filled where 5 of a cell's 6 neighbours are
    full), and every face between a full and an empty cell becomes two triangles; the corners are then smoothed twice
    so it isn't blocky. Python alone: it runs in the service."""
    pts = [p for p in ply_points(ply, limit=600000) if p[6] > 0.3] or ply_points(ply, limit=600000)
    if len(pts) < 8:
        raise Fail('the splat has too few points to make a mesh')
    pts = [turn_up(p[:3], up) + p[3:6] for p in pts]
    lo, hi = [], []
    for i in range(3):
        v = sorted(p[i] for p in pts)
        lo.append(v[int(len(v) * 0.01)])
        hi.append(v[min(len(v) - 1, int(len(v) * 0.99))])
    span = max(hi[i] - lo[i] for i in range(3)) or 1.0
    # a cell for about every three points of its surface, at most Detail's grid: fine enough, without holes
    n = max(16, min(SPLAT_GRID.get(detail, 128), int(math.sqrt(len(pts) / 3.0))))
    cell = span / n
    dims = [max(1, int(math.ceil((hi[i] - lo[i]) / cell)) + 1) for i in range(3)]
    full, col = {}, {}
    for p in pts:
        k = tuple(int((p[i] - lo[i]) / cell) for i in range(3))
        if all(0 <= k[i] < dims[i] for i in range(3)):
            full[k] = full.get(k, 0) + 1
            c = col.setdefault(k, [0.0, 0.0, 0.0])
            c[0] += p[3]
            c[1] += p[4]
            c[2] += p[5]
    occ = set(full)
    nb = ((1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1))
    holes = {(k[0] + a, k[1] + b, k[2] + c) for k in occ for a, b, c in nb} - occ
    for h in holes:
        if sum((h[0] + a, h[1] + b, h[2] + c) in occ for a, b, c in nb) >= 5:
            occ.add(h)
    verts, vcol, index, faces = [], [], {}, []

    def corner(c, k):
        if c not in index:
            index[c] = len(verts)
            verts.append([lo[0] + c[0] * cell, lo[1] + c[1] * cell, lo[2] + c[2] * cell])
            s = col.get(k)
            w = full.get(k, 0)
            vcol.append([s[0] / w, s[1] / w, s[2] / w] if s and w else [0.6, 0.6, 0.6])
        return index[c]
    # the four corners of each side of a cell, wound to face out
    sides = {(1, 0, 0): ((1, 0, 0), (1, 1, 0), (1, 1, 1), (1, 0, 1)), (-1, 0, 0): ((0, 0, 0), (0, 0, 1), (0, 1, 1), (0, 1, 0)),
             (0, 1, 0): ((0, 1, 0), (0, 1, 1), (1, 1, 1), (1, 1, 0)), (0, -1, 0): ((0, 0, 0), (1, 0, 0), (1, 0, 1), (0, 0, 1)),
             (0, 0, 1): ((0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1)), (0, 0, -1): ((0, 0, 0), (0, 1, 0), (1, 1, 0), (1, 0, 0))}
    for k in sorted(occ):
        for d3, cs in sides.items():
            if (k[0] + d3[0], k[1] + d3[1], k[2] + d3[2]) in occ:
                continue
            q = [corner((k[0] + c[0], k[1] + c[1], k[2] + c[2]), k) for c in cs]
            faces.append((q[0], q[1], q[2]))
            faces.append((q[0], q[2], q[3]))
    if not faces:
        raise Fail('the splat made no surface')
    near = [set() for _ in verts]
    for a, b, c in faces:
        near[a].update((b, c))
        near[b].update((a, c))
        near[c].update((a, b))
    for _ in range(2):
        verts = [[sum(verts[j][i] for j in nb2) / len(nb2) * 0.5 + v[i] * 0.5 for i in range(3)] if nb2 else v for v, nb2 in zip(verts, near)]
    glb_write(glb, verts, faces, vcol)
    return len(faces)


def glb_write(path, verts, faces, cols=None):
    """A binary glTF of one mesh (points, their colours, its triangles), written with Python alone."""
    pos = b''.join(struct.pack('<3f', *v) for v in verts)
    colb = b''.join(struct.pack('<3f', *c) for c in cols) if cols else b''
    idx = b''.join(struct.pack('<3I', *f) for f in faces)
    lo = [min(v[i] for v in verts) for i in range(3)]
    hi = [max(v[i] for v in verts) for i in range(3)]
    views = [{'buffer': 0, 'byteOffset': 0, 'byteLength': len(pos)}]
    accs = [{'bufferView': 0, 'componentType': 5126, 'count': len(verts), 'type': 'VEC3', 'min': lo, 'max': hi}]
    attrs = {'POSITION': 0}
    off = len(pos)
    if colb:
        views.append({'buffer': 0, 'byteOffset': off, 'byteLength': len(colb)})
        accs.append({'bufferView': 1, 'componentType': 5126, 'count': len(cols), 'type': 'VEC3'})
        attrs['COLOR_0'] = 1
        off += len(colb)
    views.append({'buffer': 0, 'byteOffset': off, 'byteLength': len(idx)})
    accs.append({'bufferView': len(views) - 1, 'componentType': 5125, 'count': len(faces) * 3, 'type': 'SCALAR'})
    binb = pos + colb + idx
    js = {'asset': {'version': '2.0', 'generator': 'Hearth Forge'}, 'scene': 0, 'scenes': [{'nodes': [0]}], 'nodes': [{'mesh': 0}],
          'meshes': [{'primitives': [{'attributes': attrs, 'indices': len(accs) - 1, 'material': 0}]}],
          'materials': [{'pbrMetallicRoughness': {'baseColorFactor': [1, 1, 1, 1], 'metallicFactor': 0.0, 'roughnessFactor': 0.9}}],
          'buffers': [{'byteLength': len(binb)}], 'bufferViews': views, 'accessors': accs}
    j = json.dumps(js).encode()
    j += b' ' * ((4 - len(j) % 4) % 4)
    binb += b'\0' * ((4 - len(binb) % 4) % 4)
    tmp = path + '.part'
    with open(tmp, 'wb') as f:
        f.write(struct.pack('<III', 0x46546C67, 2, 28 + len(j) + len(binb)) + struct.pack('<II', len(j), 0x4E4F534A) + j + struct.pack('<II', len(binb), 0x004E4942) + binb)
    os.replace(tmp, path)


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
                           'runner': bool(m.get('runner')) and all(runner_ready(x, cat) for x in [m['runner']] + list(m.get('alsoRunners') or [])),
                           'getting': getting.get(m['id'])}
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
            opts = dict(zip(argv[2::2], argv[3::2]))
            job_run(argv[1], as_model=opts.get('--as'), out=opts.get('--out', 'model.glb'), textures=opts.get('--textures'))
        elif c == 'paint':
            paint_run(argv[1])
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
