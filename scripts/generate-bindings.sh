#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
output=bindings
if [[ "${1:-}" == --check ]]; then
  output="$(mktemp -d)"
  trap 'rm -rf "$output"' EXIT
elif [[ "$#" != 0 ]]; then
  echo 'Usage: generate-bindings.sh [--check]' >&2; exit 2
fi
cargo build --locked -p swapi-ffi -p holocron-bindgen
case "$(uname -s)" in
  Linux) library=target/debug/libswapi_ffi.so ;;
  Darwin) library=target/debug/libswapi_ffi.dylib ;;
  *) echo 'Binding generation requires Linux or macOS.' >&2; exit 1 ;;
esac
for language in kotlin swift; do
  target/debug/holocron-bindgen generate --library "$library" \
    --language "$language" \
    --out-dir "$output/$language" --no-format
done
if [[ "${1:-}" == --check ]]; then
  diff -ru bindings "$output"
  echo 'Swift and Kotlin bindings match the Rust interface.'
fi
