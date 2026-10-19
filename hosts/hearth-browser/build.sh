#!/bin/bash
# Builds Hearth Browser on CEF (Ubuntu runner, from the repository root) into ./hearth-browser-cef.tar.
# Used by .github/workflows/os.yml and browser.yml. On failure the compiler's errors become annotations on the run.
set -eo pipefail
exec > >(tee /tmp/step.log) 2>&1
trap 'sleep 1; grep -E "error:|Error |undefined reference|CMake Error" /tmp/step.log | head -10 | cut -c1-500 | while IFS= read -r l; do echo "::error title=browser::${l//%/%25}"; done' ERR

sudo apt-get update -qq
sudo apt-get install -y -qq cmake ninja-build libgtk-3-dev libnss3-dev libx11-xcb-dev libxcomposite-dev libxdamage-dev libxrandr-dev libgbm-dev libasound2-dev libcups2-dev libxkbcommon-dev >/dev/null

f=$(curl -fsSL https://cef-builds.spotifycdn.com/index.json | python3 -c '
import json,sys
d=json.load(sys.stdin)["linux64"]["versions"]
v=next(x for x in d if x.get("channel")=="stable")
print(next(f["name"] for f in v["files"] if f["type"]=="minimal"))')
echo "CEF: $f"
echo "::notice title=CEF::$f"
curl -fsSL "https://cef-builds.spotifycdn.com/$f" -o /tmp/cef.tar.bz2
rm -rf /tmp/cef && mkdir -p /tmp/cef && tar -C /tmp/cef --strip-components=1 -xjf /tmp/cef.tar.bz2

v=$(grep -oP '(?<=<Version>)[^<]+' hosts/windows/Nimbus.Host/Nimbus.Host.csproj | head -1)
cp -a hosts/hearth-browser /tmp/cef/hearth
echo 'add_subdirectory(hearth)' >> /tmp/cef/CMakeLists.txt
cmake -S /tmp/cef -B /tmp/cef/build -G Ninja -DCMAKE_BUILD_TYPE=Release -DHEARTH_VERSION="$v"
ninja -k 0 -C /tmp/cef/build hearth-browser
out=/tmp/cef/build/hearth/Release
test -x $out/hearth-browser
rm -rf /tmp/hb && mkdir -p /tmp/hb && cp -a $out/. /tmp/hb/
# Hearth's own pages (the top bar, start page, history...), shared with the Electron version.
mkdir -p /tmp/hb/hearth
cp hosts/linux/src/web/chrome.html hosts/linux/src/web/pages.html hosts/linux/src/web/hearth-logo.svg /tmp/hb/hearth/
# Ad and tracker servers (Steven Black's unified hosts list).
curl -fsSL https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts | grep -E '^0\.0\.0\.0 ' > /tmp/hb/hearth/adblock-hosts.txt || true
wc -l /tmp/hb/hearth/adblock-hosts.txt
strip /tmp/hb/hearth-browser || true
tar -C /tmp/hb -cf hearth-browser-cef.tar .
