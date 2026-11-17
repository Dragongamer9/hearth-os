"""Settings › Passwords' helper (hosts/linux/desktop/shell/hearth-secrets, like KDE Wallet Manager) on the Secret
Service with a stand-in busctl: the provider and keyrings (locked, protected or not), saved items with their app,
account and site but never a secret (GetSecret(s) is never called, nothing secret printed), removing only real item
paths, lock / unlock / protect through the store's own prompt. Also: the image keeps GNOME Keyring and its PAM module,
the session starts it, and the login screen's PAM unlocks it."""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import textwrap
import unittest

sys.dont_write_bytecode = True
ROOT = pathlib.Path(__file__).resolve().parents[3]
HELPER = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth-secrets'

FAKE = textwrap.dedent(r'''
    #!/usr/bin/python3
    import json, os, sys
    st = json.load(open(os.environ['FAKE_STATE']))
    a = [x for x in sys.argv[1:] if x not in ('--user', '--json=short')]
    open(os.environ['FAKE_LOG'], 'a').write(' '.join(a) + '\n')
    def say(t, d): print(json.dumps({'type': t, 'data': d})); sys.exit(0)
    if a[0] == 'call' and a[4] == 'GetConnectionUnixProcessID':
        if not st.get('running'): sys.stderr.write('Name has no owner\n'); sys.exit(1)
        say('u', [os.getpid()])
    if a[0] == 'get-property':
        path, prop = a[2], a[4]
        v = st['props'].get(path + ' ' + prop)
        if v is None: sys.stderr.write('No such object\n'); sys.exit(1)
        say('x', v)
    if a[0] == 'call':
        path, meth = a[2], a[4]
        if meth == 'ReadAlias': say('o', [st.get('default', '/')])
        if meth == 'Delete': say('o', [st.get('deletePrompt', '/')])
        if meth in ('Lock', 'Unlock'): say('(aoo)', [a[7:], st.get('lockPrompt', '/')])
        if meth == 'Prompt': say('', [])
        if meth == 'ChangeWithPrompt': say('o', ['/org/freedesktop/secrets/prompt/p9'])
    sys.stderr.write('not faked: %s\n' % a); sys.exit(1)
''').lstrip()

C = '/org/freedesktop/secrets/collection/'
STATE = {
    'running': True, 'default': C + 'login',
    'props': {
        '/org/freedesktop/secrets Collections': [C + 'login', C + 'hearth', C + 'session'],
        C + 'login Label': 'Login', C + 'login Locked': False, C + 'login Items': [C + 'login/1', C + 'login/2'],
        C + 'hearth Label': 'Hearth', C + 'hearth Locked': True, C + 'hearth Items': [C + 'hearth/1'],
        C + 'session Label': '', C + 'session Locked': False, C + 'session Items': [],
        C + 'login/1 Label': 'Chromium Safe Storage', C + 'login/1 Attributes': {'application': 'chromium', 'xdg:schema': 'chrome_libsecret_os_crypt_password_v2'},
        C + 'login/1 Created': 1790000000, C + 'login/1 Modified': 1790000100,
        C + 'login/2 Label': 'smb://nas/share', C + 'login/2 Attributes': {'xdg:schema': 'org.gnome.keyring.NetworkPassword', 'user': 'sam', 'server': 'nas'},
        C + 'login/2 Created': 1791000000, C + 'login/2 Modified': 1791000000,
    }}


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_secrets_mod', str(HELPER))
    spec = importlib.util.spec_from_loader('hearth_secrets_mod', loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


class Secrets(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        t = pathlib.Path(self.tmp.name)
        (t / 'busctl').write_text(FAKE)
        (t / 'busctl').chmod(0o755)
        self.state, self.log = t / 'state.json', t / 'log'
        self.state.write_text(json.dumps(STATE))
        kr = t / 'share' / 'keyrings'
        kr.mkdir(parents=True)
        (kr / 'login.keyring').write_bytes(b'GnomeKeyring\n\r\x00\n\x00\x00encrypted')
        (kr / 'hearth.keyring').write_text('[keyring]\ndisplay-name=Hearth\n')
        self.env = dict(os.environ, HEARTH_BUSCTL=str(t / 'busctl'), FAKE_STATE=str(self.state), FAKE_LOG=str(self.log),
                        XDG_DATA_HOME=str(t / 'share'))

    def tearDown(self):
        self.tmp.cleanup()

    def run_h(self, *args):
        r = subprocess.run([sys.executable, str(HELPER)] + list(args), capture_output=True, text=True, env=self.env, timeout=30)
        line = r.stdout.strip().splitlines()[-1]
        return r.returncode, (line if line.startswith('error:') else json.loads(line))

    def test_status(self):
        code, st = self.run_h('status')
        self.assertEqual(code, 0)
        self.assertTrue(st['running'])
        ks = {k['label']: k for k in st['keyrings']}
        self.assertEqual((ks['Login']['locked'], ks['Login']['protected'], ks['Login']['count'], ks['Login']['default']), (False, True, 2, True))
        self.assertEqual((ks['Hearth']['locked'], ks['Hearth']['protected']), (True, False))

    def test_not_running(self):
        s = dict(STATE, running=False)
        self.state.write_text(json.dumps(s))
        code, st = self.run_h('status')
        self.assertEqual((code, st['running'], st['keyrings']), (0, False, []))

    def test_list_never_reads_secrets(self):
        code, res = self.run_h('list')
        self.assertEqual(code, 0)
        self.assertEqual([(i['app'], i['account'], i['where']) for i in res['items']],
                         [('chromium', '', ''), ('Network share', 'sam', 'nas')])
        self.assertEqual(res['lockedKeyrings'], ['Hearth'])            # locked: its items aren't listed (or unlocked)
        self.assertNotIn('GetSecret', self.log.read_text())
        self.assertNotIn('GetSecret', HELPER.read_text().replace('GetSecret(s) is never', ''))
        self.assertNotIn('OpenSession', HELPER.read_text())

    def test_remove_only_items(self):
        self.assertEqual(self.run_h('remove', C + 'login/2')[1]['message'], 'Removed.')
        self.assertIn('call org.freedesktop.secrets ' + C + 'login/2 org.freedesktop.Secret.Item Delete', self.log.read_text())
        for bad in (C + 'login', '/etc/passwd', C + 'login/2; rm -rf ~', '--help'):
            code, msg = self.run_h('remove', bad)
            self.assertEqual(code, 1, bad)
            self.assertTrue(msg.startswith('error:'))

    def test_remove_with_prompt(self):
        self.state.write_text(json.dumps(dict(STATE, deletePrompt='/org/freedesktop/secrets/prompt/u1')))
        code, r = self.run_h('remove', C + 'login/1')
        self.assertTrue(r['prompted'])
        self.assertIn('/org/freedesktop/secrets/prompt/u1 org.freedesktop.Secret.Prompt Prompt s ', self.log.read_text())

    def test_lock_unlock(self):
        self.assertEqual(self.run_h('lock')[1]['message'], 'Locked.')
        self.assertIn('Lock ao 3 ' + C + 'login ' + C + 'hearth ' + C + 'session', self.log.read_text())
        self.state.write_text(json.dumps(dict(STATE, lockPrompt='/org/freedesktop/secrets/prompt/u2')))
        self.assertTrue(self.run_h('unlock', C + 'hearth')[1]['prompted'])

    def test_pam_lines(self):
        mod = load()
        with tempfile.NamedTemporaryFile('w', delete=False) as f:
            f.write('auth include common-auth\n-auth optional pam_gnome_keyring.so\n-session optional pam_gnome_keyring.so auto_start\n')
        self.assertEqual(mod.pam_lines(f.name), {'auth': True, 'session': True})
        os.unlink(f.name)
        self.assertEqual(mod.pam_lines('/nonexistent'), {'auth': False, 'session': False})


class Image(unittest.TestCase):
    def test_image_has_the_store_and_pam(self):
        b = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        self.assertIn('gnome-keyring libpam-gnome-keyring', b)
        self.assertIn('HEARTH-PAM-KEYRING', b)
        sess = (ROOT / 'hosts' / 'linux' / 'session' / 'nimbus-session').read_text()
        self.assertIn('gnome-keyring-daemon --start --components=secrets', sess)


if __name__ == '__main__':
    unittest.main()
