# Visual design and interaction

The required outcome is a striking, polished native experience on both
platforms. On 2026-10-17, the user reviewed the interactive concept and accepted
its visual direction ("it looks good"). Carry this direction into the native
screens. Exact typography, materials, layouts, and accessibility behavior still
need refinement and verification in SwiftUI and Compose.

## Accepted direction

A cinematic Star Wars field guide: deep graphite surfaces, luminous cyan and
warm amber accents, bold headings, and generous negative space. Give major
characters and planets deliberate compositions. Use light, depth, and selective
translucency to establish hierarchy. Keep dense information legible.

Explore should invite discovery through strong visual grouping. Search should
make queries, filters if needed, results, and feedback immediately clear.
Character detail should establish identity first, then make facts and related
resources easy to scan. Avoid committing to navigation for unimplemented
features merely to fill a mockup.

## Shared identity and native behavior

The first [interactive concept](../design/concepts/holocron/README.md) explores
these roles in three connected phone previews, with platform, appearance, and
state controls. Production values come from [design/tokens.json](../design/tokens.json).

| Role | Dark | Light |
| --- | --- | --- |
| Background | `#101719` | `#F4F3EC` |
| Raised surface | `#192226` | `#E8EAE4` |
| Primary text | `#F3F3EB` | `#18292B` |
| Secondary text | `#A3B0B1` | `#536569` |
| Interactive accent | `#9CEBDC` | `#0B6A5F` |
| Warm accent | `#E7BB86` | `#825224` |

The concept uses original orbital illustration and character monograms. Its
artwork is atmospheric, not canonical imagery or an image returned by SWAPI.
Asset provenance and the exact prompt live beside the preview. Typography uses
system fonts and native text scaling. `scripts/generate-design.py` generates
Swift/Kotlin palettes, spacing, shapes, typography roles and motion intent.
Run it with `--check` to reject drift and verify semantic text colors meet
4.5:1 contrast on their solid surfaces. Image overlays still require visual review.

The first native character screens use the same original orbital artwork from
the concept in Android resources and the iOS asset catalog. Their semantic
colors use the generated palettes above.
Android large text stacks the three primary facts vertically. iOS uses an
accessibility-size vertical layout. Native screenshots, rather than the HTML
concept alone, guide subsequent refinement.

A six-button, two-row category picker sits below the archive hero on both
platforms, adapting to two or one columns as text grows. Each collection has its own headline and subtitle; selected categories
use the shared accent. People cards use monograms, film counts and birth years.
Other catalog cards use monograms and two identifying facts. Detail pages use
stacked, wrapping fact labels/values so long manufacturers and large text fit.
Films also show an opening crawl. The shared orbital art remains atmospheric
and does not imply a canonical image of the selected resource. Search has explicit
empty and failure states; page failures keep the current list visible. Detail
uses native navigation with the selected resource identity. Components remain
platform-native; shared tokens establish visual continuity.

On iOS, use SwiftUI navigation and materials, including Liquid Glass where
the chosen deployment target supports it. On Android, use Compose and Material
3 Expressive capabilities appropriate to the selected stable dependencies.
Retain native back behavior, gestures, text selection, and accessibility.

Define light and dark treatments and honor system preferences. Brand accents
must remain consistent even if optional Android dynamic color is later added.

## Motion and feedback

Use responsive press feedback, intentional transitions into details, clear
loading feedback, and selective haptics. Tie motion to user actions and state
changes. Rendering and animation execute natively. Honor reduced motion and
transparency settings, and avoid effects that impair reading or scrolling.

## Assets

SWAPI's documented data does not include character image URLs. Decide the art
direction and asset source explicitly. Record asset provenance and usage terms,
use dependable local placeholders, and keep factual API data separate from
illustrative imagery. Prototype typography and composition while assets are
being selected. Do not rely on arbitrary image-search hotlinks.

## Evidence of quality

Review Explore, search, and detail concepts at real phone proportions before
expanding the UI. For implemented screens, capture native screenshots and
interaction recordings on both platforms. Inspect normal, loading, empty,
failure, pressed, and focused states; large text; and reduced motion.

Measure frame behavior on named devices or emulators. Record the display
refresh rate and test conditions before setting performance thresholds. An
emulator screenshot demonstrates rendering, not real-device frame performance.
Every visible defect or deferred design issue belongs in Beads.

## References

- [Apple materials](https://developer.apple.com/design/human-interface-guidelines/materials)
- [Material 3 in Compose](https://developer.android.com/develop/ui/compose/designsystems/material3)
- [SWAPI fields](https://swapi.dev/documentation)
