#!/usr/bin/env bash
# Start a disposable AVD for CI; local verification can reuse an existing AVD.
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/android-env.sh
export ANDROID_SERIAL=emulator-5554
printf 'no\n' | avdmanager create avd --force -n holocron-ci -k 'system-images;android-36;google_apis;x86_64' -d pixel_7
mkdir -p .artifacts/android
emulator -avd holocron-ci -no-window -no-audio -no-snapshot -no-boot-anim -gpu swiftshader -memory 2048 -cores 2 -port 5554 > .artifacts/android/emulator.log 2>&1 &
emulator_pid=$!
trap 'kill "$emulator_pid" 2>/dev/null || true' EXIT
for attempt in $(seq 1 180); do
  kill -0 "$emulator_pid"
  if [[ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" == 1 ]]; then
    adb shell wm dismiss-keyguard
    exec_status=0
    python3 scripts/verify.py android || exec_status=$?
    exit "$exec_status"
  fi
  sleep 1
done
echo 'Emulator did not boot within three minutes.' >&2
exit 1
