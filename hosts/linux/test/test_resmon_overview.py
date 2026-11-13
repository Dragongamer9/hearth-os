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
        ov = Q[Q.index('// Overview (1.11'):Q.index('// Processor, Disk, Network, Graphics: a summary on top')]
        self.assertIn('Flickable {', ov)
        self.assertIn('contentHeight: ovCol.implicitHeight', ov)
        self.assertIn('r.filter(x => v.ticked[x.g.id])', ov)

    def test_chips(self):
        for k in ('"cpu"', '"memory"', '"disk"', '"network"', '"gpu"'):
            self.assertIn('if (k === %s)' % k, Q[Q.index('function overviewChips(k)'):])


if __name__ == '__main__':
    unittest.main()
