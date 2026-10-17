#!/usr/bin/env python3
"""Select an available iPhone on iOS 18+ without a machine-specific UUID."""
import json
import re
import subprocess

devices = json.loads(subprocess.check_output(['xcrun', 'simctl', 'list', 'devices', 'available', '--json']))['devices']
candidates = []
for runtime, values in devices.items():
    match = re.search(r'iOS-(\d+)-(\d+)', runtime)
    if match and int(match[1]) >= 18:
        for device in values:
            if device['name'].startswith('iPhone') and device.get('isAvailable'):
                candidates.append(((int(match[1]), int(match[2])), device['name'], device['udid']))
if not candidates:
    raise SystemExit('No available iPhone simulator on iOS 18+. Install a runtime in Xcode Settings > Components.')
print(sorted(candidates, reverse=True)[0][2])
