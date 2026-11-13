"""Resource Monitor built completely to its approved mockups resmon-1 … resmon-6 (1.12, red flag 11;
docs/mockups/checklists/resource-monitor.md)."""
import importlib.machinery
import importlib.util
import os
import pathlib
import sys
import tempfile
import unittest

SHELL = pathlib.Path(__file__).resolve().parents[1] / 'desktop' / 'shell'
sys.path.insert(0, str(SHELL))


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


rm = load('hearth_resmon_mock', SHELL / 'hearth-resmon')
QML = (SHELL / 'ResourceMonitorView.qml').read_text()


class Helpers(unittest.TestCase):
    def test_graphics_api(self):
        self.assertEqual(rm.graphics_api('/usr/lib/libvulkan.so.1\n/x/dxvk/d3d11.dll\n'), 'DXVK · Vulkan')
        self.assertEqual(rm.graphics_api('/usr/lib/libEGL.so.1\n/usr/lib/libGLX.so\n'), 'OpenGL')
        self.assertEqual(rm.graphics_api('/usr/lib/libva.so.2\n'), 'VA-API (video)')
        self.assertEqual(rm.graphics_api(''), '')

    def test_major_faults(self):
        line = '42 (fire fox) S 1 1 1 0 -1 0 10 0 77 0 5 6'
        self.assertEqual(rm.majflt_of(line), 77)

    def test_private_memory(self):
        x = rm.parse_status_extra('RssAnon:\t100 kB\nRssFile:\t50 kB\nRssShmem:\t10 kB\nVmSwap:\t4 kB\n')
        self.assertEqual((x['private'], x['shared'], x['swap']), (102400, 61440, 4096))

    def test_libraries_and_services(self):
        with tempfile.TemporaryDirectory() as d:
            os.makedirs(os.path.join(d, '7'))
            with open(os.path.join(d, '7', 'maps'), 'w') as f:
                f.write('7f00-7f10 r-xp 0 08:01 1 /usr/lib/libz.so.1\n7f10-7f20 r-xp 0 08:01 2 /usr/lib/libc.so.6\n7f20-7f30 r--p 0 08:01 2 /usr/lib/libc.so.6\n')
            self.assertEqual([x['name'] for x in rm.libraries_of(7, d)], ['libc.so.6', 'libz.so.1'])
            with open(os.path.join(d, '7', 'stat'), 'w') as f:
                f.write('7 (app) S 1 1 1')
            with open(os.path.join(d, '7', 'cgroup'), 'w') as f:
                f.write('0::/user.slice/user-1000.slice/user@1000.service/app.slice/hearth-ai-updater.service\n')
            self.assertEqual(rm.services_of([7], d)[0]['name'], 'hearth-ai-updater')

    def test_plain_words(self):
        self.assertEqual(rm.what_it_is('sshd'), 'remote terminal')
        self.assertEqual(rm.what_it_is('hearth-link'), 'your other Hearth devices')


class Page(unittest.TestCase):
    def test_every_tab_follows_its_mockup(self):
        for s in ('"Pause ⏸"', '"Open Task Manager"', 'Live · every second', 'Filtered to <b>', '"Services"', '"Libraries (modules)"',
                  '"Physical memory"', '["Kept for hardware"', '["Zram holds"', '"Programs using the disk"', '"Files being read and written"',
                  '"Drives"', '["Health", 1, true]', '"Programs using the network"', '"Connections (TCP)"', '"Listening ports"',
                  '["What it is", 4]', '"Programs using the graphics card"', '"Graphics card"', '["Vulkan · OpenGL"', '["Games running"',
                  '"Swapping in / out"', '"Memory pressure"', '"Core " + i', '"On drive", "Waits / s"', '"Video encode", "Video memory", "Through"'):
            self.assertIn(s, QML)


if __name__ == '__main__':
    unittest.main()
