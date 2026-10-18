#!/usr/bin/env python3
"""Collect and validate native test artifacts before the CI emulator stops."""
from pathlib import Path
import subprocess
import tarfile

root = Path(__file__).resolve().parents[1]
output = root / '.artifacts/verification/android/screenshots.tar'
output.parent.mkdir(parents=True, exist_ok=True)
with output.open('wb') as stream:
    subprocess.run(['adb', 'exec-out', 'run-as', 'dev.holocron.app', 'tar',
                    '-C', 'files', '-cf', '-', 'screenshots'], stdout=stream, check=True)
# adb exec-out can return success with a run-as error in stdout. Validate the
# archive itself so an error message can never masquerade as captured evidence.
with tarfile.open(output) as archive:
    names = set(archive.getnames())
    expected = {'screenshots/scroll-gfxinfo.txt', 'screenshots/display.txt',
                'screenshots/recreated-query-fixture.png', 'screenshots/scroll-fixture.png',
                'screenshots/people-fixture.png', 'screenshots/detail-fixture.png',
                'screenshots/page-failure-fixture.png', 'screenshots/empty-fixture.png',
                'screenshots/offline-favorites.png', 'screenshots/offline-planet-detail.png'}
    expected |= {f'screenshots/catalog-{category}{suffix}.png'
                 for category in ('planets', 'starships', 'vehicles', 'species', 'films')
                 for suffix in ('', '-detail')}
    expected |= {f'screenshots/{name}.png' for name in ('connected-homeworld', 'comparison-tray', 'ship-comparison', 'film-crawl', 'discovery', 'widget-linked-detail', 'home-widget', 'home-widget-empty')}
    if missing := expected - names:
        raise SystemExit(f'Missing native test artifacts: {sorted(missing)}')
    for name in expected:
        with archive.extractfile(name) as stream:
            content = stream.read()
        if not content or (name.endswith('.png') and not content.startswith(b'\x89PNG\r\n\x1a\n')):
            raise SystemExit(f'Invalid native test artifact: {name}')
print(f'Validated {len(expected)} native artifacts in {output}')
