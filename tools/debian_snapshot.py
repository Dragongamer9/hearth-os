#!/usr/bin/env python3
"""Smaller updates (1.13): the date Debian's packages are frozen to for an image build (snapshot.debian.org), and the
fixed time the rechunk step stamps on files without one of their own (chunkah's SOURCE_DATE_EPOCH).

It moves by itself once a week (the newest Monday, 00:00 UTC), so every build in the same week gets byte-for-byte the
same Debian packages and an update only carries Hearth's own changes. For a security fix sooner, put a newer date in
os-bases/peppermint/debian-snapshot (YYYYMMDD, or YYYYMMDDTHHMMSSZ); a date older than this week's Monday is ignored.
"live" there turns the freeze off (Debian's live repositories, as before 1.13).

  debian_snapshot.py            prints DEBIAN_SNAPSHOT=... and SOURCE_DATE_EPOCH=... (for $GITHUB_ENV)
  debian_snapshot.py --json     the same as JSON, with why
"""
import calendar
import datetime
import json
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
PIN = os.path.join(ROOT, 'os-bases', 'peppermint', 'debian-snapshot')
FMT = '%Y%m%dT%H%M%SZ'


def monday(now):
    d = now.astimezone(datetime.timezone.utc).date()
    d = d - datetime.timedelta(days=d.weekday())
    return datetime.datetime(d.year, d.month, d.day, tzinfo=datetime.timezone.utc)


def parse(text):
    lines = [l.split('#')[0].strip() for l in (text or '').splitlines()]
    t = next((l for l in lines if l), '')
    if t.lower() == 'live':
        return 'live'
    m = re.fullmatch(r'(\d{8})(?:T(\d{6})Z)?', t)
    if not m:
        return None
    try:
        return datetime.datetime.strptime(m.group(1) + 'T' + (m.group(2) or '000000') + 'Z', FMT).replace(tzinfo=datetime.timezone.utc)
    except ValueError:
        return None


def choose(now=None, pin_text=None):
    now = now or datetime.datetime.now(datetime.timezone.utc)
    if pin_text is None:
        try:
            with open(PIN) as f:
                pin_text = f.read()
        except OSError:
            pin_text = ''
    week = monday(now)
    pin = parse(pin_text)
    if pin == 'live':
        return {'snapshot': '', 'epoch': calendar.timegm(week.timetuple()), 'why': 'the freeze is off (live repositories)'}
    when, why = week, "this week's Monday"
    if isinstance(pin, datetime.datetime) and week < pin <= now:
        when, why = pin, 'the date in os-bases/peppermint/debian-snapshot (a security fix)'
    return {'snapshot': when.strftime(FMT), 'epoch': calendar.timegm(when.timetuple()), 'why': why}


def main(argv):
    c = choose()
    if '--json' in argv:
        print(json.dumps(c))
    else:
        print('DEBIAN_SNAPSHOT=%s' % c['snapshot'])
        print('SOURCE_DATE_EPOCH=%d' % c['epoch'])
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
