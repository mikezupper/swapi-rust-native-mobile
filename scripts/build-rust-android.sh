#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/android-env.sh
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
case "$(uname -s)" in
  Linux) ndk_host=linux-x86_64 ;;
  Darwin) ndk_host=darwin-x86_64 ;;
  *) echo 'Unsupported NDK build host.' >&2; exit 1 ;;
esac
ndk_bin="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$ndk_host/bin"
test -x "$ndk_bin/clang" || { echo "NDK not found: $ndk_bin" >&2; exit 1; }
rustup target add aarch64-linux-android x86_64-linux-android
for pair in aarch64-linux-android:arm64-v8a x86_64-linux-android:x86_64; do
  target="${pair%:*}"
  abi="${pair#*:}"
  env_target="${target//-/_}"
  upper_target="$(printf '%s' "$env_target" | tr '[:lower:]' '[:upper:]')"
  env "CARGO_TARGET_${upper_target}_LINKER=$ndk_bin/${target}26-clang" \
    "CC_${env_target}=$ndk_bin/${target}26-clang" \
    "AR_${env_target}=$ndk_bin/llvm-ar" \
    "CARGO_TARGET_${upper_target}_RUSTFLAGS=-C link-arg=-Wl,-z,max-page-size=16384" \
    cargo build --locked --release -p swapi-ffi --target "$target"
  mkdir -p "android/app/build/rustJniLibs/$abi"
  cp "target/$target/release/libswapi_ffi.so" "android/app/build/rustJniLibs/$abi/"
done
