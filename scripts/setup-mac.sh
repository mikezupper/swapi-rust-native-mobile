#!/usr/bin/env bash
set -euo pipefail
test "$(uname -s)" = Darwin || { echo 'Run this script on the Mac.' >&2; exit 1; }
xcodebuild -version
xcodebuild -checkFirstLaunchStatus
if ! test -x "$HOME/.cargo/bin/rustup"; then
  installer="$(mktemp -t holocron-rustup)"
  trap 'rm -f "$installer"' EXIT
  curl --proto '=https' --tlsv1.2 --fail --location https://sh.rustup.rs --output "$installer"
  sh "$installer" -y --profile minimal --default-toolchain 1.97.1 --no-modify-path
fi
export PATH="$HOME/.cargo/bin:$PATH"
rustup toolchain install 1.97.1 --profile minimal --component rustfmt --component clippy
rustup target add --toolchain 1.97.1 aarch64-apple-ios aarch64-apple-ios-sim
xcrun simctl list devices available
