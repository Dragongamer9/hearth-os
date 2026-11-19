"""Files' left menu shows each favourite folder once (the owner, Nov 19, on the Ally: "it keeps having double of the
favorited folders"): a GTK bookmark of Documents (Fedora's xdg-user-dirs-gtk adds those), the same bookmark twice, with
a trailing slash, as a link, are all one folder. The bookmarks file itself is never changed by listing it."""
import importlib.machinery
import importlib.util
import os
import pathlib
import tempfile
import unittest
from unittest import mock

SHELL = os.path.join(os.path.dirname(__file__), '..', 'desktop', 'shell')
HELPER = os.path.join(SHELL, 'hearth-files')


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_files_dedup', HELPER)
    spec = importlib.util.spec_from_loader('hearth_files_dedup', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


hf = load()


class Dedup(unittest.TestCase):
    def test_bookmarks_of_the_same_folder_show_once(self):
        with tempfile.TemporaryDirectory() as h:
            for d in ('Documents', 'Music', 'Projects'):
                os.mkdir(os.path.join(h, d))
            os.symlink(os.path.join(h, 'Projects'), os.path.join(h, 'proj-link'))
            cfg = os.path.join(h, '.config')
            os.makedirs(os.path.join(cfg, 'gtk-3.0'))
            bm = os.path.join(cfg, 'gtk-3.0', 'bookmarks')
            text = ''.join('file://%s\n' % p for p in (
                os.path.join(h, 'Documents'), os.path.join(h, 'Music') + '/', os.path.join(h, 'Projects'),
                os.path.join(h, 'Projects'), os.path.join(h, 'proj-link'))) + 'file://%s Docs\n' % os.path.join(h, 'Documents')
            pathlib.Path(bm).write_text(text)
            env = {'HOME': h, 'XDG_CONFIG_HOME': cfg, 'XDG_DATA_HOME': os.path.join(h, '.local/share')}
            empty = lambda *a, **k: []
            with mock.patch.dict(os.environ, env), \
                    mock.patch.multiple(hf, steam_libraries=empty, pc_drives=empty, account_places=empty, briefcase_places=empty,
                                        storage_places=empty, device_places=empty, system_drive=lambda: {}, free_space=lambda p: (0, 0)):
                out = hf.places()['places']
            paths = [os.path.realpath(q['path']) for q in out if q.get('section') in ('places', 'bookmarks')]
            self.assertEqual(len(paths), len(set(paths)), paths)
            self.assertIn(os.path.realpath(os.path.join(h, 'Projects')), paths)       # a real bookmark stays
            self.assertEqual(pathlib.Path(bm).read_text(), text)                     # the file is left as it was

    def test_place_key(self):
        with tempfile.TemporaryDirectory() as h:
            d = os.path.join(h, 'My Docs')
            os.mkdir(d)
            k = hf.place_key(d)
            self.assertEqual(hf.place_key(d + '/'), k)
            self.assertEqual(hf.place_key('file://' + d.replace(' ', '%20')), k)

    def test_rust_files_dedups_too(self):
        src = (pathlib.Path(__file__).resolve().parents[3] / 'hosts/hearth-ui/apps/files/src/app.rs').read_text()
        self.assertIn('seen.insert(place_key(', src)
        self.assertIn('pub fn place_key(', src)


if __name__ == '__main__':
    unittest.main()
