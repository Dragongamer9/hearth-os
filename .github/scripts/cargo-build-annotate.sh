#!/bin/bash
# cargo build --release in CRATE_DIR with the compiler's errors as annotations (logs can't be downloaded where the
# owner's sessions run). Up to 30 errors, 10 a step: errors 1-10 here, the rest in errors-<name>.txt for later steps.
#   cargo-build-annotate.sh CRATE_DIR [cargo build args...]
set -u
dir=${1:?crate dir}; shift
name=$(basename "$dir")
root=$PWD
cd "$dir" || exit 2
cargo build --release "$@" --message-format=json-diagnostic-rendered-ansi > "$root/build-$name.json" 2> "$root/build-$name.err"
rc=$?
python3 - "$root/build-$name.json" "$root/errors-$name.txt" "$dir" <<'PY'
import json, re, sys
src, out, d = sys.argv[1:4]
errs, warns = [], 0
for line in open(src, errors='replace'):
    try:
        m = json.loads(line)
    except ValueError:
        continue
    if m.get('reason') != 'compiler-message':
        continue
    msg = m['message']
    if msg.get('level') == 'warning':
        warns += 1
    if not str(msg.get('level', '')).startswith('error'):
        continue
    text = re.sub(r'\x1b\[[0-9;]*m', '', msg.get('rendered') or msg.get('message') or '')
    span = next((s for s in msg.get('spans', []) if s.get('is_primary')), None)
    errs.append((span, text))
with open(out, 'w') as f:
    for span, text in errs:
        props = 'title=build %s' % d
        if span and not span['file_name'].startswith('/'):
            props = 'file=%s/%s,line=%d,col=%d,title=build' % (d, span['file_name'], span['line_start'], span['column_start'])
        body = text[:1800].replace('%', '%25').replace('\r', '').replace('\n', '%0A')
        f.write('::error %s::%s\n' % (props, body))
print('::notice title=build %s::%d errors, %d warnings' % (d, len(errs), warns))
PY
sed -n '1,10p' "$root/errors-$name.txt"
if [ "$rc" != 0 ] && [ ! -s "$root/errors-$name.txt" ]; then
  echo "::error title=build $dir::cargo failed: $(tail -c 1500 "$root/build-$name.err" | tr '\n' '|')"
fi
exit $rc
