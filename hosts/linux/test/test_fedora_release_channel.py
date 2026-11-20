"""The Fedora edition's release channel (1.16; docs/BASE-ANATASE.md, "Release channel"): releases go to
ghcr.io/dragongamer9/hearth-os:stable, and a device on a test tag whose version is out as a release (1.15's
test-1.15) moves to :stable with its next update, as an update (bootc switch: the version it starts from stays the
one to go back to). Debian and the old pre-archive tags are never touched; nothing happens when the registry can't be
asked. Also: every place that names the Fedora edition's image (Switch edition, the installer, Recovery, Hearth
Disks, the website) points at the release channel and the Fedora offline USB."""
import sys
sys.dont_write_bytecode = True
import hashlib
import importlib.machinery
import importlib.util
import json
import pathlib
import re
import unittest
from unittest import mock

REPO = pathlib.Path(__file__).resolve().parents[3]


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


ha = load(REPO / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-admin', 'hearth_admin_channel')
STABLE = 'ghcr.io/dragongamer9/hearth-os:stable'
TEST15 = 'ghcr.io/dragongamer9/hearth-os:test-1.15'
D_REL = 'sha256:' + 'a' * 64
D_OLD = 'sha256:' + 'b' * 64


def registry(version='1.16.0', index=False, missing=False):
    """A made-up registry for stable_release(): (path, accept) -> (headers, body)."""
    cfg = json.dumps({'config': {'Labels': {'org.opencontainers.image.version': version}}}).encode()
    man = json.dumps({'mediaType': 'application/vnd.oci.image.manifest.v1+json',
                      'config': {'digest': 'sha256:' + hashlib.sha256(cfg).hexdigest()}, 'layers': []}).encode()
    man_d = 'sha256:' + hashlib.sha256(man).hexdigest()
    idx = json.dumps({'manifests': [{'digest': 'sha256:' + 'c' * 64, 'platform': {'os': 'linux', 'architecture': 'arm64'}},
                                    {'digest': man_d, 'platform': {'os': 'linux', 'architecture': 'amd64'}}]}).encode()
    calls = []

    def get(path, accept=None):
        calls.append(path)
        if missing:
            raise OSError('HTTP Error 404: Not Found')
        if path.endswith('/manifests/stable'):
            return ({'docker-content-digest': D_REL}, idx if index else man)
        if path.endswith('/manifests/' + man_d):
            return ({}, man)
        if '/blobs/' in path:
            return ({}, cfg)
        raise AssertionError(path)
    get.calls = calls
    return get


def booted(image, digest=D_OLD, version='1.15.0'):
    return {'booted': {'image': image, 'digest': digest, 'version': version}, 'staged': None}


class Channel(unittest.TestCase):
    def test_where_updates_come_from(self):
        self.assertEqual(ha.FEDORA_STABLE, STABLE)
        self.assertEqual(ha.channel_of(STABLE)['channel'], 'stable')
        self.assertEqual(ha.channel_of(TEST15), {'following': TEST15, 'channel': 'test', 'testOf': '1.15'})
        for other in ('ghcr.io/dragongamer9/hearth-peppermint:testing', 'ghcr.io/dragongamer9/hearth-os:latest',
                      'ghcr.io/dragongamer9/hearth-os:testing', 'ghcr.io/evil/hearth-os:test-1.15',
                      'ghcr.io/dragongamer9/hearth-os:test-1.15x', None, ''):
            self.assertEqual(ha.channel_of(other)['channel'], 'other', other)

    def test_the_release_is_read_from_the_registry(self):
        r = ha.stable_release(registry('1.16.0'))
        self.assertEqual(r['version'], '1.16.0')
        self.assertIn(D_REL, r['digests'])
        r = ha.stable_release(registry('1.16.0', index=True))   # an index: the PC's own (amd64) image in it
        self.assertEqual((r['version'], len(r['digests'])), ('1.16.0', 2))
        with mock.patch('builtins.print'):
            self.assertIsNone(ha.stable_release(registry(missing=True)), 'not published yet / offline: no move')
        self.assertIsNone(ha.stable_release(registry('not a version')))

    def test_a_test_of_a_released_version_moves_to_the_releases(self):
        ch = ha.release_channel(booted(TEST15), lambda: {'version': '1.16.0', 'digests': [D_REL]})
        self.assertEqual((ch['moveTo'], ch['release'], ch['same']), (STABLE, '1.16.0', False))
        ch = ha.release_channel(booted(TEST15), lambda: {'version': '1.15.0', 'digests': [D_REL]})
        self.assertEqual(ch['moveTo'], STABLE, 'its own version released: it moves too')

    def test_a_test_of_a_newer_version_stays(self):
        ch = ha.release_channel(booted('ghcr.io/dragongamer9/hearth-os:test-1.17'), lambda: {'version': '1.16.0', 'digests': [D_REL]})
        self.assertNotIn('moveTo', ch)
        self.assertEqual(ch['channel'], 'test')

    def test_the_release_that_came_through_the_test_tag_needs_no_download(self):
        # base-anatase.yml publishes each release to BRIDGE_TAGS too: the device got 1.16 through test-1.15.
        ch = ha.release_channel(booted(TEST15, D_REL, '1.16.0'), lambda: {'version': '1.16.0', 'digests': [D_REL]})
        self.assertTrue(ch['same'])
        # Its copy on the test tag compressed differently: the version says it's the release all the same.
        ch = ha.release_channel(booted(TEST15, D_OLD, '1.16.0'), lambda: {'version': '1.16.0', 'digests': [D_REL]})
        self.assertTrue(ch['same'])
        ch = ha.release_channel(booted(TEST15, D_OLD, '1.15.0'), lambda: {'version': '1.16.0', 'digests': [D_REL]})
        self.assertFalse(ch['same'])
        # A test build of 1.16 on test-1.16 isn't the 1.16 release: it moves (downloads the release).
        ch = ha.release_channel(booted('ghcr.io/dragongamer9/hearth-os:test-1.16', D_OLD, '1.16.0'), lambda: {'version': '1.16.0', 'digests': [D_REL]})
        self.assertEqual((ch['moveTo'], ch['same']), (STABLE, False))

    def test_never_asks_or_moves_anything_else(self):
        def ask():
            raise AssertionError('the registry is only asked for a device on a test tag')
        for ref in (STABLE, 'ghcr.io/dragongamer9/hearth-peppermint:testing', 'ghcr.io/dragongamer9/hearth-os:latest', None):
            self.assertNotIn('moveTo', ha.release_channel(booted(ref), ask))
        self.assertNotIn('moveTo', ha.release_channel({}, ask))
        with mock.patch('builtins.print'):
            self.assertNotIn('moveTo', ha.release_channel(booted(TEST15), lambda: None), 'offline: no move')

    def test_update_check_offers_the_release(self):
        with mock.patch('builtins.print'):
            st = ha.offer_release(booted(TEST15), lambda: {'version': '1.16.0', 'digests': [D_REL]})
        self.assertEqual(st['available'], {'version': '1.16.0', 'timestamp': None, 'channel': 'stable'})
        self.assertEqual(st['channel']['moveTo'], STABLE)
        st = ha.offer_release(booted(TEST15, D_REL, '1.16.0'), lambda: {'version': '1.16.0', 'digests': [D_REL]})
        self.assertNotIn('available', st)
        staged = {**booted(TEST15), 'staged': {'version': '1.15.0'}}
        self.assertNotIn('available', ha.offer_release(staged, lambda: {'version': '1.16.0', 'digests': [D_REL]}))
        st = ha.offer_release(booted('ghcr.io/dragongamer9/hearth-peppermint:testing'), lambda: 1 / 0)
        self.assertEqual(st['channel']['channel'], 'other')
        self.assertNotIn('available', st)

    def test_status_says_where_updates_come_from_without_the_network(self):
        out = json.dumps({'status': {'booted': {'image': {'image': {'image': TEST15}, 'version': '1.15.0', 'imageDigest': D_OLD}}}})
        with mock.patch.object(ha.subprocess, 'run', lambda *a, **k: mock.Mock(returncode=0, stdout=out, stderr='')), \
                mock.patch.object(ha, 'stable_release', lambda *a: 1 / 0):
            st = ha.bootc_status()
        self.assertEqual(st['channel'], {'following': TEST15, 'channel': 'test', 'testOf': '1.15'})
        self.assertEqual(st['booted']['digest'], D_OLD)


class TheUpdate(unittest.TestCase):
    def run_upgrade(self, st, rel, layered=False, switch_fails=False):
        calls = []

        def bootc(lo, hi, stage, argv=None):
            calls.append(argv or ['bootc', 'upgrade'])
            if switch_fails and argv and argv[1] == 'switch':
                raise ha.UpgradeFailed('network is unreachable')

        with mock.patch.object(ha, 'bootc_status', lambda quiet=False: st), mock.patch.object(ha, 'layered', lambda: layered), \
                mock.patch.object(ha, 'bootc_upgrade_with_progress', bootc), \
                mock.patch.object(ha, 'rpm_ostree_upgrade', lambda stage, argv=None: calls.append(argv or ['rpm-ostree', 'upgrade'])), \
                mock.patch.object(ha, 'staged_any', lambda: True), mock.patch.object(ha, 'rpm_ostree_has_update', lambda: False), \
                mock.patch.object(ha, 'with_retries', lambda fn, tries=3, wait=10: fn()), mock.patch('builtins.print'):
            ha.system_upgrade(ha.Stage(), lambda: rel)
        return calls

    def test_the_move_is_a_bootc_switch_to_stable(self):
        calls = self.run_upgrade(booted(TEST15), {'version': '1.16.0', 'digests': [D_REL]})
        self.assertEqual(calls, [['bootc', 'switch', STABLE]], 'one new version; the one running stays to go back to')

    def test_with_layered_packages_it_is_a_rebase(self):
        calls = self.run_upgrade(booted(TEST15), {'version': '1.16.0', 'digests': [D_REL]}, layered=True)
        self.assertEqual(calls, [['rpm-ostree', 'rebase', 'ostree-unverified-registry:' + STABLE]])

    def test_a_move_that_fails_updates_as_before(self):
        calls = self.run_upgrade(booted(TEST15), {'version': '1.16.0', 'digests': [D_REL]}, switch_fails=True)
        self.assertEqual(calls, [['bootc', 'switch', STABLE], ['bootc', 'upgrade']])

    def test_everything_else_updates_as_before(self):
        for st, rel in ((booted(STABLE), {'version': '1.17.0', 'digests': [D_REL]}),
                        (booted('ghcr.io/dragongamer9/hearth-peppermint:testing'), {'version': '1.16.0', 'digests': [D_REL]}),
                        (booted('ghcr.io/dragongamer9/hearth-os:test-1.17'), {'version': '1.16.0', 'digests': [D_REL]}),
                        (booted(TEST15, D_REL, '1.16.0'), {'version': '1.16.0', 'digests': [D_REL]}),
                        (booted(TEST15), None),
                        ({**booted(TEST15), 'staged': {'version': '1.16.0'}}, {'version': '1.16.0', 'digests': [D_REL]}),
                        (None, {'version': '1.16.0', 'digests': [D_REL]})):
            self.assertEqual(self.run_upgrade(st, rel), [['bootc', 'upgrade']], st)

    def test_no_bootc_no_problem(self):
        def broken(quiet=False):
            raise FileNotFoundError('bootc')
        calls = []
        with mock.patch.object(ha, 'bootc_status', broken), mock.patch.object(ha, 'layered', lambda: False), \
                mock.patch.object(ha, 'bootc_upgrade_with_progress', lambda lo, hi, stage, argv=None: calls.append(argv)), \
                mock.patch.object(ha, 'staged_any', lambda: True), mock.patch.object(ha, 'with_retries', lambda fn, tries=3, wait=10: fn()), \
                mock.patch('builtins.print'):
            ha.system_upgrade(ha.Stage(), lambda: 1 / 0)
        self.assertEqual(calls, [None])


class EveryPlacePointsAtTheRelease(unittest.TestCase):
    def test_switch_edition(self):
        self.assertEqual(ha.new_os_image('anatase'), STABLE)
        self.assertEqual(ha.new_os_image('peppermint'), 'ghcr.io/dragongamer9/hearth-peppermint:testing', 'Debian channels untouched')
        rs = (REPO / 'hosts' / 'hearth-core' / 'src' / 'newos.rs').read_text()
        self.assertIn('pub const FEDORA_TAG: &str = "stable";', rs)
        self.assertIn('pub const IMAGE: &str = "ghcr.io/dragongamer9/hearth-os:stable";', rs)
        self.assertNotIn('test-1.15"', rs.split('#[cfg(test)]')[0])

    def test_the_installer_and_recovery(self):
        inst = load(REPO / 'os' / 'live' / 'src' / 'hearth-install', 'hearth_install_channel')
        self.assertEqual((inst.DEFAULT_EDITION, inst.EDITIONS['anatase']['image']), ('anatase', STABLE))
        self.assertEqual(inst.EDITIONS['peppermint']['image'], 'ghcr.io/dragongamer9/hearth-peppermint:testing')
        self.assertIn('peppermint', inst.NOT_OFFERED)
        rec = (REPO / 'os' / 'recovery' / 'src' / 'hearth-recovery').read_text()
        self.assertIn("'anatase': {'image': '" + STABLE + "'", rec)

    def test_the_iso_workflow_builds_the_fedora_offline_usb(self):
        wf = (REPO / '.github' / 'workflows' / 'installer-iso.yml').read_text()
        self.assertRegex(wf, r"options: \[anatase, installer, ubuntu, cachyos, peppermint\]\n\s+default: anatase")
        self.assertIn('name="hearth-os-$ED.iso"', wf)     # hearth-os-anatase.iso on the download site
        self.assertIn("inputs.upload && inputs.carry_tag == '' && steps.e2e.outcome == 'success'", wf,
                      'a test tag is never uploaded, and nothing is uploaded without the end-to-end test')
        self.assertIn("""[ "$ED" = anatase ] && echo "$CARRY" | grep -Eqx 'test-[0-9]+(\\.[0-9]+)*'""", wf)

    def test_download_links(self):
        r2 = 'https://pub-f59beb6fa21c421ca4435ee7f42b3de4.r2.dev/hearth-os-anatase.iso'
        cat = json.loads((REPO / 'hosts' / 'hearth-disks' / 'usb' / 'catalog.json').read_text())
        ids = {e['id']: e for e in cat['entries']}
        self.assertNotIn('hearth-peppermint', ids)
        self.assertEqual(ids['hearth-fedora']['resolve']['url'], r2)
        site = (REPO / 'site' / 'index.html').read_text()
        self.assertIn('href="' + r2 + '"', site)
        self.assertIn('data-file="hearth-os-anatase.iso"', site)
        docs = (REPO / 'tools' / 'build-docs.py').read_text()
        self.assertIn('{R2}hearth-os-anatase.iso', docs)

    def test_no_page_offers_debian(self):
        pages = [REPO / 'site' / 'index.html', REPO / 'site' / 'whats-new' / 'index.html', REPO / 'site' / 'arm-devices' / 'index.html',
                 REPO / 'tools' / 'build-docs.py'] + sorted(p for p in (REPO / 'site' / 'docs').glob('*.html') if p.name != 'releases.html')
        # (releases.html is the history of every release, from docs/ROADMAP.md: what each one said at the time.)
        for p in pages:
            t = p.read_text()
            self.assertNotIn('hearth-os-peppermint.iso', t, p)
            self.assertNotIn('Debian-based', t, p)
            self.assertNotIn('switch to Debian', t, p)
            self.assertNotIn('install Hearth OS on Debian', t, p)
        for p in pages[:3]:
            self.assertIn('Fedora', p.read_text(), p)
            self.assertRegex(p.read_text(), r'Debian edition has been merged into it and archived', p)


if __name__ == '__main__':
    unittest.main()
