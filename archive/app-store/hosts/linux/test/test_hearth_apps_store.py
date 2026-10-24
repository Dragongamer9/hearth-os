"""ARCHIVED (docs/ARCHIVED.md). Hearth's app store (Apps, hearth-apps) lists apps: the image check's offline store check
(.github/scripts/hearth-store-check.sh) against the source, and the first page still opens when ~/.cache can't be
written (root-owned after moving editions; it used to fail with a permission error)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import os
import pathlib
import shutil
import subprocess
import unittest

REPO = pathlib.Path(__file__).resolve().parents[3]
HA = REPO / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth-apps'
CHECK = REPO / '.github' / 'scripts' / 'hearth-store-check.sh'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_apps_store', str(HA))
    spec = importlib.util.spec_from_loader('hearth_apps_store', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class StoreCheck(unittest.TestCase):
    def test_offline_store_check_passes(self):
        r = subprocess.run(['sh', str(CHECK), str(HA)], capture_output=True, text=True, timeout=120)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn('ok: the first page (popular apps) lists Firefox', r.stdout)
        self.assertIn('ok: search finds Firefox', r.stdout)

    @unittest.skip('archived: the image checks no longer run the store check (docs/ARCHIVED.md)')
    def test_image_checks_run_it(self):
        for wf in ('base-peppermint.yml', 'base-anatase.yml'):
            text = (REPO / '.github' / 'workflows' / wf).read_text()
            self.assertIn('hearth-store-check.sh:/store-check.sh:ro', text, wf)
            self.assertIn('sh /store-check.sh', text, wf)


class FeaturedWithoutCache(unittest.TestCase):
    def test_no_writable_cache_gives_the_fallback_list(self):
        ha = load()
        ha.CACHE = '/dev/null/nimbus/apps'
        old = os.environ.get('XDG_RUNTIME_DIR')
        os.environ['XDG_RUNTIME_DIR'] = '/dev/null'
        try:
            ha.urllib.request.urlopen = lambda *a, **k: (_ for _ in ()).throw(OSError('offline'))
            self.assertEqual(ha.featured_ids(), ha.FEATURED_FALLBACK)
        finally:
            if old is None:
                os.environ.pop('XDG_RUNTIME_DIR', None)
            else:
                os.environ['XDG_RUNTIME_DIR'] = old


if __name__ == '__main__':
    unittest.main()
