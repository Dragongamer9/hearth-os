"""Makes the data for CI's Colibri screenshots (colibri.yml) with hearth-colibri's own code: its catalog of every model,
both groups, worked out for the owner's Nvidia PC (32 GB of memory, an RTX 5070 with 12 GB, about 1.8 TB free on the
models drive), with Colibri's launcher a stand-in (fixtures/fake-coli.py). Also the dashboard's Colibri tile, made by
hearth-server-mode's own colibri_tile(). Usage: python3 colibri_shots_fixture.py OUT_COLIBRI.json DASH_IN.json DASH_OUT.json"""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import shutil
import sys
import tempfile

sys.dont_write_bytecode = True
HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent.parent
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def main(out, dash_in, dash_out):
    home = tempfile.mkdtemp(prefix='colibri-shots-')
    os.environ.update({'HOME': home, 'XDG_CONFIG_HOME': home + '/.config', 'XDG_DATA_HOME': home + '/.local/share',
                       'XDG_STATE_HOME': home + '/.local/state', 'FAKE_COLI_CATALOG': str(HERE / 'fixtures' / 'colibri-catalog.json')})
    os.environ.pop('HEARTH_COLIBRI_FIXTURE', None)
    hc = load('hearth_colibri_shots', SHELL / 'hearth-colibri')
    os.makedirs(hc.ENGINE)
    shutil.copy(HERE / 'fixtures' / 'fake-coli.py', os.path.join(hc.ENGINE, 'coli'))
    open(os.path.join(hc.ENGINE, 'colibri'), 'w').close()
    hc.meminfo_gb = lambda: 33.4
    hc.gpu = lambda: {'name': 'RTX 5070', 'vramGb': 12.0}
    hc.free_bytes = lambda p: 1_800 * 10 ** 9
    cat = hc.catalog()
    for m in cat['models']:
        if m['id'] == 'qwen36-35b':
            m.update({'have': True, 'chosen': True})
        if m['id'] == 'qwen3-coder-30b':
            m['have'] = True
    status = {'installed': True, 'version': 'v2.1.0', 'pinned': 'v2.1.0', 'installing': False, 'progress': 100, 'error': '',
              'on': True, 'pause': True, 'active': True, 'model': 'qwen36-35b', 'modelDownloaded': True,
              'downloaded': ['qwen3-coder-30b', 'qwen36-35b'],
              'download': {'id': 'qwen36-35b-abl-q4-k-m', 'name': 'Qwen3.6 35B A3B, uncensored · Q4_K_M', 'running': True,
                           'done': 8_100_000_000, 'total': 21_166_758_784, 'rate': 48_000_000, 'pct': 38.3},
              'server': {'health': 'ready', 'modelId': 'qwen36-35b', 'tokensPerSecond': 24.6, 'answers': 12},
              'port': 11440, 'openai': 'http://127.0.0.1:11440/v1', 'listen': ['127.0.0.1', '100.101.102.103'], 'ollamaOn': True,
              'engine': 'both', 'default': 'ollama', 'modelsDir': '/run/media/owner/NVMe/colibri-models'}
    for m in cat['models']:
        if m['id'] == status['download']['id']:
            m['downloading'] = True
    cat['hardware']['modelsDir'] = status['modelsDir']
    plan = {'id': 'qwen36-35b', 'vramGb': 10.7, 'ramGb': 17.0, 'diskGb': 23.1, 'bottleneck': 'memory bandwidth', 'hitRate': 0.97,
            'warnings': [], 'doctor': 'ok', 'problems': []}
    json.dump({'status': status, 'catalog': cat, 'plan': plan}, open(out, 'w'))
    sm = load('hearth_server_mode_shots', SHELL / 'hearth-server-mode')
    d = json.load(open(dash_in))
    tile = sm.colibri_tile({'active': True, 'health': 'ready', 'model': 'qwen36-35b', 'tps': 24.6, 'answers': 12}, 'running', {'colibri': 'stop'})
    i = next((n for n, t in enumerate(d['tiles']) if t['id'] == 'ai'), 0)
    d['tiles'].insert(i + 1, tile)
    d.setdefault('pills', []).append({'name': 'Colibri', 'extra': '', 'state': 'running'})
    json.dump(d, open(dash_out, 'w'))
    print('::notice title=Colibri shots::%d models (%d standard, %d uncensored); the tile is number %d' % (
        len(cat['models']), sum(m['group'] == 'standard' for m in cat['models']), sum(m['group'] == 'uncensored' for m in cat['models']), i + 2))
    shutil.rmtree(home, ignore_errors=True)
    return i + 2


if __name__ == '__main__':
    n = main(*sys.argv[1:4])
    if len(sys.argv) > 4:
        open(sys.argv[4], 'w').write(str(n))
