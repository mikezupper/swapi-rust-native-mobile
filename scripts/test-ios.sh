#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
: "${HOLOCRON_SIMULATOR:?Set HOLOCRON_SIMULATOR to an available simulator UUID.}"
mkdir -p .artifacts/ios
ready_file="$(mktemp -t holocron-fixture)"
python3 scripts/ios-fixture-server.py "$ready_file" > .artifacts/ios/fixture-server.log 2>&1 &
fixture_pid=$!
recorder_pid=""
cleanup() {
  if [[ -n "$recorder_pid" ]]; then
    kill -INT "$recorder_pid" 2>/dev/null || true
    wait "$recorder_pid" || true
  fi
  kill "$fixture_pid" 2>/dev/null || true
  rm -f "$ready_file"
}
trap cleanup EXIT
# A collision must fail the test setup rather than silently use another service.
python3 - "$fixture_pid" "$ready_file" <<'PY'
import os, sys, time
from pathlib import Path
for _ in range(50):
    os.kill(int(sys.argv[1]), 0)
    if Path(sys.argv[2]).read_text() == sys.argv[1]:
        break
    time.sleep(.1)
else:
    raise SystemExit('Fixture server did not become ready')
PY
if [[ "${HOLOCRON_RECORD:-0}" == 1 ]]; then
  xcrun simctl bootstatus "$HOLOCRON_SIMULATOR" -b
  xcrun simctl io "$HOLOCRON_SIMULATOR" recordVideo --codec=h264 ".artifacts/ios/fixture-flow-$(date +%Y%m%d-%H%M%S).mp4" &
  recorder_pid=$!
fi
xcodebuild -project ios/Holocron.xcodeproj -scheme Holocron \
  -destination "platform=iOS Simulator,id=$HOLOCRON_SIMULATOR" \
  -derivedDataPath .artifacts/ios/DerivedData \
  -resultBundlePath ".artifacts/ios/Test-$(date +%Y%m%d-%H%M%S).xcresult" \
  -parallel-testing-enabled NO CODE_SIGNING_ALLOWED=YES CODE_SIGN_IDENTITY=- test "$@"
