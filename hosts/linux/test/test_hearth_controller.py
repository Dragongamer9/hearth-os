"""Tests for desktop/shell/hearth-controller (InputPlumber profile building)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import pathlib
import tempfile
import unittest

PATH = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell' / 'hearth-controller'
loader = importlib.machinery.SourceFileLoader('hearth_controller', str(PATH))
spec = importlib.util.spec_from_loader('hearth_controller', loader)
hc = importlib.util.module_from_spec(spec)
loader.exec_module(hc)

BASE = [
    {'name': 'Keyboard', 'source_event': {'gamepad': {'button': 'Keyboard'}}, 'target_events': [{'gamepad': {'button': 'Guide'}}]},
    {'name': 'LeftTop', 'source_event': {'gamepad': {'button': 'LeftTop'}}, 'target_events': [{'gamepad': {'button': 'LeftPaddle1'}}]},
    {'name': 'Dial', 'source_event': {'gamepad': {'dial': {'name': 'LeftStickDial', 'direction': 'clockwise'}}}, 'target_events': [{'keyboard': 'KeyVolumeUp'}]},
]


class Profile(unittest.TestCase):
    def test_remaps_override_defaults_and_keep_the_rest(self):
        p = hc.build_profile({'map': {'South': 'East', 'Keyboard': 'none', 'LeftPaddle1': 'key:KeyLeftAlt+KeyTab'}, 'target': ''}, BASE)
        self.assertEqual(p['name'], 'Hearth')
        self.assertNotIn('target_devices', p)
        names = [m['name'] for m in p['mapping']]
        self.assertIn('LeftTop', names)
        self.assertIn('Dial', names)
        self.assertNotIn('Keyboard', names)                  # replaced by ours
        by = {m['name']: m for m in p['mapping']}
        self.assertEqual(by['Hearth: South']['target_events'], [{'gamepad': {'button': 'East'}}])
        self.assertEqual(by['Hearth: Keyboard']['target_events'], [])
        self.assertEqual(by['Hearth: LeftPaddle1']['target_events'], [{'keyboard': 'KeyLeftAlt'}, {'keyboard': 'KeyTab'}])
        json.dumps(p)

    def test_target_type(self):
        p = hc.build_profile({'map': {}, 'target': 'ds5'}, BASE)
        self.assertEqual(p['target_devices'], ['ds5', 'mouse', 'keyboard'])

    def test_unknown_names(self):
        p = hc.build_profile({'map': {'NotAButton': 'East'}, 'target': 'nope'}, BASE)
        self.assertEqual(len(p['mapping']), 3)
        with self.assertRaises(ValueError):
            hc.build_profile({'map': {'South': 'Banana'}, 'target': ''}, BASE)

    def test_conf_file(self):
        with tempfile.NamedTemporaryFile('w', suffix='.json', delete=False) as f:
            f.write('{"map": {"South": "East"}, "target": "deck"}')
        self.assertEqual(hc.load_conf(f.name), {'map': {'South': 'East'}, 'target': 'deck'})
        self.assertEqual(hc.load_conf('/nonexistent.json'), {'map': {}, 'target': ''})



class Choices(unittest.TestCase):
    def test_buttons_follow_capabilities(self):
        c = hc.choices(['LeftPaddle1'])
        ids = [b['id'] for b in c['buttons']]
        self.assertIn('South', ids)
        self.assertIn('LeftPaddle1', ids)
        self.assertNotIn('RightPaddle2', ids)
        self.assertTrue(all(hc.valid_target(t['id']) for t in c['targets']))
        self.assertFalse(hc.valid_target('Banana'))
        self.assertEqual(c['types'][0]['id'], '')

    def test_every_target_builds(self):
        for t in hc.choices([])['targets']:
            hc.target_events(t['id'])


if __name__ == '__main__':
    unittest.main()
