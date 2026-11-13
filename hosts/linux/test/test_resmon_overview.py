"""Resource Monitor's Overview (1.11, the approved mockup resmon-1): five sections that fold, chips, ticks that filter,
and a list that scrolls instead of being cut off."""
import os
import unittest

Q = open(os.path.join(os.path.dirname(__file__), '..', 'desktop', 'shell', 'ResourceMonitorView.qml')).read()


class Overview(unittest.TestCase):
    def test_sections(self):
        self.assertIn('[["cpu", "Processor"], ["memory", "Memory"], ["disk", "Disk"], ["network", "Network"], ["gpu", "Graphics"]]', Q)
        self.assertIn('property var folded: ({ disk: true, network: true, gpu: true })', Q)

    def test_scrolls_and_filters(self):
        ov = Q[Q.index('// Overview (1.11'):Q.index('// Processor, Memory, Disk, Network, Graphics (1.12')]
        self.assertIn('Flickable {', ov)
        self.assertIn('contentHeight: ovCol.implicitHeight', ov)
        self.assertIn('r.filter(x => v.ticked[x.g.id])', ov)

    def test_chips(self):
        for k in ('"cpu"', '"memory"', '"disk"', '"network"', '"gpu"'):
            self.assertIn('if (k === %s)' % k, Q[Q.index('function overviewChips(k)'):])



class NetworkPorts(unittest.TestCase):
    def test_addresses_and_who_can_reach(self):
        import importlib.machinery, importlib.util, sys
        shell = os.path.join(os.path.dirname(__file__), '..', 'desktop', 'shell')
        sys.path.insert(0, shell)
        loader = importlib.machinery.SourceFileLoader('hrm_ports', os.path.join(shell, 'hearth-resmon'))
        spec = importlib.util.spec_from_loader('hrm_ports', loader)
        m = importlib.util.module_from_spec(spec)
        loader.exec_module(m)
        self.assertEqual(m._hexaddr('0100007F:1F90'), ('127.0.0.1', 8080))
        self.assertEqual(m._hexaddr('00000000000000000000000000000000:0016'), ('::', 22))
        self.assertEqual(m.open_to('127.0.0.1'), 'This PC only')
        self.assertEqual(m.open_to('100.72.99.97'), 'Tailscale only')
        self.assertTrue(m.open_to('0.0.0.0').startswith('Any network'))
        self.assertIn('listening', m.ports())

if __name__ == '__main__':
    unittest.main()
