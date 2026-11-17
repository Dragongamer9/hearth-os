#!/usr/bin/python3
"""hearth_desktop_apps: the apps on this PC that open a kind of thing (web links, pictures...), read from their
.desktop files, and starting one of them with a link or a file. Used by hearth-clipboard ("Open in…" for a copied
link) and hearth-capture-helper (Share › Open with…). Reads only; never changes which app is the default.

  apps_for(mime)          [{id, name, icon, default}] for a MIME type ("x-scheme-handler/https", "image/png"),
                          image/png also finds apps that say image/*; the default one first
  launch(app_id, target)  start that app with TARGET (a link or a file path), on its own (never waited on)
"""
import configparser
import os
import re
import shlex
import shutil
import subprocess

ID_RE = re.compile(r'^[A-Za-z0-9._+-]{1,200}\.desktop$')


def data_dirs(env=None):
    env = env if env is not None else os.environ
    home = env.get('XDG_DATA_HOME') or os.path.join(os.path.expanduser('~'), '.local', 'share')
    rest = (env.get('XDG_DATA_DIRS') or '/usr/local/share:/usr/share').split(':')
    out = []
    for d in [home] + rest + [os.path.join(os.path.expanduser('~'), '.local', 'share', 'flatpak', 'exports', 'share'),
                              '/var/lib/flatpak/exports/share']:
        if d and d not in out:
            out.append(d)
    return out


def read_entry(path):
    cp = configparser.ConfigParser(interpolation=None, strict=False)
    cp.optionxform = str
    try:
        with open(path, encoding='utf-8', errors='replace') as f:
            cp.read_string(f.read())
    except (OSError, configparser.Error, UnicodeError):
        return None
    if not cp.has_section('Desktop Entry'):
        return None
    e = cp['Desktop Entry']
    if e.get('Type', 'Application') != 'Application' or e.get('Hidden', '').lower() == 'true':
        return None
    if e.get('NoDisplay', '').lower() == 'true' or not e.get('Exec'):
        return None
    return {'name': e.get('Name', ''), 'icon': e.get('Icon', ''), 'exec': e.get('Exec', ''),
            'mime': [m.strip() for m in e.get('MimeType', '').split(';') if m.strip()]}


def entries(env=None):
    """Every app's entry by its id (the first one found wins, as the desktop does: your own folder first)."""
    found = {}
    for d in data_dirs(env):
        ad = os.path.join(d, 'applications')
        try:
            names = sorted(os.listdir(ad))
        except OSError:
            continue
        for n in names:
            if not n.endswith('.desktop') or n in found:
                continue
            e = read_entry(os.path.join(ad, n))
            if e:
                found[n] = e
    return found


def default_for(mime):
    if not shutil.which('xdg-mime'):
        return ''
    try:
        r = subprocess.run(['xdg-mime', 'query', 'default', mime], capture_output=True, text=True, timeout=5)
        return r.stdout.strip().split(';')[0] if r.returncode == 0 else ''
    except (OSError, subprocess.SubprocessError):
        return ''


def apps_for(mime, env=None, default=None):
    base = mime.split('/')[0] + '/*'
    want = {mime, base}
    if mime in ('x-scheme-handler/https', 'x-scheme-handler/http'):
        want |= {'x-scheme-handler/http', 'x-scheme-handler/https', 'text/html'}
    default = default_for(mime) if default is None else default
    out = []
    for aid, e in entries(env).items():
        if want & set(e['mime']):
            if mime.startswith('x-scheme-handler/http') and 'text/html' in e['mime'] and not (
                    {'x-scheme-handler/http', 'x-scheme-handler/https'} & set(e['mime'])):
                continue       # opens web pages from files only, not links (an editor)
            out.append({'id': aid, 'name': e['name'] or aid[:-8], 'icon': e['icon'], 'default': aid == default})
    out.sort(key=lambda a: (not a['default'], a['name'].lower()))
    return out


def command(exec_line, target):
    """The Exec line with its field codes filled in (%u %U %f %F get TARGET, the rest are dropped)."""
    try:
        parts = shlex.split(exec_line)
    except ValueError:
        parts = exec_line.split()
    out, used = [], False
    for p in parts:
        if p in ('%u', '%U', '%f', '%F'):
            out.append(target)
            used = True
        elif p in ('%i', '%c', '%k', '%d', '%D', '%n', '%N', '%v', '%m'):
            continue
        else:
            q = p.replace('%%', '\0')
            if any(c in q for c in ('%u', '%U', '%f', '%F')):
                q = re.sub(r'%[uUfF]', lambda _m: target, q)
                used = True
            q = re.sub(r'%[a-zA-Z]', '', q).replace('\0', '%')
            if q:
                out.append(q)
    if not used:
        out.append(target)
    return out


def launch(app_id, target, env=None):
    if not ID_RE.match(app_id or ''):
        raise ValueError('not an app')
    e = entries(env).get(app_id)
    if not e:
        raise ValueError("that app isn't installed any more")
    cmd = command(e['exec'], target)
    subprocess.Popen(cmd, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                     start_new_session=True, close_fds=True)
    return cmd
