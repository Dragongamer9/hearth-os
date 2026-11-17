#!/usr/bin/env python3
"""A fake CalDAV server on localhost for Hearth's tests (never a real account): discovery (well-known, principal,
calendar home), two calendars (one read-only), event objects kept in memory with etags, calendar-query REPORT (the
objects as kept, never expanded, so Hearth's own expansion is tested too), GET, PUT with If-Match / If-None-Match
(412 when the etag is old) and DELETE.

Used by hosts/linux/test/test_account_calendars.py (in-process) and by Hearth Time's CI (time-apps.yml) as a program:
    python3 fake_dav.py PORT [--seed]
with a control address to change an event "on another device": POST /_control/touch?href=...&summary=...
"""
import base64
import datetime as dt
import hashlib
import http.server
import sys
import threading
import urllib.parse

USER, PASS = 'bob', 'ci-only-fake-dav-password'
HOME = '/dav/calendars/bob/'
CALS = {HOME + 'personal/': ('Personal', '#0082C9', False), HOME + 'holidays/': ('Holidays', '#9BD36A', True)}


def xml_escape(s):
    return s.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;')


class Store:
    def __init__(self):
        self.objects = {}            # href -> [etag, ics]
        self.n = 0
        self.log = []                # (method, path, If-Match/If-None-Match)
        self.lock = threading.Lock()

    def put(self, href, ics):
        with self.lock:
            self.n += 1
            etag = '"%s-%d"' % (hashlib.sha1(ics.encode()).hexdigest()[:8], self.n)
            self.objects[href] = [etag, ics]
            return etag


def stamp(d, h, m=0):
    return '%sT%02d%02d00' % (d.strftime('%Y%m%d'), h, m)


def seed(store):
    """A few events around today: a single one in New York time, a weekly repeating one with one time moved and one
    skipped, an all-day one, and a holiday in the read-only calendar."""
    t = dt.date.today()
    mon = t - dt.timedelta(days=t.weekday())
    p = HOME + 'personal/'
    cal = lambda body: 'BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//fake//EN\r\n%sEND:VCALENDAR\r\n' % body
    tz = ('BEGIN:VTIMEZONE\r\nTZID:America/New_York\r\nBEGIN:STANDARD\r\nDTSTART:19701101T020000\r\n'
          'RRULE:FREQ=YEARLY;BYMONTH=11;BYDAY=1SU\r\nTZOFFSETFROM:-0400\r\nTZOFFSETTO:-0500\r\nEND:STANDARD\r\n'
          'BEGIN:DAYLIGHT\r\nDTSTART:19700308T020000\r\nRRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=2SU\r\nTZOFFSETFROM:-0500\r\n'
          'TZOFFSETTO:-0400\r\nEND:DAYLIGHT\r\nEND:VTIMEZONE\r\n')
    store.put(p + 'dentist.ics', cal(tz + 'BEGIN:VEVENT\r\nUID:dentist@fake\r\nDTSTAMP:20260101T000000Z\r\n'
                                     'DTSTART;TZID=America/New_York:%s\r\nDTEND;TZID=America/New_York:%s\r\nSUMMARY:Dentist\r\n'
                                     'LOCATION:Smile Clinic\r\nATTENDEE;CN=Dr Lee:mailto:lee@example.org\r\n'
                                     'X-OTHER-APP:kept as it was\r\nEND:VEVENT\r\n' % (stamp(t + dt.timedelta(days=1), 10), stamp(t + dt.timedelta(days=1), 11))))
    first = mon - dt.timedelta(weeks=2)
    moved, skipped = mon + dt.timedelta(weeks=1), mon
    store.put(p + 'team.ics', cal('BEGIN:VEVENT\r\nUID:team@fake\r\nDTSTAMP:20260101T000000Z\r\nDTSTART:%sZ\r\nDTEND:%sZ\r\n'
                                  'RRULE:FREQ=WEEKLY;BYDAY=MO\r\nEXDATE:%sZ\r\nSUMMARY:Team meeting\r\nSEQUENCE:2\r\nEND:VEVENT\r\n'
                                  'BEGIN:VEVENT\r\nUID:team@fake\r\nDTSTAMP:20260101T000000Z\r\nRECURRENCE-ID:%sZ\r\n'
                                  'DTSTART:%sZ\r\nDTEND:%sZ\r\nSUMMARY:Team meeting (moved)\r\nEND:VEVENT\r\n'
                                  % (stamp(first, 15), stamp(first, 16), stamp(skipped, 15), stamp(moved, 15),
                                     stamp(moved + dt.timedelta(days=1), 17), stamp(moved + dt.timedelta(days=1), 18))))
    store.put(p + 'trip.ics', cal('BEGIN:VEVENT\r\nUID:trip@fake\r\nDTSTAMP:20260101T000000Z\r\nDTSTART;VALUE=DATE:%s\r\n'
                                  'DTEND;VALUE=DATE:%s\r\nSUMMARY:Lisbon trip\r\nEND:VEVENT\r\n'
                                  % ((t + dt.timedelta(days=4)).strftime('%Y%m%d'), (t + dt.timedelta(days=7)).strftime('%Y%m%d'))))
    store.put(HOME + 'holidays/h1.ics', cal('BEGIN:VEVENT\r\nUID:h1@fake\r\nDTSTAMP:20260101T000000Z\r\nDTSTART;VALUE=DATE:%s\r\n'
                                            'SUMMARY:Bank holiday\r\nEND:VEVENT\r\n' % (t + dt.timedelta(days=9)).strftime('%Y%m%d')))


def handler(store):
    class H(http.server.BaseHTTPRequestHandler):
        protocol_version = 'HTTP/1.1'

        def log_message(self, *a):
            pass

        def body(self):
            n = int(self.headers.get('Content-Length') or 0)
            return self.rfile.read(n).decode() if n else ''

        def send(self, code, text='', ctype='application/xml; charset=utf-8', headers=None):
            data = text.encode()
            self.send_response(code)
            self.send_header('Content-Type', ctype)
            self.send_header('Content-Length', str(len(data)))
            for k, v in (headers or {}).items():
                self.send_header(k, v)
            self.end_headers()
            self.wfile.write(data)

        def authed(self):
            a = self.headers.get('Authorization') or ''
            if a.startswith('Basic '):
                u, _, p = base64.b64decode(a[6:]).decode().partition(':')
                return (u, p) == (USER, PASS)
            return False

        def ms(self, rs):
            return ('<?xml version="1.0"?><d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" '
                    'xmlns:card="urn:ietf:params:xml:ns:carddav" xmlns:ical="http://apple.com/ns/ical/">%s</d:multistatus>') % ''.join(
                '<d:response><d:href>%s</d:href><d:propstat><d:prop>%s</d:prop><d:status>HTTP/1.1 200 OK</d:status>'
                '</d:propstat></d:response>' % r for r in rs)

        def where(self):
            return urllib.parse.unquote(urllib.parse.urlsplit(self.path).path)

        def do_PROPFIND(self):   # noqa: N802
            self.body()
            p = self.where()
            if p in ('/.well-known/caldav', '/.well-known/carddav'):
                return self.send(301, '', headers={'Location': '/dav/'})
            if not self.authed():
                return self.send(401, '', headers={'WWW-Authenticate': 'Basic realm="fake"'})
            if p in ('/', '/dav/'):
                return self.send(207, self.ms([(p, '<d:current-user-principal><d:href>/dav/principals/bob/</d:href></d:current-user-principal>')]))
            if p == '/dav/principals/bob/':
                return self.send(207, self.ms([(p, '<c:calendar-home-set><d:href>%s</d:href></c:calendar-home-set>'
                                                   '<card:addressbook-home-set><d:href>/dav/addressbooks/bob/</d:href></card:addressbook-home-set>' % HOME)]))
            if p == HOME:
                rs = [(p, '<d:resourcetype><d:collection/></d:resourcetype>')]
                for href, (name, color, ro) in CALS.items():
                    priv = '<d:privilege><d:read/></d:privilege>' + ('' if ro else '<d:privilege><d:write/></d:privilege>')
                    rs.append((href, '<d:resourcetype><d:collection/><c:calendar/></d:resourcetype><d:displayname>%s</d:displayname>'
                                     '<ical:calendar-color>%sFF</ical:calendar-color><c:supported-calendar-component-set><c:comp name="VEVENT"/>'
                                     '</c:supported-calendar-component-set><d:current-user-privilege-set>%s</d:current-user-privilege-set>'
                               % (name, color, priv)))
                return self.send(207, self.ms(rs))
            if p == '/dav/addressbooks/bob/':
                return self.send(207, self.ms([(p, '<d:resourcetype><d:collection/></d:resourcetype>'),
                                               (p + 'contacts/', '<d:resourcetype><d:collection/><card:addressbook/></d:resourcetype><d:displayname>Contacts</d:displayname>')]))
            if p in store.objects:
                return self.send(207, self.ms([(p, '<d:getetag>%s</d:getetag>' % store.objects[p][0])]))
            self.send(404, '')

        def do_REPORT(self):     # noqa: N802
            self.body()
            if not self.authed():
                return self.send(401, '')
            p = self.where()
            with store.lock:
                items = [(h, e, i) for h, (e, i) in sorted(store.objects.items()) if h.startswith(p)]
            self.send(207, self.ms([(h, '<d:getetag>%s</d:getetag><c:calendar-data>%s</c:calendar-data>' % (xml_escape(e), xml_escape(i)))
                                    for h, e, i in items]))

        def do_GET(self):        # noqa: N802
            p = self.where()
            if not self.authed():
                return self.send(401, '')
            o = store.objects.get(p)
            if not o:
                return self.send(404, '')
            self.send(200, o[1], 'text/calendar; charset=utf-8', {'ETag': o[0]})

        def do_PUT(self):        # noqa: N802
            b = self.body()
            p = self.where()
            if not self.authed():
                return self.send(401, '')
            im, inm = self.headers.get('If-Match'), self.headers.get('If-None-Match')
            store.log.append(('PUT', p, im or inm or ''))
            cal = next((c for c in CALS if p.startswith(c)), None)
            if cal is None:
                return self.send(404, '')
            if CALS[cal][2]:
                return self.send(403, '')
            cur = store.objects.get(p)
            if inm == '*' and cur or im and (not cur or cur[0] != im):
                return self.send(412, '')
            etag = store.put(p, b)
            self.send(201 if not cur else 204, '', headers={'ETag': etag})

        def do_DELETE(self):     # noqa: N802
            self.body()
            p = self.where()
            if not self.authed():
                return self.send(401, '')
            im = self.headers.get('If-Match')
            store.log.append(('DELETE', p, im or ''))
            cur = store.objects.get(p)
            if not cur:
                return self.send(404, '')
            if im and cur[0] != im:
                return self.send(412, '')
            del store.objects[p]
            self.send(204, '')

        def do_POST(self):       # noqa: N802
            self.body()
            u = urllib.parse.urlsplit(self.path)
            q = {k: v[0] for k, v in urllib.parse.parse_qs(u.query).items()}
            if u.path == '/_control/touch' and q.get('href') in store.objects:
                ics = store.objects[q['href']][1]
                import re
                ics = re.sub(r'SUMMARY:[^\r\n]*', 'SUMMARY:' + q.get('summary', 'Changed elsewhere'), ics, count=1)
                return self.send(200, store.put(q['href'], ics), 'text/plain')
            self.send(404, '')
    return H


def serve(port=0, with_seed=True):
    store = Store()
    if with_seed:
        seed(store)
    srv = http.server.ThreadingHTTPServer(('127.0.0.1', port), handler(store))
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv, store


if __name__ == '__main__':
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 5233
    srv, _ = serve(port, '--seed' in sys.argv or True)
    print('fake CalDAV on http://127.0.0.1:%d/ (user %s)' % (srv.server_port, USER), flush=True)
    threading.Event().wait()
