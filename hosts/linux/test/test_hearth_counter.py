"""hearth-counter: the player count the Hearth app sent, now from a systemd user timer (docs/SETTINGS-MOVE.md). The same
address, the same three fields with the same values as ui/index.html's counterPing and hearth-core's counter_device_id /
edition, as often (20 s after start, then every 10 minutes), and nothing at all when "Count me in" is off."""
import sys
sys.dont_write_bytecode = True
import hashlib
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
PATH = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth-counter'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_counter', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_counter', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Counter(unittest.TestCase):
    def setUp(self):
        self.c = load()
        self.tmp = tempfile.mkdtemp(prefix='hearth-counter-')
        self.root = os.path.join(self.tmp, 'root')
        self.home = os.path.join(self.tmp, 'home')
        os.makedirs(os.path.join(self.root, 'etc'))
        os.makedirs(os.path.join(self.root, 'usr', 'share', 'nimbus'))
        self.env = dict(os.environ)
        os.environ['HOME'] = self.home
        os.environ.pop('XDG_DATA_HOME', None)
        self.put('/etc/machine-id', '0123456789abcdef0123456789abcdef\n')
        self.put('/usr/share/nimbus/version', '1.0.0\n')
        self.put('/usr/share/nimbus/image-info.json', '{"image-name": "hearth-peppermint", "base": "peppermint"}')

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.env)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def put(self, path, text):
        p = self.root + path
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, 'w') as f:
            f.write(text)

    def settings(self, obj):
        d = os.path.join(self.home, '.local', 'share', 'nimbus')
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, 'desktop.json'), 'w') as f:
            json.dump(obj, f)

    def test_the_same_three_fields_as_the_app(self):
        sent = []
        body = self.c.ping(self.root, send=lambda url, data: sent.append((url, json.loads(data))))
        want = {'id': hashlib.sha1(b'hearth-counter:0123456789abcdef0123456789abcdef').hexdigest()[:32], 'v': '1.0.0', 'k': 'peppermint'}
        self.assertEqual(body, want)
        self.assertEqual(sent, [('https://hearth-counter.game305-cl.workers.dev/v1/ping', want)])

    def test_the_address_and_id_match_the_app_and_core(self):
        ui = (ROOT / 'ui' / 'index.html').read_text()
        self.assertIn("url:'https://hearth-counter.game305-cl.workers.dev'", ui)
        self.assertIn("COUNTER.url+'/v1/ping'", ui)
        self.assertIn('body:JSON.stringify({id:counterId(), v:HOSTINFO.version||\'\', k:counterKind()})', ui)
        core = (ROOT / 'hosts' / 'hearth-core' / 'src' / 'bridge.rs').read_text()
        self.assertIn('format!("hearth-counter:{mid}")', core)
        self.assertIn('h[..32].to_string()', core)

    def test_off_sends_nothing_and_it_starts_on(self):
        sent = []
        self.assertIsNotNone(self.c.ping(self.root, send=lambda *a: sent.append(a)))   # no settings file: on
        self.settings({'look': 'steam'})
        self.assertIsNotNone(self.c.ping(self.root, send=lambda *a: sent.append(a)))   # not set: on
        self.settings({'countMe': False})
        self.assertIsNone(self.c.ping(self.root, send=lambda *a: sent.append(a)))
        self.assertEqual(len(sent), 2)
        self.settings({'countMe': True})
        self.assertIsNotNone(self.c.ping(self.root, send=lambda *a: sent.append(a)))

    def test_editions(self):
        self.assertEqual(self.c.edition(self.root), 'peppermint')
        self.put('/usr/share/nimbus/image-info.json', '{"base": "anatase"}')
        self.assertEqual(self.c.edition(self.root), 'anatase')
        self.put('/usr/share/nimbus/image-info.json', '{"base": "something"}')
        self.assertEqual(self.c.edition(self.root), 'other')
        os.remove(self.root + '/usr/share/nimbus/image-info.json')
        self.assertEqual(self.c.edition(self.root), 'other')
        self.put('/usr/share/ublue-os/image-info.json', '{"image-name": "hearth-nvidia"}')
        self.assertEqual(self.c.edition(self.root), 'nvidia')
        self.put('/etc/hearth-live', '')
        self.assertEqual(self.c.edition(self.root), 'live')

    def test_live_usb_keeps_a_random_id(self):
        self.put('/etc/hearth-live', '')
        a = self.c.device_id(self.root)
        self.assertRegex(a, r'^[0-9a-f]{32}$')
        self.assertNotEqual(a, hashlib.sha1(b'hearth-counter:0123456789abcdef0123456789abcdef').hexdigest()[:32])
        self.assertEqual(self.c.device_id(self.root), a)

    def test_the_timer_runs_as_often_as_the_app_did(self):
        b = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        timer = re.search(r"cat > /usr/lib/systemd/user/hearth-counter\.timer <<'UNIT'\n(.*?)\nUNIT", b, re.S).group(1)
        self.assertIn('OnActiveSec=20s', timer)            # the app: 20 s after it started
        self.assertIn('OnUnitActiveSec=10min', timer)      # then every 10 minutes
        self.assertIn('ln -sf ../hearth-counter.timer /usr/lib/systemd/user/timers.target.wants/hearth-counter.timer', b)
        service = re.search(r"cat > /usr/lib/systemd/user/hearth-counter\.service <<'UNIT'\n(.*?)\nUNIT", b, re.S).group(1)
        self.assertIn('ExecStart=/usr/bin/python3 /usr/share/nimbus/desktop/shell/hearth-counter ping', service)
        self.assertIn('Type=oneshot', service)

    def test_the_setting_row(self):
        s = (ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'Settings.qml').read_text()
        self.assertIn('property bool countMe: true', s)
        self.assertIn('"countMe"]', s)
        a = (ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'SetAbout.qml').read_text()
        self.assertIn('title: "Count me in the player count"', a)
        self.assertIn('checked: Settings.countMe', a)


if __name__ == '__main__':
    unittest.main()
