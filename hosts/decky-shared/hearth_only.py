"""Shared by Hearth's Decky plugins (Hearth Sync, Hearth Link, Hearth Music; docs/PLUGIN-UPDATES.md).

1. Hearth OS only. The plugins are part of Hearth OS: each backend asks `check()` when Decky loads it, and on any other
   system it starts none of its features; the frontend shows one panel about Hearth OS instead. This is a check, not
   DRM: the code is public and anyone can change it. Nothing is hidden, nothing is sent anywhere, no device identity.
   Hearth OS is: /etc/os-release (or /usr/lib/os-release) with NAME=Hearth or a VARIANT_ID starting with "hearth",
   plus one of Hearth's own files (the built-in Decky service or /usr/libexec/nimbus).

2. The load mark for plugin updates (hearth-decky plugins-update): a plugin whose backend has run for LOADED_AFTER
   seconds writes hearth-loaded.json (its version) into its Decky data folder. hearth-decky puts the copy before a
   downloaded update back when the update's mark doesn't appear.

Each package.sh copies this file next to main.py; main.py also finds it here (hosts/decky-shared) in the repo's tests.
Paths can be moved for tests with HEARTH_ONLY_ROOT (a folder standing in for /).
"""
import json
import os
import time

OS_RELEASE = ('etc/os-release', 'usr/lib/os-release')
MARKERS = ('usr/lib/systemd/system/hearth-decky@.service', 'usr/libexec/nimbus')
LOADED_AFTER = 30
MESSAGE = 'This plugin is part of Hearth OS. Get Hearth OS at hearthos.fun'


def _root():
    return os.environ.get('HEARTH_ONLY_ROOT', '/')


def _path(rel):
    return os.path.join(_root(), rel)


def parse_os_release(text):
    out = {}
    for line in (text or '').splitlines():
        line = line.strip()
        if not line or line.startswith('#') or '=' not in line:
            continue
        k, v = line.split('=', 1)
        v = v.strip()
        if len(v) >= 2 and v[0] == v[-1] and v[0] in '"\'':
            v = v[1:-1]
        out[k.strip()] = v
    return out


def is_hearth_release(fields):
    """Hearth's os-release: NAME=Hearth, or VARIANT_ID hearth-… (os-bases/peppermint/build.sh writes both)."""
    return (fields.get('NAME') or '').strip() == 'Hearth' or (fields.get('VARIANT_ID') or '').strip().lower().startswith('hearth')


def check():
    """(True, '') on Hearth OS; otherwise (False, why). Never raises."""
    fields = None
    for rel in OS_RELEASE:
        try:
            with open(_path(rel), encoding='utf-8', errors='replace') as f:
                fields = parse_os_release(f.read(64 * 1024))
            break
        except OSError:
            continue
    if fields is None:
        return False, 'no os-release'
    if not is_hearth_release(fields):
        return False, 'not Hearth OS (%s)' % (fields.get('PRETTY_NAME') or fields.get('NAME') or '?')[:80]
    if not any(os.path.exists(_path(m)) for m in MARKERS):
        return False, 'Hearth OS name without Hearth\'s own files'
    return True, ''


_said = set()


def status(log=None, name='plugin'):
    """What the frontend's hearth_os call gets: {ok, message}. Logs the reason once per plugin load."""
    ok, why = check()
    if not ok and name not in _said and log:
        _said.add(name)
        try:
            log('%s: %s; its features stay off. %s' % (name, why, MESSAGE))
        except Exception:
            pass
    return {'ok': ok, 'message': '' if ok else MESSAGE, 'why': why}


def plugin_version(plugin_dir):
    try:
        with open(os.path.join(plugin_dir, 'package.json')) as f:
            v = json.load(f).get('version')
        return v if isinstance(v, str) else ''
    except (OSError, ValueError, AttributeError):
        return ''


def write_loaded(data_dir, version, now=None):
    """hearth-loaded.json in the plugin's Decky data folder: {version, at}. Atomic; errors are ignored."""
    if not data_dir:
        return False
    try:
        os.makedirs(data_dir, exist_ok=True)
        p = os.path.join(data_dir, 'hearth-loaded.json')
        tmp = '%s.%d.tmp' % (p, os.getpid())
        with open(tmp, 'w') as f:
            json.dump({'version': version, 'at': int(now if now is not None else time.time())}, f)
            f.flush()
            os.fsync(f.fileno())
        os.replace(tmp, p)
        return True
    except OSError:
        return False


async def mark_loaded_later(decky, delay=LOADED_AFTER):
    """Run from _main as a task: after `delay` seconds of running, the load mark (only on Hearth OS)."""
    import asyncio
    try:
        await asyncio.sleep(delay)
        if not check()[0]:
            return
        pdir = getattr(decky, 'DECKY_PLUGIN_DIR', None) if decky else None
        ddir = getattr(decky, 'DECKY_PLUGIN_RUNTIME_DIR', None) if decky else None
        write_loaded(ddir, plugin_version(pdir) if pdir else '')
    except Exception:
        pass
