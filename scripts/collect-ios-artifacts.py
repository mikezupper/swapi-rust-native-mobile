#!/usr/bin/env python3
"""Export XCTest attachments on the build host for review from any platform."""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1] / '.artifacts/ios'
bundles = sorted(root.glob('*.xcresult'))
if not bundles:
    print('No XCTest result bundle exists to export (check the build/test logs).')
for bundle in bundles:
    destination = root / 'attachments' / bundle.stem
    destination.mkdir(parents=True, exist_ok=True)
    subprocess.run(['xcrun', 'xcresulttool', 'export', 'attachments',
                    '--path', str(bundle), '--output-path', str(destination)], check=True)
    print(f'Exported native attachments to {destination}')
