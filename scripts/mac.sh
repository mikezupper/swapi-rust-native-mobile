#!/usr/bin/env bash
# Strict-key SSH transport for the dedicated project directory on a Mac.
set -euo pipefail
cd "$(dirname "$0")/.."
: "${HOLOCRON_MAC:?Set HOLOCRON_MAC to user@hostname or user@IP.}"
key="${HOLOCRON_MAC_KEY:-$HOME/.ssh/holocron_m4_ed25519}"
known="${HOLOCRON_MAC_KNOWN_HOSTS:-$HOME/.ssh/holocron_m4_known_hosts}"
ssh_args=(-i "$key" -o IdentitiesOnly=yes -o BatchMode=yes -o StrictHostKeyChecking=yes -o "UserKnownHostsFile=$known")
printf -v transport '%q ' ssh "${ssh_args[@]}"
action="${1:-}"; if test "$#" -gt 0; then shift; fi
case "$action" in
  sync)
    ssh "${ssh_args[@]}" "$HOLOCRON_MAC" 'set -e; dest="$HOME/Developer/holocron-codex"; if test -e "$dest" && ! test -f "$dest/.holocron-build-host"; then echo "Destination exists without project marker" >&2; exit 1; fi; mkdir -p "$dest"; touch "$dest/.holocron-build-host"'
    rsync -aR --exclude build --exclude Frameworks --exclude .gradle --exclude .kotlin -e "$transport" \
      Cargo.toml Cargo.lock rust-toolchain.toml crates bindings ios android scripts design docs .agents .github README.md ARCHITECTURE.md AGENTS.md \
      "$HOLOCRON_MAC:Developer/holocron-codex/"
    ;;
  run)
    test "$#" -gt 0 || { echo 'Provide a command to run in the Mac checkout.' >&2; exit 1; }
    printf -v command '%q ' "$@"
    ssh "${ssh_args[@]}" "$HOLOCRON_MAC" "cd \"\$HOME/Developer/holocron-codex\" && export PATH=\"\$HOME/.cargo/bin:\$PATH\" && $command"
    ;;
  pull)
    mkdir -p .artifacts/ios/mac
    rsync -a --exclude DerivedData --exclude DeviceBuild --exclude headers -e "$transport" \
      "$HOLOCRON_MAC:Developer/holocron-codex/.artifacts/ios/" .artifacts/ios/mac/
    if ssh "${ssh_args[@]}" "$HOLOCRON_MAC" 'test -d "$HOME/Developer/holocron-codex/.artifacts/verification/ios"'; then
      mkdir -p .artifacts/ios/mac/verification
      rsync -a -e "$transport" \
        "$HOLOCRON_MAC:Developer/holocron-codex/.artifacts/verification/ios/" .artifacts/ios/mac/verification/
    fi
    ;;
  *) echo 'Usage: scripts/mac.sh sync | run COMMAND [ARG...] | pull' >&2; exit 1 ;;
esac
