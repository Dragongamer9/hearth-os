"""Settings must always be remembered and never change by themselves (docs/SETTINGS-PERSISTENCE.md).

Tests for desktop/shell/hearth_settings_io.py (the safe writer Python, QML and shell use), the session's
strip_mode_args, and a repo-wide check that flags new non-atomic writes of known settings files.
"""
import sys
sys.dont_write_bytecode = True
import json
import multiprocessing
import os
import pathlib
import re
import subprocess
import tempfile
import unittest

os.environ['XDG_STATE_HOME'] = tempfile.mkdtemp()        # notices stay out of your home
os.environ.pop('DBUS_SESSION_BUS_ADDRESS', None)

REPO = pathlib.Path(__file__).resolve().parents[3]
SHELL = REPO / 'hosts' / 'linux' / 'desktop' / 'shell'
sys.path.insert(0, str(SHELL))
import hearth_settings_io as io   # noqa: E402


def _writer(path, key, n):
    for i in range(n):
        io.merge_json(path, {key: i})


class SafeWrites(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.f = os.path.join(self.tmp.name, 'nimbus', 'desktop.json')

    def tearDown(self):
        self.tmp.cleanup()

    def test_atomic_with_bak_and_no_temporary_files(self):
        io.write_json(self.f, {'look': 'hearth'})
        io.write_json(self.f, {'look': 'steam'})
        self.assertEqual(io.load_json(self.f), {'look': 'steam'})
        self.assertEqual(json.load(open(self.f + '.bak')), {'look': 'hearth'})
        self.assertEqual([n for n in os.listdir(os.path.dirname(self.f)) if n.endswith('.tmp')], [])

    def test_crash_mid_write_is_restored_from_bak_and_defaults_never_saved_over(self):
        io.write_json(self.f, {'look': 'hearth', 'nightLight': True})
        io.write_json(self.f, {'look': 'hearth', 'nightLight': True, 'clock24': True})
        for damage in (b'', b'{"look": "hea', b'\x00\x00\x00', b'[1, 2]'):
            with open(self.f, 'wb') as fh:          # what a power cut leaves after a truncate-then-write
                fh.write(damage)
            got = io.load_json(self.f, {'look': 'steam'})
            self.assertEqual(got['look'], 'hearth', damage)
            self.assertTrue(got['nightLight'])
            self.assertEqual(json.load(open(self.f))['look'], 'hearth')      # put back on disk
        # A change made while the file is damaged keeps everything else (merge, never defaults + the change).
        with open(self.f, 'wb') as fh:
            fh.write(b'')
        io.merge_json(self.f, {'mode': 'light'})
        d = json.load(open(self.f))
        self.assertEqual((d['look'], d['nightLight'], d['mode']), ('hearth', True, 'light'))

    def test_no_good_copy_keeps_the_damaged_file_and_tells_once(self):
        os.makedirs(os.path.dirname(self.f))
        with open(self.f, 'w') as fh:
            fh.write('{broken')
        self.assertEqual(io.load_json(self.f, {'look': 'steam'}), {'look': 'steam'})
        self.assertEqual(open(self.f).read(), '{broken')                     # not overwritten by reading
        kept = [n for n in os.listdir(os.path.dirname(self.f)) if n.startswith('desktop.json.broken-')]
        self.assertEqual(len(kept), 1)
        log = open(os.path.join(io.state_dir(), 'settings-events.log')).read()
        self.assertIn("couldn't be read", log)
        self.assertFalse(io.notice('broken:%s:x' % self.f, 'a') and io.notice('broken:%s:x' % self.f, 'a'))

    def test_two_processes_changing_different_keys_both_keep_theirs(self):
        io.write_json(self.f, {'look': 'steam'})
        ctx = multiprocessing.get_context('fork')
        ps = [ctx.Process(target=_writer, args=(self.f, 'k%d' % i, 40)) for i in range(4)]
        for p in ps:
            p.start()
        for p in ps:
            p.join(60)
        d = json.load(open(self.f))
        self.assertEqual(d['look'], 'steam')
        for i in range(4):
            self.assertEqual(d['k%d' % i], 39)

    def test_defaults_fill_missing_keys_only(self):
        saved = {'steamBackground': False, 'sources': {'xbox': False}, 'futureKey': 1}
        d = io.with_defaults(saved, {'steamBackground': True, 'autoUpdateCheck': True, 'sources': {'xbox': True, 'steam': True}})
        self.assertEqual(d, {'steamBackground': False, 'autoUpdateCheck': True, 'sources': {'xbox': False, 'steam': True}, 'futureKey': 1})

    def test_upgrade_keeps_values_and_unknown_keys(self):
        # A desktop.json from an older version (fewer keys, one this version doesn't know): a change merges in.
        os.makedirs(os.path.dirname(self.f))
        with open(self.f, 'w') as fh:
            json.dump({'wallpaper': '/home/p/Café.jpg', 'oldThing': [1]}, fh)
        io.merge_json(self.f, {'look': 'hearth'})
        d = json.load(open(self.f, encoding='utf-8'))
        self.assertEqual(d, {'wallpaper': '/home/p/Café.jpg', 'oldThing': [1], 'look': 'hearth'})
        self.assertIn('Café', pathlib.Path(self.f).read_text(encoding='utf-8'))          # UTF-8, not \\u escapes

    def test_cli_merge_get_and_null_removes(self):
        py = [sys.executable, str(SHELL / 'hearth_settings_io.py')]
        subprocess.run(py + ['merge', self.f, '{"nightLight": true, "x": 1}'], check=True)
        subprocess.run(py + ['merge', self.f, '{"x": null}'], check=True)
        out = subprocess.run(py + ['get', self.f], check=True, capture_output=True, text=True).stdout
        self.assertEqual(json.loads(out), {'nightLight': True})


class ConfFiles(unittest.TestCase):
    def test_conf_set_keeps_every_other_line(self):
        text = '# my notes\nexport HEARTH_STEAM_DEFAULT=1\nNIMBUS_START_MODE=game\nNIMBUS_GAMESCOPE_ARGS="-W 1280"\n'
        out = io.conf_set_text(text, 'NIMBUS_START_MODE', 'desktop')
        self.assertEqual(out, text.replace('NIMBUS_START_MODE=game', 'NIMBUS_START_MODE=desktop'))
        self.assertTrue(io.conf_set_text('A=1', 'B', '2').endswith('A=1\nB=2\n'))

    def test_conf_strip_touches_only_that_key(self):
        rx = r'(^|[\s"])(-[WHr]\s+|--(output-width|output-height|nested-refresh)[=\s]+)[^\s"]+'
        text = '# -W 5 stays in a comment\nOTHER="-r 9"\nNIMBUS_GAMESCOPE_ARGS="--hdr-enabled -W 2560 -H 1440 -r 120"\n'
        out = io.conf_strip_text(text, 'NIMBUS_GAMESCOPE_ARGS', rx)
        self.assertIn('# -W 5 stays in a comment\nOTHER="-r 9"\n', out)
        self.assertNotIn('2560', out)
        self.assertIn('--hdr-enabled', out)

    def test_session_strip_mode_args_is_safe_and_remembers_your_choice(self):
        src = (REPO / 'hosts' / 'linux' / 'session' / 'nimbus-session').read_text()
        regex = re.search(r"^MODE_ARGS_RE='[^']*'$", src, re.M).group(0)
        fn = re.search(r'^strip_mode_args\(\) \{\n.*?^\}\n', src, re.M | re.S).group(0)
        fn += re.search(r'^note_reverted\(\) \{\n.*?^\}\n', src, re.M | re.S).group(0)
        with tempfile.TemporaryDirectory() as d:
            conf = os.path.join(d, 'nimbus', 'session.conf')
            os.makedirs(os.path.dirname(conf))
            before = '# mine\nHEARTH_STEAM_DEFAULT=1\nexport NIMBUS_GAMESCOPE_ARGS="--hdr-enabled -W 2560 -H 1440 -r 120"\nNIMBUS_START_MODE=desktop\n'
            pathlib.Path(conf).write_text(before)
            script = '%s\nSESSION_CONF=%s\n%s\nstrip_mode_args "--hdr-enabled -W 2560 -H 1440 -r 120"\n' % (regex, conf, fn)
            subprocess.run(['bash', '-c', script], check=True)
            after = re.sub(r'\s+"', '"', pathlib.Path(conf).read_text())     # the session's sed leaves the spaces
            self.assertEqual(after, before.replace(' -W 2560 -H 1440 -r 120', ''))
            self.assertEqual(pathlib.Path(conf + '.bak').read_text(), before)
            rev = json.load(open(os.path.join(d, 'nimbus', 'display-reverted.json')))
            self.assertEqual(rev['args'], '--hdr-enabled -W 2560 -H 1440 -r 120')
            self.assertEqual([n for n in os.listdir(os.path.dirname(conf)) if n.startswith('.session.conf.') and not n.endswith('.lock')], [])


# ---------------------------------------------------------------------------------------------------- repo check
# Known settings files: writing one must go through the safe helpers (settings_file.rs, hearth_settings_io.py,
# SettingsWriter.qml, settings-file.js), never a truncate-then-write, a shared .tmp name or a whole-object rewrite.
KNOWN = ('desktop.json', 'session.conf', 'library.json', 'osk.json', 'buttons.json', 'profiles.json', 'controller.json',
         'accessibility.json', 'keyboard.json', 'pointer.json', 'desk.json', 'files.json', 'picker.json', 'game-mode-ui.json',
         'home-server.json', 'ai-share.json', 'taskmgr.json')
SKIP_DIRS = {'archive', 'node_modules', 'target', '.git', 'vendor', 'site', 'docs'}
SKIP_PARTS = ('os-bases/ubuntu', 'os-bases/cachyos', '/test/', '/tests/', 'hosts/android/')
# Places that still do it, each with its reason (fix them and take them off; never add to this list lightly).
ALLOWED = {
    # The recovery menu (reached from the boot menu): a booting file, left for the owner to change.
    'os/system_files/usr/libexec/nimbus/hearth-recovery',
    'os/recovery/src/hearth-recovery',
    # The live installer writes a brand-new account's first files once (nothing to lose yet).
    'os/live/src/hearth-install',
    # Building the live USB image: /etc/skel's first session.conf, before anyone has a setting.
    'os/live/src/build.sh',
    # The manual's "fix the display mode" command, typed by the person themselves (GNU sed -i renames a new file in).
    'tools/build-docs.py',
}


def _files():
    for root, dirs, files in os.walk(REPO):
        dirs[:] = [d for d in dirs if d not in SKIP_DIRS]
        rel_root = os.path.relpath(root, REPO)
        for n in files:
            rel = os.path.normpath(os.path.join(rel_root, n))
            if any(p in '/' + rel + '/' for p in SKIP_PARTS) or rel in ALLOWED:
                continue
            if n.endswith(('.rs', '.py', '.qml', '.js', '.sh')) or ('.' not in n and (rel.startswith('hosts/linux/') or rel.startswith('os/'))):
                yield rel


def _scan():
    names = '|'.join(re.escape(k) for k in KNOWN)
    known_fn = r'session_conf\(\)|desktop_json\(\)|settings_path\(\)|SESSION_CONF\b|SETTINGS_JSON\b|KEYBOARD_JSON\b|POINTER_JSON\b'
    rules = [
        ('sed -i on a settings file', re.compile(r'sed\s+-i\b.*(' + names + r'|SESSION_CONF)')),
        ('shell redirect over a settings file', re.compile(r'(?<![0-9&])>\s*"?[^"\s|&;]*(' + names + r')"?\s*(\)|;|$|&&|\|\|)')),
        ('Rust fs::write / File::create of a settings file', re.compile(r'(fs::write|File::create)\(.*(' + names + '|' + known_fn + ')')),
        ('Rust paths::write_atomic of a settings file (use settings_file)', re.compile(r'paths::write_atomic\(.*(' + names + '|' + known_fn + ')')),
        ("Python open(..., 'w') of a settings file", re.compile(r"open\(.*(" + names + '|' + known_fn + r").*['\"]w['\"]")),
        ('Node writeFileSync of a settings file', re.compile(r'writeFileSync\(.*(' + names + r'|desktopFile|session)')),
    ]
    hits = []
    fileviews = re.compile(r'FileView\s*\{[^{}]*?id:\s*(\w+)[^{}]*?path:[^\n]*(' + names + r')', re.S)
    for rel in _files():
        try:
            text = (REPO / rel).read_text(encoding='utf-8', errors='replace')
        except OSError:
            continue
        if rel.endswith('.qml'):
            for m in fileviews.finditer(text):
                if re.search(r'\b' + m.group(1) + r'\.setText\(', text):
                    hits.append('%s: QML FileView %s.setText() on %s (use SettingsWriter.merge)' % (rel, m.group(1), m.group(2)))
            continue
        if rel.endswith('.rs') and '#[cfg(test)]' in text:
            text = text[:text.index('#[cfg(test)]')]          # tests write fixtures however they like
        for i, line in enumerate(text.splitlines(), 1):
            s = line.strip()
            if s.startswith(('#', '//', '*')):
                continue
            for what, rx in rules:
                if rx.search(line):
                    hits.append('%s:%d: %s: %s' % (rel, i, what, s[:140]))
    return hits


class NoNewUnsafeWrites(unittest.TestCase):
    def test_known_settings_files_are_only_written_safely(self):
        hits = _scan()
        for h in hits:
            print('::error title=unsafe settings write::' + h)
        self.assertEqual(hits, [], 'Settings files must be written through the safe helpers (docs/SETTINGS-PERSISTENCE.md)')

    def test_the_check_catches_the_old_patterns(self):
        rx = re.compile(r'sed\s+-i\b.*(session\.conf|SESSION_CONF)')
        self.assertTrue(rx.search('[ -f "$SESSION_CONF" ] && sed -i -E "s/x//" "$SESSION_CONF"'))

    def test_hearth_app_ids_never_change(self):
        # A Hearth app's WebKit data (its localStorage: last tab, tool, sizes...) lives in ~/.local/share/<identifier>:
        # a new identifier in an update would start every person's app from scratch.
        ids = {}
        for conf in sorted((REPO / 'hosts').glob('*/tauri.conf.json')):
            ids[conf.parent.name] = json.loads(conf.read_text()).get('identifier')
        for app, ident in ids.items():
            self.assertTrue(ident and ident.startswith('io.github.dragongamer9.hearth'), (app, ident))
            expected = 'io.github.dragongamer9.hearth' + ('' if app == 'hearth-rs' else '.' + app[len('hearth-'):])
            self.assertEqual(ident, expected, '%s changed its identifier: its saved settings would be lost' % app)


if __name__ == '__main__':
    unittest.main()
