"""Archived with KWin (1.15, the owner, Nov 17): hearth-your-desktop's KWin part (nimbus-session ran
`hearth-your-desktop kwin-overrides` before KWin started). Kept as it was; not run. docs/archive/KWIN.md"""
MINE_TOLD = os.path.join(DESK_STATE, 'kwin-mine-told')
# Hearth's KWin keys that are fixed ([$i], KConfig's "immutable", in desktop/kwin/xdg and desktop/kwin/windows/xdg):
# your own ~/.config file can't change them under KWin, because Mantel can't work without them (red flag 2: KWin
# locking by itself skips hearth-lockcheck, a lockout on a handheld if its lock screen can't load). Only these;
# Night Light and tearing (Settings › Display, written to your kwinrc) are never fixed.
#   (id, file, group, key, Hearth's value, what it is in KDE, what decides it in Hearth; '' = not told: no effect alone)
LOCKED = [
    ('autolock', 'kscreenlockerrc', 'Daemon', 'Autolock', 'false', 'Lock the screen automatically',
     'Mantel locks when it\'s idle as Settings › Sign-in says'),
    ('lockonresume', 'kscreenlockerrc', 'Daemon', 'LockOnResume', 'false', 'Lock after waking from sleep',
     'Settings › Sign-in › Lock when waking decides it'),
    ('locktimeout', 'kscreenlockerrc', 'Daemon', 'Timeout', '0', '', ''),
    ('inputmethod', 'kwinrc', 'Wayland', 'InputMethod', '/usr/share/applications/hearth-kwin-im.desktop',
     'The virtual keyboard you picked in KDE', 'Hearth\'s on-screen keyboard types for you (Settings › Keyboard)'),
    ('superkey', 'kwinrc', 'ModifierOnlyShortcuts', 'Meta', 'org.hearth.Windows,/org/hearth/Windows,org.hearth.Windows,Launcher',
     'What the Super key opens on its own', 'it opens Mantel\'s app menu (Settings › Desktop › App menu picks which)'),
]

def ini_values(text):
    """{(group, key): value} of a KDE ini file (the last one wins, a key's [$e] / [$i] options dropped)."""
    out, group = {}, ''
    for raw in text.splitlines():
        s = raw.strip()
        if not s or s.startswith('#'):
            continue
        if s.startswith('['):
            s = re.sub(r'\[\$[a-z]*\]$', '', s)              # a group's own [$i]
            if s.startswith('[') and s.endswith(']'):
                group = s[1:-1].replace('][', '/')
            continue
        if '=' in s:
            k, v = s.split('=', 1)
            out[(group, re.sub(r'\[[^\]]*\]', '', k).strip())] = v.strip()
    return out


def overridden(config_home=None):
    """The ids in LOCKED whose key your own file sets to something other than Hearth's value."""
    config_home = config_home or CONFIG_HOME
    files, out = {}, []
    for lid, fname, group, key, value, _what, _how in LOCKED:
        if fname not in files:
            try:
                with open(os.path.join(config_home, fname), encoding='utf-8', errors='replace') as f:
                    files[fname] = ini_values(f.read())
            except OSError:
                files[fname] = {}
        mine = files[fname].get((group, key))
        if mine is not None and mine.lower() != value.lower():
            out.append(lid)
    return out


def cmd_kwin_overrides():
    """Queue the one-time notice for overridden keys not told yet. They're marked as told only after the notice is
    really in the notice file (read back under its lock); if queueing fails nothing is marked, so the next start
    tells again (red flag 9: the person is always told)."""
    ids = overridden()
    told = [i for i in (sio.load_json(MINE_TOLD, {}, repair=False).get('told') or []) if isinstance(i, str)] \
        if os.path.exists(MINE_TOLD) else []
    tell = [i for i in ids if i not in told and any(x[0] == i and x[5] for x in LOCKED)]
    if not tell:
        return {'overridden': ids, 'told': False}
    shown = [i for i in ids if any(x[0] == i and x[5] for x in LOCKED)]
    n = parse('kwin-mine %s %d' % ('+'.join(shown), int(time.time())))
    with sio.locked(NOTICE):
        rest = [x for x in read_notices(NOTICE) if x['what'] != 'kwin-mine']
        write_lines(NOTICE, rest + [n])
        queued = any(x['what'] == 'kwin-mine' and x['kind'] == n['kind'] for x in read_notices(NOTICE))
    if not queued:
        sio.log_event('desktop: the notice about your KDE settings couldn\'t be queued; told at the next start')
        return {'overridden': ids, 'told': False}
    sio.write_json(MINE_TOLD, {'told': sorted(set(told) | set(tell))})
    sio.log_event('desktop: your own KDE settings not used under KWin: %s' % ', '.join(shown))
    return {'overridden': ids, 'told': True}


