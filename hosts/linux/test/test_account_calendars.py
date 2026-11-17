"""Online accounts' calendars for Hearth's own apps (docs/ONLINE-ACCOUNTS.md): hearth-accounts' cal-objects, cal-edit,
cal-put and cal-delete, and `events` for the calendar widget, against the fake CalDAV server in fake_dav.py on
localhost only (never a real account, never a real secret; the keyring is a stand-in secret-tool).

Checked: an event changed with If-Match (and a change made elsewhere in the meantime is never overwritten: a
conflict comes back with the server's version); a new event only where there's none (If-None-Match); deleting;
"this time only" and "every time" for repeating events; nothing Hearth doesn't know about is lost; time zones
(VTIMEZONE written for a named zone); read-only calendars; the password only ever goes to the account's own server;
`events` with each event's zone, all-day, and repeating events expanded when the server didn't."""
import sys
sys.dont_write_bytecode = True
import datetime as dt
import json
import os
import pathlib
import subprocess
import tempfile
import unittest
from unittest import mock

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
sys.path.insert(0, str(SHELL))
sys.path.insert(0, str(HERE))
import fake_dav  # noqa: E402
import hearth_ical as hi  # noqa: E402

from test_online_accounts import FAKE_SECRET_TOOL  # noqa: E402


class Ical(unittest.TestCase):
    BASE = ('BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//EN\r\nBEGIN:VEVENT\r\nUID:u1\r\nDTSTART;TZID=Europe/Berlin:20261102T090000\r\n'
            'DTEND;TZID=Europe/Berlin:20261102T100000\r\nRRULE:FREQ=WEEKLY;BYDAY=MO\r\nSUMMARY:Standup\r\n'
            'ATTENDEE;CN="Ann, B";ROLE=REQ-PARTICIPANT:mailto:ann@example.org\r\nX-APPLE-STUFF:keep me\r\n'
            'BEGIN:VALARM\r\nTRIGGER;VALUE=DATE-TIME:20261101T080000Z\r\nACTION:DISPLAY\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n')

    def test_new_event_with_its_zone(self):
        r = hi.edit('', '', {'title': 'Lunch, with Sam', 'start': '2026-11-20T12:30:00', 'end': '2026-11-20T13:30:00',
                             'tz': 'America/Chicago', 'reminders': [10, 10, 0], 'location': 'Cafe'}, default_tz='')
        cal = hi.parse(r['ics'])
        ev = hi.master_of(cal)
        self.assertEqual(ev.value('UID'), r['uid'])
        self.assertEqual(ev.get('DTSTART'), ({'TZID': 'America/Chicago'}, '20261120T123000'))
        self.assertEqual(hi.text_unescape(ev.value('SUMMARY')), 'Lunch, with Sam')
        self.assertEqual(sorted(a.value('TRIGGER') for a in ev.subs), ['-PT0M', '-PT10M'])
        vt = [s for s in cal.subs if s.name == 'VTIMEZONE']
        self.assertEqual(len(vt), 1, 'a named zone needs its VTIMEZONE')
        kinds = sorted(s.name for s in vt[0].subs)
        self.assertEqual(kinds, ['DAYLIGHT', 'STANDARD'])
        self.assertIn('TZOFFSETTO:-0600', vt[0].write())
        self.assertTrue(all(len(l.encode()) <= 75 for l in r['ics'].split('\r\n')), 'lines are folded')

    def test_all_day_and_utc(self):
        r = hi.edit('', 'x', {'title': 'Trip', 'start': '2026-12-24', 'end': '2026-12-27', 'allDay': True})
        ev = hi.master_of(hi.parse(r['ics']))
        self.assertEqual(ev.get('DTSTART'), ({'VALUE': 'DATE'}, '20261224'))
        self.assertEqual(ev.get('DTEND'), ({'VALUE': 'DATE'}, '20261227'))
        self.assertNotIn('VTIMEZONE', r['ics'])
        r = hi.edit('', 'y', {'title': 'Call', 'start': '2026-12-01T15:00:00', 'end': '2026-12-01T15:30:00', 'tz': 'UTC'})
        self.assertEqual(hi.master_of(hi.parse(r['ics'])).value('DTSTART'), '20261201T150000Z')

    def test_every_time_keeps_what_hearth_doesnt_know(self):
        r = hi.edit(self.BASE, 'u1', {'title': 'Daily standup', 'reminders': [5]}, 'all')
        ev = hi.master_of(hi.parse(r['ics']))
        self.assertEqual(ev.value('SUMMARY'), 'Daily standup')
        self.assertIn('ATTENDEE;CN="Ann, B";ROLE=REQ-PARTICIPANT:mailto:ann@example.org', r['ics'].replace('\r\n ', ''))
        self.assertIn('X-APPLE-STUFF:keep me', r['ics'])
        self.assertEqual(ev.value('RRULE'), 'FREQ=WEEKLY;BYDAY=MO')
        self.assertEqual(ev.value('SEQUENCE'), '1')
        trig = sorted(a.value('TRIGGER') for a in ev.subs)
        self.assertEqual(trig, ['-PT5M', '20261101T080000Z'], 'a reminder at a set moment stays')

    def test_this_time_only_and_delete_one(self):
        r = hi.edit(self.BASE, 'u1', {'title': 'Standup (late)', 'start': '2026-11-09T11:00:00', 'end': '2026-11-09T11:30:00',
                                      'tz': 'Europe/Berlin', 'rrule': 'FREQ=DAILY'}, 'this', '2026-11-09T09:00:00')
        cal = hi.parse(r['ics'])
        evs = hi.events_of(cal, 'u1')
        self.assertEqual(len(evs), 2)
        o = hi.override_of(cal, 'u1', '2026-11-09T09:00:00')
        self.assertEqual(o.get('RECURRENCE-ID'), ({'TZID': 'Europe/Berlin'}, '20261109T090000'))
        self.assertIsNone(o.get('RRULE')[1], 'one time never repeats')
        self.assertEqual(o.value('SUMMARY'), 'Standup (late)')
        self.assertEqual(hi.master_of(cal).value('SUMMARY'), 'Standup')
        self.assertEqual(hi.master_of(cal).value('RRULE'), 'FREQ=WEEKLY;BYDAY=MO', 'the series is untouched')
        # Changing the same time again changes that copy (no second one).
        r = hi.edit(r['ics'], 'u1', {'title': 'Standup (later)'}, 'this', '2026-11-09T09:00:00')
        self.assertEqual(len(hi.events_of(hi.parse(r['ics']), 'u1')), 2)
        # Deleting that time: an EXDATE, and its changed copy goes.
        r = hi.edit(r['ics'], 'u1', {}, 'this', '2026-11-09T09:00:00', op='delete-one')
        cal = hi.parse(r['ics'])
        self.assertEqual(len(hi.events_of(cal, 'u1')), 1)
        self.assertEqual(hi.master_of(cal).all('EXDATE')[0][1], '20261109T090000')
        with self.assertRaises(ValueError):
            hi.edit(r['ics'], 'nope', {}, 'all')

    def test_expand(self):
        s = dt.datetime(2026, 11, 2, 9)
        got = hi.expand('FREQ=WEEKLY;BYDAY=MO,WE;COUNT=5', s, dt.datetime(2026, 1, 1), dt.datetime(2027, 1, 1), [dt.datetime(2026, 11, 4, 9)])
        self.assertEqual([g.day for g in got], [2, 9, 11, 16], 'COUNT counts the skipped one too')
        got = hi.expand('FREQ=MONTHLY;BYDAY=-1FR', dt.datetime(2026, 1, 30, 18), dt.datetime(2026, 1, 1), dt.datetime(2026, 5, 1))
        self.assertEqual([g.date().isoformat() for g in got], ['2026-01-30', '2026-02-27', '2026-03-27', '2026-04-24'])
        got = hi.expand('FREQ=DAILY;INTERVAL=2;UNTIL=20261110T000000Z', s, s, dt.datetime(2027, 1, 1))
        self.assertEqual([g.day for g in got], [2, 4, 6, 8])


class Calendars(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.srv, cls.store = fake_dav.serve(0, with_seed=True)
        cls.base = 'http://127.0.0.1:%d' % cls.srv.server_port

    @classmethod
    def tearDownClass(cls):
        cls.srv.shutdown()

    def setUp(self):
        self.store.objects.clear()
        fake_dav.seed(self.store)
        self.tmp = tempfile.mkdtemp()
        st = os.path.join(self.tmp, 'secret-tool')
        with open(st, 'w') as f:
            f.write(FAKE_SECRET_TOOL)
        os.chmod(st, 0o755)
        self.keyring = os.path.join(self.tmp, 'keyring.json')
        self.env = dict(os.environ, HEARTH_SECRET_TOOL=st, FAKE_KEYRING=self.keyring,
                        HEARTH_ACCOUNTS_FILE=os.path.join(self.tmp, 'online-accounts.json'),
                        XDG_STATE_HOME=os.path.join(self.tmp, 'state'), HEARTH_INPUT='', TZ='America/Chicago')
        out, err = self.cli('add-dav', inp={'server': self.base + '/', 'user': fake_dav.USER, 'password': fake_dav.PASS})
        self.assertEqual(err, '')
        self.aid = out['account']['id']

    def cli(self, *args, inp=None):
        text = json.dumps(inp) if inp is not None else ''
        r = subprocess.run([sys.executable, str(SHELL / 'hearth_accounts.py')] + list(args), input=text, capture_output=True,
                           text=True, env=dict(self.env, HEARTH_INPUT=text), timeout=60)
        self.assertNotIn(fake_dav.PASS, r.stdout + r.stderr, 'the password was printed')
        lines = [l for l in r.stdout.splitlines() if l.startswith('{')]
        err = next((l[7:] for l in r.stdout.splitlines() if l.startswith('error: ')), '')
        return (json.loads(lines[-1]) if lines else None), err

    def personal(self):
        return self.base + fake_dav.HOME + 'personal/'

    def test_calendars_say_which_can_be_changed(self):
        out, err = self.cli('calendars')
        self.assertEqual(err, '')
        cals = {c['name']: c for c in out['calendars']}
        self.assertEqual(set(cals), {'Personal', 'Holidays'})
        self.assertFalse(cals['Personal']['readOnly'])
        self.assertTrue(cals['Holidays']['readOnly'])
        self.assertEqual(cals['Personal']['account'], self.aid)
        self.assertEqual(cals['Personal']['color'], '#0082C9')

    def test_objects_edit_put_conflict_delete(self):
        out, err = self.cli('cal-objects', self.aid, self.personal())
        self.assertEqual(err, '')
        objs = {o['href'].rsplit('/', 1)[-1]: o for o in out['objects']}
        self.assertEqual(set(objs), {'dentist.ics', 'team.ics', 'trip.ics'})
        d = objs['dentist.ics']
        self.assertTrue(d['href'].startswith(self.base) and d['etag'])
        # Changed here: made with cal-edit, sent with the etag it was made on.
        ed, err = self.cli('cal-edit', inp={'ics': d['ics'], 'uid': 'dentist@fake', 'change': {'title': 'Dentist (check-up)'}, 'scope': 'all'})
        self.assertEqual(err, '')
        self.assertIn('X-OTHER-APP:kept as it was', ed['ics'])
        self.assertIn('ATTENDEE;CN=Dr Lee:mailto:lee@example.org', ed['ics'])
        put, err = self.cli('cal-put', self.aid, inp={'url': d['href'], 'ics': ed['ics'], 'etag': d['etag']})
        self.assertEqual(err, '')
        self.assertTrue(put['ok'])
        self.assertNotEqual(put['etag'], d['etag'])
        self.assertIn('SUMMARY:Dentist (check-up)', self.store.objects[fake_dav.HOME + 'personal/dentist.ics'][1])
        self.assertEqual(self.store.log[-1], ('PUT', fake_dav.HOME + 'personal/dentist.ics', d['etag']), 'sent without If-Match')
        # Sent again with the old etag (someone else's change came in between): never overwritten.
        before = self.store.objects[fake_dav.HOME + 'personal/dentist.ics'][1]
        again, err = self.cli('cal-put', self.aid, inp={'url': d['href'], 'ics': d['ics'], 'etag': d['etag']})
        self.assertEqual(err, '')
        self.assertTrue(again['conflict'])
        self.assertFalse(again['gone'])
        self.assertEqual(again['server']['etag'], put['etag'])
        self.assertIn('Dentist (check-up)', again['server']['ics'])
        self.assertEqual(self.store.objects[fake_dav.HOME + 'personal/dentist.ics'][1], before)
        # Deleting with an old etag: a conflict too; with the right one it goes.
        gone, _ = self.cli('cal-delete', self.aid, inp={'url': d['href'], 'etag': d['etag']})
        self.assertTrue(gone['conflict'])
        ok, _ = self.cli('cal-delete', self.aid, inp={'url': d['href'], 'etag': put['etag']})
        self.assertTrue(ok['ok'])
        self.assertNotIn(fake_dav.HOME + 'personal/dentist.ics', self.store.objects)
        # Sending a change to an event deleted elsewhere: "gone", not re-made silently.
        g, _ = self.cli('cal-put', self.aid, inp={'url': d['href'], 'ics': ed['ics'], 'etag': put['etag']})
        self.assertTrue(g['conflict'] and g['gone'])

    def test_new_event_only_where_there_is_none(self):
        ed, _ = self.cli('cal-edit', inp={'ics': '', 'change': {'title': 'Haircut', 'start': '2026-11-30T16:00:00', 'end': '2026-11-30T16:45:00'}})
        url = self.personal() + 'haircut.ics'
        put, err = self.cli('cal-put', self.aid, inp={'url': url, 'ics': ed['ics'], 'etag': ''})
        self.assertEqual(err, '')
        self.assertTrue(put['ok'])
        self.assertEqual(self.store.log[-1][2], '*')
        self.assertIn('TZID=America/Chicago', self.store.objects[fake_dav.HOME + 'personal/haircut.ics'][1], "the device's zone")
        again, _ = self.cli('cal-put', self.aid, inp={'url': url, 'ics': ed['ics'], 'etag': ''})
        self.assertTrue(again['conflict'], 'a new event replaced one that was there')

    def test_read_only_calendar_refuses(self):
        ed, _ = self.cli('cal-edit', inp={'ics': '', 'change': {'title': 'Mine', 'start': '2026-11-30', 'end': '2026-12-01', 'allDay': True}})
        r, err = self.cli('cal-put', self.aid, inp={'url': self.base + fake_dav.HOME + 'holidays/mine.ics', 'ics': ed['ics'], 'etag': ''})
        self.assertEqual(err, '')
        self.assertTrue(r['refused'])
        self.assertIn('403', r['message'])

    def test_password_only_goes_to_the_accounts_server(self):
        for cmd, inp in (('cal-put', {'url': 'https://evil.example/x.ics', 'ics': 'BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n'}),
                         ('cal-delete', {'url': 'http://127.0.0.1:1/x.ics', 'etag': '"1"'})):
            out, err = self.cli(cmd, self.aid, inp=inp)
            self.assertIsNone(out)
            self.assertIn("isn't on this account's server", err)
        out, err = self.cli('cal-objects', self.aid, 'https://evil.example/cal/')
        self.assertIn("isn't on this account's server", err)

    def test_events_for_the_widget(self):
        mon = dt.date.today() - dt.timedelta(days=dt.date.today().weekday())
        out, err = self.cli('events', '--from', (mon - dt.timedelta(days=7)).isoformat(), '--days', '28')
        self.assertEqual(err, '')
        ev = out['events']
        dentist = next(e for e in ev if e['title'] == 'Dentist')
        self.assertEqual(dentist['tz'], 'America/New_York')
        self.assertFalse(dentist['allDay'])
        trip = next(e for e in ev if e['title'] == 'Lisbon trip')
        self.assertTrue(trip['allDay'])
        self.assertEqual(trip['tz'], '')
        self.assertEqual((trip['date'], trip['lastDate']), ((dt.date.today() + dt.timedelta(days=4)).isoformat(),
                                                            (dt.date.today() + dt.timedelta(days=6)).isoformat()))
        # The fake server never expands: Hearth does, with the skipped time left out and the moved one moved.
        team = sorted((e for e in ev if e['title'].startswith('Team meeting')), key=lambda e: e['start'])
        when = [(dt.datetime.fromtimestamp(e['start'] / 1000, dt.timezone.utc).strftime('%m-%d %H'), e['title']) for e in team]
        f = lambda d, h: (d.strftime('%m-%d') + ' %02d' % h)
        self.assertEqual(when, [(f(mon - dt.timedelta(days=7), 15), 'Team meeting'),
                                (f(mon + dt.timedelta(days=8), 17), 'Team meeting (moved)'),
                                (f(mon + dt.timedelta(days=14), 15), 'Team meeting')], 'skipped and moved times')
        self.assertTrue(all(e['recurring'] for e in team))
        self.assertEqual(team[0]['tz'], 'UTC')
        for k in ('title', 'date', 'lastDate', 'allDay', 'start', 'end', 'location', 'color', 'tz', 'calendar', 'account', 'uid'):
            self.assertIn(k, dentist)


if __name__ == '__main__':
    unittest.main()
