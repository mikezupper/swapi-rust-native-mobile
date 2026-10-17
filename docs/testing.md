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
`cargo clippy --workspace --all-targets -- -D warnings`. The 20 tests cover ten
HTTP/data scenarios, eight serialized-state scenarios, and two FFI listener/
error/lifetime scenarios. State tests control request completion explicitly;
they do not depend on race-prone sleeps or SWAPI availability.

## Native boundary

Compile generated Swift and Kotlin bindings against the actual packaged Rust
library. Exercise commands, typed errors, state delivery, main-thread updates,
subscription cleanup, and view lifecycle changes in each platform.

Test device and simulator/ABI packaging in the relevant build environment.
Linux Rust tests alone do not validate the Apple target or native adapters.

Android's `connectedDebugAndroidTest` runs six tests against the packaged Rust
library through generated Kotlin bindings: loading/failure/retry/content, and
disposal during pending character and People responses, and a browse/detail/
back/page-retry/search/empty/refresh journey. Additional tests exercise activity
recreation/backgrounding with accessibility checks and collect warm-scroll frame
statistics. Its fixture server is loopback-only.
Swift has four model tests for main-actor delivery, stale/queued snapshot rejection,
and reference cleanup, plus four UI tests for character loading/failure/retry,
the People journey, accessibility/background restoration, and scrolling
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

## Recorded verification (2026-10-17)

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
The result is retained in `Test-20261017-151859.xcresult` with screenshots and
an interaction recording under the pulled Mac artifacts.

Android's final disposable `holocron-ci` Pixel 7 AVD run passed all six tests,
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
display data are in the validated Android artifact archive. No native UI
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
artifacts on GitHub once a source remote is connected; hosted execution has
not yet been observed. `scripts/ci-android.sh` creates and disposes a fresh AVD.

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
