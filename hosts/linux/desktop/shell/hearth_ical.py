"""hearth_ical: enough iCalendar (RFC 5545) to read and change one event on a CalDAV server, for hearth-accounts
(hearth_accounts.py: cal-edit, events) and through it Hearth Time's Calendar (docs/ONLINE-ACCOUNTS.md).

An event object on a CalDAV server is one VCALENDAR with one UID: the event (the "master", with its RRULE when it
repeats), and any number of changed single times (VEVENTs with a RECURRENCE-ID). Changing it keeps everything Hearth
doesn't know about (attendees, categories, other apps' X- properties, VTIMEZONEs) exactly as the server sent it:
only the lines Hearth changes are written again.

Times in and out are Hearth Calendar's form: "2026-10-22T09:30:00" (wall clock) with a zone (an IANA name, "UTC", or
none: floating), or "2026-10-22" for all-day events (the end date not included).

Python standard library only.
"""
import datetime as dt
import os
import re
import uuid

PRODID = '-//Hearth OS//Hearth Calendar//EN'


# ------------------------------------------------------------------ lines and components

def unfold(text):
    return re.sub(r'\r?\n[ \t]', '', (text or '').replace('\r\n', '\n')).split('\n')


def split_line(line):
    """'NAME;A=1;B="x:y":value' -> ('NAME', [('A', '1'), ('B', 'x:y')], 'value'). The value starts at the first ':'
    outside quotes."""
    q, cut = False, len(line)
    for i, c in enumerate(line):
        if c == '"':
            q = not q
        elif c == ':' and not q:
            cut = i
            break
    head, value = line[:cut], line[cut + 1:] if cut < len(line) else ''
    parts = head.split(';')
    params = []
    for p in parts[1:]:
        k, _, v = p.partition('=')
        params.append((k.strip().upper(), v.strip().strip('"')))
    return parts[0].strip().upper(), params, value


def join_line(name, params, value):
    ps = ''.join(';%s=%s' % (k, '"%s"' % v if re.search(r'[:;,]', v) else v) for k, v in params)
    return '%s%s:%s' % (name, ps, value)


def fold(line):
    """At 75 octets, never inside a character."""
    out, n = [], 0
    for ch in line:
        b = len(ch.encode('utf-8'))
        if n + b > 75:
            out.append('\r\n ')
            n = 1
        out.append(ch)
        n += b
    return ''.join(out) + '\r\n'


class Comp:
    """A component: its property lines (kept as written) and the components inside it."""

    def __init__(self, name):
        self.name = name
        self.lines = []          # [name, raw line]
        self.subs = []

    # reading
    def get(self, name):
        for n, raw in self.lines:
            if n == name:
                _, params, value = split_line(raw)
                return dict(params), value
        return None, None

    def value(self, name, default=''):
        return self.get(name)[1] if self.get(name)[1] is not None else default

    def all(self, name):
        return [split_line(raw)[1:] for n, raw in self.lines if n == name]

    # changing
    def remove(self, name):
        self.lines = [x for x in self.lines if x[0] != name]

    def set(self, name, value, params=()):
        """One line NAME, in the place of the first one there was (or at the end)."""
        raw = join_line(name, list(params), value)
        for i, x in enumerate(self.lines):
            if x[0] == name:
                self.lines[i] = [name, raw]
                self.lines = [y for j, y in enumerate(self.lines) if j <= i or y[0] != name]
                return
        self.lines.append([name, raw])

    def add(self, name, value, params=()):
        self.lines.append([name, join_line(name, list(params), value)])

    def write(self):
        s = 'BEGIN:%s\r\n' % self.name
        for _, raw in self.lines:
            s += fold(raw)
        for c in self.subs:
            s += c.write()
        return s + 'END:%s\r\n' % self.name

    def copy(self):
        c = Comp(self.name)
        c.lines = [list(x) for x in self.lines]
        c.subs = [s.copy() for s in self.subs]
        return c


def parse(text):
    """The VCALENDAR in TEXT (a new, empty one when there's none)."""
    root, stack = None, []
    for line in unfold(text):
        if not line.strip():
            continue
        name, _, value = split_line(line)
        if name == 'BEGIN':
            c = Comp(value.strip().upper())
            if stack:
                stack[-1].subs.append(c)
            elif root is None:
                root = c
            else:
                continue
            stack.append(c)
        elif name == 'END':
            if stack:
                stack.pop()
        elif stack:
            stack[-1].lines.append([name, line])
    if root is None or root.name != 'VCALENDAR':
        root = Comp('VCALENDAR')
        root.add('VERSION', '2.0')
        root.add('PRODID', PRODID)
    return root


# ------------------------------------------------------------------ times

def text_escape(s):
    return (s or '').replace('\\', '\\\\').replace(';', '\\;').replace(',', '\\,').replace('\r\n', '\\n').replace('\n', '\\n')


def text_unescape(v):
    return re.sub(r'\\([\\;,nN])', lambda m: '\n' if m.group(1) in 'nN' else m.group(1), v or '')


def read_when(params, value):
    """A DTSTART/DTEND/RECURRENCE-ID/EXDATE value as (Hearth's string, all_day, zone)."""
    v = (value or '').strip()
    params = dict(params or {})
    if params.get('VALUE', '').upper() == 'DATE' or re.fullmatch(r'\d{8}', v):
        return '%s-%s-%s' % (v[:4], v[4:6], v[6:8]), True, None
    m = re.fullmatch(r'(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})(\d{2})(Z?)', v)
    if not m:
        return None, False, None
    s = '%s-%s-%sT%s:%s:%s' % m.groups()[:6]
    if m.group(7):
        return s, False, 'UTC'
    return s, False, params.get('TZID') or None


def local_zone():
    """This device's IANA zone name (TZ, then /etc/localtime), or '' when it can't be told."""
    tz = os.environ.get('TZ', '').lstrip(':')
    if tz and '/' in tz and not tz.startswith('/'):
        return tz
    try:
        p = os.path.realpath('/etc/localtime')
        if '/zoneinfo/' in p:
            return p.split('/zoneinfo/', 1)[1]
    except OSError:
        pass
    return ''


def zone(name):
    if not name or name == 'UTC':
        return dt.timezone.utc if name == 'UTC' else None
    try:
        from zoneinfo import ZoneInfo
        return ZoneInfo(name)
    except Exception:            # noqa: BLE001 (an unknown name)
        return None


def write_when(s, all_day, tz):
    """Hearth's string to (params, value)."""
    if all_day:
        return [('VALUE', 'DATE')], s[:10].replace('-', '')
    v = s[:19].replace('-', '').replace(':', '')
    if tz == 'UTC':
        return [], v + 'Z'
    if tz:
        return [('TZID', tz)], v
    return [], v


def same_when(a, b):
    """Two Hearth strings for the same time (a date matches its midnight)."""
    if not a or not b:
        return False
    return a[:19] == b[:19] or (len(a) == 10 or len(b) == 10) and a[:10] == b[:10]


def vtimezone(name, year):
    """A VTIMEZONE for NAME made from this device's time zone data: its changes in YEAR as yearly rules."""
    z = zone(name)
    if z is None or name == 'UTC':
        return None
    t = dt.datetime(year, 1, 1, tzinfo=dt.timezone.utc)
    end = dt.datetime(year + 1, 1, 1, tzinfo=dt.timezone.utc)
    prev = t.astimezone(z)
    changes = []
    while t < end:
        t += dt.timedelta(hours=1)
        cur = t.astimezone(z)
        if cur.utcoffset() != prev.utcoffset():
            changes.append((prev.utcoffset(), cur.utcoffset(), bool(cur.dst()), (t + prev.utcoffset()).replace(tzinfo=None), cur.tzname()))
        prev = cur
    off = lambda d: ('+' if d >= dt.timedelta(0) else '-') + '%02d%02d' % divmod(abs(int(d.total_seconds())) // 60, 60)
    c = Comp('VTIMEZONE')
    c.add('TZID', name)
    if not changes:
        s = Comp('STANDARD')
        o = dt.datetime(year, 1, 1, tzinfo=dt.timezone.utc).astimezone(z)
        s.add('DTSTART', '19700101T000000')
        s.add('TZOFFSETFROM', off(o.utcoffset()))
        s.add('TZOFFSETTO', off(o.utcoffset()))
        s.add('TZNAME', o.tzname() or name)
        c.subs.append(s)
        return c
    for frm, to, dst, wall, tzname in changes:
        s = Comp('DAYLIGHT' if dst else 'STANDARD')
        s.add('DTSTART', wall.strftime('%Y%m%dT%H%M%S'))
        nth = (wall.day - 1) // 7 + 1
        days_in = ((wall.replace(day=28) + dt.timedelta(days=4)).replace(day=1) - dt.timedelta(days=1)).day
        if wall.day + 7 > days_in:
            nth = -1
        s.add('RRULE', 'FREQ=YEARLY;BYMONTH=%d;BYDAY=%d%s' % (wall.month, nth, ['MO', 'TU', 'WE', 'TH', 'FR', 'SA', 'SU'][wall.weekday()]))
        s.add('TZOFFSETFROM', off(frm))
        s.add('TZOFFSETTO', off(to))
        if tzname:
            s.add('TZNAME', tzname)
        c.subs.append(s)
    return c


def ensure_vtimezone(cal, name, year):
    if not name or name == 'UTC':
        return
    if any(s.name == 'VTIMEZONE' and s.value('TZID') == name for s in cal.subs):
        return
    vt = vtimezone(name, year)
    if vt is not None:
        cal.subs.insert(0, vt)


# ------------------------------------------------------------------ changing an event

def _stamp():
    return dt.datetime.now(dt.timezone.utc).strftime('%Y%m%dT%H%M%SZ')


def events_of(cal, uid=None):
    return [s for s in cal.subs if s.name == 'VEVENT' and (uid is None or s.value('UID') == uid)]


def master_of(cal, uid=None):
    evs = events_of(cal, uid)
    return next((e for e in evs if e.get('RECURRENCE-ID')[1] is None), evs[0] if evs else None)


def override_of(cal, uid, occ):
    for e in events_of(cal, uid):
        p, v = e.get('RECURRENCE-ID')
        if v is not None and same_when(read_when(p, v)[0], occ):
            return e
    return None


def _rid_like(master, occ):
    """OCC written the way the master writes its DTSTART (so servers and other apps match it)."""
    p, v = master.get('DTSTART')
    _, all_day, tz = read_when(p, v)
    if all_day:
        return [('VALUE', 'DATE')], occ[:10].replace('-', '')
    if len(occ) == 10:
        occ += 'T' + read_when(p, v)[0][11:19]
    return write_when(occ, False, tz)


def _apply(ev, change, cal, may_repeat, default_tz):
    if 'title' in change:
        ev.set('SUMMARY', text_escape((change.get('title') or '').strip()))
    for key, prop in (('location', 'LOCATION'), ('notes', 'DESCRIPTION')):
        if key in change:
            if (change.get(key) or '').strip():
                ev.set(prop, text_escape(change[key]))
            else:
                ev.remove(prop)
    if change.get('start'):
        all_day = bool(change.get('allDay'))
        tz = None if all_day else (change.get('tz') or default_tz or None)
        if tz and tz != 'UTC' and zone(tz) is None:
            tz = default_tz or None
        p, v = write_when(change['start'], all_day, tz)
        ev.set('DTSTART', v, p)
        end = change.get('end') or ''
        if end:
            p, v = write_when(end, all_day, tz)
            ev.set('DTEND', v, p)
        else:
            ev.remove('DTEND')
        ev.remove('DURATION')
        ensure_vtimezone(cal, tz, int(change['start'][:4]))
    if may_repeat and 'rrule' in change:
        r = (change.get('rrule') or '').strip()
        if r:
            ev.set('RRULE', r)
        else:
            for name in ('RRULE', 'RDATE', 'EXDATE'):
                ev.remove(name)
    if 'reminders' in change and isinstance(change.get('reminders'), list):
        # Reminders at a set moment (TRIGGER;VALUE=DATE-TIME) aren't Hearth's to change: they stay.
        ev.subs = [a for a in ev.subs if a.name != 'VALARM' or (a.get('TRIGGER')[0] or {}).get('VALUE', '').upper() == 'DATE-TIME']
        for m in sorted({int(x) for x in change['reminders'] if isinstance(x, (int, float)) and 0 <= x <= 40320}):
            a = Comp('VALARM')
            a.add('ACTION', 'DISPLAY')
            a.add('DESCRIPTION', ev.value('SUMMARY') or 'Reminder')
            a.add('TRIGGER', '-PT%dM' % m)
            ev.subs.append(a)
    seq = ev.value('SEQUENCE', '0')
    ev.set('SEQUENCE', str(int(seq) + 1 if seq.isdigit() else 1))
    ev.set('DTSTAMP', _stamp())
    ev.set('LAST-MODIFIED', _stamp())


def edit(ics, uid='', change=None, scope='all', occ='', op='save', default_tz=None):
    """Changes the event UID in ICS (a whole CalDAV object; empty for a new event) and returns {ics, uid}.
      op 'save'        CHANGE ({title, start, end, allDay, tz, location, notes, rrule, reminders}; keys left out stay)
                       to the whole event (scope 'all') or to the one time OCC of a repeating event (scope 'this':
                       a RECURRENCE-ID copy, made the first time).
      op 'delete-one'  just the time OCC of a repeating event goes (an EXDATE; a changed copy of it goes too).
    Deleting the whole event is deleting the object (hearth-accounts cal-delete)."""
    change = dict(change or {})
    default_tz = local_zone() if default_tz is None else default_tz
    cal = parse(ics) if (ics or '').strip() else None
    if cal is None:
        if op != 'save':
            raise ValueError('There is no such event.')
        cal = parse('')
        uid = uid or '%s@hearth' % uuid.uuid4()
        ev = Comp('VEVENT')
        ev.add('UID', uid)
        ev.add('DTSTAMP', _stamp())
        ev.add('CREATED', _stamp())
        cal.subs.append(ev)
        _apply(ev, change, cal, True, default_tz)
        return {'ics': cal.write(), 'uid': uid}
    master = master_of(cal, uid or None)
    if master is None:
        raise ValueError("That event isn't in the calendar any more.")
    uid = master.value('UID')
    repeats = master.get('RRULE')[1] is not None
    if op == 'delete-one':
        if not occ or not repeats:
            raise ValueError('Only one time of a repeating event can be deleted on its own.')
        p, v = _rid_like(master, occ)
        if not any(same_when(read_when(pp, x)[0], occ) for pp, vv in master.all('EXDATE') for x in vv.split(',')):
            master.add('EXDATE', v, p)
        o = override_of(cal, uid, occ)
        if o is not None:
            cal.subs.remove(o)
        master.set('DTSTAMP', _stamp())
        seq = master.value('SEQUENCE', '0')
        master.set('SEQUENCE', str(int(seq) + 1 if seq.isdigit() else 1))
        return {'ics': cal.write(), 'uid': uid}
    if op != 'save':
        raise ValueError('Unknown change.')
    if scope == 'this' and occ and (repeats or override_of(cal, uid, occ) is not None):
        ev = override_of(cal, uid, occ)
        if ev is None:
            ev = master.copy()
            for name in ('RRULE', 'RDATE', 'EXDATE', 'RECURRENCE-ID'):
                ev.remove(name)
            p, v = _rid_like(master, occ)
            ev.lines.insert(1, ['RECURRENCE-ID', join_line('RECURRENCE-ID', p, v)])
            cal.subs.append(ev)
        change.pop('rrule', None)
        _apply(ev, change, cal, False, default_tz)
    else:
        _apply(master, change, cal, True, default_tz)
    return {'ics': cal.write(), 'uid': uid}


# ------------------------------------------------------------------ reading, for checks and the widget's fallback

WEEKDAYS = ['MO', 'TU', 'WE', 'TH', 'FR', 'SA', 'SU']


def expand(rule, start, after, before, exdates=(), limit=2000):
    """The starts of a repeating event (RRULE text, its first start a naive datetime) in [after, before), for a
    server that sent the event without expanding it. FREQ DAILY/WEEKLY/MONTHLY/YEARLY with INTERVAL, COUNT, UNTIL,
    BYDAY (also 2TU / -1FR in months) and BYMONTHDAY."""
    r = {}
    for part in rule.split(';'):
        k, _, v = part.partition('=')
        r[k.strip().upper()] = v.strip()
    freq = r.get('FREQ', '').upper()
    if freq not in ('DAILY', 'WEEKLY', 'MONTHLY', 'YEARLY'):
        return [start] if after <= start < before else []
    step = max(1, int(r.get('INTERVAL') or 1)) if (r.get('INTERVAL') or '1').isdigit() else 1
    count = int(r['COUNT']) if r.get('COUNT', '').isdigit() else None
    until = None
    if r.get('UNTIL'):
        u = r['UNTIL'].rstrip('Z')
        try:
            until = dt.datetime.strptime(u, '%Y%m%dT%H%M%S') if 'T' in u else dt.datetime.strptime(u, '%Y%m%d') + dt.timedelta(days=1, seconds=-1)
        except ValueError:
            until = None
    byday = [d for d in (r.get('BYDAY') or '').upper().split(',') if d]
    bymd = [int(x) for x in (r.get('BYMONTHDAY') or '').split(',') if re.fullmatch(r'-?\d+', x)]
    ex = set(exdates)
    out, n, i = [], 0, 0

    def month_days(y, m):
        first = dt.date(y, m, 1)
        nxt = dt.date(y + (m == 12), m % 12 + 1, 1)
        days = [first + dt.timedelta(days=k) for k in range((nxt - first).days)]
        if bymd:
            return [days[d - 1] if d > 0 else days[d] for d in bymd if -len(days) <= d <= len(days) and d != 0]
        if byday:
            picks = []
            for b in byday:
                m2 = re.fullmatch(r'([+-]?\d+)?(MO|TU|WE|TH|FR|SA|SU)', b)
                if not m2:
                    continue
                wd = WEEKDAYS.index(m2.group(2))
                same = [d for d in days if d.weekday() == wd]
                if m2.group(1):
                    k = int(m2.group(1))
                    if same and -len(same) <= k <= len(same) and k:
                        picks.append(same[k - 1] if k > 0 else same[k])
                else:
                    picks += same
            return sorted(set(picks))
        return [dt.date(y, m, start.day)] if start.day <= len(days) else []

    while i < limit * 4:
        if freq == 'DAILY':
            cands = [start + dt.timedelta(days=i * step)]
        elif freq == 'WEEKLY':
            wk = start - dt.timedelta(days=start.weekday()) + dt.timedelta(weeks=i * step)
            days = [WEEKDAYS.index(d[-2:]) for d in byday if d[-2:] in WEEKDAYS] or [start.weekday()]
            cands = sorted(wk + dt.timedelta(days=d) for d in set(days))
        elif freq == 'MONTHLY':
            mm = start.month - 1 + i * step
            y, m = start.year + mm // 12, mm % 12 + 1
            cands = [dt.datetime.combine(d, start.time()) for d in month_days(y, m)]
        else:
            y = start.year + i * step
            if byday or bymd:
                months = [int(x) for x in (r.get('BYMONTH') or str(start.month)).split(',') if x.isdigit()]
                cands = [dt.datetime.combine(d, start.time()) for m in months for d in month_days(y, m)]
            else:
                try:
                    cands = [start.replace(year=y)]
                except ValueError:       # 29 February
                    cands = []
        i += 1
        for c in cands:
            if c < start:
                continue
            if until is not None and c > until or count is not None and n >= count:
                return out
            n += 1
            if c in ex:
                continue
            if c >= before:
                return out
            if c >= after:
                out.append(c)
            if len(out) >= limit:
                return out
    return out
