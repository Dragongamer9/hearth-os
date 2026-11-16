#!/usr/bin/env python3
"""Hearth Photos' helper for what's in a photo and whose face is in it (1.14). Everything runs on this PC.

    hearth_photos_ai.py status              what's ready (JSON)
    hearth_photos_ai.py setup               one-time: OpenCV in its own folder, and the small face / picture models
    hearth_photos_ai.py run LIST.json       looks at the pictures in LIST.json ([{path, mtime, thumb}]) not seen yet
    hearth_photos_ai.py forget-faces        forgets every face and group (Faces turned off)

Faces: OpenCV's YuNet finds them and SFace describes each one; faces whose descriptions are close are one group.
What's in it: a vision model in Ollama (your graphics card) when one is installed there, else a small picture
classifier (MobileNet, on the processor). Results go to ~/.local/share/hearth/photos-ai/results.json, written whole
and atomically after every few pictures; progress to progress.json. The pictures never leave the PC: Ollama is asked
on 127.0.0.1 only.
"""
import base64, json, math, os, subprocess, sys, tempfile, time, urllib.request

HOME = os.path.expanduser('~')
BASE = os.environ.get('HEARTH_PHOTOS_AI') or os.path.join(os.environ.get('XDG_DATA_HOME') or os.path.join(HOME, '.local/share'), 'hearth', 'photos-ai')
VENV = os.path.join(BASE, 'venv')
MODELS = os.path.join(BASE, 'models')
RESULTS = os.path.join(BASE, 'results.json')
PROGRESS = os.path.join(BASE, 'progress.json')
OLLAMA = os.environ.get('HEARTH_OLLAMA', 'http://127.0.0.1:11434')
ZOO = 'https://github.com/opencv/opencv_zoo/raw/main/models'
FILES = {
    'face_detection_yunet_2023mar.onnx': f'{ZOO}/face_detection_yunet/face_detection_yunet_2023mar.onnx',
    'face_recognition_sface_2021dec.onnx': f'{ZOO}/face_recognition_sface/face_recognition_sface_2021dec.onnx',
    'image_classification_mobilenetv2_2022apr.onnx': f'{ZOO}/image_classification_mobilenet/image_classification_mobilenetv2_2022apr.onnx',
    'imagenet_classes.txt': 'https://raw.githubusercontent.com/pytorch/hub/master/imagenet_classes.txt',
}
VISION = ('llava', 'moondream', 'qwen2.5vl', 'qwen2-vl', 'llama3.2-vision', 'gemma3', 'minicpm-v', 'bakllava', 'granite3.2-vision')
SAME_PERSON = 0.42   # SFace cosine: above this, the same person (OpenCV suggests 0.363; a little stricter here)


def write_json(path, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=os.path.dirname(path), prefix='.tmp-')
    with os.fdopen(fd, 'w') as f:
        json.dump(data, f)
    os.replace(tmp, path)


def read_json(path, default):
    try:
        with open(path) as f:
            return json.load(f)
    except (OSError, ValueError):
        return default


def ollama_vision():
    """The installed Ollama vision model, if there is one (only asked on this PC)."""
    try:
        with urllib.request.urlopen(f'{OLLAMA}/api/tags', timeout=2) as r:
            names = [m.get('name', '') for m in json.load(r).get('models', [])]
    except Exception:
        return None
    for want in VISION:
        for n in names:
            if n.startswith(want):
                return n
    return None


def py():
    p = os.path.join(VENV, 'bin', 'python3')
    return p if os.path.exists(p) else None


def status():
    have = all(os.path.exists(os.path.join(MODELS, f)) for f in FILES)
    ready = bool(py()) and have
    r = read_json(RESULTS, {})
    return {'ready': ready, 'models': have, 'venv': bool(py()), 'vision': ollama_vision(), 'seen': len(r.get('photos', {})),
            'groups': len(r.get('groups', [])), 'progress': read_json(PROGRESS, None)}


def setup():
    os.makedirs(MODELS, exist_ok=True)
    progress('setup', 0, 4, 'Getting OpenCV (once)…')
    if not py():
        r = subprocess.run([sys.executable, '-m', 'venv', VENV], capture_output=True, text=True)
        if r.returncode != 0:
            raise RuntimeError('Python\'s venv is missing here (Debian: python3-venv). ' + (r.stderr or '').strip()[-200:])
    r = subprocess.run([py(), '-m', 'pip', 'install', '--quiet', '--disable-pip-version-check', 'opencv-python-headless>=4.9', 'numpy'], capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError('Getting OpenCV didn\'t work: ' + (r.stderr or r.stdout or '').strip()[-300:])
    for k, (name, url) in enumerate(FILES.items(), 1):
        dest = os.path.join(MODELS, name)
        if os.path.exists(dest):
            continue
        progress('setup', k, 4, f'Getting the picture models (once)… {k} of 4')
        req = urllib.request.Request(url, headers={'User-Agent': 'HearthPhotos/1.14'})
        with urllib.request.urlopen(req, timeout=120) as r, open(dest + '.part', 'wb') as f:
            while True:
                b = r.read(1 << 16)
                if not b:
                    break
                f.write(b)
        os.replace(dest + '.part', dest)
    progress(None)
    print(json.dumps(status()))


def progress(kind, done=0, total=0, text=''):
    if kind is None:
        try:
            os.remove(PROGRESS)
        except OSError:
            pass
        return
    write_json(PROGRESS, {'kind': kind, 'done': done, 'total': total, 'text': text, 'at': time.time()})


# ---------------------------------------------------------------- the work (inside the venv)
def run(list_file):
    if os.environ.get('HEARTH_PHOTOS_AI_INNER') != '1':
        # Again with OpenCV's Python (the venv).
        if not py():
            sys.exit('Not set up yet')
        os.execve(py(), [py(), __file__, 'run', list_file], {**os.environ, 'HEARTH_PHOTOS_AI_INNER': '1'})
    import cv2
    import numpy as np
    det = cv2.FaceDetectorYN.create(os.path.join(MODELS, 'face_detection_yunet_2023mar.onnx'), '', (320, 320), 0.85, 0.3, 50)
    rec = cv2.FaceRecognizerSF.create(os.path.join(MODELS, 'face_recognition_sface_2021dec.onnx'), '')
    net = cv2.dnn.readNetFromONNX(os.path.join(MODELS, 'image_classification_mobilenetv2_2022apr.onnx'))
    labels = [l.strip() for l in open(os.path.join(MODELS, 'imagenet_classes.txt'))]
    vision = ollama_vision()
    with open(list_file) as f:
        todo = json.load(f)
    res = read_json(RESULTS, {})
    photos, groups = res.setdefault('photos', {}), res.setdefault('groups', [])
    faces_on = not res.get('facesOff')
    todo = [t for t in todo if photos.get(t['path'], {}).get('mtime') != t.get('mtime')]
    for k, t in enumerate(todo):
        progress('run', k, len(todo), f'Looking at your photos… {k} of {len(todo)}')
        img = cv2.imread(t.get('thumb') or t['path'])
        if img is None:
            photos[t['path']] = {'mtime': t.get('mtime'), 'tags': [], 'faces': []}
            continue
        h, w = img.shape[:2]
        entry = {'mtime': t.get('mtime'), 'faces': [], 'tags': []}
        if faces_on:
            det.setInputSize((w, h))
            _, found = det.detect(img)
            for fc in (found if found is not None else []):
                if fc[14] < 0.85 or fc[2] < 24:
                    continue
                aligned = rec.alignCrop(img, fc)
                v = rec.feature(aligned).flatten()
                v = v / (np.linalg.norm(v) or 1)
                gid = place_face(groups, v, t['path'])
                entry['faces'].append({'g': gid, 'box': [round(float(fc[0]) / w, 4), round(float(fc[1]) / h, 4), round(float(fc[2]) / w, 4), round(float(fc[3]) / h, 4)]})
        entry['tags'] = (ask_ollama(vision, img, cv2) if vision else None) or classify(net, labels, img, cv2, np)
        entry['by'] = 'gpu' if vision else 'cpu'
        photos[t['path']] = entry
        if k % 8 == 7:
            write_json(RESULTS, res)
    # Faces of photos that are gone: their groups' counts come from what's left (the page counts them).
    write_json(RESULTS, res)
    progress(None)
    print(json.dumps({'done': len(todo)}))


def place_face(groups, v, path):
    best, score = None, SAME_PERSON
    for g in groups:
        c = g['c']
        s = sum(a * b for a, b in zip(c, v))
        if s > score:
            best, score = g, s
    if best is None:
        best = {'id': f'g{len(groups) + 1}-{int(time.time())}', 'c': [round(float(x), 5) for x in v], 'n': 0, 'cover': path}
        groups.append(best)
    n = best['n']
    mixed = [(a * n + b) / (n + 1) for a, b in zip(best['c'], v)]
    norm = math.sqrt(sum(x * x for x in mixed)) or 1
    best['c'] = [round(x / norm, 5) for x in mixed]
    best['n'] = n + 1
    return best['id']


# ImageNet's names, said simply ("lakeside, lakeshore" → "Lake"); the ones not worth showing are left out.
FRIENDLY = {'lakeside': 'Lake', 'seashore': 'Beach', 'sandbar': 'Beach', 'dock': 'Dock', 'boathouse': 'Dock', 'pier': 'Pier', 'valley': 'Valley',
            'alp': 'Mountains', 'volcano': 'Mountains', 'cliff': 'Cliff', 'geyser': 'Geyser', 'promontory': 'Coast', 'coral reef': 'Reef',
            'sports car': 'Car', 'convertible': 'Car', 'pickup': 'Truck', 'jeep': 'Car', 'cab': 'Car', 'minivan': 'Car', 'golden retriever': 'Dog',
            'Labrador retriever': 'Dog', 'tabby': 'Cat', 'tiger cat': 'Cat', 'Egyptian cat': 'Cat', 'Persian cat': 'Cat', 'pizza': 'Pizza',
            'cheeseburger': 'Food', 'plate': 'Food', 'restaurant': 'Restaurant', 'church': 'Church', 'monitor': 'Screen', 'screen': 'Screen',
            'web site': 'Screenshot', 'menu': 'Screenshot', 'comic book': 'Drawing', 'Christmas stocking': 'Christmas', 'pot': 'Plants',
            'daisy': 'Flowers', 'snowmobile': 'Snow', 'ski': 'Snow', 'shovel': 'Snow', 'patio': 'Backyard', 'picket fence': 'Backyard',
            'lawn mower': 'Backyard', 'birdhouse': 'Backyard', 'fountain': 'Fountain', 'bonfire': 'Fire', 'fireboat': 'Boat', 'canoe': 'Boat',
            'speedboat': 'Boat', 'paddle': 'Boat', 'joystick': 'Gaming', 'mouse': 'Computer', 'desktop computer': 'Computer', 'laptop': 'Computer'}


def classify(net, labels, img, cv2, np):
    blob = cv2.dnn.blobFromImage(cv2.resize(img, (224, 224)), 1 / 58.0, (224, 224), (123.7, 116.3, 103.5), swapRB=True)
    net.setInput(blob)
    out = net.forward().flatten().astype('float64')
    # Some exports end with softmax already (all 0..1, adding up to 1); others give raw scores.
    if out.min() >= 0 and abs(out.sum() - 1) < 0.05:
        p = out
    else:
        p = np.exp(out - out.max()); p /= p.sum()
    tags = []
    for i in p.argsort()[::-1][:6]:
        if p[i] < 0.08:
            break
        name = labels[i] if i < len(labels) else ''
        word = FRIENDLY.get(name) or name.split(',')[0].strip().capitalize()
        if word and word not in tags:
            tags.append(word)
    # Light: evening / night from how warm and dark the picture is.
    small = cv2.resize(img, (32, 32)).astype('float32')
    b, g, r = small[..., 0].mean(), small[..., 1].mean(), small[..., 2].mean()
    if (r + g + b) / 3 < 45:
        tags.append('Night')
    elif r > b * 1.35 and r > 120:
        tags.append('Evening')
    return tags[:6]


def ask_ollama(model, img, cv2):
    h, w = img.shape[:2]
    k = min(1.0, 768 / max(h, w))
    ok, jpg = cv2.imencode('.jpg', cv2.resize(img, (int(w * k), int(h * k))), [cv2.IMWRITE_JPEG_QUALITY, 85])
    if not ok:
        return None
    body = json.dumps({'model': model, 'stream': False, 'images': [base64.b64encode(jpg.tobytes()).decode()],
                       'prompt': 'List 3 to 6 short tags (one or two words each) for what is in this photo: things, place, '
                                 'time of day. Answer with the tags only, separated by commas.'}).encode()
    try:
        req = urllib.request.Request(f'{OLLAMA}/api/generate', data=body, headers={'Content-Type': 'application/json'})
        with urllib.request.urlopen(req, timeout=120) as r:
            text = json.load(r).get('response', '')
    except Exception:
        return None
    tags = []
    for t in text.replace('\n', ',').split(','):
        t = t.strip(' .*-"\'').strip()
        if 0 < len(t) <= 24 and len(t.split()) <= 3 and t.lower() not in (x.lower() for x in tags):
            tags.append(t[:1].upper() + t[1:])
    return tags[:6]


def forget_faces():
    res = read_json(RESULTS, {})
    res['groups'] = []
    for p in res.get('photos', {}).values():
        p['faces'] = []
    res['facesOff'] = True
    write_json(RESULTS, res)
    print(json.dumps({'ok': True}))


def faces_on():
    res = read_json(RESULTS, {})
    res.pop('facesOff', None)
    # Look at every photo again for faces.
    for p in res.get('photos', {}).values():
        p['mtime'] = None
    write_json(RESULTS, res)
    print(json.dumps({'ok': True}))


if __name__ == '__main__':
    cmd = sys.argv[1] if len(sys.argv) > 1 else 'status'
    if cmd == 'status':
        print(json.dumps(status()))
    elif cmd == 'setup':
        try:
            setup()
        except Exception as e:  # pip or a download failed: say so (the page shows it)
            progress('setup', 0, 0, f'Setting up didn\'t work: {e}')
            print(f'setup failed: {e}', file=sys.stderr)
            sys.exit(1)
    elif cmd == 'run':
        run(sys.argv[2])
    elif cmd == 'forget-faces':
        forget_faces()
    elif cmd == 'faces-on':
        faces_on()
    else:
        sys.exit(__doc__)
