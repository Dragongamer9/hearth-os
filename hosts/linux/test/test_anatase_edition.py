"""1.15: Hearth OS on Anatase is back as a test beside Debian. Its "Unlock the file system" (the dnf form of hearth-unlock),
Desktop Mode as stock KDE Plasma (nimbus-session run_plasma, nimbus-mode), and the workflow's publish rules (only the
test tag, only from main)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import tempfile
import unittest
from unittest import mock

REPO = pathlib.Path(__file__).resolve().parents[3]
ANATASE = REPO / 'os-bases' / 'anatase'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def unlock_module(state):
    os.environ['HEARTH_UNLOCK_STATE'] = state
    os.environ['HEARTH_UNLOCK_UNLOCKEDFILE'] = os.path.join(state, 'system-unlocked')
    try:
        return load(ANATASE / 'system_files' / 'usr' / 'bin' / 'hearth-unlock', 'anatase_unlock')
    finally:
        for k in ('HEARTH_UNLOCK_STATE', 'HEARTH_UNLOCK_UNLOCKEDFILE'):
            os.environ.pop(k, None)


class Unlock(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.u = unlock_module(self.d)

    def test_locked_unless_turned_on(self):
        self.assertTrue(self.u.locked(), 'Anatase is locked by default')
        open(os.path.join(self.d, 'system-unlocked'), 'w').close()
        self.assertFalse(self.u.locked())

    def test_switch_writes_and_removes_its_file(self):
        with mock.patch.object(self.u, 'need_root', lambda: None), mock.patch.object(self.u, 'layer_id', lambda: 'a' * 64), \
                mock.patch.object(self.u, 'mount_layer', lambda lid: True), mock.patch.object(self.u, 'cmd_start_units', lambda: 0), \
                mock.patch.object(self.u, 'mounted', lambda: True):
            self.u.cmd_unlock()
            self.assertTrue(os.path.exists(os.path.join(self.d, 'system-unlocked')))
            self.assertFalse(self.u.locked())
            s = self.u.cmd_lock()
            self.assertFalse(os.path.exists(os.path.join(self.d, 'system-unlocked')))
            self.assertEqual(s['state'], 'locks-at-restart')
            self.assertEqual(s['tool'], 'dnf')

    def test_record(self):
        rec = {'manual': [], 'removed': [], 'rpms': {}, 'updated': ''}
        base = {'bash', 'dnf5', 'nano'}
        r = self.u.merge_record(rec, base, base | {'sl', 'figlet'}, base)
        self.assertEqual(r['manual'], ['figlet', 'sl'])
        r = self.u.merge_record(r, base | {'sl', 'figlet'}, (base - {'nano'}) | {'sl'}, base)
        self.assertEqual((r['manual'], r['removed']), (['sl'], ['nano']))
        r = self.u.merge_record(r, (base - {'nano'}) | {'sl'}, base | {'sl'}, base)
        self.assertEqual(r['removed'], [], 'an image package put back is forgotten as removed')
        self.assertEqual(self.u.parse_installed('bash\ngpg-pubkey\nsl\n\nnot a name\n'), {'bash', 'sl'})

    def test_reapply_plan(self):
        rec = {'manual': ['sl', 'cowsay', 'gone'], 'removed': ['nano'], 'rpms': {'cowsay': 'cowsay.rpm'}}
        rm, repo, files, missing = self.u.plan_reapply(rec, {'nano', 'bash'}, lambda p: p == 'sl', lambda f: True)
        self.assertEqual((rm, repo, missing), (['nano'], ['sl'], ['gone']))
        self.assertTrue(files[0].endswith('/rpms/cowsay.rpm'))

    def test_boot_chain(self):
        for p in ('kernel', 'kernel-core', 'grub2-efi-x64', 'shim-x64', 'dracut', 'ostree', 'rpm-ostree', 'bootc',
                  'bootupd', 'kmod-nvidia', 'akmods', 'composefs-libs'):
            self.assertTrue(self.u.BOOT_CHAIN_PKG.fullmatch(p), p)
        for p in ('sl', 'figlet', 'firefox', 'kmod', 'kernelshark-docs-not'):
            self.assertFalse(self.u.BOOT_CHAIN_PKG.fullmatch(p), p)
        table = ('Package  Arch  Version  Repository  Size\nInstalling:\n kernel-core    x86_64 6.17.1-300.fc45 updates 70.0 MiB\n'
                 ' sl             x86_64 5.05-1.fc45     fedora  20.0 KiB\nInstalling dependencies:\n figlet noarch 2.2.5 fedora 1 KiB\n')
        self.assertEqual(self.u.plan_names(table), {'kernel-core', 'sl', 'figlet'})
        self.assertTrue(self.u.removes_only(' sl x86_64 5.05 @System 20 KiB\n', 'sl'))
        self.assertFalse(self.u.removes_only(' sl x86_64 5.05 @System 20 KiB\n lib x86_64 1 @System 1 KiB\n', 'sl'))
        conf = (ANATASE / 'system_files' / 'usr' / 'share' / 'hearth' / 'unlock' / 'dnf-boot-chain.conf').read_text()
        excl = re.search(r'^excludepkgs=(.*)$', conf, re.M).group(1).split(',')
        for p in ('kernel', 'kernel-core', 'grub2*', 'shim*', 'ostree*', 'bootc*', 'bootupd*'):
            self.assertIn(p, excl)

    def test_status_says_what_survives(self):
        doc = (ANATASE / 'system_files' / 'usr' / 'bin' / 'hearth-unlock').read_text()
        for words in ('A restart: everything you changed in /usr, kept', 'Files you copied into /usr by hand are NOT carried'):
            self.assertIn(words, doc)


class Plasma(unittest.TestCase):
    SESSION = (REPO / 'hosts' / 'linux' / 'session' / 'nimbus-session').read_text(encoding='utf-8')
    MODE = (REPO / 'hosts' / 'linux' / 'session' / 'nimbus-mode').read_text()

    def test_session_starts_plasma_only_where_the_image_says(self):
        self.assertIn('plasma_image && [ -z "$HEARTH_SEAT" ]; then run_plasma; return $?; fi', self.SESSION)
        self.assertIn('/usr/share/nimbus/desktop-mode', self.SESSION)
        self.assertEqual((ANATASE / 'system_files' / 'usr' / 'share' / 'nimbus' / 'desktop-mode').read_text().strip(), 'plasma')
        self.assertFalse((REPO / 'os-bases' / 'peppermint' / 'system_files' / 'usr' / 'share' / 'nimbus' / 'desktop-mode').exists(),
                         'Debian keeps Mantel')

    def test_plasma_shows_hearths_apps_and_never_saves_a_choice(self):
        body = self.SESSION.split('run_plasma() {', 1)[1].split('\n}\n', 1)[0]
        self.assertIn('/usr/share/hearth/app-kinds', body, "Hearth's one-app-of-each-kind copies are left out for Plasma")
        self.assertIn('/usr/share/hearth/app-kinds-plasma', body, "Plasma's own copies, only when the build made them")
        self.assertIn('XDG_MENU_PREFIX', body)
        self.assertIn('XDG_CURRENT_DESKTOP=KDE', body)
        self.assertNotIn('session.conf', body, 'the saved Desktop Mode choice is never written here')
        self.assertIn('plasma-instead-told', body, 'told once')
        self.assertIn('echo steam >"$NEXT"', body, 'a Plasma that can\'t start never leaves no screen')

    def test_plasma_puts_hearths_own_apps_first(self):
        build = (ANATASE / 'build.sh').read_text()
        block = build.split("# >>> Hearth's apps in Plasma", 1)[1].split("# <<< Hearth's apps in Plasma", 1)[0]
        self.assertIn('cp -a "$PK/usr/share/hearth/app-kinds/applications" /usr/share/hearth/app-kinds-plasma/', block)
        self.assertIn('hearth-app-kinds check --root "$PK"', block, 'Plasma gets its copies only when their check passed')
        self.assertIn('echo \'{"ownApps": false}\' > "$PK/etc/hearth/desktops.json"', block, 'own apps off in the staging tree only')
        self.assertNotIn('> /etc/hearth/desktops.json', block, 'the real setting is never written')
        self.assertIn("== ['Nimbus']", block, 'Mantel-only apps are never put first')
        self.assertIn('Exec=/usr/bin/hearth-task-manager', block, 'the pure-Rust Task Manager takes its entry')
        self.assertLess(build.index('# <<< app kinds'), build.index("# >>> Hearth's apps in Plasma"))

    def test_back_to_game_mode(self):
        self.assertIn('org.kde.Shutdown.logout', self.MODE)
        self.assertIn('hearth-plasma.pid', self.MODE)
        js = (ANATASE / 'system_files' / 'usr' / 'share' / 'plasma' / 'shells' / 'org.kde.plasma.desktop' / 'contents' / 'updates' /
              'hearth-game-mode-launcher.js').read_text()
        self.assertIn('hearth-gamemode.desktop', js)
        entry = (REPO / 'os' / 'system_files' / 'usr' / 'share' / 'applications' / 'hearth-gamemode.desktop').read_text()
        self.assertIn('Exec=nimbus-mode game', entry)

    def test_mode_switch_in_a_fake_plasma(self):
        with tempfile.TemporaryDirectory() as run:
            p = subprocess.Popen(['sleep', '30'])
            try:
                open(os.path.join(run, 'hearth-plasma.pid'), 'w').write(str(p.pid))
                env = dict(os.environ, XDG_RUNTIME_DIR=run, PATH='/usr/bin:/bin')
                env.pop('HEARTH_DESKTOP_PID', None)
                env.pop('LABWC_PID', None)
                out = subprocess.run(['sh', str(REPO / 'hosts' / 'linux' / 'session' / 'nimbus-mode'), 'current'], env=env, capture_output=True, text=True)
                self.assertEqual(out.stdout.strip(), 'desktop')
                subprocess.run(['sh', str(REPO / 'hosts' / 'linux' / 'session' / 'nimbus-mode'), 'game'], env=env, capture_output=True, text=True, timeout=30)
                self.assertEqual(open(os.path.join(run, 'nimbus-next')).read().strip(), 'game')
                p.wait(timeout=20)   # no Plasma's D-Bus here: it's ended for it
            finally:
                if p.poll() is None:
                    p.kill()


class Workflow(unittest.TestCase):
    def test_publishing_the_release_and_test_tags(self):
        """1.16: a release on main publishes the release channel (:stable, :stable-<version>, the old test tags
        devices follow); by hand only test-<version> tags; never :latest, :testing or Debian's image."""
        wf = (REPO / '.github' / 'workflows' / 'base-anatase.yml').read_text()
        self.assertIn("PUBLISH: ${{ github.ref == 'refs/heads/main' && (github.event_name == 'push' || (github.event_name == 'workflow_dispatch' && inputs.publish)) }}", wf)
        self.assertIn("TAG_IN: ${{ github.event_name == 'workflow_dispatch' && inputs.test_tag || '' }}", wf)
        self.assertIn('STABLE_TAG: stable\n', wf)
        self.assertIn('BRIDGE_TAGS: test-1.15\n', wf, "1.15's devices (test-1.15) get the release through their tag")
        self.assertIn('IMAGE: ghcr.io/dragongamer9/hearth-os\n', wf)
        trig = wf.split('jobs:')[0]
        self.assertIn('push:\n    branches: [main]\n', trig, 'a release (a push to main) publishes the release channel')
        self.assertNotIn('pull_request', trig)
        step = wf.split('- name: Publish (the release, or the test tag)')[1].split('\n  # ')[0]
        self.assertIn('success()', step)
        self.assertIn('[ "$GITHUB_REF" = refs/heads/main ]', step)
        self.assertIn('tags="$STABLE_TAG $STABLE_TAG-$v $BRIDGE_TAGS"', step)
        self.assertIn("python3 tools/hearth_version.py show", step)
        self.assertIn("by hand only test-<version> tags", step)
        pushes = re.findall(r'podman push .*', step)
        self.assertEqual(len(pushes), 2, pushes)     # the first tag, then the same image to the others
        for p in pushes:
            self.assertRegex(p, r'^podman push (--[\w-]+(?: [\w./-]+)? )*--digestfile /tmp/digest hearth-os:anatase "\$IMAGE:\$tag"$')
        for never in (':latest', 'hearth-peppermint', ':testing'):
            self.assertNotIn(never, step)
        self.assertIn('case "$tag" in latest|testing|testing-*)', step)
        self.assertIn('not the image this run built', step)
        dispatch = wf.split('workflow_dispatch:')[1].split('env:')[0]
        self.assertRegex(dispatch, r"publish:[\s\S]*?default: false")
        self.assertRegex(dispatch, r"test_tag:[\s\S]*?default: ''")
        # The image knows which channel it's for, and the check reads it.
        self.assertIn('--build-arg HEARTH_CHANNEL="${{ steps.ver.outputs.channel }}"', wf)
        self.assertIn('-e HEARTH_CHANNEL="${{ steps.ver.outputs.channel }}"', wf)
        self.assertIn('if [ "$PUBLISH" != true ] && [ -n "$TAG_IN" ]; then ch="$TAG_IN"; else ch="$STABLE_TAG"; fi', wf)
        # reuse_run: only this workflow's runs, only when the app sources didn't change, every artifact there.
        self.assertRegex(dispatch, r"reuse_run:[\s\S]*?default: ''")
        self.assertIn('[ "$path" = .github/workflows/base-anatase.yml ]', wf)
        self.assertIn('git diff --name-only "$sha" HEAD -- hosts ui', wf)
        self.assertIn('is missing or expired', wf)
        for job in ('app', 'browser', 'hearth-steam', 'kde'):
            self.assertIn("  %s:\n    name:" % job, wf)
        self.assertEqual(wf.count("if: ${{ github.event_name != 'workflow_dispatch' || inputs.reuse_run == '' }}"), 4)
        # The guards themselves.
        tag_ok = r"'stable|stable-[0-9]+(\.[0-9]+)*|test-[0-9]+(\.[0-9]+)*'"
        self.assertIn("grep -Eqx " + tag_ok, step)
        for tag, ok in (('stable', True), ('stable-1.16', True), ('test-1.15', True), ('test-1', True), ('latest', False),
                        ('testing', False), ('test-1.15;x', False), ('test-', False), ('stable-', False), ('1.16', False)):
            r = subprocess.run(['bash', '-c', "echo \"$1\" | grep -Eqx " + tag_ok, '_', tag])
            self.assertEqual(r.returncode == 0, ok, tag)

    def test_the_image_says_its_channel(self):
        b = (ANATASE / 'build.sh').read_text()
        self.assertIn('"image-tag": "${HEARTH_CHANNEL}", "channel": "${CH_KIND}", "test": ${CH_TEST}', b)
        self.assertNotIn('"image-tag": "test-1.15"', b)
        c = (ANATASE / 'Containerfile').read_text()
        self.assertIn('ARG HEARTH_CHANNEL=stable', c)
        self.assertIn('HEARTH_CHANNEL="${HEARTH_CHANNEL}" /ctx/build.sh', c)
        # build.sh's own channel switch, as a shell snippet.
        snippet = b.split('HEARTH_CHANNEL=${HEARTH_CHANNEL:-stable}')[1].split('cat > /usr/share/nimbus/image-info.json')[0]
        for ch, want in (('stable', 'stable false'), ('test-1.16', 'test true')):
            out = subprocess.run(['bash', '-c', 'set -e; HEARTH_CHANNEL=$1' + snippet + 'echo "$CH_KIND $CH_TEST"', '_', ch], capture_output=True, text=True)
            self.assertEqual(out.stdout.strip(), want, ch)
        self.assertNotEqual(subprocess.run(['bash', '-c', 'HEARTH_CHANNEL=latest' + snippet + 'echo ok'], capture_output=True, text=True).returncode, 0)

    def test_build_script(self):
        b = (ANATASE / 'build.sh').read_text()
        self.assertEqual(subprocess.run(['bash', '-n', str(ANATASE / 'build.sh')]).returncode, 0)
        # 1.18 (the owner, Nov 23: "completely remove KDE"): KDE removed, greetd signs in
        for need in ('KDE_RE=', 'dnf5 -y remove $kde_pkgs', 'hearth-login-text.service', 'DECKY_VERSION=', 'decky-hearth-sync', 'GE_PROTON=',
                     'hearth-unlock.service', 'rpm-baseline.list', 'hearth-bls-grub.service', 'Back to Game Mode'):
            self.assertIn(need, b)
        # (1.16: labwc and Quickshell come back only as a choice, in their own marked block: test_labwc_fedora_session.py)
        lw = re.search(r'# >>> labwc on the Fedora edition.*?# <<< labwc on the Fedora edition\n', b, re.S)
        self.assertIsNotNone(lw)
        rest = b.replace(lw.group(0), '')
        for gone in ('dnf5 -y install labwc', 'quickshell', 'gnome-shell mutter', 'cosmic_pkgs'):
            self.assertNotIn(gone, rest.replace('Quickshell', ''))


if __name__ == '__main__':
    unittest.main()
