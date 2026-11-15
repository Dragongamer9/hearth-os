#!/bin/bash
# cargo test, one test at a time, stopped after $1 seconds: a test that hangs is named in an error annotation
# instead of the job running into GitHub's hour limit with nothing to show (hearth-core hung like that a few times
# in November 2026). Usage: tools/cargo-test-watch.sh <seconds> <title> [cargo test arguments...]
set -o pipefail
secs=$1 title=$2; shift 2
cargo test --no-run "$@" 2>&1 | tail -3 || { cargo test "$@" 2>&1 | tee test.log | grep -E -A8 '^error' | head -60 | tr '\n' '|' | sed "s/^/::error title=$title::/"; echo; exit 1; }
timeout -s INT "$secs" cargo test "$@" -- --test-threads=1 2>&1 | tee test.log
rc=${PIPESTATUS[0]}
if [ "$rc" = 124 ] || [ "$rc" = 130 ]; then
  stuck=$(grep -E '^test .* \.\.\. *$' test.log | tail -1)
  echo "::error title=$title::a test hung (stopped after ${secs}s): ${stuck:-unknown} || last lines: $(tail -8 test.log | tr '\n' '|' | cut -c1-1500)"
  exit 1
fi
if [ "$rc" != 0 ]; then
  echo "::error title=$title::$( (grep -E -A8 '^(error|---- |thread .* panicked)' test.log | head -60) | tr '\n' '|' | cut -c1-3900)"
  exit 1
fi
echo "::notice title=$title tests::$(grep -E '^test result' test.log | tr '\n' ' ')"
