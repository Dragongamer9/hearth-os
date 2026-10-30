#!/bin/bash
# Builds hearth-steam into $1 (CI only: hearth-steam.yml and the base-*.yml image builds). Self-contained for
# linux-x64, so the images need no .NET runtime. Prints compiler errors as ::error:: lines for the CI annotations.
set -o pipefail
out="${1:?output folder}"
here="$(cd "$(dirname "$0")" && pwd)"
export DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1
if ! dotnet publish "$here/HearthSteam/HearthSteam.csproj" -c Release -o "$out" 2>&1 | tee /tmp/hearth-steam-publish.log; then
  grep -E ': (error|warning NU)[^:]*:' /tmp/hearth-steam-publish.log | sort -u | head -20 | sed 's/^/::error title=hearth-steam build::/'
  exit 1
fi
test -x "$out/hearth-steam" || { echo "::error title=hearth-steam build::no hearth-steam program in the output"; exit 1; }
# Not needed to run: the debugger/diagnostics helpers, the LTTng tracing provider and the alternative GCs.
rm -f "$out"/libmscordaccore.so "$out"/libmscordbi.so "$out"/libcoreclrtraceptprovider.so "$out"/libclrgc.so "$out"/libclrgcexp.so "$out"/createdump
# Native libraries without their debug symbols (their exported symbols stay).
command -v strip >/dev/null && find "$out" -maxdepth 1 -name '*.so' -exec strip --strip-debug {} \;
echo "hearth-steam: $(du -sh "$out" | cut -f1) in $(find "$out" -type f | wc -l) files"
