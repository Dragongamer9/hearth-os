"""Stand-in answers for Settings › AI › Shared from your other devices' pictures (ai-describe.sh, ai-describe-qml.yml):
what hearth-ai-share devices and hearth-link-settings ai-status would say with three other devices: the PC ready with an
Nvidia card (its chat at DESKTOP_URL, a stand-in that lists models), the ROG Ally playing, and an office PC that Hearth
Link couldn't confirm (its certificate didn't match). Read by hearth-ai-check through HEARTH_AI_SHARED_FIXTURE and
HEARTH_AI_LINK_FIXTURE.
  python3 ai-shared-fixtures.py OUT_DIR DESKTOP_URL"""
import json
import os
import sys

out, url = sys.argv[1], sys.argv[2].rstrip('/')
os.makedirs(out, exist_ok=True)
devs = {'protocol': 1, 'devices': [
    {'device': 'laptop', 'name': 'Laptop', 'via': 'this device', 'online': True, 'available': False, 'reason': 'off', 'services': []},
    {'device': 'desktop', 'name': 'Desktop', 'via': 'tailscale', 'online': True, 'available': True, 'gaming': False, 'busy': False,
     'gpu': {'vendor': 'nvidia', 'vramMB': 12288},
     'services': [{'kind': 'chat', 'engine': 'ollama', 'url': url[:-3] if url.endswith('/v1') else url, 'openai': url, 'healthy': True,
                   'models': ['qwen3:30b-a3b', 'huihui_ai/qwen3-abliterated:14b', 'nomic-embed-text:latest']}]},
    {'device': 'rog-ally', 'name': 'ROG Ally', 'via': 'tailscale', 'online': True, 'available': False, 'gaming': True, 'reason': 'playing',
     'gpu': {'vendor': 'amd', 'vramMB': 4096},
     'services': [{'kind': 'chat', 'engine': 'ollama', 'url': 'http://100.64.0.3:11434', 'openai': 'http://100.64.0.3:11434/v1', 'healthy': True, 'models': ['llama3.2:3b']}]}]}
link = {'serve': False, 'servers': [], 'unreached': [{'id': 'ab12cd34', 'name': 'Office PC', 'mismatch': True}], 'using': None, 'clients': []}
json.dump(devs, open(os.path.join(out, 'devices.json'), 'w'))
json.dump(link, open(os.path.join(out, 'link.json'), 'w'))
