#!/bin/sh
# ARCHIVED (docs/ARCHIVED.md): no longer run by the image checks. Was a REQUIRED image check (base-peppermint.yml, base-anatase.yml; also run by test_hearth_apps_store.py):
# Hearth's app store (Apps, hearth-apps) lists apps with the image's own Python, offline. A small Flathub catalog
# (appstream.xml.gz, where flatpak keeps it) goes in a throwaway home folder; then what Apps asks for when it opens
# (status, browse featured) and when you look around (browse a category, search, an app's page) must return apps.
# The real download from Flathub is in hearth-box-check.sh (warn-only: it needs the network).
#   sh hearth-store-check.sh [PATH-TO-hearth-apps]
set -u
HA=$(realpath "${1:-/usr/share/nimbus/desktop/shell/hearth-apps}")
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
A="$T/home/.local/share/flatpak/appstream/flathub/x86_64/active"
mkdir -p "$A" "$T/run"
python3 - "$A/appstream.xml.gz" <<'PY' || { echo "store check: couldn't write the test catalog"; exit 1; }
import gzip, sys
apps = [('org.mozilla.firefox', 'Firefox', 'Fast, private web browser', 'Network'),
        ('org.example.Hello', 'Hello · Grüße', 'Says hello', 'Utility')]
xml = ['<?xml version="1.0" encoding="UTF-8"?>', '<components version="0.8" origin="flathub">']
for i, n, s, c in apps:
    xml.append('<component type="desktop-application"><id>%s</id><name>%s</name><summary>%s</summary>'
               '<description><p>%s.</p></description><categories><category>%s</category></categories>'
               '<bundle type="flatpak">app/%s/x86_64/stable</bundle><releases><release version="1.0"/></releases>'
               '</component>' % (i, n, s, s, c, i))
xml.append('</components>')
with gzip.open(sys.argv[1], 'wt', encoding='utf-8') as f:
    f.write('\n'.join(xml))
PY
# As Apps runs it (SetJob.qml: LC_ALL=C); no network for Flathub's popular list (Hearth's own list then).
run() { (cd "$T" && env HOME="$T/home" XDG_DATA_HOME= XDG_CACHE_HOME= XDG_RUNTIME_DIR="$T/run" LC_ALL=C \
  https_proxy=http://127.0.0.1:9 HTTPS_PROXY=http://127.0.0.1:9 python3 "$HA" "$@" 2>&1); }
has() { python3 -c 'import json, sys
want = sys.argv[1]
for line in sys.stdin:
    if line.startswith("{"):
        d = json.loads(line)
        ids = [a["id"] for a in d.get("apps", [])] or ([d["id"]] if d.get("id") else [])
        sys.exit(0 if want in ids else 1)
sys.exit(1)' "$1"; }
fail=0
expect() { what=$1; id=$2; shift 2; out=$(run "$@"); if printf '%s\n' "$out" | has "$id"; then echo "ok: $what"; else echo "store check FAILED: $what: $(printf '%s' "$out" | tail -3 | tr '\n' ' ' | cut -c1-400)"; fail=1; fi; }
out=$(run status)
printf '%s\n' "$out" | grep -q '"catalog": true' && echo "ok: status finds the catalog" || { echo "store check FAILED: status: $(printf '%s' "$out" | tail -2 | cut -c1-400)"; fail=1; }
expect "the first page (popular apps) lists Firefox" org.mozilla.firefox browse featured
expect "a category (Tools) lists its app" org.example.Hello browse Utility
expect "search finds Firefox" org.mozilla.firefox search firefox
expect "an app's page opens" org.example.Hello app org.example.Hello
exit $fail
