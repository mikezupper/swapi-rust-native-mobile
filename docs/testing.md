# Verification

Validate the behavior users see through the real Rust/native boundary.
Keep ordinary checks deterministic and report the limits of each result.

## Rust

Use fixtures and a local mock HTTP server for response parsing, unknown values,
timeouts, HTTP failures, pagination, and retries. Test state transitions and
request races: a delayed response for an older query must not replace current
results. Test cancellation, subscriber disposal, and refresh behavior.

Keep live SWAPI smoke tests separate from required offline checks. A mock
transport is a core dependency choice, not duplicate behavior in native views.

Current commands: `cargo test --workspace`, `cargo fmt --all --check`, and
`cargo clippy --workspace --all-targets -- -D warnings`. The 24 tests cover ten
People/character HTTP/data scenarios, four catalog scenarios, eight serialized-state
scenarios, and two FFI listener/error/lifetime scenarios. State tests control request completion explicitly;
they do not depend on race-prone sleeps or SWAPI availability.

## Native boundary

Compile generated Swift and Kotlin bindings against the actual packaged Rust
library. Exercise commands, typed errors, state delivery, main-thread updates,
subscription cleanup, and view lifecycle changes in each platform.

Test device and simulator/ABI packaging in the relevant build environment.
Linux Rust tests alone do not validate the Apple target or native adapters.

Android's `connectedDebugAndroidTest` runs eight tests against the packaged Rust
library through generated Kotlin bindings: loading/failure/retry/content, and
disposal during pending character and People responses, and a browse/detail/
back/page-retry/search/empty/refresh journey. Additional tests exercise activity
recreation/backgrounding with accessibility checks and collect warm-scroll frame
statistics. Two catalog tests cover category navigation, details, paging, retries
and searches. Its fixture server is loopback-only.
Swift has five model tests for main-actor delivery, stale/queued snapshot rejection,
and reference cleanup, plus five UI tests for character loading/failure/retry,
the People journey, the catalog tour, accessibility/background restoration, and scrolling
measurement through the actual Rust transport. The iOS fixture response waits for the test to
release it, making the loading assertion independent of network timing.

The People state tests use Tokio's paused clock and explicit completion channels
to verify debounce, cancellation, stale-result prevention, duplicate commands,
page deduplication, retained content on failure, exact-page retry and refresh.
Native journey tests also refresh a selected non-Luke character to catch an
accidental fallback to ID 1. Fixture names and measurements are synthetic test
records; they are not canonical data.

See [development commands](development.md) for invoking both suites. Verified
targets are Android API 36 x86_64 (Pixel 7 AVD), iOS 27.0 arm64 simulator
(iPhone 18 Pro), Android arm64 library compilation, and unsigned iOS arm64 app
compilation. Physical-device execution, older minimum-OS runtime checks, and
Intel Mac simulator packaging have not been performed.

## UI and performance

Run equivalent browse, detail, search, empty, failure/retry, and pagination
scenarios on both platforms. Include background/foreground transitions and
screen recreation. Inspect actual screenshots and interaction recordings.
Verify large text, accessible controls, contrast, and reduced motion.

Measure scrolling and transitions on documented hardware. Set frame-time
targets using the actual display refresh rate and report emulator limitations.
Use native platform diagnostics when a measured problem warrants them.

## Original People-only verification (2026-10-17)

On the Apple M4 Mac, Xcode 27.1 built both arm64 XCFramework slices, passed all
four model and four UI tests on the iPhone 18 Pro / iOS 27.0 simulator, and built
the unsigned device app. The UI accessibility audit passes for People and a
character detail. It caught large-text search/detail clipping and a badge
contrast/scaling defect, which were corrected without suppressing audit types.
Normal-size native screenshots were reviewed again after those fixes.

The simulator reports 60 Hz. With 40 synthetic records, a warm-up scroll and
three measured iterations of two upward/two downward swipes, XCTest recorded
scroll/deceleration durations of 2.485190, 2.485184 and 2.484810 seconds (mean
2.485 s); the whole gesture block averaged 10.299 s. The simulator returned
duration metrics only, so these are **not** frame-rate or hitch measurements.
No physical iPhone performance claim or regression threshold is established.
The result was recorded in `Test-20261017-151859.xcresult` with screenshots and
an interaction recording. Those temporary local artifacts were removed during
the requested project cleanup; the measurements above preserve the baseline.

The original disposable `holocron-ci` Pixel 7 AVD run passed all six Android tests,
assembled both packaged ABIs, passed lint with warnings, and retained all eight
expected screenshot/diagnostic artifacts. Lint warnings concern available
dependency updates, older-OS backup metadata and launcher-icon configuration;
they are not lint errors. The normal and recreated-query screenshots were
visually reviewed.

The Android diagnostic used API 36 x86_64, 1080×2400 at density 420, a reported
60.000004 Hz display, two emulator cores, 2 GiB RAM, SwiftShader, default font
scale/animation speed, a debug build and 40 synthetic records. After one warm-up
swipe and a gfxinfo reset, eight upward/eight downward swipes produced 241 frames:
158 were reported janky (65.56%), with p50 53 ms, p90 77 ms and p95 97 ms.
This is a slow development-emulator baseline, **not evidence of smooth 60 fps**.
Debug instrumentation, Compose's test clock, software rendering and shared-host
load make it unsuitable for a physical-device release budget. Preserve these
conditions when comparing emulator runs; profile a release/profileable build
on physical devices before making a performance commitment. Raw gfxinfo and
display data were checked in the Android artifact archive before cleanup. No native UI
animation or per-frame work crosses the Rust boundary.

## Harness

`python3 scripts/check_docs.py` checks local documentation links.
`python3 scripts/doctor.py` reports environment capabilities without installing
anything. Native build/test and binding-generation commands live in `scripts/`
and the Gradle wrapper; the same commands are intended for CI.

`python3 scripts/verify.py core|android|ios` runs logged, fail-fast platform
checks, including token and binding drift. Android requires `ANDROID_SERIAL`;
iOS requires `HOLOCRON_SIMULATOR`. Missing platforms are not silently skipped.
The [CI workflow](../.github/workflows/verify.yml) runs these commands and retains
artifacts on GitHub for pushes and pull requests. See the
[Actions runs](https://github.com/mikezupper/swapi-rust-native-mobile/actions)
for hosted results; the recorded results above are local verification.
`scripts/ci-android.sh` creates and disposes a fresh AVD.

`scripts/mac.sh pull` retrieves Xcode bundles, screenshots and verification logs
without copying DerivedData. Android reports stay under `android/app/build`;
logs and summaries go to `.artifacts/verification/<platform>`. Android captured
screens and frame statistics are collected in `screenshots.tar` in its
verification directory. Gradle retains test APKs so the collector can read
private files; the collector validates the tar archive, required members and
PNG signatures. An adb exit code alone does not establish successful capture.
The iOS test script accepts additional xcodebuild
arguments, including `-only-testing:HolocronUITests/CharacterTests/<method>`.

Set `HOLOCRON_RECORD=1` when invoking `scripts/test-ios.sh` to retain a simulator
video alongside the result bundle. XCTest screenshots can be exported with
`xcrun xcresulttool export attachments --path <result.xcresult> --output-path <directory>`.
Android recordings use `adb shell screenrecord` and are retrieved with `adb pull`.
Native live screenshots show 10 of 82 records on 2026-10-17; offline test artifacts
are labeled separately. Light/dark and Android 1.8 font scale / iOS accessibility-large
screens were visually inspected. Native tests were also run with Android animator
duration scale 0 and the simulator Reduce Motion preference enabled. This is
not a complete TalkBack/VoiceOver, largest-size, device-performance or minimum-OS audit.

For each completed bead, record the commands, results, and artifact locations.
Use `.artifacts/` for local generated reports, screenshots, and logs. CI should
retain equivalent artifacts. Record an unavailable check explicitly; never
substitute a plan or mockup for observed native execution.

## Six-collection catalog expansion

The catalog fixtures in `crates/swapi-core/tests/fixtures/catalog.json` cover
People, Planets, Starships, Vehicles, Species and Films. They are deterministic
samples, with a synthetic film crawl. They are never bundled as production data.
At that milestone the Rust suite had 24 tests. Catalog checks exercise collection/detail
routing, encoded searches, category-bound pagination, malformed identities and
relationships, nullable species homeworlds, film episode/crawl parsing, and
retrying the exact detail identity. The original paging tests now exercise the
shared generic paging actor used by both People and catalog sessions.

An existing Android lifecycle assertion exposed a search race during keyboard
connection: a stale empty String could replace `Leia` before debounce completed.
The shared search field now uses native TextFieldState with observed draft
updates. The original request and restoration assertions remain intact; fixture
failures now include the unexpected request for diagnosis.

Android has eight instrumentation tests, including browsing all five added
categories through Rust and a planet paging/failure/retry/search/back flow.
Artifact collection requires a list and detail screenshot for each new category,
as well as the existing navigation, failure, accessibility and performance
artifacts. Inspect screenshots for system dialogs as well as app layout; a passing
Compose assertion alone does not establish that the desktop display was clear.

The iOS suite adds a catalog adapter lifetime/revision test and a UI tour of
all five added collections, including accessibility audits of the planet list
and detail. The initial hosted run exposed contrast issues in the original
People count/monograms; the count now uses larger primary text and monograms
use primary text on the background color (13.57:1 in light mode, 16.25:1 in dark
mode). A later audit exposed category buttons losing Dynamic Type support when
the lazy grid moved them offscreen. An eager layout preserves their identities
and resolved those failures. Search fields can grow from one to three lines.
The badge text uses intrinsic bounds inside a separate decorated tile: framing
the Text itself caused the audit to treat the entire tile as the text element.
Separating those views resolved the remaining contrast failures. No accessibility
checks are suppressed. Older Mac verification above applies to the original
People-only revision.

[Hosted run 39073533572](https://github.com/mikezupper/swapi-rust-native-mobile/actions/runs/39073533572)
passed all three jobs on commit `c795f69`: 24 Rust tests plus formatting, Clippy
and generation checks; eight Android instrumentation tests, both packaged ABIs,
lint and all 18 required artifacts; five iOS model tests and five UI tests,
both arm64 XCFramework slices and an unsigned device build. iOS used Xcode 26.6,
the iOS 26.5 SDK and an iPhone Air simulator. The People/character and planet
list/detail accessibility audits passed with all audit types enabled.

Reviewed Android evidence includes live lists for all collections, dark-mode
Planets/Films, a Starship detail, 160% light-mode Planets, and the corrected
full-width light-mode detail card/status bar. Reviewed iOS evidence includes
the native Planets, Starships and Films lists and resource details. Selected
live Android and fixture-backed iOS screens are retained in the README; full
logs, screenshots, recordings and result bundles are available in the Actions
artifacts. Temporary local reports/build outputs were removed after verification,
while source, generated bindings, toolchains and Beads were preserved. These
results do not establish physical-device performance or minimum-OS coverage.

`cargo run --locked -p swapi-core --example catalog` is an explicit live check.
On October 17, 2026 it successfully fetched each collection and its first detail:
82 people, 60 planets, 36 starships, 39 vehicles, 37 species and 6 films.
The user's Mac checkout, project caches and simulator installations were removed
at their request. Hosted CI can validate iOS without restoring files to that Mac.

## Category artwork verification

[Run 39076835419](https://github.com/mikezupper/swapi-rust-native-mobile/actions/runs/39076835419)
passed the Rust, Android and iOS gates for artwork commit `4714000`. Six live
Android category screens and six iOS fixture collection screenshots were
visually reviewed. The [artwork gallery](../design/artwork/README.md) includes
Android captures. iOS captures are in that run's `ios-verification` artifact.

## Offline archive verification

The Rust archive suite checks database reopening, favorite preservation after
cache clearing, namespace and query isolation, fresh cache hits, stale-content
publication before refresh, malformed refresh fallback, two-page download
failure/retry/completeness, cancellation and switching offline during HTTP.
Additional cases verify forced refresh, schema-version protection, and a newer
collection response replacing an older cached detail.
Native tests exercise favorite actions, downloads, local browsing and reopening
the same database. Screenshot names include `offline-favorites` and
`offline-planet-detail` on Android and `Offline favorites` /
`Offline favorite after relaunch` on iOS. These are fixture tests; a live API
smoke test is separate.

iOS persistence UI tests use a separate `archive-fixture.sqlite` when launched
with `--endpoint` and `--archive`. `--archive-reset` clears only that fixture
database in this flow, preserving the normal production archive. Android's
archive integration test uses a unique temporary database under `cacheDir`.

The current suites contain 33 Rust tests, nine Android instrumentation tests,
five iOS adapter tests and six iOS UI tests. The saved-archive iOS flow performs
the full accessibility audit and checks that offline mode and a favorite survive
process termination and relaunch. Its toggle assertion waits for the state
committed by Rust before restarting.

Live Android verification downloaded all 36 Starships, enabled Offline mode,
searched for Falcon locally, and opened Millennium Falcon from saved collection
data. A separate check disabled emulator Wi-Fi and mobile data while app Offline
mode was off: refresh preserved the People list and displayed a saved-copy warning.
Networking, normal text size and animation settings were restored afterward.
Light appearance with 160% text and disabled animations was also reviewed.

[Run 39083100472](https://github.com/mikezupper/swapi-rust-native-mobile/actions/runs/39083100472)
passed all three gates for code commit `92ae821`: all 33 Rust tests, formatting,
Clippy, generated binding/token checks; all nine Android instrumentation tests,
lint, both packaged ABIs and 20 required artifacts; all five iOS adapter tests
and six UI tests, both arm64 XCFramework slices, and the unsigned device build.
The iOS People, catalog and saved-archive accessibility audits ran with every
audit type enabled. Native saved-archive and offline-detail screenshots from both
platforms were visually reviewed and selected captures are in the README.
The iOS build used hosted macOS; the user's cleaned Mac was not repopulated.
These checks do not establish physical-device frame timing or haptic feel.

## Connected exploration milestone

`crates/swapi-core/tests/exploration.rs` adds deterministic coverage for validated
relationships, paged related records, offline resolution, persisted recency and
comparison selection, numeric/range/sentinel comparison values, schema-1 migration,
and the strict widget link grammar. The core suite now contains 40 tests.

Android's exploration journey follows Luke to his homeworld, selects two ships,
opens the comparison, checks an unknown value, operates the film crawl and opens
discovery. The crawl uses a controlled Compose clock so Play/Pause is exercised
without the test runner fast-forwarding to the end. Widget tests reopen saved
content offline through a record link and render the actual RemoteViews layout
with populated and empty snapshots. Widget rendering uses isolated test preferences.
New screenshots use draw-synchronized capture; widget previews use the measured
native view renderer. The preview tests do not establish launcher placement behavior. Separately, the
widget was pinned to the local Pixel launcher with real saved API content and its
discovery link opened Skako (planet 56); the launcher screenshot is in the README.

SwiftUI's added journey follows the same related-record, comparison, crawl and
discovery flows, audits the discovery screen, and captures the shared WidgetKit
content view in an in-app preview. The widget extension is an embedded Xcode target
included in simulator and device builds. Simulator tests use ad-hoc signing to
carry App Group entitlements; an additional native test checks that the configured
shared container accepts an atomic snapshot. The UI journey opens two actual
record URLs from an existing modal presentation and verifies their destinations. A signed physical-device App Group setup
and widget scheduling remain separate from simulator UI evidence.
