# Holocron concept 01

A reviewable proposal for Explore, Search, and Character Detail. The HTML
preview is a design artifact, not a replacement for the planned SwiftUI and
Compose apps. It uses six local SWAPI records and performs no live API calls.

From the repository root:

```sh
python3 -m http.server 8765 --bind 127.0.0.1 --directory design/concepts/holocron
```

Open <http://127.0.0.1:8765/>. Select a character to update the detail phone;
search by name; switch iOS/Android and dark/light; inspect loading, empty,
and connection-error states. Retry restores fixtures immediately. It does not
simulate latency or perform HTTP recovery.

The preview targets 393 × 852 outer phone frames on a wide display. Mid-size
windows can scroll the three frames horizontally. Small windows stack them.
The inner screens scroll independently. Device chrome is illustrative.

## Visual decisions

Graphite and ivory anchor the palette. Mint-cyan marks actions and selection;
warm orbital imagery creates the dramatic focal point. Large, compact headings
contrast with quiet facts and generous separation. The same image is used as
atmospheric art across characters, without claiming it is their homeworld.

iOS uses floating translucent navigation and a chevron back control. Android
uses anchored navigation, a pill selection indicator, an arrow back control,
and rounder cards/search. These are conceptual CSS treatments: native materials,
system gestures, typography metrics, and component behavior require the real
implementations. Fonts use the local system stack; no font files are bundled.

Press feedback uses 140 ms transitions; character changes use a 280 ms reveal.
The reduced-motion media query disables both. No continuous background motion.
DOM labels and keyboard focus are included, but native screen-reader, large-text,
contrast-over-art, and device performance checks remain necessary.

## Data and artwork

[people.json](people.json) contains six records retrieved from
[SWAPI](https://swapi.dev/documentation) on 2026-10-17. Counts in Search refer to
this sample collection. Film counts come from each record's relationship URLs.
No complete-corpus search, linked-resource pages, or caching is demonstrated.

[desert-orbit.png](assets/desert-orbit.png) is an original generated illustration,
1024 × 1536, produced with the built-in image generation tool. It is atmospheric
concept art, not canonical planet imagery or a SWAPI asset. The final generation
prompt is saved in [art-prompt.txt](art-prompt.txt). Geometric icons were authored
directly as SVG paths in this prototype. Character monograms avoid depending on
unselected portrait sources. Product naming and release asset review are still
open design decisions; no trademark affiliation is implied.

## Verification

```sh
node --check design/concepts/holocron/app.js
npm install --prefix .artifacts/concept-tools --no-save --package-lock=false jsdom@26.1.0
NODE_PATH="$PWD/.artifacts/concept-tools/node_modules" node design/concepts/holocron/verify.cjs
python3 scripts/check_docs.py
```

The smoke check exercises actual event handlers in a DOM simulation. It covers
search, character selection, focus restoration, state recovery, platform/theme
controls, and reset. It cannot verify rendering, scrolling, touch gestures, or
native behavior.

This prototype is retained as design history. The user accepted its direction;
the implemented SwiftUI and Compose screens now have native screenshot and
interaction evidence. Browser rendering of this older artifact was never
verified because the installed browser runtime was unavailable. The user
explicitly retired that obsolete review on 2026-10-17 in favor of native review.
See the [project README](../../../README.md) for the implemented experience.
