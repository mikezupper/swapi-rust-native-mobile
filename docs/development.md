# Development environment

Use Linux for the Rust core and Android. iOS builds and simulator checks run
on a macOS host. Keep setup reproducible through scripts as the platform
experiments establish working commands.

## Inspect the current host

```sh
python3 scripts/doctor.py
python3 scripts/doctor.py --json
```

This report checks tool discovery, common SDK locations, resource availability,
and Linux KVM access. A missing executable on PATH does not establish that it
is absent everywhere. The report installs nothing and is not a build test.
Store host-specific observations in the relevant Beads issue.

## Android

On Linux x86_64, install tools in `~/.local/share/holocron`:

```sh
python3 scripts/setup-android.py
source scripts/android-env.sh
./android/gradlew -p android assembleDebug lintDebug
```

The bootstrap verifies archive checksums and installs the Android SDK packages
with their licenses. It does not require sudo. Build pins are Gradle 8.13,
AGP 8.13.2, Kotlin/Compose plugin 2.2.21, Compose BOM 2025.12.00, compile/target
SDK 36, minimum SDK 26, Build Tools 35.0.0, and NDK 28.2.13676358. The bootstrap
uses Temurin 21.0.12.1+1 and command-line tools 19.0; bytecode targets Java 17.
SDK packages without revision selectors use the installed stable revision:
the verified run used emulator 37.2.12, platform-tools 37.0.1, platform 36
revision 2, and Google APIs x86_64 system image 36 revision 7. The bootstrap
does not freeze Google's mutable package indexes.

Create the test device once, then run the emulator in its own terminal:

```sh
source scripts/android-env.sh
echo no | avdmanager create avd -n holocron-api36 -k 'system-images;android-36;google_apis;x86_64' -d pixel_7
emulator -avd holocron-api36 -no-window -no-audio -no-snapshot -no-boot-anim -gpu swiftshader -memory 2048 -cores 2 -port 5554
```

Wait for `adb -s emulator-5554 shell getprop sys.boot_completed` to return `1`.
From a second terminal:

```sh
source scripts/android-env.sh
ANDROID_SERIAL=emulator-5554 ./android/gradlew -p android connectedDebugAndroidTest
adb -s emulator-5554 install -r android/app/build/outputs/apk/debug/app-debug.apk
adb -s emulator-5554 shell wm dismiss-keyguard
adb -s emulator-5554 shell am start -n dev.holocron.app/.MainActivity
mkdir -p .artifacts/android
adb -s emulator-5554 shell uiautomator dump /sdcard/holocron-window.xml
adb -s emulator-5554 exec-out screencap -p > .artifacts/android/character.png
```

Drop `-no-window` for an interactive emulator. System light/dark appearance is
available through `adb -s emulator-5554 shell cmd uimode night no` / `yes`.
The generated APK is a development build; release signing is not configured.
Gradle's `preBuild` runs `scripts/prepare-android-core.sh`: it regenerates the
bindings with the pinned workspace generator and compiles Rust for `arm64-v8a`
and `x86_64`. Native libraries go into ignored `android/app/build/rustJniLibs`.
Both ELF slices use 16 KiB segment alignment. Only x86_64 execution has been
tested so far; arm64 packaging is verified by compilation.

## Rust

The repository pins Rust 1.97.1 and direct dependencies. Ordinary tests use
fixtures and local HTTP servers:

```sh
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The optional `cargo run -p swapi-core --example luke` requests live SWAPI over
HTTPS. It is separate from deterministic verification.

`bash scripts/generate-bindings.sh` regenerates Kotlin and Swift from the host
library and the crate's `uniffi.toml`. Runtime and generator both use UniFFI
0.32.2. Generated sources in `bindings/` are not hand-edited. Android builds
invoke generation; run it before syncing changes to the Mac.

## OSX-KVM experiment

The user prefers trying [OSX-KVM](https://github.com/kholia/OSX-KVM) first.
Check CPU features, KVM access, QEMU, free disk, and available memory. Then
identify a macOS/Xcode/simulator combination that supports the intended UI
capabilities before allocating large downloads or a VM disk.

Bound the experiment by its outcome: a SwiftUI app builds, a simulator test
runs, and logs/screenshots can be retrieved. If compatibility or graphics make
that impractical, record the finding in the iOS environment bead and use the
available Mac. Keep VM files outside this repository; do not make GPU
passthrough or host reconfiguration a prerequisite without evidence of need.

The 2026-10-17 trial reached macOS Sequoia Recovery using upstream commit
`4c378a4b5e0b219783683012bec680325eb40719`, portable QEMU 8.2, and verified
Apple recovery chunks. This proves recovery boot only. Full macOS, Xcode,
graphics acceleration, and iOS simulator execution have not been verified.
The Apple M4 Mac was verified over SSH on macOS 27.0.1 (26A434), with Xcode 27.1
(27A9275). It builds and tests the native app, so the bounded VM experiment
was stopped after recovery boot. Experiment files remain in
`~/.local/share/holocron/osx-kvm-spike`.

## Mac fallback

Help the user enable Remote Login for the intended account and establish
SSH key authentication from Linux. The Mac must be reachable and awake.
The user may need to handle installation, sign-in, or permission dialogs.

Install/configure full Xcode, required runtimes, and Rust targets. Verify
`xcodebuild` and `xcrun simctl` on that host. Use a logged-in desktop session
when required by the selected simulator/UI testing workflow. Keep key material
outside the repository and verify the Mac host key on initial connection.

Use the same project build/test scripts for VM and physical Mac. Sync source
without copying build caches or the live Beads database. Return test results,
screenshots, and logs to the initiating machine. Remote desktop is optional
for interactive simulator use; command execution can use SSH.

The current Mac setup has Rust 1.97.1 and `aarch64-apple-ios` /
`aarch64-apple-ios-sim` installed. The app targets iOS 18+ and Swift 6, with
arm64 device and simulator slices. Intel simulator builds are not included.
`bash scripts/setup-mac.sh` installs the Rust tools without changing shell
profiles; full Xcode and its initial setup must already be available.

After verifying the Mac host fingerprint and authorizing the dedicated public
key, configure the initiating Linux shell:

```sh
export HOLOCRON_MAC=user@your-mac.local
# Optional overrides: HOLOCRON_MAC_KEY and HOLOCRON_MAC_KNOWN_HOSTS.
# Defaults are ~/.ssh/holocron_m4_ed25519 and ~/.ssh/holocron_m4_known_hosts.
bash scripts/generate-bindings.sh
scripts/mac.sh sync
scripts/mac.sh run bash scripts/setup-mac.sh
scripts/mac.sh run bash scripts/build-rust-ios.sh
scripts/mac.sh run xcrun simctl list devices available
```

The dedicated checkout is `~/Developer/holocron-codex`. Sync copies project
sources, bindings, and scripts; caches and the live Beads database stay on
their original host. Strict host-key checking is always enabled.

Choose an available simulator UUID, then run:

```sh
scripts/mac.sh run env HOLOCRON_SIMULATOR=YOUR_SIMULATOR_UUID bash scripts/test-ios.sh
scripts/mac.sh pull
```

The test script starts a fixture server bound only to `127.0.0.1:18765`, runs
Xcode tests, and stops the server. UI tests exercise the actual Rust HTTP path
without relying on SWAPI. Results bundles and screenshots are pulled into
`.artifacts/ios/mac`. The verified simulator is iPhone 18 Pro on iOS 27.0.

For a live run, boot a simulator if it is shut down and install/launch the app
without fixture arguments:

```sh
scripts/mac.sh run xcrun simctl boot YOUR_SIMULATOR_UUID
scripts/mac.sh run xcrun simctl bootstatus YOUR_SIMULATOR_UUID -b
scripts/mac.sh run xcrun simctl install YOUR_SIMULATOR_UUID .artifacts/ios/DerivedData/Build/Products/Debug-iphonesimulator/Holocron.app
scripts/mac.sh run xcrun simctl launch YOUR_SIMULATOR_UUID dev.holocron.app
scripts/mac.sh run xcrun simctl io YOUR_SIMULATOR_UUID screenshot .artifacts/ios/live.png
scripts/mac.sh pull
```

The separate unsigned device build is:

```sh
scripts/mac.sh run xcodebuild -project ios/Holocron.xcodeproj -scheme Holocron -destination 'generic/platform=iOS' -derivedDataPath .artifacts/ios/DeviceBuild CODE_SIGNING_ALLOWED=NO build
```

This verifies device compilation/linking, not physical-device installation.
Signing and real-device performance are still separate work. The current Mac
tool installation supports `simctl` screenshots and tests; a Simulator GUI
application has not been found in the standard Xcode location.

## Beads and source control

The installed Beads CLI uses an embedded Dolt database. Run `bd prime` and use
the project-local skill from [AGENTS.md](../AGENTS.md). The integration relies
on AGENTS.md instructions; Codex lifecycle hooks have not been installed.

Source and task synchronization are separate. A plain git clone does not
fetch the issue database. Once a remote is selected, configure and verify the
supported Dolt remote workflow and use `bd bootstrap` on fresh clones.
Do not treat `.beads/issues.jsonl` as the live database. Use supported local
backups and Dolt remotes for cross-machine issue synchronization:

```sh
bd backup init "$HOME/.local/share/holocron/beads-backup"
bd backup sync
python3 scripts/check-beads.py
```

The backup destination and state are machine-local and ignored by Git. The
backup was created and synchronized successfully with bd 1.2.2. Its embedded
backend does not support `bd doctor`, and `bd lint` returns zero coverage.
The read-only audit instead checks every issue through list/show JSON, required
fields, closure evidence, dependency references and cycles. It refuses an empty
database and writes `.artifacts/beads-audit.json`. This validates the work graph,
not physical Dolt storage integrity; no database migration was needed.

## References

- [OSX-KVM](https://github.com/kholia/OSX-KVM)
- [Xcode system requirements](https://developer.apple.com/xcode/system-requirements)
- [Apple Remote Login](https://support.apple.com/guide/mac-help/allow-a-remote-computer-to-access-your-mac-mchlp1066/mac)
- [Xcode command-line tools](https://developer.apple.com/documentation/xcode/xcode-command-line-tool-reference)
- [Rust iOS targets](https://doc.rust-lang.org/rustc/platform-support/apple-ios.html)
