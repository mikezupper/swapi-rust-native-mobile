#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
python3 scripts/generate-design.py --check
bash scripts/generate-bindings.sh
bash scripts/build-rust-android.sh
