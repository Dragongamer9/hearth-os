"""1.13 Server mode › Quiet hours and "When this, do that" rules (hearth-server-rules): nothing on until you make it;
quiet hours cap fans and power, make pictures and dubs wait and halve downloads, never while you play; rules fire on
what they watch and can be paused; fans are given back the moment anything gets hot; what waits is read by the home
server's services (rule_wait) and Voice Studio; settings are saved safely."""
import sys
sys.dont_write_bytecode = True
import datetime
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import shutil
import tempfile
import time
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
LIB = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'


def load(name):
    loader = importlib.machinery.SourceFileLoader(name.replace('-', '_'), str(LIB / name))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


R = load('hearth-server-rules')
F = {'watching': False, 'game': False, 'online': set(), 'drive': 30, 'power': 100}


def at(day, hhmm):
    """2026-11-16 is a Monday."""
    h, m = map(int, hhmm.split(':'))
    return datetime.datetime(2026, 11, 16 + day, h, m)


class Base(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.mkdtemp(prefix='hearth-rules-')
        R.CONF = os.path.join(self.t, 'etc', 'server-rules.json')
        R.STATE = os.path.join(self.t, 'state.json')
        R.NOW = os.path.join(self.t, 'run', 'now.json')
        R.HWMON = os.path.join(self.t, 'hwmon')
        R.SYSTEMCTL = R.NFT = R.HS_HELPER = 'true'
        os.environ['HEARTH_RULES_TEST'] = '1'

    def tearDown(self):
        shutil.rmtree(self.t, ignore_errors=True)

    def conf(self, quiet=(), rules=()):
        c = R.read_conf()[0]
        c['quiet'], c['rules'] = list(quiet), list(rules)
        return c


class Times(Base):
    def test_windows_and_days(self):
        self.assertEqual(R.parse_days('mon-fri'), [0, 1, 2, 3, 4])
        self.assertEqual(R.parse_days('sat,sun'), [5, 6])
        self.assertEqual(R.parse_days('fri-mon'), [0, 4, 5, 6])
        self.assertEqual(R.days_text([0, 1, 2, 3, 4]), 'Mon–Fri')
        self.assertEqual(R.days_text([6]), 'Sun')
        wk = [0, 1, 2, 3, 4]
        self.assertTrue(R.in_window('23:00', '07:00', wk, at(0, '23:30')))       # Monday night
        self.assertTrue(R.in_window('23:00', '07:00', wk, at(1, '06:59')))       # ... into Tuesday morning
        self.assertTrue(R.in_window('23:00', '07:00', wk, at(5, '06:00')))       # Friday night into Saturday
        self.assertFalse(R.in_window('23:00', '07:00', wk, at(5, '23:30')))      # Saturday night isn't a weeknight
        self.assertFalse(R.in_window('23:00', '07:00', wk, at(0, '07:00')))
        for bad in ('25:00', '7', ''):
            self.assertRaises(R.Fail, R.hm, bad)


class Deciding(Base):
    def test_nothing_until_you_make_it(self):
        d = R.decide(R.read_conf()[0], dict(F), at(0, '23:30'), time.time())
        self.assertEqual((d['wait'], d['fans'], d['powerW'], d['downloads'], d['quiet']), ([], None, None, False, ''))

    def test_quiet_hours(self):
        q = {'id': '1', 'name': 'Weeknights', 'from': '23:00', 'to': '07:00', 'days': [0, 1, 2, 3, 4], 'on': True}
        d = R.decide(self.conf([q]), dict(F), at(0, '23:30'), time.time())
        self.assertEqual(d['quiet'], 'Weeknights')
        self.assertEqual((d['fans'], d['powerW'], d['downloads']), (40, 150, True))
        self.assertEqual(d['wait'], ['pictures', 'voices'])
        self.assertEqual(datetime.datetime.fromtimestamp(d['quietUntil']), at(1, '07:00'))
        self.assertEqual(R.decide(self.conf([dict(q, on=False)]), dict(F), at(0, '23:30'), time.time())['quiet'], '')
        d = R.decide(self.conf([q]), dict(F, game=True), at(0, '23:30'), time.time())   # you're playing: full speed
        self.assertEqual((d['quiet'], d['fans'], d['powerW'], d['downloads']), ('', None, None, False))

    def test_rules_fire_and_pause(self):
        now = time.time()
        rules = [{'id': '1', 'when': 'jellyfin', 'do': 'pictures', 'on': True},
                 {'id': '2', 'when': 'game', 'do': 'downloads', 'on': True},
                 {'id': '3', 'when': 'drive:50', 'do': 'backups', 'on': True, 'pausedUntil': now + 600},
                 {'id': '4', 'when': 'device:ally', 'do': 'phone', 'on': True}]
        d = R.decide(self.conf(rules=rules), dict(F, watching=True, drive=55, online={'ally'}), at(0, '12:00'), now)
        self.assertEqual(d['wait'], ['pictures'])
        self.assertEqual(d['firing'], ['1', '4'])                                 # 3 is paused
        self.assertFalse(d['downloads'])
        self.assertIn('someone is watching Jellyfin → AI pictures wait', d['why'])
        d = R.decide(self.conf(rules=rules), dict(F, game=True), at(0, '12:00'), now)
        self.assertEqual(d['firing'], ['2'])
        self.assertFalse(d['downloads'])                                          # never slows your own game
        d = R.decide(self.conf(rules=rules), dict(F, drive=55), at(0, '12:00'), now + 700)
        self.assertEqual(d['wait'], ['backups'])                                  # the pause ended by itself

    def test_texts_and_checks(self):
        self.assertEqual(R.when_text('drive:50'), 'a drive is over 50 °C')
        self.assertEqual(R.do_text('fans:40'), 'fans capped at 40 %')
        for w in ('jellyfin', 'game', 'device:ROG-Ally', 'time:23:00-07:00', 'power:300'):
            self.assertEqual(R.check_when(w), w)
        for w in ('device:../x', 'time:9-5', 'drive:5', 'rm -rf'):
            self.assertRaises(R.Fail, R.check_when, w)
        for d in ('fans:10', 'nuke', 'phone:x'):
            self.assertRaises(R.Fail, R.check_do, d)


class Acting(Base):
    def fan(self, enable='2', temp=40000):
        d = os.path.join(R.HWMON, 'hwmon0')
        os.makedirs(d, exist_ok=True)
        for n, v in (('pwm1', '200'), ('pwm1_enable', enable), ('temp1_input', str(temp))):
            with open(os.path.join(d, n), 'w') as f:
                f.write(v)
        return d

    def test_fans_capped_while_cool_and_given_back(self):
        d = self.fan()
        st = {}
        self.assertTrue(R.fans_cap(st, 40))
        self.assertEqual(open(os.path.join(d, 'pwm1_enable')).read(), '1')
        self.assertEqual(open(os.path.join(d, 'pwm1')).read(), '102')
        with open(os.path.join(d, 'temp1_input'), 'w') as f:
            f.write('80000')                                                      # hot: back to the firmware now
        R.fans_cap(st, 40)
        self.assertEqual(open(os.path.join(d, 'pwm1_enable')).read(), '2')
        self.assertNotIn('fanEnable', st)
        st = {}
        with open(os.path.join(d, 'temp1_input'), 'w') as f:
            f.write('40000')
        R.fans_cap(st, 20)
        self.assertEqual(open(os.path.join(d, 'pwm1')).read(), '80')              # never below about a third
        R.fans_release(st)
        self.assertEqual(open(os.path.join(d, 'pwm1_enable')).read(), '2')

    def test_settings_saved_safely_and_commands_checked(self):
        self.assertEqual(R.main(['x', 'quiet-add', 'Weeknights', '23:00', '07:00', 'mon-fri']), 0)
        c = json.load(open(R.CONF))
        self.assertEqual(c['quiet'][0], {'id': '1', 'name': 'Weeknights', 'from': '23:00', 'to': '07:00', 'days': [0, 1, 2, 3, 4], 'on': False})
        self.assertNotIn('while', c)                                              # defaults never saved
        self.assertEqual(R.main(['x', 'rule-add', 'jellyfin', 'pictures']), 0)
        self.assertEqual(R.main(['x', 'rule-set', '1', 'on']), 0)
        self.assertTrue(json.load(open(R.CONF))['rules'][0]['on'])
        self.assertTrue(os.path.exists(R.CONF + '.bak'))
        for bad in (['quiet-add', 'x', '23:00', '7', 'mon'], ['rule-add', 'game', 'nuke'], ['while', 'fans', '5'],
                    ['rule-set', '9', 'on'], ['while', 'powerW', 'lots']):
            self.assertNotEqual(R.main(['x'] + bad), 0, bad)
        with open(R.CONF, 'w') as f:
            f.write('{broken')
        self.assertFalse(R.read_conf()[1])
        self.assertEqual(open(R.CONF).read(), '{broken')
        R.main(['x', 'rule-add', 'game', 'downloads'])
        self.assertEqual(len([n for n in os.listdir(os.path.dirname(R.CONF)) if '.unreadable-' in n]), 1)

    def test_tick_writes_what_waits(self):
        R.main(['x', 'rule-add', 'time:00:00-23:59', 'voices'])
        R.main(['x', 'rule-set', '1', 'on'])
        now = json.load(open(R.NOW))
        self.assertEqual(now['wait'], ['voices'])
        hs = load('hearth-home-server-helper')
        hs.RULES_NOW = R.NOW
        self.assertTrue(hs.rule_wait('voices'))
        self.assertFalse(hs.rule_wait('pictures'))
        now['at'] = time.time() - 600
        with open(R.NOW, 'w') as f:
            json.dump(now, f)
        self.assertFalse(hs.rule_wait('voices'))                                  # an old file means nothing waits


if __name__ == '__main__':
    unittest.main()
