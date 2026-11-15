#!/usr/bin/env python3
"""Smaller updates (1.13): how big this build's update is for a device that has the published image now.

A device (bootc) downloads only the layers it doesn't have, by their digests. This compares the new image's layers
(the rechunk step's OCI folder) with the published image's (skopeo inspect --raw), and says how much is new, which
pieces changed (chunkah names each layer's component), and how much would be new if only compression differed
(same uncompressed content, other digest: a sign the push recompressed).

  update_size.py NEW_OCI_DIR OLD_MANIFEST.json [OLD_CONFIG.json] [--github] [--label TEXT]   prints JSON (and annotations)
  update_size.py --save NEW_OCI_DIR OUT_DIR    the new manifest and config, for the next build to compare with
"""
import json
import os
import sys


def blob(oci, digest):
    algo, h = digest.split(':', 1)
    with open(os.path.join(oci, 'blobs', algo, h)) as f:
        return json.load(f)


def new_manifest(oci):
    with open(os.path.join(oci, 'index.json')) as f:
        idx = json.load(f)
    m = idx['manifests'][0]
    man = blob(oci, m['digest'])
    if 'layers' not in man and man.get('manifests'):   # an index inside
        man = blob(oci, man['manifests'][0]['digest'])
    return man, blob(oci, man['config']['digest'])


def compare(new_man, new_cfg, old_man, old_cfg=None):
    old_digests = {l['digest'] for l in old_man.get('layers') or []}
    old_diffs = set(((old_cfg or {}).get('rootfs') or {}).get('diff_ids') or [])
    new_diffs = ((new_cfg or {}).get('rootfs') or {}).get('diff_ids') or []
    layers = new_man.get('layers') or []
    total = sum(l.get('size', 0) for l in layers)
    changed, recompressed = [], 0
    for i, l in enumerate(layers):
        if l['digest'] in old_digests:
            continue
        name = ((l.get('annotations') or {}).get('org.chunkah.component') or '?')
        same_content = i < len(new_diffs) and new_diffs[i] in old_diffs
        if same_content:
            recompressed += l.get('size', 0)
        changed.append({'component': name, 'mb': round(l.get('size', 0) / 1e6, 1), 'sameContent': same_content})
    new = sum(c['mb'] for c in changed)
    changed.sort(key=lambda c: -c['mb'])
    return {'layers': len(layers), 'newLayers': len(changed), 'imageMb': round(total / 1e6), 'updateMb': round(new),
            'recompressedMb': round(recompressed / 1e6), 'changed': changed[:20]}


def main(argv):
    gh = '--github' in argv
    label = 'the published image'
    if '--label' in argv:
        i = argv.index('--label')
        label = argv[i + 1]
        argv = argv[:i] + argv[i + 2:]
    a = [x for x in argv if not x.startswith('--')]
    if '--save' in argv:
        # The new image's manifest and config, for the next build to compare with.
        man, cfg = new_manifest(a[0])
        os.makedirs(a[1], exist_ok=True)
        for n, obj in (('manifest.json', man), ('config.json', cfg)):
            with open(os.path.join(a[1], n), 'w') as f:
                json.dump(obj, f)
        return 0
    new_man, new_cfg = new_manifest(a[0])
    with open(a[1]) as f:
        old_man = json.load(f)
    old_cfg = None
    if len(a) > 2 and os.path.isfile(a[2]):
        with open(a[2]) as f:
            old_cfg = json.load(f)
    r = compare(new_man, new_cfg, old_man, old_cfg)
    print(json.dumps(r))
    if gh:
        top = ', '.join('%s %s MB' % (c['component'], c['mb']) for c in r['changed'][:8])
        print('::notice title=update size::Against %s: a device downloads about %d MB of %d MB '
              '(%d of %d layers new). Biggest: %s' % (label, r['updateMb'], r['imageMb'], r['newLayers'], r['layers'], top or 'nothing'))
        if r['recompressedMb'] > 50:
            print('::warning title=update size::%d MB is the same content with a different compression (it shouldn\'t be '
                  'downloaded again)' % r['recompressedMb'])
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
