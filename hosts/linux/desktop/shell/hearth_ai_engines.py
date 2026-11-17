#!/usr/bin/python3
"""hearth_ai_engines: which AI engines this PC runs and where Hearth's AI pieces reach them (docs/SERVER-MODE.md, Colibri).

One settings file, ~/.config/nimbus/ai-engines.json, written only through hearth_settings_io (only the key you change,
atomic, .bak, merged under the lock; defaults are filled in when read and never saved):

  {"default": "ollama" | "colibri",          the engine Hearth's own AI pieces use first (Ollama unless you change it)
   "colibri": {"model": "<catalog id>",     the Colibri model you picked (empty: none yet; Colibri then doesn't start)
               "modelsDir": "<folder>"},    where Colibri's models go (empty: ~/colibri-models, Colibri's own default)
   "engines": [{"id": "ollama", "baseUrl": "http://127.0.0.1:11434/v1", ...},
               {"id": "colibri", "baseUrl": "http://127.0.0.1:11440/v1", "model": "<id>", "cli": "<coli>", ...}]}

"engines" is no choice of yours: it's the addresses that follow from the choices above, written again with each change
you make (so apps that only read the file, like Hearth AI / Ithil, find Colibri), and never on its own. Readers that
can run Python use `engines()` instead, which also says which engine is on right now.

Whether each engine is on is Settings › Home server's (home-server.json: "ai" is Ollama, "colibri" is Colibri), never
a copy here. `engines()` puts the two together with each engine's addresses: every engine speaks the OpenAI API
(Ollama at /v1 too), so a reader only needs `openai` and `model`.

  python3 hearth_ai_engines.py list       JSON: {"default", "engines": [{id, label, on, openai, base, model, ...}]}
  python3 hearth_ai_engines.py default    JSON: the default engine's entry
  python3 hearth_ai_engines.py set default ollama|colibri

Nothing changes by itself: with no file, the default is Ollama, exactly as before 1.14.
"""
import json
import os
import re
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.realpath(__file__))
sys.path.insert(0, HERE)
import hearth_settings_io as sio  # noqa: E402

CONFIG_HOME = os.environ.get('XDG_CONFIG_HOME') or os.path.expanduser('~/.config')
SETTINGS = os.path.join(CONFIG_HOME, 'nimbus', 'ai-engines.json')
HOME_SERVER = os.path.join(CONFIG_HOME, 'nimbus', 'home-server.json')
OLLAMA_PORT = 11434
COLIBRI_PORT = int(os.environ.get('HEARTH_COLIBRI_PORT') or 11440)       # Colibri's own default is 8000; Hearth runs it here so it never meets Ollama (11434),
                           # Hearth Link's shared AI (11435) or a whisper server (8000)
ENGINE_IDS = ('ollama', 'colibri')
DEFAULTS = {'default': 'ollama', 'colibri': {'model': '', 'modelsDir': ''}}
MODEL_ID_RE = re.compile(r'^[a-z0-9][a-z0-9._-]{0,63}$')


class Fail(Exception):
    pass


def load():
    """Your choices, read only, defaults filled in (in memory only; opening a page never writes)."""
    d = sio.load_json(SETTINGS, {}, repair=False) if os.path.exists(SETTINGS) else {}
    d = sio.with_defaults(d if isinstance(d, dict) else {}, DEFAULTS)
    if d.get('default') not in ENGINE_IDS:
        d['default'] = 'ollama'
    c = d.get('colibri') if isinstance(d.get('colibri'), dict) else {}
    model = c.get('model') if isinstance(c.get('model'), str) and MODEL_ID_RE.fullmatch(c.get('model') or '') else ''
    mdir = c.get('modelsDir') if isinstance(c.get('modelsDir'), str) and c.get('modelsDir', '').startswith('/') else ''
    d['colibri'] = {'model': model, 'modelsDir': mdir}
    return d


def models_dir(d=None):
    d = d or load()
    return d['colibri']['modelsDir'] or os.path.join(os.path.expanduser('~'), 'colibri-models')


def home_server_on():
    """Which engines Settings › Home server has on (its file, read only)."""
    h = sio.load_json(HOME_SERVER, {}, repair=False) if os.path.exists(HOME_SERVER) else {}
    h = h if isinstance(h, dict) else {}
    on = lambda k: isinstance(h.get(k), dict) and h[k].get('on') is True  # noqa: E731
    return {'ollama': on('ai'), 'colibri': on('colibri')}


def engines(d=None, on=None):
    d = d or load()
    on = home_server_on() if on is None else on
    lo = '127.0.0.1'
    out = [
        {'id': 'ollama', 'label': 'Ollama', 'on': bool(on.get('ollama')), 'port': OLLAMA_PORT,
         'base': 'http://%s:%d' % (lo, OLLAMA_PORT), 'openai': 'http://%s:%d/v1' % (lo, OLLAMA_PORT), 'native': 'ollama',
         'model': ''},
        {'id': 'colibri', 'label': 'Colibri', 'on': bool(on.get('colibri')), 'port': COLIBRI_PORT,
         'base': 'http://%s:%d' % (lo, COLIBRI_PORT), 'openai': 'http://%s:%d/v1' % (lo, COLIBRI_PORT),
         'anthropic': 'http://%s:%d' % (lo, COLIBRI_PORT), 'native': 'colibri', 'model': d['colibri']['model']},
    ]
    for e in out:
        e['default'] = e['id'] == d['default']
    return out


def default_engine(d=None, on=None):
    d = d or load()
    return next(e for e in engines(d, on) if e['id'] == d['default'])


COLI = os.path.join(os.environ.get('XDG_DATA_HOME') or os.path.expanduser('~/.local/share'), 'hearth', 'colibri', 'engine', 'coli')
COLI_SETUP_HOME = os.path.join(os.path.dirname(os.path.dirname(COLI)), 'setup')


def listing(d):
    """The "engines" list written into the file for apps that only read it (Hearth AI / Ithil reads this shape)."""
    return [{'id': 'ollama', 'label': 'Ollama', 'baseUrl': 'http://127.0.0.1:%d/v1' % OLLAMA_PORT, 'port': OLLAMA_PORT, 'model': ''},
            {'id': 'colibri', 'label': 'Colibri', 'baseUrl': 'http://127.0.0.1:%d/v1' % COLIBRI_PORT, 'port': COLIBRI_PORT,
             'anthropicUrl': 'http://127.0.0.1:%d' % COLIBRI_PORT, 'model': d['colibri']['model'], 'cli': COLI,
             'setupHome': COLI_SETUP_HOME, 'modelsDir': models_dir(d),
             'use': ['/usr/bin/python3', os.path.join(HERE, 'hearth-colibri'), 'use']}]


def _with_listing(f):
    """After a change you made: the file's "engines" list from the choices now in it (in the same locked write)."""
    d = sio.with_defaults(f, DEFAULTS)
    c = d.get('colibri') if isinstance(d.get('colibri'), dict) else {}
    d2 = {'default': d.get('default') if d.get('default') in ENGINE_IDS else 'ollama',
          'colibri': {'model': c.get('model') if MODEL_ID_RE.fullmatch(str(c.get('model') or '')) else '',
                      'modelsDir': c.get('modelsDir') if str(c.get('modelsDir') or '').startswith('/') else ''}}
    f['engines'] = listing(d2)


def refresh_listing():
    """The "engines" list again (after Colibri was installed or an engine turned on), only when the file exists: a file
    is never made for nothing."""
    if os.path.exists(SETTINGS):
        sio.update_json(SETTINGS, _with_listing)


def set_key(key, value):
    """Only you change these (Settings). Only that key is written (and the "engines" list that follows from it)."""
    if key == 'default':
        if value not in ENGINE_IDS:
            raise Fail('ollama or colibri')
        sio.merge_json(SETTINGS, {'default': value}, who='you')
        sio.update_json(SETTINGS, _with_listing)
        return load()
    if key == 'colibriModel':
        if value and not MODEL_ID_RE.fullmatch(value):
            raise Fail('not a Colibri model id')

        def change(f):
            c = dict(f.get('colibri')) if isinstance(f.get('colibri'), dict) else {}
            c['model'] = value
            f['colibri'] = c
            _with_listing(f)
        sio.update_json(SETTINGS, change)
        return load()
    if key == 'colibriModelsDir':
        if not value.startswith('/') or '\n' in value or '..' in value.split('/') or len(value) > 400:
            raise Fail('a folder on this PC')

        def change2(f):
            c = dict(f.get('colibri')) if isinstance(f.get('colibri'), dict) else {}
            c['modelsDir'] = os.path.normpath(value)
            f['colibri'] = c
            _with_listing(f)
        sio.update_json(SETTINGS, change2)
        return load()
    raise Fail('default, colibriModel or colibriModelsDir')


def main(argv):
    try:
        if argv[:1] == ['list']:
            d = load()
            print(json.dumps({'default': d['default'], 'engines': engines(d)}))
        elif argv[:1] == ['default']:
            print(json.dumps(default_engine()))
        elif len(argv) == 3 and argv[0] == 'set':
            print(json.dumps(set_key(argv[1], argv[2])))
        else:
            raise Fail('list | default | set KEY VALUE')
    except (Fail, OSError, ValueError) as e:
        print('error: %s' % e)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
