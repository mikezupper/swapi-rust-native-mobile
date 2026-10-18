#!/usr/bin/env python3
"""Run the same fail-fast, logged checks locally and in CI."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('platform', choices=['core', 'android', 'ios'])
    args = parser.parse_args()
    output = ROOT / '.artifacts/verification' / args.platform
    output.mkdir(parents=True, exist_ok=True)
    report = dict(platform=args.platform, started=datetime.now(timezone.utc).isoformat(), steps=[])

    def run(name, command):
        print(f'{name}: running (log: {output / (name + ".log")})', flush=True)
        start = time.monotonic()
        with (output / (name + '.log')).open('w') as log:
            result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)
        report['steps'].append(dict(name=name, command=command, exit_code=result.returncode,
                                   seconds=round(time.monotonic() - start, 2)))
        (output / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
        if result.returncode:
            print('\n'.join((output / (name + '.log')).read_text().splitlines()[-35:]))
            raise SystemExit(result.returncode)
        print(f'{name}: passed', flush=True)

    run('tokens', ['python3', 'scripts/generate-design.py', '--check'])
    run('bindings', ['bash', 'scripts/generate-bindings.sh', '--check'])
    if args.platform == 'core':
        run('tooling-tests', ['python3', '-m', 'unittest', 'discover', '-s', 'scripts/tests', '-v'])
        run('docs', ['python3', 'scripts/check_docs.py'])
        run('format', ['cargo', 'fmt', '--all', '--check'])
        run('tests', ['cargo', 'test', '--locked', '--workspace'])
        run('clippy', ['cargo', 'clippy', '--locked', '--workspace', '--all-targets', '--', '-D', 'warnings'])
    elif args.platform == 'android':
        if not os.environ.get('ANDROID_SERIAL'):
            raise SystemExit('Set ANDROID_SERIAL to the booted emulator/device; no test skipping.')
        run('build-lint-tests', ['bash', '-c', 'source scripts/android-env.sh && ./android/gradlew -p android assembleDebug lintDebug connectedDebugAndroidTest'])
        run('screenshots', ['bash', '-c', 'source scripts/android-env.sh && python3 scripts/collect-android-artifacts.py'])
    else:
        if not os.environ.get('HOLOCRON_SIMULATOR'):
            raise SystemExit('Set HOLOCRON_SIMULATOR to an available simulator UUID; no test skipping.')
        run('xcframework', ['bash', 'scripts/build-rust-ios.sh'])
        run('simulator-tests', ['bash', 'scripts/test-ios.sh'])
        run('device-build', ['xcodebuild', '-project', 'ios/Holocron.xcodeproj', '-scheme', 'Holocron', '-destination', 'generic/platform=iOS', '-derivedDataPath', '.artifacts/ios/DeviceBuild', 'CODE_SIGNING_ALLOWED=NO', 'build'])
    print(f'All {args.platform} checks passed; summary: {output / "summary.json"}')


if __name__ == '__main__':
    main()
