#!/usr/bin/env python3
"""Package the retained generated artwork into native app-icon resources."""
import argparse
from io import BytesIO
import json
from pathlib import Path
import sys

from PIL import Image, ImageOps


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'design/app-icon'
ANDROID = ROOT / 'android/app/src/main/res'
IOS = ROOT / 'ios/Holocron/Assets.xcassets/AppIcon.appiconset'


def png(image):
    output = BytesIO()
    image.save(output, format='PNG', optimize=False, compress_level=9)
    return output.getvalue()


def centered(art, size, height, background):
    canvas = Image.new('RGBA', (size, size), background)
    mark = ImageOps.contain(art, (height, height), Image.Resampling.LANCZOS)
    canvas.alpha_composite(mark, ((size - mark.width) // 2, (size - mark.height) // 2))
    return canvas


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='reject resource drift without writing')
    args = parser.parse_args()
    art = Image.open(SOURCE / 'holocron-source.png').convert('RGBA')
    # Ignore near-invisible alpha specks when finding the artwork's trim box.
    bounds = art.getchannel('A').point(lambda value: 255 if value > 8 else 0).getbbox()
    if not bounds:
        raise SystemExit('The source icon has no visible artwork.')
    art = art.crop(bounds)
    tokens = json.loads((ROOT / 'design/tokens.json').read_text())
    background = '#' + tokens['colors']['dark']['background']
    icon = centered(art, 1024, 804, background).convert('RGB')
    # 4 px/dp: 108dp canvas, a centered 60dp-high mark inside the safe zone.
    foreground = centered(art, 432, 240, (0, 0, 0, 0))
    contents = {
        'images': [{'filename': 'AppIcon.png', 'idiom': 'universal',
                    'platform': 'ios', 'size': '1024x1024'}],
        'info': {'author': 'xcode', 'version': 1},
    }
    outputs = {
        IOS / 'AppIcon.png': png(icon),
        IOS / 'Contents.json': (json.dumps(contents, indent=2) + '\n').encode(),
        ANDROID / 'drawable-nodpi/holocron_icon_foreground.png': png(foreground),
        ANDROID / 'drawable/launcher_monochrome.xml': (SOURCE / 'monochrome.xml').read_bytes(),
    }
    stale = []
    for path, data in outputs.items():
        if args.check:
            # Compare decoded pixels, independent of the local PNG encoder version.
            if path.suffix == '.png' and path.is_file():
                actual = Image.open(path)
                expected = Image.open(BytesIO(data))
                matches = (actual.mode == expected.mode and actual.size == expected.size
                           and actual.tobytes() == expected.tobytes())
            else:
                matches = path.is_file() and path.read_bytes() == data
            if not matches:
                stale.append(str(path.relative_to(ROOT)))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    if stale:
        raise SystemExit('App-icon resource drift; run scripts/generate-app-icons.py:\n' + '\n'.join(stale))
    print(f'App-icon resources {"checked" if args.check else "generated"}: {len(outputs)} files.')


if __name__ == '__main__':
    try:
        main()
    except OSError as error:
        sys.exit(str(error))
