#!/usr/bin/env python3
"""Stand-ins for Firehold's shell test (.github/workflows/firehold-shell.yml): Hearth's library (`nimbus --list-games
--all`), Windows apps and Saved Games, written into DIR. The names are the mockups' placeholders, so each picture can
be held next to its mockup.  firehold-fixtures.py DIR"""
import json, sys, time
d = sys.argv[1]
now = int(time.time() * 1000)
m = lambda mins: now - mins * 60000
games = [
  ('steam:1000', 'Ember Trails', 'steam', True, m(41), 754, {}), ('steam:1001', 'Skyforge', 'steam', True, m(60 * 5), 120, {}),
  ('gog:2001', 'Night Harbor', 'gog', True, m(60 * 20), 300, {}), ('emu:snes:pixel-rally.sfc', 'Pixel Rally', 'emu', True, m(60 * 26), 40, {'system': 'Super Nintendo'}),
  ('epic:hollow', 'Hollow Deep', 'epic', True, m(60 * 50), 60, {'update': True}), ('steam:1002', 'Tiny Farm', 'steam', True, m(60 * 70), 900, {}),
  ('manual:quad', 'Quad Mech', 'manual', True, m(60 * 100), 30, {'exe': '/tmp/home/projects/quad-mech/build/quad-mech.x86_64'}),
  ('gog:2002', 'Star Courier', 'gog', False, 0, 0, {}), ('epic:deep', 'Deep Root', 'epic', False, 0, 0, {}),
  ('steam:1003', 'Cloud Lanes', 'steam', False, 0, 0, {}), ('steam:1004', 'Frost Keep', 'steam', False, 0, 0, {})]
out = []
for gid, title, src, inst, last, mins, extra in games:
    g = {'id': gid, 'title': title, 'source': src, 'installed': inst, 'state': 'installed' if inst else 'notInstalled',
         'lastPlayed': last, 'minutes': mins, 'update': False, 'cover': None, 'wide': None, 'hero': None, 'icon': None}
    g.update(extra)
    out.append(g)
open(d + '/fixture-games.json', 'w').write(json.dumps(out))
open(d + '/nimbus', 'w').write('#!/bin/sh\ncase "$*" in *--list-games*) cat %s/fixture-games.json; echo ;; *) exit 0 ;; esac\n' % d)
apps = {'apps': [{'id': 'old-office', 'name': 'Old Office', 'icon': '', 'last': {'at': int(time.time()) - 60 * 60 * 80}}]}
open(d + '/hearth-winapps', 'w').write('#!/bin/sh\n[ "$1" = list ] && echo \'%s\'\n' % json.dumps(apps))
saves = {'games': [{'name': 'Cloud Lanes', 'backedUp': True}, {'name': 'Ember Trails', 'backedUp': True}]}
open(d + '/hearth-saves', 'w').write('#!/bin/sh\n[ "$1" = status ] && echo \'%s\'\n' % json.dumps(saves))
