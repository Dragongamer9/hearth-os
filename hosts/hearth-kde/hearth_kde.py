#!/usr/bin/python3
"""hearth_kde: the image-side pieces of Hearth's KDE apps (docs/KDE-APPS.md), used by install.sh at image build time
and by hearth-files-kde at run time.

  hearth_kde.py rebrand STOCK.desktop NAME ICON [EXEC]   KDE's own menu entry kept for the window's app id (so the
                                                        taskbar shows Hearth's name and icon), hidden from the menu
                                                        (Hearth's own entry is the one shown), EXEC replacing its program
  hearth_kde.py classic HEARTH.desktop                   the old Hearth app's entry, named "… (classic)" (still installed)
  hearth_kde.py places                                   Hearth Storage in KDE's Places (every KDE app's file dialogs and
                                                        Hearth Files), from Hearth Link; nothing without Hearth Link
"""
import json
import os
import re
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET

CLASSIC = ' (classic)'


# ---------------------------------------------------------------- menu entries
def _groups(text):
    """[(header or None, [lines])] in order."""
    out = [(None, [])]
    for line in text.splitlines():
        if line.startswith('[') and line.rstrip().endswith(']'):
            out.append((line.strip(), []))
        else:
            out[-1][1].append(line)
    return out


def _join(groups):
    lines = []
    for head, body in groups:
        if head is not None:
            lines.append(head)
        lines.extend(body)
    return '\n'.join(lines).rstrip('\n') + '\n'


def _write(path, text):
    d = os.path.dirname(path) or '.'
    fd, tmp = tempfile.mkstemp(dir=d, prefix='.hearth-kde-')
    with os.fdopen(fd, 'w') as f:
        f.write(text)
    os.chmod(tmp, 0o644)
    os.replace(tmp, path)


def rebrand_text(text, name, icon, exec_=None, program=None):
    """KDE's entry with Hearth's name and icon, hidden from menus; exec_ replaces `program` in its Exec lines."""
    groups = _groups(text)
    out = []
    for head, body in groups:
        new = []
        for line in body:
            key = line.split('=', 1)[0].strip()
            if head == '[Desktop Entry]':
                # Translations of the KDE name would show instead of Hearth's in other languages.
                if re.fullmatch(r'(Name|GenericName|Icon|NoDisplay|TryExec)(\[[^]]*\])?', key) and key not in ('GenericName',):
                    continue
            if exec_ and key == 'Exec' and program:
                cmd = line.split('=', 1)[1].strip()
                first = cmd.split(' ', 1)[0]
                if os.path.basename(first) == program:
                    line = 'Exec=' + exec_ + (' ' + cmd.split(' ', 1)[1] if ' ' in cmd else '')
            new.append(line)
        if head == '[Desktop Entry]':
            new = ['Name=' + name, 'Icon=' + icon, 'NoDisplay=true',
                   '# Hearth OS: this KDE app is shown as Hearth\'s own entry (hosts/hearth-kde, docs/KDE-APPS.md)'] + new
        out.append((head, new))
    return _join(out)


def rebrand(path, name, icon, exec_=None):
    with open(path) as f:
        text = f.read()
    program = None
    m = re.search(r'^Exec=(\S+)', text, re.M)
    if m:
        program = os.path.basename(m.group(1))
    _write(path, rebrand_text(text, name, icon, exec_, program))


def classic_text(text):
    groups = _groups(text)
    out = []
    for head, body in groups:
        new = []
        for line in body:
            if head == '[Desktop Entry]' and line.startswith('Name=') and not line.endswith(CLASSIC):
                line += CLASSIC
            new.append(line)
        out.append((head, new))
    return _join(out)


def classic(path):
    with open(path) as f:
        text = f.read()
    _write(path, classic_text(text))


# ---------------------------------------------------------------- Places (KDE's user-places.xbel)
NS = {'bookmark': 'http://www.freedesktop.org/standards/desktop-bookmarks',
      'mime': 'http://www.freedesktop.org/standards/shared-mime-info',
      'kdepriv': 'http://www.kde.org/kdepriv'}
for _p, _u in NS.items():
    ET.register_namespace(_p, _u)
FDO = 'http://freedesktop.org'
KDE = 'http://www.kde.org'
ID_PREFIX = 'hearth-storage'
LINK_BIN = os.environ.get('HEARTH_LINK_BIN', '/usr/bin/hearth-link')


def places_file(home=None):
    data = os.environ.get('XDG_DATA_HOME') or os.path.join(home or os.path.expanduser('~'), '.local/share')
    return os.path.join(data, 'user-places.xbel')


def _bookmark(href, title, icon, pid):
    b = ET.Element('bookmark', {'href': href})
    ET.SubElement(b, 'title').text = title
    info = ET.SubElement(b, 'info')
    m1 = ET.SubElement(info, 'metadata', {'owner': FDO})
    ET.SubElement(m1, '{%s}icon' % NS['bookmark'], {'name': icon})
    m2 = ET.SubElement(info, 'metadata', {'owner': KDE})
    ET.SubElement(m2, 'ID').text = pid
    ET.SubElement(m2, 'isSystemItem').text = 'false'
    return b


def _pid(b):
    for md in b.iter('metadata'):
        if md.get('owner') == KDE:
            i = md.find('ID')
            if i is not None and i.text:
                return i.text
    return ''


def default_places(home):
    """What KDE would start a new Places list with (it adds the rest itself), so a first Hearth Files start isn't bare."""
    out = []
    for i, (rel, title, icon) in enumerate([('', 'Home', 'user-home'), ('Desktop', 'Desktop', 'user-desktop'),
                                             ('Documents', 'Documents', 'folder-documents'),
                                             ('Downloads', 'Downloads', 'folder-downloads')]):
        out.append(_bookmark('file://' + os.path.join(home, rel).rstrip('/'), title, icon, 'hearth-default/%d' % i))
    out.append(_bookmark('trash:/', 'Trash', 'user-trash', 'hearth-default/trash'))
    return out


def storage_places(status):
    """[(href, title, icon, id)]: Hearth Storage itself, then each shared folder Hearth Link knows."""
    out = [('hearth:/', 'Hearth Storage', 'network-server', ID_PREFIX)]
    for sv in (status or {}).get('servers') or []:
        sid = str(sv.get('id') or '').lower()
        if not re.fullmatch(r'[0-9a-f]{16,64}', sid):
            continue
        for sh in sv.get('shares') or []:
            if not isinstance(sh, dict):
                continue
            shid = str(sh.get('id') or '').lower()
            if not re.fullmatch(r'[0-9a-f]{1,32}', shid):
                continue
            title = '%s on %s' % (sh.get('name') or 'Shared', sv.get('name') or 'Hearth Storage')
            out.append(('hearth:/%s/%s' % (sid, shid), title, 'folder-network', '%s/%s/%s' % (ID_PREFIX, sid, shid)))
    return out


def sync_places_xml(text, wanted, home, added_before=()):
    """(the places file's new text, or None when nothing changes; the ids Hearth has added so far).
    Hearth Storage's places as `wanted` says: ours updated, ones whose shared folder is gone removed, yours untouched.
    A place Hearth added before and you removed stays removed (added_before remembers what Hearth added)."""
    added = set(added_before)
    if text:
        try:
            root = ET.fromstring(text)
        except ET.ParseError:
            return None, added   # never rewrite a file that can't be read
    else:
        root = ET.Element('xbel')
        for b in default_places(home):
            root.append(b)
    changed = not text
    mine = {}
    for b in list(root.findall('bookmark')):
        pid = _pid(b)
        if pid.startswith(ID_PREFIX):
            mine[pid] = b
    want_ids = {w[3] for w in wanted}
    for pid, b in mine.items():
        if pid not in want_ids:
            root.remove(b)
            added.discard(pid)
            changed = True
    for pid in list(added):
        if pid not in want_ids:
            added.discard(pid)   # its shared folder is gone: if it comes back, it's added again
    for href, title, icon, pid in wanted:
        b = mine.get(pid)
        if b is None:
            if pid in added:
                continue       # you removed it
            root.append(_bookmark(href, title, icon, pid))
            added.add(pid)
            changed = True
        else:
            added.add(pid)
            t = b.find('title')
            if b.get('href') != href or t is None or t.text != title:
                b.set('href', href)
                if t is None:
                    t = ET.SubElement(b, 'title')
                t.text = title
                changed = True
    if not changed:
        return None, added
    return '<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE xbel>\n' + ET.tostring(root, encoding='unicode') + '\n', added


def state_file(home):
    cfg = os.environ.get('XDG_CONFIG_HOME') or os.path.join(home, '.config')
    return os.path.join(cfg, 'nimbus', 'hearth-files-kde.json')


def link_status(timeout=4):
    if not os.path.exists(LINK_BIN):
        return None
    try:
        r = subprocess.run([LINK_BIN, 'call', 'storage.status', '{}'], capture_output=True, text=True, timeout=timeout,
                           env=dict(os.environ, HEARTH_LINK_CALL_TIMEOUT=str(timeout)))
        lines = (r.stdout or '').strip().splitlines()
        v = json.loads(lines[-1]) if lines else None
        return v if isinstance(v, dict) and 'error' not in v else {}
    except (OSError, ValueError, subprocess.TimeoutExpired):
        return {}


def sync_places(home=None):
    """Hearth Storage in Places. Without Hearth Link (not installed): nothing. Hearth Link not answering: Hearth Storage
    itself only, shares left as they were."""
    home = home or os.path.expanduser('~')
    status = link_status()
    if status is None:
        return {'ok': True, 'skipped': 'no Hearth Link'}
    path = places_file(home)
    try:
        with open(path) as f:
            text = f.read()
    except FileNotFoundError:
        text = ''
    except OSError as e:
        return {'ok': False, 'error': str(e)}
    wanted = storage_places(status)
    if not status:
        # Not answering: keep the shares already there, so a sleeping server's places don't come and go.
        try:
            root = ET.fromstring(text) if text else None
        except ET.ParseError:
            root = None
        if root is not None:
            for b in root.findall('bookmark'):
                pid = _pid(b)
                if pid.startswith(ID_PREFIX + '/'):
                    t = b.find('title')
                    wanted.append((b.get('href'), t.text if t is not None else '', 'folder-network', pid))
    sf = state_file(home)
    try:
        with open(sf) as f:
            before = set(json.load(f).get('added') or [])
    except (OSError, ValueError, AttributeError):
        before = set()
    new, added = sync_places_xml(text, wanted, home, before)
    if added != before:
        os.makedirs(os.path.dirname(sf), exist_ok=True)
        _write(sf, json.dumps({'added': sorted(added)}))
    if new is None:
        return {'ok': True, 'changed': False}
    os.makedirs(os.path.dirname(path), exist_ok=True)
    if text and not os.path.exists(path + '.hearth-bak'):
        with open(path + '.hearth-bak', 'w') as f:
            f.write(text)
    _write(path, new)
    return {'ok': True, 'changed': True, 'places': len(wanted)}


def main(argv):
    c = argv[1] if len(argv) > 1 else ''
    if c == 'rebrand' and len(argv) in (5, 6):
        rebrand(argv[2], argv[3], argv[4], argv[5] if len(argv) == 6 else None)
    elif c == 'classic' and len(argv) == 3:
        classic(argv[2])
    elif c == 'places':
        print(json.dumps(sync_places()))
    else:
        print(__doc__, file=sys.stderr)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
