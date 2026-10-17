#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
export IPHONEOS_DEPLOYMENT_TARGET=18.0
test "$(uname -s)" = Darwin || { echo 'Run on the macOS build host.' >&2; exit 1; }
for target in aarch64-apple-ios aarch64-apple-ios-sim; do
  cargo build --locked --release -p swapi-ffi --target "$target"
done
mkdir -p .artifacts/ios/headers ios/Frameworks
cp bindings/swift/HolocronCoreFFI.h .artifacts/ios/headers/
cp bindings/swift/HolocronCoreFFI.modulemap .artifacts/ios/headers/module.modulemap
output=ios/Frameworks/HolocronCoreFFI.xcframework
if test -d "$output"; then rm -rf "$output"; fi
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libswapi_ffi.a -headers .artifacts/ios/headers \
  -library target/aarch64-apple-ios-sim/release/libswapi_ffi.a -headers .artifacts/ios/headers \
  -output "$output"
