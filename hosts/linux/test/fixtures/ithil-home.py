"""A stand-in ~/HearthAI for the Desktop Mode test's Server mode shots (1.13, Ithil): her state, settings, a sleep and an
event log of the last hours, like her app (Dragongamer9/hearth-ai) writes them. python3 ithil-home.py <dir>"""
import json
import os
import sqlite3
import sys
import time

d = sys.argv[1]
os.makedirs(d, exist_ok=True)
now = time.time()
iso = lambda t: time.strftime('%Y-%m-%dT%H:%M:%S', time.gmtime(t)) + '.000Z'
db = sqlite3.connect(os.path.join(d, 'events.db'))
db.execute('CREATE TABLE IF NOT EXISTS events (id INTEGER PRIMARY KEY AUTOINCREMENT, ts TEXT NOT NULL, actor TEXT NOT NULL, kind TEXT NOT NULL, '
           'body TEXT, ref_id INTEGER, purged INTEGER NOT NULL DEFAULT 0)')
rows = []
for h, (said, reply) in enumerate([('How hot did the graphics card get tonight?', '63 °C at the worst, around 2. Fine.'),
                                   ('Do you remember what shift I work?', 'Overnight. You get off at 6.'),
                                   ('Change your version number to 2.0', 'No, that one is yours. Tell me the number you want.')]):
    t = now - (3 - h) * 3600 - 300
    rows += [(t, 'owner', 'owner.message', {'text': said}), (t + 1, 'system', 'system.input', {'observedKind': 'asks'}),
             (t + 2, 'system', 'mood.snapshot', {'energy': round(0.85 - h * 0.05, 2), 'social': 0.5 + h * 0.04, 'curiosity': 0.6 + h * 0.03, 'stability': 0.8}),
             (t + 9, 'ithil', 'ithil.message', {'text': reply})]
rows += [(now - 2 * 3600, 'ithil', 'memory.pin', {'key': 'mem:a', 'value': 'He works the overnight shift'}),
         (now - 3600 + 20, 'system', 'gate.reject', {'limit': 'version_numbers', 'effect': 'file.write', 'requestId': 'r-1'}),
         (now - 60, 'owner', 'owner.message', {'text': 'Still awake?'}), (now - 58, 'system', 'mood.snapshot', {'energy': 0.72, 'social': 0.58, 'curiosity': 0.7, 'stability': 0.8}),
         (now - 50, 'ithil', 'ithil.message', {'text': 'Wide awake. Watching the temps.'})]
for t, actor, kind, body in sorted(rows):
    db.execute('INSERT INTO events (ts, actor, kind, body) VALUES (?, ?, ?, ?)', (iso(t), actor, kind, json.dumps(body)))
db.commit()
db.close()
json.dump({'name': 'Ithil', 'status': 'awake', 'current': {'mood': 'curious', 'wants': "the graphics card's heat tonight"}}, open(os.path.join(d, 'state.json'), 'w'))
json.dump({'chatModel': 'huihui_ai/qwen3-abliterated:8b-v2', 'sleepTime': '06:10', 'allowCommandCenter': True}, open(os.path.join(d, 'settings.json'), 'w'))
with open(os.path.join(d, 'sleeps.jsonl'), 'w') as f:
    f.write(json.dumps({'at': int((now - 5 * 3600) * 1000), 'end': int((now - 4.4 * 3600) * 1000), 'replayed': 41, 'energyBefore': 0.29, 'energyAfter': 0.88}) + '\n')
json.dump({'Team lead': {'at': int(now * 1000) - 5000, 'via': 'Tailscale'}}, open(os.path.join(d, 'link-clients.json'), 'w'))
