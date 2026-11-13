"""Hearth Files 1.11: the Windows 11 left menu (the owner's approved option A) and big folders opening fast."""
import os
import re
import unittest

FM = open(os.path.join(os.path.dirname(__file__), '..', 'desktop', 'shell', 'FileManager.qml')).read()


class Nav(unittest.TestCase):
    def test_order_and_names(self):
        nav = FM[FM.index('    function buildNav(list) {'):FM.index('    readonly property var linuxNavFolders')]
        order = [nav.index(x) for x in ('label: "Home"', 'label: "Gallery"', 'label: "Home server"', 'section: "folders"',
                                        'label: "This PC"', 'label: "Network"', 'label: "Linux"')]
        self.assertEqual(order, sorted(order))
        self.assertIn('if (cLinux) {', nav)                                   # Linux only with Show the Linux folders
        cf = FM[FM.index('    readonly property var cNavFolders'):]
        cf = cf[:cf.index(']')]
        self.assertEqual(re.findall(r'label: "([^"]+)"', cf), ['Users', 'Program Files', 'Windows apps'])

    def test_narrow_and_resizable(self):
        self.assertIn(': 158', FM[FM.index('property real navW'):][:200])
        self.assertIn('app.navW = Math.max(120, Math.min(360,', FM)
        self.assertIn('onReleased: app.saveNavWidth()', FM)
        self.assertIn('height: 30', FM[FM.index('id: place\n'):][:1200])


class BigFolders(unittest.TestCase):
    def test_sort_keys_once(self):
        ent = FM[FM.index('    readonly property var entries: {'):FM.index('    readonly property var current:')]
        self.assertIn('naturalKey(e.name)', ent)
        self.assertNotIn('natural(a.name, b.name)', ent)
        self.assertNotIn('groupRank(a, g)', ent)

    def test_hidden_view_builds_no_rows(self):
        self.assertEqual(FM.count('model: visible ? app.entries : []'), 2)



class CEverywhere(unittest.TestCase):
    """C: everywhere, step 1 (the approved mockups cdrive-1 … 4)."""

    def test_open_save_windows_show_c_but_return_real_paths(self):
        self.assertIn('readonly property bool cdriveOn: prefs.cdrive !== false\n', FM)
        pt = FM[FM.index('    function pickTargets() {'):FM.index('    readonly property string pickAcceptLabel')]
        self.assertEqual(pt.count('!DrivePaths.isCPath('), 3)
        ps = FM[FM.index('    function pickSave() {'):FM.index('    // ================================================ talking to hearth-files')]
        self.assertIn('DrivePaths.fromWindows(n, pathCtx)', ps)
        self.assertIn('DrivePaths.isCPath(path)) return say(', ps)

    def test_properties_and_terminal_switch(self):
        base = os.path.join(os.path.dirname(__file__), '..', 'desktop', 'shell')
        fp = open(os.path.join(base, 'FileProperties.qml')).read()
        self.assertIn('value: !pr.info ? "" : pr.single ? pr.winPath(pr.info.parent)', fp)
        self.assertIn('label: "Linux path"', fp)
        rc = open(os.path.join(base, '..', '..', '..', '..', 'os/system_files/usr/share/hearth/shell/windows-commands.bash')).read()
        self.assertIn('"terminalWindows": *false', rc)

if __name__ == '__main__':
    unittest.main()
